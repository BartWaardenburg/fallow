use std::path::PathBuf;

use fallow_config::{FallowConfig, OutputFormat, RulesConfig};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn create_config(root: PathBuf) -> fallow_config::ResolvedConfig {
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
        rules: RulesConfig::default(),
        production: false,
        plugins: vec![],
        overrides: vec![],
    }
    .resolve(root, OutputFormat::Human, 4, true)
}

#[test]
fn basic_project_detects_unused_files() {
    let root = fixture_path("basic-project");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    // orphan.ts should be detected as unused
    let unused_file_names: Vec<String> = results
        .unused_files
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    assert!(
        unused_file_names.contains(&"orphan.ts".to_string()),
        "orphan.ts should be detected as unused file, found: {unused_file_names:?}"
    );
}

#[test]
fn basic_project_detects_unused_exports() {
    let root = fixture_path("basic-project");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    assert!(
        unused_export_names.contains(&"unusedFunction"),
        "unusedFunction should be detected as unused export, found: {unused_export_names:?}"
    );
    assert!(
        unused_export_names.contains(&"anotherUnused"),
        "anotherUnused should be detected as unused export, found: {unused_export_names:?}"
    );
    // usedFunction should NOT be in unused
    assert!(
        !unused_export_names.contains(&"usedFunction"),
        "usedFunction should NOT be detected as unused"
    );
}

#[test]
fn basic_project_detects_unused_types() {
    let root = fixture_path("basic-project");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_type_names: Vec<&str> = results
        .unused_types
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    assert!(
        unused_type_names.contains(&"UnusedType"),
        "UnusedType should be detected as unused type, found: {unused_type_names:?}"
    );
    assert!(
        unused_type_names.contains(&"UnusedInterface"),
        "UnusedInterface should be detected as unused type, found: {unused_type_names:?}"
    );
    // UsedType should NOT be in unused
    assert!(
        !unused_type_names.contains(&"UsedType"),
        "UsedType should NOT be detected as unused"
    );
}

#[test]
fn basic_project_detects_unused_dependencies() {
    let root = fixture_path("basic-project");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_dep_names: Vec<&str> = results
        .unused_dependencies
        .iter()
        .map(|d| d.package_name.as_str())
        .collect();

    assert!(
        unused_dep_names.contains(&"unused-dep"),
        "unused-dep should be detected as unused dependency, found: {unused_dep_names:?}"
    );
}

#[test]
fn barrel_exports_resolves_through_barrel() {
    let root = fixture_path("barrel-exports");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    // fooUnused should be detected as unused (it's not re-exported from barrel)
    assert!(
        unused_export_names.contains(&"fooUnused"),
        "fooUnused should be unused, found: {unused_export_names:?}"
    );
}

#[test]
fn analysis_returns_correct_total_count() {
    let root = fixture_path("basic-project");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    assert!(results.has_issues(), "basic-project should have issues");
    assert!(results.total_issues() > 0, "total_issues should be > 0");
}

#[test]
fn dynamic_import_is_parsed() {
    use fallow_core::discover::FileId;
    use fallow_core::extract::parse_from_content;
    use std::path::Path;

    let content = r#"const mod = import('./lazy-module');"#;
    let info = parse_from_content(FileId(0), Path::new("test.ts"), content);

    assert_eq!(info.dynamic_imports.len(), 1);
    assert_eq!(info.dynamic_imports[0].source, "./lazy-module");
}

#[test]
fn cjs_interop_detects_require() {
    use fallow_core::discover::FileId;
    use fallow_core::extract::parse_from_content;
    use std::path::Path;

    let content = r#"const fs = require('fs'); const path = require('path');"#;
    let info = parse_from_content(FileId(0), Path::new("test.js"), content);

    assert_eq!(info.require_calls.len(), 2);
    assert_eq!(info.require_calls[0].source, "fs");
    assert_eq!(info.require_calls[1].source, "path");
}

#[test]
fn type_only_imports_are_marked() {
    use fallow_core::discover::FileId;
    use fallow_core::extract::parse_from_content;
    use std::path::Path;

    let content = r#"import type { Foo } from './types'; import { Bar } from './utils';"#;
    let info = parse_from_content(FileId(0), Path::new("test.ts"), content);

    assert_eq!(info.imports.len(), 2);
    assert!(info.imports[0].is_type_only);
    assert!(!info.imports[1].is_type_only);
}

#[test]
fn enum_members_are_extracted() {
    use fallow_core::discover::FileId;
    use fallow_core::extract::parse_from_content;
    use std::path::Path;

    let content = r#"export enum Color { Red = 'red', Green = 'green', Blue = 'blue' }"#;
    let info = parse_from_content(FileId(0), Path::new("test.ts"), content);

    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].members.len(), 3);
    assert_eq!(info.exports[0].members[0].name, "Red");
    assert_eq!(info.exports[0].members[1].name, "Green");
    assert_eq!(info.exports[0].members[2].name, "Blue");
}

#[test]
fn class_members_are_extracted() {
    use fallow_core::discover::FileId;
    use fallow_core::extract::parse_from_content;
    use std::path::Path;

    let content = r#"
export class MyService {
    name: string = '';
    async getUser(id: number) { return id; }
    static create() { return new MyService(); }
}
"#;
    let info = parse_from_content(FileId(0), Path::new("test.ts"), content);

    assert_eq!(info.exports.len(), 1);
    assert!(
        info.exports[0].members.len() >= 3,
        "Should have at least 3 members"
    );
}

#[test]
fn star_re_export_is_parsed() {
    use fallow_core::discover::FileId;
    use fallow_core::extract::parse_from_content;
    use std::path::Path;

    let content = r#"export * from './module';"#;
    let info = parse_from_content(FileId(0), Path::new("test.ts"), content);

    assert_eq!(info.re_exports.len(), 1);
    assert_eq!(info.re_exports[0].imported_name, "*");
    assert_eq!(info.re_exports[0].exported_name, "*");
    assert_eq!(info.re_exports[0].source, "./module");
}

#[test]
fn named_re_export_is_parsed() {
    use fallow_core::discover::FileId;
    use fallow_core::extract::parse_from_content;
    use std::path::Path;

    let content = r#"export { foo, bar as baz } from './module';"#;
    let info = parse_from_content(FileId(0), Path::new("test.ts"), content);

    assert_eq!(info.re_exports.len(), 2);
    assert_eq!(info.re_exports[0].imported_name, "foo");
    assert_eq!(info.re_exports[0].exported_name, "foo");
    assert_eq!(info.re_exports[1].imported_name, "bar");
    assert_eq!(info.re_exports[1].exported_name, "baz");
}

#[test]
fn circular_import_does_not_crash() {
    // Create temporary fixture with circular imports
    use std::time::{SystemTime, UNIX_EPOCH};
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("fallow-test-circular-{unique}"));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("src")).unwrap();

    std::fs::write(
        temp_dir.join("package.json"),
        r#"{"name": "circular", "main": "src/a.ts"}"#,
    )
    .unwrap();

    std::fs::write(
        temp_dir.join("src/a.ts"),
        "import { b } from './b';\nexport const a = b + 1;\n",
    )
    .unwrap();

    std::fs::write(
        temp_dir.join("src/b.ts"),
        "import { a } from './a';\nexport const b = a + 1;\n",
    )
    .unwrap();

    let config = create_config(temp_dir.clone());
    // This should not crash or infinite loop
    let results = fallow_core::analyze(&config).expect("analysis should succeed");
    let _ = &results; // ensure analysis completed without panic

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn namespace_import_marks_all_exports_used() {
    use fallow_core::discover::FileId;
    use fallow_core::extract::parse_from_content;
    use std::path::Path;

    let content = r#"import * as utils from './utils';"#;
    let info = parse_from_content(FileId(0), Path::new("test.ts"), content);

    assert_eq!(info.imports.len(), 1);
    assert_eq!(
        info.imports[0].imported_name,
        fallow_core::extract::ImportedName::Namespace
    );
}

#[test]
fn default_export_is_parsed() {
    use fallow_core::discover::FileId;
    use fallow_core::extract::parse_from_content;
    use std::path::Path;

    let content = r#"export default class MyComponent {}"#;
    let info = parse_from_content(FileId(0), Path::new("test.tsx"), content);

    assert_eq!(info.exports.len(), 1);
    assert_eq!(
        info.exports[0].name,
        fallow_core::extract::ExportName::Default
    );
}

#[test]
fn destructured_exports_are_parsed() {
    use fallow_core::discover::FileId;
    use fallow_core::extract::parse_from_content;
    use std::path::Path;

    let content = r#"export const { a, b } = { a: 1, b: 2 };"#;
    let info = parse_from_content(FileId(0), Path::new("test.ts"), content);

    assert_eq!(info.exports.len(), 2);
    assert_eq!(
        info.exports[0].name,
        fallow_core::extract::ExportName::Named("a".to_string())
    );
    assert_eq!(
        info.exports[1].name,
        fallow_core::extract::ExportName::Named("b".to_string())
    );
}

#[test]
fn side_effect_import_is_parsed() {
    use fallow_core::discover::FileId;
    use fallow_core::extract::parse_from_content;
    use std::path::Path;

    let content = r#"import './polyfills';"#;
    let info = parse_from_content(FileId(0), Path::new("test.ts"), content);

    assert_eq!(info.imports.len(), 1);
    assert_eq!(
        info.imports[0].imported_name,
        fallow_core::extract::ImportedName::SideEffect
    );
    assert_eq!(info.imports[0].source, "./polyfills");
}

#[test]
fn named_re_export_with_alias() {
    use fallow_core::discover::FileId;
    use fallow_core::extract::parse_from_content;
    use std::path::Path;

    let content = r#"export { default as MyComponent } from './Component';"#;
    let info = parse_from_content(FileId(0), Path::new("test.ts"), content);

    assert_eq!(info.re_exports.len(), 1);
    assert_eq!(info.re_exports[0].imported_name, "default");
    assert_eq!(info.re_exports[0].exported_name, "MyComponent");
}

#[test]
fn cjs_module_exports_assignment() {
    use fallow_core::discover::FileId;
    use fallow_core::extract::parse_from_content;
    use std::path::Path;

    let content = r#"module.exports = { foo: 1, bar: 2 };"#;
    let info = parse_from_content(FileId(0), Path::new("test.js"), content);

    assert!(info.has_cjs_exports);
}

#[test]
fn cjs_exports_dot_assignment() {
    use fallow_core::discover::FileId;
    use fallow_core::extract::parse_from_content;
    use std::path::Path;

    let content = r#"exports.foo = 42; exports.bar = 'hello';"#;
    let info = parse_from_content(FileId(0), Path::new("test.js"), content);

    assert!(info.has_cjs_exports);
    assert_eq!(info.exports.len(), 2);
}

#[test]
fn multiple_export_types_in_one_file() {
    use fallow_core::discover::FileId;
    use fallow_core::extract::parse_from_content;
    use std::path::Path;

    let content = r#"
export const VALUE = 42;
export function helper() {}
export type Config = { key: string };
export interface Logger { log(msg: string): void }
export enum Level { Debug, Info, Warn, Error }
export default class App {}
"#;
    let info = parse_from_content(FileId(0), Path::new("test.ts"), content);

    // VALUE, helper, Config, Logger, Level, default = 6 exports
    assert_eq!(
        info.exports.len(),
        6,
        "Expected 6 exports, got: {:?}",
        info.exports
            .iter()
            .map(|e| e.name.to_string())
            .collect::<Vec<_>>()
    );

    // Level enum should have 4 members
    let level_export = info
        .exports
        .iter()
        .find(|e| e.name.to_string() == "Level")
        .unwrap();
    assert_eq!(level_export.members.len(), 4);
}

#[test]
fn extract_package_name_scoped() {
    use fallow_core::resolve::extract_package_name;

    assert_eq!(extract_package_name("react"), "react");
    assert_eq!(extract_package_name("react/jsx-runtime"), "react");
    assert_eq!(extract_package_name("@scope/pkg"), "@scope/pkg");
    assert_eq!(extract_package_name("@scope/pkg/utils"), "@scope/pkg");
    assert_eq!(extract_package_name("@types/node"), "@types/node");
}

#[test]
fn cache_roundtrip() {
    use fallow_core::cache::CacheStore;
    use std::time::{SystemTime, UNIX_EPOCH};

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("fallow-test-cache-{unique}"));
    let _ = std::fs::remove_dir_all(&temp_dir);

    let mut store = CacheStore::new();
    assert!(store.is_empty());

    let cached = fallow_core::cache::CachedModule {
        content_hash: 12345,
        exports: vec![],
        imports: vec![],
        re_exports: vec![],
        dynamic_imports: vec![],
        require_calls: vec![],
        member_accesses: vec![],
        whole_object_uses: vec![],
        dynamic_import_patterns: vec![],
        has_cjs_exports: false,
        suppressions: vec![],
    };

    store.insert(std::path::Path::new("test.ts"), cached);
    assert_eq!(store.len(), 1);

    // Save and reload
    store.save(&temp_dir).unwrap();
    let loaded = CacheStore::load(&temp_dir).unwrap();
    assert_eq!(loaded.len(), 1);

    // Correct hash -> hit
    assert!(loaded.get(std::path::Path::new("test.ts"), 12345).is_some());
    // Wrong hash -> miss
    assert!(loaded.get(std::path::Path::new("test.ts"), 99999).is_none());
    // Unknown file -> miss
    assert!(
        loaded
            .get(std::path::Path::new("other.ts"), 12345)
            .is_none()
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn workspace_patterns_from_package_json() {
    let pkg: fallow_config::PackageJson =
        serde_json::from_str(r#"{"workspaces": ["packages/*", "apps/*"]}"#).unwrap();

    let patterns = pkg.workspace_patterns();
    assert_eq!(patterns, vec!["packages/*", "apps/*"]);
}

#[test]
fn workspace_patterns_yarn_format() {
    let pkg: fallow_config::PackageJson =
        serde_json::from_str(r#"{"workspaces": {"packages": ["packages/*"]}}"#).unwrap();

    let patterns = pkg.workspace_patterns();
    assert_eq!(patterns, vec!["packages/*"]);
}

// ── Namespace imports ─────────────────────────────────────────

#[test]
fn namespace_import_makes_all_exports_used() {
    let root = fixture_path("namespace-imports");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    // With import * as utils, only members accessed via utils.member are used.
    // In the fixture, only utils.foo is accessed; bar and baz are unused.
    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    assert!(
        !unused_export_names.contains(&"foo"),
        "foo should be used via utils.foo member access"
    );
    assert!(
        unused_export_names.contains(&"bar"),
        "bar should be unused (not accessed via utils.bar)"
    );
    assert!(
        unused_export_names.contains(&"baz"),
        "baz should be unused (not accessed via utils.baz)"
    );
}

// ── Duplicate exports ─────────────────────────────────────────

#[test]
fn duplicate_exports_detected() {
    let root = fixture_path("duplicate-exports");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let dup_names: Vec<&str> = results
        .duplicate_exports
        .iter()
        .map(|d| d.export_name.as_str())
        .collect();

    assert!(
        dup_names.contains(&"shared"),
        "shared should be detected as duplicate export, found: {dup_names:?}"
    );
}

// ── Rules "off" disables detection ─────────────────────────────

#[test]
fn rules_off_disables_unused_files() {
    let root = fixture_path("detect-config");
    let mut config = create_config(root.clone());
    config.rules.unused_files = fallow_config::Severity::Off;
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    assert!(
        results.unused_files.is_empty(),
        "unused files should be empty when rule is off"
    );
}

#[test]
fn rules_off_disables_unused_exports() {
    let root = fixture_path("detect-config");
    let mut config = create_config(root.clone());
    config.rules.unused_exports = fallow_config::Severity::Off;
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    assert!(
        results.unused_exports.is_empty(),
        "unused exports should be empty when rule is off"
    );
}

#[test]
fn rules_off_disables_unused_types() {
    let root = fixture_path("detect-config");
    let mut config = create_config(root.clone());
    config.rules.unused_types = fallow_config::Severity::Off;
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    assert!(
        results.unused_types.is_empty(),
        "unused types should be empty when rule is off"
    );
}

#[test]
fn rules_off_disables_unused_dependencies() {
    let root = fixture_path("detect-config");
    let mut config = create_config(root.clone());
    config.rules.unused_dependencies = fallow_config::Severity::Off;
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    assert!(
        results.unused_dependencies.is_empty(),
        "unused dependencies should be empty when rule is off"
    );
}

#[test]
fn rules_off_disables_duplicate_exports() {
    let root = fixture_path("duplicate-exports");
    let mut config = create_config(root.clone());
    config.rules.duplicate_exports = fallow_config::Severity::Off;
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    assert!(
        results.duplicate_exports.is_empty(),
        "duplicate exports should be empty when rule is off"
    );
}

// ── Ignore exports ─────────────────────────────────────────────

#[test]
fn ignore_exports_wildcard() {
    let root = fixture_path("ignore-exports");
    let config = FallowConfig {
        schema: None,
        extends: vec![],
        entry: vec![],
        ignore_patterns: vec![],
        framework: vec![],
        workspaces: None,
        ignore_dependencies: vec![],
        ignore_exports: vec![fallow_config::IgnoreExportRule {
            file: "src/utils.ts".to_string(),
            exports: vec!["*".to_string()],
        }],
        duplicates: fallow_config::DuplicatesConfig::default(),
        rules: RulesConfig::default(),
        production: false,
        plugins: vec![],
        overrides: vec![],
    }
    .resolve(root.clone(), OutputFormat::Human, 4, true);

    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    assert!(
        !unused_export_names.contains(&"ignored"),
        "ignored should not appear when wildcard ignore is set"
    );
    assert!(
        !unused_export_names.contains(&"notIgnored"),
        "notIgnored should also be ignored by wildcard"
    );
}

#[test]
fn ignore_exports_specific() {
    let root = fixture_path("ignore-exports");
    let config = FallowConfig {
        schema: None,
        extends: vec![],
        entry: vec![],
        ignore_patterns: vec![],
        framework: vec![],
        workspaces: None,
        ignore_dependencies: vec![],
        ignore_exports: vec![fallow_config::IgnoreExportRule {
            file: "src/utils.ts".to_string(),
            exports: vec!["ignored".to_string()],
        }],
        duplicates: fallow_config::DuplicatesConfig::default(),
        rules: RulesConfig::default(),
        production: false,
        plugins: vec![],
        overrides: vec![],
    }
    .resolve(root.clone(), OutputFormat::Human, 4, true);

    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    assert!(
        !unused_export_names.contains(&"ignored"),
        "ignored should not appear when specifically ignored"
    );
    assert!(
        unused_export_names.contains(&"notIgnored"),
        "notIgnored should still be reported, found: {unused_export_names:?}"
    );
}

// ── CJS project ────────────────────────────────────────────────

#[test]
fn cjs_project_detects_orphan() {
    let root = fixture_path("cjs-project");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_file_names: Vec<String> = results
        .unused_files
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    assert!(
        unused_file_names.contains(&"orphan.js".to_string()),
        "orphan.js should be detected as unused, found: {unused_file_names:?}"
    );
}

// ── Dynamic imports ────────────────────────────────────────────

#[test]
fn dynamic_import_makes_module_reachable() {
    let root = fixture_path("dynamic-imports");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_file_names: Vec<String> = results
        .unused_files
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    // lazy.ts is dynamically imported, so it should be reachable
    assert!(
        !unused_file_names.contains(&"lazy.ts".to_string()),
        "lazy.ts should be reachable via dynamic import, unused files: {unused_file_names:?}"
    );

    // orphan.ts should still be unused
    assert!(
        unused_file_names.contains(&"orphan.ts".to_string()),
        "orphan.ts should be unused, found: {unused_file_names:?}"
    );
}

// ── Ignore dependencies ────────────────────────────────────────

#[test]
fn ignore_dependencies_config() {
    let root = fixture_path("basic-project");
    let config = FallowConfig {
        schema: None,
        extends: vec![],
        entry: vec![],
        ignore_patterns: vec![],
        framework: vec![],
        workspaces: None,
        ignore_dependencies: vec!["unused-dep".to_string()],
        ignore_exports: vec![],
        duplicates: fallow_config::DuplicatesConfig::default(),
        rules: RulesConfig::default(),
        production: false,
        plugins: vec![],
        overrides: vec![],
    }
    .resolve(root.clone(), OutputFormat::Human, 4, true);

    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_dep_names: Vec<&str> = results
        .unused_dependencies
        .iter()
        .map(|d| d.package_name.as_str())
        .collect();

    assert!(
        !unused_dep_names.contains(&"unused-dep"),
        "unused-dep should be ignored"
    );
}

// ── Full pipeline sanity checks ────────────────────────────────

#[test]
fn analyze_project_convenience_function() {
    let root = fixture_path("basic-project");
    let results = fallow_core::analyze_project(&root).expect("analysis should succeed");
    assert!(results.has_issues());
}

#[test]
fn results_serializable_to_json() {
    let root = fixture_path("basic-project");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");
    let json = serde_json::to_string(&results).unwrap();
    assert!(!json.is_empty());
    // Verify it round-trips
    let _: serde_json::Value = serde_json::from_str(&json).unwrap();
}

// ── Workspace integration ──────────────────────────────────────

#[test]
fn workspace_project_discovers_workspace_packages() {
    let root = fixture_path("workspace-project");

    // Set up node_modules symlinks for cross-workspace resolution (like npm/pnpm install would)
    let nm = root.join("node_modules");
    let _ = std::fs::create_dir_all(nm.join("@workspace"));
    #[cfg(unix)]
    {
        let _ = std::os::unix::fs::symlink(root.join("packages/shared"), nm.join("shared"));
        let _ =
            std::os::unix::fs::symlink(root.join("packages/utils"), nm.join("@workspace/utils"));
    }

    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    // Workspace discovery should find files across workspace packages
    // orphan.ts should always be detected as unused since nothing imports it
    let unused_file_names: Vec<String> = results
        .unused_files
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    assert!(
        unused_file_names.contains(&"orphan.ts".to_string()),
        "orphan.ts should be detected as unused file, found: {unused_file_names:?}"
    );

    // Cross-workspace resolution via node_modules symlinks:
    // app imports `@workspace/utils/src/deep` which resolves through the symlink,
    // making deep.ts reachable. If symlinks are broken, deep.ts would be unreachable.
    assert!(
        !unused_file_names.contains(&"deep.ts".to_string()),
        "deep.ts should NOT be unused (reachable via cross-workspace import through symlink), \
         but found in unused files: {unused_file_names:?}"
    );

    // `unusedDeep` should be detected as unused export (deep.ts is reachable but
    // only `deepHelper` is imported, not `unusedDeep`)
    let unused_export_names: Vec<String> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.clone())
        .collect();
    assert!(
        unused_export_names.contains(&"unusedDeep".to_string()),
        "unusedDeep should be detected as unused export, found: {unused_export_names:?}"
    );

    // No unresolved imports — all cross-workspace imports should resolve
    assert!(
        results.unresolved_imports.is_empty(),
        "should have no unresolved imports, found: {:?}",
        results
            .unresolved_imports
            .iter()
            .map(|i| &i.specifier)
            .collect::<Vec<_>>()
    );

    // The analysis should have found issues across all workspace packages
    assert!(
        results.has_issues(),
        "workspace project should have issues detected"
    );
}

#[test]
fn project_state_stable_file_ids_by_path() {
    // FileIds should be deterministic: sorted by path, not size.
    // Running discovery twice on the same project must produce identical IDs.
    let root = fixture_path("workspace-project");
    let config = create_config(root);

    let files_a = fallow_core::discover::discover_files(&config);
    let files_b = fallow_core::discover::discover_files(&config);

    assert_eq!(files_a.len(), files_b.len());
    for (a, b) in files_a.iter().zip(files_b.iter()) {
        assert_eq!(a.id, b.id, "FileId mismatch for {:?}", a.path);
        assert_eq!(a.path, b.path);
    }

    // Files should be sorted by path (not by size)
    for window in files_a.windows(2) {
        assert!(
            window[0].path <= window[1].path,
            "Files not sorted by path: {:?} > {:?}",
            window[0].path,
            window[1].path
        );
    }
}

#[test]
fn project_state_workspace_queries() {
    use fallow_config::discover_workspaces;

    let root = fixture_path("workspace-project");
    let config = create_config(root.clone());
    let files = fallow_core::discover::discover_files(&config);
    let workspaces = discover_workspaces(&root);
    let project = fallow_core::project::ProjectState::new(files, workspaces);

    // Should find all three workspace packages
    assert!(project.workspace_by_name("app").is_some());
    assert!(project.workspace_by_name("shared").is_some());
    assert!(project.workspace_by_name("@workspace/utils").is_some());
    assert!(project.workspace_by_name("nonexistent").is_none());

    // Files should be assignable to workspaces
    let app_ws = project.workspace_by_name("app").unwrap();
    let app_files = project.files_in_workspace(app_ws);
    assert!(
        !app_files.is_empty(),
        "app workspace should have at least one file"
    );

    // All app files should be under the app workspace root
    for fid in &app_files {
        if let Some(file) = project.file_by_id(*fid) {
            assert!(
                file.path.starts_with(&app_ws.root),
                "File {:?} should be under app workspace root {:?}",
                file.path,
                app_ws.root
            );
        }
    }
}

// ── Enum/class members integration ─────────────────────────────

#[test]
fn enum_class_members_detects_unused_members() {
    let root = fixture_path("enum-class-members");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_enum_member_names: Vec<&str> = results
        .unused_enum_members
        .iter()
        .map(|m| m.member_name.as_str())
        .collect();

    // Only Status.Active is used; Inactive and Pending should be unused
    assert!(
        unused_enum_member_names.contains(&"Inactive"),
        "Inactive should be detected as unused enum member, found: {unused_enum_member_names:?}"
    );
    assert!(
        unused_enum_member_names.contains(&"Pending"),
        "Pending should be detected as unused enum member, found: {unused_enum_member_names:?}"
    );

    let unused_class_member_names: Vec<&str> = results
        .unused_class_members
        .iter()
        .map(|m| m.member_name.as_str())
        .collect();

    // unusedMethod is never called
    assert!(
        unused_class_member_names.contains(&"unusedMethod"),
        "unusedMethod should be detected as unused class member, found: {unused_class_member_names:?}"
    );
}

// ── Unlisted dependencies integration ──────────────────────────

#[test]
fn unlisted_dependencies_detected() {
    let root = fixture_path("unlisted-deps");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unlisted_names: Vec<&str> = results
        .unlisted_dependencies
        .iter()
        .map(|d| d.package_name.as_str())
        .collect();

    assert!(
        unlisted_names.contains(&"some-pkg"),
        "some-pkg should be detected as unlisted dependency, found: {unlisted_names:?}"
    );
}

// ── Unresolved imports integration ─────────────────────────────

#[test]
fn unresolved_imports_detected() {
    let root = fixture_path("unresolved-imports");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unresolved_specifiers: Vec<&str> = results
        .unresolved_imports
        .iter()
        .map(|u| u.specifier.as_str())
        .collect();

    assert!(
        unresolved_specifiers.contains(&"./nonexistent"),
        "\"./nonexistent\" should be detected as unresolved import, found: {unresolved_specifiers:?}"
    );
}

// ── Barrel re-export unused detection ──────────────────────────

#[test]
fn barrel_unused_re_exports_detected() {
    let root = fixture_path("barrel-unused-reexports");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    // UnusedComponent is re-exported from barrel but never imported by anyone
    assert!(
        unused_export_names.contains(&"UnusedComponent"),
        "UnusedComponent should be detected as unused re-export on barrel, found: {unused_export_names:?}"
    );

    // UsedComponent IS imported via barrel, so it should NOT be unused
    assert!(
        !unused_export_names.contains(&"UsedComponent"),
        "UsedComponent should NOT be detected as unused"
    );
}

#[test]
fn barrel_unused_type_re_exports_detected() {
    let root = fixture_path("barrel-unused-reexports");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_type_names: Vec<&str> = results
        .unused_types
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    // UnusedType is re-exported as type from barrel but never imported
    assert!(
        unused_type_names.contains(&"UnusedType"),
        "UnusedType should be detected as unused type re-export on barrel, found: {unused_type_names:?}"
    );

    // UsedType IS imported via barrel, so it should NOT be unused
    assert!(
        !unused_type_names.contains(&"UsedType"),
        "UsedType should NOT be detected as unused type"
    );
}

#[test]
fn barrel_re_export_propagates_to_source_module() {
    // When a re-export on a barrel is unused, the source module's export
    // should also be flagged if only consumed through the (unused) barrel re-export.
    // Conversely, if the barrel re-export IS used, the source should NOT be flagged.
    let root = fixture_path("barrel-unused-reexports");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    // UsedComponent on the source module should NOT be flagged
    // (it's referenced through the barrel which is consumed)
    assert!(
        !unused_export_names.contains(&"UsedComponent"),
        "source UsedComponent should not be unused since barrel re-export is consumed"
    );
}

#[test]
fn barrel_exports_detects_unused_re_export_bar() {
    // In the existing barrel-exports fixture, `bar` is re-exported from barrel
    // but nobody imports `bar` from the barrel.
    let root = fixture_path("barrel-exports");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    assert!(
        unused_export_names.contains(&"bar"),
        "bar should be detected as unused re-export on barrel (nobody imports it), found: {unused_export_names:?}"
    );

    // foo should not be flagged (it IS imported from barrel by index.ts)
    assert!(
        !unused_export_names.contains(&"foo"),
        "foo should NOT be unused since index.ts imports it from barrel"
    );
}

// ── Framework entry points (Next.js) ───────────────────────────

#[test]
fn nextjs_page_default_export_not_flagged() {
    let root = fixture_path("nextjs-project");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_file_names: Vec<String> = results
        .unused_files
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    // page.tsx is a Next.js App Router entry point, so it should NOT be unused
    assert!(
        !unused_file_names.contains(&"page.tsx".to_string()),
        "page.tsx should be treated as framework entry point, unused files: {unused_file_names:?}"
    );

    // utils.ts is not imported by anything, so it should be unused
    assert!(
        unused_file_names.contains(&"utils.ts".to_string()),
        "utils.ts should be detected as unused file, found: {unused_file_names:?}"
    );
}

#[test]
fn nextjs_unused_util_export_flagged() {
    let root = fixture_path("nextjs-project");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    // unusedUtil is exported but never imported — however, since utils.ts is an
    // unreachable file, it may be reported as unused file instead of unused export.
    // The key point is that it IS flagged as a problem in some way.
    let unused_file_names: Vec<String> = results
        .unused_files
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    assert!(
        unused_export_names.contains(&"unusedUtil")
            || unused_file_names.contains(&"utils.ts".to_string()),
        "unusedUtil should be flagged as unused export or utils.ts as unused file"
    );
}

// ── Unused devDependencies ─────────────────────────────────────

#[test]
fn unused_dev_dependency_detected() {
    let root = fixture_path("unused-dev-deps");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_dev_dep_names: Vec<&str> = results
        .unused_dev_dependencies
        .iter()
        .map(|d| d.package_name.as_str())
        .collect();

    assert!(
        unused_dev_dep_names.contains(&"my-custom-dev-tool"),
        "my-custom-dev-tool should be detected as unused dev dependency, found: {unused_dev_dep_names:?}"
    );
}

// ── Default export detection ───────────────────────────────────

#[test]
fn default_export_flagged_when_not_imported() {
    let root = fixture_path("default-export");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    // unused-default.ts is never imported, so it should be an unused file
    let unused_file_names: Vec<String> = results
        .unused_files
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    assert!(
        unused_file_names.contains(&"unused-default.ts".to_string()),
        "unused-default.ts should be detected as unused file, found: {unused_file_names:?}"
    );
}

#[test]
fn default_export_flagged_when_only_named_imported() {
    let root = fixture_path("default-export");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    // component.ts is imported for { usedNamed } only, so its default export
    // should be flagged as unused
    let unused_export_entries: Vec<(&str, String)> = results
        .unused_exports
        .iter()
        .map(|e| {
            (
                e.export_name.as_str(),
                e.path.file_name().unwrap().to_string_lossy().to_string(),
            )
        })
        .collect();

    assert!(
        unused_export_entries
            .iter()
            .any(|(name, file)| *name == "default" && file == "component.ts"),
        "default export on component.ts should be flagged as unused, found: {unused_export_entries:?}"
    );

    // usedNamed should NOT be flagged
    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    assert!(
        !unused_export_names.contains(&"usedNamed"),
        "usedNamed should NOT be detected as unused"
    );
}

// ── Side-effect imports ────────────────────────────────────────

#[test]
fn side_effect_import_makes_file_reachable() {
    let root = fixture_path("side-effect-imports");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_file_names: Vec<String> = results
        .unused_files
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    // setup.ts is imported via side-effect import, so it should be reachable
    assert!(
        !unused_file_names.contains(&"setup.ts".to_string()),
        "setup.ts should be reachable via side-effect import, unused files: {unused_file_names:?}"
    );

    // orphan.ts is never imported, so it should be unused
    assert!(
        unused_file_names.contains(&"orphan.ts".to_string()),
        "orphan.ts should be detected as unused file, found: {unused_file_names:?}"
    );
}

// ── Multi-hop barrel chains ────────────────────────────────────

#[test]
fn multi_hop_barrel_used_propagates() {
    let root = fixture_path("multi-hop-barrel");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    // `used` is imported through barrel1 -> barrel2 -> source, so it should NOT be flagged
    assert!(
        !unused_export_names.contains(&"used"),
        "used should propagate through barrel chain and NOT be flagged"
    );
}

#[test]
fn multi_hop_barrel_unused_detected() {
    let root = fixture_path("multi-hop-barrel");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    // unused2 is only exported from source.ts and re-exported from barrel2
    // but NOT re-exported from barrel1, so it should be flagged
    assert!(
        unused_export_names.contains(&"unused2"),
        "unused2 should be detected as unused export, found: {unused_export_names:?}"
    );
}

// ── Star re-export chains ──────────────────────────────────────

#[test]
fn star_re_export_chain_used_propagates() {
    let root = fixture_path("star-re-export-chain");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    // `used` is imported through barrel1 (export *) -> barrel2 (export *) -> source
    assert!(
        !unused_export_names.contains(&"used"),
        "used should propagate through star re-export chain and NOT be flagged, found: {unused_export_names:?}"
    );
}

#[test]
fn star_re_export_chain_unused_detected() {
    let root = fixture_path("star-re-export-chain");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    // `unused` is exported from source.ts but never imported
    assert!(
        unused_export_names.contains(&"unused"),
        "unused should be detected as unused export, found: {unused_export_names:?}"
    );
}

// ── Path aliases ───────────────────────────────────────────────

#[test]
fn path_alias_not_flagged_as_unlisted() {
    let root = fixture_path("path-aliases");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unlisted_names: Vec<&str> = results
        .unlisted_dependencies
        .iter()
        .map(|d| d.package_name.as_str())
        .collect();

    // @/utils is a path alias, not an npm package, so it should NOT be flagged
    assert!(
        !unlisted_names.contains(&"@/utils"),
        "@/utils should not be flagged as unlisted dependency, found: {unlisted_names:?}"
    );
}

// ── Duplication detection integration tests ─────────────────────────────

#[test]
fn duplicate_code_detects_exact_clones() {
    let root = fixture_path("duplicate-code");
    let config = create_config(root.clone());
    let files = fallow_core::discover::discover_files(&config);

    let dupes_config = fallow_core::duplicates::DuplicatesConfig {
        min_tokens: 20,
        min_lines: 3,
        ..fallow_core::duplicates::DuplicatesConfig::default()
    };

    let report = fallow_core::duplicates::find_duplicates(&root, &files, &dupes_config);

    assert!(
        !report.clone_groups.is_empty(),
        "Should detect clones in duplicate-code fixture"
    );
    assert!(
        report.stats.files_with_clones >= 2,
        "At least 2 files should have clones"
    );
    assert!(
        report.stats.duplication_percentage > 0.0,
        "Duplication percentage should be > 0"
    );
}

#[test]
fn duplicate_code_semantic_mode_detects_type2_clones() {
    let root = fixture_path("duplicate-code");
    let config = create_config(root.clone());
    let files = fallow_core::discover::discover_files(&config);

    let dupes_config = fallow_core::duplicates::DuplicatesConfig {
        min_tokens: 20,
        min_lines: 3,
        mode: fallow_core::duplicates::DetectionMode::Semantic,
        ..fallow_core::duplicates::DuplicatesConfig::default()
    };

    let report = fallow_core::duplicates::find_duplicates(&root, &files, &dupes_config);

    // In semantic mode, copy2.ts (renamed variables) should also match
    let files_with_clones: std::collections::HashSet<_> = report
        .clone_groups
        .iter()
        .flat_map(|g| g.instances.iter())
        .map(|inst| inst.file.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    assert!(
        files_with_clones.contains("copy2.ts"),
        "Semantic mode should detect copy2.ts with renamed variables, files found: {files_with_clones:?}"
    );
}

#[test]
fn duplicate_code_unique_file_has_no_clones() {
    let root = fixture_path("duplicate-code");
    let config = create_config(root.clone());
    let files = fallow_core::discover::discover_files(&config);

    let dupes_config = fallow_core::duplicates::DuplicatesConfig {
        min_tokens: 20,
        min_lines: 3,
        ..fallow_core::duplicates::DuplicatesConfig::default()
    };

    let report = fallow_core::duplicates::find_duplicates(&root, &files, &dupes_config);

    // unique.ts should not appear in any clone group (its code is distinct)
    let all_clone_files: Vec<String> = report
        .clone_groups
        .iter()
        .flat_map(|g| g.instances.iter())
        .map(|inst| inst.file.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    assert!(
        !all_clone_files.contains(&"unique.ts".to_string()),
        "unique.ts should not appear in any clone group, found in: {all_clone_files:?}"
    );
}

#[test]
fn duplicate_code_json_output_serializable() {
    let root = fixture_path("duplicate-code");
    let config = create_config(root.clone());
    let files = fallow_core::discover::discover_files(&config);

    let dupes_config = fallow_core::duplicates::DuplicatesConfig {
        min_tokens: 20,
        min_lines: 3,
        ..fallow_core::duplicates::DuplicatesConfig::default()
    };

    let report = fallow_core::duplicates::find_duplicates(&root, &files, &dupes_config);

    // Should be serializable to JSON
    let json = serde_json::to_string_pretty(&report).expect("report should serialize to JSON");
    let reparsed: serde_json::Value = serde_json::from_str(&json).expect("JSON should be valid");
    assert!(reparsed["clone_groups"].is_array());
    assert!(reparsed["stats"]["total_files"].is_number());
}

#[test]
fn duplicate_code_skip_local_filters_same_directory() {
    let root = fixture_path("duplicate-code");
    let config = create_config(root.clone());
    let files = fallow_core::discover::discover_files(&config);

    let dupes_config = fallow_core::duplicates::DuplicatesConfig {
        min_tokens: 20,
        min_lines: 3,
        skip_local: true,
        ..fallow_core::duplicates::DuplicatesConfig::default()
    };

    let report = fallow_core::duplicates::find_duplicates(&root, &files, &dupes_config);

    // All fixture files are in the same directory (src/), so skip_local should filter them all
    assert!(
        report.clone_groups.is_empty(),
        "skip_local should filter same-directory clones"
    );
}

#[test]
fn duplicate_code_min_tokens_threshold_filters() {
    let root = fixture_path("duplicate-code");
    let config = create_config(root.clone());
    let files = fallow_core::discover::discover_files(&config);

    // Use very high min_tokens — should find no clones
    let dupes_config = fallow_core::duplicates::DuplicatesConfig {
        min_tokens: 10000,
        min_lines: 1,
        ..fallow_core::duplicates::DuplicatesConfig::default()
    };

    let report = fallow_core::duplicates::find_duplicates(&root, &files, &dupes_config);

    assert!(
        report.clone_groups.is_empty(),
        "Very high min_tokens should find no clones"
    );
}

#[test]
fn duplicate_code_find_duplicates_in_project_convenience() {
    let root = fixture_path("duplicate-code");

    let dupes_config = fallow_core::duplicates::DuplicatesConfig {
        min_tokens: 20,
        min_lines: 3,
        ..fallow_core::duplicates::DuplicatesConfig::default()
    };

    let report = fallow_core::duplicates::find_duplicates_in_project(&root, &dupes_config);

    assert!(
        !report.clone_groups.is_empty(),
        "Convenience function should detect clones"
    );
}

// ── Whole-object enum member heuristics ────────────────────────

#[test]
fn enum_whole_object_uses_no_false_positives() {
    let root = fixture_path("enum-whole-object");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_enum_member_names: Vec<&str> = results
        .unused_enum_members
        .iter()
        .map(|m| m.member_name.as_str())
        .collect();

    // Status used via Object.values — no members should be unused
    assert!(
        !unused_enum_member_names.contains(&"Active"),
        "Active should not be unused (Object.values), found: {unused_enum_member_names:?}"
    );
    assert!(
        !unused_enum_member_names.contains(&"Inactive"),
        "Inactive should not be unused (Object.values), found: {unused_enum_member_names:?}"
    );
    assert!(
        !unused_enum_member_names.contains(&"Pending"),
        "Pending should not be unused (Object.values), found: {unused_enum_member_names:?}"
    );

    // Direction used via Object.keys — no members should be unused
    assert!(
        !unused_enum_member_names.contains(&"Up"),
        "Up should not be unused (Object.keys), found: {unused_enum_member_names:?}"
    );
    assert!(
        !unused_enum_member_names.contains(&"Down"),
        "Down should not be unused (Object.keys), found: {unused_enum_member_names:?}"
    );

    // Color used via for..in — no members should be unused
    assert!(
        !unused_enum_member_names.contains(&"Red"),
        "Red should not be unused (for..in), found: {unused_enum_member_names:?}"
    );
    assert!(
        !unused_enum_member_names.contains(&"Green"),
        "Green should not be unused (for..in), found: {unused_enum_member_names:?}"
    );

    // Priority — only High accessed via computed literal, Low and Medium should be unused
    assert!(
        unused_enum_member_names.contains(&"Low"),
        "Low should be unused (only High accessed via computed), found: {unused_enum_member_names:?}"
    );
    assert!(
        unused_enum_member_names.contains(&"Medium"),
        "Medium should be unused (only High accessed via computed), found: {unused_enum_member_names:?}"
    );
}

// ── Vue SFC parsing ────────────────────────────────────────────

#[test]
fn vue_project_discovers_vue_files() {
    let root = fixture_path("vue-project");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_file_names: Vec<String> = results
        .unused_files
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    // App.vue is imported by main.ts, should NOT be unused
    assert!(
        !unused_file_names.contains(&"App.vue".to_string()),
        "App.vue should be reachable via import from main.ts, unused: {unused_file_names:?}"
    );

    // Orphan.vue is not imported by anything, should be unused
    assert!(
        unused_file_names.contains(&"Orphan.vue".to_string()),
        "Orphan.vue should be detected as unused file, found: {unused_file_names:?}"
    );
}

#[test]
fn vue_imports_mark_exports_used() {
    let root = fixture_path("vue-project");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    // formatDate is imported inside App.vue <script>, should be used
    assert!(
        !unused_export_names.contains(&"formatDate"),
        "formatDate should be used (imported in App.vue), found: {unused_export_names:?}"
    );

    // unusedUtil is not imported anywhere, should be unused
    assert!(
        unused_export_names.contains(&"unusedUtil"),
        "unusedUtil should be detected as unused export, found: {unused_export_names:?}"
    );
}

// ── Svelte SFC parsing ─────────────────────────────────────────

#[test]
fn svelte_project_discovers_svelte_files() {
    let root = fixture_path("svelte-project");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_file_names: Vec<String> = results
        .unused_files
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    // App.svelte is imported by main.ts, should NOT be unused
    assert!(
        !unused_file_names.contains(&"App.svelte".to_string()),
        "App.svelte should be reachable via import from main.ts, unused: {unused_file_names:?}"
    );

    // Orphan.svelte is not imported, should be unused
    assert!(
        unused_file_names.contains(&"Orphan.svelte".to_string()),
        "Orphan.svelte should be detected as unused file, found: {unused_file_names:?}"
    );
}

#[test]
fn svelte_imports_mark_exports_used() {
    let root = fixture_path("svelte-project");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    // formatName is imported inside App.svelte, should be used
    assert!(
        !unused_export_names.contains(&"formatName"),
        "formatName should be used (imported in App.svelte), found: {unused_export_names:?}"
    );

    // unusedUtil is not imported anywhere, should be unused
    assert!(
        unused_export_names.contains(&"unusedUtil"),
        "unusedUtil should be detected as unused export, found: {unused_export_names:?}"
    );
}

// ── Dynamic import pattern resolution ──────────────────────────

#[test]
fn dynamic_import_pattern_makes_files_reachable() {
    let root = fixture_path("dynamic-import-patterns");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_file_names: Vec<String> = results
        .unused_files
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    // Locale files should be reachable via template literal pattern
    assert!(
        !unused_file_names.contains(&"en.ts".to_string()),
        "en.ts should be reachable via template literal import pattern, unused: {unused_file_names:?}"
    );
    assert!(
        !unused_file_names.contains(&"fr.ts".to_string()),
        "fr.ts should be reachable via template literal import pattern, unused: {unused_file_names:?}"
    );

    // Page files should be reachable via string concatenation pattern
    assert!(
        !unused_file_names.contains(&"home.ts".to_string()),
        "home.ts should be reachable via concat import pattern, unused: {unused_file_names:?}"
    );
    assert!(
        !unused_file_names.contains(&"about.ts".to_string()),
        "about.ts should be reachable via concat import pattern, unused: {unused_file_names:?}"
    );

    // utils.ts should be reachable via static dynamic import
    assert!(
        !unused_file_names.contains(&"utils.ts".to_string()),
        "utils.ts should be reachable via static dynamic import"
    );

    // orphan.ts should still be unused
    assert!(
        unused_file_names.contains(&"orphan.ts".to_string()),
        "orphan.ts should be detected as unused file, found: {unused_file_names:?}"
    );
}

// ── Vite import.meta.glob ──────────────────────────────────────

#[test]
fn vite_glob_makes_files_reachable() {
    let root = fixture_path("vite-glob");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_file_names: Vec<String> = results
        .unused_files
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    // Components matched by import.meta.glob('./components/*.ts') should be reachable
    assert!(
        !unused_file_names.contains(&"Button.ts".to_string()),
        "Button.ts should be reachable via import.meta.glob, unused: {unused_file_names:?}"
    );
    assert!(
        !unused_file_names.contains(&"Modal.ts".to_string()),
        "Modal.ts should be reachable via import.meta.glob, unused: {unused_file_names:?}"
    );

    // orphan.ts is outside components/, should be unused
    assert!(
        unused_file_names.contains(&"orphan.ts".to_string()),
        "orphan.ts should be unused (not matched by glob), found: {unused_file_names:?}"
    );
}

// ── Webpack require.context ────────────────────────────────────

#[test]
fn webpack_context_makes_files_reachable() {
    let root = fixture_path("webpack-context");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_file_names: Vec<String> = results
        .unused_files
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    // Icons matched by require.context('./icons', false) should be reachable
    assert!(
        !unused_file_names.contains(&"arrow.ts".to_string()),
        "arrow.ts should be reachable via require.context, unused: {unused_file_names:?}"
    );
    assert!(
        !unused_file_names.contains(&"star.ts".to_string()),
        "star.ts should be reachable via require.context, unused: {unused_file_names:?}"
    );

    // orphan.ts is outside icons/, should be unused
    assert!(
        unused_file_names.contains(&"orphan.ts".to_string()),
        "orphan.ts should be unused (not in icons/), found: {unused_file_names:?}"
    );
}

// ── External plugins ───────────────────────────────────────────

#[test]
fn external_plugin_entry_points_discovered() {
    let root = fixture_path("external-plugins");
    let config = FallowConfig {
        schema: None,
        extends: vec![],
        entry: vec![],
        ignore_patterns: vec![],
        framework: vec![],
        workspaces: None,
        ignore_dependencies: vec![],
        ignore_exports: vec![],
        duplicates: fallow_config::DuplicatesConfig::default(),
        rules: RulesConfig::default(),
        production: false,
        plugins: vec![],
        overrides: vec![],
    }
    .resolve(root.clone(), OutputFormat::Human, 4, true);

    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_file_names: Vec<String> = results
        .unused_files
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    // home.ts is a route file — external plugin marks src/routes/**/*.{ts,tsx} as entry points
    assert!(
        !unused_file_names.contains(&"home.ts".to_string()),
        "home.ts should be an entry point via external plugin, unused: {unused_file_names:?}"
    );

    // setup.ts is always-used via external plugin
    assert!(
        !unused_file_names.contains(&"setup.ts".to_string()),
        "setup.ts should be always-used via external plugin, unused: {unused_file_names:?}"
    );

    // orphan.ts is NOT covered by the plugin, should be unused
    assert!(
        unused_file_names.contains(&"orphan.ts".to_string()),
        "orphan.ts should be unused, found: {unused_file_names:?}"
    );
}

#[test]
fn external_plugin_used_exports_respected() {
    let root = fixture_path("external-plugins");
    let config = FallowConfig {
        schema: None,
        extends: vec![],
        entry: vec![],
        ignore_patterns: vec![],
        framework: vec![],
        workspaces: None,
        ignore_dependencies: vec![],
        ignore_exports: vec![],
        duplicates: fallow_config::DuplicatesConfig::default(),
        rules: RulesConfig::default(),
        production: false,
        plugins: vec![],
        overrides: vec![],
    }
    .resolve(root.clone(), OutputFormat::Human, 4, true);

    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    // `default` and `loader` exports are marked as used by the plugin
    assert!(
        !unused_export_names.contains(&"default"),
        "default export should be used via external plugin used_exports"
    );
    assert!(
        !unused_export_names.contains(&"loader"),
        "loader export should be used via external plugin used_exports"
    );

    // `unused` export in utils.ts (not an entry point) should be flagged
    assert!(
        unused_export_names.contains(&"unused"),
        "unused export in utils.ts should be flagged, found: {unused_export_names:?}"
    );
}

#[test]
fn external_plugin_tooling_dependencies_not_flagged() {
    let root = fixture_path("external-plugins");
    let config = FallowConfig {
        schema: None,
        extends: vec![],
        entry: vec![],
        ignore_patterns: vec![],
        framework: vec![],
        workspaces: None,
        ignore_dependencies: vec![],
        ignore_exports: vec![],
        duplicates: fallow_config::DuplicatesConfig::default(),
        rules: RulesConfig::default(),
        production: false,
        plugins: vec![],
        overrides: vec![],
    }
    .resolve(root.clone(), OutputFormat::Human, 4, true);

    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_dev_dep_names: Vec<&str> = results
        .unused_dev_dependencies
        .iter()
        .map(|d| d.package_name.as_str())
        .collect();

    // my-framework-cli is listed as tooling dependency in the external plugin
    assert!(
        !unused_dev_dep_names.contains(&"my-framework-cli"),
        "my-framework-cli should not be flagged (tooling dep), found: {unused_dev_dep_names:?}"
    );
}

#[test]
fn external_plugin_active_in_list() {
    let root = fixture_path("external-plugins");
    let config = FallowConfig {
        schema: None,
        extends: vec![],
        entry: vec![],
        ignore_patterns: vec![],
        framework: vec![],
        workspaces: None,
        ignore_dependencies: vec![],
        ignore_exports: vec![],
        duplicates: fallow_config::DuplicatesConfig::default(),
        rules: RulesConfig::default(),
        production: false,
        plugins: vec![],
        overrides: vec![],
    }
    .resolve(root.clone(), OutputFormat::Human, 4, true);

    let files = fallow_core::discover::discover_files(&config);
    let file_paths: Vec<std::path::PathBuf> = files.iter().map(|f| f.path.clone()).collect();

    let pkg_path = root.join("package.json");
    let pkg = fallow_config::PackageJson::load(&pkg_path).unwrap();

    let registry = fallow_core::plugins::PluginRegistry::new(config.external_plugins.clone());
    let result = registry.run(&pkg, &root, &file_paths);

    assert!(
        result.active_plugins.contains(&"my-framework".to_string()),
        "my-framework external plugin should be active, found: {:?}",
        result.active_plugins
    );
}

#[test]
fn external_plugin_config_patterns_always_used() {
    let root = fixture_path("external-plugins");
    let config = FallowConfig {
        schema: None,
        extends: vec![],
        entry: vec![],
        ignore_patterns: vec![],
        framework: vec![],
        workspaces: None,
        ignore_dependencies: vec![],
        ignore_exports: vec![],
        duplicates: fallow_config::DuplicatesConfig::default(),
        rules: RulesConfig::default(),
        production: false,
        plugins: vec![],
        overrides: vec![],
    }
    .resolve(root.clone(), OutputFormat::Human, 4, true);

    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_file_names: Vec<String> = results
        .unused_files
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    // my-framework.config.ts is matched by config_patterns, should be always-used
    assert!(
        !unused_file_names.contains(&"my-framework.config.ts".to_string()),
        "my-framework.config.ts should be always-used via config_patterns, unused: {unused_file_names:?}"
    );
}

#[test]
fn css_apply_marks_tailwind_as_used() {
    let root = fixture_path("css-apply-project");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    // tailwindcss should NOT be in unused dependencies (it's used via @apply in styles.css)
    let unused_dep_names: Vec<&str> = results
        .unused_dependencies
        .iter()
        .map(|d| d.package_name.as_str())
        .collect();
    assert!(
        !unused_dep_names.contains(&"tailwindcss"),
        "tailwindcss should not be unused, it's referenced via @apply in CSS: {unused_dep_names:?}"
    );

    // unused.css should be detected as an unused file
    let unused_files: Vec<&str> = results
        .unused_files
        .iter()
        .filter_map(|f| f.path.file_name())
        .filter_map(|f| f.to_str())
        .collect();
    assert!(
        unused_files.contains(&"unused.css"),
        "unused.css should be detected as unused: {unused_files:?}"
    );
}

// ── Workspace exports map resolution ───────────────────────────

#[test]
fn workspace_exports_map_resolves_subpath_imports() {
    let root = fixture_path("workspace-exports-map");

    // Set up node_modules symlinks for cross-workspace resolution
    let nm = root.join("node_modules");
    let _ = std::fs::create_dir_all(nm.join("@workspace"));
    #[cfg(unix)]
    {
        let _ = std::os::unix::fs::symlink(root.join("packages/ui"), nm.join("@workspace/ui"));
    }

    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_file_names: Vec<String> = results
        .unused_files
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    // orphan.ts is not exported via exports map and not imported — should be unused
    assert!(
        unused_file_names.contains(&"orphan.ts".to_string()),
        "orphan.ts should be detected as unused file, found: {unused_file_names:?}"
    );

    // utils.ts is imported via `@workspace/ui/utils` through exports map → should NOT be unused
    assert!(
        !unused_file_names.contains(&"utils.ts".to_string()),
        "utils.ts should be reachable via exports map subpath import, unused: {unused_file_names:?}"
    );

    // helpers.ts (source) should be reachable via exports map pointing to dist/helpers.js
    // fallow should map dist/helpers.js back to src/helpers.ts
    assert!(
        !unused_file_names.contains(&"helpers.ts".to_string()),
        "helpers.ts should be reachable via dist→src fallback from exports map, unused: {unused_file_names:?}"
    );

    // internal.ts is imported by utils.ts, so it should be reachable
    assert!(
        !unused_file_names.contains(&"internal.ts".to_string()),
        "internal.ts should be reachable via import from utils.ts, unused: {unused_file_names:?}"
    );

    // Unused exports on non-entry-point files should still be detected.
    // internal.ts is NOT an entry point (not in exports map) but is imported
    // by utils.ts — so its unused exports should be flagged.
    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    assert!(
        unused_export_names.contains(&"unusedInternal"),
        "unusedInternal should be unused (internal.ts is not an entry point), found: {unused_export_names:?}"
    );

    // Used exports should NOT be flagged
    assert!(
        !unused_export_names.contains(&"internalHelper"),
        "internalHelper should be used (imported by utils.ts)"
    );

    // No unresolved imports — exports map subpaths should all resolve
    assert!(
        results.unresolved_imports.is_empty(),
        "should have no unresolved imports, found: {:?}",
        results
            .unresolved_imports
            .iter()
            .map(|i| &i.specifier)
            .collect::<Vec<_>>()
    );
}

// ── Incremental analysis (Phase A) tests ──────────────────────────

fn create_config_with_cache(
    root: PathBuf,
    cache_dir: std::path::PathBuf,
) -> fallow_config::ResolvedConfig {
    let mut config = FallowConfig {
        schema: None,
        extends: vec![],
        entry: vec![],
        ignore_patterns: vec![],
        framework: vec![],
        workspaces: None,
        ignore_dependencies: vec![],
        ignore_exports: vec![],
        duplicates: fallow_config::DuplicatesConfig::default(),
        rules: RulesConfig::default(),
        production: false,
        plugins: vec![],
        overrides: vec![],
    }
    .resolve(root, OutputFormat::Human, 4, false); // no_cache = false to enable caching
    config.cache_dir = cache_dir;
    config
}

#[test]
fn incremental_no_cache_all_misses() {
    // First run without any existing cache: all files should be cache misses
    let root = fixture_path("basic-project");
    let files = fallow_core::discover::discover_files(&create_config(root.clone()));
    let parse_result = fallow_core::extract::parse_all_files(&files, None);

    assert_eq!(parse_result.cache_hits, 0);
    assert_eq!(parse_result.cache_misses, parse_result.modules.len());
    assert!(!parse_result.modules.is_empty());
}

#[test]
fn incremental_with_cache_all_hits() {
    // Build a cache from the first parse, then parse again — should be all hits
    let root = fixture_path("basic-project");
    let config = create_config(root.clone());
    let files = fallow_core::discover::discover_files(&config);

    // First parse: build cache
    let first = fallow_core::extract::parse_all_files(&files, None);
    let mut cache_store = fallow_core::cache::CacheStore::new();
    for module in &first.modules {
        if let Some(file) = files.get(module.file_id.0 as usize) {
            cache_store.insert(&file.path, fallow_core::cache::module_to_cached(module));
        }
    }

    // Second parse: should hit cache for every file
    let second = fallow_core::extract::parse_all_files(&files, Some(&cache_store));
    assert_eq!(second.cache_hits, first.modules.len());
    assert_eq!(second.cache_misses, 0);
    assert_eq!(second.modules.len(), first.modules.len());
}

#[test]
fn incremental_results_identical() {
    // Results from a cached run should be identical to a fresh run
    let root = fixture_path("basic-project");
    let config = create_config(root.clone());
    let files = fallow_core::discover::discover_files(&config);

    // First parse
    let first = fallow_core::extract::parse_all_files(&files, None);
    let mut cache_store = fallow_core::cache::CacheStore::new();
    for module in &first.modules {
        if let Some(file) = files.get(module.file_id.0 as usize) {
            cache_store.insert(&file.path, fallow_core::cache::module_to_cached(module));
        }
    }

    // Second parse (from cache)
    let second = fallow_core::extract::parse_all_files(&files, Some(&cache_store));

    // Verify all module data matches
    assert_eq!(first.modules.len(), second.modules.len());
    for (a, b) in first.modules.iter().zip(second.modules.iter()) {
        assert_eq!(a.file_id, b.file_id);
        assert_eq!(a.content_hash, b.content_hash);
        assert_eq!(a.exports.len(), b.exports.len());
        assert_eq!(a.imports.len(), b.imports.len());
        assert_eq!(a.re_exports.len(), b.re_exports.len());
        assert_eq!(a.dynamic_imports.len(), b.dynamic_imports.len());
        assert_eq!(a.has_cjs_exports, b.has_cjs_exports);
        assert_eq!(a.suppressions.len(), b.suppressions.len());
    }
}

#[test]
fn incremental_full_pipeline_results_match() {
    // Full pipeline results should be identical whether using cache or not
    let root = fixture_path("basic-project");
    let tmp_cache = tempfile::tempdir().expect("create temp dir");
    let config = create_config_with_cache(root.clone(), tmp_cache.path().to_path_buf());

    // First run: populates cache
    let first = fallow_core::analyze(&config).expect("first analysis should succeed");

    // Second run: uses cache
    let second = fallow_core::analyze(&config).expect("second analysis should succeed");

    // Results should be identical
    assert_eq!(first.unused_files.len(), second.unused_files.len());
    assert_eq!(first.unused_exports.len(), second.unused_exports.len());
    assert_eq!(first.unused_types.len(), second.unused_types.len());
    assert_eq!(
        first.unresolved_imports.len(),
        second.unresolved_imports.len()
    );
}

#[test]
fn incremental_cache_prune_stale_entries() {
    // Cache entries for deleted files should be pruned
    let mut store = fallow_core::cache::CacheStore::new();
    let make_module = || fallow_core::cache::CachedModule {
        content_hash: 1,
        exports: vec![],
        imports: vec![],
        re_exports: vec![],
        dynamic_imports: vec![],
        require_calls: vec![],
        member_accesses: vec![],
        whole_object_uses: vec![],
        dynamic_import_patterns: vec![],
        has_cjs_exports: false,
        suppressions: vec![],
    };

    store.insert(std::path::Path::new("/project/existing.ts"), make_module());
    store.insert(std::path::Path::new("/project/deleted.ts"), make_module());
    assert_eq!(store.len(), 2);

    // Only "existing.ts" is in the current file set
    let files = vec![fallow_core::discover::DiscoveredFile {
        id: fallow_core::discover::FileId(0),
        path: PathBuf::from("/project/existing.ts"),
        size_bytes: 100,
    }];
    store.retain_paths(&files);

    assert_eq!(store.len(), 1);
    assert!(
        store
            .get_by_path_only(std::path::Path::new("/project/existing.ts"))
            .is_some()
    );
    assert!(
        store
            .get_by_path_only(std::path::Path::new("/project/deleted.ts"))
            .is_none()
    );
}

// ── CSS Modules support ────────────────────────────────────────

#[test]
fn css_modules_exports_tracked() {
    let root = fixture_path("css-modules-project");
    let config = create_config(root.clone());
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_file_names: Vec<String> = results
        .unused_files
        .iter()
        .filter_map(|f| f.path.file_name())
        .filter_map(|n| n.to_str())
        .map(|s| s.to_string())
        .collect();
    assert!(
        unused_file_names.contains(&"unused.module.css".to_string()),
        "unused.module.css should be unused: {unused_file_names:?}"
    );
    assert!(
        !unused_file_names.contains(&"Layout.module.css".to_string()),
        "Layout.module.css should be used via named imports: {unused_file_names:?}"
    );
    assert!(
        !unused_file_names.contains(&"Button.module.css".to_string()),
        "Button.module.css should be used via default import: {unused_file_names:?}"
    );

    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();
    assert!(
        unused_export_names.contains(&"sidebar"),
        "sidebar should be an unused export: {unused_export_names:?}"
    );
    assert!(
        unused_export_names.contains(&"secondary"),
        "secondary should be an unused export: {unused_export_names:?}"
    );
    assert!(
        !unused_export_names.contains(&"header"),
        "header should be used via named import: {unused_export_names:?}"
    );
    assert!(
        !unused_export_names.contains(&"footer"),
        "footer should be used via named import: {unused_export_names:?}"
    );
    assert!(
        !unused_export_names.contains(&"primary"),
        "primary should be used via member access: {unused_export_names:?}"
    );
    assert!(
        unused_file_names.contains(&"regular.css".to_string()),
        "regular.css should be unused: {unused_file_names:?}"
    );
}
