use std::path::Path;
use std::process::ExitCode;

use fallow_config::{RulesConfig, Severity};
use fallow_core::duplicates::DuplicationReport;
use fallow_core::results::AnalysisResults;

use super::{emit_json, normalize_uri, relative_path};
use crate::health_types::{ExceededThreshold, HealthReport};

/// Map fallow severity to CodeClimate severity.
const fn severity_to_codeclimate(s: Severity) -> &'static str {
    match s {
        Severity::Error => "major",
        Severity::Warn | Severity::Off => "minor",
    }
}

/// Compute a relative path string with forward-slash normalization.
///
/// Uses `normalize_uri` to ensure forward slashes on all platforms
/// and percent-encode brackets for Next.js dynamic routes.
fn cc_path(path: &Path, root: &Path) -> String {
    normalize_uri(&relative_path(path, root).display().to_string())
}

/// Compute a deterministic fingerprint hash from key fields.
///
/// Uses FNV-1a (64-bit) for guaranteed cross-version stability.
/// `DefaultHasher` is explicitly not specified across Rust versions.
fn fingerprint_hash(parts: &[&str]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325; // FNV offset basis
    for part in parts {
        for byte in part.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3); // FNV prime
        }
        // Separator between parts to avoid "ab"+"c" == "a"+"bc"
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Build a single CodeClimate issue object.
fn cc_issue(
    check_name: &str,
    description: &str,
    severity: &str,
    category: &str,
    path: &str,
    begin_line: Option<u32>,
    fingerprint: &str,
) -> serde_json::Value {
    let lines = match begin_line {
        Some(line) => serde_json::json!({ "begin": line }),
        None => serde_json::json!({ "begin": 1 }),
    };

    serde_json::json!({
        "type": "issue",
        "check_name": check_name,
        "description": description,
        "categories": [category],
        "severity": severity,
        "fingerprint": fingerprint,
        "location": {
            "path": path,
            "lines": lines
        }
    })
}

/// Build CodeClimate JSON array from dead-code analysis results.
pub fn build_codeclimate(
    results: &AnalysisResults,
    root: &Path,
    rules: &RulesConfig,
) -> serde_json::Value {
    let mut issues = Vec::new();

    // Unused files
    let level = severity_to_codeclimate(rules.unused_files);
    for file in &results.unused_files {
        let path = cc_path(&file.path, root);
        let fp = fingerprint_hash(&["fallow/unused-file", &path]);
        issues.push(cc_issue(
            "fallow/unused-file",
            "File is not reachable from any entry point",
            level,
            "Bug Risk",
            &path,
            None,
            &fp,
        ));
    }

    // Unused exports
    let level = severity_to_codeclimate(rules.unused_exports);
    for export in &results.unused_exports {
        let path = cc_path(&export.path, root);
        let kind = if export.is_re_export {
            "Re-export"
        } else {
            "Export"
        };
        let line_str = export.line.to_string();
        let fp = fingerprint_hash(&[
            "fallow/unused-export",
            &path,
            &line_str,
            &export.export_name,
        ]);
        issues.push(cc_issue(
            "fallow/unused-export",
            &format!(
                "{kind} '{}' is never imported by other modules",
                export.export_name
            ),
            level,
            "Bug Risk",
            &path,
            Some(export.line),
            &fp,
        ));
    }

    // Unused types
    let level = severity_to_codeclimate(rules.unused_types);
    for export in &results.unused_types {
        let path = cc_path(&export.path, root);
        let kind = if export.is_re_export {
            "Type re-export"
        } else {
            "Type export"
        };
        let line_str = export.line.to_string();
        let fp = fingerprint_hash(&["fallow/unused-type", &path, &line_str, &export.export_name]);
        issues.push(cc_issue(
            "fallow/unused-type",
            &format!(
                "{kind} '{}' is never imported by other modules",
                export.export_name
            ),
            level,
            "Bug Risk",
            &path,
            Some(export.line),
            &fp,
        ));
    }

    // Unused dependencies (shared closure for all dep locations)
    let push_deps = |issues: &mut Vec<serde_json::Value>,
                     deps: &[fallow_core::results::UnusedDependency],
                     rule_id: &str,
                     location_label: &str,
                     severity: Severity| {
        let level = severity_to_codeclimate(severity);
        for dep in deps {
            let path = cc_path(&dep.path, root);
            let line = if dep.line > 0 { Some(dep.line) } else { None };
            let fp = fingerprint_hash(&[rule_id, &dep.package_name]);
            issues.push(cc_issue(
                rule_id,
                &format!(
                    "Package '{}' is in {location_label} but never imported",
                    dep.package_name
                ),
                level,
                "Bug Risk",
                &path,
                line,
                &fp,
            ));
        }
    };

    push_deps(
        &mut issues,
        &results.unused_dependencies,
        "fallow/unused-dependency",
        "dependencies",
        rules.unused_dependencies,
    );
    push_deps(
        &mut issues,
        &results.unused_dev_dependencies,
        "fallow/unused-dev-dependency",
        "devDependencies",
        rules.unused_dev_dependencies,
    );
    push_deps(
        &mut issues,
        &results.unused_optional_dependencies,
        "fallow/unused-optional-dependency",
        "optionalDependencies",
        rules.unused_optional_dependencies,
    );

    // Type-only dependencies
    let level = severity_to_codeclimate(rules.type_only_dependencies);
    for dep in &results.type_only_dependencies {
        let path = cc_path(&dep.path, root);
        let line = if dep.line > 0 { Some(dep.line) } else { None };
        let fp = fingerprint_hash(&["fallow/type-only-dependency", &dep.package_name]);
        issues.push(cc_issue(
            "fallow/type-only-dependency",
            &format!(
                "Package '{}' is only imported via type-only imports (consider moving to devDependencies)",
                dep.package_name
            ),
            level,
            "Bug Risk",
            &path,
            line,
            &fp,
        ));
    }

    // Unused enum members
    let level = severity_to_codeclimate(rules.unused_enum_members);
    for member in &results.unused_enum_members {
        let path = cc_path(&member.path, root);
        let line_str = member.line.to_string();
        let fp = fingerprint_hash(&[
            "fallow/unused-enum-member",
            &path,
            &line_str,
            &member.parent_name,
            &member.member_name,
        ]);
        issues.push(cc_issue(
            "fallow/unused-enum-member",
            &format!(
                "Enum member '{}.{}' is never referenced",
                member.parent_name, member.member_name
            ),
            level,
            "Bug Risk",
            &path,
            Some(member.line),
            &fp,
        ));
    }

    // Unused class members
    let level = severity_to_codeclimate(rules.unused_class_members);
    for member in &results.unused_class_members {
        let path = cc_path(&member.path, root);
        let line_str = member.line.to_string();
        let fp = fingerprint_hash(&[
            "fallow/unused-class-member",
            &path,
            &line_str,
            &member.parent_name,
            &member.member_name,
        ]);
        issues.push(cc_issue(
            "fallow/unused-class-member",
            &format!(
                "Class member '{}.{}' is never referenced",
                member.parent_name, member.member_name
            ),
            level,
            "Bug Risk",
            &path,
            Some(member.line),
            &fp,
        ));
    }

    // Unresolved imports
    let level = severity_to_codeclimate(rules.unresolved_imports);
    for import in &results.unresolved_imports {
        let path = cc_path(&import.path, root);
        let line_str = import.line.to_string();
        let fp = fingerprint_hash(&[
            "fallow/unresolved-import",
            &path,
            &line_str,
            &import.specifier,
        ]);
        issues.push(cc_issue(
            "fallow/unresolved-import",
            &format!("Import '{}' could not be resolved", import.specifier),
            level,
            "Bug Risk",
            &path,
            Some(import.line),
            &fp,
        ));
    }

    // Unlisted dependencies — one issue per import site
    let level = severity_to_codeclimate(rules.unlisted_dependencies);
    for dep in &results.unlisted_dependencies {
        for site in &dep.imported_from {
            let path = cc_path(&site.path, root);
            let line_str = site.line.to_string();
            let fp = fingerprint_hash(&[
                "fallow/unlisted-dependency",
                &path,
                &line_str,
                &dep.package_name,
            ]);
            issues.push(cc_issue(
                "fallow/unlisted-dependency",
                &format!(
                    "Package '{}' is imported but not listed in package.json",
                    dep.package_name
                ),
                level,
                "Bug Risk",
                &path,
                Some(site.line),
                &fp,
            ));
        }
    }

    // Duplicate exports — one issue per location
    let level = severity_to_codeclimate(rules.duplicate_exports);
    for dup in &results.duplicate_exports {
        for loc in &dup.locations {
            let path = cc_path(&loc.path, root);
            let line_str = loc.line.to_string();
            let fp = fingerprint_hash(&[
                "fallow/duplicate-export",
                &path,
                &line_str,
                &dup.export_name,
            ]);
            issues.push(cc_issue(
                "fallow/duplicate-export",
                &format!("Export '{}' appears in multiple modules", dup.export_name),
                level,
                "Bug Risk",
                &path,
                Some(loc.line),
                &fp,
            ));
        }
    }

    // Circular dependencies
    let level = severity_to_codeclimate(rules.circular_dependencies);
    for cycle in &results.circular_dependencies {
        let Some(first) = cycle.files.first() else {
            continue;
        };
        let path = cc_path(first, root);
        let chain: Vec<String> = cycle.files.iter().map(|f| cc_path(f, root)).collect();
        let chain_str = chain.join(":");
        let fp = fingerprint_hash(&["fallow/circular-dependency", &chain_str]);
        let line = if cycle.line > 0 {
            Some(cycle.line)
        } else {
            None
        };
        issues.push(cc_issue(
            "fallow/circular-dependency",
            &format!("Circular dependency: {}", chain.join(" \u{2192} ")),
            level,
            "Bug Risk",
            &path,
            line,
            &fp,
        ));
    }

    serde_json::Value::Array(issues)
}

/// Print dead-code analysis results in CodeClimate format.
pub(super) fn print_codeclimate(
    results: &AnalysisResults,
    root: &Path,
    rules: &RulesConfig,
) -> ExitCode {
    let value = build_codeclimate(results, root, rules);
    emit_json(&value, "CodeClimate")
}

/// Compute graduated severity for health findings based on threshold ratio.
///
/// - 1.0×–1.5× threshold → minor
/// - 1.5×–2.5× threshold → major
/// - >2.5× threshold → critical
fn health_severity(value: u16, threshold: u16) -> &'static str {
    if threshold == 0 {
        return "minor";
    }
    let ratio = f64::from(value) / f64::from(threshold);
    if ratio > 2.5 {
        "critical"
    } else if ratio > 1.5 {
        "major"
    } else {
        "minor"
    }
}

/// Build CodeClimate JSON array from health/complexity analysis results.
pub fn build_health_codeclimate(report: &HealthReport, root: &Path) -> serde_json::Value {
    let mut issues = Vec::new();

    let cyc_t = report.summary.max_cyclomatic_threshold;
    let cog_t = report.summary.max_cognitive_threshold;

    for finding in &report.findings {
        let path = cc_path(&finding.path, root);
        let description = match finding.exceeded {
            ExceededThreshold::Both => format!(
                "'{}' has cyclomatic complexity {} (threshold: {}) and cognitive complexity {} (threshold: {})",
                finding.name, finding.cyclomatic, cyc_t, finding.cognitive, cog_t
            ),
            ExceededThreshold::Cyclomatic => format!(
                "'{}' has cyclomatic complexity {} (threshold: {})",
                finding.name, finding.cyclomatic, cyc_t
            ),
            ExceededThreshold::Cognitive => format!(
                "'{}' has cognitive complexity {} (threshold: {})",
                finding.name, finding.cognitive, cog_t
            ),
        };
        let check_name = match finding.exceeded {
            ExceededThreshold::Both => "fallow/high-complexity",
            ExceededThreshold::Cyclomatic => "fallow/high-cyclomatic-complexity",
            ExceededThreshold::Cognitive => "fallow/high-cognitive-complexity",
        };
        // Graduate severity: use the worst exceeded metric
        let severity = match finding.exceeded {
            ExceededThreshold::Both => {
                let cyc_sev = health_severity(finding.cyclomatic, cyc_t);
                let cog_sev = health_severity(finding.cognitive, cog_t);
                // Pick the more severe of the two
                match (cyc_sev, cog_sev) {
                    ("critical", _) | (_, "critical") => "critical",
                    ("major", _) | (_, "major") => "major",
                    _ => "minor",
                }
            }
            ExceededThreshold::Cyclomatic => health_severity(finding.cyclomatic, cyc_t),
            ExceededThreshold::Cognitive => health_severity(finding.cognitive, cog_t),
        };
        let line_str = finding.line.to_string();
        let fp = fingerprint_hash(&[check_name, &path, &line_str, &finding.name]);
        issues.push(cc_issue(
            check_name,
            &description,
            severity,
            "Complexity",
            &path,
            Some(finding.line),
            &fp,
        ));
    }

    serde_json::Value::Array(issues)
}

/// Print health analysis results in CodeClimate format.
pub(super) fn print_health_codeclimate(report: &HealthReport, root: &Path) -> ExitCode {
    let value = build_health_codeclimate(report, root);
    emit_json(&value, "CodeClimate")
}

/// Build CodeClimate JSON array from duplication analysis results.
pub fn build_duplication_codeclimate(report: &DuplicationReport, root: &Path) -> serde_json::Value {
    let mut issues = Vec::new();

    for (i, group) in report.clone_groups.iter().enumerate() {
        // Content-based fingerprint: hash token_count + line_count + first 64 chars of fragment
        // This is stable across runs regardless of group ordering.
        let token_str = group.token_count.to_string();
        let line_count_str = group.line_count.to_string();
        let fragment_prefix: String = group
            .instances
            .first()
            .map(|inst| inst.fragment.chars().take(64).collect())
            .unwrap_or_default();

        for instance in &group.instances {
            let path = cc_path(&instance.file, root);
            let start_str = instance.start_line.to_string();
            let fp = fingerprint_hash(&[
                "fallow/code-duplication",
                &path,
                &start_str,
                &token_str,
                &line_count_str,
                &fragment_prefix,
            ]);
            issues.push(cc_issue(
                "fallow/code-duplication",
                &format!(
                    "Code clone group {} ({} lines, {} instances)",
                    i + 1,
                    group.line_count,
                    group.instances.len()
                ),
                "minor",
                "Duplication",
                &path,
                Some(instance.start_line as u32),
                &fp,
            ));
        }
    }

    serde_json::Value::Array(issues)
}

/// Print duplication analysis results in CodeClimate format.
pub(super) fn print_duplication_codeclimate(report: &DuplicationReport, root: &Path) -> ExitCode {
    let value = build_duplication_codeclimate(report, root);
    emit_json(&value, "CodeClimate")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::test_helpers::sample_results;
    use fallow_config::RulesConfig;
    use fallow_core::results::*;
    use std::path::PathBuf;

    #[test]
    fn codeclimate_empty_results_produces_empty_array() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let rules = RulesConfig::default();
        let output = build_codeclimate(&results, &root, &rules);
        let arr = output.as_array().unwrap();
        assert!(arr.is_empty());
    }

    #[test]
    fn codeclimate_produces_array_of_issues() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let rules = RulesConfig::default();
        let output = build_codeclimate(&results, &root, &rules);
        assert!(output.is_array());
        let arr = output.as_array().unwrap();
        // Should have at least one issue per type
        assert!(!arr.is_empty());
    }

    #[test]
    fn codeclimate_issue_has_required_fields() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/dead.ts"),
        });
        let rules = RulesConfig::default();
        let output = build_codeclimate(&results, &root, &rules);
        let issue = &output.as_array().unwrap()[0];

        assert_eq!(issue["type"], "issue");
        assert_eq!(issue["check_name"], "fallow/unused-file");
        assert!(issue["description"].is_string());
        assert!(issue["categories"].is_array());
        assert!(issue["severity"].is_string());
        assert!(issue["fingerprint"].is_string());
        assert!(issue["location"].is_object());
        assert!(issue["location"]["path"].is_string());
        assert!(issue["location"]["lines"].is_object());
    }

    #[test]
    fn codeclimate_unused_file_severity_follows_rules() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/dead.ts"),
        });

        // Error severity -> major
        let rules = RulesConfig::default();
        let output = build_codeclimate(&results, &root, &rules);
        assert_eq!(output[0]["severity"], "major");

        // Warn severity -> minor
        let rules = RulesConfig {
            unused_files: Severity::Warn,
            ..RulesConfig::default()
        };
        let output = build_codeclimate(&results, &root, &rules);
        assert_eq!(output[0]["severity"], "minor");
    }

    #[test]
    fn codeclimate_unused_export_has_line_number() {
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
        let rules = RulesConfig::default();
        let output = build_codeclimate(&results, &root, &rules);
        let issue = &output[0];
        assert_eq!(issue["location"]["lines"]["begin"], 10);
    }

    #[test]
    fn codeclimate_unused_file_line_defaults_to_1() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/dead.ts"),
        });
        let rules = RulesConfig::default();
        let output = build_codeclimate(&results, &root, &rules);
        let issue = &output[0];
        assert_eq!(issue["location"]["lines"]["begin"], 1);
    }

    #[test]
    fn codeclimate_paths_are_relative() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/deep/nested/file.ts"),
        });
        let rules = RulesConfig::default();
        let output = build_codeclimate(&results, &root, &rules);
        let path = output[0]["location"]["path"].as_str().unwrap();
        assert_eq!(path, "src/deep/nested/file.ts");
        assert!(!path.starts_with("/project"));
    }

    #[test]
    fn codeclimate_re_export_label_in_description() {
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
        let rules = RulesConfig::default();
        let output = build_codeclimate(&results, &root, &rules);
        let desc = output[0]["description"].as_str().unwrap();
        assert!(desc.contains("Re-export"));
    }

    #[test]
    fn codeclimate_unlisted_dep_one_issue_per_import_site() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unlisted_dependencies.push(UnlistedDependency {
            package_name: "chalk".to_string(),
            imported_from: vec![
                ImportSite {
                    path: root.join("src/a.ts"),
                    line: 1,
                    col: 0,
                },
                ImportSite {
                    path: root.join("src/b.ts"),
                    line: 5,
                    col: 0,
                },
            ],
        });
        let rules = RulesConfig::default();
        let output = build_codeclimate(&results, &root, &rules);
        let arr = output.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["check_name"], "fallow/unlisted-dependency");
        assert_eq!(arr[1]["check_name"], "fallow/unlisted-dependency");
    }

    #[test]
    fn codeclimate_duplicate_export_one_issue_per_location() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.duplicate_exports.push(DuplicateExport {
            export_name: "Config".to_string(),
            locations: vec![
                DuplicateLocation {
                    path: root.join("src/a.ts"),
                    line: 10,
                    col: 0,
                },
                DuplicateLocation {
                    path: root.join("src/b.ts"),
                    line: 20,
                    col: 0,
                },
                DuplicateLocation {
                    path: root.join("src/c.ts"),
                    line: 30,
                    col: 0,
                },
            ],
        });
        let rules = RulesConfig::default();
        let output = build_codeclimate(&results, &root, &rules);
        let arr = output.as_array().unwrap();
        assert_eq!(arr.len(), 3);
    }

    #[test]
    fn codeclimate_circular_dep_emits_chain_in_description() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.circular_dependencies.push(CircularDependency {
            files: vec![root.join("src/a.ts"), root.join("src/b.ts")],
            length: 2,
            line: 3,
            col: 0,
        });
        let rules = RulesConfig::default();
        let output = build_codeclimate(&results, &root, &rules);
        let desc = output[0]["description"].as_str().unwrap();
        assert!(desc.contains("Circular dependency"));
        assert!(desc.contains("src/a.ts"));
        assert!(desc.contains("src/b.ts"));
    }

    #[test]
    fn codeclimate_fingerprints_are_deterministic() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let rules = RulesConfig::default();
        let output1 = build_codeclimate(&results, &root, &rules);
        let output2 = build_codeclimate(&results, &root, &rules);

        let fps1: Vec<&str> = output1
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["fingerprint"].as_str().unwrap())
            .collect();
        let fps2: Vec<&str> = output2
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["fingerprint"].as_str().unwrap())
            .collect();
        assert_eq!(fps1, fps2);
    }

    #[test]
    fn codeclimate_fingerprints_are_unique() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let rules = RulesConfig::default();
        let output = build_codeclimate(&results, &root, &rules);

        let mut fps: Vec<&str> = output
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["fingerprint"].as_str().unwrap())
            .collect();
        let original_len = fps.len();
        fps.sort_unstable();
        fps.dedup();
        assert_eq!(fps.len(), original_len, "fingerprints should be unique");
    }

    #[test]
    fn codeclimate_type_only_dep_has_correct_check_name() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.type_only_dependencies.push(TypeOnlyDependency {
            package_name: "zod".to_string(),
            path: root.join("package.json"),
            line: 8,
        });
        let rules = RulesConfig::default();
        let output = build_codeclimate(&results, &root, &rules);
        assert_eq!(output[0]["check_name"], "fallow/type-only-dependency");
        let desc = output[0]["description"].as_str().unwrap();
        assert!(desc.contains("zod"));
        assert!(desc.contains("type-only"));
    }

    #[test]
    fn codeclimate_dep_with_zero_line_omits_line_number() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_dependencies.push(UnusedDependency {
            package_name: "lodash".to_string(),
            location: DependencyLocation::Dependencies,
            path: root.join("package.json"),
            line: 0,
        });
        let rules = RulesConfig::default();
        let output = build_codeclimate(&results, &root, &rules);
        // Line 0 -> begin defaults to 1
        assert_eq!(output[0]["location"]["lines"]["begin"], 1);
    }

    // ── fingerprint_hash tests ─────────────────────────────────────

    #[test]
    fn fingerprint_hash_different_inputs_differ() {
        let h1 = fingerprint_hash(&["a", "b"]);
        let h2 = fingerprint_hash(&["a", "c"]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn fingerprint_hash_order_matters() {
        let h1 = fingerprint_hash(&["a", "b"]);
        let h2 = fingerprint_hash(&["b", "a"]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn fingerprint_hash_separator_prevents_collision() {
        // "ab" + "c" should differ from "a" + "bc"
        let h1 = fingerprint_hash(&["ab", "c"]);
        let h2 = fingerprint_hash(&["a", "bc"]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn fingerprint_hash_is_16_hex_chars() {
        let h = fingerprint_hash(&["test"]);
        assert_eq!(h.len(), 16);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // ── severity_to_codeclimate ─────────────────────────────────────

    #[test]
    fn severity_error_maps_to_major() {
        assert_eq!(severity_to_codeclimate(Severity::Error), "major");
    }

    #[test]
    fn severity_warn_maps_to_minor() {
        assert_eq!(severity_to_codeclimate(Severity::Warn), "minor");
    }

    #[test]
    fn severity_off_maps_to_minor() {
        assert_eq!(severity_to_codeclimate(Severity::Off), "minor");
    }

    // ── health_severity ─────────────────────────────────────────────

    #[test]
    fn health_severity_zero_threshold_returns_minor() {
        assert_eq!(health_severity(100, 0), "minor");
    }

    #[test]
    fn health_severity_at_threshold_returns_minor() {
        assert_eq!(health_severity(10, 10), "minor");
    }

    #[test]
    fn health_severity_1_5x_threshold_returns_minor() {
        assert_eq!(health_severity(15, 10), "minor");
    }

    #[test]
    fn health_severity_above_1_5x_returns_major() {
        assert_eq!(health_severity(16, 10), "major");
    }

    #[test]
    fn health_severity_at_2_5x_returns_major() {
        assert_eq!(health_severity(25, 10), "major");
    }

    #[test]
    fn health_severity_above_2_5x_returns_critical() {
        assert_eq!(health_severity(26, 10), "critical");
    }
}
