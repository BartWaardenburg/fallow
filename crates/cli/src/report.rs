use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use colored::Colorize;
use fallow_config::{OutputFormat, ResolvedConfig, RulesConfig, Severity};
use fallow_core::duplicates::DuplicationReport;
use fallow_core::results::{AnalysisResults, UnusedDependency, UnusedExport, UnusedMember};
use fallow_core::trace::{CloneTrace, DependencyTrace, ExportTrace, FileTrace, PipelineTimings};

/// Strip the project root prefix from a path for display, falling back to the full path.
fn relative_path<'a>(path: &'a Path, root: &Path) -> &'a Path {
    path.strip_prefix(root).unwrap_or(path)
}

/// Compute a SARIF-compatible relative URI from an absolute path and project root.
fn relative_uri(path: &Path, root: &Path) -> String {
    normalize_uri(&relative_path(path, root).display().to_string())
}

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
            print_human(results, &config.root, &config.rules, elapsed, quiet);
            ExitCode::SUCCESS
        }
        OutputFormat::Json => print_json(results, elapsed),
        OutputFormat::Compact => {
            print_compact(results, &config.root);
            ExitCode::SUCCESS
        }
        OutputFormat::Sarif => print_sarif(results, &config.root, &config.rules),
    }
}

fn print_human(
    results: &AnalysisResults,
    root: &Path,
    rules: &RulesConfig,
    elapsed: Duration,
    quiet: bool,
) {
    if !quiet {
        eprintln!();
    }

    let format_export = |e: &UnusedExport| -> String {
        let tag = if e.is_re_export {
            " (re-export)".dimmed().to_string()
        } else {
            String::new()
        };
        format!(
            "{} {}{}",
            format!(":{}", e.line).dimmed(),
            e.export_name.bold(),
            tag
        )
    };

    let format_member = |m: &UnusedMember| -> String {
        format!(
            "{} {}",
            format!(":{}", m.line).dimmed(),
            format!("{}.{}", m.parent_name, m.member_name).bold()
        )
    };

    let format_dep = |dep: &UnusedDependency| -> String {
        let pkg_label = relative_path(&dep.path, root).display().to_string();
        if pkg_label == "package.json" {
            format!("{}", dep.package_name.bold())
        } else {
            format!("{} ({})", dep.package_name.bold(), pkg_label.dimmed())
        }
    };

    print_human_section(
        &results.unused_files,
        "Unused files",
        severity_to_level(rules.unused_files),
        |file| vec![format!("  {}", relative_path(&file.path, root).display())],
    );

    print_human_grouped_section(
        &results.unused_exports,
        "Unused exports",
        severity_to_level(rules.unused_exports),
        root,
        |e| e.path.as_path(),
        format_export,
    );

    print_human_grouped_section(
        &results.unused_types,
        "Unused type exports",
        severity_to_level(rules.unused_types),
        root,
        |e| e.path.as_path(),
        format_export,
    );

    print_human_section(
        &results.unused_dependencies,
        "Unused dependencies",
        severity_to_level(rules.unused_dependencies),
        |dep| vec![format!("  {}", format_dep(dep))],
    );

    print_human_section(
        &results.unused_dev_dependencies,
        "Unused devDependencies",
        severity_to_level(rules.unused_dev_dependencies),
        |dep| vec![format!("  {}", format_dep(dep))],
    );

    print_human_grouped_section(
        &results.unused_enum_members,
        "Unused enum members",
        severity_to_level(rules.unused_enum_members),
        root,
        |m| m.path.as_path(),
        format_member,
    );

    print_human_grouped_section(
        &results.unused_class_members,
        "Unused class members",
        severity_to_level(rules.unused_class_members),
        root,
        |m| m.path.as_path(),
        format_member,
    );

    print_human_grouped_section(
        &results.unresolved_imports,
        "Unresolved imports",
        severity_to_level(rules.unresolved_imports),
        root,
        |i| i.path.as_path(),
        |i| format!("{} {}", format!(":{}", i.line).dimmed(), i.specifier.bold()),
    );

    print_human_section(
        &results.unlisted_dependencies,
        "Unlisted dependencies",
        severity_to_level(rules.unlisted_dependencies),
        |dep| vec![format!("  {}", dep.package_name.bold())],
    );

    print_human_section(
        &results.duplicate_exports,
        "Duplicate exports",
        severity_to_level(rules.duplicate_exports),
        |dup| {
            let locations: Vec<String> = dup
                .locations
                .iter()
                .map(|p| relative_path(p, root).display().to_string())
                .collect();
            vec![format!(
                "  {}  {}",
                dup.export_name.bold(),
                locations.join(", ").dimmed()
            )]
        },
    );

    if !results.type_only_dependencies.is_empty() {
        print_human_section(
            &results.type_only_dependencies,
            "Type-only dependencies (consider moving to devDependencies)",
            Level::Warn,
            |dep| {
                let pkg_label = relative_path(&dep.path, root).display().to_string();
                if pkg_label == "package.json" {
                    vec![format!("  {}", dep.package_name.bold())]
                } else {
                    vec![format!(
                        "  {} ({})",
                        dep.package_name.bold(),
                        pkg_label.dimmed()
                    )]
                }
            },
        );
    }

    if !quiet {
        let total = results.total_issues();
        if total == 0 {
            eprintln!(
                "{}",
                format!("\u{2713} No issues found ({:.2}s)", elapsed.as_secs_f64())
                    .green()
                    .bold()
            );
        } else {
            eprintln!(
                "{}",
                format!(
                    "\u{2717} Found {} issue{} ({:.2}s)",
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

/// Print a non-empty section with a header and per-item lines.
fn print_human_section<T>(
    items: &[T],
    title: &str,
    level: Level,
    format_lines: impl Fn(&T) -> Vec<String>,
) {
    if items.is_empty() {
        return;
    }
    print_section_header(title, items.len(), level);
    for item in items {
        for line in format_lines(item) {
            println!("{line}");
        }
    }
    println!();
}

/// Print a non-empty section whose items are grouped by file path.
fn print_human_grouped_section<'a, T>(
    items: &'a [T],
    title: &str,
    level: Level,
    root: &Path,
    get_path: impl Fn(&'a T) -> &'a Path,
    format_detail: impl Fn(&T) -> String,
) {
    if items.is_empty() {
        return;
    }
    print_section_header(title, items.len(), level);
    print_grouped_by_file(items, root, get_path, format_detail);
    println!();
}

enum Level {
    Warn,
    Info,
    Error,
}

fn severity_to_level(s: Severity) -> Level {
    match s {
        Severity::Error => Level::Error,
        Severity::Warn => Level::Warn,
        // Off issues are filtered before reporting; fall back to Info.
        Severity::Off => Level::Info,
    }
}

fn print_section_header(title: &str, count: usize, level: Level) {
    let label = format!("{title} ({count})");
    match level {
        Level::Warn => println!("{} {}", "\u{25cf}".yellow(), label.yellow().bold()),
        Level::Info => println!("{} {}", "\u{25cf}".cyan(), label.cyan().bold()),
        Level::Error => println!("{} {}", "\u{25cf}".red(), label.red().bold()),
    }
}

/// Print items grouped by file path. Items are sorted by path so that
/// entries from the same file appear together, with the file path printed
/// once as a dimmed header and each item indented beneath it.
fn print_grouped_by_file<'a, T>(
    items: &'a [T],
    root: &Path,
    get_path: impl Fn(&'a T) -> &'a Path,
    format_detail: impl Fn(&T) -> String,
) {
    let mut indices: Vec<usize> = (0..items.len()).collect();
    indices.sort_by(|&a, &b| get_path(&items[a]).cmp(get_path(&items[b])));

    let mut last_file = String::new();
    for &i in &indices {
        let item = &items[i];
        let file_str = relative_path(get_path(item), root).display().to_string();
        if file_str != last_file {
            println!("  {}", file_str.dimmed());
            last_file = file_str;
        }
        println!("    {}", format_detail(item));
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

fn print_compact(results: &AnalysisResults, root: &Path) {
    for line in build_compact_lines(results, root) {
        println!("{line}");
    }
}

/// Normalize a path string to use forward slashes for cross-platform SARIF compatibility.
fn normalize_uri(path_str: &str) -> String {
    path_str.replace('\\', "/")
}

fn print_sarif(results: &AnalysisResults, root: &Path, rules: &RulesConfig) -> ExitCode {
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

/// Build compact output lines for analysis results.
/// Each issue is represented as a single `prefix:details` line.
fn build_compact_lines(results: &AnalysisResults, root: &Path) -> Vec<String> {
    let rel = |p: &Path| relative_path(p, root).display().to_string();

    let compact_export = |export: &UnusedExport, kind: &str, re_kind: &str| -> String {
        let tag = if export.is_re_export { re_kind } else { kind };
        format!(
            "{}:{}:{}:{}",
            tag,
            rel(&export.path),
            export.line,
            export.export_name
        )
    };

    let compact_member = |member: &UnusedMember, kind: &str| -> String {
        format!(
            "{}:{}:{}:{}.{}",
            kind,
            rel(&member.path),
            member.line,
            member.parent_name,
            member.member_name
        )
    };

    let mut lines = Vec::new();

    for file in &results.unused_files {
        lines.push(format!("unused-file:{}", rel(&file.path)));
    }
    for export in &results.unused_exports {
        lines.push(compact_export(export, "unused-export", "unused-re-export"));
    }
    for export in &results.unused_types {
        lines.push(compact_export(
            export,
            "unused-type",
            "unused-re-export-type",
        ));
    }
    for dep in &results.unused_dependencies {
        lines.push(format!("unused-dep:{}", dep.package_name));
    }
    for dep in &results.unused_dev_dependencies {
        lines.push(format!("unused-devdep:{}", dep.package_name));
    }
    for member in &results.unused_enum_members {
        lines.push(compact_member(member, "unused-enum-member"));
    }
    for member in &results.unused_class_members {
        lines.push(compact_member(member, "unused-class-member"));
    }
    for import in &results.unresolved_imports {
        lines.push(format!(
            "unresolved-import:{}:{}:{}",
            rel(&import.path),
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
fn build_json(
    results: &AnalysisResults,
    elapsed: Duration,
) -> Result<serde_json::Value, serde_json::Error> {
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

/// Intermediate fields extracted from an issue for SARIF result construction.
struct SarifFields {
    rule_id: &'static str,
    level: &'static str,
    message: String,
    uri: String,
    region: Option<(u32, u32)>,
    properties: Option<serde_json::Value>,
}

fn severity_to_sarif_level(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warn | Severity::Off => "warning",
    }
}

pub(crate) fn build_sarif(
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
        for loc_path in &dup.locations {
            sarif_results.push(sarif_result(
                "fallow/duplicate-export",
                severity_to_sarif_level(rules.duplicate_exports),
                &format!("Export '{}' appears in multiple modules", dup.export_name),
                &relative_uri(loc_path, root),
                None,
            ));
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
                        }
                    ]
                }
            },
            "results": sarif_results
        }]
    })
}

// ── Duplication report ────────────────────────────────────────────

/// Print duplication analysis results in the configured format.
pub fn print_duplication_report(
    report: &DuplicationReport,
    config: &ResolvedConfig,
    elapsed: Duration,
    quiet: bool,
    output: &OutputFormat,
) -> ExitCode {
    match output {
        OutputFormat::Human => {
            print_duplication_human(report, &config.root, elapsed, quiet);
            ExitCode::SUCCESS
        }
        OutputFormat::Json => print_duplication_json(report, elapsed),
        OutputFormat::Compact => {
            print_duplication_compact(report, &config.root);
            ExitCode::SUCCESS
        }
        OutputFormat::Sarif => print_duplication_sarif(report, &config.root),
    }
}

fn print_duplication_human(
    report: &DuplicationReport,
    root: &Path,
    elapsed: Duration,
    quiet: bool,
) {
    if !quiet {
        eprintln!();
    }

    if report.clone_groups.is_empty() {
        if !quiet {
            eprintln!(
                "{}",
                format!(
                    "\u{2713} No code duplication found ({:.2}s)",
                    elapsed.as_secs_f64()
                )
                .green()
                .bold()
            );
        }
        return;
    }

    println!("{} {}", "\u{25cf}".cyan(), "Duplicates".cyan().bold());
    println!();

    for (i, group) in report.clone_groups.iter().enumerate() {
        let instance_count = group.instances.len();
        println!(
            "  {} ({} lines, {} instance{})",
            format!("Clone group {}", i + 1).bold(),
            group.line_count,
            instance_count,
            if instance_count == 1 { "" } else { "s" }
        );

        for (j, instance) in group.instances.iter().enumerate() {
            let relative = relative_path(&instance.file, root);
            let location = format!(
                "{}:{}-{}",
                relative.display(),
                instance.start_line,
                instance.end_line
            );
            let connector = if j == instance_count - 1 {
                "\u{2514}\u{2500}"
            } else {
                "\u{251c}\u{2500}"
            };
            println!("  {} {}", connector, location.dimmed());
        }
        println!();
    }

    // Print clone families with refactoring suggestions
    if !report.clone_families.is_empty() {
        println!(
            "{} {}",
            "\u{25cf}".yellow(),
            "Clone Families".yellow().bold()
        );
        println!();

        for (i, family) in report.clone_families.iter().enumerate() {
            let file_names: Vec<_> = family
                .files
                .iter()
                .map(|f| relative_path(f, root).display().to_string())
                .collect();
            println!(
                "  {} ({} group{}, {} lines across {})",
                format!("Family {}", i + 1).bold(),
                family.groups.len(),
                if family.groups.len() == 1 { "" } else { "s" },
                family.total_duplicated_lines,
                file_names.join(", "),
            );

            for suggestion in &family.suggestions {
                let savings = if suggestion.estimated_savings > 0 {
                    format!(" (~{} lines saved)", suggestion.estimated_savings)
                } else {
                    String::new()
                };
                println!(
                    "  {} {}{}",
                    "\u{2192}".yellow(),
                    suggestion.description.dimmed(),
                    savings.dimmed(),
                );
            }
            println!();
        }
    }

    let stats = &report.stats;
    if !quiet {
        eprintln!(
            "{}",
            format!(
                "Found {} clone group{} with {} instance{} in {} famil{}",
                stats.clone_groups,
                if stats.clone_groups == 1 { "" } else { "s" },
                stats.clone_instances,
                if stats.clone_instances == 1 { "" } else { "s" },
                report.clone_families.len(),
                if report.clone_families.len() == 1 {
                    "y"
                } else {
                    "ies"
                },
            )
            .bold()
        );
        eprintln!(
            "{}",
            format!(
                "Duplicated: {} lines ({:.1}%) across {} file{}",
                stats.duplicated_lines,
                stats.duplication_percentage,
                stats.files_with_clones,
                if stats.files_with_clones == 1 {
                    ""
                } else {
                    "s"
                },
            )
            .dimmed()
        );
        eprintln!(
            "{}",
            format!("Completed in {:.2}s", elapsed.as_secs_f64()).dimmed()
        );
    }
}

fn print_duplication_json(report: &DuplicationReport, elapsed: Duration) -> ExitCode {
    let mut output = match serde_json::to_value(report) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: failed to serialize duplication report: {e}");
            return ExitCode::from(2);
        }
    };

    if let serde_json::Value::Object(ref mut map) = output {
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

fn print_duplication_compact(report: &DuplicationReport, root: &Path) {
    for (i, group) in report.clone_groups.iter().enumerate() {
        for instance in &group.instances {
            let relative = relative_path(&instance.file, root);
            println!(
                "clone-group-{}:{}:{}-{}:{}tokens",
                i + 1,
                relative.display(),
                instance.start_line,
                instance.end_line,
                group.token_count
            );
        }
    }
}

fn print_duplication_sarif(report: &DuplicationReport, root: &Path) -> ExitCode {
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

/// Print cross-reference findings (duplicated code that is also dead code).
///
/// Only emits output in human format to avoid corrupting structured JSON/SARIF output.
pub fn print_cross_reference_findings(
    cross_ref: &fallow_core::cross_reference::CrossReferenceResult,
    root: &Path,
    quiet: bool,
    output: &OutputFormat,
) {
    use fallow_core::cross_reference::DeadCodeKind;

    if cross_ref.combined_findings.is_empty() {
        return;
    }

    // Only emit human-readable output; structured formats (JSON, SARIF, Compact)
    // should not have unstructured text mixed into stdout.
    if !matches!(output, OutputFormat::Human) {
        return;
    }

    if quiet {
        return;
    }

    println!();
    println!(
        "{} {}",
        "\u{25cf}".yellow(),
        "Duplicated + Unused (safe to delete)".yellow().bold()
    );
    println!();

    for finding in &cross_ref.combined_findings {
        let relative = relative_path(&finding.clone_instance.file, root);
        let location = format!(
            "{}:{}-{}",
            relative.display(),
            finding.clone_instance.start_line,
            finding.clone_instance.end_line
        );

        let reason = match &finding.dead_code_kind {
            DeadCodeKind::UnusedFile => "entire file is unused".to_string(),
            DeadCodeKind::UnusedExport { export_name } => {
                format!("export '{export_name}' is unused")
            }
            DeadCodeKind::UnusedType { type_name } => {
                format!("type '{type_name}' is unused")
            }
        };

        println!("  {} {}", location.bold(), format!("({reason})").dimmed());
    }

    println!();
    let total = cross_ref.total();
    let files = cross_ref.clones_in_unused_files;
    let exports = cross_ref.clones_with_unused_exports;
    eprintln!(
        "  {} combined finding{}: {} in unused file{}, {} overlapping unused export{}",
        total,
        if total == 1 { "" } else { "s" },
        files,
        if files == 1 { "" } else { "s" },
        exports,
        if exports == 1 { "" } else { "s" },
    );
}

// ── Trace output ──────────────────────────────────────────────────

fn print_trace_json<T: serde::Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("Error: failed to serialize trace output: {e}");
            std::process::exit(2);
        }
    }
}

/// Print export trace results.
pub fn print_export_trace(trace: &ExportTrace, format: &OutputFormat) {
    match format {
        OutputFormat::Json => print_trace_json(trace),
        _ => print_export_trace_human(trace),
    }
}

fn print_export_trace_human(trace: &ExportTrace) {
    eprintln!();
    let status_icon = if trace.is_used {
        "USED".green().bold()
    } else {
        "UNUSED".red().bold()
    };
    eprintln!(
        "  {} {} in {}",
        status_icon,
        trace.export_name.bold(),
        trace.file.display().to_string().dimmed()
    );
    eprintln!();

    // File status
    let reachable = if trace.file_reachable {
        "reachable".green()
    } else {
        "unreachable".red()
    };
    let entry = if trace.is_entry_point {
        " (entry point)".cyan().to_string()
    } else {
        String::new()
    };
    eprintln!("  File: {reachable}{entry}");
    eprintln!("  Reason: {}", trace.reason);

    if !trace.direct_references.is_empty() {
        eprintln!();
        eprintln!("  {} direct reference(s):", trace.direct_references.len());
        for r in &trace.direct_references {
            eprintln!(
                "    {} {} ({})",
                "->".dimmed(),
                r.from_file.display(),
                r.kind.dimmed()
            );
        }
    }

    if !trace.re_export_chains.is_empty() {
        eprintln!();
        eprintln!("  Re-exported through:");
        for chain in &trace.re_export_chains {
            eprintln!(
                "    {} {} as '{}' ({} ref(s))",
                "->".dimmed(),
                chain.barrel_file.display(),
                chain.exported_as,
                chain.reference_count
            );
        }
    }
    eprintln!();
}

/// Print file trace results.
pub fn print_file_trace(trace: &FileTrace, format: &OutputFormat) {
    match format {
        OutputFormat::Json => print_trace_json(trace),
        _ => print_file_trace_human(trace),
    }
}

fn print_file_trace_human(trace: &FileTrace) {
    eprintln!();
    let reachable = if trace.is_reachable {
        "REACHABLE".green().bold()
    } else {
        "UNREACHABLE".red().bold()
    };
    let entry = if trace.is_entry_point {
        format!(" {}", "(entry point)".cyan())
    } else {
        String::new()
    };
    eprintln!(
        "  {} {}{}",
        reachable,
        trace.file.display().to_string().bold(),
        entry
    );

    if !trace.exports.is_empty() {
        eprintln!();
        eprintln!("  Exports ({}):", trace.exports.len());
        for export in &trace.exports {
            let used_indicator = if export.reference_count > 0 {
                format!("{} ref(s)", export.reference_count)
                    .green()
                    .to_string()
            } else {
                "unused".red().to_string()
            };
            let type_tag = if export.is_type_only {
                " (type)".dimmed().to_string()
            } else {
                String::new()
            };
            eprintln!(
                "    {} {}{} [{}]",
                "export".dimmed(),
                export.name.bold(),
                type_tag,
                used_indicator
            );
            for r in &export.referenced_by {
                eprintln!(
                    "      {} {} ({})",
                    "->".dimmed(),
                    r.from_file.display(),
                    r.kind.dimmed()
                );
            }
        }
    }

    if !trace.imports_from.is_empty() {
        eprintln!();
        eprintln!("  Imports from ({}):", trace.imports_from.len());
        for path in &trace.imports_from {
            eprintln!("    {} {}", "<-".dimmed(), path.display());
        }
    }

    if !trace.imported_by.is_empty() {
        eprintln!();
        eprintln!("  Imported by ({}):", trace.imported_by.len());
        for path in &trace.imported_by {
            eprintln!("    {} {}", "->".dimmed(), path.display());
        }
    }

    if !trace.re_exports.is_empty() {
        eprintln!();
        eprintln!("  Re-exports ({}):", trace.re_exports.len());
        for re in &trace.re_exports {
            eprintln!(
                "    {} '{}' as '{}' from {}",
                "re-export".dimmed(),
                re.imported_name,
                re.exported_name,
                re.source_file.display()
            );
        }
    }
    eprintln!();
}

/// Print dependency trace results.
pub fn print_dependency_trace(trace: &DependencyTrace, format: &OutputFormat) {
    match format {
        OutputFormat::Json => print_trace_json(trace),
        _ => print_dependency_trace_human(trace),
    }
}

fn print_dependency_trace_human(trace: &DependencyTrace) {
    eprintln!();
    let status = if trace.is_used {
        "USED".green().bold()
    } else {
        "UNUSED".red().bold()
    };
    eprintln!(
        "  {} {} ({} import(s))",
        status,
        trace.package_name.bold(),
        trace.import_count
    );

    if !trace.imported_by.is_empty() {
        eprintln!();
        eprintln!("  Imported by:");
        for path in &trace.imported_by {
            let is_type_only = trace.type_only_imported_by.contains(path);
            let tag = if is_type_only {
                " (type-only)".dimmed().to_string()
            } else {
                String::new()
            };
            eprintln!("    {} {}{}", "->".dimmed(), path.display(), tag);
        }
    }
    eprintln!();
}

/// Print clone trace results.
pub fn print_clone_trace(trace: &CloneTrace, root: &Path, format: &OutputFormat) {
    match format {
        OutputFormat::Json => print_trace_json(trace),
        _ => print_clone_trace_human(trace, root),
    }
}

fn print_clone_trace_human(trace: &CloneTrace, root: &Path) {
    eprintln!();
    if let Some(ref matched) = trace.matched_instance {
        let relative = relative_path(&matched.file, root);
        eprintln!(
            "  {} clone at {}:{}-{}",
            "FOUND".green().bold(),
            relative.display(),
            matched.start_line,
            matched.end_line,
        );
    }
    eprintln!(
        "  {} clone group(s) containing this location",
        trace.clone_groups.len()
    );
    for (i, group) in trace.clone_groups.iter().enumerate() {
        eprintln!();
        eprintln!(
            "  {} ({} lines, {} tokens, {} instance{})",
            format!("Clone group {}", i + 1).bold(),
            group.line_count,
            group.token_count,
            group.instances.len(),
            if group.instances.len() == 1 { "" } else { "s" }
        );
        for instance in &group.instances {
            let relative = relative_path(&instance.file, root);
            let is_queried = trace.matched_instance.as_ref().is_some_and(|m| {
                m.file == instance.file
                    && m.start_line == instance.start_line
                    && m.end_line == instance.end_line
            });
            let marker = if is_queried {
                ">>".cyan()
            } else {
                "->".dimmed()
            };
            eprintln!(
                "    {} {}:{}-{}",
                marker,
                relative.display(),
                instance.start_line,
                instance.end_line
            );
        }
    }
    if let Some(ref matched) = trace.matched_instance {
        eprintln!();
        eprintln!("  {}:", "Code fragment".dimmed());
        for (i, line) in matched.fragment.lines().enumerate() {
            eprintln!(
                "    {} {}",
                format!("{:>4}", matched.start_line + i).dimmed(),
                line
            );
        }
    }
    eprintln!();
}

/// Print pipeline performance timings.
/// In JSON mode, outputs to stderr to avoid polluting the JSON analysis output on stdout.
pub fn print_performance(timings: &PipelineTimings, format: &OutputFormat) {
    match format {
        OutputFormat::Json => match serde_json::to_string_pretty(timings) {
            Ok(json) => eprintln!("{json}"),
            Err(e) => eprintln!("Error: failed to serialize timings: {e}"),
        },
        _ => print_performance_human(timings),
    }
}

fn print_performance_human(t: &PipelineTimings) {
    eprintln!();
    eprintln!(
        "{}",
        "┌─ Pipeline Performance ─────────────────────────────".dimmed()
    );
    eprintln!(
        "{}",
        format!(
            "│  discover files:   {:>8.1}ms  ({} files)",
            t.discover_files_ms, t.file_count
        )
        .dimmed()
    );
    eprintln!(
        "{}",
        format!(
            "│  workspaces:       {:>8.1}ms  ({} workspaces)",
            t.workspaces_ms, t.workspace_count
        )
        .dimmed()
    );
    eprintln!(
        "{}",
        format!("│  plugins:          {:>8.1}ms", t.plugins_ms).dimmed()
    );
    eprintln!(
        "{}",
        format!("│  script analysis:  {:>8.1}ms", t.script_analysis_ms).dimmed()
    );
    eprintln!(
        "{}",
        format!(
            "│  parse/extract:    {:>8.1}ms  ({} modules)",
            t.parse_extract_ms, t.module_count
        )
        .dimmed()
    );
    eprintln!(
        "{}",
        format!("│  cache update:     {:>8.1}ms", t.cache_update_ms).dimmed()
    );
    eprintln!(
        "{}",
        format!(
            "│  entry points:     {:>8.1}ms  ({} entries)",
            t.entry_points_ms, t.entry_point_count
        )
        .dimmed()
    );
    eprintln!(
        "{}",
        format!("│  resolve imports:  {:>8.1}ms", t.resolve_imports_ms).dimmed()
    );
    eprintln!(
        "{}",
        format!("│  build graph:      {:>8.1}ms", t.build_graph_ms).dimmed()
    );
    eprintln!(
        "{}",
        format!("│  analyze:          {:>8.1}ms", t.analyze_ms).dimmed()
    );
    eprintln!(
        "{}",
        "│  ────────────────────────────────────────────────".dimmed()
    );
    eprintln!(
        "{}",
        format!("│  TOTAL:            {:>8.1}ms", t.total_ms)
            .bold()
            .dimmed()
    );
    eprintln!(
        "{}",
        "└───────────────────────────────────────────────────".dimmed()
    );
    eprintln!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use fallow_core::extract::MemberKind;
    use fallow_core::results::*;
    use std::path::PathBuf;
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
    fn sarif_declares_all_ten_rules() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let sarif = build_sarif(&results, &root, &RulesConfig::default());

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
            locations: vec![root.join("src/a.ts"), root.join("src/b.ts")],
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
        // 10 issues but duplicate_exports has 2 locations => 11 SARIF results
        assert_eq!(entries.len(), 11);

        let rule_ids: Vec<&str> = entries
            .iter()
            .map(|e| e["ruleId"].as_str().unwrap())
            .collect();
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
            is_re_export: false,
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
            is_re_export: false,
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
            path: root.join("package.json"),
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
            path: root.join("package.json"),
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
            kind: MemberKind::EnumMember,
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
            kind: MemberKind::ClassMethod,
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
        assert_eq!(lines[0], "unresolved-import:src/app.ts:3:./missing-module");
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
