use std::path::{Path, PathBuf};
use std::time::Duration;

use fallow_cli::report::{build_compact_lines, build_json, build_sarif};
use fallow_config::RulesConfig;
use fallow_core::extract::MemberKind;
use fallow_core::results::*;

/// Build sample `AnalysisResults` with one issue of each type for consistent snapshots.
fn sample_results(root: &Path) -> AnalysisResults {
    let mut r = AnalysisResults::default();

    r.unused_files.push(UnusedFile {
        path: root.join("src/dead.ts"),
    });
    r.unused_exports.push(UnusedExport {
        path: root.join("src/utils.ts"),
        export_name: "helperFn".to_string(),
        is_type_only: false,
        line: 10,
        col: 4,
        span_start: 120,
        is_re_export: false,
    });
    r.unused_types.push(UnusedExport {
        path: root.join("src/types.ts"),
        export_name: "OldType".to_string(),
        is_type_only: true,
        line: 5,
        col: 0,
        span_start: 60,
        is_re_export: false,
    });
    r.unused_dependencies.push(UnusedDependency {
        package_name: "lodash".to_string(),
        location: DependencyLocation::Dependencies,
        path: root.join("package.json"),
    });
    r.unused_dev_dependencies.push(UnusedDependency {
        package_name: "jest".to_string(),
        location: DependencyLocation::DevDependencies,
        path: root.join("package.json"),
    });
    r.unused_enum_members.push(UnusedMember {
        path: root.join("src/enums.ts"),
        parent_name: "Status".to_string(),
        member_name: "Deprecated".to_string(),
        kind: MemberKind::EnumMember,
        line: 8,
        col: 2,
    });
    r.unused_class_members.push(UnusedMember {
        path: root.join("src/service.ts"),
        parent_name: "UserService".to_string(),
        member_name: "legacyMethod".to_string(),
        kind: MemberKind::ClassMethod,
        line: 42,
        col: 4,
    });
    r.unresolved_imports.push(UnresolvedImport {
        path: root.join("src/app.ts"),
        specifier: "./missing-module".to_string(),
        line: 3,
        col: 0,
    });
    r.unlisted_dependencies.push(UnlistedDependency {
        package_name: "chalk".to_string(),
        imported_from: vec![root.join("src/cli.ts")],
    });
    r.duplicate_exports.push(DuplicateExport {
        export_name: "Config".to_string(),
        locations: vec![
            DuplicateLocation {
                path: root.join("src/config.ts"),
                line: 15,
                col: 0,
            },
            DuplicateLocation {
                path: root.join("src/types.ts"),
                line: 30,
                col: 0,
            },
        ],
    });
    r.unused_optional_dependencies.push(UnusedDependency {
        package_name: "fsevents".to_string(),
        location: DependencyLocation::OptionalDependencies,
        path: root.join("package.json"),
    });
    r.type_only_dependencies.push(TypeOnlyDependency {
        package_name: "zod".to_string(),
        path: root.join("package.json"),
    });
    r.circular_dependencies.push(CircularDependency {
        files: vec![root.join("src/a.ts"), root.join("src/b.ts")],
        length: 2,
    });

    r
}

// ── JSON format ──────────────────────────────────────────────────

#[test]
fn json_output_snapshot() {
    let root = PathBuf::from("/project");
    let results = sample_results(&root);
    let elapsed = Duration::from_millis(42);
    let value = build_json(&results, &root, elapsed).expect("JSON build should succeed");
    let json_str = serde_json::to_string_pretty(&value).expect("should serialize");

    // Redact dynamic values (version changes with releases, elapsed_ms may vary)
    insta::assert_snapshot!(
        "json_output",
        json_str.replace(
            &format!("\"version\": \"{}\"", env!("CARGO_PKG_VERSION")),
            "\"version\": \"[VERSION]\"",
        )
    );
}

#[test]
fn json_empty_results_snapshot() {
    let root = PathBuf::from("/project");
    let results = AnalysisResults::default();
    let elapsed = Duration::from_millis(0);
    let value = build_json(&results, &root, elapsed).expect("JSON build should succeed");
    let json_str = serde_json::to_string_pretty(&value).expect("should serialize");

    insta::assert_snapshot!(
        "json_empty",
        json_str.replace(
            &format!("\"version\": \"{}\"", env!("CARGO_PKG_VERSION")),
            "\"version\": \"[VERSION]\"",
        )
    );
}

// ── SARIF format ─────────────────────────────────────────────────

#[test]
fn sarif_output_snapshot() {
    let root = PathBuf::from("/project");
    let results = sample_results(&root);
    let rules = RulesConfig::default();
    let sarif = build_sarif(&results, &root, &rules);
    let json_str = serde_json::to_string_pretty(&sarif).expect("should serialize");

    insta::assert_snapshot!(
        "sarif_output",
        json_str.replace(
            &format!("\"version\": \"{}\"", env!("CARGO_PKG_VERSION")),
            "\"version\": \"[VERSION]\"",
        )
    );
}

#[test]
fn sarif_empty_results_snapshot() {
    let root = PathBuf::from("/project");
    let results = AnalysisResults::default();
    let rules = RulesConfig::default();
    let sarif = build_sarif(&results, &root, &rules);
    let json_str = serde_json::to_string_pretty(&sarif).expect("should serialize");

    insta::assert_snapshot!(
        "sarif_empty",
        json_str.replace(
            &format!("\"version\": \"{}\"", env!("CARGO_PKG_VERSION")),
            "\"version\": \"[VERSION]\"",
        )
    );
}

// ── Compact format ───────────────────────────────────────────────

#[test]
fn compact_output_snapshot() {
    let root = PathBuf::from("/project");
    let results = sample_results(&root);
    let lines = build_compact_lines(&results, &root);
    let output = lines.join("\n");

    insta::assert_snapshot!("compact_output", output);
}

#[test]
fn compact_empty_results_snapshot() {
    let root = PathBuf::from("/project");
    let results = AnalysisResults::default();
    let lines = build_compact_lines(&results, &root);
    let output = lines.join("\n");

    insta::assert_snapshot!("compact_empty", output);
}

// ── Per-issue-type compact snapshots ────────────────────────────

#[test]
fn compact_unused_files_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_files.push(UnusedFile {
        path: root.join("src/dead.ts"),
    });
    results.unused_files.push(UnusedFile {
        path: root.join("src/orphan.ts"),
    });
    let lines = build_compact_lines(&results, &root);
    insta::assert_snapshot!("compact_unused_files_only", lines.join("\n"));
}

#[test]
fn compact_unused_exports_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_exports.push(UnusedExport {
        path: root.join("src/utils.ts"),
        export_name: "helperFn".to_string(),
        is_type_only: false,
        line: 10,
        col: 4,
        span_start: 120,
        is_re_export: false,
    });
    results.unused_exports.push(UnusedExport {
        path: root.join("src/utils.ts"),
        export_name: "formatDate".to_string(),
        is_type_only: false,
        line: 25,
        col: 0,
        span_start: 300,
        is_re_export: false,
    });
    let lines = build_compact_lines(&results, &root);
    insta::assert_snapshot!("compact_unused_exports_only", lines.join("\n"));
}

#[test]
fn compact_unused_types_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_types.push(UnusedExport {
        path: root.join("src/types.ts"),
        export_name: "OldType".to_string(),
        is_type_only: true,
        line: 5,
        col: 0,
        span_start: 60,
        is_re_export: false,
    });
    let lines = build_compact_lines(&results, &root);
    insta::assert_snapshot!("compact_unused_types_only", lines.join("\n"));
}

#[test]
fn compact_unused_deps_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_dependencies.push(UnusedDependency {
        package_name: "lodash".to_string(),
        location: DependencyLocation::Dependencies,
        path: root.join("package.json"),
    });
    results.unused_dependencies.push(UnusedDependency {
        package_name: "moment".to_string(),
        location: DependencyLocation::Dependencies,
        path: root.join("package.json"),
    });
    let lines = build_compact_lines(&results, &root);
    insta::assert_snapshot!("compact_unused_deps_only", lines.join("\n"));
}

#[test]
fn compact_unused_dev_deps_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_dev_dependencies.push(UnusedDependency {
        package_name: "jest".to_string(),
        location: DependencyLocation::DevDependencies,
        path: root.join("package.json"),
    });
    let lines = build_compact_lines(&results, &root);
    insta::assert_snapshot!("compact_unused_dev_deps_only", lines.join("\n"));
}

#[test]
fn compact_unused_optional_deps_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_optional_dependencies.push(UnusedDependency {
        package_name: "fsevents".to_string(),
        location: DependencyLocation::OptionalDependencies,
        path: root.join("package.json"),
    });
    let lines = build_compact_lines(&results, &root);
    insta::assert_snapshot!("compact_unused_optional_deps_only", lines.join("\n"));
}

#[test]
fn compact_unresolved_imports_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unresolved_imports.push(UnresolvedImport {
        path: root.join("src/app.ts"),
        specifier: "./missing-module".to_string(),
        line: 3,
        col: 0,
    });
    results.unresolved_imports.push(UnresolvedImport {
        path: root.join("src/app.ts"),
        specifier: "@org/nonexistent".to_string(),
        line: 4,
        col: 0,
    });
    let lines = build_compact_lines(&results, &root);
    insta::assert_snapshot!("compact_unresolved_imports_only", lines.join("\n"));
}

#[test]
fn compact_unlisted_deps_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unlisted_dependencies.push(UnlistedDependency {
        package_name: "chalk".to_string(),
        imported_from: vec![root.join("src/cli.ts")],
    });
    let lines = build_compact_lines(&results, &root);
    insta::assert_snapshot!("compact_unlisted_deps_only", lines.join("\n"));
}

#[test]
fn compact_unused_enum_members_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_enum_members.push(UnusedMember {
        path: root.join("src/enums.ts"),
        parent_name: "Status".to_string(),
        member_name: "Deprecated".to_string(),
        kind: MemberKind::EnumMember,
        line: 8,
        col: 2,
    });
    let lines = build_compact_lines(&results, &root);
    insta::assert_snapshot!("compact_unused_enum_members_only", lines.join("\n"));
}

#[test]
fn compact_unused_class_members_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_class_members.push(UnusedMember {
        path: root.join("src/service.ts"),
        parent_name: "UserService".to_string(),
        member_name: "legacyMethod".to_string(),
        kind: MemberKind::ClassMethod,
        line: 42,
        col: 4,
    });
    let lines = build_compact_lines(&results, &root);
    insta::assert_snapshot!("compact_unused_class_members_only", lines.join("\n"));
}

#[test]
fn compact_duplicate_exports_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.duplicate_exports.push(DuplicateExport {
        export_name: "Config".to_string(),
        locations: vec![
            DuplicateLocation {
                path: root.join("src/config.ts"),
                line: 15,
                col: 0,
            },
            DuplicateLocation {
                path: root.join("src/types.ts"),
                line: 30,
                col: 0,
            },
        ],
    });
    let lines = build_compact_lines(&results, &root);
    insta::assert_snapshot!("compact_duplicate_exports_only", lines.join("\n"));
}

// ── Re-export variant snapshots ─────────────────────────────────

#[test]
fn compact_re_export_variant_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_exports.push(UnusedExport {
        path: root.join("src/index.ts"),
        export_name: "reExportedFn".to_string(),
        is_type_only: false,
        line: 1,
        col: 0,
        span_start: 0,
        is_re_export: true,
    });
    results.unused_types.push(UnusedExport {
        path: root.join("src/index.ts"),
        export_name: "ReExportedType".to_string(),
        is_type_only: true,
        line: 2,
        col: 0,
        span_start: 30,
        is_re_export: true,
    });
    let lines = build_compact_lines(&results, &root);
    insta::assert_snapshot!("compact_re_export_variants", lines.join("\n"));
}

#[test]
fn json_re_export_variant_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_exports.push(UnusedExport {
        path: root.join("src/index.ts"),
        export_name: "reExportedFn".to_string(),
        is_type_only: false,
        line: 1,
        col: 0,
        span_start: 0,
        is_re_export: true,
    });
    let elapsed = Duration::from_millis(0);
    let value = build_json(&results, &root, elapsed).expect("JSON build should succeed");
    let json_str = serde_json::to_string_pretty(&value).expect("should serialize");
    insta::assert_snapshot!(
        "json_re_export_variant",
        json_str.replace(
            &format!("\"version\": \"{}\"", env!("CARGO_PKG_VERSION")),
            "\"version\": \"[VERSION]\"",
        )
    );
}

#[test]
fn sarif_re_export_variant_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_exports.push(UnusedExport {
        path: root.join("src/index.ts"),
        export_name: "reExportedFn".to_string(),
        is_type_only: false,
        line: 1,
        col: 0,
        span_start: 0,
        is_re_export: true,
    });
    let rules = RulesConfig::default();
    let sarif = build_sarif(&results, &root, &rules);
    let json_str = serde_json::to_string_pretty(&sarif).expect("should serialize");
    insta::assert_snapshot!(
        "sarif_re_export_variant",
        json_str.replace(
            &format!("\"version\": \"{}\"", env!("CARGO_PKG_VERSION")),
            "\"version\": \"[VERSION]\"",
        )
    );
}

// ── SARIF with mixed severity levels ────────────────────────────

#[test]
fn sarif_mixed_severity_snapshot() {
    let root = PathBuf::from("/project");
    let results = sample_results(&root);
    let rules = RulesConfig {
        unused_files: fallow_config::Severity::Error,
        unused_exports: fallow_config::Severity::Warn,
        unused_types: fallow_config::Severity::Warn,
        unused_dependencies: fallow_config::Severity::Error,
        unused_dev_dependencies: fallow_config::Severity::Warn,
        unused_optional_dependencies: fallow_config::Severity::Warn,
        unused_enum_members: fallow_config::Severity::Warn,
        unused_class_members: fallow_config::Severity::Warn,
        unresolved_imports: fallow_config::Severity::Error,
        unlisted_dependencies: fallow_config::Severity::Error,
        duplicate_exports: fallow_config::Severity::Warn,
        type_only_dependencies: fallow_config::Severity::Warn,
        circular_dependencies: fallow_config::Severity::Warn,
    };
    let sarif = build_sarif(&results, &root, &rules);
    let json_str = serde_json::to_string_pretty(&sarif).expect("should serialize");
    insta::assert_snapshot!(
        "sarif_mixed_severity",
        json_str.replace(
            &format!("\"version\": \"{}\"", env!("CARGO_PKG_VERSION")),
            "\"version\": \"[VERSION]\"",
        )
    );
}

// ── Type-only dependency snapshots ──────────────────────────────

#[test]
fn json_type_only_deps_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.type_only_dependencies.push(TypeOnlyDependency {
        package_name: "zod".to_string(),
        path: root.join("package.json"),
    });
    results.type_only_dependencies.push(TypeOnlyDependency {
        package_name: "@types/react".to_string(),
        path: root.join("package.json"),
    });
    let elapsed = Duration::from_millis(10);
    let value = build_json(&results, &root, elapsed).expect("JSON build should succeed");
    let json_str = serde_json::to_string_pretty(&value).expect("should serialize");
    insta::assert_snapshot!(
        "json_type_only_deps",
        json_str.replace(
            &format!("\"version\": \"{}\"", env!("CARGO_PKG_VERSION")),
            "\"version\": \"[VERSION]\"",
        )
    );
}

// ── Per-issue-type JSON snapshots ───────────────────────────────

fn redact_version(json_str: &str) -> String {
    json_str.replace(
        &format!("\"version\": \"{}\"", env!("CARGO_PKG_VERSION")),
        "\"version\": \"[VERSION]\"",
    )
}

#[test]
fn json_unused_files_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_files.push(UnusedFile {
        path: root.join("src/dead.ts"),
    });
    let value = build_json(&results, &root, Duration::ZERO).expect("JSON build should succeed");
    let json_str = serde_json::to_string_pretty(&value).expect("should serialize");
    insta::assert_snapshot!("json_unused_files_only", redact_version(&json_str));
}

#[test]
fn json_unused_exports_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_exports.push(UnusedExport {
        path: root.join("src/utils.ts"),
        export_name: "helperFn".to_string(),
        is_type_only: false,
        line: 10,
        col: 4,
        span_start: 120,
        is_re_export: false,
    });
    let value = build_json(&results, &root, Duration::ZERO).expect("JSON build should succeed");
    let json_str = serde_json::to_string_pretty(&value).expect("should serialize");
    insta::assert_snapshot!("json_unused_exports_only", redact_version(&json_str));
}

#[test]
fn json_unused_types_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_types.push(UnusedExport {
        path: root.join("src/types.ts"),
        export_name: "OldType".to_string(),
        is_type_only: true,
        line: 5,
        col: 0,
        span_start: 60,
        is_re_export: false,
    });
    let value = build_json(&results, &root, Duration::ZERO).expect("JSON build should succeed");
    let json_str = serde_json::to_string_pretty(&value).expect("should serialize");
    insta::assert_snapshot!("json_unused_types_only", redact_version(&json_str));
}

#[test]
fn json_unused_deps_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_dependencies.push(UnusedDependency {
        package_name: "lodash".to_string(),
        location: DependencyLocation::Dependencies,
        path: root.join("package.json"),
    });
    let value = build_json(&results, &root, Duration::ZERO).expect("JSON build should succeed");
    let json_str = serde_json::to_string_pretty(&value).expect("should serialize");
    insta::assert_snapshot!("json_unused_deps_only", redact_version(&json_str));
}

#[test]
fn json_unresolved_imports_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unresolved_imports.push(UnresolvedImport {
        path: root.join("src/app.ts"),
        specifier: "./missing-module".to_string(),
        line: 3,
        col: 0,
    });
    let value = build_json(&results, &root, Duration::ZERO).expect("JSON build should succeed");
    let json_str = serde_json::to_string_pretty(&value).expect("should serialize");
    insta::assert_snapshot!("json_unresolved_imports_only", redact_version(&json_str));
}

#[test]
fn json_unlisted_deps_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unlisted_dependencies.push(UnlistedDependency {
        package_name: "chalk".to_string(),
        imported_from: vec![root.join("src/cli.ts")],
    });
    let value = build_json(&results, &root, Duration::ZERO).expect("JSON build should succeed");
    let json_str = serde_json::to_string_pretty(&value).expect("should serialize");
    insta::assert_snapshot!("json_unlisted_deps_only", redact_version(&json_str));
}

#[test]
fn json_unused_enum_members_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_enum_members.push(UnusedMember {
        path: root.join("src/enums.ts"),
        parent_name: "Status".to_string(),
        member_name: "Deprecated".to_string(),
        kind: MemberKind::EnumMember,
        line: 8,
        col: 2,
    });
    let value = build_json(&results, &root, Duration::ZERO).expect("JSON build should succeed");
    let json_str = serde_json::to_string_pretty(&value).expect("should serialize");
    insta::assert_snapshot!("json_unused_enum_members_only", redact_version(&json_str));
}

#[test]
fn json_unused_class_members_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_class_members.push(UnusedMember {
        path: root.join("src/service.ts"),
        parent_name: "UserService".to_string(),
        member_name: "legacyMethod".to_string(),
        kind: MemberKind::ClassMethod,
        line: 42,
        col: 4,
    });
    let value = build_json(&results, &root, Duration::ZERO).expect("JSON build should succeed");
    let json_str = serde_json::to_string_pretty(&value).expect("should serialize");
    insta::assert_snapshot!("json_unused_class_members_only", redact_version(&json_str));
}

#[test]
fn json_duplicate_exports_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.duplicate_exports.push(DuplicateExport {
        export_name: "Config".to_string(),
        locations: vec![
            DuplicateLocation {
                path: root.join("src/config.ts"),
                line: 15,
                col: 0,
            },
            DuplicateLocation {
                path: root.join("src/types.ts"),
                line: 30,
                col: 0,
            },
        ],
    });
    let value = build_json(&results, &root, Duration::ZERO).expect("JSON build should succeed");
    let json_str = serde_json::to_string_pretty(&value).expect("should serialize");
    insta::assert_snapshot!("json_duplicate_exports_only", redact_version(&json_str));
}

// ── Per-issue-type SARIF snapshots ──────────────────────────────

fn redact_sarif_version(json_str: &str) -> String {
    json_str.replace(
        &format!("\"version\": \"{}\"", env!("CARGO_PKG_VERSION")),
        "\"version\": \"[VERSION]\"",
    )
}

#[test]
fn sarif_unused_files_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_files.push(UnusedFile {
        path: root.join("src/dead.ts"),
    });
    let sarif = build_sarif(&results, &root, &RulesConfig::default());
    let json_str = serde_json::to_string_pretty(&sarif).expect("should serialize");
    insta::assert_snapshot!("sarif_unused_files_only", redact_sarif_version(&json_str));
}

#[test]
fn sarif_unused_exports_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_exports.push(UnusedExport {
        path: root.join("src/utils.ts"),
        export_name: "helperFn".to_string(),
        is_type_only: false,
        line: 10,
        col: 4,
        span_start: 120,
        is_re_export: false,
    });
    let sarif = build_sarif(&results, &root, &RulesConfig::default());
    let json_str = serde_json::to_string_pretty(&sarif).expect("should serialize");
    insta::assert_snapshot!("sarif_unused_exports_only", redact_sarif_version(&json_str));
}

#[test]
fn sarif_unused_types_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_types.push(UnusedExport {
        path: root.join("src/types.ts"),
        export_name: "OldType".to_string(),
        is_type_only: true,
        line: 5,
        col: 0,
        span_start: 60,
        is_re_export: false,
    });
    let sarif = build_sarif(&results, &root, &RulesConfig::default());
    let json_str = serde_json::to_string_pretty(&sarif).expect("should serialize");
    insta::assert_snapshot!("sarif_unused_types_only", redact_sarif_version(&json_str));
}

#[test]
fn sarif_unused_deps_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_dependencies.push(UnusedDependency {
        package_name: "lodash".to_string(),
        location: DependencyLocation::Dependencies,
        path: root.join("package.json"),
    });
    let sarif = build_sarif(&results, &root, &RulesConfig::default());
    let json_str = serde_json::to_string_pretty(&sarif).expect("should serialize");
    insta::assert_snapshot!("sarif_unused_deps_only", redact_sarif_version(&json_str));
}

#[test]
fn sarif_unresolved_imports_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unresolved_imports.push(UnresolvedImport {
        path: root.join("src/app.ts"),
        specifier: "./missing-module".to_string(),
        line: 3,
        col: 0,
    });
    let sarif = build_sarif(&results, &root, &RulesConfig::default());
    let json_str = serde_json::to_string_pretty(&sarif).expect("should serialize");
    insta::assert_snapshot!(
        "sarif_unresolved_imports_only",
        redact_sarif_version(&json_str)
    );
}

#[test]
fn sarif_unlisted_deps_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unlisted_dependencies.push(UnlistedDependency {
        package_name: "chalk".to_string(),
        imported_from: vec![root.join("src/cli.ts")],
    });
    let sarif = build_sarif(&results, &root, &RulesConfig::default());
    let json_str = serde_json::to_string_pretty(&sarif).expect("should serialize");
    insta::assert_snapshot!("sarif_unlisted_deps_only", redact_sarif_version(&json_str));
}

#[test]
fn sarif_unused_enum_members_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_enum_members.push(UnusedMember {
        path: root.join("src/enums.ts"),
        parent_name: "Status".to_string(),
        member_name: "Deprecated".to_string(),
        kind: MemberKind::EnumMember,
        line: 8,
        col: 2,
    });
    let sarif = build_sarif(&results, &root, &RulesConfig::default());
    let json_str = serde_json::to_string_pretty(&sarif).expect("should serialize");
    insta::assert_snapshot!(
        "sarif_unused_enum_members_only",
        redact_sarif_version(&json_str)
    );
}

#[test]
fn sarif_unused_class_members_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_class_members.push(UnusedMember {
        path: root.join("src/service.ts"),
        parent_name: "UserService".to_string(),
        member_name: "legacyMethod".to_string(),
        kind: MemberKind::ClassMethod,
        line: 42,
        col: 4,
    });
    let sarif = build_sarif(&results, &root, &RulesConfig::default());
    let json_str = serde_json::to_string_pretty(&sarif).expect("should serialize");
    insta::assert_snapshot!(
        "sarif_unused_class_members_only",
        redact_sarif_version(&json_str)
    );
}

#[test]
fn sarif_duplicate_exports_only_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.duplicate_exports.push(DuplicateExport {
        export_name: "Config".to_string(),
        locations: vec![
            DuplicateLocation {
                path: root.join("src/config.ts"),
                line: 15,
                col: 0,
            },
            DuplicateLocation {
                path: root.join("src/types.ts"),
                line: 30,
                col: 0,
            },
        ],
    });
    let sarif = build_sarif(&results, &root, &RulesConfig::default());
    let json_str = serde_json::to_string_pretty(&sarif).expect("should serialize");
    insta::assert_snapshot!(
        "sarif_duplicate_exports_only",
        redact_sarif_version(&json_str)
    );
}

// ── Multiple items grouping ─────────────────────────────────────

#[test]
fn json_multiple_exports_same_file_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_exports.push(UnusedExport {
        path: root.join("src/utils.ts"),
        export_name: "helperFn".to_string(),
        is_type_only: false,
        line: 10,
        col: 4,
        span_start: 120,
        is_re_export: false,
    });
    results.unused_exports.push(UnusedExport {
        path: root.join("src/utils.ts"),
        export_name: "formatDate".to_string(),
        is_type_only: false,
        line: 25,
        col: 0,
        span_start: 300,
        is_re_export: false,
    });
    results.unused_exports.push(UnusedExport {
        path: root.join("src/helpers.ts"),
        export_name: "capitalize".to_string(),
        is_type_only: false,
        line: 1,
        col: 0,
        span_start: 0,
        is_re_export: false,
    });
    let value = build_json(&results, &root, Duration::ZERO).expect("JSON build should succeed");
    let json_str = serde_json::to_string_pretty(&value).expect("should serialize");
    insta::assert_snapshot!("json_multiple_exports_same_file", redact_version(&json_str));
}

#[test]
fn sarif_multiple_exports_same_file_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_exports.push(UnusedExport {
        path: root.join("src/utils.ts"),
        export_name: "helperFn".to_string(),
        is_type_only: false,
        line: 10,
        col: 4,
        span_start: 120,
        is_re_export: false,
    });
    results.unused_exports.push(UnusedExport {
        path: root.join("src/utils.ts"),
        export_name: "formatDate".to_string(),
        is_type_only: false,
        line: 25,
        col: 0,
        span_start: 300,
        is_re_export: false,
    });
    let rules = RulesConfig::default();
    let sarif = build_sarif(&results, &root, &rules);
    let json_str = serde_json::to_string_pretty(&sarif).expect("should serialize");
    insta::assert_snapshot!(
        "sarif_multiple_exports_same_file",
        redact_sarif_version(&json_str)
    );
}

#[test]
fn compact_multiple_exports_same_file_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_exports.push(UnusedExport {
        path: root.join("src/utils.ts"),
        export_name: "helperFn".to_string(),
        is_type_only: false,
        line: 10,
        col: 4,
        span_start: 120,
        is_re_export: false,
    });
    results.unused_exports.push(UnusedExport {
        path: root.join("src/utils.ts"),
        export_name: "formatDate".to_string(),
        is_type_only: false,
        line: 25,
        col: 0,
        span_start: 300,
        is_re_export: false,
    });
    let lines = build_compact_lines(&results, &root);
    insta::assert_snapshot!("compact_multiple_exports_same_file", lines.join("\n"));
}

// ── Workspace package.json path variant ─────────────────────────

#[test]
fn json_workspace_dep_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_dependencies.push(UnusedDependency {
        package_name: "lodash".to_string(),
        location: DependencyLocation::Dependencies,
        path: root.join("packages/ui/package.json"),
    });
    results.unused_dev_dependencies.push(UnusedDependency {
        package_name: "jest".to_string(),
        location: DependencyLocation::DevDependencies,
        path: root.join("packages/ui/package.json"),
    });
    let value = build_json(&results, &root, Duration::ZERO).expect("JSON build should succeed");
    let json_str = serde_json::to_string_pretty(&value).expect("should serialize");
    insta::assert_snapshot!("json_workspace_deps", redact_version(&json_str));
}

#[test]
fn sarif_workspace_dep_snapshot() {
    let root = PathBuf::from("/project");
    let mut results = AnalysisResults::default();
    results.unused_dependencies.push(UnusedDependency {
        package_name: "lodash".to_string(),
        location: DependencyLocation::Dependencies,
        path: root.join("packages/ui/package.json"),
    });
    let rules = RulesConfig::default();
    let sarif = build_sarif(&results, &root, &rules);
    let json_str = serde_json::to_string_pretty(&sarif).expect("should serialize");
    insta::assert_snapshot!("sarif_workspace_deps", redact_sarif_version(&json_str));
}
