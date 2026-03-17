use std::process::ExitCode;
use std::time::Duration;

use colored::Colorize;
use fallow_config::{OutputFormat, ResolvedConfig};
use fallow_core::results::AnalysisResults;

/// Print analysis results in the configured format.
/// Returns exit code 2 if serialization fails, SUCCESS otherwise.
pub fn print_results(
    results: &AnalysisResults,
    config: &ResolvedConfig,
    elapsed: Duration,
    quiet: bool,
) -> ExitCode {
    match config.output {
        OutputFormat::Human => {
            print_human(results, &config.root, elapsed, quiet);
            ExitCode::SUCCESS
        }
        OutputFormat::Json => print_json(results, elapsed),
        OutputFormat::Compact => {
            print_compact(results, &config.root);
            ExitCode::SUCCESS
        }
        OutputFormat::Sarif => print_sarif(results, &config.root),
    }
}

fn print_human(results: &AnalysisResults, root: &std::path::Path, elapsed: Duration, quiet: bool) {
    let separator = "-".repeat(60);

    if !quiet {
        eprintln!();
    }

    // Warning-level: unused files
    if !results.unused_files.is_empty() {
        println!(
            "{}",
            format!("Unused files ({})", results.unused_files.len())
                .yellow()
                .bold()
        );
        println!("{}", separator.dimmed());
        for file in &results.unused_files {
            let relative = file.path.strip_prefix(root).unwrap_or(&file.path);
            println!("  {}", relative.display());
        }
        println!();
    }

    // Info-level: unused exports
    if !results.unused_exports.is_empty() {
        println!(
            "{}",
            format!("Unused exports ({})", results.unused_exports.len())
                .cyan()
                .bold()
        );
        println!("{}", separator.dimmed());
        for export in &results.unused_exports {
            let relative = export.path.strip_prefix(root).unwrap_or(&export.path);
            println!("  {}  {}", relative.display(), export.export_name.bold());
        }
        println!();
    }

    // Info-level: unused types
    if !results.unused_types.is_empty() {
        println!(
            "{}",
            format!("Unused type exports ({})", results.unused_types.len())
                .cyan()
                .bold()
        );
        println!("{}", separator.dimmed());
        for export in &results.unused_types {
            let relative = export.path.strip_prefix(root).unwrap_or(&export.path);
            println!("  {}  {}", relative.display(), export.export_name.bold());
        }
        println!();
    }

    // Warning-level: unused dependencies
    if !results.unused_dependencies.is_empty() {
        println!(
            "{}",
            format!(
                "Unused dependencies ({})",
                results.unused_dependencies.len()
            )
            .yellow()
            .bold()
        );
        println!("{}", separator.dimmed());
        for dep in &results.unused_dependencies {
            println!("  {}", dep.package_name.bold());
        }
        println!();
    }

    // Warning-level: unused devDependencies
    if !results.unused_dev_dependencies.is_empty() {
        println!(
            "{}",
            format!(
                "Unused devDependencies ({})",
                results.unused_dev_dependencies.len()
            )
            .yellow()
            .bold()
        );
        println!("{}", separator.dimmed());
        for dep in &results.unused_dev_dependencies {
            println!("  {}", dep.package_name.bold());
        }
        println!();
    }

    // Info-level: unused enum members
    if !results.unused_enum_members.is_empty() {
        println!(
            "{}",
            format!(
                "Unused enum members ({})",
                results.unused_enum_members.len()
            )
            .cyan()
            .bold()
        );
        println!("{}", separator.dimmed());
        for member in &results.unused_enum_members {
            let relative = member.path.strip_prefix(root).unwrap_or(&member.path);
            println!(
                "  {}  {}",
                relative.display(),
                format!("{}.{}", member.parent_name, member.member_name).bold()
            );
        }
        println!();
    }

    // Info-level: unused class members
    if !results.unused_class_members.is_empty() {
        println!(
            "{}",
            format!(
                "Unused class members ({})",
                results.unused_class_members.len()
            )
            .cyan()
            .bold()
        );
        println!("{}", separator.dimmed());
        for member in &results.unused_class_members {
            let relative = member.path.strip_prefix(root).unwrap_or(&member.path);
            println!(
                "  {}  {}",
                relative.display(),
                format!("{}.{}", member.parent_name, member.member_name).bold()
            );
        }
        println!();
    }

    // Error-level: unresolved imports
    if !results.unresolved_imports.is_empty() {
        println!(
            "{}",
            format!("Unresolved imports ({})", results.unresolved_imports.len())
                .red()
                .bold()
        );
        println!("{}", separator.dimmed());
        for import in &results.unresolved_imports {
            let relative = import.path.strip_prefix(root).unwrap_or(&import.path);
            println!("  {}  {}", relative.display(), import.specifier.bold());
        }
        println!();
    }

    // Warning-level: unlisted dependencies
    if !results.unlisted_dependencies.is_empty() {
        println!(
            "{}",
            format!(
                "Unlisted dependencies ({})",
                results.unlisted_dependencies.len()
            )
            .yellow()
            .bold()
        );
        println!("{}", separator.dimmed());
        for dep in &results.unlisted_dependencies {
            println!("  {}", dep.package_name.bold());
        }
        println!();
    }

    // Info-level: duplicate exports
    if !results.duplicate_exports.is_empty() {
        println!(
            "{}",
            format!("Duplicate exports ({})", results.duplicate_exports.len())
                .cyan()
                .bold()
        );
        println!("{}", separator.dimmed());
        for dup in &results.duplicate_exports {
            let locations: Vec<String> = dup
                .locations
                .iter()
                .map(|p| p.strip_prefix(root).unwrap_or(p).display().to_string())
                .collect();
            println!("  {} in {}", dup.export_name.bold(), locations.join(", "));
        }
        println!();
    }

    if !quiet {
        let total = results.total_issues();
        if total == 0 {
            eprintln!(
                "{}",
                format!("No issues found. ({:.2}s)", elapsed.as_secs_f64())
                    .green()
                    .bold()
            );
        } else {
            eprintln!(
                "{}",
                format!(
                    "Found {} issue{} ({:.2}s)",
                    total,
                    if total == 1 { "" } else { "s" },
                    elapsed.as_secs_f64()
                )
                .red()
                .bold()
            );
        }
    }
}

fn print_json(results: &AnalysisResults, elapsed: Duration) -> ExitCode {
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

fn print_compact(results: &AnalysisResults, root: &std::path::Path) {
    for line in build_compact_lines(results, root) {
        println!("{line}");
    }
}

/// Normalize a path string to use forward slashes for cross-platform SARIF compatibility.
fn normalize_uri(path_str: &str) -> String {
    path_str.replace('\\', "/")
}

fn print_sarif(results: &AnalysisResults, root: &std::path::Path) -> ExitCode {
    let sarif = build_sarif(results, root);
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

/// Build compact output lines for analysis results.
/// Each issue is represented as a single `prefix:details` line.
fn build_compact_lines(results: &AnalysisResults, root: &std::path::Path) -> Vec<String> {
    let mut lines = Vec::new();

    for file in &results.unused_files {
        let relative = file.path.strip_prefix(root).unwrap_or(&file.path);
        lines.push(format!("unused-file:{}", relative.display()));
    }
    for export in &results.unused_exports {
        let relative = export.path.strip_prefix(root).unwrap_or(&export.path);
        lines.push(format!(
            "unused-export:{}:{}:{}",
            relative.display(),
            export.line,
            export.export_name
        ));
    }
    for export in &results.unused_types {
        let relative = export.path.strip_prefix(root).unwrap_or(&export.path);
        lines.push(format!(
            "unused-type:{}:{}:{}",
            relative.display(),
            export.line,
            export.export_name
        ));
    }
    for dep in &results.unused_dependencies {
        lines.push(format!("unused-dep:{}", dep.package_name));
    }
    for dep in &results.unused_dev_dependencies {
        lines.push(format!("unused-devdep:{}", dep.package_name));
    }
    for member in &results.unused_enum_members {
        let relative = member.path.strip_prefix(root).unwrap_or(&member.path);
        lines.push(format!(
            "unused-enum-member:{}:{}:{}.{}",
            relative.display(),
            member.line,
            member.parent_name,
            member.member_name
        ));
    }
    for member in &results.unused_class_members {
        let relative = member.path.strip_prefix(root).unwrap_or(&member.path);
        lines.push(format!(
            "unused-class-member:{}:{}:{}.{}",
            relative.display(),
            member.line,
            member.parent_name,
            member.member_name
        ));
    }
    for import in &results.unresolved_imports {
        let relative = import.path.strip_prefix(root).unwrap_or(&import.path);
        lines.push(format!(
            "unresolved-import:{}:{}:{}",
            relative.display(),
            import.line,
            import.specifier
        ));
    }
    for dep in &results.unlisted_dependencies {
        lines.push(format!("unlisted-dep:{}", dep.package_name));
    }
    for dup in &results.duplicate_exports {
        lines.push(format!("duplicate-export:{}", dup.export_name));
    }

    lines
}

/// Build the JSON output value for analysis results.
fn build_json(results: &AnalysisResults, elapsed: Duration) -> Result<serde_json::Value, serde_json::Error> {
    let mut output = serde_json::to_value(results)?;
    if let serde_json::Value::Object(ref mut map) = output {
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

fn build_sarif(results: &AnalysisResults, root: &std::path::Path) -> serde_json::Value {
    let mut sarif_results = Vec::new();

    for file in &results.unused_files {
        let relative = file.path.strip_prefix(root).unwrap_or(&file.path);
        sarif_results.push(serde_json::json!({
            "ruleId": "fallow/unused-file",
            "level": "warning",
            "message": { "text": "File is not reachable from any entry point" },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": { "uri": normalize_uri(&relative.display().to_string()) }
                }
            }]
        }));
    }

    for export in &results.unused_exports {
        let relative = export.path.strip_prefix(root).unwrap_or(&export.path);
        sarif_results.push(serde_json::json!({
            "ruleId": "fallow/unused-export",
            "level": "warning",
            "message": {
                "text": format!("Export '{}' is never imported by other modules", export.export_name)
            },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": { "uri": normalize_uri(&relative.display().to_string()) },
                    "region": { "startLine": export.line, "startColumn": export.col + 1 }
                }
            }]
        }));
    }

    for export in &results.unused_types {
        let relative = export.path.strip_prefix(root).unwrap_or(&export.path);
        sarif_results.push(serde_json::json!({
            "ruleId": "fallow/unused-type",
            "level": "warning",
            "message": {
                "text": format!("Type export '{}' is never imported by other modules", export.export_name)
            },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": { "uri": normalize_uri(&relative.display().to_string()) },
                    "region": { "startLine": export.line, "startColumn": export.col + 1 }
                }
            }]
        }));
    }

    for dep in &results.unused_dependencies {
        sarif_results.push(serde_json::json!({
            "ruleId": "fallow/unused-dependency",
            "level": "warning",
            "message": {
                "text": format!("Package '{}' is in dependencies but never imported", dep.package_name)
            },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": { "uri": "package.json" }
                }
            }]
        }));
    }

    for dep in &results.unused_dev_dependencies {
        sarif_results.push(serde_json::json!({
            "ruleId": "fallow/unused-dev-dependency",
            "level": "warning",
            "message": {
                "text": format!("Package '{}' is in devDependencies but never imported", dep.package_name)
            },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": { "uri": "package.json" }
                }
            }]
        }));
    }

    for member in &results.unused_enum_members {
        let relative = member.path.strip_prefix(root).unwrap_or(&member.path);
        sarif_results.push(serde_json::json!({
            "ruleId": "fallow/unused-enum-member",
            "level": "warning",
            "message": {
                "text": format!("Enum member '{}.{}' is never referenced", member.parent_name, member.member_name)
            },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": { "uri": normalize_uri(&relative.display().to_string()) },
                    "region": { "startLine": member.line, "startColumn": member.col + 1 }
                }
            }]
        }));
    }

    for member in &results.unused_class_members {
        let relative = member.path.strip_prefix(root).unwrap_or(&member.path);
        sarif_results.push(serde_json::json!({
            "ruleId": "fallow/unused-class-member",
            "level": "warning",
            "message": {
                "text": format!("Class member '{}.{}' is never referenced", member.parent_name, member.member_name)
            },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": { "uri": normalize_uri(&relative.display().to_string()) },
                    "region": { "startLine": member.line, "startColumn": member.col + 1 }
                }
            }]
        }));
    }

    for import in &results.unresolved_imports {
        let relative = import.path.strip_prefix(root).unwrap_or(&import.path);
        sarif_results.push(serde_json::json!({
            "ruleId": "fallow/unresolved-import",
            "level": "error",
            "message": {
                "text": format!("Import '{}' could not be resolved", import.specifier)
            },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": { "uri": normalize_uri(&relative.display().to_string()) },
                    "region": { "startLine": import.line, "startColumn": import.col + 1 }
                }
            }]
        }));
    }

    for dep in &results.unlisted_dependencies {
        sarif_results.push(serde_json::json!({
            "ruleId": "fallow/unlisted-dependency",
            "level": "error",
            "message": {
                "text": format!("Package '{}' is imported but not listed in package.json", dep.package_name)
            },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": { "uri": "package.json" }
                }
            }]
        }));
    }

    for dup in &results.duplicate_exports {
        // Emit one result per location (SARIF 2.1.0 section 3.27.12)
        for loc_path in &dup.locations {
            let relative = loc_path.strip_prefix(root).unwrap_or(loc_path);
            sarif_results.push(serde_json::json!({
                "ruleId": "fallow/duplicate-export",
                "level": "warning",
                "message": {
                    "text": format!("Export '{}' appears in multiple modules", dup.export_name)
                },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": normalize_uri(&relative.display().to_string()) }
                    }
                }]
            }));
        }
    }

    serde_json::json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "fallow",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/bartwaardenburg/fallow",
                    "rules": [
                        {
                            "id": "fallow/unused-file",
                            "shortDescription": { "text": "File is not reachable from any entry point" },
                            "defaultConfiguration": { "level": "warning" }
                        },
                        {
                            "id": "fallow/unused-export",
                            "shortDescription": { "text": "Export is never imported" },
                            "defaultConfiguration": { "level": "warning" }
                        },
                        {
                            "id": "fallow/unused-type",
                            "shortDescription": { "text": "Type export is never imported" },
                            "defaultConfiguration": { "level": "warning" }
                        },
                        {
                            "id": "fallow/unused-dependency",
                            "shortDescription": { "text": "Dependency listed but never imported" },
                            "defaultConfiguration": { "level": "warning" }
                        },
                        {
                            "id": "fallow/unused-dev-dependency",
                            "shortDescription": { "text": "Dev dependency listed but never imported" },
                            "defaultConfiguration": { "level": "warning" }
                        },
                        {
                            "id": "fallow/unused-enum-member",
                            "shortDescription": { "text": "Enum member is never referenced" },
                            "defaultConfiguration": { "level": "warning" }
                        },
                        {
                            "id": "fallow/unused-class-member",
                            "shortDescription": { "text": "Class member is never referenced" },
                            "defaultConfiguration": { "level": "warning" }
                        },
                        {
                            "id": "fallow/unresolved-import",
                            "shortDescription": { "text": "Import could not be resolved" },
                            "defaultConfiguration": { "level": "error" }
                        },
                        {
                            "id": "fallow/unlisted-dependency",
                            "shortDescription": { "text": "Dependency used but not in package.json" },
                            "defaultConfiguration": { "level": "error" }
                        },
                        {
                            "id": "fallow/duplicate-export",
                            "shortDescription": { "text": "Export name appears in multiple modules" },
                            "defaultConfiguration": { "level": "warning" }
                        }
                    ]
                }
            },
            "results": sarif_results
        }]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fallow_core::results::*;
    use std::path::PathBuf;
    use std::time::Duration;

    /// Helper: build an `AnalysisResults` populated with one issue of every type.
    fn sample_results(root: &std::path::Path) -> AnalysisResults {
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
        });
        r.unused_types.push(UnusedExport {
            path: root.join("src/types.ts"),
            export_name: "OldType".to_string(),
            is_type_only: true,
            line: 5,
            col: 0,
            span_start: 60,
        });
        r.unused_dependencies.push(UnusedDependency {
            package_name: "lodash".to_string(),
            location: DependencyLocation::Dependencies,
        });
        r.unused_dev_dependencies.push(UnusedDependency {
            package_name: "jest".to_string(),
            location: DependencyLocation::DevDependencies,
        });
        r.unused_enum_members.push(UnusedMember {
            path: root.join("src/enums.ts"),
            parent_name: "Status".to_string(),
            member_name: "Deprecated".to_string(),
            kind: "enum_member".to_string(),
            line: 8,
            col: 2,
        });
        r.unused_class_members.push(UnusedMember {
            path: root.join("src/service.ts"),
            parent_name: "UserService".to_string(),
            member_name: "legacyMethod".to_string(),
            kind: "class_method".to_string(),
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

    // ── normalize_uri ────────────────────────────────────────────────

    #[test]
    fn normalize_uri_forward_slashes_unchanged() {
        assert_eq!(normalize_uri("src/utils.ts"), "src/utils.ts");
    }

    #[test]
    fn normalize_uri_backslashes_replaced() {
        assert_eq!(normalize_uri("src\\utils\\index.ts"), "src/utils/index.ts");
    }

    #[test]
    fn normalize_uri_mixed_slashes() {
        assert_eq!(normalize_uri("src\\utils/index.ts"), "src/utils/index.ts");
    }

    #[test]
    fn normalize_uri_path_with_spaces() {
        assert_eq!(
            normalize_uri("src\\my folder\\file.ts"),
            "src/my folder/file.ts"
        );
    }

    #[test]
    fn normalize_uri_empty_string() {
        assert_eq!(normalize_uri(""), "");
    }

    // ── SARIF output ─────────────────────────────────────────────────

    #[test]
    fn sarif_has_required_top_level_fields() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let sarif = build_sarif(&results, &root);

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
        let sarif = build_sarif(&results, &root);

        let driver = &sarif["runs"][0]["tool"]["driver"];
        assert_eq!(driver["name"], "fallow");
        assert!(driver["version"].is_string());
        assert_eq!(
            driver["informationUri"],
            "https://github.com/bartwaardenburg/fallow"
        );
    }

    #[test]
    fn sarif_declares_all_ten_rules() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let sarif = build_sarif(&results, &root);

        let rules = sarif["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .expect("rules should be an array");
        assert_eq!(rules.len(), 10);

        let rule_ids: Vec<&str> = rules.iter().map(|r| r["id"].as_str().unwrap()).collect();
        assert!(rule_ids.contains(&"fallow/unused-file"));
        assert!(rule_ids.contains(&"fallow/unused-export"));
        assert!(rule_ids.contains(&"fallow/unused-type"));
        assert!(rule_ids.contains(&"fallow/unused-dependency"));
        assert!(rule_ids.contains(&"fallow/unused-dev-dependency"));
        assert!(rule_ids.contains(&"fallow/unused-enum-member"));
        assert!(rule_ids.contains(&"fallow/unused-class-member"));
        assert!(rule_ids.contains(&"fallow/unresolved-import"));
        assert!(rule_ids.contains(&"fallow/unlisted-dependency"));
        assert!(rule_ids.contains(&"fallow/duplicate-export"));
    }

    #[test]
    fn sarif_empty_results_no_results_entries() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let sarif = build_sarif(&results, &root);

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

        let sarif = build_sarif(&results, &root);
        let entries = sarif["runs"][0]["results"].as_array().unwrap();
        assert_eq!(entries.len(), 1);

        let entry = &entries[0];
        assert_eq!(entry["ruleId"], "fallow/unused-file");
        assert_eq!(entry["level"], "warning");
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
        });

        let sarif = build_sarif(&results, &root);
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

        let sarif = build_sarif(&results, &root);
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

        let sarif = build_sarif(&results, &root);
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
        });
        results.unused_dev_dependencies.push(UnusedDependency {
            package_name: "jest".to_string(),
            location: DependencyLocation::DevDependencies,
        });

        let sarif = build_sarif(&results, &root);
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
            locations: vec![root.join("src/a.ts"), root.join("src/b.ts")],
        });

        let sarif = build_sarif(&results, &root);
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
        let sarif = build_sarif(&results, &root);

        let entries = sarif["runs"][0]["results"].as_array().unwrap();
        // 10 issues but duplicate_exports has 2 locations => 11 SARIF results
        assert_eq!(entries.len(), 11);

        let rule_ids: Vec<&str> = entries.iter().map(|e| e["ruleId"].as_str().unwrap()).collect();
        assert!(rule_ids.contains(&"fallow/unused-file"));
        assert!(rule_ids.contains(&"fallow/unused-export"));
        assert!(rule_ids.contains(&"fallow/unused-type"));
        assert!(rule_ids.contains(&"fallow/unused-dependency"));
        assert!(rule_ids.contains(&"fallow/unused-dev-dependency"));
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
        let sarif = build_sarif(&results, &root);

        let json_str = serde_json::to_string_pretty(&sarif).expect("SARIF should serialize");
        let reparsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("SARIF output should be valid JSON");
        assert_eq!(reparsed, sarif);
    }

    // ── JSON output ──────────────────────────────────────────────────

    #[test]
    fn json_output_has_metadata_fields() {
        let results = AnalysisResults::default();
        let elapsed = Duration::from_millis(123);
        let output = build_json(&results, elapsed).expect("should serialize");

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
        });
        let elapsed = Duration::from_millis(0);
        let output = build_json(&results, elapsed).expect("should serialize");

        let export = &output["unused_exports"][0];
        assert_eq!(export["export_name"], "helperFn");
        assert_eq!(export["line"], 10);
        assert_eq!(export["col"], 4);
        assert_eq!(export["is_type_only"], false);
        assert_eq!(export["span_start"], 120);
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

    // ── Compact output ───────────────────────────────────────────────

    #[test]
    fn compact_empty_results_no_lines() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let lines = build_compact_lines(&results, &root);
        assert!(lines.is_empty());
    }

    #[test]
    fn compact_unused_file_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/dead.ts"),
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "unused-file:src/dead.ts");
    }

    #[test]
    fn compact_unused_export_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_exports.push(UnusedExport {
            path: root.join("src/utils.ts"),
            export_name: "helperFn".to_string(),
            is_type_only: false,
            line: 10,
            col: 4,
            span_start: 120,
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(lines[0], "unused-export:src/utils.ts:10:helperFn");
    }

    #[test]
    fn compact_unused_type_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_types.push(UnusedExport {
            path: root.join("src/types.ts"),
            export_name: "OldType".to_string(),
            is_type_only: true,
            line: 5,
            col: 0,
            span_start: 60,
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(lines[0], "unused-type:src/types.ts:5:OldType");
    }

    #[test]
    fn compact_unused_dep_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_dependencies.push(UnusedDependency {
            package_name: "lodash".to_string(),
            location: DependencyLocation::Dependencies,
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(lines[0], "unused-dep:lodash");
    }

    #[test]
    fn compact_unused_devdep_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_dev_dependencies.push(UnusedDependency {
            package_name: "jest".to_string(),
            location: DependencyLocation::DevDependencies,
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(lines[0], "unused-devdep:jest");
    }

    #[test]
    fn compact_unused_enum_member_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_enum_members.push(UnusedMember {
            path: root.join("src/enums.ts"),
            parent_name: "Status".to_string(),
            member_name: "Deprecated".to_string(),
            kind: "enum_member".to_string(),
            line: 8,
            col: 2,
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(
            lines[0],
            "unused-enum-member:src/enums.ts:8:Status.Deprecated"
        );
    }

    #[test]
    fn compact_unused_class_member_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_class_members.push(UnusedMember {
            path: root.join("src/service.ts"),
            parent_name: "UserService".to_string(),
            member_name: "legacyMethod".to_string(),
            kind: "class_method".to_string(),
            line: 42,
            col: 4,
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(
            lines[0],
            "unused-class-member:src/service.ts:42:UserService.legacyMethod"
        );
    }

    #[test]
    fn compact_unresolved_import_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unresolved_imports.push(UnresolvedImport {
            path: root.join("src/app.ts"),
            specifier: "./missing-module".to_string(),
            line: 3,
            col: 0,
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(
            lines[0],
            "unresolved-import:src/app.ts:3:./missing-module"
        );
    }

    #[test]
    fn compact_unlisted_dep_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unlisted_dependencies.push(UnlistedDependency {
            package_name: "chalk".to_string(),
            imported_from: vec![],
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(lines[0], "unlisted-dep:chalk");
    }

    #[test]
    fn compact_duplicate_export_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.duplicate_exports.push(DuplicateExport {
            export_name: "Config".to_string(),
            locations: vec![root.join("src/a.ts"), root.join("src/b.ts")],
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(lines[0], "duplicate-export:Config");
    }

    #[test]
    fn compact_all_issue_types_produce_lines() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let lines = build_compact_lines(&results, &root);

        // 10 issue types, one of each
        assert_eq!(lines.len(), 10);

        // Verify ordering: unused_files first, duplicate_exports last
        assert!(lines[0].starts_with("unused-file:"));
        assert!(lines[1].starts_with("unused-export:"));
        assert!(lines[2].starts_with("unused-type:"));
        assert!(lines[3].starts_with("unused-dep:"));
        assert!(lines[4].starts_with("unused-devdep:"));
        assert!(lines[5].starts_with("unused-enum-member:"));
        assert!(lines[6].starts_with("unused-class-member:"));
        assert!(lines[7].starts_with("unresolved-import:"));
        assert!(lines[8].starts_with("unlisted-dep:"));
        assert!(lines[9].starts_with("duplicate-export:"));
    }

    #[test]
    fn compact_strips_root_prefix_from_paths() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: PathBuf::from("/project/src/deep/nested/file.ts"),
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(lines[0], "unused-file:src/deep/nested/file.ts");
    }
}
