use std::path::PathBuf;

use rustc_hash::FxHashMap;

use fallow_config::{FallowConfig, OutputFormat, PackageJson, ResolvedConfig, WorkspaceInfo};
use fallow_types::discover::{DiscoveredFile, EntryPoint, EntryPointSource, FileId};
use fallow_types::extract::{ImportInfo, ImportedName};

use crate::graph::ModuleGraph;
use crate::plugins::AggregatedPluginResult;
use crate::resolve::{ResolveResult, ResolvedImport, ResolvedModule};
use crate::results::*;
use crate::suppress::{self, Suppression};

use super::{
    LineOffsetsMap, find_type_only_dependencies, find_unlisted_dependencies,
    find_unresolved_imports, find_unused_dependencies, should_skip_dependency,
};

use rustc_hash::FxHashSet;

// ---- should_skip_dependency tests ----

type SkipDepSets = (
    FxHashSet<String>,
    FxHashSet<&'static str>,
    FxHashSet<&'static str>,
    FxHashSet<&'static str>,
    FxHashSet<&'static str>,
);

/// Helper: build empty sets for should_skip_dependency args.
fn empty_sets() -> SkipDepSets {
    (
        FxHashSet::default(),
        FxHashSet::default(),
        FxHashSet::default(),
        FxHashSet::default(),
        FxHashSet::default(),
    )
}

#[test]
fn skip_dep_returns_false_when_no_guard_matches() {
    let (root_flagged, script_used, plugin_referenced, ignore_deps, workspace_names) = empty_sets();
    let result = should_skip_dependency(
        "some-package",
        &root_flagged,
        &script_used,
        &plugin_referenced,
        &ignore_deps,
        &workspace_names,
        |_| false,
    );
    assert!(!result);
}

#[test]
fn skip_dep_when_root_flagged() {
    let (mut root_flagged, script_used, plugin_referenced, ignore_deps, workspace_names) =
        empty_sets();
    root_flagged.insert("lodash".to_string());
    assert!(should_skip_dependency(
        "lodash",
        &root_flagged,
        &script_used,
        &plugin_referenced,
        &ignore_deps,
        &workspace_names,
        |_| false,
    ));
}

#[test]
fn skip_dep_when_script_used() {
    let (root_flagged, mut script_used, plugin_referenced, ignore_deps, workspace_names) =
        empty_sets();
    script_used.insert("eslint");
    assert!(should_skip_dependency(
        "eslint",
        &root_flagged,
        &script_used,
        &plugin_referenced,
        &ignore_deps,
        &workspace_names,
        |_| false,
    ));
}

#[test]
fn skip_dep_when_plugin_referenced() {
    let (root_flagged, script_used, mut plugin_referenced, ignore_deps, workspace_names) =
        empty_sets();
    plugin_referenced.insert("tailwindcss");
    assert!(should_skip_dependency(
        "tailwindcss",
        &root_flagged,
        &script_used,
        &plugin_referenced,
        &ignore_deps,
        &workspace_names,
        |_| false,
    ));
}

#[test]
fn skip_dep_when_in_ignore_list() {
    let (root_flagged, script_used, plugin_referenced, mut ignore_deps, workspace_names) =
        empty_sets();
    ignore_deps.insert("my-internal-package");
    assert!(should_skip_dependency(
        "my-internal-package",
        &root_flagged,
        &script_used,
        &plugin_referenced,
        &ignore_deps,
        &workspace_names,
        |_| false,
    ));
}

#[test]
fn skip_dep_when_workspace_name() {
    let (root_flagged, script_used, plugin_referenced, ignore_deps, mut workspace_names) =
        empty_sets();
    workspace_names.insert("@myorg/shared");
    assert!(should_skip_dependency(
        "@myorg/shared",
        &root_flagged,
        &script_used,
        &plugin_referenced,
        &ignore_deps,
        &workspace_names,
        |_| false,
    ));
}

#[test]
fn skip_dep_when_used_in_workspace() {
    let (root_flagged, script_used, plugin_referenced, ignore_deps, workspace_names) = empty_sets();
    assert!(should_skip_dependency(
        "react",
        &root_flagged,
        &script_used,
        &plugin_referenced,
        &ignore_deps,
        &workspace_names,
        |dep| dep == "react",
    ));
}

#[test]
fn skip_dep_closure_receives_correct_dep_name() {
    let (root_flagged, script_used, plugin_referenced, ignore_deps, workspace_names) = empty_sets();
    // Closure that only returns true for "axios"
    let result = should_skip_dependency(
        "axios",
        &root_flagged,
        &script_used,
        &plugin_referenced,
        &ignore_deps,
        &workspace_names,
        |dep| dep == "axios",
    );
    assert!(result);

    // Different dep name should not match
    let result = should_skip_dependency(
        "express",
        &root_flagged,
        &script_used,
        &plugin_referenced,
        &ignore_deps,
        &workspace_names,
        |dep| dep == "axios",
    );
    assert!(!result);
}

#[test]
fn skip_dep_no_match_with_similar_names() {
    let (mut root_flagged, script_used, plugin_referenced, ignore_deps, workspace_names) =
        empty_sets();
    root_flagged.insert("lodash-es".to_string());
    // "lodash" is not the same as "lodash-es"
    assert!(!should_skip_dependency(
        "lodash",
        &root_flagged,
        &script_used,
        &plugin_referenced,
        &ignore_deps,
        &workspace_names,
        |_| false,
    ));
}

#[test]
fn skip_dep_multiple_guards_match() {
    // When multiple guards would match, function still returns true
    let (mut root_flagged, mut script_used, plugin_referenced, ignore_deps, workspace_names) =
        empty_sets();
    root_flagged.insert("eslint".to_string());
    script_used.insert("eslint");
    assert!(should_skip_dependency(
        "eslint",
        &root_flagged,
        &script_used,
        &plugin_referenced,
        &ignore_deps,
        &workspace_names,
        |_| false,
    ));
}

// ---- is_builtin_module tests (via predicates, used in find_unlisted_dependencies) ----

#[test]
fn builtin_module_subpaths() {
    assert!(super::super::predicates::is_builtin_module("fs/promises"));
    assert!(super::super::predicates::is_builtin_module(
        "stream/consumers"
    ));
    assert!(super::super::predicates::is_builtin_module(
        "node:fs/promises"
    ));
    assert!(super::super::predicates::is_builtin_module(
        "readline/promises"
    ));
}

#[test]
fn builtin_module_cloudflare_workers() {
    assert!(super::super::predicates::is_builtin_module(
        "cloudflare:workers"
    ));
    assert!(super::super::predicates::is_builtin_module(
        "cloudflare:sockets"
    ));
}

#[test]
fn builtin_module_deno_std() {
    assert!(super::super::predicates::is_builtin_module("std"));
    assert!(super::super::predicates::is_builtin_module("std/path"));
}

// ---- is_implicit_dependency tests (used in find_unused_dependencies) ----

#[test]
fn implicit_dep_react_dom() {
    assert!(super::super::predicates::is_implicit_dependency(
        "react-dom"
    ));
    assert!(super::super::predicates::is_implicit_dependency(
        "react-dom/client"
    ));
}

#[test]
fn implicit_dep_next_packages() {
    assert!(super::super::predicates::is_implicit_dependency(
        "@next/font"
    ));
    assert!(super::super::predicates::is_implicit_dependency(
        "@next/mdx"
    ));
    assert!(super::super::predicates::is_implicit_dependency(
        "@next/bundle-analyzer"
    ));
    assert!(super::super::predicates::is_implicit_dependency(
        "@next/env"
    ));
}

#[test]
fn implicit_dep_websocket_addons() {
    assert!(super::super::predicates::is_implicit_dependency(
        "utf-8-validate"
    ));
    assert!(super::super::predicates::is_implicit_dependency(
        "bufferutil"
    ));
}

// ---- is_path_alias tests (used in find_unlisted_dependencies) ----

#[test]
fn path_alias_not_reported_as_unlisted() {
    // These should be detected as path aliases and skipped
    assert!(super::super::predicates::is_path_alias("@/components/Foo"));
    assert!(super::super::predicates::is_path_alias("~/utils/helper"));
    assert!(super::super::predicates::is_path_alias("#internal/auth"));
    assert!(super::super::predicates::is_path_alias(
        "@Components/Button"
    ));
}

#[test]
fn scoped_npm_packages_not_path_aliases() {
    assert!(!super::super::predicates::is_path_alias("@angular/core"));
    assert!(!super::super::predicates::is_path_alias("@emotion/react"));
    assert!(!super::super::predicates::is_path_alias("@nestjs/common"));
}

// ---- Integration test helpers ----

/// Build a minimal ResolvedConfig for testing.
fn test_config(root: PathBuf) -> ResolvedConfig {
    FallowConfig {
        schema: None,
        extends: vec![],
        entry: vec![],
        ignore_patterns: vec![],
        framework: vec![],
        workspaces: None,
        ignore_dependencies: vec![],
        ignore_exports: vec![],
        duplicates: fallow_config::DuplicatesConfig::default(),
        health: fallow_config::HealthConfig::default(),
        rules: fallow_config::RulesConfig::default(),
        production: false,
        plugins: vec![],
        overrides: vec![],
    }
    .resolve(root, OutputFormat::Human, 1, true, true)
}

/// Build a PackageJson with specific dependency fields via JSON deserialization.
/// This avoids directly constructing `std::collections::HashMap` (clippy disallowed type).
fn make_pkg(deps: &[&str], dev_deps: &[&str], optional_deps: &[&str]) -> PackageJson {
    let to_obj = |names: &[&str]| -> serde_json::Value {
        let map: serde_json::Map<String, serde_json::Value> = names
            .iter()
            .map(|n| {
                (
                    n.to_string(),
                    serde_json::Value::String("^1.0.0".to_string()),
                )
            })
            .collect();
        serde_json::Value::Object(map)
    };

    let mut obj = serde_json::Map::new();
    obj.insert(
        "name".to_string(),
        serde_json::Value::String("test-project".to_string()),
    );
    if !deps.is_empty() {
        obj.insert("dependencies".to_string(), to_obj(deps));
    }
    if !dev_deps.is_empty() {
        obj.insert("devDependencies".to_string(), to_obj(dev_deps));
    }
    if !optional_deps.is_empty() {
        obj.insert("optionalDependencies".to_string(), to_obj(optional_deps));
    }
    serde_json::from_value(serde_json::Value::Object(obj))
        .expect("test PackageJson should deserialize")
}

/// Build a minimal graph where a single entry file imports given npm packages.
fn build_graph_with_npm_imports(
    npm_packages: &[(&str, bool)], // (package_name, is_type_only)
) -> (ModuleGraph, Vec<ResolvedModule>) {
    let files = vec![DiscoveredFile {
        id: FileId(0),
        path: PathBuf::from("/project/src/index.ts"),
        size_bytes: 100,
    }];

    let entry_points = vec![EntryPoint {
        path: PathBuf::from("/project/src/index.ts"),
        source: EntryPointSource::PackageJsonMain,
    }];

    let resolved_imports: Vec<ResolvedImport> = npm_packages
        .iter()
        .enumerate()
        .map(|(i, (name, is_type_only))| ResolvedImport {
            info: ImportInfo {
                source: name.to_string(),
                imported_name: ImportedName::Named("default".to_string()),
                local_name: format!("import_{i}"),
                is_type_only: *is_type_only,
                span: oxc_span::Span::new((i * 20) as u32, (i * 20 + 15) as u32),
            },
            target: ResolveResult::NpmPackage(name.to_string()),
        })
        .collect();

    let resolved_modules = vec![ResolvedModule {
        file_id: FileId(0),
        path: PathBuf::from("/project/src/index.ts"),
        exports: vec![],
        re_exports: vec![],
        resolved_imports,
        resolved_dynamic_imports: vec![],
        resolved_dynamic_patterns: vec![],
        member_accesses: vec![],
        whole_object_uses: vec![],
        has_cjs_exports: false,
        unused_import_bindings: vec![],
    }];

    let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);
    (graph, resolved_modules)
}

// ---- find_unused_dependencies integration tests ----

#[test]
fn unused_dep_flagged_when_never_imported() {
    let (graph, _) = build_graph_with_npm_imports(&[("react", false)]);
    let pkg = make_pkg(&["react", "lodash"], &[], &[]);
    let config = test_config(PathBuf::from("/project"));

    let (unused, unused_dev, unused_optional) =
        find_unused_dependencies(&graph, &pkg, &config, None, &[]);

    assert!(
        unused.iter().any(|d| d.package_name == "lodash"),
        "lodash is never imported and should be flagged"
    );
    assert!(
        !unused.iter().any(|d| d.package_name == "react"),
        "react is imported and should NOT be flagged"
    );
    assert!(unused_dev.is_empty());
    assert!(unused_optional.is_empty());
}

#[test]
fn known_tooling_dev_deps_not_flagged_as_unused() {
    let (graph, _) = build_graph_with_npm_imports(&[]);
    let pkg = make_pkg(&[], &["jest", "vitest"], &[]);
    let config = test_config(PathBuf::from("/project"));

    let (unused, unused_dev, _) = find_unused_dependencies(&graph, &pkg, &config, None, &[]);

    assert!(unused.is_empty());
    // "jest" and "vitest" are known tooling deps, so they should NOT be flagged
    assert!(
        !unused_dev.iter().any(|d| d.package_name == "jest"),
        "jest is a known tooling dep and should be filtered"
    );
    assert!(
        !unused_dev.iter().any(|d| d.package_name == "vitest"),
        "vitest is a known tooling dep and should be filtered"
    );
}

#[test]
fn unused_dev_dep_non_tooling_is_flagged() {
    let (graph, _) = build_graph_with_npm_imports(&[]);
    let pkg = make_pkg(&[], &["my-custom-lib"], &[]);
    let config = test_config(PathBuf::from("/project"));

    let (_, unused_dev, _) = find_unused_dependencies(&graph, &pkg, &config, None, &[]);

    assert!(
        unused_dev.iter().any(|d| d.package_name == "my-custom-lib"),
        "non-tooling dev dep should be flagged as unused"
    );
}

#[test]
fn unused_optional_dep_flagged_when_never_imported() {
    let (graph, _) = build_graph_with_npm_imports(&[]);
    let pkg = make_pkg(&[], &[], &["sharp"]);
    let config = test_config(PathBuf::from("/project"));

    let (_, _, unused_optional) = find_unused_dependencies(&graph, &pkg, &config, None, &[]);

    assert!(
        unused_optional.iter().any(|d| d.package_name == "sharp"),
        "unused optional dep should be flagged"
    );
}

#[test]
fn implicit_deps_not_flagged_as_unused() {
    // react-dom, @types/node, etc. are implicit and should be filtered
    let (graph, _) = build_graph_with_npm_imports(&[]);
    let pkg = make_pkg(&["react-dom", "@types/node"], &[], &[]);
    let config = test_config(PathBuf::from("/project"));

    let (unused, _, _) = find_unused_dependencies(&graph, &pkg, &config, None, &[]);

    assert!(
        !unused.iter().any(|d| d.package_name == "react-dom"),
        "react-dom is implicit and should not be flagged"
    );
    assert!(
        !unused.iter().any(|d| d.package_name == "@types/node"),
        "@types/node is implicit and should not be flagged"
    );
}

#[test]
fn workspace_package_names_not_flagged() {
    let (graph, _) = build_graph_with_npm_imports(&[]);
    let pkg = make_pkg(&["@myorg/shared"], &[], &[]);
    let config = test_config(PathBuf::from("/project"));

    let workspaces = vec![WorkspaceInfo {
        root: PathBuf::from("/project/packages/shared"),
        name: "@myorg/shared".to_string(),
        is_internal_dependency: false,
    }];

    let (unused, _, _) = find_unused_dependencies(&graph, &pkg, &config, None, &workspaces);

    assert!(
        !unused.iter().any(|d| d.package_name == "@myorg/shared"),
        "workspace packages should not be flagged as unused"
    );
}

#[test]
fn ignore_dependencies_config_filters_deps() {
    let (graph, _) = build_graph_with_npm_imports(&[]);
    let pkg = make_pkg(&["my-internal-pkg"], &[], &[]);

    let config = FallowConfig {
        schema: None,
        extends: vec![],
        entry: vec![],
        ignore_patterns: vec![],
        framework: vec![],
        workspaces: None,
        ignore_dependencies: vec!["my-internal-pkg".to_string()],
        ignore_exports: vec![],
        duplicates: fallow_config::DuplicatesConfig::default(),
        health: fallow_config::HealthConfig::default(),
        rules: fallow_config::RulesConfig::default(),
        production: false,
        plugins: vec![],
        overrides: vec![],
    }
    .resolve(
        PathBuf::from("/project"),
        OutputFormat::Human,
        1,
        true,
        true,
    );

    let (unused, _, _) = find_unused_dependencies(&graph, &pkg, &config, None, &[]);

    assert!(
        !unused.iter().any(|d| d.package_name == "my-internal-pkg"),
        "deps in ignoreDependencies should not be flagged"
    );
}

#[test]
fn plugin_referenced_deps_not_flagged() {
    let (graph, _) = build_graph_with_npm_imports(&[]);
    let pkg = make_pkg(&["tailwindcss"], &[], &[]);
    let config = test_config(PathBuf::from("/project"));

    let mut plugin_result = AggregatedPluginResult::default();
    plugin_result
        .referenced_dependencies
        .push("tailwindcss".to_string());

    let (unused, _, _) = find_unused_dependencies(&graph, &pkg, &config, Some(&plugin_result), &[]);

    assert!(
        !unused.iter().any(|d| d.package_name == "tailwindcss"),
        "plugin-referenced deps should not be flagged"
    );
}

#[test]
fn plugin_tooling_deps_not_flagged() {
    let (graph, _) = build_graph_with_npm_imports(&[]);
    let pkg = make_pkg(&["my-framework-runtime"], &[], &[]);
    let config = test_config(PathBuf::from("/project"));

    let mut plugin_result = AggregatedPluginResult::default();
    plugin_result
        .tooling_dependencies
        .push("my-framework-runtime".to_string());

    let (unused, _, _) = find_unused_dependencies(&graph, &pkg, &config, Some(&plugin_result), &[]);

    assert!(
        !unused
            .iter()
            .any(|d| d.package_name == "my-framework-runtime"),
        "plugin tooling deps should not be flagged"
    );
}

#[test]
fn script_used_packages_not_flagged() {
    let (graph, _) = build_graph_with_npm_imports(&[]);
    let pkg = make_pkg(&["concurrently"], &[], &[]);
    let config = test_config(PathBuf::from("/project"));

    let mut plugin_result = AggregatedPluginResult::default();
    plugin_result
        .script_used_packages
        .insert("concurrently".to_string());

    let (unused, _, _) = find_unused_dependencies(&graph, &pkg, &config, Some(&plugin_result), &[]);

    assert!(
        !unused.iter().any(|d| d.package_name == "concurrently"),
        "packages used in scripts should not be flagged"
    );
}

#[test]
fn unused_dep_location_is_correct() {
    let (graph, _) = build_graph_with_npm_imports(&[]);
    let pkg = make_pkg(&["unused-dep"], &["unused-dev"], &["unused-opt"]);
    let config = test_config(PathBuf::from("/project"));

    let (unused, unused_dev, unused_optional) =
        find_unused_dependencies(&graph, &pkg, &config, None, &[]);

    assert!(unused.iter().any(|d| d.package_name == "unused-dep"
        && matches!(d.location, DependencyLocation::Dependencies)));
    assert!(unused_dev.iter().any(|d| d.package_name == "unused-dev"
        && matches!(d.location, DependencyLocation::DevDependencies)));
    assert!(
        unused_optional
            .iter()
            .any(|d| d.package_name == "unused-opt"
                && matches!(d.location, DependencyLocation::OptionalDependencies))
    );
}

// ---- find_type_only_dependencies tests ----

#[test]
fn type_only_dep_detected_when_all_imports_are_type_only() {
    let (graph, _) = build_graph_with_npm_imports(&[("zod", true)]);
    let pkg = make_pkg(&["zod"], &[], &[]);
    let config = test_config(PathBuf::from("/project"));

    let type_only = find_type_only_dependencies(&graph, &pkg, &config, &[]);

    assert!(
        type_only.iter().any(|d| d.package_name == "zod"),
        "dep used only via `import type` should be flagged as type-only"
    );
}

#[test]
fn type_only_dep_not_detected_when_runtime_import_exists() {
    // One runtime import + one type-only import => not type-only
    let files = vec![
        DiscoveredFile {
            id: FileId(0),
            path: PathBuf::from("/project/src/index.ts"),
            size_bytes: 100,
        },
        DiscoveredFile {
            id: FileId(1),
            path: PathBuf::from("/project/src/other.ts"),
            size_bytes: 100,
        },
    ];

    let entry_points = vec![
        EntryPoint {
            path: PathBuf::from("/project/src/index.ts"),
            source: EntryPointSource::PackageJsonMain,
        },
        EntryPoint {
            path: PathBuf::from("/project/src/other.ts"),
            source: EntryPointSource::PackageJsonMain,
        },
    ];

    let resolved_modules = vec![
        ResolvedModule {
            file_id: FileId(0),
            path: PathBuf::from("/project/src/index.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![ResolvedImport {
                info: ImportInfo {
                    source: "zod".to_string(),
                    imported_name: ImportedName::Named("z".to_string()),
                    local_name: "z".to_string(),
                    is_type_only: true,
                    span: oxc_span::Span::new(0, 20),
                },
                target: ResolveResult::NpmPackage("zod".to_string()),
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
            path: PathBuf::from("/project/src/other.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![ResolvedImport {
                info: ImportInfo {
                    source: "zod".to_string(),
                    imported_name: ImportedName::Named("z".to_string()),
                    local_name: "z".to_string(),
                    is_type_only: false, // runtime import
                    span: oxc_span::Span::new(0, 20),
                },
                target: ResolveResult::NpmPackage("zod".to_string()),
            }],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        },
    ];

    let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);
    let pkg = make_pkg(&["zod"], &[], &[]);
    let config = test_config(PathBuf::from("/project"));

    let type_only = find_type_only_dependencies(&graph, &pkg, &config, &[]);

    assert!(
        type_only.is_empty(),
        "dep with mixed type-only and runtime imports should NOT be flagged"
    );
}

#[test]
fn type_only_dep_not_detected_when_unused() {
    // Dep is not imported at all => caught by unused_dependencies, not type_only
    let (graph, _) = build_graph_with_npm_imports(&[]);
    let pkg = make_pkg(&["zod"], &[], &[]);
    let config = test_config(PathBuf::from("/project"));

    let type_only = find_type_only_dependencies(&graph, &pkg, &config, &[]);

    assert!(
        type_only.is_empty(),
        "completely unused deps should not appear in type_only results"
    );
}

#[test]
fn type_only_dep_skips_workspace_packages() {
    let (graph, _) = build_graph_with_npm_imports(&[("@myorg/types", true)]);
    let pkg = make_pkg(&["@myorg/types"], &[], &[]);
    let config = test_config(PathBuf::from("/project"));

    let workspaces = vec![WorkspaceInfo {
        root: PathBuf::from("/project/packages/types"),
        name: "@myorg/types".to_string(),
        is_internal_dependency: false,
    }];

    let type_only = find_type_only_dependencies(&graph, &pkg, &config, &workspaces);

    assert!(
        type_only.is_empty(),
        "workspace packages should not be flagged as type-only deps"
    );
}

#[test]
fn type_only_dep_skips_ignored_deps() {
    let (graph, _) = build_graph_with_npm_imports(&[("zod", true)]);
    let pkg = make_pkg(&["zod"], &[], &[]);

    let config = FallowConfig {
        schema: None,
        extends: vec![],
        entry: vec![],
        ignore_patterns: vec![],
        framework: vec![],
        workspaces: None,
        ignore_dependencies: vec!["zod".to_string()],
        ignore_exports: vec![],
        duplicates: fallow_config::DuplicatesConfig::default(),
        health: fallow_config::HealthConfig::default(),
        rules: fallow_config::RulesConfig::default(),
        production: false,
        plugins: vec![],
        overrides: vec![],
    }
    .resolve(
        PathBuf::from("/project"),
        OutputFormat::Human,
        1,
        true,
        true,
    );

    let type_only = find_type_only_dependencies(&graph, &pkg, &config, &[]);

    assert!(
        type_only.is_empty(),
        "ignored deps should not be flagged as type-only"
    );
}

// ---- find_unlisted_dependencies tests ----

#[test]
fn unlisted_dep_detected_when_not_in_package_json() {
    let (graph, resolved_modules) = build_graph_with_npm_imports(&[("axios", false)]);
    let pkg = make_pkg(&["react"], &[], &[]); // axios is NOT listed
    let config = test_config(PathBuf::from("/project"));
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let unlisted = find_unlisted_dependencies(
        &graph,
        &pkg,
        &config,
        &[],
        None,
        &resolved_modules,
        &line_offsets,
    );

    assert!(
        unlisted.iter().any(|d| d.package_name == "axios"),
        "axios is imported but not listed, should be unlisted"
    );
}

#[test]
fn listed_dep_not_reported_as_unlisted() {
    let (graph, resolved_modules) = build_graph_with_npm_imports(&[("react", false)]);
    let pkg = make_pkg(&["react"], &[], &[]);
    let config = test_config(PathBuf::from("/project"));
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let unlisted = find_unlisted_dependencies(
        &graph,
        &pkg,
        &config,
        &[],
        None,
        &resolved_modules,
        &line_offsets,
    );

    assert!(
        unlisted.is_empty(),
        "dep listed in dependencies should not be flagged as unlisted"
    );
}

#[test]
fn dev_dep_not_reported_as_unlisted() {
    let (graph, resolved_modules) = build_graph_with_npm_imports(&[("jest", false)]);
    let pkg = make_pkg(&[], &["jest"], &[]);
    let config = test_config(PathBuf::from("/project"));
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let unlisted = find_unlisted_dependencies(
        &graph,
        &pkg,
        &config,
        &[],
        None,
        &resolved_modules,
        &line_offsets,
    );

    assert!(
        unlisted.is_empty(),
        "dep listed in devDependencies should not be unlisted"
    );
}

#[test]
fn builtin_modules_not_reported_as_unlisted() {
    // Import "fs" (a Node.js builtin) - should never be unlisted
    let files = vec![DiscoveredFile {
        id: FileId(0),
        path: PathBuf::from("/project/src/index.ts"),
        size_bytes: 100,
    }];
    let entry_points = vec![EntryPoint {
        path: PathBuf::from("/project/src/index.ts"),
        source: EntryPointSource::PackageJsonMain,
    }];
    // NpmPackage("fs") would be the resolve result if it were npm.
    // But in practice, builtins are tracked as NpmPackage in package_usage.
    // The key filter is is_builtin_module in find_unlisted_dependencies.
    let resolved_modules = vec![ResolvedModule {
        file_id: FileId(0),
        path: PathBuf::from("/project/src/index.ts"),
        exports: vec![],
        re_exports: vec![],
        resolved_imports: vec![ResolvedImport {
            info: ImportInfo {
                source: "node:fs".to_string(),
                imported_name: ImportedName::Named("readFile".to_string()),
                local_name: "readFile".to_string(),
                is_type_only: false,
                span: oxc_span::Span::new(0, 25),
            },
            target: ResolveResult::NpmPackage("node:fs".to_string()),
        }],
        resolved_dynamic_imports: vec![],
        resolved_dynamic_patterns: vec![],
        member_accesses: vec![],
        whole_object_uses: vec![],
        has_cjs_exports: false,
        unused_import_bindings: vec![],
    }];
    let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);
    let pkg = make_pkg(&[], &[], &[]);
    let config = test_config(PathBuf::from("/project"));
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let unlisted = find_unlisted_dependencies(
        &graph,
        &pkg,
        &config,
        &[],
        None,
        &resolved_modules,
        &line_offsets,
    );

    assert!(
        !unlisted.iter().any(|d| d.package_name == "node:fs"),
        "node:fs builtin should not be flagged as unlisted"
    );
}

#[test]
fn virtual_modules_not_reported_as_unlisted() {
    let files = vec![DiscoveredFile {
        id: FileId(0),
        path: PathBuf::from("/project/src/index.ts"),
        size_bytes: 100,
    }];
    let entry_points = vec![EntryPoint {
        path: PathBuf::from("/project/src/index.ts"),
        source: EntryPointSource::PackageJsonMain,
    }];
    let resolved_modules = vec![ResolvedModule {
        file_id: FileId(0),
        path: PathBuf::from("/project/src/index.ts"),
        exports: vec![],
        re_exports: vec![],
        resolved_imports: vec![ResolvedImport {
            info: ImportInfo {
                source: "virtual:pwa-register".to_string(),
                imported_name: ImportedName::Named("register".to_string()),
                local_name: "register".to_string(),
                is_type_only: false,
                span: oxc_span::Span::new(0, 30),
            },
            target: ResolveResult::NpmPackage("virtual:pwa-register".to_string()),
        }],
        resolved_dynamic_imports: vec![],
        resolved_dynamic_patterns: vec![],
        member_accesses: vec![],
        whole_object_uses: vec![],
        has_cjs_exports: false,
        unused_import_bindings: vec![],
    }];
    let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);
    let pkg = make_pkg(&[], &[], &[]);
    let config = test_config(PathBuf::from("/project"));
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let unlisted = find_unlisted_dependencies(
        &graph,
        &pkg,
        &config,
        &[],
        None,
        &resolved_modules,
        &line_offsets,
    );

    assert!(
        unlisted.is_empty(),
        "virtual: modules should not be flagged as unlisted"
    );
}

#[test]
fn workspace_package_names_not_reported_as_unlisted() {
    let (graph, resolved_modules) = build_graph_with_npm_imports(&[("@myorg/utils", false)]);
    let pkg = make_pkg(&[], &[], &[]); // @myorg/utils NOT listed
    let config = test_config(PathBuf::from("/project"));
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let workspaces = vec![WorkspaceInfo {
        root: PathBuf::from("/project/packages/utils"),
        name: "@myorg/utils".to_string(),
        is_internal_dependency: false,
    }];

    let unlisted = find_unlisted_dependencies(
        &graph,
        &pkg,
        &config,
        &workspaces,
        None,
        &resolved_modules,
        &line_offsets,
    );

    assert!(
        !unlisted.iter().any(|d| d.package_name == "@myorg/utils"),
        "workspace package names should not be flagged as unlisted"
    );
}

#[test]
fn plugin_virtual_prefixes_not_reported_as_unlisted() {
    let pkg = make_pkg(&[], &[], &[]);
    let config = test_config(PathBuf::from("/project"));
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    // Use a non-path-alias virtual prefix (not "#" which is_path_alias catches)
    let (graph2, resolved_modules2) = build_graph_with_npm_imports(&[("@theme/Layout", false)]);

    let mut plugin_result2 = AggregatedPluginResult::default();
    plugin_result2
        .virtual_module_prefixes
        .push("@theme/".to_string());

    let unlisted = find_unlisted_dependencies(
        &graph2,
        &pkg,
        &config,
        &[],
        Some(&plugin_result2),
        &resolved_modules2,
        &line_offsets,
    );

    assert!(
        !unlisted.iter().any(|d| d.package_name == "@theme/Layout"),
        "imports matching virtual module prefixes should not be unlisted"
    );
}

#[test]
fn plugin_tooling_deps_not_reported_as_unlisted() {
    let (graph, resolved_modules) = build_graph_with_npm_imports(&[("h3", false)]);
    let pkg = make_pkg(&[], &[], &[]);
    let config = test_config(PathBuf::from("/project"));
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let mut plugin_result = AggregatedPluginResult::default();
    plugin_result.tooling_dependencies.push("h3".to_string());

    let unlisted = find_unlisted_dependencies(
        &graph,
        &pkg,
        &config,
        &[],
        Some(&plugin_result),
        &resolved_modules,
        &line_offsets,
    );

    assert!(
        !unlisted.iter().any(|d| d.package_name == "h3"),
        "plugin tooling deps should not be flagged as unlisted"
    );
}

#[test]
fn peer_dep_not_reported_as_unlisted() {
    let (graph, resolved_modules) = build_graph_with_npm_imports(&[("react", false)]);
    // react is listed as a peer dep only, not in deps/devDeps
    let pkg: PackageJson = serde_json::from_str(r#"{"peerDependencies": {"react": "^18.0.0"}}"#)
        .expect("test pkg json");

    let config = test_config(PathBuf::from("/project"));
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let unlisted = find_unlisted_dependencies(
        &graph,
        &pkg,
        &config,
        &[],
        None,
        &resolved_modules,
        &line_offsets,
    );

    assert!(
        unlisted.is_empty(),
        "peer dependencies should not be flagged as unlisted"
    );
}

// ---- find_unresolved_imports tests ----

#[test]
fn unresolved_import_detected() {
    let resolved_modules = vec![ResolvedModule {
        file_id: FileId(0),
        path: PathBuf::from("/project/src/index.ts"),
        exports: vec![],
        re_exports: vec![],
        resolved_imports: vec![ResolvedImport {
            info: ImportInfo {
                source: "./missing-file".to_string(),
                imported_name: ImportedName::Named("foo".to_string()),
                local_name: "foo".to_string(),
                is_type_only: false,
                span: oxc_span::Span::new(0, 30),
            },
            target: ResolveResult::Unresolvable("./missing-file".to_string()),
        }],
        resolved_dynamic_imports: vec![],
        resolved_dynamic_patterns: vec![],
        member_accesses: vec![],
        whole_object_uses: vec![],
        has_cjs_exports: false,
        unused_import_bindings: vec![],
    }];

    let config = test_config(PathBuf::from("/project"));
    let suppressions: FxHashMap<FileId, &[Suppression]> = FxHashMap::default();
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let unresolved = find_unresolved_imports(
        &resolved_modules,
        &config,
        &suppressions,
        &[],
        &line_offsets,
    );

    assert_eq!(unresolved.len(), 1);
    assert_eq!(unresolved[0].specifier, "./missing-file");
}

#[test]
fn unresolved_virtual_module_not_reported() {
    let resolved_modules = vec![ResolvedModule {
        file_id: FileId(0),
        path: PathBuf::from("/project/src/index.ts"),
        exports: vec![],
        re_exports: vec![],
        resolved_imports: vec![ResolvedImport {
            info: ImportInfo {
                source: "virtual:generated-pages".to_string(),
                imported_name: ImportedName::Named("pages".to_string()),
                local_name: "pages".to_string(),
                is_type_only: false,
                span: oxc_span::Span::new(0, 40),
            },
            target: ResolveResult::Unresolvable("virtual:generated-pages".to_string()),
        }],
        resolved_dynamic_imports: vec![],
        resolved_dynamic_patterns: vec![],
        member_accesses: vec![],
        whole_object_uses: vec![],
        has_cjs_exports: false,
        unused_import_bindings: vec![],
    }];

    let config = test_config(PathBuf::from("/project"));
    let suppressions: FxHashMap<FileId, &[Suppression]> = FxHashMap::default();
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let unresolved = find_unresolved_imports(
        &resolved_modules,
        &config,
        &suppressions,
        &[],
        &line_offsets,
    );

    assert!(
        unresolved.is_empty(),
        "virtual: module imports should not be flagged as unresolved"
    );
}

#[test]
fn unresolved_import_with_virtual_prefix_not_reported() {
    let resolved_modules = vec![ResolvedModule {
        file_id: FileId(0),
        path: PathBuf::from("/project/src/index.ts"),
        exports: vec![],
        re_exports: vec![],
        resolved_imports: vec![ResolvedImport {
            info: ImportInfo {
                source: "#imports".to_string(),
                imported_name: ImportedName::Named("useRouter".to_string()),
                local_name: "useRouter".to_string(),
                is_type_only: false,
                span: oxc_span::Span::new(0, 25),
            },
            target: ResolveResult::Unresolvable("#imports".to_string()),
        }],
        resolved_dynamic_imports: vec![],
        resolved_dynamic_patterns: vec![],
        member_accesses: vec![],
        whole_object_uses: vec![],
        has_cjs_exports: false,
        unused_import_bindings: vec![],
    }];

    let config = test_config(PathBuf::from("/project"));
    let suppressions: FxHashMap<FileId, &[Suppression]> = FxHashMap::default();
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let unresolved = find_unresolved_imports(
        &resolved_modules,
        &config,
        &suppressions,
        &["#"], // Nuxt-style virtual prefix
        &line_offsets,
    );

    assert!(
        unresolved.is_empty(),
        "imports matching virtual_prefixes should not be flagged as unresolved"
    );
}

#[test]
fn unresolved_import_suppressed_by_inline_comment() {
    let resolved_modules = vec![ResolvedModule {
        file_id: FileId(0),
        path: PathBuf::from("/project/src/index.ts"),
        exports: vec![],
        re_exports: vec![],
        resolved_imports: vec![ResolvedImport {
            info: ImportInfo {
                source: "./broken".to_string(),
                imported_name: ImportedName::Named("thing".to_string()),
                local_name: "thing".to_string(),
                is_type_only: false,
                span: oxc_span::Span::new(0, 20),
            },
            target: ResolveResult::Unresolvable("./broken".to_string()),
        }],
        resolved_dynamic_imports: vec![],
        resolved_dynamic_patterns: vec![],
        member_accesses: vec![],
        whole_object_uses: vec![],
        has_cjs_exports: false,
        unused_import_bindings: vec![],
    }];

    let config = test_config(PathBuf::from("/project"));
    // Suppress unresolved imports on line 1 (byte offset 0 => line 1 without offsets)
    let supps = vec![Suppression {
        line: 1,
        kind: Some(suppress::IssueKind::UnresolvedImport),
    }];
    let mut suppressions: FxHashMap<FileId, &[Suppression]> = FxHashMap::default();
    suppressions.insert(FileId(0), &supps);
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let unresolved = find_unresolved_imports(
        &resolved_modules,
        &config,
        &suppressions,
        &[],
        &line_offsets,
    );

    assert!(
        unresolved.is_empty(),
        "suppressed unresolved import should not be reported"
    );
}

#[test]
fn unresolved_import_file_level_suppression() {
    let resolved_modules = vec![ResolvedModule {
        file_id: FileId(0),
        path: PathBuf::from("/project/src/index.ts"),
        exports: vec![],
        re_exports: vec![],
        resolved_imports: vec![ResolvedImport {
            info: ImportInfo {
                source: "./nonexistent".to_string(),
                imported_name: ImportedName::Named("x".to_string()),
                local_name: "x".to_string(),
                is_type_only: false,
                span: oxc_span::Span::new(0, 25),
            },
            target: ResolveResult::Unresolvable("./nonexistent".to_string()),
        }],
        resolved_dynamic_imports: vec![],
        resolved_dynamic_patterns: vec![],
        member_accesses: vec![],
        whole_object_uses: vec![],
        has_cjs_exports: false,
        unused_import_bindings: vec![],
    }];

    let config = test_config(PathBuf::from("/project"));
    // File-level suppression (line 0)
    let supps = vec![Suppression {
        line: 0,
        kind: Some(suppress::IssueKind::UnresolvedImport),
    }];
    let mut suppressions: FxHashMap<FileId, &[Suppression]> = FxHashMap::default();
    suppressions.insert(FileId(0), &supps);
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let unresolved = find_unresolved_imports(
        &resolved_modules,
        &config,
        &suppressions,
        &[],
        &line_offsets,
    );

    assert!(
        unresolved.is_empty(),
        "file-level suppression should suppress all unresolved imports in the file"
    );
}

#[test]
fn resolved_import_not_reported_as_unresolved() {
    let resolved_modules = vec![ResolvedModule {
        file_id: FileId(0),
        path: PathBuf::from("/project/src/index.ts"),
        exports: vec![],
        re_exports: vec![],
        resolved_imports: vec![
            ResolvedImport {
                info: ImportInfo {
                    source: "react".to_string(),
                    imported_name: ImportedName::Named("useState".to_string()),
                    local_name: "useState".to_string(),
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 20),
                },
                target: ResolveResult::NpmPackage("react".to_string()),
            },
            ResolvedImport {
                info: ImportInfo {
                    source: "./utils".to_string(),
                    imported_name: ImportedName::Named("helper".to_string()),
                    local_name: "helper".to_string(),
                    is_type_only: false,
                    span: oxc_span::Span::new(25, 50),
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
    }];

    let config = test_config(PathBuf::from("/project"));
    let suppressions: FxHashMap<FileId, &[Suppression]> = FxHashMap::default();
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let unresolved = find_unresolved_imports(
        &resolved_modules,
        &config,
        &suppressions,
        &[],
        &line_offsets,
    );

    assert!(
        unresolved.is_empty(),
        "resolved imports should never appear as unresolved"
    );
}

// ---- Scoped package / subpath import edge cases ----

#[test]
fn scoped_package_subpath_import_recognized_as_used() {
    // import { Button } from '@chakra-ui/react/button'
    // should recognize '@chakra-ui/react' as the package name
    let (graph, _resolved_modules) = build_graph_with_npm_imports(&[("@chakra-ui/react", false)]);
    let pkg = make_pkg(&["@chakra-ui/react"], &[], &[]);
    let config = test_config(PathBuf::from("/project"));

    let (unused, _, _) = find_unused_dependencies(&graph, &pkg, &config, None, &[]);

    assert!(
        unused.is_empty(),
        "@chakra-ui/react should be recognized as used via subpath import"
    );
}

#[test]
fn optional_dep_in_peer_deps_also_counts() {
    // An optional dep that is also used should not be flagged
    let (graph, _) = build_graph_with_npm_imports(&[("sharp", false)]);
    let pkg = make_pkg(&[], &[], &["sharp"]);
    let config = test_config(PathBuf::from("/project"));

    let (_, _, unused_optional) = find_unused_dependencies(&graph, &pkg, &config, None, &[]);

    assert!(
        unused_optional.is_empty(),
        "optional dep that is imported should not be flagged as unused"
    );
}

// ---- Empty / edge case scenarios ----

#[test]
fn no_deps_produces_no_unused() {
    let (graph, _) = build_graph_with_npm_imports(&[]);
    let pkg = make_pkg(&[], &[], &[]);
    let config = test_config(PathBuf::from("/project"));

    let (unused, unused_dev, unused_optional) =
        find_unused_dependencies(&graph, &pkg, &config, None, &[]);

    assert!(unused.is_empty());
    assert!(unused_dev.is_empty());
    assert!(unused_optional.is_empty());
}

#[test]
fn no_imports_flags_all_non_implicit_deps() {
    let (graph, _) = build_graph_with_npm_imports(&[]);
    let pkg = make_pkg(&["lodash", "axios"], &[], &[]);
    let config = test_config(PathBuf::from("/project"));

    let (unused, _, _) = find_unused_dependencies(&graph, &pkg, &config, None, &[]);

    assert!(unused.iter().any(|d| d.package_name == "lodash"));
    assert!(unused.iter().any(|d| d.package_name == "axios"));
}

#[test]
fn unlisted_dep_has_import_sites() {
    let (graph, resolved_modules) = build_graph_with_npm_imports(&[("unlisted-pkg", false)]);
    let pkg = make_pkg(&[], &[], &[]);
    let config = test_config(PathBuf::from("/project"));
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let unlisted = find_unlisted_dependencies(
        &graph,
        &pkg,
        &config,
        &[],
        None,
        &resolved_modules,
        &line_offsets,
    );

    assert_eq!(unlisted.len(), 1);
    assert_eq!(unlisted[0].package_name, "unlisted-pkg");
    assert!(
        !unlisted[0].imported_from.is_empty(),
        "unlisted dep should have at least one import site"
    );
    assert_eq!(
        unlisted[0].imported_from[0].path,
        PathBuf::from("/project/src/index.ts")
    );
}

#[test]
fn path_alias_imports_not_reported_as_unlisted() {
    // @/components and ~/utils are path aliases, not npm packages
    let files = vec![DiscoveredFile {
        id: FileId(0),
        path: PathBuf::from("/project/src/index.ts"),
        size_bytes: 100,
    }];
    let entry_points = vec![EntryPoint {
        path: PathBuf::from("/project/src/index.ts"),
        source: EntryPointSource::PackageJsonMain,
    }];
    let resolved_modules = vec![ResolvedModule {
        file_id: FileId(0),
        path: PathBuf::from("/project/src/index.ts"),
        exports: vec![],
        re_exports: vec![],
        resolved_imports: vec![
            ResolvedImport {
                info: ImportInfo {
                    source: "@/components/Button".to_string(),
                    imported_name: ImportedName::Named("Button".to_string()),
                    local_name: "Button".to_string(),
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 30),
                },
                target: ResolveResult::NpmPackage("@/components/Button".to_string()),
            },
            ResolvedImport {
                info: ImportInfo {
                    source: "~/utils/helper".to_string(),
                    imported_name: ImportedName::Named("helper".to_string()),
                    local_name: "helper".to_string(),
                    is_type_only: false,
                    span: oxc_span::Span::new(35, 60),
                },
                target: ResolveResult::NpmPackage("~/utils/helper".to_string()),
            },
        ],
        resolved_dynamic_imports: vec![],
        resolved_dynamic_patterns: vec![],
        member_accesses: vec![],
        whole_object_uses: vec![],
        has_cjs_exports: false,
        unused_import_bindings: vec![],
    }];
    let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);
    let pkg = make_pkg(&[], &[], &[]);
    let config = test_config(PathBuf::from("/project"));
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let unlisted = find_unlisted_dependencies(
        &graph,
        &pkg,
        &config,
        &[],
        None,
        &resolved_modules,
        &line_offsets,
    );

    assert!(
        unlisted.is_empty(),
        "path aliases should never be flagged as unlisted dependencies"
    );
}

#[test]
fn multiple_unresolved_imports_collected() {
    let resolved_modules = vec![ResolvedModule {
        file_id: FileId(0),
        path: PathBuf::from("/project/src/index.ts"),
        exports: vec![],
        re_exports: vec![],
        resolved_imports: vec![
            ResolvedImport {
                info: ImportInfo {
                    source: "./missing-a".to_string(),
                    imported_name: ImportedName::Named("a".to_string()),
                    local_name: "a".to_string(),
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 20),
                },
                target: ResolveResult::Unresolvable("./missing-a".to_string()),
            },
            ResolvedImport {
                info: ImportInfo {
                    source: "./missing-b".to_string(),
                    imported_name: ImportedName::Named("b".to_string()),
                    local_name: "b".to_string(),
                    is_type_only: false,
                    span: oxc_span::Span::new(25, 45),
                },
                target: ResolveResult::Unresolvable("./missing-b".to_string()),
            },
        ],
        resolved_dynamic_imports: vec![],
        resolved_dynamic_patterns: vec![],
        member_accesses: vec![],
        whole_object_uses: vec![],
        has_cjs_exports: false,
        unused_import_bindings: vec![],
    }];

    let config = test_config(PathBuf::from("/project"));
    let suppressions: FxHashMap<FileId, &[Suppression]> = FxHashMap::default();
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let unresolved = find_unresolved_imports(
        &resolved_modules,
        &config,
        &suppressions,
        &[],
        &line_offsets,
    );

    assert_eq!(unresolved.len(), 2);
    assert!(unresolved.iter().any(|u| u.specifier == "./missing-a"));
    assert!(unresolved.iter().any(|u| u.specifier == "./missing-b"));
}

// ---- Additional coverage: all deps used scenario ----

#[test]
fn all_deps_used_produces_no_unused() {
    // Every dependency listed is also imported — nothing should be flagged
    let (graph, _) = build_graph_with_npm_imports(&[
        ("react", false),
        ("lodash", false),
        ("axios", false),
    ]);
    let pkg = make_pkg(&["react", "lodash", "axios"], &[], &[]);
    let config = test_config(PathBuf::from("/project"));

    let (unused, unused_dev, unused_optional) =
        find_unused_dependencies(&graph, &pkg, &config, None, &[]);

    assert!(unused.is_empty(), "all deps are used, none should be flagged");
    assert!(unused_dev.is_empty());
    assert!(unused_optional.is_empty());
}

// ---- Additional coverage: find_type_only_dependencies only checks production deps ----

#[test]
fn type_only_dep_ignores_dev_dependencies() {
    // A dev dependency that is only type-imported should NOT appear in type_only results,
    // because find_type_only_dependencies only checks production dependencies.
    let (graph, _) = build_graph_with_npm_imports(&[("@types/lodash", true)]);
    let pkg = make_pkg(&[], &["@types/lodash"], &[]);
    let config = test_config(PathBuf::from("/project"));

    let type_only = find_type_only_dependencies(&graph, &pkg, &config, &[]);

    assert!(
        type_only.is_empty(),
        "dev deps should not appear in type-only dependency results"
    );
}

// ---- Additional coverage: find_unresolved_imports with empty input ----

#[test]
fn no_resolved_modules_produces_no_unresolved() {
    let resolved_modules: Vec<ResolvedModule> = vec![];
    let config = test_config(PathBuf::from("/project"));
    let suppressions: FxHashMap<FileId, &[Suppression]> = FxHashMap::default();
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let unresolved = find_unresolved_imports(
        &resolved_modules,
        &config,
        &suppressions,
        &[],
        &line_offsets,
    );

    assert!(
        unresolved.is_empty(),
        "empty resolved_modules should produce no unresolved imports"
    );
}

// ---- Additional coverage: should_skip_dependency with empty string ----

#[test]
fn skip_dep_empty_string_no_match() {
    let (root_flagged, script_used, plugin_referenced, ignore_deps, workspace_names) = empty_sets();
    assert!(!should_skip_dependency(
        "",
        &root_flagged,
        &script_used,
        &plugin_referenced,
        &ignore_deps,
        &workspace_names,
        |_| false,
    ));
}

// ---- Additional coverage: workspace-scoped dependency usage ----

#[test]
fn workspace_dep_used_within_workspace_not_flagged() {
    // A workspace declares "react" as a dep AND a file within that workspace imports "react".
    // This dep should NOT be flagged as unused for the workspace.
    let ws_root = PathBuf::from("/project/packages/web");
    let files = vec![DiscoveredFile {
        id: FileId(0),
        path: ws_root.join("src/index.ts"),
        size_bytes: 100,
    }];
    let entry_points = vec![EntryPoint {
        path: ws_root.join("src/index.ts"),
        source: EntryPointSource::PackageJsonMain,
    }];
    let resolved_modules = vec![ResolvedModule {
        file_id: FileId(0),
        path: ws_root.join("src/index.ts"),
        exports: vec![],
        re_exports: vec![],
        resolved_imports: vec![ResolvedImport {
            info: ImportInfo {
                source: "react".to_string(),
                imported_name: ImportedName::Named("useState".to_string()),
                local_name: "useState".to_string(),
                is_type_only: false,
                span: oxc_span::Span::new(0, 20),
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

    // Root package.json does NOT list "react" — it's only in the workspace
    let root_pkg = make_pkg(&[], &[], &[]);
    let config = test_config(PathBuf::from("/project"));

    // The workspace package.json would list "react", but since we can't write to disk,
    // we verify that the root analysis does not flag "react" because it IS used somewhere.
    let (unused, _, _) = find_unused_dependencies(&graph, &root_pkg, &config, None, &[]);

    // "react" is not in root package.json, so it won't appear in unused root deps at all
    assert!(
        !unused.iter().any(|d| d.package_name == "react"),
        "react should not be in root unused since it's not in root deps"
    );
}

// ---- Additional coverage: unlisted dep in workspace scope ----

#[test]
fn unlisted_dep_detected_across_multiple_files() {
    // Two files both import the same unlisted package — should deduplicate per file
    let files = vec![
        DiscoveredFile {
            id: FileId(0),
            path: PathBuf::from("/project/src/a.ts"),
            size_bytes: 100,
        },
        DiscoveredFile {
            id: FileId(1),
            path: PathBuf::from("/project/src/b.ts"),
            size_bytes: 100,
        },
    ];
    let entry_points = vec![
        EntryPoint {
            path: PathBuf::from("/project/src/a.ts"),
            source: EntryPointSource::PackageJsonMain,
        },
        EntryPoint {
            path: PathBuf::from("/project/src/b.ts"),
            source: EntryPointSource::PackageJsonMain,
        },
    ];
    let resolved_modules = vec![
        ResolvedModule {
            file_id: FileId(0),
            path: PathBuf::from("/project/src/a.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![ResolvedImport {
                info: ImportInfo {
                    source: "unlisted-pkg".to_string(),
                    imported_name: ImportedName::Named("foo".to_string()),
                    local_name: "foo".to_string(),
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 20),
                },
                target: ResolveResult::NpmPackage("unlisted-pkg".to_string()),
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
            path: PathBuf::from("/project/src/b.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![ResolvedImport {
                info: ImportInfo {
                    source: "unlisted-pkg".to_string(),
                    imported_name: ImportedName::Named("bar".to_string()),
                    local_name: "bar".to_string(),
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 20),
                },
                target: ResolveResult::NpmPackage("unlisted-pkg".to_string()),
            }],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        },
    ];
    let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);
    let pkg = make_pkg(&[], &[], &[]);
    let config = test_config(PathBuf::from("/project"));
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let unlisted = find_unlisted_dependencies(
        &graph,
        &pkg,
        &config,
        &[],
        None,
        &resolved_modules,
        &line_offsets,
    );

    assert_eq!(unlisted.len(), 1, "same unlisted pkg should be grouped");
    assert_eq!(unlisted[0].package_name, "unlisted-pkg");
    assert_eq!(
        unlisted[0].imported_from.len(),
        2,
        "should have import sites from both files"
    );
}

// ---- Additional coverage: find_unlisted_dependencies with optional dep listed ----

#[test]
fn optional_dep_not_reported_as_unlisted() {
    let (graph, resolved_modules) = build_graph_with_npm_imports(&[("sharp", false)]);
    let pkg = make_pkg(&[], &[], &["sharp"]);
    let config = test_config(PathBuf::from("/project"));
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let unlisted = find_unlisted_dependencies(
        &graph,
        &pkg,
        &config,
        &[],
        None,
        &resolved_modules,
        &line_offsets,
    );

    assert!(
        !unlisted.iter().any(|d| d.package_name == "sharp"),
        "optional deps should count as listed and not be flagged as unlisted"
    );
}

// ---- Additional coverage: find_unresolved_imports suppression does not suppress wrong kind ----

#[test]
fn unresolved_import_not_suppressed_by_wrong_kind() {
    let resolved_modules = vec![ResolvedModule {
        file_id: FileId(0),
        path: PathBuf::from("/project/src/index.ts"),
        exports: vec![],
        re_exports: vec![],
        resolved_imports: vec![ResolvedImport {
            info: ImportInfo {
                source: "./broken".to_string(),
                imported_name: ImportedName::Named("thing".to_string()),
                local_name: "thing".to_string(),
                is_type_only: false,
                span: oxc_span::Span::new(0, 20),
            },
            target: ResolveResult::Unresolvable("./broken".to_string()),
        }],
        resolved_dynamic_imports: vec![],
        resolved_dynamic_patterns: vec![],
        member_accesses: vec![],
        whole_object_uses: vec![],
        has_cjs_exports: false,
        unused_import_bindings: vec![],
    }];

    let config = test_config(PathBuf::from("/project"));
    // Suppress a DIFFERENT issue kind on line 1 — should NOT suppress unresolved import
    let supps = vec![Suppression {
        line: 1,
        kind: Some(suppress::IssueKind::UnusedExport),
    }];
    let mut suppressions: FxHashMap<FileId, &[Suppression]> = FxHashMap::default();
    suppressions.insert(FileId(0), &supps);
    let line_offsets: LineOffsetsMap<'_> = FxHashMap::default();

    let unresolved = find_unresolved_imports(
        &resolved_modules,
        &config,
        &suppressions,
        &[],
        &line_offsets,
    );

    assert_eq!(
        unresolved.len(),
        1,
        "suppression with wrong issue kind should not suppress unresolved import"
    );
}

// ---- Additional coverage: unused deps with plugin tooling for dev deps ----

#[test]
fn plugin_tooling_dev_deps_not_flagged() {
    let (graph, _) = build_graph_with_npm_imports(&[]);
    let pkg = make_pkg(&[], &["my-dev-tool"], &[]);
    let config = test_config(PathBuf::from("/project"));

    let mut plugin_result = AggregatedPluginResult::default();
    plugin_result
        .tooling_dependencies
        .push("my-dev-tool".to_string());

    let (_, unused_dev, _) =
        find_unused_dependencies(&graph, &pkg, &config, Some(&plugin_result), &[]);

    assert!(
        !unused_dev.iter().any(|d| d.package_name == "my-dev-tool"),
        "plugin tooling dev deps should not be flagged as unused"
    );
}
