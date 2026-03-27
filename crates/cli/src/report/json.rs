use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use fallow_core::duplicates::DuplicationReport;
use fallow_core::results::AnalysisResults;

use super::emit_json;
use crate::explain;

pub(super) fn print_json(
    results: &AnalysisResults,
    root: &Path,
    elapsed: Duration,
    explain: bool,
) -> ExitCode {
    match build_json(results, root, elapsed) {
        Ok(mut output) => {
            if explain {
                insert_meta(&mut output, explain::check_meta());
            }
            emit_json(&output, "JSON")
        }
        Err(e) => {
            eprintln!("Error: failed to serialize results: {e}");
            ExitCode::from(2)
        }
    }
}

/// JSON output schema version as an integer (independent of tool version).
///
/// Bump this when the structure of the JSON output changes in a
/// backwards-incompatible way (removing/renaming fields, changing types).
/// Adding new fields is always backwards-compatible and does not require a bump.
const SCHEMA_VERSION: u32 = 3;

/// Build a JSON envelope with standard metadata fields at the top.
///
/// Creates a JSON object with `schema_version`, `version`, and `elapsed_ms`,
/// then merges all fields from `report_value` into the envelope.
/// Fields from `report_value` appear after the metadata header.
fn build_json_envelope(report_value: serde_json::Value, elapsed: Duration) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert(
        "schema_version".to_string(),
        serde_json::json!(SCHEMA_VERSION),
    );
    map.insert(
        "version".to_string(),
        serde_json::json!(env!("CARGO_PKG_VERSION")),
    );
    map.insert(
        "elapsed_ms".to_string(),
        serde_json::json!(elapsed.as_millis()),
    );
    if let serde_json::Value::Object(report_map) = report_value {
        for (key, value) in report_map {
            map.insert(key, value);
        }
    }
    serde_json::Value::Object(map)
}

/// Build the JSON output value for analysis results.
///
/// Metadata fields (`schema_version`, `version`, `elapsed_ms`, `total_issues`)
/// appear first in the output for readability. Paths are made relative to `root`.
pub fn build_json(
    results: &AnalysisResults,
    root: &Path,
    elapsed: Duration,
) -> Result<serde_json::Value, serde_json::Error> {
    let results_value = serde_json::to_value(results)?;

    let mut map = serde_json::Map::new();
    map.insert(
        "schema_version".to_string(),
        serde_json::json!(SCHEMA_VERSION),
    );
    map.insert(
        "version".to_string(),
        serde_json::json!(env!("CARGO_PKG_VERSION")),
    );
    map.insert(
        "elapsed_ms".to_string(),
        serde_json::json!(elapsed.as_millis()),
    );
    map.insert(
        "total_issues".to_string(),
        serde_json::json!(results.total_issues()),
    );

    if let serde_json::Value::Object(results_map) = results_value {
        for (key, value) in results_map {
            map.insert(key, value);
        }
    }

    let mut output = serde_json::Value::Object(map);
    let root_prefix = format!("{}/", root.display());
    strip_root_prefix(&mut output, &root_prefix);
    Ok(output)
}

/// Recursively strip the root prefix from all string values in the JSON tree.
///
/// This converts absolute paths (e.g., `/home/runner/work/repo/repo/src/utils.ts`)
/// to relative paths (`src/utils.ts`) for all output fields.
fn strip_root_prefix(value: &mut serde_json::Value, prefix: &str) {
    match value {
        serde_json::Value::String(s) => {
            if let Some(rest) = s.strip_prefix(prefix) {
                *s = rest.to_string();
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                strip_root_prefix(item, prefix);
            }
        }
        serde_json::Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                strip_root_prefix(v, prefix);
            }
        }
        _ => {}
    }
}

/// Insert a `_meta` key into a JSON object value.
fn insert_meta(output: &mut serde_json::Value, meta: serde_json::Value) {
    if let serde_json::Value::Object(map) = output {
        map.insert("_meta".to_string(), meta);
    }
}

pub(super) fn print_health_json(
    report: &crate::health_types::HealthReport,
    root: &Path,
    elapsed: Duration,
    explain: bool,
) -> ExitCode {
    let report_value = match serde_json::to_value(report) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: failed to serialize health report: {e}");
            return ExitCode::from(2);
        }
    };

    let mut output = build_json_envelope(report_value, elapsed);
    let root_prefix = format!("{}/", root.display());
    strip_root_prefix(&mut output, &root_prefix);

    if explain {
        insert_meta(&mut output, explain::health_meta());
    }

    emit_json(&output, "JSON")
}

pub(super) fn print_duplication_json(
    report: &DuplicationReport,
    elapsed: Duration,
    explain: bool,
) -> ExitCode {
    let report_value = match serde_json::to_value(report) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: failed to serialize duplication report: {e}");
            return ExitCode::from(2);
        }
    };

    let mut output = build_json_envelope(report_value, elapsed);

    if explain {
        insert_meta(&mut output, explain::dupes_meta());
    }

    emit_json(&output, "JSON")
}

pub(super) fn print_trace_json<T: serde::Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("Error: failed to serialize trace output: {e}");
            #[expect(clippy::exit)]
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::test_helpers::sample_results;
    use fallow_core::extract::MemberKind;
    use fallow_core::results::*;
    use std::path::PathBuf;
    use std::time::Duration;

    #[test]
    fn json_output_has_metadata_fields() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let elapsed = Duration::from_millis(123);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        assert_eq!(output["schema_version"], 3);
        assert!(output["version"].is_string());
        assert_eq!(output["elapsed_ms"], 123);
        assert_eq!(output["total_issues"], 0);
    }

    #[test]
    fn json_output_includes_issue_arrays() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let elapsed = Duration::from_millis(50);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        assert_eq!(output["unused_files"].as_array().unwrap().len(), 1);
        assert_eq!(output["unused_exports"].as_array().unwrap().len(), 1);
        assert_eq!(output["unused_types"].as_array().unwrap().len(), 1);
        assert_eq!(output["unused_dependencies"].as_array().unwrap().len(), 1);
        assert_eq!(
            output["unused_dev_dependencies"].as_array().unwrap().len(),
            1
        );
        assert_eq!(output["unused_enum_members"].as_array().unwrap().len(), 1);
        assert_eq!(output["unused_class_members"].as_array().unwrap().len(), 1);
        assert_eq!(output["unresolved_imports"].as_array().unwrap().len(), 1);
        assert_eq!(output["unlisted_dependencies"].as_array().unwrap().len(), 1);
        assert_eq!(output["duplicate_exports"].as_array().unwrap().len(), 1);
        assert_eq!(
            output["type_only_dependencies"].as_array().unwrap().len(),
            1
        );
        assert_eq!(output["circular_dependencies"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn json_metadata_fields_appear_first() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");
        let keys: Vec<&String> = output.as_object().unwrap().keys().collect();
        assert_eq!(keys[0], "schema_version");
        assert_eq!(keys[1], "version");
        assert_eq!(keys[2], "elapsed_ms");
        assert_eq!(keys[3], "total_issues");
    }

    #[test]
    fn json_total_issues_matches_results() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let total = results.total_issues();
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        assert_eq!(output["total_issues"], total);
    }

    #[test]
    fn json_unused_export_contains_expected_fields() {
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
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let export = &output["unused_exports"][0];
        assert_eq!(export["export_name"], "helperFn");
        assert_eq!(export["line"], 10);
        assert_eq!(export["col"], 4);
        assert_eq!(export["is_type_only"], false);
        assert_eq!(export["span_start"], 120);
        assert_eq!(export["is_re_export"], false);
    }

    #[test]
    fn json_serializes_to_valid_json() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let elapsed = Duration::from_millis(42);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let json_str = serde_json::to_string_pretty(&output).expect("should stringify");
        let reparsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("JSON output should be valid JSON");
        assert_eq!(reparsed, output);
    }

    // ── Empty results ───────────────────────────────────────────────

    #[test]
    fn json_empty_results_produce_valid_structure() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        assert_eq!(output["total_issues"], 0);
        assert_eq!(output["unused_files"].as_array().unwrap().len(), 0);
        assert_eq!(output["unused_exports"].as_array().unwrap().len(), 0);
        assert_eq!(output["unused_types"].as_array().unwrap().len(), 0);
        assert_eq!(output["unused_dependencies"].as_array().unwrap().len(), 0);
        assert_eq!(
            output["unused_dev_dependencies"].as_array().unwrap().len(),
            0
        );
        assert_eq!(output["unused_enum_members"].as_array().unwrap().len(), 0);
        assert_eq!(output["unused_class_members"].as_array().unwrap().len(), 0);
        assert_eq!(output["unresolved_imports"].as_array().unwrap().len(), 0);
        assert_eq!(output["unlisted_dependencies"].as_array().unwrap().len(), 0);
        assert_eq!(output["duplicate_exports"].as_array().unwrap().len(), 0);
        assert_eq!(
            output["type_only_dependencies"].as_array().unwrap().len(),
            0
        );
        assert_eq!(output["circular_dependencies"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn json_empty_results_round_trips_through_string() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let json_str = serde_json::to_string(&output).expect("should stringify");
        let reparsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("should parse back");
        assert_eq!(reparsed["total_issues"], 0);
    }

    // ── Path stripping ──────────────────────────────────────────────

    #[test]
    fn json_paths_are_relative_to_root() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/deep/nested/file.ts"),
        });
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let path = output["unused_files"][0]["path"].as_str().unwrap();
        assert_eq!(path, "src/deep/nested/file.ts");
        assert!(!path.starts_with("/project"));
    }

    #[test]
    fn json_strips_root_from_nested_locations() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unlisted_dependencies.push(UnlistedDependency {
            package_name: "chalk".to_string(),
            imported_from: vec![ImportSite {
                path: root.join("src/cli.ts"),
                line: 2,
                col: 0,
            }],
        });
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let site_path = output["unlisted_dependencies"][0]["imported_from"][0]["path"]
            .as_str()
            .unwrap();
        assert_eq!(site_path, "src/cli.ts");
    }

    #[test]
    fn json_strips_root_from_duplicate_export_locations() {
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
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let loc0 = output["duplicate_exports"][0]["locations"][0]["path"]
            .as_str()
            .unwrap();
        let loc1 = output["duplicate_exports"][0]["locations"][1]["path"]
            .as_str()
            .unwrap();
        assert_eq!(loc0, "src/config.ts");
        assert_eq!(loc1, "src/types.ts");
    }

    #[test]
    fn json_strips_root_from_circular_dependency_files() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.circular_dependencies.push(CircularDependency {
            files: vec![root.join("src/a.ts"), root.join("src/b.ts")],
            length: 2,
            line: 1,
            col: 0,
        });
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let files = output["circular_dependencies"][0]["files"]
            .as_array()
            .unwrap();
        assert_eq!(files[0].as_str().unwrap(), "src/a.ts");
        assert_eq!(files[1].as_str().unwrap(), "src/b.ts");
    }

    #[test]
    fn json_path_outside_root_not_stripped() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: PathBuf::from("/other/project/src/file.ts"),
        });
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let path = output["unused_files"][0]["path"].as_str().unwrap();
        assert!(path.contains("/other/project/"));
    }

    // ── Individual issue type field verification ────────────────────

    #[test]
    fn json_unused_file_contains_path() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/orphan.ts"),
        });
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let file = &output["unused_files"][0];
        assert_eq!(file["path"], "src/orphan.ts");
    }

    #[test]
    fn json_unused_type_contains_expected_fields() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_types.push(UnusedExport {
            path: root.join("src/types.ts"),
            export_name: "OldInterface".to_string(),
            is_type_only: true,
            line: 20,
            col: 0,
            span_start: 300,
            is_re_export: false,
        });
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let typ = &output["unused_types"][0];
        assert_eq!(typ["export_name"], "OldInterface");
        assert_eq!(typ["is_type_only"], true);
        assert_eq!(typ["line"], 20);
        assert_eq!(typ["path"], "src/types.ts");
    }

    #[test]
    fn json_unused_dependency_contains_expected_fields() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_dependencies.push(UnusedDependency {
            package_name: "axios".to_string(),
            location: DependencyLocation::Dependencies,
            path: root.join("package.json"),
            line: 10,
        });
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let dep = &output["unused_dependencies"][0];
        assert_eq!(dep["package_name"], "axios");
        assert_eq!(dep["line"], 10);
    }

    #[test]
    fn json_unused_dev_dependency_contains_expected_fields() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_dev_dependencies.push(UnusedDependency {
            package_name: "vitest".to_string(),
            location: DependencyLocation::DevDependencies,
            path: root.join("package.json"),
            line: 15,
        });
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let dep = &output["unused_dev_dependencies"][0];
        assert_eq!(dep["package_name"], "vitest");
    }

    #[test]
    fn json_unused_optional_dependency_contains_expected_fields() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_optional_dependencies.push(UnusedDependency {
            package_name: "fsevents".to_string(),
            location: DependencyLocation::OptionalDependencies,
            path: root.join("package.json"),
            line: 12,
        });
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let dep = &output["unused_optional_dependencies"][0];
        assert_eq!(dep["package_name"], "fsevents");
        assert_eq!(output["total_issues"], 1);
    }

    #[test]
    fn json_unused_enum_member_contains_expected_fields() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_enum_members.push(UnusedMember {
            path: root.join("src/enums.ts"),
            parent_name: "Color".to_string(),
            member_name: "Purple".to_string(),
            kind: MemberKind::EnumMember,
            line: 5,
            col: 2,
        });
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let member = &output["unused_enum_members"][0];
        assert_eq!(member["parent_name"], "Color");
        assert_eq!(member["member_name"], "Purple");
        assert_eq!(member["line"], 5);
        assert_eq!(member["path"], "src/enums.ts");
    }

    #[test]
    fn json_unused_class_member_contains_expected_fields() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_class_members.push(UnusedMember {
            path: root.join("src/api.ts"),
            parent_name: "ApiClient".to_string(),
            member_name: "deprecatedFetch".to_string(),
            kind: MemberKind::ClassMethod,
            line: 100,
            col: 4,
        });
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let member = &output["unused_class_members"][0];
        assert_eq!(member["parent_name"], "ApiClient");
        assert_eq!(member["member_name"], "deprecatedFetch");
        assert_eq!(member["line"], 100);
    }

    #[test]
    fn json_unresolved_import_contains_expected_fields() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unresolved_imports.push(UnresolvedImport {
            path: root.join("src/app.ts"),
            specifier: "@acme/missing-pkg".to_string(),
            line: 7,
            col: 0,
            specifier_col: 0,
        });
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let import = &output["unresolved_imports"][0];
        assert_eq!(import["specifier"], "@acme/missing-pkg");
        assert_eq!(import["line"], 7);
        assert_eq!(import["path"], "src/app.ts");
    }

    #[test]
    fn json_unlisted_dependency_contains_import_sites() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unlisted_dependencies.push(UnlistedDependency {
            package_name: "dotenv".to_string(),
            imported_from: vec![
                ImportSite {
                    path: root.join("src/config.ts"),
                    line: 1,
                    col: 0,
                },
                ImportSite {
                    path: root.join("src/server.ts"),
                    line: 3,
                    col: 0,
                },
            ],
        });
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let dep = &output["unlisted_dependencies"][0];
        assert_eq!(dep["package_name"], "dotenv");
        let sites = dep["imported_from"].as_array().unwrap();
        assert_eq!(sites.len(), 2);
        assert_eq!(sites[0]["path"], "src/config.ts");
        assert_eq!(sites[1]["path"], "src/server.ts");
    }

    #[test]
    fn json_duplicate_export_contains_locations() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.duplicate_exports.push(DuplicateExport {
            export_name: "Button".to_string(),
            locations: vec![
                DuplicateLocation {
                    path: root.join("src/ui.ts"),
                    line: 10,
                    col: 0,
                },
                DuplicateLocation {
                    path: root.join("src/components.ts"),
                    line: 25,
                    col: 0,
                },
            ],
        });
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let dup = &output["duplicate_exports"][0];
        assert_eq!(dup["export_name"], "Button");
        let locs = dup["locations"].as_array().unwrap();
        assert_eq!(locs.len(), 2);
        assert_eq!(locs[0]["line"], 10);
        assert_eq!(locs[1]["line"], 25);
    }

    #[test]
    fn json_type_only_dependency_contains_expected_fields() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.type_only_dependencies.push(TypeOnlyDependency {
            package_name: "zod".to_string(),
            path: root.join("package.json"),
            line: 8,
        });
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let dep = &output["type_only_dependencies"][0];
        assert_eq!(dep["package_name"], "zod");
        assert_eq!(dep["line"], 8);
    }

    #[test]
    fn json_circular_dependency_contains_expected_fields() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.circular_dependencies.push(CircularDependency {
            files: vec![
                root.join("src/a.ts"),
                root.join("src/b.ts"),
                root.join("src/c.ts"),
            ],
            length: 3,
            line: 5,
            col: 0,
        });
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let cycle = &output["circular_dependencies"][0];
        assert_eq!(cycle["length"], 3);
        assert_eq!(cycle["line"], 5);
        let files = cycle["files"].as_array().unwrap();
        assert_eq!(files.len(), 3);
    }

    // ── Re-export tagging ───────────────────────────────────────────

    #[test]
    fn json_re_export_flagged_correctly() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_exports.push(UnusedExport {
            path: root.join("src/index.ts"),
            export_name: "reExported".to_string(),
            is_type_only: false,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: true,
        });
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        assert_eq!(output["unused_exports"][0]["is_re_export"], true);
    }

    // ── Schema version stability ────────────────────────────────────

    #[test]
    fn json_schema_version_is_3() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        assert_eq!(output["schema_version"], SCHEMA_VERSION);
        assert_eq!(output["schema_version"], 3);
    }

    // ── Version string ──────────────────────────────────────────────

    #[test]
    fn json_version_matches_cargo_pkg_version() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        assert_eq!(output["version"], env!("CARGO_PKG_VERSION"));
    }

    // ── Elapsed time encoding ───────────────────────────────────────

    #[test]
    fn json_elapsed_ms_zero_duration() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let output = build_json(&results, &root, Duration::ZERO).expect("should serialize");

        assert_eq!(output["elapsed_ms"], 0);
    }

    #[test]
    fn json_elapsed_ms_large_duration() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let elapsed = Duration::from_secs(120);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        assert_eq!(output["elapsed_ms"], 120_000);
    }

    #[test]
    fn json_elapsed_ms_sub_millisecond_truncated() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        // 500 microseconds = 0 milliseconds (truncated)
        let elapsed = Duration::from_micros(500);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        assert_eq!(output["elapsed_ms"], 0);
    }

    // ── Multiple issues of same type ────────────────────────────────

    #[test]
    fn json_multiple_unused_files() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/a.ts"),
        });
        results.unused_files.push(UnusedFile {
            path: root.join("src/b.ts"),
        });
        results.unused_files.push(UnusedFile {
            path: root.join("src/c.ts"),
        });
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        assert_eq!(output["unused_files"].as_array().unwrap().len(), 3);
        assert_eq!(output["total_issues"], 3);
    }

    // ── strip_root_prefix unit tests ────────────────────────────────

    #[test]
    fn strip_root_prefix_on_string_value() {
        let mut value = serde_json::json!("/project/src/file.ts");
        strip_root_prefix(&mut value, "/project/");
        assert_eq!(value, "src/file.ts");
    }

    #[test]
    fn strip_root_prefix_leaves_non_matching_string() {
        let mut value = serde_json::json!("/other/src/file.ts");
        strip_root_prefix(&mut value, "/project/");
        assert_eq!(value, "/other/src/file.ts");
    }

    #[test]
    fn strip_root_prefix_recurses_into_arrays() {
        let mut value = serde_json::json!(["/project/a.ts", "/project/b.ts", "/other/c.ts"]);
        strip_root_prefix(&mut value, "/project/");
        assert_eq!(value[0], "a.ts");
        assert_eq!(value[1], "b.ts");
        assert_eq!(value[2], "/other/c.ts");
    }

    #[test]
    fn strip_root_prefix_recurses_into_nested_objects() {
        let mut value = serde_json::json!({
            "outer": {
                "path": "/project/src/nested.ts"
            }
        });
        strip_root_prefix(&mut value, "/project/");
        assert_eq!(value["outer"]["path"], "src/nested.ts");
    }

    #[test]
    fn strip_root_prefix_leaves_numbers_and_booleans() {
        let mut value = serde_json::json!({
            "line": 42,
            "is_type_only": false,
            "path": "/project/src/file.ts"
        });
        strip_root_prefix(&mut value, "/project/");
        assert_eq!(value["line"], 42);
        assert_eq!(value["is_type_only"], false);
        assert_eq!(value["path"], "src/file.ts");
    }

    #[test]
    fn strip_root_prefix_handles_empty_string_after_strip() {
        // Edge case: the string IS the prefix (without trailing content).
        // This shouldn't happen in practice but should not panic.
        let mut value = serde_json::json!("/project/");
        strip_root_prefix(&mut value, "/project/");
        assert_eq!(value, "");
    }

    #[test]
    fn strip_root_prefix_deeply_nested_array_of_objects() {
        let mut value = serde_json::json!({
            "groups": [{
                "instances": [{
                    "file": "/project/src/a.ts"
                }, {
                    "file": "/project/src/b.ts"
                }]
            }]
        });
        strip_root_prefix(&mut value, "/project/");
        assert_eq!(value["groups"][0]["instances"][0]["file"], "src/a.ts");
        assert_eq!(value["groups"][0]["instances"][1]["file"], "src/b.ts");
    }

    // ── Full sample results round-trip ──────────────────────────────

    #[test]
    fn json_full_sample_results_total_issues_correct() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let elapsed = Duration::from_millis(100);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        // sample_results adds one of each issue type (12 total).
        // unused_files + unused_exports + unused_types + unused_dependencies
        // + unused_dev_dependencies + unused_enum_members + unused_class_members
        // + unresolved_imports + unlisted_dependencies + duplicate_exports
        // + type_only_dependencies + circular_dependencies
        assert_eq!(output["total_issues"], results.total_issues());
    }

    #[test]
    fn json_full_sample_no_absolute_paths_in_output() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let json_str = serde_json::to_string(&output).expect("should stringify");
        // The root prefix should be stripped from all paths.
        assert!(!json_str.contains("/project/src/"));
        assert!(!json_str.contains("/project/package.json"));
    }

    // ── JSON output is deterministic ────────────────────────────────

    #[test]
    fn json_output_is_deterministic() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let elapsed = Duration::from_millis(50);

        let output1 = build_json(&results, &root, elapsed).expect("first build");
        let output2 = build_json(&results, &root, elapsed).expect("second build");

        assert_eq!(output1, output2);
    }

    // ── Metadata not overwritten by results fields ──────────────────

    #[test]
    fn json_results_fields_do_not_shadow_metadata() {
        // Ensure that serialized results don't contain keys like "schema_version"
        // that could overwrite the metadata fields we insert first.
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let elapsed = Duration::from_millis(99);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        // Metadata should reflect our explicit values, not anything from AnalysisResults.
        assert_eq!(output["schema_version"], 3);
        assert_eq!(output["elapsed_ms"], 99);
    }

    // ── All 13 issue type arrays present ────────────────────────────

    #[test]
    fn json_all_issue_type_arrays_present_in_empty_results() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let expected_arrays = [
            "unused_files",
            "unused_exports",
            "unused_types",
            "unused_dependencies",
            "unused_dev_dependencies",
            "unused_optional_dependencies",
            "unused_enum_members",
            "unused_class_members",
            "unresolved_imports",
            "unlisted_dependencies",
            "duplicate_exports",
            "type_only_dependencies",
            "circular_dependencies",
        ];
        for key in &expected_arrays {
            assert!(
                output[key].is_array(),
                "expected '{key}' to be an array in JSON output"
            );
        }
    }

    // ── insert_meta ─────────────────────────────────────────────────

    #[test]
    fn insert_meta_adds_key_to_object() {
        let mut output = serde_json::json!({ "foo": 1 });
        let meta = serde_json::json!({ "docs": "https://example.com" });
        insert_meta(&mut output, meta.clone());
        assert_eq!(output["_meta"], meta);
    }

    #[test]
    fn insert_meta_noop_on_non_object() {
        let mut output = serde_json::json!([1, 2, 3]);
        let meta = serde_json::json!({ "docs": "https://example.com" });
        insert_meta(&mut output, meta);
        // Should not panic or add anything
        assert!(output.is_array());
    }

    #[test]
    fn insert_meta_overwrites_existing_meta() {
        let mut output = serde_json::json!({ "_meta": "old" });
        let meta = serde_json::json!({ "new": true });
        insert_meta(&mut output, meta.clone());
        assert_eq!(output["_meta"], meta);
    }

    // ── build_json_envelope ─────────────────────────────────────────

    #[test]
    fn build_json_envelope_has_metadata_fields() {
        let report = serde_json::json!({ "findings": [] });
        let elapsed = Duration::from_millis(42);
        let output = build_json_envelope(report, elapsed);

        assert_eq!(output["schema_version"], 3);
        assert!(output["version"].is_string());
        assert_eq!(output["elapsed_ms"], 42);
        assert!(output["findings"].is_array());
    }

    #[test]
    fn build_json_envelope_metadata_appears_first() {
        let report = serde_json::json!({ "data": "value" });
        let output = build_json_envelope(report, Duration::from_millis(10));

        let keys: Vec<&String> = output.as_object().unwrap().keys().collect();
        assert_eq!(keys[0], "schema_version");
        assert_eq!(keys[1], "version");
        assert_eq!(keys[2], "elapsed_ms");
    }

    #[test]
    fn build_json_envelope_non_object_report() {
        // If report_value is not an Object, only metadata fields appear
        let report = serde_json::json!("not an object");
        let output = build_json_envelope(report, Duration::from_millis(0));

        let obj = output.as_object().unwrap();
        assert_eq!(obj.len(), 3);
        assert!(obj.contains_key("schema_version"));
        assert!(obj.contains_key("version"));
        assert!(obj.contains_key("elapsed_ms"));
    }

    // ── strip_root_prefix with null value ──

    #[test]
    fn strip_root_prefix_null_unchanged() {
        let mut value = serde_json::Value::Null;
        strip_root_prefix(&mut value, "/project/");
        assert!(value.is_null());
    }

    // ── strip_root_prefix with empty string ──

    #[test]
    fn strip_root_prefix_empty_string() {
        let mut value = serde_json::json!("");
        strip_root_prefix(&mut value, "/project/");
        assert_eq!(value, "");
    }

    // ── strip_root_prefix on mixed nested structure ──

    #[test]
    fn strip_root_prefix_mixed_types() {
        let mut value = serde_json::json!({
            "path": "/project/src/file.ts",
            "line": 42,
            "flag": true,
            "nested": {
                "items": ["/project/a.ts", 99, null, "/project/b.ts"],
                "deep": { "path": "/project/c.ts" }
            }
        });
        strip_root_prefix(&mut value, "/project/");
        assert_eq!(value["path"], "src/file.ts");
        assert_eq!(value["line"], 42);
        assert_eq!(value["flag"], true);
        assert_eq!(value["nested"]["items"][0], "a.ts");
        assert_eq!(value["nested"]["items"][1], 99);
        assert!(value["nested"]["items"][2].is_null());
        assert_eq!(value["nested"]["items"][3], "b.ts");
        assert_eq!(value["nested"]["deep"]["path"], "c.ts");
    }

    // ── JSON with explain meta for check ──

    #[test]
    fn json_check_meta_integrates_correctly() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let elapsed = Duration::from_millis(0);
        let mut output = build_json(&results, &root, elapsed).expect("should serialize");
        insert_meta(&mut output, crate::explain::check_meta());

        assert!(output["_meta"]["docs"].is_string());
        assert!(output["_meta"]["rules"].is_object());
    }

    // ── JSON unused member kind serialization ──

    #[test]
    fn json_unused_member_kind_serialized() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_enum_members.push(UnusedMember {
            path: root.join("src/enums.ts"),
            parent_name: "Color".to_string(),
            member_name: "Red".to_string(),
            kind: MemberKind::EnumMember,
            line: 3,
            col: 2,
        });
        results.unused_class_members.push(UnusedMember {
            path: root.join("src/class.ts"),
            parent_name: "Foo".to_string(),
            member_name: "bar".to_string(),
            kind: MemberKind::ClassMethod,
            line: 10,
            col: 4,
        });

        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, &root, elapsed).expect("should serialize");

        let enum_member = &output["unused_enum_members"][0];
        assert!(enum_member["kind"].is_string());
        let class_member = &output["unused_class_members"][0];
        assert!(class_member["kind"].is_string());
    }
}
