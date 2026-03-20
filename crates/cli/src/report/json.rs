use std::process::ExitCode;
use std::time::Duration;

use fallow_core::duplicates::DuplicationReport;
use fallow_core::results::AnalysisResults;

pub(super) fn print_json(results: &AnalysisResults, elapsed: Duration) -> ExitCode {
    match build_json(results, elapsed) {
        Ok(output) => match serde_json::to_string_pretty(&output) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("Error: failed to serialize JSON output: {e}");
                ExitCode::from(2)
            }
        },
        Err(e) => {
            eprintln!("Error: failed to serialize results: {e}");
            ExitCode::from(2)
        }
    }
}

/// JSON output schema version (independent of tool version).
///
/// Bump this when the structure of the JSON output changes:
/// - Major: removing or renaming fields, changing types
/// - Minor: adding new fields or issue types
/// - Patch: documentation-only changes
const SCHEMA_VERSION: &str = "1.0.0";

/// Build the JSON output value for analysis results.
pub fn build_json(
    results: &AnalysisResults,
    elapsed: Duration,
) -> Result<serde_json::Value, serde_json::Error> {
    let mut output = serde_json::to_value(results)?;
    if let serde_json::Value::Object(ref mut map) = output {
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
    }
    Ok(output)
}

pub(super) fn print_duplication_json(report: &DuplicationReport, elapsed: Duration) -> ExitCode {
    let mut output = match serde_json::to_value(report) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: failed to serialize duplication report: {e}");
            return ExitCode::from(2);
        }
    };

    if let serde_json::Value::Object(ref mut map) = output {
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
    }

    match serde_json::to_string_pretty(&output) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: failed to serialize JSON output: {e}");
            ExitCode::from(2)
        }
    }
}

pub(super) fn print_trace_json<T: serde::Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("Error: failed to serialize trace output: {e}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fallow_core::extract::MemberKind;
    use fallow_core::results::*;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    /// Helper: build an `AnalysisResults` populated with one issue of every type.
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
            locations: vec![root.join("src/config.ts"), root.join("src/types.ts")],
        });

        r
    }

    #[test]
    fn json_output_has_metadata_fields() {
        let results = AnalysisResults::default();
        let elapsed = Duration::from_millis(123);
        let output = build_json(&results, elapsed).expect("should serialize");

        assert_eq!(output["schema_version"], "1.0.0");
        assert!(output["version"].is_string());
        assert_eq!(output["elapsed_ms"], 123);
        assert_eq!(output["total_issues"], 0);
    }

    #[test]
    fn json_output_includes_issue_arrays() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let elapsed = Duration::from_millis(50);
        let output = build_json(&results, elapsed).expect("should serialize");

        assert!(output["unused_files"].is_array());
        assert!(output["unused_exports"].is_array());
        assert!(output["unused_types"].is_array());
        assert!(output["unused_dependencies"].is_array());
        assert!(output["unused_dev_dependencies"].is_array());
        assert!(output["unused_enum_members"].is_array());
        assert!(output["unused_class_members"].is_array());
        assert!(output["unresolved_imports"].is_array());
        assert!(output["unlisted_dependencies"].is_array());
        assert!(output["duplicate_exports"].is_array());
    }

    #[test]
    fn json_total_issues_matches_results() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let total = results.total_issues();
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, elapsed).expect("should serialize");

        assert_eq!(output["total_issues"], total);
    }

    #[test]
    fn json_unused_export_contains_expected_fields() {
        let mut results = AnalysisResults::default();
        results.unused_exports.push(UnusedExport {
            path: PathBuf::from("/project/src/utils.ts"),
            export_name: "helperFn".to_string(),
            is_type_only: false,
            line: 10,
            col: 4,
            span_start: 120,
            is_re_export: false,
        });
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, elapsed).expect("should serialize");

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
        let output = build_json(&results, elapsed).expect("should serialize");

        let json_str = serde_json::to_string_pretty(&output).expect("should stringify");
        let reparsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("JSON output should be valid JSON");
        assert_eq!(reparsed, output);
    }
}
