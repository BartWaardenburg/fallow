//! Propagation functions for re-export chain resolution.
//!
//! Handles both star (`export * from`) and named (`export { foo } from`) re-exports,
//! including entry-point special cases where exports are consumed externally.

use rustc_hash::FxHashSet;

use fallow_types::discover::FileId;
use fallow_types::extract::ExportName;

use crate::graph::types::{ExportSymbol, ModuleNode, ReferenceKind, SymbolReference};
use crate::graph::{Edge, ImportedName};

/// Handle `export * from './source'` — propagate named imports through to the source module.
///
/// Star re-exports don't create named `ExportSymbol` entries on the barrel. Instead we look
/// at which named imports other modules make from the barrel and propagate each to the
/// matching export in the source module.
///
/// Returns `true` if any new references were added.
pub(in crate::graph) fn propagate_star_re_export(
    modules: &mut [ModuleNode],
    edges: &[Edge],
    barrel_id: FileId,
    barrel_idx: usize,
    source_idx: usize,
) -> bool {
    // Entry point barrels with star re-exports: all source exports are
    // transitively exposed to external consumers — mark them as used.
    if modules[barrel_idx].is_entry_point {
        return propagate_entry_point_star(modules, barrel_id, source_idx);
    }

    // Collect named imports that target the barrel from ALL edges
    let barrel_file_id = modules[barrel_idx].file_id;
    let named_refs: Vec<(String, SymbolReference)> = edges
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
    let barrel_export_refs: Vec<(String, SymbolReference)> = modules[barrel_idx]
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
    let source_has_star_re_exports = modules[source_idx]
        .re_exports
        .iter()
        .any(|re| re.exported_name == "*");

    // Propagate each named import to the matching source export.
    // For multi-level star re-export chains (e.g., index -> intermediate -> source),
    // intermediate barrels may not have ExportSymbol entries for the names being
    // imported. When the source has its own star re-exports, create synthetic
    // ExportSymbol entries so the iterative loop can propagate further on the
    // next pass.
    let mut changed = false;
    let source = &mut modules[source_idx];
    for (name, ref_item) in named_refs.iter().chain(barrel_export_refs.iter()) {
        let export_name = if name == "default" {
            ExportName::Default
        } else {
            ExportName::Named(name.clone())
        };
        if let Some(export) = source.exports.iter_mut().find(|e| e.name == export_name) {
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
                is_public: false,
                span: oxc_span::Span::new(0, 0),
                references: vec![ref_item.clone()],
                members: Vec::new(),
            });
            changed = true;
        }
    }
    changed
}

/// Entry point barrel with `export *` — mark all non-default source exports as used.
fn propagate_entry_point_star(
    modules: &mut [ModuleNode],
    barrel_id: FileId,
    source_idx: usize,
) -> bool {
    let mut changed = false;
    let source = &mut modules[source_idx];
    for export in &mut source.exports {
        // `export *` does not re-export the default export per ES spec.
        if matches!(export.name, ExportName::Default) {
            continue;
        }
        if export.references.iter().all(|r| r.from_file != barrel_id) {
            export.references.push(SymbolReference {
                from_file: barrel_id,
                kind: ReferenceKind::ReExport,
                import_span: oxc_span::Span::new(0, 0),
            });
            changed = true;
        }
    }
    changed
}

/// Handle named re-exports (`export { foo } from './source'`) — propagate barrel references
/// to the source module's matching export.
///
/// Returns `true` if any new references were added.
pub(in crate::graph) fn propagate_named_re_export(
    modules: &mut [ModuleNode],
    barrel_id: FileId,
    barrel_idx: usize,
    source_idx: usize,
    imported_name: &str,
    exported_name: &str,
    existing_refs: &mut FxHashSet<FileId>,
) -> bool {
    // Find references to the exported name on the barrel
    let refs_on_barrel: Vec<SymbolReference> = modules[barrel_idx]
        .exports
        .iter()
        .filter(|e| e.name.matches_str(exported_name))
        .flat_map(|e| e.references.iter().cloned())
        .collect();

    if refs_on_barrel.is_empty() {
        // Entry point barrels' re-exports are consumed externally (not
        // tracked in the graph). Synthesize a ReExport reference so the
        // source export is correctly marked as used.
        if modules[barrel_idx].is_entry_point {
            return propagate_entry_point_named(modules, barrel_id, source_idx, imported_name);
        }
        return false;
    }

    // Propagate to source module's export
    let mut changed = false;
    let source = &mut modules[source_idx];
    let target_exports: Vec<usize> = source
        .exports
        .iter()
        .enumerate()
        .filter(|(_, e)| e.name.matches_str(imported_name))
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
    changed
}

/// Entry point barrel with named re-export and no in-graph consumers — synthesize
/// a `ReExport` reference so the source export is correctly marked as used.
fn propagate_entry_point_named(
    modules: &mut [ModuleNode],
    barrel_id: FileId,
    source_idx: usize,
    imported_name: &str,
) -> bool {
    let synthetic_ref = SymbolReference {
        from_file: barrel_id,
        kind: ReferenceKind::ReExport,
        import_span: oxc_span::Span::new(0, 0),
    };
    let mut changed = false;
    let source = &mut modules[source_idx];
    let target_exports: Vec<usize> = source
        .exports
        .iter()
        .enumerate()
        .filter(|(_, e)| e.name.matches_str(imported_name))
        .map(|(i, _)| i)
        .collect();
    for export_idx in target_exports {
        if source.exports[export_idx]
            .references
            .iter()
            .all(|r| r.from_file != barrel_id)
        {
            source.exports[export_idx]
                .references
                .push(synthetic_ref.clone());
            changed = true;
        }
    }
    changed
}
