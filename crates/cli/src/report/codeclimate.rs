use std::path::Path;
use std::process::ExitCode;

use fallow_config::{RulesConfig, Severity};
use fallow_core::duplicates::DuplicationReport;
use fallow_core::results::AnalysisResults;

use super::relative_path;
use crate::health_types::{ExceededThreshold, HealthReport};

/// Map fallow severity to CodeClimate severity.
const fn severity_to_codeclimate(s: Severity) -> &'static str {
    match s {
        Severity::Error => "major",
        Severity::Warn | Severity::Off => "minor",
    }
}

/// Build a single CodeClimate issue object.
fn cc_issue(
    check_name: &str,
    description: &str,
    severity: &str,
    path: &str,
    begin_line: Option<u32>,
) -> serde_json::Value {
    let fingerprint = match begin_line {
        Some(line) => format!("{check_name}:{path}:{line}"),
        None => format!("{check_name}:{path}"),
    };

    let lines = match begin_line {
        Some(line) => serde_json::json!({ "begin": line }),
        None => serde_json::json!({ "begin": 1 }),
    };

    serde_json::json!({
        "type": "issue",
        "check_name": check_name,
        "description": description,
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
        let path = relative_path(&file.path, root).display().to_string();
        issues.push(cc_issue(
            "fallow/unused-file",
            "File is not reachable from any entry point",
            level,
            &path,
            None,
        ));
    }

    // Unused exports
    let level = severity_to_codeclimate(rules.unused_exports);
    for export in &results.unused_exports {
        let path = relative_path(&export.path, root).display().to_string();
        let kind = if export.is_re_export {
            "Re-export"
        } else {
            "Export"
        };
        issues.push(cc_issue(
            "fallow/unused-export",
            &format!(
                "{kind} '{}' is never imported by other modules",
                export.export_name
            ),
            level,
            &path,
            Some(export.line),
        ));
    }

    // Unused types
    let level = severity_to_codeclimate(rules.unused_types);
    for export in &results.unused_types {
        let path = relative_path(&export.path, root).display().to_string();
        let kind = if export.is_re_export {
            "Type re-export"
        } else {
            "Type export"
        };
        issues.push(cc_issue(
            "fallow/unused-type",
            &format!(
                "{kind} '{}' is never imported by other modules",
                export.export_name
            ),
            level,
            &path,
            Some(export.line),
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
            let path = relative_path(&dep.path, root).display().to_string();
            let line = if dep.line > 0 { Some(dep.line) } else { None };
            issues.push(cc_issue(
                rule_id,
                &format!(
                    "Package '{}' is in {location_label} but never imported",
                    dep.package_name
                ),
                level,
                &path,
                line,
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
        let path = relative_path(&dep.path, root).display().to_string();
        let line = if dep.line > 0 { Some(dep.line) } else { None };
        issues.push(cc_issue(
            "fallow/type-only-dependency",
            &format!(
                "Package '{}' is only imported via type-only imports (consider moving to devDependencies)",
                dep.package_name
            ),
            level,
            &path,
            line,
        ));
    }

    // Unused enum members
    let level = severity_to_codeclimate(rules.unused_enum_members);
    for member in &results.unused_enum_members {
        let path = relative_path(&member.path, root).display().to_string();
        issues.push(cc_issue(
            "fallow/unused-enum-member",
            &format!(
                "Enum member '{}.{}' is never referenced",
                member.parent_name, member.member_name
            ),
            level,
            &path,
            Some(member.line),
        ));
    }

    // Unused class members
    let level = severity_to_codeclimate(rules.unused_class_members);
    for member in &results.unused_class_members {
        let path = relative_path(&member.path, root).display().to_string();
        issues.push(cc_issue(
            "fallow/unused-class-member",
            &format!(
                "Class member '{}.{}' is never referenced",
                member.parent_name, member.member_name
            ),
            level,
            &path,
            Some(member.line),
        ));
    }

    // Unresolved imports
    let level = severity_to_codeclimate(rules.unresolved_imports);
    for import in &results.unresolved_imports {
        let path = relative_path(&import.path, root).display().to_string();
        issues.push(cc_issue(
            "fallow/unresolved-import",
            &format!("Import '{}' could not be resolved", import.specifier),
            level,
            &path,
            Some(import.line),
        ));
    }

    // Unlisted dependencies — one issue per import site
    let level = severity_to_codeclimate(rules.unlisted_dependencies);
    for dep in &results.unlisted_dependencies {
        for site in &dep.imported_from {
            let path = relative_path(&site.path, root).display().to_string();
            issues.push(cc_issue(
                "fallow/unlisted-dependency",
                &format!(
                    "Package '{}' is imported but not listed in package.json",
                    dep.package_name
                ),
                level,
                &path,
                Some(site.line),
            ));
        }
    }

    // Duplicate exports — one issue per location
    let level = severity_to_codeclimate(rules.duplicate_exports);
    for dup in &results.duplicate_exports {
        for loc in &dup.locations {
            let path = relative_path(&loc.path, root).display().to_string();
            issues.push(cc_issue(
                "fallow/duplicate-export",
                &format!("Export '{}' appears in multiple modules", dup.export_name),
                level,
                &path,
                Some(loc.line),
            ));
        }
    }

    // Circular dependencies
    let level = severity_to_codeclimate(rules.circular_dependencies);
    for cycle in &results.circular_dependencies {
        let first = &cycle.files[0];
        let path = relative_path(first, root).display().to_string();
        let chain: Vec<String> = cycle
            .files
            .iter()
            .map(|f| relative_path(f, root).display().to_string())
            .collect();
        let line = if cycle.line > 0 {
            Some(cycle.line)
        } else {
            None
        };
        issues.push(cc_issue(
            "fallow/circular-dependency",
            &format!("Circular dependency: {}", chain.join(" \u{2192} ")),
            level,
            &path,
            line,
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
    match serde_json::to_string_pretty(&value) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: failed to serialize CodeClimate output: {e}");
            ExitCode::from(2)
        }
    }
}

/// Build CodeClimate JSON array from health/complexity analysis results.
pub fn build_health_codeclimate(report: &HealthReport, root: &Path) -> serde_json::Value {
    let mut issues = Vec::new();

    for finding in &report.findings {
        let path = relative_path(&finding.path, root).display().to_string();
        let description = match finding.exceeded {
            ExceededThreshold::Both => format!(
                "'{}' has cyclomatic complexity {} (threshold: {}) and cognitive complexity {} (threshold: {})",
                finding.name,
                finding.cyclomatic,
                report.summary.max_cyclomatic_threshold,
                finding.cognitive,
                report.summary.max_cognitive_threshold
            ),
            ExceededThreshold::Cyclomatic => format!(
                "'{}' has cyclomatic complexity {} (threshold: {})",
                finding.name, finding.cyclomatic, report.summary.max_cyclomatic_threshold
            ),
            ExceededThreshold::Cognitive => format!(
                "'{}' has cognitive complexity {} (threshold: {})",
                finding.name, finding.cognitive, report.summary.max_cognitive_threshold
            ),
        };
        let check_name = match finding.exceeded {
            ExceededThreshold::Both => "fallow/high-complexity",
            ExceededThreshold::Cyclomatic => "fallow/high-cyclomatic-complexity",
            ExceededThreshold::Cognitive => "fallow/high-cognitive-complexity",
        };
        issues.push(cc_issue(
            check_name,
            &description,
            "minor",
            &path,
            Some(finding.line),
        ));
    }

    serde_json::Value::Array(issues)
}

/// Print health analysis results in CodeClimate format.
pub(super) fn print_health_codeclimate(report: &HealthReport, root: &Path) -> ExitCode {
    let value = build_health_codeclimate(report, root);
    match serde_json::to_string_pretty(&value) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: failed to serialize CodeClimate output: {e}");
            ExitCode::from(2)
        }
    }
}

/// Build CodeClimate JSON array from duplication analysis results.
pub fn build_duplication_codeclimate(report: &DuplicationReport, root: &Path) -> serde_json::Value {
    let mut issues = Vec::new();

    for (i, group) in report.clone_groups.iter().enumerate() {
        for instance in &group.instances {
            let path = relative_path(&instance.file, root).display().to_string();
            let line = instance.start_line as u32;
            // Include group index in fingerprint to distinguish overlapping clone groups
            let fingerprint = format!("fallow/code-duplication:{}:{}:g{}", path, line, i + 1);
            let mut issue = cc_issue(
                "fallow/code-duplication",
                &format!(
                    "Code clone group {} ({} lines, {} instances)",
                    i + 1,
                    group.line_count,
                    group.instances.len()
                ),
                "minor",
                &path,
                Some(line),
            );
            issue["fingerprint"] = serde_json::Value::String(fingerprint);
            issues.push(issue);
        }
    }

    serde_json::Value::Array(issues)
}

/// Print duplication analysis results in CodeClimate format.
pub(super) fn print_duplication_codeclimate(report: &DuplicationReport, root: &Path) -> ExitCode {
    let value = build_duplication_codeclimate(report, root);
    match serde_json::to_string_pretty(&value) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: failed to serialize CodeClimate output: {e}");
            ExitCode::from(2)
        }
    }
}
