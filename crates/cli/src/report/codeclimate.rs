use std::path::Path;
use std::process::ExitCode;

use fallow_config::{RulesConfig, Severity};
use fallow_core::duplicates::DuplicationReport;
use fallow_core::results::AnalysisResults;

use super::{normalize_uri, relative_path};
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
