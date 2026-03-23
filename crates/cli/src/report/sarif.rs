use std::path::Path;
use std::process::ExitCode;

use fallow_config::{RulesConfig, Severity};
use fallow_core::duplicates::DuplicationReport;
use fallow_core::results::{AnalysisResults, UnusedDependency, UnusedExport, UnusedMember};

use super::relative_uri;

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
            region: None,
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
            region: None,
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

    push_sarif_results(&mut sarif_results, &results.unlisted_dependencies, |dep| {
        SarifFields {
            rule_id: "fallow/unlisted-dependency",
            level: severity_to_sarif_level(rules.unlisted_dependencies),
            message: format!(
                "Package '{}' is imported but not listed in package.json",
                dep.package_name
            ),
            uri: "package.json".to_string(),
            region: None,
            properties: None,
        }
    });

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
                region: None,
                properties: None,
            }
        },
    );

    serde_json::json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "fallow",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/fallow-rs/fallow",
                    "rules": [
                        {
                            "id": "fallow/unused-file",
                            "shortDescription": { "text": "File is not reachable from any entry point" },
                            "defaultConfiguration": { "level": severity_to_sarif_level(rules.unused_files) }
                        },
                        {
                            "id": "fallow/unused-export",
                            "shortDescription": { "text": "Export is never imported" },
                            "defaultConfiguration": { "level": severity_to_sarif_level(rules.unused_exports) }
                        },
                        {
                            "id": "fallow/unused-type",
                            "shortDescription": { "text": "Type export is never imported" },
                            "defaultConfiguration": { "level": severity_to_sarif_level(rules.unused_types) }
                        },
                        {
                            "id": "fallow/unused-dependency",
                            "shortDescription": { "text": "Dependency listed but never imported" },
                            "defaultConfiguration": { "level": severity_to_sarif_level(rules.unused_dependencies) }
                        },
                        {
                            "id": "fallow/unused-dev-dependency",
                            "shortDescription": { "text": "Dev dependency listed but never imported" },
                            "defaultConfiguration": { "level": severity_to_sarif_level(rules.unused_dev_dependencies) }
                        },
                        {
                            "id": "fallow/unused-optional-dependency",
                            "shortDescription": { "text": "Optional dependency listed but never imported" },
                            "defaultConfiguration": { "level": severity_to_sarif_level(rules.unused_optional_dependencies) }
                        },
                        {
                            "id": "fallow/type-only-dependency",
                            "shortDescription": { "text": "Production dependency only used via type-only imports" },
                            "defaultConfiguration": { "level": severity_to_sarif_level(rules.type_only_dependencies) }
                        },
                        {
                            "id": "fallow/unused-enum-member",
                            "shortDescription": { "text": "Enum member is never referenced" },
                            "defaultConfiguration": { "level": severity_to_sarif_level(rules.unused_enum_members) }
                        },
                        {
                            "id": "fallow/unused-class-member",
                            "shortDescription": { "text": "Class member is never referenced" },
                            "defaultConfiguration": { "level": severity_to_sarif_level(rules.unused_class_members) }
                        },
                        {
                            "id": "fallow/unresolved-import",
                            "shortDescription": { "text": "Import could not be resolved" },
                            "defaultConfiguration": { "level": severity_to_sarif_level(rules.unresolved_imports) }
                        },
                        {
                            "id": "fallow/unlisted-dependency",
                            "shortDescription": { "text": "Dependency used but not in package.json" },
                            "defaultConfiguration": { "level": severity_to_sarif_level(rules.unlisted_dependencies) }
                        },
                        {
                            "id": "fallow/duplicate-export",
                            "shortDescription": { "text": "Export name appears in multiple modules" },
                            "defaultConfiguration": { "level": severity_to_sarif_level(rules.duplicate_exports) }
                        },
                        {
                            "id": "fallow/circular-dependency",
                            "shortDescription": { "text": "Circular dependency chain detected" },
                            "defaultConfiguration": { "level": severity_to_sarif_level(rules.circular_dependencies) }
                        }
                    ]
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
                    "rules": [{
                        "id": "fallow/code-duplication",
                        "shortDescription": { "text": "Duplicated code block" },
                        "defaultConfiguration": { "level": "warning" }
                    }]
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
        });

        let sarif = build_sarif(&results, &root, &RulesConfig::default());
        let entry = &sarif["runs"][0]["results"][0];
        assert_eq!(entry["ruleId"], "fallow/unresolved-import");
        assert_eq!(entry["level"], "error");
    }

    #[test]
    fn sarif_unlisted_dependency_is_error_level() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unlisted_dependencies.push(UnlistedDependency {
            package_name: "chalk".to_string(),
            imported_from: vec![],
        });

        let sarif = build_sarif(&results, &root, &RulesConfig::default());
        let entry = &sarif["runs"][0]["results"][0];
        assert_eq!(entry["ruleId"], "fallow/unlisted-dependency");
        assert_eq!(entry["level"], "error");
        assert_eq!(
            entry["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            "package.json"
        );
    }

    #[test]
    fn sarif_dependency_issues_point_to_package_json() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_dependencies.push(UnusedDependency {
            package_name: "lodash".to_string(),
            location: DependencyLocation::Dependencies,
            path: root.join("package.json"),
        });
        results.unused_dev_dependencies.push(UnusedDependency {
            package_name: "jest".to_string(),
            location: DependencyLocation::DevDependencies,
            path: root.join("package.json"),
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
}
