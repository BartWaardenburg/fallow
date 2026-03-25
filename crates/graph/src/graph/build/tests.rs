//! Tests for graph construction phases (edges and references).

use rustc_hash::{FxHashMap, FxHashSet};

use super::edges::{
    collect_edges_for_module, collect_import_edge, record_namespace_import, record_package_usage,
    EdgeAccumulator,
};
use super::references::{
    create_synthetic_exports_for_star_re_exports, export_matches, extract_accessed_members,
    is_css_module_path, is_unused_import_binding, mark_all_exports_referenced,
    mark_member_exports_referenced, reference_kind_for,
};
use super::super::types::{ExportSymbol, ReExportEdge, ReferenceKind, SymbolReference};
use super::super::{ImportedSymbol, ModuleGraph};
use crate::resolve::{ResolveResult, ResolvedImport, ResolvedModule};
use fallow_types::discover::{DiscoveredFile, EntryPoint, EntryPointSource, FileId};
use fallow_types::extract::{ExportName, ImportedName};
use std::path::PathBuf;

// ── export_matches ──────────────────────────────────────────────────

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
    assert!(!is_css_module_path(std::path::Path::new(
        "Button.module.ts"
    )));
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
            source_span: oxc_span::Span::default(),
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
            source_span: oxc_span::Span::default(),
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
                    source_span: oxc_span::Span::default(),
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
                    source_span: oxc_span::Span::default(),
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
    assert_eq!(exports[1].name, ExportName::Named("missing".to_string()));
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
                    source_span: oxc_span::Span::default(),
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
                    source_span: oxc_span::Span::default(),
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
                    source_span: oxc_span::Span::default(),
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
                    source_span: oxc_span::Span::default(),
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
                    source_span: oxc_span::Span::default(),
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
    assert_eq!(
        default_export.references[0].kind,
        ReferenceKind::DefaultImport
    );
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
                source_span: oxc_span::Span::default(),
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
