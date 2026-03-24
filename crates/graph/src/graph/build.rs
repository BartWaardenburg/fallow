//! Phase 1 (populate_edges) and Phase 2 (populate_references) of graph construction.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::resolve::{ResolveResult, ResolvedImport, ResolvedModule};
use fallow_types::discover::{DiscoveredFile, FileId};
use fallow_types::extract::{ExportName, ImportedName};

use super::types::ModuleNode;
use super::types::{ExportSymbol, ReExportEdge, ReferenceKind, SymbolReference};
use super::{Edge, ImportedSymbol, ModuleGraph};

/// Mutable accumulator state shared across all files during edge population.
struct EdgeAccumulator {
    package_usage: FxHashMap<String, Vec<FileId>>,
    type_only_package_usage: FxHashMap<String, Vec<FileId>>,
    namespace_imported: fixedbitset::FixedBitSet,
    total_capacity: usize,
}

/// Insert into the namespace-imported bitset with bounds checking.
fn record_namespace_import(
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
fn record_package_usage(acc: &mut EdgeAccumulator, name: &str, file_id: FileId, is_type_only: bool) {
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
fn collect_import_edge(
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
fn collect_edges_for_module(
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

/// Check whether an import binding is unused in the source file.
///
/// Returns `true` if the binding should be skipped (unused).
fn is_unused_import_binding(
    sym_local_name: &str,
    sym_imported_name: &ImportedName,
    source_mod: Option<&&ResolvedModule>,
) -> bool {
    !sym_local_name.is_empty()
        && !matches!(sym_imported_name, ImportedName::SideEffect)
        && source_mod.is_some_and(|m| m.unused_import_bindings.iter().any(|n| n == sym_local_name))
}

/// Extract member access names for a given local variable from a resolved module.
fn extract_accessed_members(source_mod: Option<&&ResolvedModule>, local_name: &str) -> Vec<String> {
    source_mod
        .map(|m| {
            m.member_accesses
                .iter()
                .filter(|ma| ma.object == local_name)
                .map(|ma| ma.member.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Mark all exports on a module as referenced by a given source file.
///
/// Deduplicates: skips exports already referenced by `source_id`.
fn mark_all_exports_referenced(
    exports: &mut Vec<ExportSymbol>,
    source_id: FileId,
    import_span: oxc_span::Span,
    kind: &ReferenceKind,
) {
    for export in exports {
        if export.references.iter().all(|r| r.from_file != source_id) {
            export.references.push(SymbolReference {
                from_file: source_id,
                kind: kind.clone(),
                import_span,
            });
        }
    }
}

/// Mark only exports whose names appear in `accessed_members` as referenced.
///
/// Returns the set of member names that were found among the exports.
fn mark_member_exports_referenced(
    exports: &mut [ExportSymbol],
    source_id: FileId,
    accessed_members: &[String],
    import_span: oxc_span::Span,
    kind: &ReferenceKind,
) -> FxHashSet<String> {
    let mut found_members: FxHashSet<String> = FxHashSet::default();
    for export in exports {
        let name_str = export.name.to_string();
        if accessed_members.contains(&name_str) {
            found_members.insert(name_str);
            if export.references.iter().all(|r| r.from_file != source_id) {
                export.references.push(SymbolReference {
                    from_file: source_id,
                    kind: kind.clone(),
                    import_span,
                });
            }
        }
    }
    found_members
}

/// Create synthetic `ExportSymbol` entries for members accessed via namespace import
/// that were not found among the target's own exports, but the target has `export *`
/// re-exports that may forward those names.
fn create_synthetic_exports_for_star_re_exports(
    exports: &mut Vec<ExportSymbol>,
    re_exports: &[ReExportEdge],
    source_id: FileId,
    accessed_members: &[String],
    found_members: &FxHashSet<String>,
    import_span: oxc_span::Span,
) {
    let has_star_re_exports = re_exports.iter().any(|re| re.exported_name == "*");
    if !has_star_re_exports {
        return;
    }
    for member in accessed_members {
        if found_members.contains(member) {
            continue;
        }
        let export_name = if member == "default" {
            ExportName::Default
        } else {
            ExportName::Named(member.clone())
        };
        exports.push(ExportSymbol {
            name: export_name,
            is_type_only: false,
            is_public: false,
            span: oxc_span::Span::new(0, 0),
            references: vec![SymbolReference {
                from_file: source_id,
                kind: ReferenceKind::NamespaceImport,
                import_span,
            }],
            members: Vec::new(),
        });
    }
}

/// Handle namespace import narrowing for `import * as ns from './x'`.
///
/// If member accesses can be determined, only those exports are marked as used.
/// Otherwise, all exports are conservatively marked as referenced.
fn narrow_namespace_references(
    module: &mut ModuleNode,
    source_id: FileId,
    sym_local_name: &str,
    sym_import_span: oxc_span::Span,
    module_by_id: &FxHashMap<FileId, &ResolvedModule>,
    entry_point_ids: &FxHashSet<FileId>,
) {
    let local_name = sym_local_name;
    let source_mod = module_by_id.get(&source_id);
    let accessed_members = extract_accessed_members(source_mod, local_name);

    // Check if the namespace is consumed as a whole object
    // (Object.values, for..in, spread, destructuring with rest, etc.)
    let is_whole_object =
        source_mod.is_some_and(|m| m.whole_object_uses.iter().any(|n| n == local_name));

    // Check if the namespace variable is re-exported (export { ns } or export default ns)
    // from a NON-entry-point file. If the importing file IS an entry point,
    // the re-export is for external consumption and doesn't prove internal usage.
    let is_re_exported_from_non_entry = source_mod.is_some_and(|m| {
        m.exports
            .iter()
            .any(|e| e.local_name.as_deref() == Some(local_name))
    }) && !entry_point_ids.contains(&source_id);

    // For entry point files with no member accesses, the namespace
    // is purely re-exported for external use — don't mark all exports
    // as used internally. The `export *` path handles individual tracking.
    let is_entry_with_no_access =
        accessed_members.is_empty() && !is_whole_object && entry_point_ids.contains(&source_id);

    if is_whole_object
        || (!is_entry_with_no_access
            && (accessed_members.is_empty() || is_re_exported_from_non_entry))
    {
        // Can't narrow — mark all exports as referenced (conservative)
        mark_all_exports_referenced(
            &mut module.exports,
            source_id,
            sym_import_span,
            &ReferenceKind::NamespaceImport,
        );
    } else {
        // Narrow: only mark accessed members as referenced
        let found_members = mark_member_exports_referenced(
            &mut module.exports,
            source_id,
            &accessed_members,
            sym_import_span,
            &ReferenceKind::NamespaceImport,
        );

        // For members not found on the target (e.g., barrel with
        // `export *` that has no own exports for these names),
        // create synthetic ExportSymbol entries so that
        // resolve_re_export_chains can propagate them to the
        // actual source modules.
        create_synthetic_exports_for_star_re_exports(
            &mut module.exports,
            &module.re_exports,
            source_id,
            &accessed_members,
            &found_members,
            sym_import_span,
        );
    }
}

/// Handle CSS Module default-import narrowing.
///
/// `import styles from './Button.module.css'` — member accesses like `styles.primary`
/// mark the `primary` named export as referenced, since CSS module default imports act
/// as namespace objects where each property corresponds to a class name (named export).
fn narrow_css_module_references(
    exports: &mut Vec<ExportSymbol>,
    source_id: FileId,
    sym_local_name: &str,
    sym_import_span: oxc_span::Span,
    module_by_id: &FxHashMap<FileId, &ResolvedModule>,
) {
    let local_name = sym_local_name;
    let source_mod = module_by_id.get(&source_id);
    let is_whole_object =
        source_mod.is_some_and(|m| m.whole_object_uses.iter().any(|n| n == local_name));
    let accessed_members = extract_accessed_members(source_mod, local_name);

    if is_whole_object || accessed_members.is_empty() {
        mark_all_exports_referenced(
            exports,
            source_id,
            sym_import_span,
            &ReferenceKind::DefaultImport,
        );
    } else {
        mark_member_exports_referenced(
            exports,
            source_id,
            &accessed_members,
            sym_import_span,
            &ReferenceKind::DefaultImport,
        );
    }
}

/// Determine the `ReferenceKind` for an imported name.
fn reference_kind_for(imported_name: &ImportedName) -> ReferenceKind {
    match imported_name {
        ImportedName::Named(_) => ReferenceKind::NamedImport,
        ImportedName::Default => ReferenceKind::DefaultImport,
        ImportedName::Namespace => ReferenceKind::NamespaceImport,
        ImportedName::SideEffect => ReferenceKind::SideEffectImport,
    }
}

/// Process a single imported symbol, attaching references to the target module's exports.
///
/// Handles: direct export matching, namespace import narrowing, and CSS module narrowing.
fn attach_symbol_reference(
    target_module: &mut ModuleNode,
    source_id: FileId,
    sym: &ImportedSymbol,
    module_by_id: &FxHashMap<FileId, &ResolvedModule>,
    entry_point_ids: &FxHashSet<FileId>,
) {
    let ref_kind = reference_kind_for(&sym.imported_name);

    // Skip references for import bindings that are never used in the importing file.
    if is_unused_import_binding(
        &sym.local_name,
        &sym.imported_name,
        module_by_id.get(&source_id),
    ) {
        return;
    }

    // Match to specific export
    if let Some(export) = target_module
        .exports
        .iter_mut()
        .find(|e| export_matches(&e.name, &sym.imported_name))
    {
        export.references.push(SymbolReference {
            from_file: source_id,
            kind: ref_kind,
            import_span: sym.import_span,
        });
    }

    // Namespace imports: narrow to specific member accesses when possible,
    // otherwise conservatively mark all exports as used.
    if matches!(sym.imported_name, ImportedName::Namespace) {
        if !sym.local_name.is_empty() {
            narrow_namespace_references(
                target_module,
                source_id,
                &sym.local_name,
                sym.import_span,
                module_by_id,
                entry_point_ids,
            );
        } else {
            // No local name available — mark all (conservative)
            mark_all_exports_referenced(
                &mut target_module.exports,
                source_id,
                sym.import_span,
                &ReferenceKind::NamespaceImport,
            );
        }
    }

    // CSS Module default imports: member accesses like `styles.primary` mark
    // the `primary` named export as referenced.
    if matches!(sym.imported_name, ImportedName::Default)
        && !sym.local_name.is_empty()
        && is_css_module_path(&target_module.path)
    {
        narrow_css_module_references(
            &mut target_module.exports,
            source_id,
            &sym.local_name,
            sym.import_span,
            module_by_id,
        );
    }
}

impl ModuleGraph {
    /// Build flat edge storage from resolved modules.
    ///
    /// Creates `ModuleNode` entries, flat `Edge` storage, reverse dependency
    /// indices, package usage maps, and the namespace-imported bitset.
    pub(super) fn populate_edges(
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

    /// Record which files reference which exports from edges.
    ///
    /// Walks every edge and attaches `SymbolReference` entries to the target
    /// module's exports. Includes namespace import narrowing (member access
    /// tracking) and CSS Module default-import narrowing.
    pub(super) fn populate_references(
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
                let sym = &self.edges[edge_idx].symbols[sym_idx];
                attach_symbol_reference(
                    &mut self.modules[target_idx],
                    source_id,
                    sym,
                    module_by_id,
                    entry_point_ids,
                );
            }
        }
    }
}

/// Check if a path is a CSS Module file (`.module.css` or `.module.scss`).
pub(super) fn is_css_module_path(path: &std::path::Path) -> bool {
    path.file_stem()
        .and_then(|s| s.to_str())
        .is_some_and(|stem| stem.ends_with(".module"))
        && path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| ext == "css" || ext == "scss")
}

/// Check if an export name matches an imported name.
pub(super) fn export_matches(export: &ExportName, import: &ImportedName) -> bool {
    match (export, import) {
        (ExportName::Named(e), ImportedName::Named(i)) => e == i,
        (ExportName::Default, ImportedName::Default) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    // ── reference_kind_for ──────────────────────────────────────────────

    #[test]
    fn reference_kind_for_named() {
        assert_eq!(
            reference_kind_for(&ImportedName::Named("x".to_string())),
            ReferenceKind::NamedImport,
        );
    }

    #[test]
    fn reference_kind_for_default() {
        assert_eq!(
            reference_kind_for(&ImportedName::Default),
            ReferenceKind::DefaultImport,
        );
    }

    #[test]
    fn reference_kind_for_namespace() {
        assert_eq!(
            reference_kind_for(&ImportedName::Namespace),
            ReferenceKind::NamespaceImport,
        );
    }

    #[test]
    fn reference_kind_for_side_effect() {
        assert_eq!(
            reference_kind_for(&ImportedName::SideEffect),
            ReferenceKind::SideEffectImport,
        );
    }

    // ── is_css_module_path ──────────────────────────────────────────────

    #[test]
    fn css_module_path_css() {
        assert!(is_css_module_path(std::path::Path::new(
            "Button.module.css"
        )));
    }

    #[test]
    fn css_module_path_scss() {
        assert!(is_css_module_path(std::path::Path::new(
            "Button.module.scss"
        )));
    }

    #[test]
    fn css_module_path_plain_css() {
        assert!(!is_css_module_path(std::path::Path::new("Button.css")));
    }

    #[test]
    fn css_module_path_ts() {
        assert!(!is_css_module_path(std::path::Path::new("Button.module.ts")));
    }

    // ── record_namespace_import ─────────────────────────────────────────

    #[test]
    fn record_namespace_import_within_bounds() {
        let mut bitset = fixedbitset::FixedBitSet::with_capacity(4);
        record_namespace_import(FileId(2), &mut bitset, 4);
        assert!(bitset.contains(2));
    }

    #[test]
    fn record_namespace_import_out_of_bounds() {
        let mut bitset = fixedbitset::FixedBitSet::with_capacity(4);
        record_namespace_import(FileId(10), &mut bitset, 4);
        // Should silently skip — bitset unchanged
        assert!(!bitset.contains(3));
    }

    // ── record_package_usage ────────────────────────────────────────────

    #[test]
    fn record_package_usage_non_type_only() {
        let mut acc = EdgeAccumulator {
            package_usage: FxHashMap::default(),
            type_only_package_usage: FxHashMap::default(),
            namespace_imported: fixedbitset::FixedBitSet::with_capacity(4),
            total_capacity: 4,
        };
        record_package_usage(&mut acc, "react", FileId(0), false);
        assert_eq!(acc.package_usage["react"], vec![FileId(0)]);
        assert!(!acc.type_only_package_usage.contains_key("react"));
    }

    #[test]
    fn record_package_usage_type_only() {
        let mut acc = EdgeAccumulator {
            package_usage: FxHashMap::default(),
            type_only_package_usage: FxHashMap::default(),
            namespace_imported: fixedbitset::FixedBitSet::with_capacity(4),
            total_capacity: 4,
        };
        record_package_usage(&mut acc, "react", FileId(1), true);
        assert_eq!(acc.package_usage["react"], vec![FileId(1)]);
        assert_eq!(acc.type_only_package_usage["react"], vec![FileId(1)]);
    }

    #[test]
    fn record_package_usage_multiple_files() {
        let mut acc = EdgeAccumulator {
            package_usage: FxHashMap::default(),
            type_only_package_usage: FxHashMap::default(),
            namespace_imported: fixedbitset::FixedBitSet::with_capacity(4),
            total_capacity: 4,
        };
        record_package_usage(&mut acc, "lodash", FileId(0), false);
        record_package_usage(&mut acc, "lodash", FileId(1), true);
        assert_eq!(acc.package_usage["lodash"], vec![FileId(0), FileId(1)]);
        assert_eq!(acc.type_only_package_usage["lodash"], vec![FileId(1)]);
    }

    // ── collect_import_edge ─────────────────────────────────────────────

    fn make_acc(cap: usize) -> EdgeAccumulator {
        EdgeAccumulator {
            package_usage: FxHashMap::default(),
            type_only_package_usage: FxHashMap::default(),
            namespace_imported: fixedbitset::FixedBitSet::with_capacity(cap),
            total_capacity: cap,
        }
    }

    fn make_import(imported_name: ImportedName, target: ResolveResult) -> ResolvedImport {
        ResolvedImport {
            info: fallow_types::extract::ImportInfo {
                source: "./target".to_string(),
                imported_name,
                local_name: "localVar".to_string(),
                is_type_only: false,
                span: oxc_span::Span::new(0, 10),
            },
            target,
        }
    }

    #[test]
    fn collect_import_edge_named_internal() {
        let mut acc = make_acc(4);
        let mut edges: FxHashMap<FileId, Vec<ImportedSymbol>> = FxHashMap::default();
        let import = make_import(
            ImportedName::Named("foo".to_string()),
            ResolveResult::InternalModule(FileId(2)),
        );
        collect_import_edge(&import, FileId(0), &mut edges, &mut acc);

        assert_eq!(edges.len(), 1);
        assert_eq!(edges[&FileId(2)].len(), 1);
        assert!(matches!(
            edges[&FileId(2)][0].imported_name,
            ImportedName::Named(ref n) if n == "foo"
        ));
        assert!(!acc.namespace_imported.contains(2));
    }

    #[test]
    fn collect_import_edge_default_internal() {
        let mut acc = make_acc(4);
        let mut edges: FxHashMap<FileId, Vec<ImportedSymbol>> = FxHashMap::default();
        let import = make_import(
            ImportedName::Default,
            ResolveResult::InternalModule(FileId(1)),
        );
        collect_import_edge(&import, FileId(0), &mut edges, &mut acc);

        assert_eq!(edges[&FileId(1)].len(), 1);
        assert!(matches!(
            edges[&FileId(1)][0].imported_name,
            ImportedName::Default
        ));
    }

    #[test]
    fn collect_import_edge_namespace_sets_bitset() {
        let mut acc = make_acc(4);
        let mut edges: FxHashMap<FileId, Vec<ImportedSymbol>> = FxHashMap::default();
        let import = make_import(
            ImportedName::Namespace,
            ResolveResult::InternalModule(FileId(3)),
        );
        collect_import_edge(&import, FileId(0), &mut edges, &mut acc);

        assert!(acc.namespace_imported.contains(3));
        assert_eq!(edges[&FileId(3)].len(), 1);
    }

    #[test]
    fn collect_import_edge_side_effect_internal() {
        let mut acc = make_acc(4);
        let mut edges: FxHashMap<FileId, Vec<ImportedSymbol>> = FxHashMap::default();
        let import = make_import(
            ImportedName::SideEffect,
            ResolveResult::InternalModule(FileId(1)),
        );
        collect_import_edge(&import, FileId(0), &mut edges, &mut acc);

        assert_eq!(edges[&FileId(1)].len(), 1);
        assert!(matches!(
            edges[&FileId(1)][0].imported_name,
            ImportedName::SideEffect
        ));
        // Side-effect should NOT set namespace bitset
        assert!(!acc.namespace_imported.contains(1));
    }

    #[test]
    fn collect_import_edge_npm_package() {
        let mut acc = make_acc(4);
        let mut edges: FxHashMap<FileId, Vec<ImportedSymbol>> = FxHashMap::default();
        let import = make_import(
            ImportedName::Named("merge".to_string()),
            ResolveResult::NpmPackage("lodash".to_string()),
        );
        collect_import_edge(&import, FileId(0), &mut edges, &mut acc);

        assert!(edges.is_empty(), "npm packages should not create edges");
        assert_eq!(acc.package_usage["lodash"], vec![FileId(0)]);
    }

    #[test]
    fn collect_import_edge_npm_type_only() {
        let mut acc = make_acc(4);
        let mut edges: FxHashMap<FileId, Vec<ImportedSymbol>> = FxHashMap::default();
        let import = ResolvedImport {
            info: fallow_types::extract::ImportInfo {
                source: "react".to_string(),
                imported_name: ImportedName::Named("FC".to_string()),
                local_name: "FC".to_string(),
                is_type_only: true,
                span: oxc_span::Span::new(0, 10),
            },
            target: ResolveResult::NpmPackage("react".to_string()),
        };
        collect_import_edge(&import, FileId(0), &mut edges, &mut acc);

        assert_eq!(acc.package_usage["react"], vec![FileId(0)]);
        assert_eq!(acc.type_only_package_usage["react"], vec![FileId(0)]);
    }

    #[test]
    fn collect_import_edge_external_file_ignored() {
        let mut acc = make_acc(4);
        let mut edges: FxHashMap<FileId, Vec<ImportedSymbol>> = FxHashMap::default();
        let import = make_import(
            ImportedName::Named("x".to_string()),
            ResolveResult::ExternalFile(std::path::PathBuf::from("/node_modules/foo/index.js")),
        );
        collect_import_edge(&import, FileId(0), &mut edges, &mut acc);

        assert!(edges.is_empty());
        assert!(acc.package_usage.is_empty());
    }

    #[test]
    fn collect_import_edge_unresolvable_ignored() {
        let mut acc = make_acc(4);
        let mut edges: FxHashMap<FileId, Vec<ImportedSymbol>> = FxHashMap::default();
        let import = make_import(
            ImportedName::Named("x".to_string()),
            ResolveResult::Unresolvable("./missing".to_string()),
        );
        collect_import_edge(&import, FileId(0), &mut edges, &mut acc);

        assert!(edges.is_empty());
    }

    // ── collect_edges_for_module ─────────────────────────────────────────

    #[test]
    fn collect_edges_sorted_by_target_id() {
        let resolved = ResolvedModule {
            file_id: FileId(0),
            path: std::path::PathBuf::from("/project/entry.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![
                ResolvedImport {
                    info: fallow_types::extract::ImportInfo {
                        source: "./c".to_string(),
                        imported_name: ImportedName::Named("c".to_string()),
                        local_name: "c".to_string(),
                        is_type_only: false,
                        span: oxc_span::Span::new(0, 5),
                    },
                    target: ResolveResult::InternalModule(FileId(3)),
                },
                ResolvedImport {
                    info: fallow_types::extract::ImportInfo {
                        source: "./a".to_string(),
                        imported_name: ImportedName::Named("a".to_string()),
                        local_name: "a".to_string(),
                        is_type_only: false,
                        span: oxc_span::Span::new(10, 15),
                    },
                    target: ResolveResult::InternalModule(FileId(1)),
                },
            ],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        };
        let mut acc = make_acc(4);
        let sorted = collect_edges_for_module(&resolved, FileId(0), &mut acc);

        // Should be sorted: FileId(1) before FileId(3)
        assert_eq!(sorted.len(), 2);
        assert_eq!(sorted[0].0, FileId(1));
        assert_eq!(sorted[1].0, FileId(3));
    }

    #[test]
    fn collect_edges_re_exports_use_side_effect() {
        let resolved = ResolvedModule {
            file_id: FileId(0),
            path: std::path::PathBuf::from("/project/barrel.ts"),
            exports: vec![],
            re_exports: vec![crate::resolve::ResolvedReExport {
                info: fallow_types::extract::ReExportInfo {
                    source: "./utils".to_string(),
                    imported_name: "foo".to_string(),
                    exported_name: "foo".to_string(),
                    is_type_only: false,
                },
                target: ResolveResult::InternalModule(FileId(1)),
            }],
            resolved_imports: vec![],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        };
        let mut acc = make_acc(4);
        let sorted = collect_edges_for_module(&resolved, FileId(0), &mut acc);

        assert_eq!(sorted.len(), 1);
        assert_eq!(sorted[0].0, FileId(1));
        assert!(matches!(
            sorted[0].1[0].imported_name,
            ImportedName::SideEffect
        ));
    }

    #[test]
    fn collect_edges_re_export_npm_records_usage() {
        let resolved = ResolvedModule {
            file_id: FileId(0),
            path: std::path::PathBuf::from("/project/barrel.ts"),
            exports: vec![],
            re_exports: vec![crate::resolve::ResolvedReExport {
                info: fallow_types::extract::ReExportInfo {
                    source: "react".to_string(),
                    imported_name: "useState".to_string(),
                    exported_name: "useState".to_string(),
                    is_type_only: false,
                },
                target: ResolveResult::NpmPackage("react".to_string()),
            }],
            resolved_imports: vec![],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        };
        let mut acc = make_acc(4);
        let sorted = collect_edges_for_module(&resolved, FileId(0), &mut acc);

        assert!(sorted.is_empty(), "npm re-exports should not create edges");
        assert_eq!(acc.package_usage["react"], vec![FileId(0)]);
    }

    #[test]
    fn collect_edges_dynamic_patterns_set_namespace() {
        let pattern = fallow_types::extract::DynamicImportPattern {
            prefix: "./locales/".to_string(),
            suffix: Some(".json".to_string()),
            span: oxc_span::Span::new(0, 10),
        };
        let resolved = ResolvedModule {
            file_id: FileId(0),
            path: std::path::PathBuf::from("/project/i18n.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![(pattern, vec![FileId(1), FileId(2)])],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        };
        let mut acc = make_acc(4);
        let sorted = collect_edges_for_module(&resolved, FileId(0), &mut acc);

        assert_eq!(sorted.len(), 2);
        assert!(acc.namespace_imported.contains(1));
        assert!(acc.namespace_imported.contains(2));
    }

    // ── is_unused_import_binding ────────────────────────────────────────

    #[test]
    fn is_unused_binding_true() {
        let resolved = ResolvedModule {
            file_id: FileId(0),
            path: std::path::PathBuf::from("/project/entry.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec!["unusedVar".to_string()],
        };
        assert!(is_unused_import_binding(
            "unusedVar",
            &ImportedName::Named("x".to_string()),
            Some(&&resolved),
        ));
    }

    #[test]
    fn is_unused_binding_false_when_used() {
        let resolved = ResolvedModule {
            file_id: FileId(0),
            path: std::path::PathBuf::from("/project/entry.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec!["otherVar".to_string()],
        };
        assert!(!is_unused_import_binding(
            "usedVar",
            &ImportedName::Named("x".to_string()),
            Some(&&resolved),
        ));
    }

    #[test]
    fn is_unused_binding_false_for_side_effect() {
        let resolved = ResolvedModule {
            file_id: FileId(0),
            path: std::path::PathBuf::from("/project/entry.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec!["x".to_string()],
        };
        // SideEffect imports are never "unused bindings"
        assert!(!is_unused_import_binding(
            "x",
            &ImportedName::SideEffect,
            Some(&&resolved),
        ));
    }

    #[test]
    fn is_unused_binding_false_for_empty_local_name() {
        let resolved = ResolvedModule {
            file_id: FileId(0),
            path: std::path::PathBuf::from("/project/entry.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        };
        assert!(!is_unused_import_binding(
            "",
            &ImportedName::Named("x".to_string()),
            Some(&&resolved),
        ));
    }

    #[test]
    fn is_unused_binding_false_for_no_source_module() {
        assert!(!is_unused_import_binding(
            "x",
            &ImportedName::Named("x".to_string()),
            None,
        ));
    }

    // ── extract_accessed_members ─────────────────────────────────────────

    #[test]
    fn extract_accessed_members_found() {
        let resolved = ResolvedModule {
            file_id: FileId(0),
            path: std::path::PathBuf::from("/project/entry.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![
                fallow_types::extract::MemberAccess {
                    object: "ns".to_string(),
                    member: "foo".to_string(),
                },
                fallow_types::extract::MemberAccess {
                    object: "ns".to_string(),
                    member: "bar".to_string(),
                },
                fallow_types::extract::MemberAccess {
                    object: "other".to_string(),
                    member: "baz".to_string(),
                },
            ],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        };
        let members = extract_accessed_members(Some(&&resolved), "ns");
        assert_eq!(members, vec!["foo".to_string(), "bar".to_string()]);
    }

    #[test]
    fn extract_accessed_members_none_module() {
        let members = extract_accessed_members(None, "ns");
        assert!(members.is_empty());
    }

    // ── mark_all_exports_referenced ─────────────────────────────────────

    #[test]
    fn mark_all_exports_referenced_adds_refs() {
        let mut exports = vec![
            ExportSymbol {
                name: ExportName::Named("a".to_string()),
                is_type_only: false,
                is_public: false,
                span: oxc_span::Span::new(0, 5),
                references: Vec::new(),
                members: Vec::new(),
            },
            ExportSymbol {
                name: ExportName::Named("b".to_string()),
                is_type_only: false,
                is_public: false,
                span: oxc_span::Span::new(10, 15),
                references: Vec::new(),
                members: Vec::new(),
            },
        ];
        mark_all_exports_referenced(
            &mut exports,
            FileId(5),
            oxc_span::Span::new(0, 10),
            &ReferenceKind::NamespaceImport,
        );
        assert_eq!(exports[0].references.len(), 1);
        assert_eq!(exports[0].references[0].from_file, FileId(5));
        assert_eq!(exports[1].references.len(), 1);
    }

    #[test]
    fn mark_all_exports_referenced_deduplicates() {
        let mut exports = vec![ExportSymbol {
            name: ExportName::Named("a".to_string()),
            is_type_only: false,
            is_public: false,
            span: oxc_span::Span::new(0, 5),
            references: vec![SymbolReference {
                from_file: FileId(5),
                kind: ReferenceKind::NamedImport,
                import_span: oxc_span::Span::new(0, 10),
            }],
            members: Vec::new(),
        }];
        // Same source file — should not add a duplicate
        mark_all_exports_referenced(
            &mut exports,
            FileId(5),
            oxc_span::Span::new(0, 10),
            &ReferenceKind::NamespaceImport,
        );
        assert_eq!(exports[0].references.len(), 1);
    }

    // ── mark_member_exports_referenced ──────────────────────────────────

    #[test]
    fn mark_member_exports_referenced_only_accessed() {
        let mut exports = vec![
            ExportSymbol {
                name: ExportName::Named("foo".to_string()),
                is_type_only: false,
                is_public: false,
                span: oxc_span::Span::new(0, 5),
                references: Vec::new(),
                members: Vec::new(),
            },
            ExportSymbol {
                name: ExportName::Named("bar".to_string()),
                is_type_only: false,
                is_public: false,
                span: oxc_span::Span::new(10, 15),
                references: Vec::new(),
                members: Vec::new(),
            },
        ];
        let accessed = vec!["foo".to_string()];
        let found = mark_member_exports_referenced(
            &mut exports,
            FileId(0),
            &accessed,
            oxc_span::Span::new(0, 10),
            &ReferenceKind::NamespaceImport,
        );

        assert_eq!(exports[0].references.len(), 1);
        assert!(exports[1].references.is_empty());
        assert!(found.contains("foo"));
        assert!(!found.contains("bar"));
    }

    // ── create_synthetic_exports_for_star_re_exports ────────────────────

    #[test]
    fn create_synthetic_exports_with_star_re_export() {
        let mut exports = vec![ExportSymbol {
            name: ExportName::Named("existing".to_string()),
            is_type_only: false,
            is_public: false,
            span: oxc_span::Span::new(0, 5),
            references: Vec::new(),
            members: Vec::new(),
        }];
        let re_exports = vec![ReExportEdge {
            source_file: FileId(2),
            imported_name: "*".to_string(),
            exported_name: "*".to_string(),
            is_type_only: false,
        }];
        let accessed = vec!["missing".to_string()];
        let found = FxHashSet::default(); // nothing found among own exports

        create_synthetic_exports_for_star_re_exports(
            &mut exports,
            &re_exports,
            FileId(0),
            &accessed,
            &found,
            oxc_span::Span::new(0, 10),
        );

        assert_eq!(exports.len(), 2);
        assert_eq!(
            exports[1].name,
            ExportName::Named("missing".to_string())
        );
        assert_eq!(exports[1].references.len(), 1);
    }

    #[test]
    fn create_synthetic_exports_skips_already_found() {
        let mut exports = Vec::new();
        let re_exports = vec![ReExportEdge {
            source_file: FileId(2),
            imported_name: "*".to_string(),
            exported_name: "*".to_string(),
            is_type_only: false,
        }];
        let accessed = vec!["already".to_string()];
        let mut found = FxHashSet::default();
        found.insert("already".to_string());

        create_synthetic_exports_for_star_re_exports(
            &mut exports,
            &re_exports,
            FileId(0),
            &accessed,
            &found,
            oxc_span::Span::new(0, 10),
        );

        assert!(
            exports.is_empty(),
            "should not create synthetic for already-found members"
        );
    }

    #[test]
    fn create_synthetic_exports_no_star_re_exports() {
        let mut exports = Vec::new();
        let re_exports = vec![ReExportEdge {
            source_file: FileId(2),
            imported_name: "foo".to_string(),
            exported_name: "foo".to_string(),
            is_type_only: false,
        }];
        let accessed = vec!["missing".to_string()];
        let found = FxHashSet::default();

        create_synthetic_exports_for_star_re_exports(
            &mut exports,
            &re_exports,
            FileId(0),
            &accessed,
            &found,
            oxc_span::Span::new(0, 10),
        );

        assert!(
            exports.is_empty(),
            "should not create synthetic without star re-exports"
        );
    }

    // ── attach_symbol_reference (integration-level, through public build) ──

    #[test]
    fn attach_ref_skips_unused_binding() {
        // entry imports "foo" from utils, but "foo" is in unused_import_bindings
        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: std::path::PathBuf::from("/project/entry.ts"),
                size_bytes: 100,
            },
            DiscoveredFile {
                id: FileId(1),
                path: std::path::PathBuf::from("/project/utils.ts"),
                size_bytes: 50,
            },
        ];
        let entry_points = vec![fallow_types::discover::EntryPoint {
            path: std::path::PathBuf::from("/project/entry.ts"),
            source: fallow_types::discover::EntryPointSource::PackageJsonMain,
        }];
        let resolved_modules = vec![
            ResolvedModule {
                file_id: FileId(0),
                path: std::path::PathBuf::from("/project/entry.ts"),
                exports: vec![],
                re_exports: vec![],
                resolved_imports: vec![ResolvedImport {
                    info: fallow_types::extract::ImportInfo {
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
                unused_import_bindings: vec!["foo".to_string()],
            },
            ResolvedModule {
                file_id: FileId(1),
                path: std::path::PathBuf::from("/project/utils.ts"),
                exports: vec![fallow_types::extract::ExportInfo {
                    name: ExportName::Named("foo".to_string()),
                    local_name: Some("foo".to_string()),
                    is_type_only: false,
                    is_public: false,
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
                unused_import_bindings: vec![],
            },
        ];
        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);
        let foo_export = graph.modules[1]
            .exports
            .iter()
            .find(|e| e.name.to_string() == "foo")
            .unwrap();
        assert!(
            foo_export.references.is_empty(),
            "unused binding should not create a reference"
        );
    }

    #[test]
    fn attach_ref_namespace_narrows_to_member_accesses() {
        // entry.ts: import * as utils from './utils'; uses utils.foo, not utils.bar
        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: std::path::PathBuf::from("/project/entry.ts"),
                size_bytes: 100,
            },
            DiscoveredFile {
                id: FileId(1),
                path: std::path::PathBuf::from("/project/utils.ts"),
                size_bytes: 50,
            },
        ];
        let entry_points = vec![fallow_types::discover::EntryPoint {
            path: std::path::PathBuf::from("/project/entry.ts"),
            source: fallow_types::discover::EntryPointSource::PackageJsonMain,
        }];
        let resolved_modules = vec![
            ResolvedModule {
                file_id: FileId(0),
                path: std::path::PathBuf::from("/project/entry.ts"),
                exports: vec![],
                re_exports: vec![],
                resolved_imports: vec![ResolvedImport {
                    info: fallow_types::extract::ImportInfo {
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
                member_accesses: vec![fallow_types::extract::MemberAccess {
                    object: "utils".to_string(),
                    member: "foo".to_string(),
                }],
                whole_object_uses: vec![],
                has_cjs_exports: false,
                unused_import_bindings: vec![],
            },
            ResolvedModule {
                file_id: FileId(1),
                path: std::path::PathBuf::from("/project/utils.ts"),
                exports: vec![
                    fallow_types::extract::ExportInfo {
                        name: ExportName::Named("foo".to_string()),
                        local_name: Some("foo".to_string()),
                        is_type_only: false,
                        is_public: false,
                        span: oxc_span::Span::new(0, 20),
                        members: vec![],
                    },
                    fallow_types::extract::ExportInfo {
                        name: ExportName::Named("bar".to_string()),
                        local_name: Some("bar".to_string()),
                        is_type_only: false,
                        is_public: false,
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
                unused_import_bindings: vec![],
            },
        ];
        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);

        let foo_export = graph.modules[1]
            .exports
            .iter()
            .find(|e| e.name.to_string() == "foo")
            .unwrap();
        assert!(
            !foo_export.references.is_empty(),
            "foo should be referenced via namespace narrowing"
        );

        let bar_export = graph.modules[1]
            .exports
            .iter()
            .find(|e| e.name.to_string() == "bar")
            .unwrap();
        assert!(
            bar_export.references.is_empty(),
            "bar should not be referenced when only foo is accessed"
        );
    }

    #[test]
    fn attach_ref_namespace_whole_object_marks_all() {
        // entry.ts: import * as utils from './utils'; Object.values(utils)
        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: std::path::PathBuf::from("/project/entry.ts"),
                size_bytes: 100,
            },
            DiscoveredFile {
                id: FileId(1),
                path: std::path::PathBuf::from("/project/utils.ts"),
                size_bytes: 50,
            },
        ];
        let entry_points = vec![fallow_types::discover::EntryPoint {
            path: std::path::PathBuf::from("/project/entry.ts"),
            source: fallow_types::discover::EntryPointSource::PackageJsonMain,
        }];
        let resolved_modules = vec![
            ResolvedModule {
                file_id: FileId(0),
                path: std::path::PathBuf::from("/project/entry.ts"),
                exports: vec![],
                re_exports: vec![],
                resolved_imports: vec![ResolvedImport {
                    info: fallow_types::extract::ImportInfo {
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
                whole_object_uses: vec!["utils".to_string()],
                has_cjs_exports: false,
                unused_import_bindings: vec![],
            },
            ResolvedModule {
                file_id: FileId(1),
                path: std::path::PathBuf::from("/project/utils.ts"),
                exports: vec![
                    fallow_types::extract::ExportInfo {
                        name: ExportName::Named("foo".to_string()),
                        local_name: Some("foo".to_string()),
                        is_type_only: false,
                        is_public: false,
                        span: oxc_span::Span::new(0, 20),
                        members: vec![],
                    },
                    fallow_types::extract::ExportInfo {
                        name: ExportName::Named("bar".to_string()),
                        local_name: Some("bar".to_string()),
                        is_type_only: false,
                        is_public: false,
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
                unused_import_bindings: vec![],
            },
        ];
        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);

        // Both exports should be referenced because the namespace is used as whole object
        for export in &graph.modules[1].exports {
            assert!(
                !export.references.is_empty(),
                "{} should be referenced when namespace is used as whole object",
                export.name
            );
        }
    }

    #[test]
    fn attach_ref_css_module_narrows_to_member_accesses() {
        // entry.ts: import styles from './Button.module.css'; uses styles.primary
        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: std::path::PathBuf::from("/project/entry.ts"),
                size_bytes: 100,
            },
            DiscoveredFile {
                id: FileId(1),
                path: std::path::PathBuf::from("/project/Button.module.css"),
                size_bytes: 50,
            },
        ];
        let entry_points = vec![fallow_types::discover::EntryPoint {
            path: std::path::PathBuf::from("/project/entry.ts"),
            source: fallow_types::discover::EntryPointSource::PackageJsonMain,
        }];
        let resolved_modules = vec![
            ResolvedModule {
                file_id: FileId(0),
                path: std::path::PathBuf::from("/project/entry.ts"),
                exports: vec![],
                re_exports: vec![],
                resolved_imports: vec![ResolvedImport {
                    info: fallow_types::extract::ImportInfo {
                        source: "./Button.module.css".to_string(),
                        imported_name: ImportedName::Default,
                        local_name: "styles".to_string(),
                        is_type_only: false,
                        span: oxc_span::Span::new(0, 10),
                    },
                    target: ResolveResult::InternalModule(FileId(1)),
                }],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![fallow_types::extract::MemberAccess {
                    object: "styles".to_string(),
                    member: "primary".to_string(),
                }],
                whole_object_uses: vec![],
                has_cjs_exports: false,
                unused_import_bindings: vec![],
            },
            ResolvedModule {
                file_id: FileId(1),
                path: std::path::PathBuf::from("/project/Button.module.css"),
                exports: vec![
                    fallow_types::extract::ExportInfo {
                        name: ExportName::Named("primary".to_string()),
                        local_name: Some("primary".to_string()),
                        is_type_only: false,
                        is_public: false,
                        span: oxc_span::Span::new(0, 20),
                        members: vec![],
                    },
                    fallow_types::extract::ExportInfo {
                        name: ExportName::Named("secondary".to_string()),
                        local_name: Some("secondary".to_string()),
                        is_type_only: false,
                        is_public: false,
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
                unused_import_bindings: vec![],
            },
        ];
        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);

        let primary = graph.modules[1]
            .exports
            .iter()
            .find(|e| e.name.to_string() == "primary")
            .unwrap();
        assert!(
            !primary.references.is_empty(),
            "primary should be referenced via CSS module narrowing"
        );

        let secondary = graph.modules[1]
            .exports
            .iter()
            .find(|e| e.name.to_string() == "secondary")
            .unwrap();
        assert!(
            secondary.references.is_empty(),
            "secondary should not be referenced — only primary is accessed"
        );
    }

    #[test]
    fn attach_ref_default_import_creates_reference() {
        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: std::path::PathBuf::from("/project/entry.ts"),
                size_bytes: 100,
            },
            DiscoveredFile {
                id: FileId(1),
                path: std::path::PathBuf::from("/project/component.ts"),
                size_bytes: 50,
            },
        ];
        let entry_points = vec![fallow_types::discover::EntryPoint {
            path: std::path::PathBuf::from("/project/entry.ts"),
            source: fallow_types::discover::EntryPointSource::PackageJsonMain,
        }];
        let resolved_modules = vec![
            ResolvedModule {
                file_id: FileId(0),
                path: std::path::PathBuf::from("/project/entry.ts"),
                exports: vec![],
                re_exports: vec![],
                resolved_imports: vec![ResolvedImport {
                    info: fallow_types::extract::ImportInfo {
                        source: "./component".to_string(),
                        imported_name: ImportedName::Default,
                        local_name: "Component".to_string(),
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
                unused_import_bindings: vec![],
            },
            ResolvedModule {
                file_id: FileId(1),
                path: std::path::PathBuf::from("/project/component.ts"),
                exports: vec![fallow_types::extract::ExportInfo {
                    name: ExportName::Default,
                    local_name: Some("Component".to_string()),
                    is_type_only: false,
                    is_public: false,
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
                unused_import_bindings: vec![],
            },
        ];
        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);

        let default_export = graph.modules[1]
            .exports
            .iter()
            .find(|e| matches!(e.name, ExportName::Default))
            .unwrap();
        assert_eq!(default_export.references.len(), 1);
        assert_eq!(default_export.references[0].kind, ReferenceKind::DefaultImport);
    }

    #[test]
    fn type_only_package_usage_tracked_through_build() {
        let files = vec![DiscoveredFile {
            id: FileId(0),
            path: std::path::PathBuf::from("/project/entry.ts"),
            size_bytes: 100,
        }];
        let entry_points = vec![fallow_types::discover::EntryPoint {
            path: std::path::PathBuf::from("/project/entry.ts"),
            source: fallow_types::discover::EntryPointSource::PackageJsonMain,
        }];
        let resolved_modules = vec![ResolvedModule {
            file_id: FileId(0),
            path: std::path::PathBuf::from("/project/entry.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![ResolvedImport {
                info: fallow_types::extract::ImportInfo {
                    source: "react".to_string(),
                    imported_name: ImportedName::Named("FC".to_string()),
                    local_name: "FC".to_string(),
                    is_type_only: true,
                    span: oxc_span::Span::new(0, 10),
                },
                target: ResolveResult::NpmPackage("react".to_string()),
            }],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        }];
        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);

        assert!(graph.package_usage.contains_key("react"));
        assert!(graph.type_only_package_usage.contains_key("react"));
    }
}
