use std::path::Path;
use std::time::Duration;

use colored::Colorize;
use fallow_config::{OutputFormat, RulesConfig};
use fallow_core::duplicates::DuplicationReport;
use fallow_core::results::{AnalysisResults, UnusedExport, UnusedMember};
use fallow_core::trace::{CloneTrace, DependencyTrace, ExportTrace, FileTrace, PipelineTimings};

use super::{Level, relative_path, severity_to_level};

pub(super) fn print_human(
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

    let format_dep = |name: &str, pkg_path: &Path| -> String {
        let pkg_label = relative_path(pkg_path, root).display().to_string();
        if pkg_label == "package.json" {
            format!("{}", name.bold())
        } else {
            format!("{} ({})", name.bold(), pkg_label.dimmed())
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
        |dep| vec![format!("  {}", format_dep(&dep.package_name, &dep.path))],
    );

    print_human_section(
        &results.unused_dev_dependencies,
        "Unused devDependencies",
        severity_to_level(rules.unused_dev_dependencies),
        |dep| vec![format!("  {}", format_dep(&dep.package_name, &dep.path))],
    );

    print_human_section(
        &results.unused_optional_dependencies,
        "Unused optionalDependencies",
        severity_to_level(rules.unused_optional_dependencies),
        |dep| vec![format!("  {}", format_dep(&dep.package_name, &dep.path))],
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

    print_human_section(
        &results.type_only_dependencies,
        "Type-only dependencies (consider moving to devDependencies)",
        severity_to_level(rules.type_only_dependencies),
        |dep| vec![format!("  {}", format_dep(&dep.package_name, &dep.path))],
    );

    print_human_section(
        &results.circular_dependencies,
        "Circular dependencies",
        severity_to_level(rules.circular_dependencies),
        |cycle| {
            let chain: Vec<String> = cycle
                .files
                .iter()
                .map(|p| relative_path(p, root).display().to_string())
                .collect();
            // Repeat the first file at the end to make the cycle visually clear
            let mut display_chain = chain.clone();
            if let Some(first) = chain.first() {
                display_chain.push(first.clone());
            }
            vec![format!("  {}", display_chain.join(" \u{2192} "))]
        },
    );

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

// ── Duplication human output ──────────────────────────────────────

pub(super) fn print_duplication_human(
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

// ── Cross-reference findings ──────────────────────────────────────

pub(super) fn print_cross_reference_findings(
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

// ── Trace human output ────────────────────────────────────────────

pub(super) fn print_export_trace_human(trace: &ExportTrace) {
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

pub(super) fn print_file_trace_human(trace: &FileTrace) {
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

pub(super) fn print_dependency_trace_human(trace: &DependencyTrace) {
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

pub(super) fn print_clone_trace_human(trace: &CloneTrace, root: &Path) {
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

// ── Performance human output ──────────────────────────────────────

pub(super) fn print_performance_human(t: &PipelineTimings) {
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
    let cache_detail = if t.cache_hits > 0 {
        format!(", {} cached, {} parsed", t.cache_hits, t.cache_misses)
    } else {
        String::new()
    };
    eprintln!(
        "{}",
        format!(
            "│  parse/extract:    {:>8.1}ms  ({} modules{})",
            t.parse_extract_ms, t.module_count, cache_detail
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
