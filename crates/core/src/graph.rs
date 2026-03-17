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

        // Build path -> FileId index
        let path_to_id: HashMap<PathBuf, FileId> =
            files.iter().map(|f| (f.path.clone(), f.id)).collect();

        // Build FileId -> ResolvedModule index
        let module_by_id: HashMap<FileId, &ResolvedModule> =
            resolved_modules.iter().map(|m| (m.file_id, m)).collect();

        let mut all_edges = Vec::new();
        let mut modules = Vec::with_capacity(module_count);
        let mut package_usage: HashMap<String, Vec<FileId>> = HashMap::new();
        let mut reverse_deps = vec![Vec::new(); module_count];

        // Build entry point set
        let entry_point_ids: HashSet<FileId> = entry_points
            .iter()
            .filter_map(|ep| {
                ep.path
                    .canonicalize()
                    .ok()
                    .and_then(|c| {
                        files
                            .iter()
                            .find(|f| f.path.canonicalize().ok().as_ref() == Some(&c))
                            .map(|f| f.id)
                    })
                    .or_else(|| path_to_id.get(&ep.path).copied())
            })
            .collect();

        for file in files {
            let edge_start = all_edges.len();

            if let Some(resolved) = module_by_id.get(&file.id) {
                // Group imports by target
                let mut edges_by_target: HashMap<FileId, Vec<ImportedSymbol>> = HashMap::new();

                for import in &resolved.resolved_imports {
                    match &import.target {
                        ResolveResult::InternalModule(target_id) => {
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
                        edges_by_target
                            .entry(*target_id)
                            .or_default()
                            .push(ImportedSymbol {
                                imported_name: if re_export.info.imported_name == "*" {
                                    ImportedName::Namespace
                                } else {
                                    ImportedName::Named(re_export.info.imported_name.clone())
                                },
                                local_name: re_export.info.exported_name.clone(),
                            });
                    } else if let ResolveResult::NpmPackage(name) = &re_export.target {
                        package_usage.entry(name.clone()).or_default().push(file.id);
                    }
                }

                // Dynamic imports
                for import in &resolved.resolved_dynamic_imports {
                    if let ResolveResult::InternalModule(target_id) = &import.target {
                        edges_by_target
                            .entry(*target_id)
                            .or_default()
                            .push(ImportedSymbol {
                                imported_name: ImportedName::SideEffect,
                                local_name: String::new(),
                            });
                    }
                }

                for (target_id, symbols) in edges_by_target {
                    let is_side_effect = symbols
                        .iter()
                        .any(|s| matches!(s.imported_name, ImportedName::SideEffect));

                    all_edges.push(Edge {
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

            let exports = module_by_id
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

        // Populate export references from edges
        for edge in &all_edges {
            let source_file_id = modules
                .iter()
                .find(|m| {
                    m.edge_range
                        .contains(&(edge as *const _ as usize - all_edges.as_ptr() as usize))
                })
                .map(|m| m.file_id);

            // Find source file from reverse lookup
            if let Some(source_id) = find_edge_source(&modules, &all_edges, edge) {
                let target_module = &mut modules[edge.target.0 as usize];
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

                    // Namespace imports mark ALL exports as referenced
                    if matches!(sym.imported_name, ImportedName::Namespace) {
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

            let _ = source_file_id; // suppress warning
        }

        // Mark reachable modules via BFS from entry points
        let mut visited = FixedBitSet::with_capacity(module_count);
        let mut queue = VecDeque::new();

        for &ep_id in &entry_point_ids {
            if (ep_id.0 as usize) < module_count {
                visited.insert(ep_id.0 as usize);
                queue.push_back(ep_id);
            }
        }

        while let Some(file_id) = queue.pop_front() {
            let module = &modules[file_id.0 as usize];
            for edge in &all_edges[module.edge_range.clone()] {
                let target_idx = edge.target.0 as usize;
                if target_idx < module_count && !visited.contains(target_idx) {
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

        // For each re-export, if the barrel's exported symbol has references,
        // propagate those references to the source module's original export.
        // We iterate until no new references are added (handles chains).
        let mut changed = true;
        let max_iterations = 20; // prevent infinite loops on cycles
        let mut iteration = 0;

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
                    for ref_item in &refs_on_barrel {
                        let already_has = source.exports[export_idx]
                            .references
                            .iter()
                            .any(|r| r.from_file == ref_item.from_file);
                        if !already_has {
                            source.exports[export_idx].references.push(ref_item.clone());
                            changed = true;
                        }
                    }
                }
            }
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
    pub fn has_namespace_import(&self, file_id: FileId) -> bool {
        let idx = file_id.0 as usize;
        if idx >= self.reverse_deps.len() {
            return false;
        }

        for &importer_id in &self.reverse_deps[idx] {
            let importer = &self.modules[importer_id.0 as usize];
            for edge in &self.edges[importer.edge_range.clone()] {
                if edge.target == file_id {
                    for sym in &edge.symbols {
                        if matches!(sym.imported_name, ImportedName::Namespace) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

/// Find the source FileId for an edge by checking which module's edge_range contains it.
fn find_edge_source(modules: &[ModuleNode], all_edges: &[Edge], edge: &Edge) -> Option<FileId> {
    let edge_idx =
        (edge as *const Edge as usize - all_edges.as_ptr() as usize) / std::mem::size_of::<Edge>();

    modules
        .iter()
        .find(|m| m.edge_range.contains(&edge_idx))
        .map(|m| m.file_id)
}

/// Check if an export name matches an imported name.
fn export_matches(export: &ExportName, import: &ImportedName) -> bool {
    match (export, import) {
        (ExportName::Named(e), ImportedName::Named(i)) => e == i,
        (ExportName::Default, ImportedName::Default) => true,
        _ => false,
    }
}
