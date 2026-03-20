//! Module dependency graph with re-export chain propagation and reachability analysis.
//!
//! The graph is built from resolved modules and entry points, then used to determine
//! which files are reachable and which exports are referenced.

use std::collections::VecDeque;

use rustc_hash::{FxHashMap, FxHashSet};
use std::ops::Range;
use std::path::PathBuf;

use fixedbitset::FixedBitSet;

use crate::resolve::{ResolveResult, ResolvedModule};
use fallow_types::discover::{DiscoveredFile, EntryPoint, FileId};
use fallow_types::extract::{ExportName, ImportedName};

/// The core module dependency graph.
#[derive(Debug)]
pub struct ModuleGraph {
    /// All modules indexed by `FileId`.
    pub modules: Vec<ModuleNode>,
    /// Flat edge storage for cache-friendly iteration.
    edges: Vec<Edge>,
    /// Maps npm package names to the set of `FileId`s that import them.
    pub package_usage: FxHashMap<String, Vec<FileId>>,
    /// Maps npm package names to the set of `FileId`s that import them with type-only imports.
    /// A package appearing here but not in `package_usage` (or only in both) indicates
    /// it's only used for types and could be a devDependency.
    pub type_only_package_usage: FxHashMap<String, Vec<FileId>>,
    /// All entry point `FileId`s.
    pub entry_points: FxHashSet<FileId>,
    /// Reverse index: for each `FileId`, which files import it.
    pub reverse_deps: Vec<Vec<FileId>>,
    /// Precomputed: which modules have namespace imports (import * as ns).
    namespace_imported: FixedBitSet,
}

/// A single module in the graph.
#[derive(Debug)]
pub struct ModuleNode {
    /// Unique identifier for this module.
    pub file_id: FileId,
    /// Absolute path to the module file.
    pub path: PathBuf,
    /// Range into the flat `edges` array.
    pub edge_range: Range<usize>,
    /// Exports declared by this module.
    pub exports: Vec<ExportSymbol>,
    /// Re-exports from this module (export { x } from './y', export * from './z').
    pub re_exports: Vec<ReExportEdge>,
    /// Whether this module is an entry point.
    pub is_entry_point: bool,
    /// Whether this module is reachable from any entry point.
    pub is_reachable: bool,
    /// Whether this module has CJS exports (module.exports / exports.*).
    pub has_cjs_exports: bool,
}

/// A re-export edge, tracking which exports are forwarded from which module.
#[derive(Debug)]
pub struct ReExportEdge {
    /// The module being re-exported from.
    pub source_file: FileId,
    /// The name imported from the source (or "*" for star re-exports).
    pub imported_name: String,
    /// The name exported from this module.
    pub exported_name: String,
    /// Whether this is a type-only re-export.
    pub is_type_only: bool,
}

/// An export with reference tracking.
#[derive(Debug)]
pub struct ExportSymbol {
    /// The exported name (named or default).
    pub name: ExportName,
    /// Whether this is a type-only export.
    pub is_type_only: bool,
    /// Source span of the export declaration.
    pub span: oxc_span::Span,
    /// Which files reference this export.
    pub references: Vec<SymbolReference>,
    /// Members of this export (enum members, class members).
    pub members: Vec<fallow_types::extract::MemberInfo>,
}

/// A reference to an export from another file.
#[derive(Debug, Clone)]
pub struct SymbolReference {
    /// The file that references this export.
    pub from_file: FileId,
    /// How the export is referenced.
    pub kind: ReferenceKind,
    /// Byte span of the import statement in the referencing file.
    /// Used by the LSP to locate references for Code Lens navigation.
    pub import_span: oxc_span::Span,
}

/// How an export is referenced.
#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceKind {
    /// A named import (`import { foo }`).
    NamedImport,
    /// A default import (`import Foo`).
    DefaultImport,
    /// A namespace import (`import * as ns`).
    NamespaceImport,
    /// A re-export (`export { foo } from './bar'`).
    ReExport,
    /// A dynamic import (`import('./foo')`).
    DynamicImport,
    /// A side-effect import (`import './styles'`).
    SideEffectImport,
}

/// An edge in the module graph.
#[derive(Debug)]
struct Edge {
    source: FileId,
    target: FileId,
    symbols: Vec<ImportedSymbol>,
}

/// A symbol imported across an edge.
#[derive(Debug)]
struct ImportedSymbol {
    imported_name: ImportedName,
    local_name: String,
    /// Byte span of the import statement in the source file.
    import_span: oxc_span::Span,
}

// Size assertions to prevent memory regressions in hot-path graph types.
// `Edge` is stored in a flat contiguous Vec for cache-friendly traversal.
// `ImportedSymbol` is stored in a Vec per Edge.
// `ExportSymbol` and `SymbolReference` are stored in Vecs per module node.
// `ReExportEdge` is stored in a Vec per module for re-export chain resolution.
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<Edge>() == 32);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<ImportedSymbol>() == 56);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<ExportSymbol>() == 88);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<SymbolReference>() == 16);
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<ReExportEdge>() == 56);

impl ModuleGraph {
    /// Build the module graph from resolved modules and entry points.
    pub fn build(
        resolved_modules: &[ResolvedModule],
        entry_points: &[EntryPoint],
        files: &[DiscoveredFile],
    ) -> Self {
        let _span = tracing::info_span!("build_graph").entered();

        let module_count = files.len();

        // Compute the total capacity needed, accounting for workspace FileIds
        // that may exceed files.len() if IDs are assigned beyond the file count.
        let max_file_id = files
            .iter()
            .map(|f| f.id.0 as usize)
            .max()
            .map_or(0, |m| m + 1);
        let total_capacity = max_file_id.max(module_count);

        // Build path -> FileId index
        let path_to_id: FxHashMap<PathBuf, FileId> =
            files.iter().map(|f| (f.path.clone(), f.id)).collect();

        // Build FileId -> ResolvedModule index
        let module_by_id: FxHashMap<FileId, &ResolvedModule> =
            resolved_modules.iter().map(|m| (m.file_id, m)).collect();

        // Build entry point set — use path_to_id map instead of O(n) scan per entry
        let entry_point_ids: FxHashSet<FileId> = entry_points
            .iter()
            .filter_map(|ep| {
                // Try direct lookup first (fast path)
                path_to_id.get(&ep.path).copied().or_else(|| {
                    // Fallback: canonicalize entry point and do a direct FxHashMap lookup
                    ep.path
                        .canonicalize()
                        .ok()
                        .and_then(|c| path_to_id.get(&c).copied())
                })
            })
            .collect();

        // Phase 1: Build flat edge storage, module nodes, and package usage from resolved modules
        let mut graph = Self::populate_edges(
            files,
            &module_by_id,
            &entry_point_ids,
            module_count,
            total_capacity,
        );

        // Phase 2: Record which files reference which exports (namespace + CSS module narrowing)
        graph.populate_references(&module_by_id, &entry_point_ids);

        // Phase 3: BFS from entry points to mark reachable modules
        graph.mark_reachable(total_capacity);

        // Phase 4: Propagate references through re-export chains
        graph.resolve_re_export_chains();

        graph
    }

    /// Build flat edge storage from resolved modules.
    ///
    /// Creates `ModuleNode` entries, flat `Edge` storage, reverse dependency
    /// indices, package usage maps, and the namespace-imported bitset.
    #[allow(clippy::cognitive_complexity)] // Complex but coherent edge-population logic
    fn populate_edges(
        files: &[DiscoveredFile],
        module_by_id: &FxHashMap<FileId, &ResolvedModule>,
        entry_point_ids: &FxHashSet<FileId>,
        module_count: usize,
        total_capacity: usize,
    ) -> Self {
        let mut all_edges = Vec::new();
        let mut modules = Vec::with_capacity(module_count);
        let mut package_usage: FxHashMap<String, Vec<FileId>> = FxHashMap::default();
        let mut type_only_package_usage: FxHashMap<String, Vec<FileId>> = FxHashMap::default();
        let mut reverse_deps = vec![Vec::new(); total_capacity];
        let mut namespace_imported = FixedBitSet::with_capacity(total_capacity);

        for file in files {
            let edge_start = all_edges.len();

            if let Some(resolved) = module_by_id.get(&file.id) {
                // Group imports by target
                let mut edges_by_target: FxHashMap<FileId, Vec<ImportedSymbol>> =
                    FxHashMap::default();

                for import in &resolved.resolved_imports {
                    match &import.target {
                        ResolveResult::InternalModule(target_id) => {
                            // Track namespace imports during edge creation
                            if matches!(import.info.imported_name, ImportedName::Namespace) {
                                let idx = target_id.0 as usize;
                                if idx < total_capacity {
                                    namespace_imported.insert(idx);
                                }
                            }
                            edges_by_target
                                .entry(*target_id)
                                .or_default()
                                .push(ImportedSymbol {
                                    imported_name: import.info.imported_name.clone(),
                                    local_name: import.info.local_name.clone(),
                                    import_span: import.info.span,
                                });
                        }
                        ResolveResult::NpmPackage(name) => {
                            package_usage.entry(name.clone()).or_default().push(file.id);
                            if import.info.is_type_only {
                                type_only_package_usage
                                    .entry(name.clone())
                                    .or_default()
                                    .push(file.id);
                            }
                        }
                        _ => {}
                    }
                }

                // Re-exports also create edges
                for re_export in &resolved.re_exports {
                    if let ResolveResult::InternalModule(target_id) = &re_export.target {
                        // ALL re-exports use SideEffect edges to avoid marking source
                        // exports as "used" just because they're re-exported. The
                        // re-export chain propagation handles tracking which specific
                        // names consumers actually import.
                        edges_by_target
                            .entry(*target_id)
                            .or_default()
                            .push(ImportedSymbol {
                                imported_name: ImportedName::SideEffect,
                                local_name: String::new(),
                                import_span: oxc_span::Span::new(0, 0),
                            });
                    } else if let ResolveResult::NpmPackage(name) = &re_export.target {
                        package_usage.entry(name.clone()).or_default().push(file.id);
                        if re_export.info.is_type_only {
                            type_only_package_usage
                                .entry(name.clone())
                                .or_default()
                                .push(file.id);
                        }
                    }
                }

                // Dynamic imports — use the imported_name/local_name from resolution.
                // Named imports (`const { foo } = await import('./x')`) create Named edges.
                // Namespace imports (`const mod = await import('./x')`) create Namespace edges
                // with a local_name, enabling member access narrowing.
                // Side-effect imports (`await import('./x')`) create SideEffect edges.
                for import in &resolved.resolved_dynamic_imports {
                    if let ResolveResult::InternalModule(target_id) = &import.target {
                        if matches!(import.info.imported_name, ImportedName::Namespace) {
                            let idx = target_id.0 as usize;
                            if idx < total_capacity {
                                namespace_imported.insert(idx);
                            }
                        }
                        edges_by_target
                            .entry(*target_id)
                            .or_default()
                            .push(ImportedSymbol {
                                imported_name: import.info.imported_name.clone(),
                                local_name: import.info.local_name.clone(),
                                import_span: import.info.span,
                            });
                    }
                }

                // Dynamic import patterns (template literals, string concat, import.meta.glob)
                for (_pattern, matched_ids) in &resolved.resolved_dynamic_patterns {
                    for target_id in matched_ids {
                        let idx = target_id.0 as usize;
                        if idx < total_capacity {
                            namespace_imported.insert(idx);
                        }
                        edges_by_target
                            .entry(*target_id)
                            .or_default()
                            .push(ImportedSymbol {
                                imported_name: ImportedName::Namespace,
                                local_name: String::new(),
                                import_span: oxc_span::Span::new(0, 0),
                            });
                    }
                }

                // Sort by target FileId for deterministic edge order across runs
                let mut sorted_edges: Vec<_> = edges_by_target.into_iter().collect();
                sorted_edges.sort_by_key(|(target_id, _)| target_id.0);

                for (target_id, symbols) in sorted_edges {
                    all_edges.push(Edge {
                        source: file.id,
                        target: target_id,
                        symbols,
                    });

                    if (target_id.0 as usize) < reverse_deps.len() {
                        reverse_deps[target_id.0 as usize].push(file.id);
                    }
                }
            }

            let edge_end = all_edges.len();

            let mut exports: Vec<ExportSymbol> = module_by_id
                .get(&file.id)
                .map(|m| {
                    m.exports
                        .iter()
                        .map(|e| ExportSymbol {
                            name: e.name.clone(),
                            is_type_only: e.is_type_only,
                            span: e.span,
                            references: Vec::new(),
                            members: e.members.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            // Create ExportSymbol entries for re-exports so that consumers
            // importing from this barrel can have their references attached.
            // Without this, `export { Foo } from './source'` on a barrel would
            // not be trackable as an export of the barrel module.
            if let Some(resolved) = module_by_id.get(&file.id) {
                for re in &resolved.re_exports {
                    // Skip star re-exports without an alias (`export * from './x'`)
                    // — they don't create a named export on the barrel.
                    // But `export * as name from './x'` does create one.
                    if re.info.exported_name == "*" {
                        continue;
                    }

                    // Avoid duplicates: if an export with this name already exists
                    // (e.g. the module both declares and re-exports the same name),
                    // skip creating another one.
                    let export_name = if re.info.exported_name == "default" {
                        ExportName::Default
                    } else {
                        ExportName::Named(re.info.exported_name.clone())
                    };
                    let already_exists = exports.iter().any(|e| e.name == export_name);
                    if already_exists {
                        continue;
                    }

                    exports.push(ExportSymbol {
                        name: export_name,
                        is_type_only: re.info.is_type_only,
                        span: oxc_span::Span::new(0, 0), // re-exports don't have a meaningful span on the barrel
                        references: Vec::new(),
                        members: Vec::new(),
                    });
                }
            }

            let has_cjs_exports = module_by_id
                .get(&file.id)
                .is_some_and(|m| m.has_cjs_exports);

            // Build re-export edges
            let re_export_edges: Vec<ReExportEdge> = module_by_id
                .get(&file.id)
                .map(|m| {
                    m.re_exports
                        .iter()
                        .filter_map(|re| {
                            if let ResolveResult::InternalModule(target_id) = &re.target {
                                Some(ReExportEdge {
                                    source_file: *target_id,
                                    imported_name: re.info.imported_name.clone(),
                                    exported_name: re.info.exported_name.clone(),
                                    is_type_only: re.info.is_type_only,
                                })
                            } else {
                                None
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            modules.push(ModuleNode {
                file_id: file.id,
                path: file.path.clone(),
                edge_range: edge_start..edge_end,
                exports,
                re_exports: re_export_edges,
                is_entry_point: entry_point_ids.contains(&file.id),
                is_reachable: false,
                has_cjs_exports,
            });
        }

        Self {
            modules,
            edges: all_edges,
            package_usage,
            type_only_package_usage,
            entry_points: entry_point_ids.clone(),
            reverse_deps,
            namespace_imported,
        }
    }

    /// Record which files reference which exports from edges.
    ///
    /// Walks every edge and attaches `SymbolReference` entries to the target
    /// module's exports. Includes namespace import narrowing (member access
    /// tracking) and CSS Module default-import narrowing.
    #[allow(clippy::cognitive_complexity)]
    fn populate_references(
        &mut self,
        module_by_id: &FxHashMap<FileId, &ResolvedModule>,
        entry_point_ids: &FxHashSet<FileId>,
    ) {
        for edge_idx in 0..self.edges.len() {
            let source_id = self.edges[edge_idx].source;
            let target_idx = self.edges[edge_idx].target.0 as usize;
            if target_idx >= self.modules.len() {
                continue;
            }
            for sym_idx in 0..self.edges[edge_idx].symbols.len() {
                let sym_imported_name = self.edges[edge_idx].symbols[sym_idx].imported_name.clone();
                let sym_local_name = self.edges[edge_idx].symbols[sym_idx].local_name.clone();
                let sym_import_span = self.edges[edge_idx].symbols[sym_idx].import_span;

                let ref_kind = match &sym_imported_name {
                    ImportedName::Named(_) => ReferenceKind::NamedImport,
                    ImportedName::Default => ReferenceKind::DefaultImport,
                    ImportedName::Namespace => ReferenceKind::NamespaceImport,
                    ImportedName::SideEffect => ReferenceKind::SideEffectImport,
                };

                let target_module = &mut self.modules[target_idx];

                // Match to specific export
                if let Some(export) = target_module
                    .exports
                    .iter_mut()
                    .find(|e| export_matches(&e.name, &sym_imported_name))
                {
                    export.references.push(SymbolReference {
                        from_file: source_id,
                        kind: ref_kind,
                        import_span: sym_import_span,
                    });
                }

                // Namespace imports: check if we can narrow to specific member accesses.
                // `import * as ns from './x'; ns.foo; ns.bar` → only mark foo, bar as used.
                // If the namespace variable is re-exported (`export { ns }`) or no member
                // accesses are found, conservatively mark ALL exports as used.
                if matches!(sym_imported_name, ImportedName::Namespace)
                    && !sym_local_name.is_empty()
                {
                    let local_name = &sym_local_name;
                    let source_mod = module_by_id.get(&source_id);
                    let accessed_members: Vec<String> = source_mod
                        .map(|m| {
                            m.member_accesses
                                .iter()
                                .filter(|ma| ma.object == *local_name)
                                .map(|ma| ma.member.clone())
                                .collect()
                        })
                        .unwrap_or_default();

                    // Check if the namespace variable is re-exported (export { ns } or export default ns)
                    // from a NON-entry-point file. If the importing file IS an entry point,
                    // the re-export is for external consumption and doesn't prove internal usage.
                    let is_re_exported_from_non_entry = source_mod.is_some_and(|m| {
                        m.exports
                            .iter()
                            .any(|e| e.local_name.as_deref() == Some(local_name.as_str()))
                    }) && !entry_point_ids.contains(&source_id);

                    // For entry point files with no member accesses, the namespace
                    // is purely re-exported for external use — don't mark all exports
                    // as used internally. The `export *` path handles individual tracking.
                    let is_entry_with_no_access =
                        accessed_members.is_empty() && entry_point_ids.contains(&source_id);

                    if !is_entry_with_no_access
                        && (accessed_members.is_empty() || is_re_exported_from_non_entry)
                    {
                        // Can't narrow — mark all exports as referenced (conservative)
                        for export in &mut self.modules[target_idx].exports {
                            if export.references.iter().all(|r| r.from_file != source_id) {
                                export.references.push(SymbolReference {
                                    from_file: source_id,
                                    kind: ReferenceKind::NamespaceImport,
                                    import_span: sym_import_span,
                                });
                            }
                        }
                    } else {
                        // Narrow: only mark accessed members as referenced
                        let mut found_members: FxHashSet<String> = FxHashSet::default();
                        for export in &mut self.modules[target_idx].exports {
                            let name_str = export.name.to_string();
                            if accessed_members.contains(&name_str) {
                                found_members.insert(name_str);
                                if export.references.iter().all(|r| r.from_file != source_id) {
                                    export.references.push(SymbolReference {
                                        from_file: source_id,
                                        kind: ReferenceKind::NamespaceImport,
                                        import_span: sym_import_span,
                                    });
                                }
                            }
                        }

                        // For members not found on the target (e.g., barrel with
                        // `export *` that has no own exports for these names),
                        // create synthetic ExportSymbol entries so that
                        // resolve_re_export_chains can propagate them to the
                        // actual source modules.
                        let target_has_star_re_exports = self.modules[target_idx]
                            .re_exports
                            .iter()
                            .any(|re| re.exported_name == "*");
                        if target_has_star_re_exports {
                            for member in &accessed_members {
                                if !found_members.contains(member) {
                                    let export_name = if member == "default" {
                                        ExportName::Default
                                    } else {
                                        ExportName::Named(member.clone())
                                    };
                                    self.modules[target_idx].exports.push(ExportSymbol {
                                        name: export_name,
                                        is_type_only: false,
                                        span: oxc_span::Span::new(0, 0),
                                        references: vec![SymbolReference {
                                            from_file: source_id,
                                            kind: ReferenceKind::NamespaceImport,
                                            import_span: sym_import_span,
                                        }],
                                        members: Vec::new(),
                                    });
                                }
                            }
                        }
                    }
                } else if matches!(sym_imported_name, ImportedName::Namespace) {
                    // No local name available — mark all (conservative)
                    for export in &mut self.modules[target_idx].exports {
                        if export.references.iter().all(|r| r.from_file != source_id) {
                            export.references.push(SymbolReference {
                                from_file: source_id,
                                kind: ReferenceKind::NamespaceImport,
                                import_span: sym_import_span,
                            });
                        }
                    }
                }

                // CSS Module default imports: `import styles from './Button.module.css'`
                // Member accesses like `styles.primary` should mark the `primary` named
                // export as referenced, since CSS module default imports act as namespace
                // objects where each property corresponds to a class name (named export).
                if matches!(sym_imported_name, ImportedName::Default)
                    && !sym_local_name.is_empty()
                    && is_css_module_path(&self.modules[target_idx].path)
                {
                    let local_name = &sym_local_name;
                    let source_mod = module_by_id.get(&source_id);
                    let is_whole_object = source_mod
                        .is_some_and(|m| m.whole_object_uses.iter().any(|n| n == local_name));
                    let accessed_members: Vec<String> = source_mod
                        .map(|m| {
                            m.member_accesses
                                .iter()
                                .filter(|ma| ma.object == *local_name)
                                .map(|ma| ma.member.clone())
                                .collect()
                        })
                        .unwrap_or_default();
                    if is_whole_object || accessed_members.is_empty() {
                        for export in &mut self.modules[target_idx].exports {
                            if export.references.iter().all(|r| r.from_file != source_id) {
                                export.references.push(SymbolReference {
                                    from_file: source_id,
                                    kind: ReferenceKind::DefaultImport,
                                    import_span: sym_import_span,
                                });
                            }
                        }
                    } else {
                        for export in &mut self.modules[target_idx].exports {
                            let name_str = export.name.to_string();
                            if accessed_members.contains(&name_str)
                                && export.references.iter().all(|r| r.from_file != source_id)
                            {
                                export.references.push(SymbolReference {
                                    from_file: source_id,
                                    kind: ReferenceKind::DefaultImport,
                                    import_span: sym_import_span,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    /// Mark modules reachable from entry points via BFS.
    fn mark_reachable(&mut self, total_capacity: usize) {
        let mut visited = FixedBitSet::with_capacity(total_capacity);
        let mut queue = VecDeque::new();

        for &ep_id in &self.entry_points {
            if (ep_id.0 as usize) < total_capacity {
                visited.insert(ep_id.0 as usize);
                queue.push_back(ep_id);
            }
        }

        while let Some(file_id) = queue.pop_front() {
            if (file_id.0 as usize) >= self.modules.len() {
                continue;
            }
            let module = &self.modules[file_id.0 as usize];
            for edge in &self.edges[module.edge_range.clone()] {
                let target_idx = edge.target.0 as usize;
                if target_idx < total_capacity && !visited.contains(target_idx) {
                    visited.insert(target_idx);
                    queue.push_back(edge.target);
                }
            }
        }

        for (idx, module) in self.modules.iter_mut().enumerate() {
            module.is_reachable = visited.contains(idx);
        }
    }

    /// Resolve re-export chains: when module A re-exports from B,
    /// any reference to A's re-exported symbol should also count as a reference
    /// to B's original export (and transitively through the chain).
    fn resolve_re_export_chains(&mut self) {
        // Collect re-export info: (barrel_file_id, source_file_id, imported_name, exported_name)
        let re_export_info: Vec<(FileId, FileId, String, String)> = self
            .modules
            .iter()
            .flat_map(|m| {
                m.re_exports.iter().map(move |re| {
                    (
                        m.file_id,
                        re.source_file,
                        re.imported_name.clone(),
                        re.exported_name.clone(),
                    )
                })
            })
            .collect();

        if re_export_info.is_empty() {
            return;
        }

        // For each re-export, if the barrel's exported symbol has references,
        // propagate those references to the source module's original export.
        // We iterate until no new references are added (handles chains).
        let mut changed = true;
        let max_iterations = 20; // prevent infinite loops on cycles
        let mut iteration = 0;
        // Reuse a single HashSet across iterations to avoid repeated allocations.
        // In barrel-heavy monorepos, this loop can run up to max_iterations × re_export_info.len()
        // × target_exports.len() times — reusing with .clear() avoids O(n) allocations.
        let mut existing_refs: FxHashSet<FileId> = FxHashSet::default();

        while changed && iteration < max_iterations {
            changed = false;
            iteration += 1;

            for &(barrel_id, source_id, ref imported_name, ref exported_name) in &re_export_info {
                let barrel_idx = barrel_id.0 as usize;
                let source_idx = source_id.0 as usize;

                if barrel_idx >= self.modules.len() || source_idx >= self.modules.len() {
                    continue;
                }

                if exported_name == "*" {
                    // Star re-export (`export * from './source'`): the barrel has no named
                    // ExportSymbol entries for the re-exported names. Instead, look at which
                    // named imports other modules make from this barrel and propagate each
                    // to the matching export in the source module.

                    // Collect named imports that target the barrel from ALL edges
                    let barrel_file_id = self.modules[barrel_idx].file_id;
                    let named_refs: Vec<(String, SymbolReference)> = self
                        .edges
                        .iter()
                        .filter(|edge| edge.target == barrel_file_id)
                        .flat_map(|edge| {
                            edge.symbols.iter().filter_map(move |sym| {
                                if let ImportedName::Named(name) = &sym.imported_name {
                                    Some((
                                        name.clone(),
                                        SymbolReference {
                                            from_file: edge.source,
                                            kind: ReferenceKind::NamedImport,
                                            import_span: sym.import_span,
                                        },
                                    ))
                                } else {
                                    None
                                }
                            })
                        })
                        .collect();

                    // Also check for references already on barrel exports from
                    // prior chain propagation (handles multi-level barrel chains)
                    let barrel_export_refs: Vec<(String, SymbolReference)> = self.modules
                        [barrel_idx]
                        .exports
                        .iter()
                        .flat_map(|e| {
                            e.references
                                .iter()
                                .map(move |r| (e.name.to_string(), r.clone()))
                        })
                        .collect();

                    // Check if the source module itself has star re-exports (for multi-level chains).
                    // If so, we may need to create synthetic ExportSymbol entries on it so
                    // that the next iteration can propagate names further down the chain.
                    let source_has_star_re_exports = self.modules[source_idx]
                        .re_exports
                        .iter()
                        .any(|re| re.exported_name == "*");

                    // Propagate each named import to the matching source export.
                    // For multi-level star re-export chains (e.g., index -> intermediate -> source),
                    // intermediate barrels may not have ExportSymbol entries for the names being
                    // imported. When the source has its own star re-exports, create synthetic
                    // ExportSymbol entries so the iterative loop can propagate further on the
                    // next pass.
                    let source = &mut self.modules[source_idx];
                    for (name, ref_item) in named_refs.iter().chain(barrel_export_refs.iter()) {
                        let export_name = if name == "default" {
                            ExportName::Default
                        } else {
                            ExportName::Named(name.clone())
                        };
                        if let Some(export) =
                            source.exports.iter_mut().find(|e| e.name == export_name)
                        {
                            if export
                                .references
                                .iter()
                                .all(|r| r.from_file != ref_item.from_file)
                            {
                                export.references.push(ref_item.clone());
                                changed = true;
                            }
                        } else if source_has_star_re_exports {
                            // The source module doesn't have this export directly but
                            // it has star re-exports — create a synthetic ExportSymbol
                            // so the name can propagate through the chain on the next
                            // iteration.
                            source.exports.push(ExportSymbol {
                                name: export_name,
                                is_type_only: false,
                                span: oxc_span::Span::new(0, 0),
                                references: vec![ref_item.clone()],
                                members: Vec::new(),
                            });
                            changed = true;
                        }
                    }
                } else {
                    // Named re-export: find references to the exported name on the barrel
                    let refs_on_barrel: Vec<SymbolReference> = {
                        let barrel = &self.modules[barrel_idx];
                        barrel
                            .exports
                            .iter()
                            .filter(|e| e.name.to_string() == *exported_name)
                            .flat_map(|e| e.references.clone())
                            .collect()
                    };

                    if refs_on_barrel.is_empty() {
                        continue;
                    }

                    // Propagate to source module's export
                    let source = &mut self.modules[source_idx];
                    let target_exports: Vec<usize> = source
                        .exports
                        .iter()
                        .enumerate()
                        .filter(|(_, e)| e.name.to_string() == *imported_name)
                        .map(|(i, _)| i)
                        .collect();

                    for export_idx in target_exports {
                        existing_refs.clear();
                        existing_refs.extend(
                            source.exports[export_idx]
                                .references
                                .iter()
                                .map(|r| r.from_file),
                        );
                        for ref_item in &refs_on_barrel {
                            if !existing_refs.contains(&ref_item.from_file) {
                                source.exports[export_idx].references.push(ref_item.clone());
                                changed = true;
                            }
                        }
                    }
                }
            }
        }

        if iteration >= max_iterations {
            tracing::warn!(
                iterations = max_iterations,
                "Re-export chain resolution hit iteration limit, some chains may be incomplete"
            );
        }
    }

    /// Total number of modules.
    pub const fn module_count(&self) -> usize {
        self.modules.len()
    }

    /// Total number of edges.
    pub const fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Check if any importer uses `import * as ns` for this module.
    /// Uses precomputed bitset — O(1) lookup.
    pub fn has_namespace_import(&self, file_id: FileId) -> bool {
        let idx = file_id.0 as usize;
        if idx >= self.namespace_imported.len() {
            return false;
        }
        self.namespace_imported.contains(idx)
    }

    /// Get the target `FileId`s of all outgoing edges for a module.
    pub fn edges_for(&self, file_id: FileId) -> Vec<FileId> {
        let idx = file_id.0 as usize;
        if idx >= self.modules.len() {
            return Vec::new();
        }
        let range = &self.modules[idx].edge_range;
        self.edges[range.clone()].iter().map(|e| e.target).collect()
    }

    /// Find all circular dependency cycles in the module graph.
    ///
    /// Uses an iterative implementation of Tarjan's strongly connected components
    /// algorithm (O(V + E)) to find all SCCs with 2 or more nodes. Each such SCC
    /// represents a set of files involved in a circular dependency.
    ///
    /// Returns cycles sorted by length (shortest first), with files within each
    /// cycle sorted by path for deterministic output.
    pub fn find_cycles(&self) -> Vec<Vec<FileId>> {
        let n = self.modules.len();
        if n == 0 {
            return Vec::new();
        }

        // Tarjan's SCC state
        let mut index_counter: u32 = 0;
        let mut indices: Vec<u32> = vec![u32::MAX; n]; // u32::MAX = undefined
        let mut lowlinks: Vec<u32> = vec![0; n];
        let mut on_stack = FixedBitSet::with_capacity(n);
        let mut stack: Vec<usize> = Vec::new();
        let mut sccs: Vec<Vec<FileId>> = Vec::new();

        // Iterative DFS stack frame
        struct Frame {
            node: usize,
            succ_pos: usize,
            succ_end: usize,
        }

        // Pre-collect all successors (deduplicated) into a flat vec for cache-friendly access.
        let mut all_succs: Vec<usize> = Vec::with_capacity(self.edges.len());
        let mut succ_ranges: Vec<Range<usize>> = Vec::with_capacity(n);
        let mut seen_set = FxHashSet::default();
        for module in &self.modules {
            let start = all_succs.len();
            seen_set.clear();
            for edge in &self.edges[module.edge_range.clone()] {
                let target = edge.target.0 as usize;
                if target < n && seen_set.insert(target) {
                    all_succs.push(target);
                }
            }
            let end = all_succs.len();
            succ_ranges.push(start..end);
        }

        let mut dfs_stack: Vec<Frame> = Vec::new();

        for start_node in 0..n {
            if indices[start_node] != u32::MAX {
                continue;
            }

            // Push the starting node
            indices[start_node] = index_counter;
            lowlinks[start_node] = index_counter;
            index_counter += 1;
            on_stack.insert(start_node);
            stack.push(start_node);

            let range = &succ_ranges[start_node];
            dfs_stack.push(Frame {
                node: start_node,
                succ_pos: range.start,
                succ_end: range.end,
            });

            while let Some(frame) = dfs_stack.last_mut() {
                if frame.succ_pos < frame.succ_end {
                    let w = all_succs[frame.succ_pos];
                    frame.succ_pos += 1;

                    if indices[w] == u32::MAX {
                        // Tree edge: push w onto the DFS stack
                        indices[w] = index_counter;
                        lowlinks[w] = index_counter;
                        index_counter += 1;
                        on_stack.insert(w);
                        stack.push(w);

                        let range = &succ_ranges[w];
                        dfs_stack.push(Frame {
                            node: w,
                            succ_pos: range.start,
                            succ_end: range.end,
                        });
                    } else if on_stack.contains(w) {
                        // Back edge: update lowlink
                        let v = frame.node;
                        lowlinks[v] = lowlinks[v].min(indices[w]);
                    }
                } else {
                    // All successors processed — pop this frame
                    let v = frame.node;
                    let v_lowlink = lowlinks[v];
                    let v_index = indices[v];
                    dfs_stack.pop();

                    // Update parent's lowlink
                    if let Some(parent) = dfs_stack.last_mut() {
                        lowlinks[parent.node] = lowlinks[parent.node].min(v_lowlink);
                    }

                    // If v is a root node, pop the SCC
                    if v_lowlink == v_index {
                        let mut scc = Vec::new();
                        loop {
                            let w = stack.pop().expect("SCC stack should not be empty");
                            on_stack.set(w, false);
                            scc.push(FileId(w as u32));
                            if w == v {
                                break;
                            }
                        }
                        // Only report cycles of length >= 2
                        if scc.len() >= 2 {
                            sccs.push(scc);
                        }
                    }
                }
            }
        }

        // Phase 2: Enumerate individual elementary cycles within each SCC.
        // For small SCCs (len == 2), there's exactly one cycle.
        // For larger SCCs, use bounded DFS to find up to MAX_CYCLES_PER_SCC cycles.
        const MAX_CYCLES_PER_SCC: usize = 20;

        let mut result: Vec<Vec<FileId>> = Vec::new();
        let mut seen_cycles: FxHashSet<Vec<u32>> = FxHashSet::default();

        for scc in &sccs {
            if scc.len() == 2 {
                let mut cycle = vec![scc[0].0 as usize, scc[1].0 as usize];
                // Canonical: smallest path first
                if self.modules[cycle[1]].path < self.modules[cycle[0]].path {
                    cycle.swap(0, 1);
                }
                let key: Vec<u32> = cycle.iter().map(|&n| n as u32).collect();
                if seen_cycles.insert(key) {
                    result.push(cycle.into_iter().map(|n| FileId(n as u32)).collect());
                }
                continue;
            }

            let scc_nodes: Vec<usize> = scc.iter().map(|id| id.0 as usize).collect();
            let elementary = enumerate_elementary_cycles(
                &scc_nodes,
                &all_succs,
                &succ_ranges,
                MAX_CYCLES_PER_SCC,
                &self.modules,
            );

            for cycle in elementary {
                let key: Vec<u32> = cycle.iter().map(|&n| n as u32).collect();
                if seen_cycles.insert(key) {
                    result.push(cycle.into_iter().map(|n| FileId(n as u32)).collect());
                }
            }
        }

        // Sort: shortest first, then by first file path
        result.sort_by(|a, b| {
            a.len().cmp(&b.len()).then_with(|| {
                self.modules[a[0].0 as usize]
                    .path
                    .cmp(&self.modules[b[0].0 as usize].path)
            })
        });

        result
    }
}

/// Rotate a cycle so the node with the smallest path is first (canonical form for dedup).
fn canonical_cycle(cycle: &[usize], modules: &[ModuleNode]) -> Vec<usize> {
    if cycle.is_empty() {
        return Vec::new();
    }
    let min_pos = cycle
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| modules[**a].path.cmp(&modules[**b].path))
        .map_or(0, |(i, _)| i);
    let mut result = cycle[min_pos..].to_vec();
    result.extend_from_slice(&cycle[..min_pos]);
    result
}

/// Enumerate individual elementary cycles within an SCC using depth-limited DFS.
///
/// Uses iterative deepening: first finds all 2-node cycles, then 3-node, etc.
/// This ensures the shortest, most actionable cycles are always found first.
/// Stops after `max_cycles` total cycles to bound work on dense SCCs.
fn enumerate_elementary_cycles(
    scc_nodes: &[usize],
    all_succs: &[usize],
    succ_ranges: &[Range<usize>],
    max_cycles: usize,
    modules: &[ModuleNode],
) -> Vec<Vec<usize>> {
    let scc_set: FxHashSet<usize> = scc_nodes.iter().copied().collect();
    let mut cycles: Vec<Vec<usize>> = Vec::new();
    let mut seen: FxHashSet<Vec<u32>> = FxHashSet::default();

    // Sort start nodes by path for deterministic enumeration order
    let mut sorted_nodes: Vec<usize> = scc_nodes.to_vec();
    sorted_nodes.sort_by(|a, b| modules[*a].path.cmp(&modules[*b].path));

    // DFS frame for iterative cycle finding
    struct CycleFrame {
        succ_pos: usize,
        succ_end: usize,
    }

    // Iterative deepening: increase max depth from 2 up to SCC size
    let max_depth = scc_nodes.len().min(12); // Cap depth to avoid very long cycles
    for depth_limit in 2..=max_depth {
        if cycles.len() >= max_cycles {
            break;
        }

        for &start in &sorted_nodes {
            if cycles.len() >= max_cycles {
                break;
            }

            let mut path: Vec<usize> = vec![start];
            let mut path_set = FixedBitSet::with_capacity(modules.len());
            path_set.insert(start);

            let range = &succ_ranges[start];
            let mut dfs: Vec<CycleFrame> = vec![CycleFrame {
                succ_pos: range.start,
                succ_end: range.end,
            }];

            while let Some(frame) = dfs.last_mut() {
                if cycles.len() >= max_cycles {
                    break;
                }

                if frame.succ_pos >= frame.succ_end {
                    // Backtrack
                    dfs.pop();
                    if path.len() > 1 {
                        let last = path.pop().unwrap();
                        path_set.set(last, false);
                    }
                    continue;
                }

                let w = all_succs[frame.succ_pos];
                frame.succ_pos += 1;

                // Only follow edges within this SCC
                if !scc_set.contains(&w) {
                    continue;
                }

                if w == start && path.len() >= 2 && path.len() == depth_limit {
                    // Found an elementary cycle at exactly this depth
                    let canonical = canonical_cycle(&path, modules);
                    let key: Vec<u32> = canonical.iter().map(|&n| n as u32).collect();
                    if seen.insert(key) {
                        cycles.push(canonical);
                    }
                } else if !path_set.contains(w) && path.len() < depth_limit {
                    // Extend path (only if within depth limit)
                    path.push(w);
                    path_set.insert(w);

                    let range = &succ_ranges[w];
                    dfs.push(CycleFrame {
                        succ_pos: range.start,
                        succ_end: range.end,
                    });
                }
            }
        }
    }

    cycles
}

/// Check if a path is a CSS Module file (`.module.css` or `.module.scss`).
fn is_css_module_path(path: &std::path::Path) -> bool {
    path.file_stem()
        .and_then(|s| s.to_str())
        .is_some_and(|stem| stem.ends_with(".module"))
        && path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| ext == "css" || ext == "scss")
}

/// Check if an export name matches an imported name.
fn export_matches(export: &ExportName, import: &ImportedName) -> bool {
    match (export, import) {
        (ExportName::Named(e), ImportedName::Named(i)) => e == i,
        (ExportName::Default, ImportedName::Default) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::{ResolveResult, ResolvedImport, ResolvedModule, ResolvedReExport};
    use fallow_types::discover::{DiscoveredFile, EntryPoint, EntryPointSource, FileId};
    use fallow_types::extract::{ExportName, ImportInfo, ImportedName};
    use std::path::PathBuf;

    #[test]
    fn export_matches_named_same() {
        assert!(export_matches(
            &ExportName::Named("foo".to_string()),
            &ImportedName::Named("foo".to_string())
        ));
    }

    #[test]
    fn export_matches_named_different() {
        assert!(!export_matches(
            &ExportName::Named("foo".to_string()),
            &ImportedName::Named("bar".to_string())
        ));
    }

    #[test]
    fn export_matches_default() {
        assert!(export_matches(&ExportName::Default, &ImportedName::Default));
    }

    #[test]
    fn export_matches_named_vs_default() {
        assert!(!export_matches(
            &ExportName::Named("foo".to_string()),
            &ImportedName::Default
        ));
    }

    #[test]
    fn export_matches_default_vs_named() {
        assert!(!export_matches(
            &ExportName::Default,
            &ImportedName::Named("foo".to_string())
        ));
    }

    #[test]
    fn export_matches_namespace_no_match() {
        assert!(!export_matches(
            &ExportName::Named("foo".to_string()),
            &ImportedName::Namespace
        ));
        assert!(!export_matches(
            &ExportName::Default,
            &ImportedName::Namespace
        ));
    }

    #[test]
    fn export_matches_side_effect_no_match() {
        assert!(!export_matches(
            &ExportName::Named("foo".to_string()),
            &ImportedName::SideEffect
        ));
    }

    // Helper to build a simple module graph
    fn build_simple_graph() -> ModuleGraph {
        // Two files: entry.ts imports foo from utils.ts
        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: PathBuf::from("/project/src/entry.ts"),
                size_bytes: 100,
            },
            DiscoveredFile {
                id: FileId(1),
                path: PathBuf::from("/project/src/utils.ts"),
                size_bytes: 50,
            },
        ];

        let entry_points = vec![EntryPoint {
            path: PathBuf::from("/project/src/entry.ts"),
            source: EntryPointSource::PackageJsonMain,
        }];

        let resolved_modules = vec![
            ResolvedModule {
                file_id: FileId(0),
                path: PathBuf::from("/project/src/entry.ts"),
                exports: vec![],
                re_exports: vec![],
                resolved_imports: vec![ResolvedImport {
                    info: ImportInfo {
                        source: "./utils".to_string(),
                        imported_name: ImportedName::Named("foo".to_string()),
                        local_name: "foo".to_string(),
                        is_type_only: false,
                        span: oxc_span::Span::new(0, 10),
                    },
                    target: ResolveResult::InternalModule(FileId(1)),
                }],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
            ResolvedModule {
                file_id: FileId(1),
                path: PathBuf::from("/project/src/utils.ts"),
                exports: vec![
                    fallow_types::extract::ExportInfo {
                        name: ExportName::Named("foo".to_string()),
                        local_name: Some("foo".to_string()),
                        is_type_only: false,
                        span: oxc_span::Span::new(0, 20),
                        members: vec![],
                    },
                    fallow_types::extract::ExportInfo {
                        name: ExportName::Named("bar".to_string()),
                        local_name: Some("bar".to_string()),
                        is_type_only: false,
                        span: oxc_span::Span::new(25, 45),
                        members: vec![],
                    },
                ],
                re_exports: vec![],
                resolved_imports: vec![],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
        ];

        ModuleGraph::build(&resolved_modules, &entry_points, &files)
    }

    #[test]
    fn graph_module_count() {
        let graph = build_simple_graph();
        assert_eq!(graph.module_count(), 2);
    }

    #[test]
    fn graph_edge_count() {
        let graph = build_simple_graph();
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn graph_entry_point_is_reachable() {
        let graph = build_simple_graph();
        assert!(graph.modules[0].is_entry_point);
        assert!(graph.modules[0].is_reachable);
    }

    #[test]
    fn graph_imported_module_is_reachable() {
        let graph = build_simple_graph();
        assert!(!graph.modules[1].is_entry_point);
        assert!(graph.modules[1].is_reachable);
    }

    #[test]
    fn graph_export_has_reference() {
        let graph = build_simple_graph();
        let utils = &graph.modules[1];
        let foo_export = utils
            .exports
            .iter()
            .find(|e| e.name.to_string() == "foo")
            .unwrap();
        assert!(
            !foo_export.references.is_empty(),
            "foo should have references"
        );
    }

    #[test]
    fn graph_unused_export_no_reference() {
        let graph = build_simple_graph();
        let utils = &graph.modules[1];
        let bar_export = utils
            .exports
            .iter()
            .find(|e| e.name.to_string() == "bar")
            .unwrap();
        assert!(
            bar_export.references.is_empty(),
            "bar should have no references"
        );
    }

    #[test]
    fn graph_no_namespace_import() {
        let graph = build_simple_graph();
        assert!(!graph.has_namespace_import(FileId(0)));
        assert!(!graph.has_namespace_import(FileId(1)));
    }

    #[test]
    fn graph_has_namespace_import() {
        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: PathBuf::from("/project/entry.ts"),
                size_bytes: 100,
            },
            DiscoveredFile {
                id: FileId(1),
                path: PathBuf::from("/project/utils.ts"),
                size_bytes: 50,
            },
        ];

        let entry_points = vec![EntryPoint {
            path: PathBuf::from("/project/entry.ts"),
            source: EntryPointSource::PackageJsonMain,
        }];

        let resolved_modules = vec![
            ResolvedModule {
                file_id: FileId(0),
                path: PathBuf::from("/project/entry.ts"),
                exports: vec![],
                re_exports: vec![],
                resolved_imports: vec![ResolvedImport {
                    info: ImportInfo {
                        source: "./utils".to_string(),
                        imported_name: ImportedName::Namespace,
                        local_name: "utils".to_string(),
                        is_type_only: false,
                        span: oxc_span::Span::new(0, 10),
                    },
                    target: ResolveResult::InternalModule(FileId(1)),
                }],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
            ResolvedModule {
                file_id: FileId(1),
                path: PathBuf::from("/project/utils.ts"),
                exports: vec![fallow_types::extract::ExportInfo {
                    name: ExportName::Named("foo".to_string()),
                    local_name: Some("foo".to_string()),
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 20),
                    members: vec![],
                }],
                re_exports: vec![],
                resolved_imports: vec![],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
        ];

        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);
        assert!(
            graph.has_namespace_import(FileId(1)),
            "utils should have namespace import"
        );
    }

    #[test]
    fn graph_has_namespace_import_out_of_bounds() {
        let graph = build_simple_graph();
        assert!(!graph.has_namespace_import(FileId(999)));
    }

    #[test]
    fn graph_unreachable_module() {
        // Three files: entry imports utils, orphan is not imported
        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: PathBuf::from("/project/entry.ts"),
                size_bytes: 100,
            },
            DiscoveredFile {
                id: FileId(1),
                path: PathBuf::from("/project/utils.ts"),
                size_bytes: 50,
            },
            DiscoveredFile {
                id: FileId(2),
                path: PathBuf::from("/project/orphan.ts"),
                size_bytes: 30,
            },
        ];

        let entry_points = vec![EntryPoint {
            path: PathBuf::from("/project/entry.ts"),
            source: EntryPointSource::PackageJsonMain,
        }];

        let resolved_modules = vec![
            ResolvedModule {
                file_id: FileId(0),
                path: PathBuf::from("/project/entry.ts"),
                exports: vec![],
                re_exports: vec![],
                resolved_imports: vec![ResolvedImport {
                    info: ImportInfo {
                        source: "./utils".to_string(),
                        imported_name: ImportedName::Named("foo".to_string()),
                        local_name: "foo".to_string(),
                        is_type_only: false,
                        span: oxc_span::Span::new(0, 10),
                    },
                    target: ResolveResult::InternalModule(FileId(1)),
                }],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
            ResolvedModule {
                file_id: FileId(1),
                path: PathBuf::from("/project/utils.ts"),
                exports: vec![fallow_types::extract::ExportInfo {
                    name: ExportName::Named("foo".to_string()),
                    local_name: Some("foo".to_string()),
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 20),
                    members: vec![],
                }],
                re_exports: vec![],
                resolved_imports: vec![],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
            ResolvedModule {
                file_id: FileId(2),
                path: PathBuf::from("/project/orphan.ts"),
                exports: vec![fallow_types::extract::ExportInfo {
                    name: ExportName::Named("orphan".to_string()),
                    local_name: Some("orphan".to_string()),
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 20),
                    members: vec![],
                }],
                re_exports: vec![],
                resolved_imports: vec![],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
        ];

        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);

        assert!(graph.modules[0].is_reachable, "entry should be reachable");
        assert!(graph.modules[1].is_reachable, "utils should be reachable");
        assert!(
            !graph.modules[2].is_reachable,
            "orphan should NOT be reachable"
        );
    }

    #[test]
    fn graph_package_usage_tracked() {
        let files = vec![DiscoveredFile {
            id: FileId(0),
            path: PathBuf::from("/project/entry.ts"),
            size_bytes: 100,
        }];

        let entry_points = vec![EntryPoint {
            path: PathBuf::from("/project/entry.ts"),
            source: EntryPointSource::PackageJsonMain,
        }];

        let resolved_modules = vec![ResolvedModule {
            file_id: FileId(0),
            path: PathBuf::from("/project/entry.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![
                ResolvedImport {
                    info: ImportInfo {
                        source: "react".to_string(),
                        imported_name: ImportedName::Default,
                        local_name: "React".to_string(),
                        is_type_only: false,
                        span: oxc_span::Span::new(0, 10),
                    },
                    target: ResolveResult::NpmPackage("react".to_string()),
                },
                ResolvedImport {
                    info: ImportInfo {
                        source: "lodash".to_string(),
                        imported_name: ImportedName::Named("merge".to_string()),
                        local_name: "merge".to_string(),
                        is_type_only: false,
                        span: oxc_span::Span::new(15, 30),
                    },
                    target: ResolveResult::NpmPackage("lodash".to_string()),
                },
            ],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
        }];

        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);
        assert!(graph.package_usage.contains_key("react"));
        assert!(graph.package_usage.contains_key("lodash"));
        assert!(!graph.package_usage.contains_key("express"));
    }

    #[test]
    fn graph_re_export_chain_propagates_references() {
        // entry.ts -> barrel.ts -re-exports-> source.ts
        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: PathBuf::from("/project/entry.ts"),
                size_bytes: 100,
            },
            DiscoveredFile {
                id: FileId(1),
                path: PathBuf::from("/project/barrel.ts"),
                size_bytes: 50,
            },
            DiscoveredFile {
                id: FileId(2),
                path: PathBuf::from("/project/source.ts"),
                size_bytes: 50,
            },
        ];

        let entry_points = vec![EntryPoint {
            path: PathBuf::from("/project/entry.ts"),
            source: EntryPointSource::PackageJsonMain,
        }];

        let resolved_modules = vec![
            // entry imports "foo" from barrel
            ResolvedModule {
                file_id: FileId(0),
                path: PathBuf::from("/project/entry.ts"),
                exports: vec![],
                re_exports: vec![],
                resolved_imports: vec![ResolvedImport {
                    info: ImportInfo {
                        source: "./barrel".to_string(),
                        imported_name: ImportedName::Named("foo".to_string()),
                        local_name: "foo".to_string(),
                        is_type_only: false,
                        span: oxc_span::Span::new(0, 10),
                    },
                    target: ResolveResult::InternalModule(FileId(1)),
                }],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
            // barrel re-exports "foo" from source
            ResolvedModule {
                file_id: FileId(1),
                path: PathBuf::from("/project/barrel.ts"),
                exports: vec![fallow_types::extract::ExportInfo {
                    name: ExportName::Named("foo".to_string()),
                    local_name: Some("foo".to_string()),
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 20),
                    members: vec![],
                }],
                re_exports: vec![ResolvedReExport {
                    info: fallow_types::extract::ReExportInfo {
                        source: "./source".to_string(),
                        imported_name: "foo".to_string(),
                        exported_name: "foo".to_string(),
                        is_type_only: false,
                    },
                    target: ResolveResult::InternalModule(FileId(2)),
                }],
                resolved_imports: vec![],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
            // source has the actual export
            ResolvedModule {
                file_id: FileId(2),
                path: PathBuf::from("/project/source.ts"),
                exports: vec![fallow_types::extract::ExportInfo {
                    name: ExportName::Named("foo".to_string()),
                    local_name: Some("foo".to_string()),
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 20),
                    members: vec![],
                }],
                re_exports: vec![],
                resolved_imports: vec![],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
        ];

        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);

        // The source module's "foo" export should have references propagated through the barrel
        let source_module = &graph.modules[2];
        let foo_export = source_module
            .exports
            .iter()
            .find(|e| e.name.to_string() == "foo")
            .unwrap();
        assert!(
            !foo_export.references.is_empty(),
            "source foo should have propagated references through barrel re-export chain"
        );
    }

    #[test]
    fn graph_empty() {
        let graph = ModuleGraph::build(&[], &[], &[]);
        assert_eq!(graph.module_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn graph_cjs_exports_tracked() {
        let files = vec![DiscoveredFile {
            id: FileId(0),
            path: PathBuf::from("/project/entry.ts"),
            size_bytes: 100,
        }];

        let entry_points = vec![EntryPoint {
            path: PathBuf::from("/project/entry.ts"),
            source: EntryPointSource::PackageJsonMain,
        }];

        let resolved_modules = vec![ResolvedModule {
            file_id: FileId(0),
            path: PathBuf::from("/project/entry.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: true,
        }];

        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);
        assert!(graph.modules[0].has_cjs_exports);
    }

    #[test]
    fn barrel_re_export_creates_export_symbol() {
        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: PathBuf::from("/project/entry.ts"),
                size_bytes: 100,
            },
            DiscoveredFile {
                id: FileId(1),
                path: PathBuf::from("/project/barrel.ts"),
                size_bytes: 50,
            },
            DiscoveredFile {
                id: FileId(2),
                path: PathBuf::from("/project/source.ts"),
                size_bytes: 50,
            },
        ];

        let entry_points = vec![EntryPoint {
            path: PathBuf::from("/project/entry.ts"),
            source: EntryPointSource::PackageJsonMain,
        }];

        let resolved_modules = vec![
            ResolvedModule {
                file_id: FileId(0),
                path: PathBuf::from("/project/entry.ts"),
                exports: vec![],
                re_exports: vec![],
                resolved_imports: vec![ResolvedImport {
                    info: ImportInfo {
                        source: "./barrel".to_string(),
                        imported_name: ImportedName::Named("foo".to_string()),
                        local_name: "foo".to_string(),
                        is_type_only: false,
                        span: oxc_span::Span::new(0, 10),
                    },
                    target: ResolveResult::InternalModule(FileId(1)),
                }],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
            ResolvedModule {
                file_id: FileId(1),
                path: PathBuf::from("/project/barrel.ts"),
                exports: vec![],
                re_exports: vec![ResolvedReExport {
                    info: fallow_types::extract::ReExportInfo {
                        source: "./source".to_string(),
                        imported_name: "foo".to_string(),
                        exported_name: "foo".to_string(),
                        is_type_only: false,
                    },
                    target: ResolveResult::InternalModule(FileId(2)),
                }],
                resolved_imports: vec![],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
            ResolvedModule {
                file_id: FileId(2),
                path: PathBuf::from("/project/source.ts"),
                exports: vec![fallow_types::extract::ExportInfo {
                    name: ExportName::Named("foo".to_string()),
                    local_name: Some("foo".to_string()),
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 20),
                    members: vec![],
                }],
                re_exports: vec![],
                resolved_imports: vec![],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
        ];

        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);

        let barrel = &graph.modules[1];
        let foo_export = barrel.exports.iter().find(|e| e.name.to_string() == "foo");
        assert!(
            foo_export.is_some(),
            "barrel should have ExportSymbol for re-exported 'foo'"
        );

        let foo = foo_export.unwrap();
        assert!(
            !foo.references.is_empty(),
            "barrel's foo should have a reference from entry.ts"
        );

        let source = &graph.modules[2];
        let source_foo = source
            .exports
            .iter()
            .find(|e| e.name.to_string() == "foo")
            .unwrap();
        assert!(
            !source_foo.references.is_empty(),
            "source foo should have propagated references through barrel"
        );
    }

    #[test]
    fn barrel_unused_re_export_has_no_references() {
        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: PathBuf::from("/project/entry.ts"),
                size_bytes: 100,
            },
            DiscoveredFile {
                id: FileId(1),
                path: PathBuf::from("/project/barrel.ts"),
                size_bytes: 50,
            },
            DiscoveredFile {
                id: FileId(2),
                path: PathBuf::from("/project/source.ts"),
                size_bytes: 50,
            },
        ];

        let entry_points = vec![EntryPoint {
            path: PathBuf::from("/project/entry.ts"),
            source: EntryPointSource::PackageJsonMain,
        }];

        let resolved_modules = vec![
            ResolvedModule {
                file_id: FileId(0),
                path: PathBuf::from("/project/entry.ts"),
                exports: vec![],
                re_exports: vec![],
                resolved_imports: vec![ResolvedImport {
                    info: ImportInfo {
                        source: "./barrel".to_string(),
                        imported_name: ImportedName::Named("foo".to_string()),
                        local_name: "foo".to_string(),
                        is_type_only: false,
                        span: oxc_span::Span::new(0, 10),
                    },
                    target: ResolveResult::InternalModule(FileId(1)),
                }],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
            ResolvedModule {
                file_id: FileId(1),
                path: PathBuf::from("/project/barrel.ts"),
                exports: vec![],
                re_exports: vec![
                    ResolvedReExport {
                        info: fallow_types::extract::ReExportInfo {
                            source: "./source".to_string(),
                            imported_name: "foo".to_string(),
                            exported_name: "foo".to_string(),
                            is_type_only: false,
                        },
                        target: ResolveResult::InternalModule(FileId(2)),
                    },
                    ResolvedReExport {
                        info: fallow_types::extract::ReExportInfo {
                            source: "./source".to_string(),
                            imported_name: "bar".to_string(),
                            exported_name: "bar".to_string(),
                            is_type_only: false,
                        },
                        target: ResolveResult::InternalModule(FileId(2)),
                    },
                ],
                resolved_imports: vec![],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
            ResolvedModule {
                file_id: FileId(2),
                path: PathBuf::from("/project/source.ts"),
                exports: vec![
                    fallow_types::extract::ExportInfo {
                        name: ExportName::Named("foo".to_string()),
                        local_name: Some("foo".to_string()),
                        is_type_only: false,
                        span: oxc_span::Span::new(0, 20),
                        members: vec![],
                    },
                    fallow_types::extract::ExportInfo {
                        name: ExportName::Named("bar".to_string()),
                        local_name: Some("bar".to_string()),
                        is_type_only: false,
                        span: oxc_span::Span::new(25, 45),
                        members: vec![],
                    },
                ],
                re_exports: vec![],
                resolved_imports: vec![],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
        ];

        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);

        let barrel = &graph.modules[1];
        let foo = barrel
            .exports
            .iter()
            .find(|e| e.name.to_string() == "foo")
            .unwrap();
        assert!(!foo.references.is_empty(), "barrel's foo should be used");

        let bar = barrel
            .exports
            .iter()
            .find(|e| e.name.to_string() == "bar")
            .unwrap();
        assert!(
            bar.references.is_empty(),
            "barrel's bar should be unused (no consumer imports it)"
        );
    }

    #[test]
    fn type_only_re_export_creates_type_only_export_symbol() {
        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: PathBuf::from("/project/entry.ts"),
                size_bytes: 100,
            },
            DiscoveredFile {
                id: FileId(1),
                path: PathBuf::from("/project/barrel.ts"),
                size_bytes: 50,
            },
            DiscoveredFile {
                id: FileId(2),
                path: PathBuf::from("/project/source.ts"),
                size_bytes: 50,
            },
        ];

        let entry_points = vec![EntryPoint {
            path: PathBuf::from("/project/entry.ts"),
            source: EntryPointSource::PackageJsonMain,
        }];

        let resolved_modules = vec![
            ResolvedModule {
                file_id: FileId(0),
                path: PathBuf::from("/project/entry.ts"),
                exports: vec![],
                re_exports: vec![],
                resolved_imports: vec![ResolvedImport {
                    info: ImportInfo {
                        source: "./barrel".to_string(),
                        imported_name: ImportedName::Named("UsedType".to_string()),
                        local_name: "UsedType".to_string(),
                        is_type_only: true,
                        span: oxc_span::Span::new(0, 10),
                    },
                    target: ResolveResult::InternalModule(FileId(1)),
                }],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
            ResolvedModule {
                file_id: FileId(1),
                path: PathBuf::from("/project/barrel.ts"),
                exports: vec![],
                re_exports: vec![
                    ResolvedReExport {
                        info: fallow_types::extract::ReExportInfo {
                            source: "./source".to_string(),
                            imported_name: "UsedType".to_string(),
                            exported_name: "UsedType".to_string(),
                            is_type_only: true,
                        },
                        target: ResolveResult::InternalModule(FileId(2)),
                    },
                    ResolvedReExport {
                        info: fallow_types::extract::ReExportInfo {
                            source: "./source".to_string(),
                            imported_name: "UnusedType".to_string(),
                            exported_name: "UnusedType".to_string(),
                            is_type_only: true,
                        },
                        target: ResolveResult::InternalModule(FileId(2)),
                    },
                ],
                resolved_imports: vec![],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
            ResolvedModule {
                file_id: FileId(2),
                path: PathBuf::from("/project/source.ts"),
                exports: vec![
                    fallow_types::extract::ExportInfo {
                        name: ExportName::Named("UsedType".to_string()),
                        local_name: Some("UsedType".to_string()),
                        is_type_only: true,
                        span: oxc_span::Span::new(0, 20),
                        members: vec![],
                    },
                    fallow_types::extract::ExportInfo {
                        name: ExportName::Named("UnusedType".to_string()),
                        local_name: Some("UnusedType".to_string()),
                        is_type_only: true,
                        span: oxc_span::Span::new(25, 45),
                        members: vec![],
                    },
                ],
                re_exports: vec![],
                resolved_imports: vec![],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
        ];

        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);

        let barrel = &graph.modules[1];

        let used_type = barrel
            .exports
            .iter()
            .find(|e| e.name.to_string() == "UsedType")
            .expect("barrel should have ExportSymbol for UsedType");
        assert!(used_type.is_type_only, "UsedType should be type-only");
        assert!(
            !used_type.references.is_empty(),
            "UsedType should have references"
        );

        let unused_type = barrel
            .exports
            .iter()
            .find(|e| e.name.to_string() == "UnusedType")
            .expect("barrel should have ExportSymbol for UnusedType");
        assert!(unused_type.is_type_only, "UnusedType should be type-only");
        assert!(
            unused_type.references.is_empty(),
            "UnusedType should have no references"
        );
    }

    #[test]
    fn default_re_export_creates_default_export_symbol() {
        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: PathBuf::from("/project/entry.ts"),
                size_bytes: 100,
            },
            DiscoveredFile {
                id: FileId(1),
                path: PathBuf::from("/project/barrel.ts"),
                size_bytes: 50,
            },
            DiscoveredFile {
                id: FileId(2),
                path: PathBuf::from("/project/source.ts"),
                size_bytes: 50,
            },
        ];

        let entry_points = vec![EntryPoint {
            path: PathBuf::from("/project/entry.ts"),
            source: EntryPointSource::PackageJsonMain,
        }];

        let resolved_modules = vec![
            ResolvedModule {
                file_id: FileId(0),
                path: PathBuf::from("/project/entry.ts"),
                exports: vec![],
                re_exports: vec![],
                resolved_imports: vec![ResolvedImport {
                    info: ImportInfo {
                        source: "./barrel".to_string(),
                        imported_name: ImportedName::Named("Accordion".to_string()),
                        local_name: "Accordion".to_string(),
                        is_type_only: false,
                        span: oxc_span::Span::new(0, 10),
                    },
                    target: ResolveResult::InternalModule(FileId(1)),
                }],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
            ResolvedModule {
                file_id: FileId(1),
                path: PathBuf::from("/project/barrel.ts"),
                exports: vec![],
                re_exports: vec![ResolvedReExport {
                    info: fallow_types::extract::ReExportInfo {
                        source: "./source".to_string(),
                        imported_name: "default".to_string(),
                        exported_name: "Accordion".to_string(),
                        is_type_only: false,
                    },
                    target: ResolveResult::InternalModule(FileId(2)),
                }],
                resolved_imports: vec![],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
            ResolvedModule {
                file_id: FileId(2),
                path: PathBuf::from("/project/source.ts"),
                exports: vec![fallow_types::extract::ExportInfo {
                    name: ExportName::Default,
                    local_name: None,
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 20),
                    members: vec![],
                }],
                re_exports: vec![],
                resolved_imports: vec![],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
        ];

        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);

        let barrel = &graph.modules[1];
        let accordion = barrel
            .exports
            .iter()
            .find(|e| e.name.to_string() == "Accordion")
            .expect("barrel should have ExportSymbol for Accordion");
        assert!(
            !accordion.references.is_empty(),
            "Accordion should have reference from entry.ts"
        );

        let source = &graph.modules[2];
        let default_export = source
            .exports
            .iter()
            .find(|e| matches!(e.name, ExportName::Default))
            .unwrap();
        assert!(
            !default_export.references.is_empty(),
            "source default export should have propagated references"
        );
    }

    #[test]
    fn multi_level_re_export_chain_propagation() {
        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: PathBuf::from("/project/entry.ts"),
                size_bytes: 100,
            },
            DiscoveredFile {
                id: FileId(1),
                path: PathBuf::from("/project/barrel1.ts"),
                size_bytes: 50,
            },
            DiscoveredFile {
                id: FileId(2),
                path: PathBuf::from("/project/barrel2.ts"),
                size_bytes: 50,
            },
            DiscoveredFile {
                id: FileId(3),
                path: PathBuf::from("/project/source.ts"),
                size_bytes: 50,
            },
        ];

        let entry_points = vec![EntryPoint {
            path: PathBuf::from("/project/entry.ts"),
            source: EntryPointSource::PackageJsonMain,
        }];

        let resolved_modules = vec![
            ResolvedModule {
                file_id: FileId(0),
                path: PathBuf::from("/project/entry.ts"),
                exports: vec![],
                re_exports: vec![],
                resolved_imports: vec![ResolvedImport {
                    info: ImportInfo {
                        source: "./barrel1".to_string(),
                        imported_name: ImportedName::Named("foo".to_string()),
                        local_name: "foo".to_string(),
                        is_type_only: false,
                        span: oxc_span::Span::new(0, 10),
                    },
                    target: ResolveResult::InternalModule(FileId(1)),
                }],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
            ResolvedModule {
                file_id: FileId(1),
                path: PathBuf::from("/project/barrel1.ts"),
                exports: vec![],
                re_exports: vec![ResolvedReExport {
                    info: fallow_types::extract::ReExportInfo {
                        source: "./barrel2".to_string(),
                        imported_name: "foo".to_string(),
                        exported_name: "foo".to_string(),
                        is_type_only: false,
                    },
                    target: ResolveResult::InternalModule(FileId(2)),
                }],
                resolved_imports: vec![],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
            ResolvedModule {
                file_id: FileId(2),
                path: PathBuf::from("/project/barrel2.ts"),
                exports: vec![],
                re_exports: vec![ResolvedReExport {
                    info: fallow_types::extract::ReExportInfo {
                        source: "./source".to_string(),
                        imported_name: "foo".to_string(),
                        exported_name: "foo".to_string(),
                        is_type_only: false,
                    },
                    target: ResolveResult::InternalModule(FileId(3)),
                }],
                resolved_imports: vec![],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
            ResolvedModule {
                file_id: FileId(3),
                path: PathBuf::from("/project/source.ts"),
                exports: vec![fallow_types::extract::ExportInfo {
                    name: ExportName::Named("foo".to_string()),
                    local_name: Some("foo".to_string()),
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 20),
                    members: vec![],
                }],
                re_exports: vec![],
                resolved_imports: vec![],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            },
        ];

        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);

        let barrel1 = &graph.modules[1];
        let b1_foo = barrel1
            .exports
            .iter()
            .find(|e| e.name.to_string() == "foo")
            .unwrap();
        assert!(
            !b1_foo.references.is_empty(),
            "barrel1's foo should be referenced"
        );

        let barrel2 = &graph.modules[2];
        let b2_foo = barrel2
            .exports
            .iter()
            .find(|e| e.name.to_string() == "foo")
            .unwrap();
        assert!(
            !b2_foo.references.is_empty(),
            "barrel2's foo should be referenced (propagated through chain)"
        );

        let source = &graph.modules[3];
        let src_foo = source
            .exports
            .iter()
            .find(|e| e.name.to_string() == "foo")
            .unwrap();
        assert!(
            !src_foo.references.is_empty(),
            "source's foo should be referenced (propagated through 2-level chain)"
        );
    }

    // ---- Cycle detection tests ----

    /// Helper: build a graph from files+edges, no entry points needed for cycle detection.
    fn build_cycle_graph(file_count: usize, edges_spec: &[(u32, u32)]) -> ModuleGraph {
        let files: Vec<DiscoveredFile> = (0..file_count)
            .map(|i| DiscoveredFile {
                id: FileId(i as u32),
                path: PathBuf::from(format!("/project/file{i}.ts")),
                size_bytes: 100,
            })
            .collect();

        let resolved_modules: Vec<ResolvedModule> = (0..file_count)
            .map(|i| {
                let imports: Vec<ResolvedImport> = edges_spec
                    .iter()
                    .filter(|(src, _)| *src == i as u32)
                    .map(|(_, tgt)| ResolvedImport {
                        info: ImportInfo {
                            source: format!("./file{tgt}"),
                            imported_name: ImportedName::Named("x".to_string()),
                            local_name: "x".to_string(),
                            is_type_only: false,
                            span: oxc_span::Span::new(0, 10),
                        },
                        target: ResolveResult::InternalModule(FileId(*tgt)),
                    })
                    .collect();

                ResolvedModule {
                    file_id: FileId(i as u32),
                    path: PathBuf::from(format!("/project/file{i}.ts")),
                    exports: vec![fallow_types::extract::ExportInfo {
                        name: ExportName::Named("x".to_string()),
                        local_name: Some("x".to_string()),
                        is_type_only: false,
                        span: oxc_span::Span::new(0, 20),
                        members: vec![],
                    }],
                    re_exports: vec![],
                    resolved_imports: imports,
                    resolved_dynamic_imports: vec![],
                    resolved_dynamic_patterns: vec![],
                    member_accesses: vec![],
                    whole_object_uses: vec![],
                    has_cjs_exports: false,
                }
            })
            .collect();

        let entry_points = vec![EntryPoint {
            path: PathBuf::from("/project/file0.ts"),
            source: EntryPointSource::PackageJsonMain,
        }];

        ModuleGraph::build(&resolved_modules, &entry_points, &files)
    }

    #[test]
    fn find_cycles_empty_graph() {
        let graph = ModuleGraph::build(&[], &[], &[]);
        assert!(graph.find_cycles().is_empty());
    }

    #[test]
    fn find_cycles_no_cycles() {
        // A -> B -> C (no back edges)
        let graph = build_cycle_graph(3, &[(0, 1), (1, 2)]);
        assert!(graph.find_cycles().is_empty());
    }

    #[test]
    fn find_cycles_simple_two_node_cycle() {
        // A -> B -> A
        let graph = build_cycle_graph(2, &[(0, 1), (1, 0)]);
        let cycles = graph.find_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 2);
    }

    #[test]
    fn find_cycles_three_node_cycle() {
        // A -> B -> C -> A
        let graph = build_cycle_graph(3, &[(0, 1), (1, 2), (2, 0)]);
        let cycles = graph.find_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 3);
    }

    #[test]
    fn find_cycles_self_import_ignored() {
        // A -> A (self-import, should NOT be reported)
        let graph = build_cycle_graph(1, &[(0, 0)]);
        let cycles = graph.find_cycles();
        assert!(
            cycles.is_empty(),
            "self-imports should not be reported as cycles"
        );
    }

    #[test]
    fn find_cycles_multiple_independent_cycles() {
        // Cycle 1: A -> B -> A
        // Cycle 2: C -> D -> C
        // No connection between cycles
        let graph = build_cycle_graph(4, &[(0, 1), (1, 0), (2, 3), (3, 2)]);
        let cycles = graph.find_cycles();
        assert_eq!(cycles.len(), 2);
        // Both cycles should have length 2
        assert!(cycles.iter().all(|c| c.len() == 2));
    }

    #[test]
    fn find_cycles_linear_chain_with_back_edge() {
        // A -> B -> C -> D -> B (cycle is B-C-D)
        let graph = build_cycle_graph(4, &[(0, 1), (1, 2), (2, 3), (3, 1)]);
        let cycles = graph.find_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 3);
        // The cycle should contain files 1, 2, 3
        let ids: Vec<u32> = cycles[0].iter().map(|f| f.0).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
        assert!(ids.contains(&3));
        assert!(!ids.contains(&0));
    }

    #[test]
    fn find_cycles_overlapping_cycles_enumerated() {
        // A -> B -> A, B -> C -> B => SCC is {A, B, C} but should report 2 elementary cycles
        let graph = build_cycle_graph(3, &[(0, 1), (1, 0), (1, 2), (2, 1)]);
        let cycles = graph.find_cycles();
        assert_eq!(
            cycles.len(),
            2,
            "should find 2 elementary cycles, not 1 SCC"
        );
        assert!(
            cycles.iter().all(|c| c.len() == 2),
            "both cycles should have length 2"
        );
    }

    #[test]
    fn find_cycles_deterministic_ordering() {
        // Run twice with the same graph — results should be identical
        let graph1 = build_cycle_graph(3, &[(0, 1), (1, 2), (2, 0)]);
        let graph2 = build_cycle_graph(3, &[(0, 1), (1, 2), (2, 0)]);
        let cycles1 = graph1.find_cycles();
        let cycles2 = graph2.find_cycles();
        assert_eq!(cycles1.len(), cycles2.len());
        for (c1, c2) in cycles1.iter().zip(cycles2.iter()) {
            let paths1: Vec<&PathBuf> = c1
                .iter()
                .map(|f| &graph1.modules[f.0 as usize].path)
                .collect();
            let paths2: Vec<&PathBuf> = c2
                .iter()
                .map(|f| &graph2.modules[f.0 as usize].path)
                .collect();
            assert_eq!(paths1, paths2);
        }
    }

    #[test]
    fn find_cycles_sorted_by_length() {
        // Two cycles: A-B (len 2) and C-D-E (len 3)
        let graph = build_cycle_graph(5, &[(0, 1), (1, 0), (2, 3), (3, 4), (4, 2)]);
        let cycles = graph.find_cycles();
        assert_eq!(cycles.len(), 2);
        assert!(
            cycles[0].len() <= cycles[1].len(),
            "cycles should be sorted by length"
        );
    }

    #[test]
    fn find_cycles_large_cycle() {
        // Chain of 10 nodes forming a single cycle: 0->1->2->...->9->0
        let edges: Vec<(u32, u32)> = (0..10).map(|i| (i, (i + 1) % 10)).collect();
        let graph = build_cycle_graph(10, &edges);
        let cycles = graph.find_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 10);
    }

    #[test]
    fn find_cycles_complex_scc_multiple_elementary() {
        // A square: A->B, B->C, C->D, D->A, plus diagonal A->C
        // Elementary cycles: A->B->C->D->A, A->C->D->A, and A->B->C->...
        let graph = build_cycle_graph(4, &[(0, 1), (1, 2), (2, 3), (3, 0), (0, 2)]);
        let cycles = graph.find_cycles();
        // Should find multiple elementary cycles, not just one SCC of 4
        assert!(
            cycles.len() >= 2,
            "should find at least 2 elementary cycles, got {}",
            cycles.len()
        );
        // All cycles should be shorter than the full SCC
        assert!(cycles.iter().all(|c| c.len() <= 4));
    }

    #[test]
    fn find_cycles_no_duplicate_cycles() {
        // Triangle: A->B->C->A — should find exactly 1 cycle, not duplicates
        // from different DFS start points
        let graph = build_cycle_graph(3, &[(0, 1), (1, 2), (2, 0)]);
        let cycles = graph.find_cycles();
        assert_eq!(cycles.len(), 1, "triangle should produce exactly 1 cycle");
        assert_eq!(cycles[0].len(), 3);
    }
}
