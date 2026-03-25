use std::path::Path;
use std::process::ExitCode;

use fallow_config::{RulesConfig, Severity};
use fallow_core::duplicates::DuplicationReport;
use fallow_core::results::{AnalysisResults, UnusedDependency, UnusedExport, UnusedMember};

use super::relative_uri;
use crate::explain;

/// Intermediate fields extracted from an issue for SARIF result construction.
struct SarifFields {
    rule_id: &'static str,
    level: &'static str,
    message: String,
    uri: String,
    region: Option<(u32, u32)>,
    properties: Option<serde_json::Value>,
}

const fn severity_to_sarif_level(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warn | Severity::Off => "warning",
    }
}

/// Build a single SARIF result object.
///
/// When `region` is `Some((line, col))`, a `region` block with 1-based
/// `startLine` and `startColumn` is included in the physical location.
fn sarif_result(
    rule_id: &str,
    level: &str,
    message: &str,
    uri: &str,
    region: Option<(u32, u32)>,
) -> serde_json::Value {
    let mut physical_location = serde_json::json!({
        "artifactLocation": { "uri": uri }
    });
    if let Some((line, col)) = region {
        physical_location["region"] = serde_json::json!({
            "startLine": line,
            "startColumn": col
        });
    }
    serde_json::json!({
        "ruleId": rule_id,
        "level": level,
        "message": { "text": message },
        "locations": [{ "physicalLocation": physical_location }]
    })
}

/// Append SARIF results for a slice of items using a closure to extract fields.
fn push_sarif_results<T>(
    sarif_results: &mut Vec<serde_json::Value>,
    items: &[T],
    extract: impl Fn(&T) -> SarifFields,
) {
    for item in items {
        let fields = extract(item);
        let mut result = sarif_result(
            fields.rule_id,
            fields.level,
            &fields.message,
            &fields.uri,
            fields.region,
        );
        if let Some(props) = fields.properties {
            result["properties"] = props;
        }
        sarif_results.push(result);
    }
}

/// Build a SARIF rule definition with optional `fullDescription` and `helpUri`
/// sourced from the centralized explain module.
fn sarif_rule(id: &str, fallback_short: &str, level: &str) -> serde_json::Value {
    if let Some(def) = explain::rule_by_id(id) {
        serde_json::json!({
            "id": id,
            "shortDescription": { "text": def.short },
            "fullDescription": { "text": def.full },
            "helpUri": explain::rule_docs_url(def),
            "defaultConfiguration": { "level": level }
        })
    } else {
        serde_json::json!({
            "id": id,
            "shortDescription": { "text": fallback_short },
            "defaultConfiguration": { "level": level }
        })
    }
}

pub fn build_sarif(
    results: &AnalysisResults,
    root: &Path,
    rules: &RulesConfig,
) -> serde_json::Value {
    let mut sarif_results = Vec::new();

    push_sarif_results(&mut sarif_results, &results.unused_files, |file| {
        SarifFields {
            rule_id: "fallow/unused-file",
            level: severity_to_sarif_level(rules.unused_files),
            message: "File is not reachable from any entry point".to_string(),
            uri: relative_uri(&file.path, root),
            region: None,
            properties: None,
        }
    });

    let sarif_export = |export: &UnusedExport,
                        rule_id: &'static str,
                        level: &'static str,
                        kind: &str,
                        re_kind: &str|
     -> SarifFields {
        let label = if export.is_re_export { re_kind } else { kind };
        SarifFields {
            rule_id,
            level,
            message: format!(
                "{} '{}' is never imported by other modules",
                label, export.export_name
            ),
            uri: relative_uri(&export.path, root),
            region: Some((export.line, export.col + 1)),
            properties: if export.is_re_export {
                Some(serde_json::json!({ "is_re_export": true }))
            } else {
                None
            },
        }
    };

    push_sarif_results(&mut sarif_results, &results.unused_exports, |export| {
        sarif_export(
            export,
            "fallow/unused-export",
            severity_to_sarif_level(rules.unused_exports),
            "Export",
            "Re-export",
        )
    });

    push_sarif_results(&mut sarif_results, &results.unused_types, |export| {
        sarif_export(
            export,
            "fallow/unused-type",
            severity_to_sarif_level(rules.unused_types),
            "Type export",
            "Type re-export",
        )
    });

    let sarif_dep = |dep: &UnusedDependency,
                     rule_id: &'static str,
                     level: &'static str,
                     section: &str|
     -> SarifFields {
        SarifFields {
            rule_id,
            level,
            message: format!(
                "Package '{}' is in {} but never imported",
                dep.package_name, section
            ),
            uri: relative_uri(&dep.path, root),
            region: if dep.line > 0 {
                Some((dep.line, 1))
            } else {
                None
            },
            properties: None,
        }
    };

    push_sarif_results(&mut sarif_results, &results.unused_dependencies, |dep| {
        sarif_dep(
            dep,
            "fallow/unused-dependency",
            severity_to_sarif_level(rules.unused_dependencies),
            "dependencies",
        )
    });

    push_sarif_results(
        &mut sarif_results,
        &results.unused_dev_dependencies,
        |dep| {
            sarif_dep(
                dep,
                "fallow/unused-dev-dependency",
                severity_to_sarif_level(rules.unused_dev_dependencies),
                "devDependencies",
            )
        },
    );

    push_sarif_results(
        &mut sarif_results,
        &results.unused_optional_dependencies,
        |dep| {
            sarif_dep(
                dep,
                "fallow/unused-optional-dependency",
                severity_to_sarif_level(rules.unused_optional_dependencies),
                "optionalDependencies",
            )
        },
    );

    push_sarif_results(&mut sarif_results, &results.type_only_dependencies, |dep| {
        SarifFields {
            rule_id: "fallow/type-only-dependency",
            level: severity_to_sarif_level(rules.type_only_dependencies),
            message: format!(
                "Package '{}' is only imported via type-only imports (consider moving to devDependencies)",
                dep.package_name
            ),
            uri: relative_uri(&dep.path, root),
            region: if dep.line > 0 {
                Some((dep.line, 1))
            } else {
                None
            },
            properties: None,
        }
    });

    let sarif_member = |member: &UnusedMember,
                        rule_id: &'static str,
                        level: &'static str,
                        kind: &str|
     -> SarifFields {
        SarifFields {
            rule_id,
            level,
            message: format!(
                "{} member '{}.{}' is never referenced",
                kind, member.parent_name, member.member_name
            ),
            uri: relative_uri(&member.path, root),
            region: Some((member.line, member.col + 1)),
            properties: None,
        }
    };

    push_sarif_results(&mut sarif_results, &results.unused_enum_members, |member| {
        sarif_member(
            member,
            "fallow/unused-enum-member",
            severity_to_sarif_level(rules.unused_enum_members),
            "Enum",
        )
    });

    push_sarif_results(
        &mut sarif_results,
        &results.unused_class_members,
        |member| {
            sarif_member(
                member,
                "fallow/unused-class-member",
                severity_to_sarif_level(rules.unused_class_members),
                "Class",
            )
        },
    );

    push_sarif_results(&mut sarif_results, &results.unresolved_imports, |import| {
        SarifFields {
            rule_id: "fallow/unresolved-import",
            level: severity_to_sarif_level(rules.unresolved_imports),
            message: format!("Import '{}' could not be resolved", import.specifier),
            uri: relative_uri(&import.path, root),
            region: Some((import.line, import.col + 1)),
            properties: None,
        }
    });

    // Unlisted deps: one result per importing file (SARIF points to the import site)
    for dep in &results.unlisted_dependencies {
        for site in &dep.imported_from {
            sarif_results.push(sarif_result(
                "fallow/unlisted-dependency",
                severity_to_sarif_level(rules.unlisted_dependencies),
                &format!(
                    "Package '{}' is imported but not listed in package.json",
                    dep.package_name
                ),
                &relative_uri(&site.path, root),
                Some((site.line, site.col + 1)),
            ));
        }
    }

    // Duplicate exports: one result per location (SARIF 2.1.0 section 3.27.12)
    for dup in &results.duplicate_exports {
        for loc in &dup.locations {
            sarif_results.push(sarif_result(
                "fallow/duplicate-export",
                severity_to_sarif_level(rules.duplicate_exports),
                &format!("Export '{}' appears in multiple modules", dup.export_name),
                &relative_uri(&loc.path, root),
                Some((loc.line, loc.col + 1)),
            ));
        }
    }

    push_sarif_results(
        &mut sarif_results,
        &results.circular_dependencies,
        |cycle| {
            let chain: Vec<String> = cycle.files.iter().map(|p| relative_uri(p, root)).collect();
            let mut display_chain = chain.clone();
            if let Some(first) = chain.first() {
                display_chain.push(first.clone());
            }
            let first_uri = chain.first().map_or_else(String::new, Clone::clone);
            SarifFields {
                rule_id: "fallow/circular-dependency",
                level: severity_to_sarif_level(rules.circular_dependencies),
                message: format!("Circular dependency: {}", display_chain.join(" \u{2192} ")),
                uri: first_uri,
                region: if cycle.line > 0 {
                    Some((cycle.line, cycle.col + 1))
                } else {
                    None
                },
                properties: None,
            }
        },
    );

    let sarif_rules = vec![
        sarif_rule(
            "fallow/unused-file",
            "File is not reachable from any entry point",
            severity_to_sarif_level(rules.unused_files),
        ),
        sarif_rule(
            "fallow/unused-export",
            "Export is never imported",
            severity_to_sarif_level(rules.unused_exports),
        ),
        sarif_rule(
            "fallow/unused-type",
            "Type export is never imported",
            severity_to_sarif_level(rules.unused_types),
        ),
        sarif_rule(
            "fallow/unused-dependency",
            "Dependency listed but never imported",
            severity_to_sarif_level(rules.unused_dependencies),
        ),
        sarif_rule(
            "fallow/unused-dev-dependency",
            "Dev dependency listed but never imported",
            severity_to_sarif_level(rules.unused_dev_dependencies),
        ),
        sarif_rule(
            "fallow/unused-optional-dependency",
            "Optional dependency listed but never imported",
            severity_to_sarif_level(rules.unused_optional_dependencies),
        ),
        sarif_rule(
            "fallow/type-only-dependency",
            "Production dependency only used via type-only imports",
            severity_to_sarif_level(rules.type_only_dependencies),
        ),
        sarif_rule(
            "fallow/unused-enum-member",
            "Enum member is never referenced",
            severity_to_sarif_level(rules.unused_enum_members),
        ),
        sarif_rule(
            "fallow/unused-class-member",
            "Class member is never referenced",
            severity_to_sarif_level(rules.unused_class_members),
        ),
        sarif_rule(
            "fallow/unresolved-import",
            "Import could not be resolved",
            severity_to_sarif_level(rules.unresolved_imports),
        ),
        sarif_rule(
            "fallow/unlisted-dependency",
            "Dependency used but not in package.json",
            severity_to_sarif_level(rules.unlisted_dependencies),
        ),
        sarif_rule(
            "fallow/duplicate-export",
            "Export name appears in multiple modules",
            severity_to_sarif_level(rules.duplicate_exports),
        ),
        sarif_rule(
            "fallow/circular-dependency",
            "Circular dependency chain detected",
            severity_to_sarif_level(rules.circular_dependencies),
        ),
    ];

    serde_json::json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "fallow",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/fallow-rs/fallow",
                    "rules": sarif_rules
                }
            },
            "results": sarif_results
        }]
    })
}

pub(super) fn print_sarif(results: &AnalysisResults, root: &Path, rules: &RulesConfig) -> ExitCode {
    let sarif = build_sarif(results, root, rules);
    match serde_json::to_string_pretty(&sarif) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: failed to serialize SARIF output: {e}");
            ExitCode::from(2)
        }
    }
}

pub(super) fn print_duplication_sarif(report: &DuplicationReport, root: &Path) -> ExitCode {
    let mut sarif_results = Vec::new();

    for (i, group) in report.clone_groups.iter().enumerate() {
        for instance in &group.instances {
            sarif_results.push(sarif_result(
                "fallow/code-duplication",
                "warning",
                &format!(
                    "Code clone group {} ({} lines, {} instances)",
                    i + 1,
                    group.line_count,
                    group.instances.len()
                ),
                &relative_uri(&instance.file, root),
                Some((instance.start_line as u32, (instance.start_col + 1) as u32)),
            ));
        }
    }

    let sarif = serde_json::json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "fallow",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/fallow-rs/fallow",
                    "rules": [sarif_rule("fallow/code-duplication", "Duplicated code block", "warning")]
                }
            },
            "results": sarif_results
        }]
    });

    match serde_json::to_string_pretty(&sarif) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: failed to serialize SARIF output: {e}");
            ExitCode::from(2)
        }
    }
}

// ── Health SARIF output ────────────────────────────────────────────
// Note: file_scores are intentionally omitted from SARIF output.
// SARIF is designed for diagnostic results (issues/findings), not metric tables.
// File health scores are available in JSON, human, compact, and markdown formats.

pub fn build_health_sarif(
    report: &crate::health_types::HealthReport,
    root: &Path,
) -> serde_json::Value {
    use crate::health_types::ExceededThreshold;

    let mut sarif_results = Vec::new();

    for finding in &report.findings {
        let uri = relative_uri(&finding.path, root);
        let (rule_id, message) = match finding.exceeded {
            ExceededThreshold::Cyclomatic => (
                "fallow/high-cyclomatic-complexity",
                format!(
                    "'{}' has cyclomatic complexity {} (threshold: {})",
                    finding.name, finding.cyclomatic, report.summary.max_cyclomatic_threshold,
                ),
            ),
            ExceededThreshold::Cognitive => (
                "fallow/high-cognitive-complexity",
                format!(
                    "'{}' has cognitive complexity {} (threshold: {})",
                    finding.name, finding.cognitive, report.summary.max_cognitive_threshold,
                ),
            ),
            ExceededThreshold::Both => (
                "fallow/high-complexity",
                format!(
                    "'{}' has cyclomatic complexity {} (threshold: {}) and cognitive complexity {} (threshold: {})",
                    finding.name,
                    finding.cyclomatic,
                    report.summary.max_cyclomatic_threshold,
                    finding.cognitive,
                    report.summary.max_cognitive_threshold,
                ),
            ),
        };

        sarif_results.push(sarif_result(
            rule_id,
            "warning",
            &message,
            &uri,
            Some((finding.line, finding.col + 1)),
        ));
    }

    // Refactoring targets as SARIF results (warning level — advisory recommendations)
    for target in &report.targets {
        let uri = relative_uri(&target.path, root);
        let message = format!(
            "[{}] {} (effort: {})",
            target.category.label(),
            target.recommendation,
            target.effort.label(),
        );
        sarif_results.push(sarif_result(
            "fallow/refactoring-target",
            "warning",
            &message,
            &uri,
            None,
        ));
    }

    let health_rules = vec![
        sarif_rule(
            "fallow/high-cyclomatic-complexity",
            "Function has high cyclomatic complexity",
            "warning",
        ),
        sarif_rule(
            "fallow/high-cognitive-complexity",
            "Function has high cognitive complexity",
            "warning",
        ),
        sarif_rule(
            "fallow/high-complexity",
            "Function exceeds both complexity thresholds",
            "warning",
        ),
        sarif_rule(
            "fallow/refactoring-target",
            "File identified as a high-priority refactoring candidate",
            "warning",
        ),
    ];

    serde_json::json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "fallow",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/fallow-rs/fallow",
                    "rules": health_rules
                }
            },
            "results": sarif_results
        }]
    })
}

pub(super) fn print_health_sarif(
    report: &crate::health_types::HealthReport,
    root: &Path,
) -> ExitCode {
    let sarif = build_health_sarif(report, root);
    match serde_json::to_string_pretty(&sarif) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: failed to serialize SARIF output: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fallow_core::extract::MemberKind;
    use fallow_core::results::*;
    use std::path::PathBuf;

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
            line: 5,
        });
        r.unused_dev_dependencies.push(UnusedDependency {
            package_name: "jest".to_string(),
            location: DependencyLocation::DevDependencies,
            path: root.join("package.json"),
            line: 5,
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
            specifier_col: 0,
        });
        r.unlisted_dependencies.push(UnlistedDependency {
            package_name: "chalk".to_string(),
            imported_from: vec![ImportSite {
                path: root.join("src/cli.ts"),
                line: 2,
                col: 0,
            }],
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
        r.type_only_dependencies.push(TypeOnlyDependency {
            package_name: "zod".to_string(),
            path: root.join("package.json"),
            line: 8,
        });
        r.circular_dependencies.push(CircularDependency {
            files: vec![root.join("src/a.ts"), root.join("src/b.ts")],
            length: 2,
            line: 3,
            col: 0,
        });

        r
    }

    #[test]
    fn sarif_has_required_top_level_fields() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let sarif = build_sarif(&results, &root, &RulesConfig::default());

        assert_eq!(
            sarif["$schema"],
            "https://json.schemastore.org/sarif-2.1.0.json"
        );
        assert_eq!(sarif["version"], "2.1.0");
        assert!(sarif["runs"].is_array());
    }

    #[test]
    fn sarif_has_tool_driver_info() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let sarif = build_sarif(&results, &root, &RulesConfig::default());

        let driver = &sarif["runs"][0]["tool"]["driver"];
        assert_eq!(driver["name"], "fallow");
        assert!(driver["version"].is_string());
        assert_eq!(
            driver["informationUri"],
            "https://github.com/fallow-rs/fallow"
        );
    }

    #[test]
    fn sarif_declares_all_rules() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let sarif = build_sarif(&results, &root, &RulesConfig::default());

        let rules = sarif["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .expect("rules should be an array");
        assert_eq!(rules.len(), 13);

        let rule_ids: Vec<&str> = rules.iter().map(|r| r["id"].as_str().unwrap()).collect();
        assert!(rule_ids.contains(&"fallow/unused-file"));
        assert!(rule_ids.contains(&"fallow/unused-export"));
        assert!(rule_ids.contains(&"fallow/unused-type"));
        assert!(rule_ids.contains(&"fallow/unused-dependency"));
        assert!(rule_ids.contains(&"fallow/unused-dev-dependency"));
        assert!(rule_ids.contains(&"fallow/unused-optional-dependency"));
        assert!(rule_ids.contains(&"fallow/type-only-dependency"));
        assert!(rule_ids.contains(&"fallow/unused-enum-member"));
        assert!(rule_ids.contains(&"fallow/unused-class-member"));
        assert!(rule_ids.contains(&"fallow/unresolved-import"));
        assert!(rule_ids.contains(&"fallow/unlisted-dependency"));
        assert!(rule_ids.contains(&"fallow/duplicate-export"));
        assert!(rule_ids.contains(&"fallow/circular-dependency"));
    }

    #[test]
    fn sarif_empty_results_no_results_entries() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let sarif = build_sarif(&results, &root, &RulesConfig::default());

        let sarif_results = sarif["runs"][0]["results"]
            .as_array()
            .expect("results should be an array");
        assert!(sarif_results.is_empty());
    }

    #[test]
    fn sarif_unused_file_result() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/dead.ts"),
        });

        let sarif = build_sarif(&results, &root, &RulesConfig::default());
        let entries = sarif["runs"][0]["results"].as_array().unwrap();
        assert_eq!(entries.len(), 1);

        let entry = &entries[0];
        assert_eq!(entry["ruleId"], "fallow/unused-file");
        // Default severity is "error" per RulesConfig::default()
        assert_eq!(entry["level"], "error");
        assert_eq!(
            entry["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            "src/dead.ts"
        );
    }

    #[test]
    fn sarif_unused_export_includes_region() {
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
        let entry = &sarif["runs"][0]["results"][0];
        assert_eq!(entry["ruleId"], "fallow/unused-export");

        let region = &entry["locations"][0]["physicalLocation"]["region"];
        assert_eq!(region["startLine"], 10);
        // SARIF columns are 1-based, code adds +1 to the 0-based col
        assert_eq!(region["startColumn"], 5);
    }

    #[test]
    fn sarif_unresolved_import_is_error_level() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unresolved_imports.push(UnresolvedImport {
            path: root.join("src/app.ts"),
            specifier: "./missing".to_string(),
            line: 1,
            col: 0,
            specifier_col: 0,
        });

        let sarif = build_sarif(&results, &root, &RulesConfig::default());
        let entry = &sarif["runs"][0]["results"][0];
        assert_eq!(entry["ruleId"], "fallow/unresolved-import");
        assert_eq!(entry["level"], "error");
    }

    #[test]
    fn sarif_unlisted_dependency_points_to_import_site() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unlisted_dependencies.push(UnlistedDependency {
            package_name: "chalk".to_string(),
            imported_from: vec![ImportSite {
                path: root.join("src/cli.ts"),
                line: 3,
                col: 0,
            }],
        });

        let sarif = build_sarif(&results, &root, &RulesConfig::default());
        let entry = &sarif["runs"][0]["results"][0];
        assert_eq!(entry["ruleId"], "fallow/unlisted-dependency");
        assert_eq!(entry["level"], "error");
        assert_eq!(
            entry["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            "src/cli.ts"
        );
        let region = &entry["locations"][0]["physicalLocation"]["region"];
        assert_eq!(region["startLine"], 3);
        assert_eq!(region["startColumn"], 1);
    }

    #[test]
    fn sarif_dependency_issues_point_to_package_json() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_dependencies.push(UnusedDependency {
            package_name: "lodash".to_string(),
            location: DependencyLocation::Dependencies,
            path: root.join("package.json"),
            line: 5,
        });
        results.unused_dev_dependencies.push(UnusedDependency {
            package_name: "jest".to_string(),
            location: DependencyLocation::DevDependencies,
            path: root.join("package.json"),
            line: 5,
        });

        let sarif = build_sarif(&results, &root, &RulesConfig::default());
        let entries = sarif["runs"][0]["results"].as_array().unwrap();
        for entry in entries {
            assert_eq!(
                entry["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
                "package.json"
            );
        }
    }

    #[test]
    fn sarif_duplicate_export_emits_one_result_per_location() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.duplicate_exports.push(DuplicateExport {
            export_name: "Config".to_string(),
            locations: vec![
                DuplicateLocation {
                    path: root.join("src/a.ts"),
                    line: 15,
                    col: 0,
                },
                DuplicateLocation {
                    path: root.join("src/b.ts"),
                    line: 30,
                    col: 0,
                },
            ],
        });

        let sarif = build_sarif(&results, &root, &RulesConfig::default());
        let entries = sarif["runs"][0]["results"].as_array().unwrap();
        // One SARIF result per location, not one per DuplicateExport
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["ruleId"], "fallow/duplicate-export");
        assert_eq!(entries[1]["ruleId"], "fallow/duplicate-export");
        assert_eq!(
            entries[0]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            "src/a.ts"
        );
        assert_eq!(
            entries[1]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            "src/b.ts"
        );
    }

    #[test]
    fn sarif_all_issue_types_produce_results() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let sarif = build_sarif(&results, &root, &RulesConfig::default());

        let entries = sarif["runs"][0]["results"].as_array().unwrap();
        // 12 issues but duplicate_exports has 2 locations => 13 SARIF results
        assert_eq!(entries.len(), 13);

        let rule_ids: Vec<&str> = entries
            .iter()
            .map(|e| e["ruleId"].as_str().unwrap())
            .collect();
        assert!(rule_ids.contains(&"fallow/unused-file"));
        assert!(rule_ids.contains(&"fallow/unused-export"));
        assert!(rule_ids.contains(&"fallow/unused-type"));
        assert!(rule_ids.contains(&"fallow/unused-dependency"));
        assert!(rule_ids.contains(&"fallow/unused-dev-dependency"));
        assert!(rule_ids.contains(&"fallow/type-only-dependency"));
        assert!(rule_ids.contains(&"fallow/unused-enum-member"));
        assert!(rule_ids.contains(&"fallow/unused-class-member"));
        assert!(rule_ids.contains(&"fallow/unresolved-import"));
        assert!(rule_ids.contains(&"fallow/unlisted-dependency"));
        assert!(rule_ids.contains(&"fallow/duplicate-export"));
    }

    #[test]
    fn sarif_serializes_to_valid_json() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let sarif = build_sarif(&results, &root, &RulesConfig::default());

        let json_str = serde_json::to_string_pretty(&sarif).expect("SARIF should serialize");
        let reparsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("SARIF output should be valid JSON");
        assert_eq!(reparsed, sarif);
    }

    #[test]
    fn sarif_file_write_produces_valid_sarif() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let sarif = build_sarif(&results, &root, &RulesConfig::default());
        let json_str = serde_json::to_string_pretty(&sarif).expect("SARIF should serialize");

        let dir = std::env::temp_dir().join("fallow-test-sarif-file");
        let _ = std::fs::create_dir_all(&dir);
        let sarif_path = dir.join("results.sarif");
        std::fs::write(&sarif_path, &json_str).expect("should write SARIF file");

        let contents = std::fs::read_to_string(&sarif_path).expect("should read SARIF file");
        let parsed: serde_json::Value =
            serde_json::from_str(&contents).expect("file should contain valid JSON");

        assert_eq!(parsed["version"], "2.1.0");
        assert_eq!(
            parsed["$schema"],
            "https://json.schemastore.org/sarif-2.1.0.json"
        );
        let sarif_results = parsed["runs"][0]["results"]
            .as_array()
            .expect("results should be an array");
        assert!(!sarif_results.is_empty());

        // Clean up
        let _ = std::fs::remove_file(&sarif_path);
        let _ = std::fs::remove_dir(&dir);
    }

    // ── Health SARIF ──

    #[test]
    fn health_sarif_empty_no_results() {
        let root = PathBuf::from("/project");
        let report = crate::health_types::HealthReport {
            findings: vec![],
            summary: crate::health_types::HealthSummary {
                files_analyzed: 10,
                functions_analyzed: 50,
                functions_above_threshold: 0,
                max_cyclomatic_threshold: 20,
                max_cognitive_threshold: 15,
                files_scored: None,
                average_maintainability: None,
            },
            vital_signs: None,
            file_scores: vec![],
            hotspots: vec![],
            hotspot_summary: None,
            targets: vec![],
        };
        let sarif = build_health_sarif(&report, &root);
        assert_eq!(sarif["version"], "2.1.0");
        let results = sarif["runs"][0]["results"].as_array().unwrap();
        assert!(results.is_empty());
        let rules = sarif["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .unwrap();
        assert_eq!(rules.len(), 4);
    }

    #[test]
    fn health_sarif_cyclomatic_only() {
        let root = PathBuf::from("/project");
        let report = crate::health_types::HealthReport {
            findings: vec![crate::health_types::HealthFinding {
                path: root.join("src/utils.ts"),
                name: "parseExpression".to_string(),
                line: 42,
                col: 0,
                cyclomatic: 25,
                cognitive: 10,
                line_count: 80,
                exceeded: crate::health_types::ExceededThreshold::Cyclomatic,
            }],
            summary: crate::health_types::HealthSummary {
                files_analyzed: 5,
                functions_analyzed: 20,
                functions_above_threshold: 1,
                max_cyclomatic_threshold: 20,
                max_cognitive_threshold: 15,
                files_scored: None,
                average_maintainability: None,
            },
            vital_signs: None,
            file_scores: vec![],
            hotspots: vec![],
            hotspot_summary: None,
            targets: vec![],
        };
        let sarif = build_health_sarif(&report, &root);
        let entry = &sarif["runs"][0]["results"][0];
        assert_eq!(entry["ruleId"], "fallow/high-cyclomatic-complexity");
        assert_eq!(entry["level"], "warning");
        assert!(
            entry["message"]["text"]
                .as_str()
                .unwrap()
                .contains("cyclomatic complexity 25")
        );
        assert_eq!(
            entry["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            "src/utils.ts"
        );
        let region = &entry["locations"][0]["physicalLocation"]["region"];
        assert_eq!(region["startLine"], 42);
        assert_eq!(region["startColumn"], 1);
    }

    #[test]
    fn health_sarif_cognitive_only() {
        let root = PathBuf::from("/project");
        let report = crate::health_types::HealthReport {
            findings: vec![crate::health_types::HealthFinding {
                path: root.join("src/api.ts"),
                name: "handleRequest".to_string(),
                line: 10,
                col: 4,
                cyclomatic: 8,
                cognitive: 20,
                line_count: 40,
                exceeded: crate::health_types::ExceededThreshold::Cognitive,
            }],
            summary: crate::health_types::HealthSummary {
                files_analyzed: 3,
                functions_analyzed: 10,
                functions_above_threshold: 1,
                max_cyclomatic_threshold: 20,
                max_cognitive_threshold: 15,
                files_scored: None,
                average_maintainability: None,
            },
            vital_signs: None,
            file_scores: vec![],
            hotspots: vec![],
            hotspot_summary: None,
            targets: vec![],
        };
        let sarif = build_health_sarif(&report, &root);
        let entry = &sarif["runs"][0]["results"][0];
        assert_eq!(entry["ruleId"], "fallow/high-cognitive-complexity");
        assert!(
            entry["message"]["text"]
                .as_str()
                .unwrap()
                .contains("cognitive complexity 20")
        );
        let region = &entry["locations"][0]["physicalLocation"]["region"];
        assert_eq!(region["startColumn"], 5); // col 4 + 1
    }

    #[test]
    fn health_sarif_both_thresholds() {
        let root = PathBuf::from("/project");
        let report = crate::health_types::HealthReport {
            findings: vec![crate::health_types::HealthFinding {
                path: root.join("src/complex.ts"),
                name: "doEverything".to_string(),
                line: 1,
                col: 0,
                cyclomatic: 30,
                cognitive: 45,
                line_count: 100,
                exceeded: crate::health_types::ExceededThreshold::Both,
            }],
            summary: crate::health_types::HealthSummary {
                files_analyzed: 1,
                functions_analyzed: 1,
                functions_above_threshold: 1,
                max_cyclomatic_threshold: 20,
                max_cognitive_threshold: 15,
                files_scored: None,
                average_maintainability: None,
            },
            vital_signs: None,
            file_scores: vec![],
            hotspots: vec![],
            hotspot_summary: None,
            targets: vec![],
        };
        let sarif = build_health_sarif(&report, &root);
        let entry = &sarif["runs"][0]["results"][0];
        assert_eq!(entry["ruleId"], "fallow/high-complexity");
        let msg = entry["message"]["text"].as_str().unwrap();
        assert!(msg.contains("cyclomatic complexity 30"));
        assert!(msg.contains("cognitive complexity 45"));
    }
}
