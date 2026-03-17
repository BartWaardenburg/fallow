use std::time::Duration;

use fallow_config::{OutputFormat, ResolvedConfig};
use fallow_core::results::AnalysisResults;

/// Print analysis results in the configured format.
pub fn print_results(
    results: &AnalysisResults,
    config: &ResolvedConfig,
    elapsed: Duration,
    quiet: bool,
) {
    match config.output {
        OutputFormat::Human => print_human(results, &config.root, elapsed, quiet),
        OutputFormat::Json => print_json(results, elapsed),
        OutputFormat::Compact => print_compact(results, &config.root),
        OutputFormat::Sarif => print_sarif(results, &config.root),
    }
}

fn print_human(results: &AnalysisResults, root: &std::path::Path, elapsed: Duration, quiet: bool) {
    if !quiet {
        eprintln!();
    }

    if !results.unused_files.is_empty() {
        println!("Unused files ({})", results.unused_files.len());
        println!("{}", "-".repeat(60));
        for file in &results.unused_files {
            let relative = file.path.strip_prefix(root).unwrap_or(&file.path);
            println!("  {}", relative.display());
        }
        println!();
    }

    if !results.unused_exports.is_empty() {
        println!("Unused exports ({})", results.unused_exports.len());
        println!("{}", "-".repeat(60));
        for export in &results.unused_exports {
            let relative = export.path.strip_prefix(root).unwrap_or(&export.path);
            println!("  {}  `{}`", relative.display(), export.export_name);
        }
        println!();
    }

    if !results.unused_types.is_empty() {
        println!("Unused type exports ({})", results.unused_types.len());
        println!("{}", "-".repeat(60));
        for export in &results.unused_types {
            let relative = export.path.strip_prefix(root).unwrap_or(&export.path);
            println!("  {}  `{}`", relative.display(), export.export_name);
        }
        println!();
    }

    if !results.unused_dependencies.is_empty() {
        println!(
            "Unused dependencies ({})",
            results.unused_dependencies.len()
        );
        println!("{}", "-".repeat(60));
        for dep in &results.unused_dependencies {
            println!("  {}", dep.package_name);
        }
        println!();
    }

    if !results.unused_dev_dependencies.is_empty() {
        println!(
            "Unused devDependencies ({})",
            results.unused_dev_dependencies.len()
        );
        println!("{}", "-".repeat(60));
        for dep in &results.unused_dev_dependencies {
            println!("  {}", dep.package_name);
        }
        println!();
    }

    if !results.unused_enum_members.is_empty() {
        println!(
            "Unused enum members ({})",
            results.unused_enum_members.len()
        );
        println!("{}", "-".repeat(60));
        for member in &results.unused_enum_members {
            let relative = member.path.strip_prefix(root).unwrap_or(&member.path);
            println!(
                "  {}  `{}.{}`",
                relative.display(),
                member.parent_name,
                member.member_name
            );
        }
        println!();
    }

    if !results.unused_class_members.is_empty() {
        println!(
            "Unused class members ({})",
            results.unused_class_members.len()
        );
        println!("{}", "-".repeat(60));
        for member in &results.unused_class_members {
            let relative = member.path.strip_prefix(root).unwrap_or(&member.path);
            println!(
                "  {}  `{}.{}`",
                relative.display(),
                member.parent_name,
                member.member_name
            );
        }
        println!();
    }

    if !results.unresolved_imports.is_empty() {
        println!("Unresolved imports ({})", results.unresolved_imports.len());
        println!("{}", "-".repeat(60));
        for import in &results.unresolved_imports {
            let relative = import.path.strip_prefix(root).unwrap_or(&import.path);
            println!("  {}  `{}`", relative.display(), import.specifier);
        }
        println!();
    }

    if !results.unlisted_dependencies.is_empty() {
        println!(
            "Unlisted dependencies ({})",
            results.unlisted_dependencies.len()
        );
        println!("{}", "-".repeat(60));
        for dep in &results.unlisted_dependencies {
            println!("  {}", dep.package_name);
        }
        println!();
    }

    if !results.duplicate_exports.is_empty() {
        println!("Duplicate exports ({})", results.duplicate_exports.len());
        println!("{}", "-".repeat(60));
        for dup in &results.duplicate_exports {
            let locations: Vec<String> = dup
                .locations
                .iter()
                .map(|p| p.strip_prefix(root).unwrap_or(p).display().to_string())
                .collect();
            println!("  `{}` in {}", dup.export_name, locations.join(", "));
        }
        println!();
    }

    if !quiet {
        let total = results.total_issues();
        if total == 0 {
            eprintln!("No issues found. ({:.2}s)", elapsed.as_secs_f64());
        } else {
            eprintln!(
                "Found {} issue{} ({:.2}s)",
                total,
                if total == 1 { "" } else { "s" },
                elapsed.as_secs_f64()
            );
        }
    }
}

fn print_json(results: &AnalysisResults, elapsed: Duration) {
    // Merge metadata alongside result fields for backwards compatibility
    let mut output = serde_json::to_value(results).expect("Failed to serialize results");
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
    println!(
        "{}",
        serde_json::to_string_pretty(&output).expect("Failed to serialize JSON")
    );
}

fn print_compact(results: &AnalysisResults, root: &std::path::Path) {
    for file in &results.unused_files {
        let relative = file.path.strip_prefix(root).unwrap_or(&file.path);
        println!("unused-file:{}", relative.display());
    }
    for export in &results.unused_exports {
        let relative = export.path.strip_prefix(root).unwrap_or(&export.path);
        println!(
            "unused-export:{}:{}:{}",
            relative.display(),
            export.line,
            export.export_name
        );
    }
    for export in &results.unused_types {
        let relative = export.path.strip_prefix(root).unwrap_or(&export.path);
        println!(
            "unused-type:{}:{}:{}",
            relative.display(),
            export.line,
            export.export_name
        );
    }
    for dep in &results.unused_dependencies {
        println!("unused-dep:{}", dep.package_name);
    }
    for dep in &results.unused_dev_dependencies {
        println!("unused-devdep:{}", dep.package_name);
    }
    for member in &results.unused_enum_members {
        let relative = member.path.strip_prefix(root).unwrap_or(&member.path);
        println!(
            "unused-enum-member:{}:{}:{}.{}",
            relative.display(),
            member.line,
            member.parent_name,
            member.member_name
        );
    }
    for member in &results.unused_class_members {
        let relative = member.path.strip_prefix(root).unwrap_or(&member.path);
        println!(
            "unused-class-member:{}:{}:{}.{}",
            relative.display(),
            member.line,
            member.parent_name,
            member.member_name
        );
    }
    for import in &results.unresolved_imports {
        let relative = import.path.strip_prefix(root).unwrap_or(&import.path);
        println!(
            "unresolved-import:{}:{}:{}",
            relative.display(),
            import.line,
            import.specifier
        );
    }
    for dep in &results.unlisted_dependencies {
        println!("unlisted-dep:{}", dep.package_name);
    }
    for dup in &results.duplicate_exports {
        println!("duplicate-export:{}", dup.export_name);
    }
}

fn print_sarif(results: &AnalysisResults, root: &std::path::Path) {
    let sarif = build_sarif(results, root);
    let json = serde_json::to_string_pretty(&sarif).expect("Failed to serialize SARIF");
    println!("{json}");
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
                    "artifactLocation": { "uri": relative.display().to_string() }
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
                    "artifactLocation": { "uri": relative.display().to_string() },
                    "region": { "startLine": export.line }
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
                    "artifactLocation": { "uri": relative.display().to_string() },
                    "region": { "startLine": export.line }
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
                    "artifactLocation": { "uri": relative.display().to_string() },
                    "region": { "startLine": member.line }
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
                    "artifactLocation": { "uri": relative.display().to_string() },
                    "region": { "startLine": member.line }
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
                    "artifactLocation": { "uri": relative.display().to_string() },
                    "region": { "startLine": import.line }
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
        // Emit one result per location (SARIF 2.1.0 §3.27.12)
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
                        "artifactLocation": { "uri": relative.display().to_string() }
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
