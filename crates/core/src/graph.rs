use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::Range;
use std::path::PathBuf;

use fixedbitset::FixedBitSet;

use crate::discover::{DiscoveredFile, EntryPoint, FileId};
use crate::extract::{ExportName, ImportedName};
use crate::resolve::{ResolveResult, ResolvedModule};

/// The core module dependency graph.
#[derive(Debug)]
pub struct ModuleGraph {
    /// All modules indexed by FileId.
    pub modules: Vec<ModuleNode>,
    /// Flat edge storage for cache-friendly iteration.
    edges: Vec<Edge>,
    /// Maps npm package names to the set of FileIds that import them.
    pub package_usage: HashMap<String, Vec<FileId>>,
    /// All entry point FileIds.
    pub entry_points: HashSet<FileId>,
    /// Reverse index: for each FileId, which files import it.
    pub reverse_deps: Vec<Vec<FileId>>,
    /// Precomputed: which modules have namespace imports (import * as ns).
    namespace_imported: FixedBitSet,
}

/// A single module in the graph.
#[derive(Debug)]
pub struct ModuleNode {
    pub file_id: FileId,
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
    pub name: ExportName,
    pub is_type_only: bool,
    pub span: oxc_span::Span,
    /// Which files reference this export.
    pub references: Vec<SymbolReference>,
    /// Members of this export (enum members, class members).
    pub members: Vec<crate::extract::MemberInfo>,
}

/// A reference to an export from another file.
#[derive(Debug, Clone)]
pub struct SymbolReference {
    pub from_file: FileId,
    pub kind: ReferenceKind,
}

/// How an export is referenced.
#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceKind {
    NamedImport,
    DefaultImport,
    NamespaceImport,
    ReExport,
    DynamicImport,
    SideEffectImport,
}

/// An edge in the module graph.
#[derive(Debug)]
#[allow(dead_code)]
struct Edge {
    source: FileId,
    target: FileId,
    symbols: Vec<ImportedSymbol>,
    is_dynamic: bool,
    is_side_effect: bool,
}

/// A symbol imported across an edge.
#[derive(Debug)]
struct ImportedSymbol {
    imported_name: ImportedName,
    #[allow(dead_code)]
    local_name: String,
}

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
            .map(|m| m + 1)
            .unwrap_or(0);
        let total_capacity = max_file_id.max(module_count);

        // Build path -> FileId index
        let path_to_id: HashMap<PathBuf, FileId> =
            files.iter().map(|f| (f.path.clone(), f.id)).collect();

        // Build FileId -> ResolvedModule index
        let module_by_id: HashMap<FileId, &ResolvedModule> =
            resolved_modules.iter().map(|m| (m.file_id, m)).collect();

        let mut all_edges = Vec::new();
        let mut modules = Vec::with_capacity(module_count);
        let mut package_usage: HashMap<String, Vec<FileId>> = HashMap::new();
        let mut reverse_deps = vec![Vec::new(); total_capacity];

        // Build entry point set — use path_to_id map instead of O(n) scan per entry
        let entry_point_ids: HashSet<FileId> = entry_points
            .iter()
            .filter_map(|ep| {
                // Try direct lookup first (fast path)
                path_to_id.get(&ep.path).copied().or_else(|| {
                    // Fallback: canonicalize entry point and match
                    ep.path.canonicalize().ok().and_then(|c| {
                        path_to_id
                            .iter()
                            .find(|(p, _)| p.canonicalize().ok().as_ref() == Some(&c))
                            .map(|(_, &id)| id)
                    })
                })
            })
            .collect();

        // Track which modules have namespace imports (precomputed)
        let mut namespace_imported = FixedBitSet::with_capacity(total_capacity);

        for file in files {
            let edge_start = all_edges.len();

            if let Some(resolved) = module_by_id.get(&file.id) {
                // Group imports by target
                let mut edges_by_target: HashMap<FileId, Vec<ImportedSymbol>> = HashMap::new();

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
                                });
                        }
                        ResolveResult::NpmPackage(name) => {
                            package_usage.entry(name.clone()).or_default().push(file.id);
                        }
                        _ => {}
                    }
                }

                // Re-exports also create edges
                for re_export in &resolved.re_exports {
                    if let ResolveResult::InternalModule(target_id) = &re_export.target {
                        let imp_name = if re_export.info.imported_name == "*" {
                            ImportedName::Namespace
                        } else {
                            ImportedName::Named(re_export.info.imported_name.clone())
                        };
                        // Track namespace re-exports
                        if matches!(imp_name, ImportedName::Namespace) {
                            let idx = target_id.0 as usize;
                            if idx < total_capacity {
                                namespace_imported.insert(idx);
                            }
                        }
                        edges_by_target
                            .entry(*target_id)
                            .or_default()
                            .push(ImportedSymbol {
                                imported_name: imp_name,
                                local_name: re_export.info.exported_name.clone(),
                            });
                    } else if let ResolveResult::NpmPackage(name) = &re_export.target {
                        package_usage.entry(name.clone()).or_default().push(file.id);
                    }
                }

                // Dynamic imports - treat as namespace import since caller can access any export
                for import in &resolved.resolved_dynamic_imports {
                    if let ResolveResult::InternalModule(target_id) = &import.target {
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
                            });
                    }
                }

                for (target_id, symbols) in edges_by_target {
                    let is_side_effect = symbols
                        .iter()
                        .any(|s| matches!(s.imported_name, ImportedName::SideEffect));

                    all_edges.push(Edge {
                        source: file.id,
                        target: target_id,
                        symbols,
                        is_dynamic: false,
                        is_side_effect,
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
                .map(|m| m.has_cjs_exports)
                .unwrap_or(false);

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

        // Populate export references from edges — O(edges) not O(edges × modules)
        for edge in &all_edges {
            let source_id = edge.source;
            let Some(target_module) = modules.get_mut(edge.target.0 as usize) else {
                continue;
            };
            for sym in &edge.symbols {
                let ref_kind = match &sym.imported_name {
                    ImportedName::Named(_) => ReferenceKind::NamedImport,
                    ImportedName::Default => ReferenceKind::DefaultImport,
                    ImportedName::Namespace => ReferenceKind::NamespaceImport,
                    ImportedName::SideEffect => ReferenceKind::SideEffectImport,
                };

                // Match to specific export
                if let Some(export) = target_module
                    .exports
                    .iter_mut()
                    .find(|e| export_matches(&e.name, &sym.imported_name))
                {
                    export.references.push(SymbolReference {
                        from_file: source_id,
                        kind: ref_kind,
                    });
                }

                // Namespace imports: check if we can narrow to specific member accesses.
                // `import * as ns from './x'; ns.foo; ns.bar` → only mark foo, bar as used.
                // If the namespace variable is re-exported (`export { ns }`) or no member
                // accesses are found, conservatively mark ALL exports as used.
                if matches!(sym.imported_name, ImportedName::Namespace)
                    && !sym.local_name.is_empty()
                {
                    let local_name = &sym.local_name;
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
                    let is_re_exported_from_non_entry = source_mod
                        .map(|m| {
                            m.exports
                                .iter()
                                .any(|e| e.local_name.as_deref() == Some(local_name.as_str()))
                        })
                        .unwrap_or(false)
                        && !entry_point_ids.contains(&source_id);

                    // For entry point files with no member accesses, the namespace
                    // is purely re-exported for external use — don't mark all exports
                    // as used internally. The `export *` path handles individual tracking.
                    let is_entry_with_no_access =
                        accessed_members.is_empty() && entry_point_ids.contains(&source_id);

                    if !is_entry_with_no_access
                        && (accessed_members.is_empty() || is_re_exported_from_non_entry)
                    {
                        // Can't narrow — mark all exports as referenced (conservative)
                        for export in &mut target_module.exports {
                            if export.references.iter().all(|r| r.from_file != source_id) {
                                export.references.push(SymbolReference {
                                    from_file: source_id,
                                    kind: ReferenceKind::NamespaceImport,
                                });
                            }
                        }
                    } else {
                        // Narrow: only mark accessed members as referenced
                        for export in &mut target_module.exports {
                            let name_str = export.name.to_string();
                            if accessed_members.contains(&name_str)
                                && export.references.iter().all(|r| r.from_file != source_id)
                            {
                                export.references.push(SymbolReference {
                                    from_file: source_id,
                                    kind: ReferenceKind::NamespaceImport,
                                });
                            }
                        }
                    }
                } else if matches!(sym.imported_name, ImportedName::Namespace) {
                    // No local name available — mark all (conservative)
                    for export in &mut target_module.exports {
                        if export.references.iter().all(|r| r.from_file != source_id) {
                            export.references.push(SymbolReference {
                                from_file: source_id,
                                kind: ReferenceKind::NamespaceImport,
                            });
                        }
                    }
                }
            }
        }

        // Mark reachable modules via BFS from entry points
        let mut visited = FixedBitSet::with_capacity(total_capacity);
        let mut queue = VecDeque::new();

        for &ep_id in &entry_point_ids {
            if (ep_id.0 as usize) < total_capacity {
                visited.insert(ep_id.0 as usize);
                queue.push_back(ep_id);
            }
        }

        while let Some(file_id) = queue.pop_front() {
            if (file_id.0 as usize) >= modules.len() {
                continue;
            }
            let module = &modules[file_id.0 as usize];
            for edge in &all_edges[module.edge_range.clone()] {
                let target_idx = edge.target.0 as usize;
                if target_idx < total_capacity && !visited.contains(target_idx) {
                    visited.insert(target_idx);
                    queue.push_back(edge.target);
                }
            }
        }

        for (idx, module) in modules.iter_mut().enumerate() {
            module.is_reachable = visited.contains(idx);
        }

        let mut graph = Self {
            modules,
            edges: all_edges,
            package_usage,
            entry_points: entry_point_ids,
            reverse_deps,
            namespace_imported,
        };

        // Propagate references through re-export chains
        graph.resolve_re_export_chains();

        graph
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
        let mut existing_refs: HashSet<FileId> = HashSet::new();

        while changed && iteration < max_iterations {
            changed = false;
            iteration += 1;

            for &(barrel_id, source_id, ref imported_name, ref exported_name) in &re_export_info {
                let barrel_idx = barrel_id.0 as usize;
                let source_idx = source_id.0 as usize;

                if barrel_idx >= self.modules.len() || source_idx >= self.modules.len() {
                    continue;
                }

                // Find references to the re-exported name on the barrel module
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
                let target_exports: Vec<usize> = if imported_name == "*" {
                    // Star re-export: all exports in source are candidates
                    (0..source.exports.len()).collect()
                } else {
                    source
                        .exports
                        .iter()
                        .enumerate()
                        .filter(|(_, e)| e.name.to_string() == *imported_name)
                        .map(|(i, _)| i)
                        .collect()
                };

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

        if iteration >= max_iterations {
            tracing::warn!(
                iterations = max_iterations,
                "Re-export chain resolution hit iteration limit, some chains may be incomplete"
            );
        }
    }

    /// Total number of modules.
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    /// Total number of edges.
    pub fn edge_count(&self) -> usize {
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
    use crate::discover::{DiscoveredFile, EntryPoint, EntryPointSource, FileId};
    use crate::extract::{ExportName, ImportInfo, ImportedName};
    use crate::resolve::{ResolveResult, ResolvedImport, ResolvedModule, ResolvedReExport};
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
                    crate::extract::ExportInfo {
                        name: ExportName::Named("foo".to_string()),
                        local_name: Some("foo".to_string()),
                        is_type_only: false,
                        span: oxc_span::Span::new(0, 20),
                        members: vec![],
                    },
                    crate::extract::ExportInfo {
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
                exports: vec![crate::extract::ExportInfo {
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
                exports: vec![crate::extract::ExportInfo {
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
                exports: vec![crate::extract::ExportInfo {
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
                exports: vec![crate::extract::ExportInfo {
                    name: ExportName::Named("foo".to_string()),
                    local_name: Some("foo".to_string()),
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 20),
                    members: vec![],
                }],
                re_exports: vec![ResolvedReExport {
                    info: crate::extract::ReExportInfo {
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
                exports: vec![crate::extract::ExportInfo {
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
        // barrel.ts has `export { foo } from './source'`
        // The barrel should have an ExportSymbol for "foo" so references can attach.
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
            // barrel re-exports "foo" from source (no local exports)
            ResolvedModule {
                file_id: FileId(1),
                path: PathBuf::from("/project/barrel.ts"),
                exports: vec![], // barrel has NO local exports
                re_exports: vec![ResolvedReExport {
                    info: crate::extract::ReExportInfo {
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
                exports: vec![crate::extract::ExportInfo {
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

        // The barrel should have an ExportSymbol for "foo" created from its re-export
        let barrel = &graph.modules[1];
        let foo_export = barrel.exports.iter().find(|e| e.name.to_string() == "foo");
        assert!(
            foo_export.is_some(),
            "barrel should have ExportSymbol for re-exported 'foo'"
        );

        // The barrel's foo export should have a reference from entry.ts
        let foo = foo_export.unwrap();
        assert!(
            !foo.references.is_empty(),
            "barrel's foo should have a reference from entry.ts"
        );

        // The source module's foo should also have propagated references
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
        // barrel.ts re-exports both foo and bar from source
        // entry.ts only imports foo — bar should have no references on barrel
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
                        info: crate::extract::ReExportInfo {
                            source: "./source".to_string(),
                            imported_name: "foo".to_string(),
                            exported_name: "foo".to_string(),
                            is_type_only: false,
                        },
                        target: ResolveResult::InternalModule(FileId(2)),
                    },
                    ResolvedReExport {
                        info: crate::extract::ReExportInfo {
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
                    crate::extract::ExportInfo {
                        name: ExportName::Named("foo".to_string()),
                        local_name: Some("foo".to_string()),
                        is_type_only: false,
                        span: oxc_span::Span::new(0, 20),
                        members: vec![],
                    },
                    crate::extract::ExportInfo {
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
        // foo should be referenced, bar should NOT
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
        // barrel has: export type { FooType } from './source'
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
                        info: crate::extract::ReExportInfo {
                            source: "./source".to_string(),
                            imported_name: "UsedType".to_string(),
                            exported_name: "UsedType".to_string(),
                            is_type_only: true,
                        },
                        target: ResolveResult::InternalModule(FileId(2)),
                    },
                    ResolvedReExport {
                        info: crate::extract::ReExportInfo {
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
                    crate::extract::ExportInfo {
                        name: ExportName::Named("UsedType".to_string()),
                        local_name: Some("UsedType".to_string()),
                        is_type_only: true,
                        span: oxc_span::Span::new(0, 20),
                        members: vec![],
                    },
                    crate::extract::ExportInfo {
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

        // Both type re-exports should create type-only ExportSymbols
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
        // barrel has: export { default as Accordion } from './Accordion'
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
                    info: crate::extract::ReExportInfo {
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
                exports: vec![crate::extract::ExportInfo {
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

        // Barrel should have "Accordion" as an export
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

        // Source's default export should have propagated references
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
        // entry.ts -> barrel1.ts -re-exports-> barrel2.ts -re-exports-> source.ts
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
            // barrel1 re-exports foo from barrel2
            ResolvedModule {
                file_id: FileId(1),
                path: PathBuf::from("/project/barrel1.ts"),
                exports: vec![],
                re_exports: vec![ResolvedReExport {
                    info: crate::extract::ReExportInfo {
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
            // barrel2 re-exports foo from source
            ResolvedModule {
                file_id: FileId(2),
                path: PathBuf::from("/project/barrel2.ts"),
                exports: vec![],
                re_exports: vec![ResolvedReExport {
                    info: crate::extract::ReExportInfo {
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
                exports: vec![crate::extract::ExportInfo {
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

        // All modules in the chain should have foo referenced
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
}
