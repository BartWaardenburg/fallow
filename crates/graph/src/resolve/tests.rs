use std::path::{Path, PathBuf};

use oxc_span::Span;
use rustc_hash::{FxHashMap, FxHashSet};

use fallow_types::discover::{DiscoveredFile, FileId};
use fallow_types::extract::{
    DynamicImportInfo, DynamicImportPattern, ImportInfo, ImportedName, ReExportInfo,
    RequireCallInfo,
};

use super::dynamic_imports::{resolve_dynamic_imports, resolve_dynamic_patterns, resolve_single_dynamic_import};
use super::re_exports::resolve_re_exports;
use super::require_imports::{resolve_require_imports, resolve_single_require};
use super::specifier;
use super::static_imports::resolve_static_imports;
use super::types::ResolveContext;
use super::upgrades::apply_specifier_upgrades;
use super::{ResolveResult, ResolvedImport, ResolvedModule, ResolvedReExport};

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

fn dummy_span() -> Span {
    Span::new(0, 0)
}

/// Build a minimal `ResolveContext` backed by a real resolver but with
/// empty lookup tables. Every specifier resolves to `NpmPackage` or
/// `Unresolvable`, which is fine — the tests focus on how helper functions
/// *transform* inputs into `ResolvedImport` / `ResolvedReExport` structs.
///
/// Under Miri this is a no-op: `oxc_resolver` uses the `statx` syscall
/// (via `rustix`) which Miri does not support.
#[cfg(not(miri))]
fn with_empty_ctx<F: FnOnce(&ResolveContext)>(f: F) {
    let resolver = specifier::create_resolver(&[]);
    let path_to_id = FxHashMap::default();
    let raw_path_to_id = FxHashMap::default();
    let workspace_roots = FxHashMap::default();
    let root = PathBuf::from("/project");
    let ctx = ResolveContext {
        resolver: &resolver,
        path_to_id: &path_to_id,
        raw_path_to_id: &raw_path_to_id,
        workspace_roots: &workspace_roots,
        path_aliases: &[],
        root: &root,
        canonical_fallback: None,
    };
    f(&ctx);
}

#[cfg(miri)]
fn with_empty_ctx<F: FnOnce(&ResolveContext)>(_f: F) {
    // oxc_resolver uses statx syscall unsupported by Miri — skip.
}

fn make_import(source: &str, imported: ImportedName, local: &str) -> ImportInfo {
    ImportInfo {
        source: source.to_string(),
        imported_name: imported,
        local_name: local.to_string(),
        is_type_only: false,
        span: dummy_span(),
        source_span: Span::default(),
    }
}

fn make_re_export(source: &str, imported: &str, exported: &str) -> ReExportInfo {
    ReExportInfo {
        source: source.to_string(),
        imported_name: imported.to_string(),
        exported_name: exported.to_string(),
        is_type_only: false,
    }
}

fn make_dynamic(
    source: &str,
    destructured: Vec<&str>,
    local_name: Option<&str>,
) -> DynamicImportInfo {
    DynamicImportInfo {
        source: source.to_string(),
        span: dummy_span(),
        destructured_names: destructured.into_iter().map(String::from).collect(),
        local_name: local_name.map(String::from),
    }
}

fn make_require(
    source: &str,
    destructured: Vec<&str>,
    local_name: Option<&str>,
) -> RequireCallInfo {
    RequireCallInfo {
        source: source.to_string(),
        span: dummy_span(),
        destructured_names: destructured.into_iter().map(String::from).collect(),
        local_name: local_name.map(String::from),
    }
}

/// Build a minimal `ResolvedModule` for `apply_specifier_upgrades` tests.
fn make_resolved_module(
    file_id: u32,
    imports: Vec<ResolvedImport>,
    dynamic_imports: Vec<ResolvedImport>,
    re_exports: Vec<ResolvedReExport>,
) -> ResolvedModule {
    ResolvedModule {
        file_id: FileId(file_id),
        path: PathBuf::from(format!("/project/src/file_{file_id}.ts")),
        exports: vec![],
        re_exports,
        resolved_imports: imports,
        resolved_dynamic_imports: dynamic_imports,
        resolved_dynamic_patterns: vec![],
        member_accesses: vec![],
        whole_object_uses: vec![],
        has_cjs_exports: false,
        unused_import_bindings: FxHashSet::default(),
    }
}

fn make_resolved_import(source: &str, target: ResolveResult) -> ResolvedImport {
    ResolvedImport {
        info: make_import(source, ImportedName::Named("x".into()), "x"),
        target,
    }
}

fn make_resolved_re_export(source: &str, target: ResolveResult) -> ResolvedReExport {
    ResolvedReExport {
        info: make_re_export(source, "x", "x"),
        target,
    }
}

// -----------------------------------------------------------------------
// resolve_static_imports
// -----------------------------------------------------------------------

#[test]
fn static_imports_named() {
    with_empty_ctx(|ctx| {
        let imports = vec![make_import(
            "react",
            ImportedName::Named("useState".into()),
            "useState",
        )];
        let file = Path::new("/project/src/app.ts");
        let result = resolve_static_imports(ctx, file, &imports);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].info.source, "react");
        assert!(matches!(
            result[0].info.imported_name,
            ImportedName::Named(ref n) if n == "useState"
        ));
    });
}

#[test]
fn static_imports_default() {
    with_empty_ctx(|ctx| {
        let imports = vec![make_import("react", ImportedName::Default, "React")];
        let file = Path::new("/project/src/app.ts");
        let result = resolve_static_imports(ctx, file, &imports);

        assert_eq!(result.len(), 1);
        assert!(matches!(
            result[0].info.imported_name,
            ImportedName::Default
        ));
        assert_eq!(result[0].info.local_name, "React");
    });
}

#[test]
fn static_imports_namespace() {
    with_empty_ctx(|ctx| {
        let imports = vec![make_import("lodash", ImportedName::Namespace, "_")];
        let file = Path::new("/project/src/utils.ts");
        let result = resolve_static_imports(ctx, file, &imports);

        assert_eq!(result.len(), 1);
        assert!(matches!(
            result[0].info.imported_name,
            ImportedName::Namespace
        ));
        assert_eq!(result[0].info.local_name, "_");
    });
}

#[test]
fn static_imports_side_effect() {
    with_empty_ctx(|ctx| {
        let imports = vec![make_import("./styles.css", ImportedName::SideEffect, "")];
        let file = Path::new("/project/src/app.ts");
        let result = resolve_static_imports(ctx, file, &imports);

        assert_eq!(result.len(), 1);
        assert!(matches!(
            result[0].info.imported_name,
            ImportedName::SideEffect
        ));
        assert_eq!(result[0].info.local_name, "");
    });
}

#[test]
fn static_imports_empty_list() {
    with_empty_ctx(|ctx| {
        let file = Path::new("/project/src/app.ts");
        let result = resolve_static_imports(ctx, file, &[]);
        assert!(result.is_empty());
    });
}

#[test]
fn static_imports_multiple() {
    with_empty_ctx(|ctx| {
        let imports = vec![
            make_import("react", ImportedName::Default, "React"),
            make_import("react", ImportedName::Named("useState".into()), "useState"),
            make_import("lodash", ImportedName::Namespace, "_"),
        ];
        let file = Path::new("/project/src/app.ts");
        let result = resolve_static_imports(ctx, file, &imports);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].info.source, "react");
        assert_eq!(result[1].info.source, "react");
        assert_eq!(result[2].info.source, "lodash");
    });
}

#[test]
fn static_imports_preserves_type_only() {
    with_empty_ctx(|ctx| {
        let imports = vec![ImportInfo {
            source: "react".into(),
            imported_name: ImportedName::Named("FC".into()),
            local_name: "FC".into(),
            is_type_only: true,
            span: dummy_span(),
            source_span: Span::default(),
        }];
        let file = Path::new("/project/src/app.ts");
        let result = resolve_static_imports(ctx, file, &imports);

        assert_eq!(result.len(), 1);
        assert!(result[0].info.is_type_only);
    });
}

// -----------------------------------------------------------------------
// resolve_single_dynamic_import
// -----------------------------------------------------------------------

#[test]
fn dynamic_import_with_destructured_names() {
    with_empty_ctx(|ctx| {
        let imp = make_dynamic("./utils", vec!["foo", "bar"], None);
        let file = Path::new("/project/src/app.ts");
        let result = resolve_single_dynamic_import(ctx, file, &imp);

        assert_eq!(result.len(), 2);
        assert!(matches!(
            result[0].info.imported_name,
            ImportedName::Named(ref n) if n == "foo"
        ));
        assert_eq!(result[0].info.local_name, "foo");
        assert!(matches!(
            result[1].info.imported_name,
            ImportedName::Named(ref n) if n == "bar"
        ));
        assert_eq!(result[1].info.local_name, "bar");
        // Both should have the same source
        assert_eq!(result[0].info.source, "./utils");
        assert_eq!(result[1].info.source, "./utils");
        // Both should be non-type-only
        assert!(!result[0].info.is_type_only);
        assert!(!result[1].info.is_type_only);
    });
}

#[test]
fn dynamic_import_namespace_with_local_name() {
    with_empty_ctx(|ctx| {
        let imp = make_dynamic("./utils", vec![], Some("utils"));
        let file = Path::new("/project/src/app.ts");
        let result = resolve_single_dynamic_import(ctx, file, &imp);

        assert_eq!(result.len(), 1);
        assert!(matches!(
            result[0].info.imported_name,
            ImportedName::Namespace
        ));
        assert_eq!(result[0].info.local_name, "utils");
    });
}

#[test]
fn dynamic_import_side_effect() {
    with_empty_ctx(|ctx| {
        let imp = make_dynamic("./polyfill", vec![], None);
        let file = Path::new("/project/src/app.ts");
        let result = resolve_single_dynamic_import(ctx, file, &imp);

        assert_eq!(result.len(), 1);
        assert!(matches!(
            result[0].info.imported_name,
            ImportedName::SideEffect
        ));
        assert_eq!(result[0].info.local_name, "");
        assert_eq!(result[0].info.source, "./polyfill");
    });
}

#[test]
fn dynamic_import_destructured_takes_priority_over_local_name() {
    // When both destructured_names and local_name are set,
    // destructured_names wins (checked first).
    with_empty_ctx(|ctx| {
        let imp = DynamicImportInfo {
            source: "./mod".into(),
            span: dummy_span(),
            destructured_names: vec!["a".into()],
            local_name: Some("mod".into()),
        };
        let file = Path::new("/project/src/app.ts");
        let result = resolve_single_dynamic_import(ctx, file, &imp);

        assert_eq!(result.len(), 1);
        assert!(matches!(
            result[0].info.imported_name,
            ImportedName::Named(ref n) if n == "a"
        ));
    });
}

// -----------------------------------------------------------------------
// resolve_dynamic_imports (batch)
// -----------------------------------------------------------------------

#[test]
fn dynamic_imports_flattens_multiple() {
    with_empty_ctx(|ctx| {
        let imports = vec![
            make_dynamic("./a", vec!["x", "y"], None),
            make_dynamic("./b", vec![], Some("b")),
            make_dynamic("./c", vec![], None),
        ];
        let file = Path::new("/project/src/app.ts");
        let result = resolve_dynamic_imports(ctx, file, &imports);

        // ./a -> 2 Named, ./b -> 1 Namespace, ./c -> 1 SideEffect = 4 total
        assert_eq!(result.len(), 4);
    });
}

#[test]
fn dynamic_imports_empty_list() {
    with_empty_ctx(|ctx| {
        let file = Path::new("/project/src/app.ts");
        let result = resolve_dynamic_imports(ctx, file, &[]);
        assert!(result.is_empty());
    });
}

// -----------------------------------------------------------------------
// resolve_re_exports
// -----------------------------------------------------------------------

#[test]
fn re_exports_maps_each_entry() {
    with_empty_ctx(|ctx| {
        let re_exports = vec![
            make_re_export("./utils", "helper", "helper"),
            make_re_export("./types", "*", "*"),
        ];
        let file = Path::new("/project/src/index.ts");
        let result = resolve_re_exports(ctx, file, &re_exports);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].info.source, "./utils");
        assert_eq!(result[0].info.imported_name, "helper");
        assert_eq!(result[0].info.exported_name, "helper");
        assert_eq!(result[1].info.source, "./types");
        assert_eq!(result[1].info.imported_name, "*");
    });
}

#[test]
fn re_exports_empty_list() {
    with_empty_ctx(|ctx| {
        let file = Path::new("/project/src/index.ts");
        let result = resolve_re_exports(ctx, file, &[]);
        assert!(result.is_empty());
    });
}

#[test]
fn re_exports_preserves_type_only() {
    with_empty_ctx(|ctx| {
        let re_exports = vec![ReExportInfo {
            source: "./types".into(),
            imported_name: "MyType".into(),
            exported_name: "MyType".into(),
            is_type_only: true,
        }];
        let file = Path::new("/project/src/index.ts");
        let result = resolve_re_exports(ctx, file, &re_exports);

        assert_eq!(result.len(), 1);
        assert!(result[0].info.is_type_only);
    });
}

// -----------------------------------------------------------------------
// resolve_single_require
// -----------------------------------------------------------------------

#[test]
fn require_namespace_without_destructuring() {
    with_empty_ctx(|ctx| {
        let req = make_require("fs", vec![], Some("fs"));
        let file = Path::new("/project/src/app.js");
        let result = resolve_single_require(ctx, file, &req);

        assert_eq!(result.len(), 1);
        assert!(matches!(
            result[0].info.imported_name,
            ImportedName::Namespace
        ));
        assert_eq!(result[0].info.local_name, "fs");
        assert_eq!(result[0].info.source, "fs");
    });
}

#[test]
fn require_namespace_without_local_name() {
    with_empty_ctx(|ctx| {
        let req = make_require("./side-effect", vec![], None);
        let file = Path::new("/project/src/app.js");
        let result = resolve_single_require(ctx, file, &req);

        assert_eq!(result.len(), 1);
        assert!(matches!(
            result[0].info.imported_name,
            ImportedName::Namespace
        ));
        // No local name -> empty string from unwrap_or_default
        assert_eq!(result[0].info.local_name, "");
    });
}

#[test]
fn require_with_destructured_names() {
    with_empty_ctx(|ctx| {
        let req = make_require("path", vec!["join", "resolve"], None);
        let file = Path::new("/project/src/app.js");
        let result = resolve_single_require(ctx, file, &req);

        assert_eq!(result.len(), 2);
        assert!(matches!(
            result[0].info.imported_name,
            ImportedName::Named(ref n) if n == "join"
        ));
        assert_eq!(result[0].info.local_name, "join");
        assert!(matches!(
            result[1].info.imported_name,
            ImportedName::Named(ref n) if n == "resolve"
        ));
        assert_eq!(result[1].info.local_name, "resolve");
        // Both share the same source
        assert_eq!(result[0].info.source, "path");
        assert_eq!(result[1].info.source, "path");
    });
}

#[test]
fn require_destructured_is_not_type_only() {
    with_empty_ctx(|ctx| {
        let req = make_require("path", vec!["join"], None);
        let file = Path::new("/project/src/app.js");
        let result = resolve_single_require(ctx, file, &req);

        assert_eq!(result.len(), 1);
        assert!(!result[0].info.is_type_only);
    });
}

// -----------------------------------------------------------------------
// resolve_require_imports (batch)
// -----------------------------------------------------------------------

#[test]
fn require_imports_flattens_multiple() {
    with_empty_ctx(|ctx| {
        let reqs = vec![
            make_require("fs", vec![], Some("fs")),
            make_require("path", vec!["join", "resolve"], None),
        ];
        let file = Path::new("/project/src/app.js");
        let result = resolve_require_imports(ctx, file, &reqs);

        // fs -> 1 Namespace, path -> 2 Named = 3 total
        assert_eq!(result.len(), 3);
    });
}

#[test]
fn require_imports_empty_list() {
    with_empty_ctx(|ctx| {
        let file = Path::new("/project/src/app.js");
        let result = resolve_require_imports(ctx, file, &[]);
        assert!(result.is_empty());
    });
}

// -----------------------------------------------------------------------
// apply_specifier_upgrades
// -----------------------------------------------------------------------

#[test]
fn specifier_upgrades_npm_to_internal() {
    // Module 0 resolves `preact/hooks` to InternalModule(FileId(5))
    // Module 1 resolves `preact/hooks` to NpmPackage("preact")
    // After upgrade, module 1 should also point to InternalModule(FileId(5))
    let mut modules = vec![
        make_resolved_module(
            0,
            vec![make_resolved_import(
                "preact/hooks",
                ResolveResult::InternalModule(FileId(5)),
            )],
            vec![],
            vec![],
        ),
        make_resolved_module(
            1,
            vec![make_resolved_import(
                "preact/hooks",
                ResolveResult::NpmPackage("preact".into()),
            )],
            vec![],
            vec![],
        ),
    ];

    apply_specifier_upgrades(&mut modules);

    assert!(matches!(
        modules[1].resolved_imports[0].target,
        ResolveResult::InternalModule(FileId(5))
    ));
}

#[test]
fn specifier_upgrades_noop_when_no_internal() {
    // All modules resolve `lodash` to NpmPackage — no upgrade should happen
    let mut modules = vec![
        make_resolved_module(
            0,
            vec![make_resolved_import(
                "lodash",
                ResolveResult::NpmPackage("lodash".into()),
            )],
            vec![],
            vec![],
        ),
        make_resolved_module(
            1,
            vec![make_resolved_import(
                "lodash",
                ResolveResult::NpmPackage("lodash".into()),
            )],
            vec![],
            vec![],
        ),
    ];

    apply_specifier_upgrades(&mut modules);

    assert!(matches!(
        modules[0].resolved_imports[0].target,
        ResolveResult::NpmPackage(_)
    ));
    assert!(matches!(
        modules[1].resolved_imports[0].target,
        ResolveResult::NpmPackage(_)
    ));
}

#[test]
fn specifier_upgrades_empty_modules() {
    let mut modules: Vec<ResolvedModule> = vec![];
    apply_specifier_upgrades(&mut modules);
    assert!(modules.is_empty());
}

#[test]
fn specifier_upgrades_skips_relative_specifiers() {
    // Relative specifiers (./foo) are NOT bare specifiers, so they should
    // never be candidates for upgrade.
    let mut modules = vec![
        make_resolved_module(
            0,
            vec![make_resolved_import(
                "./utils",
                ResolveResult::InternalModule(FileId(5)),
            )],
            vec![],
            vec![],
        ),
        make_resolved_module(
            1,
            vec![make_resolved_import(
                "./utils",
                ResolveResult::NpmPackage("utils".into()),
            )],
            vec![],
            vec![],
        ),
    ];

    apply_specifier_upgrades(&mut modules);

    // Module 1 should still be NpmPackage — relative specifier not upgraded
    assert!(matches!(
        modules[1].resolved_imports[0].target,
        ResolveResult::NpmPackage(_)
    ));
}

#[test]
fn specifier_upgrades_applies_to_dynamic_imports() {
    let mut modules = vec![
        make_resolved_module(
            0,
            vec![],
            vec![make_resolved_import(
                "preact/hooks",
                ResolveResult::InternalModule(FileId(5)),
            )],
            vec![],
        ),
        make_resolved_module(
            1,
            vec![],
            vec![make_resolved_import(
                "preact/hooks",
                ResolveResult::NpmPackage("preact".into()),
            )],
            vec![],
        ),
    ];

    apply_specifier_upgrades(&mut modules);

    assert!(matches!(
        modules[1].resolved_dynamic_imports[0].target,
        ResolveResult::InternalModule(FileId(5))
    ));
}

#[test]
fn specifier_upgrades_applies_to_re_exports() {
    let mut modules = vec![
        make_resolved_module(
            0,
            vec![],
            vec![],
            vec![make_resolved_re_export(
                "preact/hooks",
                ResolveResult::InternalModule(FileId(5)),
            )],
        ),
        make_resolved_module(
            1,
            vec![],
            vec![],
            vec![make_resolved_re_export(
                "preact/hooks",
                ResolveResult::NpmPackage("preact".into()),
            )],
        ),
    ];

    apply_specifier_upgrades(&mut modules);

    assert!(matches!(
        modules[1].re_exports[0].target,
        ResolveResult::InternalModule(FileId(5))
    ));
}

#[test]
fn specifier_upgrades_does_not_downgrade_internal() {
    // If both modules already resolve to InternalModule, nothing changes
    let mut modules = vec![
        make_resolved_module(
            0,
            vec![make_resolved_import(
                "preact/hooks",
                ResolveResult::InternalModule(FileId(5)),
            )],
            vec![],
            vec![],
        ),
        make_resolved_module(
            1,
            vec![make_resolved_import(
                "preact/hooks",
                ResolveResult::InternalModule(FileId(5)),
            )],
            vec![],
            vec![],
        ),
    ];

    apply_specifier_upgrades(&mut modules);

    assert!(matches!(
        modules[0].resolved_imports[0].target,
        ResolveResult::InternalModule(FileId(5))
    ));
    assert!(matches!(
        modules[1].resolved_imports[0].target,
        ResolveResult::InternalModule(FileId(5))
    ));
}

#[test]
fn specifier_upgrades_first_internal_wins() {
    // Two modules resolve the same bare specifier to different internal files.
    // The first one (by module order) wins.
    let mut modules = vec![
        make_resolved_module(
            0,
            vec![make_resolved_import(
                "shared-lib",
                ResolveResult::InternalModule(FileId(10)),
            )],
            vec![],
            vec![],
        ),
        make_resolved_module(
            1,
            vec![make_resolved_import(
                "shared-lib",
                ResolveResult::InternalModule(FileId(20)),
            )],
            vec![],
            vec![],
        ),
        make_resolved_module(
            2,
            vec![make_resolved_import(
                "shared-lib",
                ResolveResult::NpmPackage("shared-lib".into()),
            )],
            vec![],
            vec![],
        ),
    ];

    apply_specifier_upgrades(&mut modules);

    // Module 2 should be upgraded to the first FileId encountered (10)
    assert!(matches!(
        modules[2].resolved_imports[0].target,
        ResolveResult::InternalModule(FileId(10))
    ));
}

#[test]
fn specifier_upgrades_does_not_touch_unresolvable() {
    // Unresolvable should not be upgraded even if a bare specifier
    // matches an InternalModule elsewhere.
    let mut modules = vec![
        make_resolved_module(
            0,
            vec![make_resolved_import(
                "my-lib",
                ResolveResult::InternalModule(FileId(1)),
            )],
            vec![],
            vec![],
        ),
        make_resolved_module(
            1,
            vec![ResolvedImport {
                info: make_import("my-lib", ImportedName::Default, "myLib"),
                target: ResolveResult::Unresolvable("my-lib".into()),
            }],
            vec![],
            vec![],
        ),
    ];

    apply_specifier_upgrades(&mut modules);

    // Unresolvable should remain unresolvable
    assert!(matches!(
        modules[1].resolved_imports[0].target,
        ResolveResult::Unresolvable(_)
    ));
}

#[test]
fn specifier_upgrades_cross_import_and_re_export() {
    // An import in module 0 resolves to InternalModule, a re-export in
    // module 1 for the same specifier should also be upgraded.
    let mut modules = vec![
        make_resolved_module(
            0,
            vec![make_resolved_import(
                "@myorg/utils",
                ResolveResult::InternalModule(FileId(3)),
            )],
            vec![],
            vec![],
        ),
        make_resolved_module(
            1,
            vec![],
            vec![],
            vec![make_resolved_re_export(
                "@myorg/utils",
                ResolveResult::NpmPackage("@myorg/utils".into()),
            )],
        ),
    ];

    apply_specifier_upgrades(&mut modules);

    assert!(matches!(
        modules[1].re_exports[0].target,
        ResolveResult::InternalModule(FileId(3))
    ));
}

// -----------------------------------------------------------------------
// resolve_dynamic_patterns
// -----------------------------------------------------------------------

#[test]
fn dynamic_patterns_matches_files_in_dir() {
    let from_dir = Path::new("/project/src");
    let patterns = vec![DynamicImportPattern {
        prefix: "./locales/".into(),
        suffix: Some(".json".into()),
        span: dummy_span(),
    }];
    let canonical_paths = vec![
        PathBuf::from("/project/src/locales/en.json"),
        PathBuf::from("/project/src/locales/fr.json"),
        PathBuf::from("/project/src/utils.ts"),
    ];
    let files = vec![
        DiscoveredFile {
            id: FileId(0),
            path: PathBuf::from("/project/src/locales/en.json"),
            size_bytes: 100,
        },
        DiscoveredFile {
            id: FileId(1),
            path: PathBuf::from("/project/src/locales/fr.json"),
            size_bytes: 100,
        },
        DiscoveredFile {
            id: FileId(2),
            path: PathBuf::from("/project/src/utils.ts"),
            size_bytes: 100,
        },
    ];

    let result = resolve_dynamic_patterns(from_dir, &patterns, &canonical_paths, &files);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].1.len(), 2);
    assert!(result[0].1.contains(&FileId(0)));
    assert!(result[0].1.contains(&FileId(1)));
}

#[test]
fn dynamic_patterns_no_matches_returns_empty() {
    let from_dir = Path::new("/project/src");
    let patterns = vec![DynamicImportPattern {
        prefix: "./locales/".into(),
        suffix: Some(".json".into()),
        span: dummy_span(),
    }];
    let canonical_paths = vec![PathBuf::from("/project/src/utils.ts")];
    let files = vec![DiscoveredFile {
        id: FileId(0),
        path: PathBuf::from("/project/src/utils.ts"),
        size_bytes: 100,
    }];

    let result = resolve_dynamic_patterns(from_dir, &patterns, &canonical_paths, &files);

    assert!(result.is_empty());
}

#[test]
fn dynamic_patterns_empty_patterns_list() {
    let from_dir = Path::new("/project/src");
    let canonical_paths = vec![PathBuf::from("/project/src/utils.ts")];
    let files = vec![DiscoveredFile {
        id: FileId(0),
        path: PathBuf::from("/project/src/utils.ts"),
        size_bytes: 100,
    }];

    let result = resolve_dynamic_patterns(from_dir, &[], &canonical_paths, &files);
    assert!(result.is_empty());
}

#[test]
fn dynamic_patterns_glob_prefix_passthrough() {
    let from_dir = Path::new("/project/src");
    let patterns = vec![DynamicImportPattern {
        prefix: "./**/*.ts".into(),
        suffix: None,
        span: dummy_span(),
    }];
    let canonical_paths = vec![
        PathBuf::from("/project/src/utils.ts"),
        PathBuf::from("/project/src/deep/nested.ts"),
    ];
    let files = vec![
        DiscoveredFile {
            id: FileId(0),
            path: PathBuf::from("/project/src/utils.ts"),
            size_bytes: 100,
        },
        DiscoveredFile {
            id: FileId(1),
            path: PathBuf::from("/project/src/deep/nested.ts"),
            size_bytes: 100,
        },
    ];

    let result = resolve_dynamic_patterns(from_dir, &patterns, &canonical_paths, &files);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].1.len(), 2);
}

// -----------------------------------------------------------------------
// Unresolvable specifier handling
// -----------------------------------------------------------------------

#[test]
fn static_import_unresolvable_relative_path() {
    with_empty_ctx(|ctx| {
        let imports = vec![make_import(
            "./nonexistent",
            ImportedName::Default,
            "missing",
        )];
        let file = Path::new("/project/src/app.ts");
        let result = resolve_static_imports(ctx, file, &imports);

        assert_eq!(result.len(), 1);
        assert!(matches!(result[0].target, ResolveResult::Unresolvable(_)));
    });
}

#[test]
fn static_import_bare_specifier_becomes_npm_package() {
    with_empty_ctx(|ctx| {
        let imports = vec![make_import("react", ImportedName::Default, "React")];
        let file = Path::new("/project/src/app.ts");
        let result = resolve_static_imports(ctx, file, &imports);

        assert_eq!(result.len(), 1);
        assert!(matches!(
            result[0].target,
            ResolveResult::NpmPackage(ref pkg) if pkg == "react"
        ));
    });
}

#[test]
fn require_bare_specifier_becomes_npm_package() {
    with_empty_ctx(|ctx| {
        let req = make_require("express", vec![], Some("express"));
        let file = Path::new("/project/src/app.js");
        let result = resolve_single_require(ctx, file, &req);

        assert_eq!(result.len(), 1);
        assert!(matches!(
            result[0].target,
            ResolveResult::NpmPackage(ref pkg) if pkg == "express"
        ));
    });
}

#[test]
fn dynamic_import_unresolvable() {
    with_empty_ctx(|ctx| {
        let imp = make_dynamic("./missing-module", vec![], None);
        let file = Path::new("/project/src/app.ts");
        let result = resolve_single_dynamic_import(ctx, file, &imp);

        assert_eq!(result.len(), 1);
        assert!(matches!(result[0].target, ResolveResult::Unresolvable(_)));
    });
}

#[test]
fn re_export_unresolvable() {
    with_empty_ctx(|ctx| {
        let re_exports = vec![make_re_export("./missing", "foo", "foo")];
        let file = Path::new("/project/src/index.ts");
        let result = resolve_re_exports(ctx, file, &re_exports);

        assert_eq!(result.len(), 1);
        assert!(matches!(result[0].target, ResolveResult::Unresolvable(_)));
    });
}

// -----------------------------------------------------------------------
// apply_specifier_upgrades: re-export triggers upgrade for imports
// -----------------------------------------------------------------------

#[test]
fn specifier_upgrades_re_export_triggers_import_upgrade() {
    // Module 0 has a re-export that resolves to InternalModule.
    // Module 1 has a static import for the same specifier as NpmPackage.
    // The re-export should trigger the import to be upgraded.
    let mut modules = vec![
        make_resolved_module(
            0,
            vec![],
            vec![],
            vec![make_resolved_re_export(
                "@myorg/shared",
                ResolveResult::InternalModule(FileId(5)),
            )],
        ),
        make_resolved_module(
            1,
            vec![make_resolved_import(
                "@myorg/shared",
                ResolveResult::NpmPackage("@myorg/shared".into()),
            )],
            vec![],
            vec![],
        ),
    ];

    apply_specifier_upgrades(&mut modules);

    assert!(matches!(
        modules[1].resolved_imports[0].target,
        ResolveResult::InternalModule(FileId(5))
    ));
}

#[test]
fn specifier_upgrades_re_export_triggers_dynamic_import_upgrade() {
    // A re-export resolving to InternalModule should also upgrade
    // dynamic imports for the same specifier.
    let mut modules = vec![
        make_resolved_module(
            0,
            vec![],
            vec![],
            vec![make_resolved_re_export(
                "my-workspace-pkg",
                ResolveResult::InternalModule(FileId(7)),
            )],
        ),
        make_resolved_module(
            1,
            vec![],
            vec![make_resolved_import(
                "my-workspace-pkg",
                ResolveResult::NpmPackage("my-workspace-pkg".into()),
            )],
            vec![],
        ),
    ];

    apply_specifier_upgrades(&mut modules);

    assert!(matches!(
        modules[1].resolved_dynamic_imports[0].target,
        ResolveResult::InternalModule(FileId(7))
    ));
}

#[test]
fn specifier_upgrades_does_not_upgrade_external_file() {
    // ExternalFile results should never be upgraded, even when a bare
    // specifier matches.
    let mut modules = vec![
        make_resolved_module(
            0,
            vec![make_resolved_import(
                "shared-lib",
                ResolveResult::InternalModule(FileId(3)),
            )],
            vec![],
            vec![],
        ),
        make_resolved_module(
            1,
            vec![ResolvedImport {
                info: make_import("shared-lib", ImportedName::Default, "lib"),
                target: ResolveResult::ExternalFile(PathBuf::from("/node_modules/shared-lib/index.js")),
            }],
            vec![],
            vec![],
        ),
    ];

    apply_specifier_upgrades(&mut modules);

    // ExternalFile should remain ExternalFile (only NpmPackage gets upgraded)
    assert!(matches!(
        modules[1].resolved_imports[0].target,
        ResolveResult::ExternalFile(_)
    ));
}

// -----------------------------------------------------------------------
// resolve_dynamic_patterns: edge cases
// -----------------------------------------------------------------------

#[test]
fn dynamic_patterns_prefix_without_suffix() {
    let from_dir = Path::new("/project/src");
    let patterns = vec![DynamicImportPattern {
        prefix: "./pages/".into(),
        suffix: None,
        span: dummy_span(),
    }];
    let canonical_paths = vec![
        PathBuf::from("/project/src/pages/Home.tsx"),
        PathBuf::from("/project/src/pages/About.tsx"),
        PathBuf::from("/project/src/utils.ts"),
    ];
    let files = vec![
        DiscoveredFile {
            id: FileId(0),
            path: PathBuf::from("/project/src/pages/Home.tsx"),
            size_bytes: 100,
        },
        DiscoveredFile {
            id: FileId(1),
            path: PathBuf::from("/project/src/pages/About.tsx"),
            size_bytes: 100,
        },
        DiscoveredFile {
            id: FileId(2),
            path: PathBuf::from("/project/src/utils.ts"),
            size_bytes: 100,
        },
    ];

    let result = resolve_dynamic_patterns(from_dir, &patterns, &canonical_paths, &files);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].1.len(), 2);
    assert!(result[0].1.contains(&FileId(0)));
    assert!(result[0].1.contains(&FileId(1)));
}

#[test]
fn dynamic_patterns_empty_canonical_paths() {
    let from_dir = Path::new("/project/src");
    let patterns = vec![DynamicImportPattern {
        prefix: "./locales/".into(),
        suffix: Some(".json".into()),
        span: dummy_span(),
    }];

    let result = resolve_dynamic_patterns(from_dir, &patterns, &[], &[]);
    assert!(result.is_empty());
}

// -----------------------------------------------------------------------
// resolve_single_require: edge cases
// -----------------------------------------------------------------------

#[test]
fn require_destructured_empty_names_uses_namespace() {
    // Empty destructured_names means the whole module is imported
    with_empty_ctx(|ctx| {
        let req = RequireCallInfo {
            source: "path".into(),
            span: dummy_span(),
            destructured_names: vec![],
            local_name: None,
        };
        let file = Path::new("/project/src/app.js");
        let result = resolve_single_require(ctx, file, &req);

        assert_eq!(result.len(), 1);
        assert!(matches!(
            result[0].info.imported_name,
            ImportedName::Namespace
        ));
    });
}

// -----------------------------------------------------------------------
// resolve_single_dynamic_import: edge cases
// -----------------------------------------------------------------------

#[test]
fn dynamic_import_empty_destructured_and_no_local_is_side_effect() {
    with_empty_ctx(|ctx| {
        let imp = DynamicImportInfo {
            source: "./init".into(),
            span: dummy_span(),
            destructured_names: vec![],
            local_name: None,
        };
        let file = Path::new("/project/src/app.ts");
        let result = resolve_single_dynamic_import(ctx, file, &imp);

        assert_eq!(result.len(), 1);
        assert!(matches!(
            result[0].info.imported_name,
            ImportedName::SideEffect
        ));
        assert_eq!(result[0].info.local_name, "");
    });
}

#[test]
fn dynamic_import_preserves_source_span() {
    with_empty_ctx(|ctx| {
        let imp = DynamicImportInfo {
            source: "./lazy".into(),
            span: Span::new(42, 84),
            destructured_names: vec!["x".into()],
            local_name: None,
        };
        let file = Path::new("/project/src/app.ts");
        let result = resolve_single_dynamic_import(ctx, file, &imp);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].info.span.start, 42);
        assert_eq!(result[0].info.span.end, 84);
    });
}
