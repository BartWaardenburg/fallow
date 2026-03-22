//! Phase 4: Re-export chain resolution — propagate references through barrel files.

use rustc_hash::FxHashSet;

use fallow_types::discover::FileId;
use fallow_types::extract::ExportName;

use super::types::{ExportSymbol, ReferenceKind, SymbolReference};
use super::{ImportedName, ModuleGraph};

impl ModuleGraph {
    /// Resolve re-export chains: when module A re-exports from B,
    /// any reference to A's re-exported symbol should also count as a reference
    /// to B's original export (and transitively through the chain).
    pub(super) fn resolve_re_export_chains(&mut self) {
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
}

#[cfg(test)]
mod tests {
    use crate::resolve::{ResolveResult, ResolvedImport, ResolvedModule, ResolvedReExport};
    use fallow_types::discover::{DiscoveredFile, EntryPoint, EntryPointSource, FileId};
    use fallow_types::extract::{ExportName, ImportInfo, ImportedName};
    use std::path::PathBuf;

    use super::ModuleGraph;

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
                unused_import_bindings: vec![],
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
                unused_import_bindings: vec![],
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
                unused_import_bindings: vec![],
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
                unused_import_bindings: vec![],
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
                unused_import_bindings: vec![],
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
                unused_import_bindings: vec![],
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
                unused_import_bindings: vec![],
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
                unused_import_bindings: vec![],
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
                unused_import_bindings: vec![],
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
                unused_import_bindings: vec![],
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
                unused_import_bindings: vec![],
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
                unused_import_bindings: vec![],
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
                unused_import_bindings: vec![],
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
                unused_import_bindings: vec![],
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
                unused_import_bindings: vec![],
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
                unused_import_bindings: vec![],
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
                unused_import_bindings: vec![],
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
                unused_import_bindings: vec![],
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
                unused_import_bindings: vec![],
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
}
