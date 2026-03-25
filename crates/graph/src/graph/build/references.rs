//! Phase 2: Reference population — attaching `SymbolReference` entries to exports.
//!
//! Walks every edge and determines which exports on the target module are referenced,
//! including namespace import narrowing and CSS module default-import narrowing.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::resolve::ResolvedModule;
use fallow_types::discover::FileId;
use fallow_types::extract::{ExportName, ImportedName};

use super::super::types::{ExportSymbol, ReExportEdge, ReferenceKind, SymbolReference};
use super::super::types::ModuleNode;
use super::super::{ImportedSymbol, ModuleGraph};

/// Check whether an import binding is unused in the source file.
///
/// Returns `true` if the binding should be skipped (unused).
pub(super) fn is_unused_import_binding(
    sym_local_name: &str,
    sym_imported_name: &ImportedName,
    source_mod: Option<&&ResolvedModule>,
) -> bool {
    !sym_local_name.is_empty()
        && !matches!(sym_imported_name, ImportedName::SideEffect)
        && source_mod.is_some_and(|m| m.unused_import_bindings.iter().any(|n| n == sym_local_name))
}

/// Extract member access names for a given local variable from a resolved module.
pub(super) fn extract_accessed_members(
    source_mod: Option<&&ResolvedModule>,
    local_name: &str,
) -> Vec<String> {
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
pub(super) fn mark_all_exports_referenced(
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
pub(super) fn mark_member_exports_referenced(
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
pub(super) fn create_synthetic_exports_for_star_re_exports(
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
pub(super) fn reference_kind_for(imported_name: &ImportedName) -> ReferenceKind {
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
    /// Record which files reference which exports from edges.
    ///
    /// Walks every edge and attaches `SymbolReference` entries to the target
    /// module's exports. Includes namespace import narrowing (member access
    /// tracking) and CSS Module default-import narrowing.
    pub(in super::super) fn populate_references(
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
pub(in super::super) fn is_css_module_path(path: &std::path::Path) -> bool {
    path.file_stem()
        .and_then(|s| s.to_str())
        .is_some_and(|stem| stem.ends_with(".module"))
        && path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| ext == "css" || ext == "scss")
}

/// Check if an export name matches an imported name.
pub(in super::super) fn export_matches(export: &ExportName, import: &ImportedName) -> bool {
    match (export, import) {
        (ExportName::Named(e), ImportedName::Named(i)) => e == i,
        (ExportName::Default, ImportedName::Default) => true,
        _ => false,
    }
}
