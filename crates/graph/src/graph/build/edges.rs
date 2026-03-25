//! Phase 1: Edge construction and module node building.
//!
//! Collects import edges from resolved modules, builds `ModuleNode` entries,
//! flat `Edge` storage, reverse dependency indices, and package usage maps.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::resolve::{ResolveResult, ResolvedImport, ResolvedModule};
use fallow_types::discover::{DiscoveredFile, FileId};
use fallow_types::extract::{ExportName, ImportedName};

use super::super::types::ModuleNode;
use super::super::types::{ExportSymbol, ReExportEdge};
use super::super::{Edge, ImportedSymbol, ModuleGraph};

/// Mutable accumulator state shared across all files during edge population.
pub(super) struct EdgeAccumulator {
    pub(super) package_usage: FxHashMap<String, Vec<FileId>>,
    pub(super) type_only_package_usage: FxHashMap<String, Vec<FileId>>,
    pub(super) namespace_imported: fixedbitset::FixedBitSet,
    pub(super) total_capacity: usize,
}

/// Insert into the namespace-imported bitset with bounds checking.
pub(super) fn record_namespace_import(
    target_id: FileId,
    namespace_imported: &mut fixedbitset::FixedBitSet,
    total_capacity: usize,
) {
    let idx = target_id.0 as usize;
    if idx < total_capacity {
        namespace_imported.insert(idx);
    }
}

/// Track that a file uses an npm package, and optionally record type-only usage.
pub(super) fn record_package_usage(
    acc: &mut EdgeAccumulator,
    name: &str,
    file_id: FileId,
    is_type_only: bool,
) {
    acc.package_usage
        .entry(name.to_owned())
        .or_default()
        .push(file_id);
    if is_type_only {
        acc.type_only_package_usage
            .entry(name.to_owned())
            .or_default()
            .push(file_id);
    }
}

/// Process a single resolved import (static or dynamic), adding it to the edge map.
///
/// Internal module imports create an `ImportedSymbol` entry grouped by target.
/// Namespace imports are also recorded in the namespace-imported bitset.
/// npm package imports are recorded in the package usage maps.
pub(super) fn collect_import_edge(
    import: &ResolvedImport,
    file_id: FileId,
    edges_by_target: &mut FxHashMap<FileId, Vec<ImportedSymbol>>,
    acc: &mut EdgeAccumulator,
) {
    match &import.target {
        ResolveResult::InternalModule(target_id) => {
            if matches!(import.info.imported_name, ImportedName::Namespace) {
                record_namespace_import(
                    *target_id,
                    &mut acc.namespace_imported,
                    acc.total_capacity,
                );
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
            record_package_usage(acc, name, file_id, import.info.is_type_only);
        }
        _ => {}
    }
}

/// Collect edges from a resolved module's static imports, re-exports, dynamic imports,
/// and dynamic import patterns into a grouped edge map.
///
/// Returns the grouped edges sorted by target `FileId` for deterministic ordering.
pub(super) fn collect_edges_for_module(
    resolved: &ResolvedModule,
    file_id: FileId,
    acc: &mut EdgeAccumulator,
) -> Vec<(FileId, Vec<ImportedSymbol>)> {
    let mut edges_by_target: FxHashMap<FileId, Vec<ImportedSymbol>> = FxHashMap::default();

    // Static imports
    for import in &resolved.resolved_imports {
        collect_import_edge(import, file_id, &mut edges_by_target, acc);
    }

    // Re-exports — use SideEffect edges to avoid marking source exports as "used"
    // just because they're re-exported. Re-export chain propagation handles tracking
    // which specific names consumers actually import.
    for re_export in &resolved.re_exports {
        if let ResolveResult::InternalModule(target_id) = &re_export.target {
            edges_by_target
                .entry(*target_id)
                .or_default()
                .push(ImportedSymbol {
                    imported_name: ImportedName::SideEffect,
                    local_name: String::new(),
                    import_span: oxc_span::Span::new(0, 0),
                });
        } else if let ResolveResult::NpmPackage(name) = &re_export.target {
            record_package_usage(acc, name, file_id, re_export.info.is_type_only);
        }
    }

    // Dynamic imports — Named imports create Named edges, Namespace imports create
    // Namespace edges with a local_name (enabling member access narrowing),
    // Side-effect imports create SideEffect edges.
    for import in &resolved.resolved_dynamic_imports {
        collect_import_edge(import, file_id, &mut edges_by_target, acc);
    }

    // Dynamic import patterns (template literals, string concat, import.meta.glob)
    for (_pattern, matched_ids) in &resolved.resolved_dynamic_patterns {
        for target_id in matched_ids {
            record_namespace_import(*target_id, &mut acc.namespace_imported, acc.total_capacity);
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
    let mut sorted: Vec<_> = edges_by_target.into_iter().collect();
    sorted.sort_by_key(|(target_id, _)| target_id.0);
    sorted
}

/// Build a `ModuleNode` for a file, including exports, re-export edges, and metadata.
fn build_module_node(
    file: &DiscoveredFile,
    module_by_id: &FxHashMap<FileId, &ResolvedModule>,
    entry_point_ids: &FxHashSet<FileId>,
    edge_range: std::ops::Range<usize>,
) -> ModuleNode {
    let mut exports: Vec<ExportSymbol> = module_by_id
        .get(&file.id)
        .map(|m| {
            m.exports
                .iter()
                .map(|e| ExportSymbol {
                    name: e.name.clone(),
                    is_type_only: e.is_type_only,
                    is_public: e.is_public,
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
                is_public: false,
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

    ModuleNode {
        file_id: file.id,
        path: file.path.clone(),
        edge_range,
        exports,
        re_exports: re_export_edges,
        is_entry_point: entry_point_ids.contains(&file.id),
        is_reachable: false,
        has_cjs_exports,
    }
}

impl ModuleGraph {
    /// Build flat edge storage from resolved modules.
    ///
    /// Creates `ModuleNode` entries, flat `Edge` storage, reverse dependency
    /// indices, package usage maps, and the namespace-imported bitset.
    pub(in super::super) fn populate_edges(
        files: &[DiscoveredFile],
        module_by_id: &FxHashMap<FileId, &ResolvedModule>,
        entry_point_ids: &FxHashSet<FileId>,
        module_count: usize,
        total_capacity: usize,
    ) -> Self {
        let mut all_edges = Vec::new();
        let mut modules = Vec::with_capacity(module_count);
        let mut reverse_deps = vec![Vec::new(); total_capacity];
        let mut acc = EdgeAccumulator {
            package_usage: FxHashMap::default(),
            type_only_package_usage: FxHashMap::default(),
            namespace_imported: fixedbitset::FixedBitSet::with_capacity(total_capacity),
            total_capacity,
        };

        for file in files {
            let edge_start = all_edges.len();

            if let Some(resolved) = module_by_id.get(&file.id) {
                let sorted_edges = collect_edges_for_module(resolved, file.id, &mut acc);

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

            modules.push(build_module_node(
                file,
                module_by_id,
                entry_point_ids,
                edge_start..edge_end,
            ));
        }

        Self {
            modules,
            edges: all_edges,
            package_usage: acc.package_usage,
            type_only_package_usage: acc.type_only_package_usage,
            entry_points: entry_point_ids.clone(),
            reverse_deps,
            namespace_imported: acc.namespace_imported,
        }
    }
}
