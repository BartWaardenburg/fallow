use std::fmt::Write as _;
use std::path::Path;
use std::time::Duration;

use colored::Colorize;
use fallow_config::{OutputFormat, RulesConfig};
use fallow_core::duplicates::DuplicationReport;
use fallow_core::results::{AnalysisResults, UnusedExport, UnusedMember};
use fallow_core::trace::{CloneTrace, DependencyTrace, ExportTrace, FileTrace, PipelineTimings};

use super::{Level, elide_common_prefix, relative_path, severity_to_level, split_dir_filename};

/// Maximum items shown per flat section (unused files, deps, etc.).
const MAX_FLAT_ITEMS: usize = 10;
/// Maximum files shown per grouped section (unused exports, types, etc.).
const MAX_GROUPED_FILES: usize = 10;
/// Maximum detail items shown per file within a grouped section.
const MAX_ITEMS_PER_FILE: usize = 5;
/// Maximum clone groups shown in duplication output.
const MAX_CLONE_GROUPS: usize = 10;

/// Format a path with dimmed directory and bold filename.
fn format_path(path_str: &str) -> String {
    let (dir, filename) = split_dir_filename(path_str);
    format!("{}{}", dir.dimmed(), filename.bold())
}

/// Format a number with thousands separators (e.g., 5433 → "5,433").
fn thousands(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(c);
    }
    result
}

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

    // Human output always includes section footers with doc links.
    for line in build_human_lines(results, root, rules) {
        println!("{line}");
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
            let summary = build_summary_footer(results);
            eprintln!(
                "{}",
                format!("\u{2717} {summary} ({:.2}s)", elapsed.as_secs_f64())
                    .red()
                    .bold()
            );
        }
    }
}

/// Build human-readable output lines for analysis results.
///
/// Each section (unused files, exports, etc.) produces a header line followed by
/// detail lines. Empty sections are omitted entirely.
pub(super) fn build_human_lines(
    results: &AnalysisResults,
    root: &Path,
    rules: &RulesConfig,
) -> Vec<String> {
    let mut lines = Vec::new();

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

    build_human_section_ex(
        &mut lines,
        &results.unused_files,
        "Unused files",
        severity_to_level(rules.unused_files),
        |file| {
            let path_str = relative_path(&file.path, root).display().to_string();
            vec![format!("  {}", format_path(&path_str))]
        },
    );

    build_human_grouped_section(
        &mut lines,
        &results.unused_exports,
        "Unused exports",
        severity_to_level(rules.unused_exports),
        root,
        |e| e.path.as_path(),
        &format_export,
    );

    build_human_grouped_section(
        &mut lines,
        &results.unused_types,
        "Unused type exports",
        severity_to_level(rules.unused_types),
        root,
        |e| e.path.as_path(),
        &format_export,
    );

    build_human_section_ex(
        &mut lines,
        &results.unused_dependencies,
        "Unused dependencies",
        severity_to_level(rules.unused_dependencies),
        |dep| vec![format!("  {}", format_dep(&dep.package_name, &dep.path))],
    );

    build_human_section_ex(
        &mut lines,
        &results.unused_dev_dependencies,
        "Unused devDependencies",
        severity_to_level(rules.unused_dev_dependencies),
        |dep| vec![format!("  {}", format_dep(&dep.package_name, &dep.path))],
    );

    build_human_section_ex(
        &mut lines,
        &results.unused_optional_dependencies,
        "Unused optionalDependencies",
        severity_to_level(rules.unused_optional_dependencies),
        |dep| vec![format!("  {}", format_dep(&dep.package_name, &dep.path))],
    );

    build_human_grouped_section(
        &mut lines,
        &results.unused_enum_members,
        "Unused enum members",
        severity_to_level(rules.unused_enum_members),
        root,
        |m| m.path.as_path(),
        &format_member,
    );

    build_human_grouped_section(
        &mut lines,
        &results.unused_class_members,
        "Unused class members",
        severity_to_level(rules.unused_class_members),
        root,
        |m| m.path.as_path(),
        &format_member,
    );

    build_human_grouped_section(
        &mut lines,
        &results.unresolved_imports,
        "Unresolved imports",
        severity_to_level(rules.unresolved_imports),
        root,
        |i| i.path.as_path(),
        &|i| format!("{} {}", format!(":{}", i.line).dimmed(), i.specifier.bold()),
    );

    build_human_section_ex(
        &mut lines,
        &results.unlisted_dependencies,
        "Unlisted dependencies",
        severity_to_level(rules.unlisted_dependencies),
        |dep| vec![format!("  {}", dep.package_name.bold())],
    );

    build_duplicate_exports_section(
        &mut lines,
        &results.duplicate_exports,
        severity_to_level(rules.duplicate_exports),
        root,
    );

    build_human_section_ex(
        &mut lines,
        &results.type_only_dependencies,
        "Type-only dependencies (consider moving to devDependencies)",
        severity_to_level(rules.type_only_dependencies),
        |dep| vec![format!("  {}", format_dep(&dep.package_name, &dep.path))],
    );

    build_circular_deps_section(
        &mut lines,
        &results.circular_dependencies,
        severity_to_level(rules.circular_dependencies),
        root,
    );

    lines
}

/// Docs base URLs.
const DOCS_HEALTH: &str = "https://docs.fallow.tools/explanations/health";
const DOCS_DUPLICATION: &str = "https://docs.fallow.tools/explanations/duplication";

/// Section footer: description + docs URL (with anchor to specific section).
fn section_footer_text(title: &str) -> Option<(&'static str, &'static str)> {
    match title {
        "Unused files" => Some((
            "Files not imported or referenced by any entry point",
            "https://docs.fallow.tools/explanations/dead-code#unused-files",
        )),
        "Unused exports" => Some((
            "Exported symbols not imported by any reachable file",
            "https://docs.fallow.tools/explanations/dead-code#unused-exports",
        )),
        "Unused type exports" => Some((
            "Exported types/interfaces not imported by any reachable file",
            "https://docs.fallow.tools/explanations/dead-code#unused-types",
        )),
        "Unused dependencies" | "Unused devDependencies" | "Unused optionalDependencies" => Some((
            "Packages in package.json not imported anywhere in the project",
            "https://docs.fallow.tools/explanations/dead-code#unused-dependencies",
        )),
        "Unused enum members" => Some((
            "Enum variants never referenced outside their declaration",
            "https://docs.fallow.tools/explanations/dead-code#unused-enum-members",
        )),
        "Unused class members" => Some((
            "Class methods/properties never referenced outside their class",
            "https://docs.fallow.tools/explanations/dead-code#unused-class-members",
        )),
        "Unresolved imports" => Some((
            "Import specifiers that could not be resolved to a file",
            "https://docs.fallow.tools/explanations/dead-code#unresolved-imports",
        )),
        "Unlisted dependencies" => Some((
            "Packages imported in code but missing from package.json",
            "https://docs.fallow.tools/explanations/dead-code#unlisted-dependencies",
        )),
        "Duplicate exports" => Some((
            "Same export name defined in multiple files",
            "https://docs.fallow.tools/explanations/dead-code#duplicate-exports",
        )),
        "Circular dependencies" => Some((
            "Files that import each other, forming dependency cycles",
            "https://docs.fallow.tools/explanations/dead-code#circular-dependencies",
        )),
        t if t.starts_with("Type-only") => Some((
            "Dependencies only used for type imports, safe to move to devDependencies",
            "https://docs.fallow.tools/explanations/dead-code#type-only-dependencies",
        )),
        _ => None,
    }
}

/// Push a dimmed section footer line: description — docs_url
fn push_section_footer(lines: &mut Vec<String>, title: &str) {
    if let Some((desc, url)) = section_footer_text(title) {
        lines.push(format!("  {}", format!("{desc} \u{2014} {url}").dimmed()));
    }
}

/// Append a non-empty section with a header, doc-link footer, and truncated items.
fn build_human_section_ex<T>(
    lines: &mut Vec<String>,
    items: &[T],
    title: &str,
    level: Level,
    format_lines: impl Fn(&T) -> Vec<String>,
) {
    if items.is_empty() {
        return;
    }
    lines.push(build_section_header(title, items.len(), level));
    let shown = items.len().min(MAX_FLAT_ITEMS);
    for item in &items[..shown] {
        for line in format_lines(item) {
            lines.push(line);
        }
    }
    if items.len() > MAX_FLAT_ITEMS {
        lines.push(format!(
            "  {}",
            format!("... and {} more", items.len() - MAX_FLAT_ITEMS).dimmed()
        ));
    }
    push_section_footer(lines, title);
    lines.push(String::new());
}

/// Append a non-empty section whose items are grouped by file path (truncated).
///
/// Files are sorted by item count descending. Shows `(N exports)` next to each
/// file header. Truncates to `MAX_GROUPED_FILES` files and `MAX_ITEMS_PER_FILE`
/// items per file.
fn build_human_grouped_section<'a, T>(
    lines: &mut Vec<String>,
    items: &'a [T],
    title: &str,
    level: Level,
    root: &Path,
    get_path: impl Fn(&'a T) -> &'a Path,
    format_detail: &impl Fn(&T) -> String,
) {
    if items.is_empty() {
        return;
    }
    lines.push(build_section_header(title, items.len(), level));
    build_grouped_by_file(lines, items, root, get_path, format_detail);
    push_section_footer(lines, title);
    lines.push(String::new());
}

fn build_section_header(title: &str, count: usize, level: Level) -> String {
    let label = format!("{title} ({count})");
    match level {
        Level::Warn => format!("{} {}", "\u{25cf}".yellow(), label.yellow().bold()),
        Level::Info => format!("{} {}", "\u{25cf}".cyan(), label.cyan().bold()),
        Level::Error => format!("{} {}", "\u{25cf}".red(), label.red().bold()),
    }
}

/// Build items grouped by file path, sorted by count descending, with truncation.
fn build_grouped_by_file<'a, T>(
    lines: &mut Vec<String>,
    items: &'a [T],
    root: &Path,
    get_path: impl Fn(&'a T) -> &'a Path,
    format_detail: &impl Fn(&T) -> String,
) {
    // Group items by file path, preserving indices
    let mut file_groups: Vec<(String, Vec<usize>)> = Vec::new();
    let mut file_map: rustc_hash::FxHashMap<String, usize> = rustc_hash::FxHashMap::default();

    for (i, item) in items.iter().enumerate() {
        let file_str = relative_path(get_path(item), root).display().to_string();
        if let Some(&group_idx) = file_map.get(&file_str) {
            file_groups[group_idx].1.push(i);
        } else {
            file_map.insert(file_str.clone(), file_groups.len());
            file_groups.push((file_str, vec![i]));
        }
    }

    // Sort files by item count descending, alphabetical tiebreaker
    file_groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));

    let total_files = file_groups.len();
    let shown_files = total_files.min(MAX_GROUPED_FILES);

    for (file_str, indices) in &file_groups[..shown_files] {
        let count_tag = if indices.len() > 1 {
            format!(" ({})", indices.len()).dimmed().to_string()
        } else {
            String::new()
        };
        lines.push(format!("  {}{}", format_path(file_str), count_tag));

        let shown_items = indices.len().min(MAX_ITEMS_PER_FILE);
        for &i in &indices[..shown_items] {
            lines.push(format!("    {}", format_detail(&items[i])));
        }
        if indices.len() > MAX_ITEMS_PER_FILE {
            lines.push(format!(
                "    {}",
                format!("... and {} more", indices.len() - MAX_ITEMS_PER_FILE).dimmed()
            ));
        }
    }

    if total_files > MAX_GROUPED_FILES {
        let hidden_files = total_files - MAX_GROUPED_FILES;
        let hidden_items: usize = file_groups[MAX_GROUPED_FILES..]
            .iter()
            .map(|(_, indices)| indices.len())
            .sum();
        lines.push(format!(
            "  {}",
            format!(
                "... and {} more in {} file{}",
                hidden_items,
                hidden_files,
                if hidden_files == 1 { "" } else { "s" }
            )
            .dimmed()
        ));
    }
}

/// Build duplicate exports grouped by file pair instead of flat list.
fn build_duplicate_exports_section(
    lines: &mut Vec<String>,
    items: &[fallow_core::results::DuplicateExport],
    level: Level,
    root: &Path,
) {
    if items.is_empty() {
        return;
    }
    let title = "Duplicate exports";
    lines.push(build_section_header(title, items.len(), level));

    // Group by sorted file-pair key
    let mut pair_groups: Vec<(String, String, Vec<&str>)> = Vec::new();
    let mut pair_map: rustc_hash::FxHashMap<(String, String), usize> =
        rustc_hash::FxHashMap::default();

    for dup in items {
        if dup.locations.len() < 2 {
            continue;
        }
        let mut paths: Vec<String> = dup
            .locations
            .iter()
            .map(|loc| relative_path(&loc.path, root).display().to_string())
            .collect();
        paths.sort();
        paths.dedup();

        // For multi-file duplicates, pair the first two
        let key = (paths[0].clone(), paths.get(1).cloned().unwrap_or_default());
        if let Some(&group_idx) = pair_map.get(&key) {
            pair_groups[group_idx].2.push(&dup.export_name);
        } else {
            pair_map.insert(key, pair_groups.len());
            pair_groups.push((
                paths[0].clone(),
                paths.get(1).cloned().unwrap_or_default(),
                vec![&dup.export_name],
            ));
        }
    }

    // Sort by count descending
    pair_groups.sort_by(|a, b| b.2.len().cmp(&a.2.len()));

    let shown = pair_groups.len().min(MAX_FLAT_ITEMS);
    for (file_a, file_b, exports) in &pair_groups[..shown] {
        let export_list = if exports.len() <= 5 {
            exports
                .iter()
                .map(|e| e.bold().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            let mut display: Vec<String> =
                exports[..5].iter().map(|e| e.bold().to_string()).collect();
            display.push(format!("... +{}", exports.len() - 5).dimmed().to_string());
            display.join(", ")
        };

        // Vertical layout: file_a on line 1, ↔ file_b on line 2, exports on line 3
        let elided_b = elide_common_prefix(file_a, file_b);
        lines.push(format!("  {}", format_path(file_a)));
        lines.push(format!(
            "    {} {} ({} export{})",
            "\u{2194}".dimmed(),
            format_path(elided_b),
            exports.len(),
            if exports.len() == 1 { "" } else { "s" }
        ));
        lines.push(format!("    {export_list}"));
        lines.push(String::new());
    }

    if pair_groups.len() > MAX_FLAT_ITEMS {
        lines.push(format!(
            "  {}",
            format!(
                "... and {} more pair{}",
                pair_groups.len() - MAX_FLAT_ITEMS,
                if pair_groups.len() - MAX_FLAT_ITEMS == 1 {
                    ""
                } else {
                    "s"
                }
            )
            .dimmed()
        ));
    }
    push_section_footer(lines, title);
    lines.push(String::new());
}

/// Build circular dependencies grouped by hub file with path elision.
fn build_circular_deps_section(
    lines: &mut Vec<String>,
    items: &[fallow_core::results::CircularDependency],
    level: Level,
    root: &Path,
) {
    if items.is_empty() {
        return;
    }
    let title = "Circular dependencies";
    lines.push(build_section_header(title, items.len(), level));

    // Group cycles by their first file (hub)
    let mut hub_groups: Vec<(String, Vec<&fallow_core::results::CircularDependency>)> = Vec::new();
    let mut hub_map: rustc_hash::FxHashMap<String, usize> = rustc_hash::FxHashMap::default();

    for cycle in items {
        let hub = cycle
            .files
            .first()
            .map(|p| relative_path(p, root).display().to_string())
            .unwrap_or_default();
        if let Some(&idx) = hub_map.get(&hub) {
            hub_groups[idx].1.push(cycle);
        } else {
            hub_map.insert(hub.clone(), hub_groups.len());
            hub_groups.push((hub, vec![cycle]));
        }
    }

    // Sort by cycle count descending, alphabetical tiebreaker
    hub_groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));

    let shown = hub_groups.len().min(MAX_FLAT_ITEMS);
    for (hub_path, cycles) in &hub_groups[..shown] {
        let count_tag = if cycles.len() > 1 {
            format!(" ({} cycles)", cycles.len()).dimmed().to_string()
        } else {
            String::new()
        };
        lines.push(format!("  {}{}", format_path(hub_path), count_tag));

        for cycle in cycles {
            let rel_paths: Vec<String> = cycle
                .files
                .iter()
                .map(|p| relative_path(p, root).display().to_string())
                .collect();

            // Build chain: elide common prefix with hub, add closing return to hub
            let mut chain_parts: Vec<String> = Vec::new();
            for path in &rel_paths[1..] {
                let elided = elide_common_prefix(hub_path, path);
                chain_parts.push(format_path(elided));
            }
            // Close the cycle back to hub filename
            let (_, hub_filename) = split_dir_filename(hub_path);
            chain_parts.push(hub_filename.bold().to_string());

            lines.push(format!(
                "    {} {}",
                "\u{2192}".dimmed(),
                chain_parts.join(&format!(" {} ", "\u{2192}".dimmed()))
            ));
        }
        lines.push(String::new());
    }

    if hub_groups.len() > MAX_FLAT_ITEMS {
        let hidden: usize = hub_groups[MAX_FLAT_ITEMS..]
            .iter()
            .map(|(_, cycles)| cycles.len())
            .sum();
        lines.push(format!("  {}", format!("... and {hidden} more").dimmed()));
        lines.push(String::new());
    }
    push_section_footer(lines, title);
    if !lines.last().is_some_and(|l| l.is_empty()) {
        lines.push(String::new());
    }
}

/// Build a one-line summary footer showing counts per issue type.
fn build_summary_footer(results: &AnalysisResults) -> String {
    let mut parts = Vec::new();
    let mut add = |count: usize, label: &str| {
        if count > 0 {
            let mut s = String::new();
            let _ = write!(s, "{count} {label}");
            if count != 1 && !label.ends_with('s') {
                s.push('s');
            }
            parts.push(s);
        }
    };

    add(results.unused_files.len(), "file");
    add(results.unused_exports.len(), "export");
    add(results.unused_types.len(), "type");
    add(
        results.unused_dependencies.len()
            + results.unused_dev_dependencies.len()
            + results.unused_optional_dependencies.len(),
        "dep",
    );
    add(results.unused_enum_members.len(), "enum");
    add(results.unused_class_members.len(), "class");
    add(results.unresolved_imports.len(), "unresolved");
    add(results.unlisted_dependencies.len(), "unlisted");
    add(results.duplicate_exports.len(), "duplicate");
    add(results.type_only_dependencies.len(), "type-only");
    add(results.circular_dependencies.len(), "circular");

    parts.join(" \u{00b7} ")
}

// ── Health / complexity human output ──────────────────────────────

pub(super) fn print_health_human(
    report: &crate::health_types::HealthReport,
    root: &Path,
    elapsed: Duration,
    quiet: bool,
) {
    if !quiet {
        eprintln!();
    }

    if report.findings.is_empty() && report.file_scores.is_empty() && report.hotspots.is_empty() {
        if !quiet {
            eprintln!(
                "{}",
                format!(
                    "\u{2713} No functions exceed complexity thresholds ({:.2}s)",
                    elapsed.as_secs_f64()
                )
                .green()
                .bold()
            );
            eprintln!(
                "{}",
                format!(
                    "  {} functions analyzed (max cyclomatic: {}, max cognitive: {})",
                    report.summary.functions_analyzed,
                    report.summary.max_cyclomatic_threshold,
                    report.summary.max_cognitive_threshold,
                )
                .dimmed()
            );
        }
        return;
    }

    for line in build_health_human_lines(report, root) {
        println!("{line}");
    }

    if !quiet {
        let s = &report.summary;
        let mut parts = Vec::new();
        parts.push(format!("{} above threshold", s.functions_above_threshold));
        parts.push(format!("{} analyzed", s.functions_analyzed));
        if let Some(avg) = s.average_maintainability {
            parts.push(format!("MI {avg:.1}"));
        }
        eprintln!(
            "{}",
            format!(
                "\u{2717} {} ({:.2}s)",
                parts.join(" \u{00b7} "),
                elapsed.as_secs_f64()
            )
            .red()
            .bold()
        );
    }
}

/// Build human-readable output lines for health (complexity) findings.
pub(super) fn build_health_human_lines(
    report: &crate::health_types::HealthReport,
    root: &Path,
) -> Vec<String> {
    let mut lines = Vec::new();

    if !report.findings.is_empty() {
        lines.push(format!(
            "{} {}",
            "\u{25cf}".red(),
            if report.findings.len() < report.summary.functions_above_threshold {
                format!(
                    "High complexity functions ({} shown, {} total)",
                    report.findings.len(),
                    report.summary.functions_above_threshold
                )
            } else {
                format!(
                    "High complexity functions ({})",
                    report.summary.functions_above_threshold
                )
            }
            .red()
            .bold()
        ));
    }

    let mut last_file = String::new();
    for finding in &report.findings {
        let file_str = relative_path(&finding.path, root).display().to_string();
        if file_str != last_file {
            lines.push(format!("  {}", format_path(&file_str)));
            last_file = file_str;
        }

        let cyc_val = format!("{:>3}", finding.cyclomatic);
        let cog_val = format!("{:>3}", finding.cognitive);

        let cyc_colored = if finding.cyclomatic > report.summary.max_cyclomatic_threshold {
            cyc_val.red().bold().to_string()
        } else {
            cyc_val.dimmed().to_string()
        };
        let cog_colored = if finding.cognitive > report.summary.max_cognitive_threshold {
            cog_val.red().bold().to_string()
        } else {
            cog_val.dimmed().to_string()
        };

        // Line 1: function name
        lines.push(format!(
            "    {} {}",
            format!(":{}", finding.line).dimmed(),
            finding.name.bold(),
        ));
        // Line 2: metrics (indented, aligned like hotspots)
        lines.push(format!(
            "         {} cyclomatic  {} cognitive  {} lines",
            cyc_colored,
            cog_colored,
            format!("{:>3}", finding.line_count).dimmed(),
        ));
    }
    if !report.findings.is_empty() {
        lines.push(format!(
            "  {}",
            format!(
                "Functions exceeding cyclomatic or cognitive complexity thresholds \u{2014} {DOCS_HEALTH}#complexity-metrics"
            )
            .dimmed()
        ));
        lines.push(String::new());
    }

    // File health scores (truncated)
    if !report.file_scores.is_empty() {
        lines.push(format!(
            "{} {}",
            "\u{25cf}".cyan(),
            format!("File health scores ({} files)", report.file_scores.len())
                .cyan()
                .bold()
        ));
        lines.push(String::new());

        let shown_scores = report.file_scores.len().min(MAX_FLAT_ITEMS);
        for score in &report.file_scores[..shown_scores] {
            let file_str = relative_path(&score.path, root).display().to_string();
            let mi = score.maintainability_index;

            // MI score: color-coded by quality
            let mi_str = format!("{mi:>5.1}");
            let mi_colored = if mi >= 80.0 {
                mi_str.green().to_string()
            } else if mi >= 50.0 {
                mi_str.yellow().to_string()
            } else {
                mi_str.red().bold().to_string()
            };

            // Path: dim directory, normal filename
            let (dir, filename) = split_dir_filename(&file_str);

            // Line 1: MI score + path
            lines.push(format!("  {}    {}{}", mi_colored, dir.dimmed(), filename,));

            // Line 2: metrics (indented, dimmed)
            lines.push(format!(
                "         {} fan-in  {} fan-out  {} dead  {} density",
                format!("{:>3}", score.fan_in).dimmed(),
                format!("{:>3}", score.fan_out).dimmed(),
                format!("{:>3.0}%", score.dead_code_ratio * 100.0).dimmed(),
                format!("{:.2}", score.complexity_density).dimmed(),
            ));

            // Blank line between entries
            lines.push(String::new());
        }
        if report.file_scores.len() > MAX_FLAT_ITEMS {
            lines.push(format!(
                "  {}",
                format!(
                    "... and {} more files",
                    report.file_scores.len() - MAX_FLAT_ITEMS
                )
                .dimmed()
            ));
            lines.push(String::new());
        }
        lines.push(format!(
            "  {}",
            format!("Composite file quality scores based on complexity, coupling, and dead code \u{2014} {DOCS_HEALTH}#file-health-scores").dimmed()
        ));
        lines.push(String::new());
    }

    // Hotspots
    if !report.hotspots.is_empty() {
        let header = if let Some(ref summary) = report.hotspot_summary {
            format!(
                "Hotspots ({} files, since {})",
                report.hotspots.len(),
                summary.since,
            )
        } else {
            format!("Hotspots ({} files)", report.hotspots.len())
        };
        lines.push(format!("{} {}", "\u{25cf}".red(), header.red().bold()));
        lines.push(String::new());

        for entry in &report.hotspots {
            let file_str = relative_path(&entry.path, root).display().to_string();

            // Score: color-coded by severity
            let score_str = format!("{:>5.1}", entry.score);
            let score_colored = if entry.score >= 70.0 {
                score_str.red().bold().to_string()
            } else if entry.score >= 30.0 {
                score_str.yellow().to_string()
            } else {
                score_str.green().to_string()
            };

            // Trend: symbol + color
            let (trend_symbol, trend_colored) = match entry.trend {
                fallow_core::churn::ChurnTrend::Accelerating => {
                    ("\u{25b2}", "\u{25b2} accelerating".red().to_string())
                }
                fallow_core::churn::ChurnTrend::Cooling => {
                    ("\u{25bc}", "\u{25bc} cooling".green().to_string())
                }
                fallow_core::churn::ChurnTrend::Stable => {
                    ("\u{2500}", "\u{2500} stable".dimmed().to_string())
                }
            };

            // Path: dim directory, normal filename
            let (dir, filename) = split_dir_filename(&file_str);

            // Line 1: score + trend symbol + path
            lines.push(format!(
                "  {} {}  {}{}",
                score_colored,
                match entry.trend {
                    fallow_core::churn::ChurnTrend::Accelerating => trend_symbol.red().to_string(),
                    fallow_core::churn::ChurnTrend::Cooling => trend_symbol.green().to_string(),
                    fallow_core::churn::ChurnTrend::Stable => trend_symbol.dimmed().to_string(),
                },
                dir.dimmed(),
                filename,
            ));

            // Line 2: metrics (indented, dimmed) + trend label
            lines.push(format!(
                "         {} commits  {} churn  {} density  {} fan-in  {}",
                format!("{:>3}", entry.commits).dimmed(),
                format!("{:>5}", entry.lines_added + entry.lines_deleted).dimmed(),
                format!("{:.2}", entry.complexity_density).dimmed(),
                format!("{:>2}", entry.fan_in).dimmed(),
                trend_colored,
            ));

            // Blank line between entries
            lines.push(String::new());
        }

        if let Some(ref summary) = report.hotspot_summary
            && summary.files_excluded > 0
        {
            lines.push(format!(
                "  {}",
                format!(
                    "{} file{} excluded (< {} commits)",
                    summary.files_excluded,
                    if summary.files_excluded == 1 { "" } else { "s" },
                    summary.min_commits,
                )
                .dimmed()
            ));
            lines.push(String::new());
        }
        lines.push(format!(
            "  {}",
            format!(
                "Files with high churn and high complexity \u{2014} {DOCS_HEALTH}#hotspot-metrics"
            )
            .dimmed()
        ));
        lines.push(String::new());
    }

    lines
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

    for line in build_duplication_human_lines(report, root) {
        println!("{line}");
    }

    let stats = &report.stats;
    if !quiet {
        eprintln!(
            "{}",
            format!(
                "\u{2717} {} lines ({:.1}%) duplicated across {} file{} ({:.2}s)",
                thousands(stats.duplicated_lines),
                stats.duplication_percentage,
                stats.files_with_clones,
                if stats.files_with_clones == 1 {
                    ""
                } else {
                    "s"
                },
                elapsed.as_secs_f64(),
            )
            .red()
            .bold()
        );
    }
}

/// Build human-readable output lines for duplication report.
pub(super) fn build_duplication_human_lines(
    report: &DuplicationReport,
    root: &Path,
) -> Vec<String> {
    let mut lines = Vec::new();

    if report.clone_groups.is_empty() && report.clone_families.is_empty() {
        return lines;
    }

    // Sort clone groups by line count descending for most impactful first
    let mut sorted_groups: Vec<&fallow_core::duplicates::CloneGroup> =
        report.clone_groups.iter().collect();
    sorted_groups.sort_by(|a, b| b.line_count.cmp(&a.line_count));

    let total_groups = sorted_groups.len();
    let shown = total_groups.min(MAX_CLONE_GROUPS);

    lines.push(format!(
        "{} {}",
        "\u{25cf}".cyan(),
        format!("Duplicates ({total_groups} clone groups)")
            .cyan()
            .bold()
    ));
    lines.push(String::new());

    for group in &sorted_groups[..shown] {
        let instance_count = group.instances.len();

        // Line count: right-aligned, color-coded
        let lc = group.line_count;
        let lc_str = format!("{:>5}", thousands(lc));
        let lc_colored = if lc > 1000 {
            lc_str.red().bold().to_string()
        } else if lc > 100 {
            lc_str.yellow().to_string()
        } else {
            lc_str.dimmed().to_string()
        };

        lines.push(format!(
            "  {} lines  {} instance{}",
            lc_colored,
            instance_count,
            if instance_count == 1 { "" } else { "s" }
        ));

        for instance in &group.instances {
            let relative = relative_path(&instance.file, root);
            let path_str = relative.display().to_string();
            let (dir, filename) = split_dir_filename(&path_str);
            lines.push(format!(
                "    {}{}:{}-{}",
                dir.dimmed(),
                filename,
                instance.start_line,
                instance.end_line
            ));
        }
        lines.push(String::new());
    }

    if total_groups > MAX_CLONE_GROUPS {
        lines.push(format!(
            "  {}",
            format!(
                "... and {} more clone groups",
                total_groups - MAX_CLONE_GROUPS
            )
            .dimmed()
        ));
    }
    lines.push(format!(
        "  {}",
        format!("Identical code blocks detected via suffix-array analysis \u{2014} {DOCS_DUPLICATION}#clone-groups").dimmed()
    ));
    lines.push(String::new());

    // Detect mirrored directory patterns across families.
    // Families with exactly 2 files that share a common filename after stripping
    // directory prefixes are grouped under a "Mirrored directories" header.
    let (mirrored, non_mirrored) = detect_mirrored_families(&report.clone_families, root);

    if !mirrored.is_empty() {
        let shown_mirrors = mirrored.len().min(MAX_FLAT_ITEMS);
        for mirror in &mirrored[..shown_mirrors] {
            lines.push(format!(
                "{} {}",
                "\u{25cf}".yellow(),
                format!(
                    "Mirrored: {} \u{2194} {} ({} files, {} lines)",
                    mirror.dir_a,
                    mirror.dir_b,
                    mirror.file_count,
                    thousands(mirror.total_lines),
                )
                .yellow()
                .bold()
            ));

            let shown = mirror.files.len().min(MAX_FLAT_ITEMS);
            for filename in &mirror.files[..shown] {
                lines.push(format!("  {}", filename.dimmed()));
            }
            if mirror.files.len() > MAX_FLAT_ITEMS {
                lines.push(format!(
                    "  {}",
                    format!("... and {} more", mirror.files.len() - MAX_FLAT_ITEMS).dimmed()
                ));
            }
            lines.push(String::new());
        }
        if mirrored.len() > MAX_FLAT_ITEMS {
            lines.push(format!(
                "  {}",
                format!(
                    "... and {} more mirrored pairs",
                    mirrored.len() - MAX_FLAT_ITEMS
                )
                .dimmed()
            ));
            lines.push(String::new());
        }
        lines.push(format!(
            "  {}",
            format!("Directories containing identical file copies \u{2014} {DOCS_DUPLICATION}#clone-families").dimmed()
        ));
        lines.push(String::new());
    }

    // Print remaining clone families with refactoring suggestions
    // Suppress single-group families — not actionable
    let multi_group_families: Vec<_> = non_mirrored.iter().filter(|f| f.groups.len() > 1).collect();

    if !multi_group_families.is_empty() {
        lines.push(format!(
            "{} {}",
            "\u{25cf}".yellow(),
            format!(
                "Clone families ({} with multiple groups)",
                multi_group_families.len()
            )
            .yellow()
            .bold()
        ));
        lines.push(String::new());

        let shown_families = multi_group_families.len().min(MAX_FLAT_ITEMS);
        for family in &multi_group_families[..shown_families] {
            let file_names: Vec<_> = family
                .files
                .iter()
                .map(|f| {
                    let path_str = relative_path(f, root).display().to_string();
                    format_path(&path_str)
                })
                .collect();

            lines.push(format!(
                "  {} groups, {} lines across {}",
                family.groups.len().to_string().bold(),
                thousands(family.total_duplicated_lines).bold(),
                file_names.join(", "),
            ));

            for suggestion in &family.suggestions {
                // Drop "lines saved" — misleading
                lines.push(format!(
                    "    {} {}",
                    "\u{2192}".yellow(),
                    suggestion.description.dimmed(),
                ));
            }
            lines.push(String::new());
        }
        if multi_group_families.len() > MAX_FLAT_ITEMS {
            lines.push(format!(
                "  {}",
                format!(
                    "... and {} more families",
                    multi_group_families.len() - MAX_FLAT_ITEMS
                )
                .dimmed()
            ));
            lines.push(String::new());
        }
        lines.push(format!(
            "  {}",
            format!("Groups of related clones across the same files \u{2014} {DOCS_DUPLICATION}#clone-families").dimmed()
        ));
        lines.push(String::new());
    }

    lines
}

/// A detected mirrored directory pattern: two directory prefixes that contain
/// identical files (e.g., `src/` and `deno/lib/`).
struct MirroredDirs {
    dir_a: String,
    dir_b: String,
    files: Vec<String>,
    file_count: usize,
    total_lines: usize,
}

/// Detect mirrored directory patterns in clone families.
///
/// Scans families with exactly 2 files. If multiple families share the same
/// directory prefix pair (after stripping to the common filename), they're
/// grouped into a `MirroredDirs`. Families that don't match any mirror pattern
/// are returned as non-mirrored.
///
/// Minimum 3 families must share a pattern to qualify as "mirrored".
fn detect_mirrored_families<'a>(
    families: &'a [fallow_core::duplicates::CloneFamily],
    root: &Path,
) -> (
    Vec<MirroredDirs>,
    Vec<&'a fallow_core::duplicates::CloneFamily>,
) {
    const MIN_MIRROR_FAMILIES: usize = 3;

    // For each 2-file family, extract the directory pair + relative filename
    // Entry: (family_index, filename, duplicated_lines)
    type MirrorEntry = (usize, String, usize);
    let mut pair_map: rustc_hash::FxHashMap<(String, String), Vec<MirrorEntry>> =
        rustc_hash::FxHashMap::default();

    for (idx, family) in families.iter().enumerate() {
        if family.files.len() != 2 {
            continue;
        }
        let path_a = relative_path(&family.files[0], root).display().to_string();
        let path_b = relative_path(&family.files[1], root).display().to_string();

        let (dir_a, file_a) = split_dir_filename(&path_a);
        let (dir_b, file_b) = split_dir_filename(&path_b);

        // Only match if the filenames are the same
        if file_a != file_b {
            continue;
        }

        // Normalize: always use the lexically smaller dir first
        let (da, db) = if dir_a <= dir_b {
            (dir_a.to_string(), dir_b.to_string())
        } else {
            (dir_b.to_string(), dir_a.to_string())
        };

        pair_map.entry((da, db)).or_default().push((
            idx,
            file_a.to_string(),
            family.total_duplicated_lines,
        ));
    }

    let mut mirrored_indices: rustc_hash::FxHashSet<usize> = rustc_hash::FxHashSet::default();
    let mut mirrors: Vec<MirroredDirs> = Vec::new();

    for ((dir_a, dir_b), entries) in &pair_map {
        if entries.len() < MIN_MIRROR_FAMILIES {
            continue;
        }
        for &(idx, _, _) in entries {
            mirrored_indices.insert(idx);
        }
        let total_lines: usize = entries.iter().map(|&(_, _, lines)| lines).sum();
        let mut files: Vec<String> = entries.iter().map(|(_, f, _)| f.clone()).collect();
        files.sort();
        let file_count = files.len();
        mirrors.push(MirroredDirs {
            dir_a: dir_a.clone(),
            dir_b: dir_b.clone(),
            files,
            file_count,
            total_lines,
        });
    }

    // Sort mirrors by total lines descending
    mirrors.sort_by(|a, b| b.total_lines.cmp(&a.total_lines));

    let non_mirrored: Vec<&fallow_core::duplicates::CloneFamily> = families
        .iter()
        .enumerate()
        .filter(|(idx, _)| !mirrored_indices.contains(idx))
        .map(|(_, f)| f)
        .collect();

    (mirrors, non_mirrored)
}

// ── Cross-reference findings ──────────────────────────────────────

pub(super) fn print_cross_reference_findings(
    cross_ref: &fallow_core::cross_reference::CrossReferenceResult,
    root: &Path,
    quiet: bool,
    output: &OutputFormat,
) {
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

    for line in build_cross_reference_lines(cross_ref, root) {
        println!("{line}");
    }

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

/// Build human-readable output lines for cross-reference findings.
pub(super) fn build_cross_reference_lines(
    cross_ref: &fallow_core::cross_reference::CrossReferenceResult,
    root: &Path,
) -> Vec<String> {
    use fallow_core::cross_reference::DeadCodeKind;

    let mut lines = Vec::new();

    if cross_ref.combined_findings.is_empty() {
        return lines;
    }

    lines.push(String::new());
    lines.push(format!(
        "{} {}",
        "\u{25cf}".yellow(),
        "Duplicated + Unused (safe to delete)".yellow().bold()
    ));
    lines.push(String::new());

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

        lines.push(format!(
            "  {} {}",
            location.bold(),
            format!("({reason})").dimmed()
        ));
    }

    lines.push(String::new());
    lines
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
    for line in build_performance_human_lines(t) {
        eprintln!("{line}");
    }
}

/// Build human-readable output lines for pipeline performance timings.
pub(super) fn build_performance_human_lines(t: &PipelineTimings) -> Vec<String> {
    let mut lines = Vec::new();

    lines.push(String::new());
    lines.push(
        "┌─ Pipeline Performance ─────────────────────────────"
            .dimmed()
            .to_string(),
    );
    lines.push(
        format!(
            "│  discover files:   {:>8.1}ms  ({} files)",
            t.discover_files_ms, t.file_count
        )
        .dimmed()
        .to_string(),
    );
    lines.push(
        format!(
            "│  workspaces:       {:>8.1}ms  ({} workspaces)",
            t.workspaces_ms, t.workspace_count
        )
        .dimmed()
        .to_string(),
    );
    lines.push(
        format!("│  plugins:          {:>8.1}ms", t.plugins_ms)
            .dimmed()
            .to_string(),
    );
    lines.push(
        format!("│  script analysis:  {:>8.1}ms", t.script_analysis_ms)
            .dimmed()
            .to_string(),
    );
    let cache_detail = if t.cache_hits > 0 {
        format!(", {} cached, {} parsed", t.cache_hits, t.cache_misses)
    } else {
        String::new()
    };
    lines.push(
        format!(
            "│  parse/extract:    {:>8.1}ms  ({} modules{})",
            t.parse_extract_ms, t.module_count, cache_detail
        )
        .dimmed()
        .to_string(),
    );
    lines.push(
        format!("│  cache update:     {:>8.1}ms", t.cache_update_ms)
            .dimmed()
            .to_string(),
    );
    lines.push(
        format!(
            "│  entry points:     {:>8.1}ms  ({} entries)",
            t.entry_points_ms, t.entry_point_count
        )
        .dimmed()
        .to_string(),
    );
    lines.push(
        format!("│  resolve imports:  {:>8.1}ms", t.resolve_imports_ms)
            .dimmed()
            .to_string(),
    );
    lines.push(
        format!("│  build graph:      {:>8.1}ms", t.build_graph_ms)
            .dimmed()
            .to_string(),
    );
    lines.push(
        format!("│  analyze:          {:>8.1}ms", t.analyze_ms)
            .dimmed()
            .to_string(),
    );
    lines.push(
        "│  ────────────────────────────────────────────────"
            .dimmed()
            .to_string(),
    );
    lines.push(
        format!("│  TOTAL:            {:>8.1}ms", t.total_ms)
            .bold()
            .dimmed()
            .to_string(),
    );
    lines.push(
        "└───────────────────────────────────────────────────"
            .dimmed()
            .to_string(),
    );
    lines.push(String::new());

    lines
}

/// Strip ANSI escape sequences from a string, leaving only the printable text.
#[cfg(test)]
fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip until 'm' (end of SGR sequence)
            for inner in chars.by_ref() {
                if inner == 'm' {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use fallow_config::{RulesConfig, Severity};
    use fallow_core::cross_reference::{CombinedFinding, CrossReferenceResult, DeadCodeKind};
    use fallow_core::duplicates::{
        CloneFamily, CloneGroup, CloneInstance, DuplicationReport, DuplicationStats,
        RefactoringKind, RefactoringSuggestion,
    };
    use fallow_core::extract::MemberKind;
    use fallow_core::results::*;
    use std::path::PathBuf;

    /// Strip ANSI codes from all lines and join with newlines for easy assertion.
    fn plain(lines: &[String]) -> String {
        lines
            .iter()
            .map(|l| strip_ansi(l))
            .collect::<Vec<_>>()
            .join("\n")
    }

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
        r.unused_optional_dependencies.push(UnusedDependency {
            package_name: "fsevents".to_string(),
            location: DependencyLocation::OptionalDependencies,
            path: root.join("package.json"),
            line: 10,
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

    // ── Empty results ──

    #[test]
    fn empty_results_produce_no_lines() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        assert!(lines.is_empty());
    }

    // ── Section headers contain title and count ──

    #[test]
    fn section_headers_contain_title_and_count() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);

        assert!(text.contains("Unused files (1)"));
        assert!(text.contains("Unused exports (1)"));
        assert!(text.contains("Unused type exports (1)"));
        assert!(text.contains("Unused dependencies (1)"));
        assert!(text.contains("Unused devDependencies (1)"));
        assert!(text.contains("Unused optionalDependencies (1)"));
        assert!(text.contains("Unused enum members (1)"));
        assert!(text.contains("Unused class members (1)"));
        assert!(text.contains("Unresolved imports (1)"));
        assert!(text.contains("Unlisted dependencies (1)"));
        assert!(text.contains("Duplicate exports (1)"));
        assert!(text.contains("Type-only dependencies (consider moving to devDependencies) (1)"));
        assert!(text.contains("Circular dependencies (1)"));
    }

    // ── Multiple items show correct counts ──

    #[test]
    fn section_header_shows_correct_count_for_multiple_items() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        for i in 0..5 {
            results.unused_files.push(UnusedFile {
                path: root.join(format!("src/dead{i}.ts")),
            });
        }
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        assert!(text.contains("Unused files (5)"));
    }

    // ── Unused files display relative paths ──

    #[test]
    fn unused_files_show_relative_paths() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/components/Button.tsx"),
        });
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        assert!(text.contains("src/components/Button.tsx"));
        assert!(!text.contains("/project/"));
    }

    // ── Unused exports show file grouping, line, and name ──

    #[test]
    fn unused_exports_grouped_by_file_with_line_and_name() {
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
        results.unused_exports.push(UnusedExport {
            path: root.join("src/utils.ts"),
            export_name: "anotherFn".to_string(),
            is_type_only: false,
            line: 25,
            col: 0,
            span_start: 300,
            is_re_export: false,
        });
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);

        // Count of 2 in header
        assert!(text.contains("Unused exports (2)"));
        // File path appears as group header
        assert!(text.contains("src/utils.ts"));
        // Both export names appear
        assert!(text.contains(":10 helperFn"));
        assert!(text.contains(":25 anotherFn"));
    }

    // ── Re-exports are tagged ──

    #[test]
    fn re_exports_are_tagged() {
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
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        assert!(text.contains("(re-export)"));
    }

    #[test]
    fn non_re_exports_have_no_tag() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_exports.push(UnusedExport {
            path: root.join("src/utils.ts"),
            export_name: "helper".to_string(),
            is_type_only: false,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        assert!(!text.contains("(re-export)"));
    }

    // ── Unused members show parent.member format ──

    #[test]
    fn unused_enum_members_show_parent_dot_member() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_enum_members.push(UnusedMember {
            path: root.join("src/enums.ts"),
            parent_name: "Color".to_string(),
            member_name: "Purple".to_string(),
            kind: MemberKind::EnumMember,
            line: 5,
            col: 2,
        });
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        assert!(text.contains("Color.Purple"));
        assert!(text.contains(":5"));
    }

    #[test]
    fn unused_class_members_show_parent_dot_member() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_class_members.push(UnusedMember {
            path: root.join("src/service.ts"),
            parent_name: "ApiService".to_string(),
            member_name: "disconnect".to_string(),
            kind: MemberKind::ClassMethod,
            line: 99,
            col: 4,
        });
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        assert!(text.contains("ApiService.disconnect"));
        assert!(text.contains(":99"));
    }

    // ── Dependencies display ──

    #[test]
    fn unused_deps_at_root_show_package_name_only() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_dependencies.push(UnusedDependency {
            package_name: "lodash".to_string(),
            location: DependencyLocation::Dependencies,
            path: root.join("package.json"),
            line: 5,
        });
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        assert!(text.contains("lodash"));
        // Should NOT show "(package.json)" for root deps
        assert!(!text.contains("(package.json)"));
    }

    #[test]
    fn unused_deps_in_workspace_show_workspace_path() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_dependencies.push(UnusedDependency {
            package_name: "axios".to_string(),
            location: DependencyLocation::Dependencies,
            path: root.join("packages/web/package.json"),
            line: 8,
        });
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        assert!(text.contains("axios"));
        assert!(text.contains("(packages/web/package.json)"));
    }

    // ── Unresolved imports show specifier ──

    #[test]
    fn unresolved_imports_show_specifier_and_line() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unresolved_imports.push(UnresolvedImport {
            path: root.join("src/app.ts"),
            specifier: "@org/missing-pkg".to_string(),
            line: 7,
            col: 0,
        });
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        assert!(text.contains("src/app.ts"));
        assert!(text.contains(":7"));
        assert!(text.contains("@org/missing-pkg"));
    }

    // ── Duplicate exports show locations ──

    #[test]
    fn duplicate_exports_show_name_and_locations() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.duplicate_exports.push(DuplicateExport {
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
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        assert!(text.contains("Config"));
        assert!(text.contains("src/config.ts"));
        // file_b shown with common prefix elided
        assert!(text.contains("types.ts"));
    }

    // ── Circular dependencies show cycle with arrow ──

    #[test]
    fn circular_dependencies_show_cycle_with_arrow_and_repeat() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.circular_dependencies.push(CircularDependency {
            files: vec![
                root.join("src/a.ts"),
                root.join("src/b.ts"),
                root.join("src/c.ts"),
            ],
            length: 3,
            line: 1,
            col: 0,
        });
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        // Hub file shown first, chain with elided paths and arrows
        assert!(text.contains("a.ts"));
        assert!(text.contains("b.ts"));
        assert!(text.contains("c.ts"));
        assert!(text.contains("\u{2192}"));
    }

    // ── Empty sections are omitted ──

    #[test]
    fn empty_sections_are_omitted() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        // Only add unused files, no other issues
        results.unused_files.push(UnusedFile {
            path: root.join("src/dead.ts"),
        });
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        assert!(text.contains("Unused files (1)"));
        assert!(!text.contains("Unused exports"));
        assert!(!text.contains("Unused dependencies"));
        assert!(!text.contains("Unresolved imports"));
    }

    // ── Severity levels affect section header indicator ──

    #[test]
    fn section_header_uses_bullet_indicator() {
        // The section header always contains the bullet character
        let header = build_section_header("Test section", 3, Level::Error);
        let text = strip_ansi(&header);
        assert!(text.contains("\u{25cf}"));
        assert!(text.contains("Test section (3)"));
    }

    #[test]
    fn section_header_formats_for_all_levels() {
        // Verify all three levels produce valid headers (not panicking, contain the title)
        for level in [Level::Error, Level::Warn, Level::Info] {
            let header = build_section_header("Items", 7, level);
            let text = strip_ansi(&header);
            assert!(
                text.contains("Items (7)"),
                "Missing title for level {level:?}"
            );
        }
    }

    // ── Grouped sections sort by file path ──

    #[test]
    fn grouped_exports_from_different_files_sorted_by_path() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        // Add exports in non-alphabetical order
        results.unused_exports.push(UnusedExport {
            path: root.join("src/z-file.ts"),
            export_name: "zExport".to_string(),
            is_type_only: false,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        results.unused_exports.push(UnusedExport {
            path: root.join("src/a-file.ts"),
            export_name: "aExport".to_string(),
            is_type_only: false,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        // a-file should appear before z-file in output
        let a_pos = text.find("src/a-file.ts").unwrap();
        let z_pos = text.find("src/z-file.ts").unwrap();
        assert!(a_pos < z_pos, "Files should be sorted alphabetically");
    }

    // ── File grouping deduplicates file headers ──

    #[test]
    fn grouped_items_from_same_file_share_one_file_header() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        for i in 0..3 {
            results.unused_exports.push(UnusedExport {
                path: root.join("src/utils.ts"),
                export_name: format!("fn{i}"),
                is_type_only: false,
                line: (i + 1) as u32,
                col: 0,
                span_start: 0,
                is_re_export: false,
            });
        }
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        // "src/utils.ts" should appear exactly once as a group header
        let count = text.matches("src/utils.ts").count();
        assert_eq!(count, 1, "File header should appear once, found {count}");
    }

    // ── Severity affects which sections appear ──

    #[test]
    fn off_severity_still_shows_section_when_items_present() {
        // When severity is Off, the items are normally filtered before reaching
        // the reporter. But if items ARE present, the section should still render
        // (with Info-level styling).
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/dead.ts"),
        });
        let rules = RulesConfig {
            unused_files: Severity::Off,
            ..RulesConfig::default()
        };
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        assert!(text.contains("Unused files (1)"));
    }

    // ── Deeply nested paths display correctly ──

    #[test]
    fn deeply_nested_paths_display_correctly() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("packages/ui/src/components/forms/inputs/TextInput.tsx"),
        });
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        assert!(text.contains("packages/ui/src/components/forms/inputs/TextInput.tsx"));
    }

    // ── All section types produce output when populated ──

    #[test]
    fn all_issue_types_produce_output_lines() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        // Every populated section must produce a header with a count
        assert!(text.contains("Unused files (1)"));
        assert!(text.contains("Unused exports (1)"));
        assert!(text.contains("Unused type exports (1)"));
        assert!(text.contains("Unused dependencies (1)"));
        assert!(text.contains("Unused devDependencies (1)"));
        assert!(text.contains("Unused optionalDependencies (1)"));
        assert!(text.contains("Unused enum members (1)"));
        assert!(text.contains("Unused class members (1)"));
        assert!(text.contains("Unresolved imports (1)"));
        assert!(text.contains("Unlisted dependencies (1)"));
        assert!(text.contains("Duplicate exports (1)"));
        assert!(text.contains("Type-only dependencies (consider moving to devDependencies) (1)"));
        assert!(text.contains("Circular dependencies (1)"));
    }

    // ── Sections end with empty line separator ──

    #[test]
    fn each_section_ends_with_empty_line_separator() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/a.ts"),
        });
        results.unused_dependencies.push(UnusedDependency {
            package_name: "pkg".to_string(),
            location: DependencyLocation::Dependencies,
            path: root.join("package.json"),
            line: 1,
        });
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        // After each section, there should be an empty string separator
        let empty_count = lines.iter().filter(|l| l.is_empty()).count();
        assert_eq!(
            empty_count, 2,
            "Expected 2 empty separators (one per section), got {empty_count}"
        );
    }

    // ── Type-only dependencies section has specific title ──

    #[test]
    fn type_only_deps_section_title_includes_suggestion() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.type_only_dependencies.push(TypeOnlyDependency {
            package_name: "zod".to_string(),
            path: root.join("package.json"),
            line: 8,
        });
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        assert!(text.contains("Type-only dependencies (consider moving to devDependencies)"));
    }

    // ── Health report tests ──

    #[test]
    fn health_empty_findings_produces_no_header() {
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
            file_scores: vec![],
            hotspots: vec![],
            hotspot_summary: None,
        };
        let lines = build_health_human_lines(&report, &root);
        let text = plain(&lines);
        // With no findings and no file scores, no complexity header is produced
        assert!(!text.contains("High complexity functions"));
    }

    #[test]
    fn health_findings_show_function_details() {
        let root = PathBuf::from("/project");
        let report = crate::health_types::HealthReport {
            findings: vec![crate::health_types::HealthFinding {
                path: root.join("src/parser.ts"),
                name: "parseExpression".to_string(),
                line: 42,
                col: 0,
                cyclomatic: 25,
                cognitive: 30,
                line_count: 80,
                exceeded: crate::health_types::ExceededThreshold::Both,
            }],
            summary: crate::health_types::HealthSummary {
                files_analyzed: 10,
                functions_analyzed: 50,
                functions_above_threshold: 1,
                max_cyclomatic_threshold: 20,
                max_cognitive_threshold: 15,
                files_scored: None,
                average_maintainability: None,
            },
            file_scores: vec![],
            hotspots: vec![],
            hotspot_summary: None,
        };
        let lines = build_health_human_lines(&report, &root);
        let text = plain(&lines);
        assert!(text.contains("High complexity functions (1)"));
        assert!(text.contains("src/parser.ts"));
        assert!(text.contains(":42"));
        assert!(text.contains("parseExpression"));
        assert!(text.contains("25 cyclomatic"));
        assert!(text.contains("30 cognitive"));
        assert!(text.contains("80 lines"));
    }

    #[test]
    fn health_shown_vs_total_when_truncated() {
        let root = PathBuf::from("/project");
        let report = crate::health_types::HealthReport {
            findings: vec![crate::health_types::HealthFinding {
                path: root.join("src/a.ts"),
                name: "fn1".to_string(),
                line: 1,
                col: 0,
                cyclomatic: 25,
                cognitive: 20,
                line_count: 50,
                exceeded: crate::health_types::ExceededThreshold::Both,
            }],
            summary: crate::health_types::HealthSummary {
                files_analyzed: 100,
                functions_analyzed: 500,
                functions_above_threshold: 10,
                max_cyclomatic_threshold: 20,
                max_cognitive_threshold: 15,
                files_scored: None,
                average_maintainability: None,
            },
            file_scores: vec![],
            hotspots: vec![],
            hotspot_summary: None,
        };
        let lines = build_health_human_lines(&report, &root);
        let text = plain(&lines);
        // When shown < total, header says "N shown, M total"
        assert!(text.contains("1 shown, 10 total"));
    }

    #[test]
    fn health_findings_grouped_by_file() {
        let root = PathBuf::from("/project");
        let report = crate::health_types::HealthReport {
            findings: vec![
                crate::health_types::HealthFinding {
                    path: root.join("src/parser.ts"),
                    name: "fn1".to_string(),
                    line: 10,
                    col: 0,
                    cyclomatic: 25,
                    cognitive: 20,
                    line_count: 40,
                    exceeded: crate::health_types::ExceededThreshold::Both,
                },
                crate::health_types::HealthFinding {
                    path: root.join("src/parser.ts"),
                    name: "fn2".to_string(),
                    line: 60,
                    col: 0,
                    cyclomatic: 22,
                    cognitive: 18,
                    line_count: 30,
                    exceeded: crate::health_types::ExceededThreshold::Both,
                },
            ],
            summary: crate::health_types::HealthSummary {
                files_analyzed: 10,
                functions_analyzed: 50,
                functions_above_threshold: 2,
                max_cyclomatic_threshold: 20,
                max_cognitive_threshold: 15,
                files_scored: None,
                average_maintainability: None,
            },
            file_scores: vec![],
            hotspots: vec![],
            hotspot_summary: None,
        };
        let lines = build_health_human_lines(&report, &root);
        let text = plain(&lines);
        // File path should appear once (grouping)
        let count = text.matches("src/parser.ts").count();
        assert_eq!(count, 1, "File header should appear once for grouped items");
    }

    // ── Duplication report tests ──

    #[test]
    fn duplication_empty_report_produces_no_output() {
        let root = PathBuf::from("/project");
        let report = DuplicationReport::default();
        let lines = build_duplication_human_lines(&report, &root);
        assert!(lines.is_empty(), "Empty report should produce no lines");
    }

    #[test]
    fn duplication_groups_show_instances_with_line_count() {
        let root = PathBuf::from("/project");
        let report = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![
                    CloneInstance {
                        file: root.join("src/a.ts"),
                        start_line: 1,
                        end_line: 10,
                        start_col: 0,
                        end_col: 0,
                        fragment: String::new(),
                    },
                    CloneInstance {
                        file: root.join("src/b.ts"),
                        start_line: 5,
                        end_line: 14,
                        start_col: 0,
                        end_col: 0,
                        fragment: String::new(),
                    },
                ],
                token_count: 50,
                line_count: 10,
            }],
            clone_families: vec![],
            stats: DuplicationStats {
                clone_groups: 1,
                clone_instances: 2,
                ..Default::default()
            },
        };
        let lines = build_duplication_human_lines(&report, &root);
        let text = plain(&lines);
        // Line-count-led format, no "Clone group N" label
        assert!(text.contains("10"));
        assert!(text.contains("lines"));
        assert!(text.contains("2 instances"));
        assert!(text.contains("a.ts:1-10"));
        assert!(text.contains("b.ts:5-14"));
        // No tree connectors
        assert!(!text.contains("\u{251c}\u{2500}"));
        assert!(!text.contains("\u{2514}\u{2500}"));
    }

    #[test]
    fn duplication_single_instance_no_plural() {
        let root = PathBuf::from("/project");
        let report = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![CloneInstance {
                    file: root.join("src/a.ts"),
                    start_line: 1,
                    end_line: 10,
                    start_col: 0,
                    end_col: 0,
                    fragment: String::new(),
                }],
                token_count: 50,
                line_count: 10,
            }],
            clone_families: vec![],
            stats: DuplicationStats::default(),
        };
        let lines = build_duplication_human_lines(&report, &root);
        let text = plain(&lines);
        assert!(text.contains("1 instance"));
        assert!(!text.contains("1 instances"));
    }

    #[test]
    fn duplication_families_show_suggestions() {
        let root = PathBuf::from("/project");
        let dummy_group = CloneGroup {
            instances: vec![],
            token_count: 30,
            line_count: 5,
        };
        let report = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![CloneInstance {
                    file: root.join("src/a.ts"),
                    start_line: 1,
                    end_line: 5,
                    start_col: 0,
                    end_col: 0,
                    fragment: String::new(),
                }],
                token_count: 30,
                line_count: 5,
            }],
            clone_families: vec![CloneFamily {
                files: vec![root.join("src/a.ts"), root.join("src/b.ts")],
                groups: vec![dummy_group.clone(), dummy_group],
                total_duplicated_lines: 20,
                total_duplicated_tokens: 100,
                suggestions: vec![RefactoringSuggestion {
                    kind: RefactoringKind::ExtractFunction,
                    description: "Extract shared utility function".to_string(),
                    estimated_savings: 15,
                }],
            }],
            stats: DuplicationStats::default(),
        };
        let lines = build_duplication_human_lines(&report, &root);
        let text = plain(&lines);
        assert!(text.contains("Clone families"));
        assert!(text.contains("Extract shared utility function"));
        // "lines saved" is no longer shown
        assert!(!text.contains("lines saved"));
    }

    #[test]
    fn duplication_suggestion_with_zero_savings_omits_savings_text() {
        let root = PathBuf::from("/project");
        let dummy_group = CloneGroup {
            instances: vec![],
            token_count: 30,
            line_count: 5,
        };
        let report = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![CloneInstance {
                    file: root.join("src/a.ts"),
                    start_line: 1,
                    end_line: 5,
                    start_col: 0,
                    end_col: 0,
                    fragment: String::new(),
                }],
                token_count: 30,
                line_count: 5,
            }],
            clone_families: vec![CloneFamily {
                files: vec![root.join("src/a.ts")],
                groups: vec![dummy_group.clone(), dummy_group],
                total_duplicated_lines: 10,
                total_duplicated_tokens: 50,
                suggestions: vec![RefactoringSuggestion {
                    kind: RefactoringKind::ExtractModule,
                    description: "Extract to shared module".to_string(),
                    estimated_savings: 0,
                }],
            }],
            stats: DuplicationStats::default(),
        };
        let lines = build_duplication_human_lines(&report, &root);
        let text = plain(&lines);
        assert!(text.contains("Extract to shared module"));
        assert!(!text.contains("lines saved"));
    }

    // ── Cross-reference tests ──

    #[test]
    fn cross_reference_empty_findings_produces_header_and_blanks() {
        let root = PathBuf::from("/project");
        let cross_ref = CrossReferenceResult {
            combined_findings: vec![CombinedFinding {
                clone_instance: CloneInstance {
                    file: root.join("src/dead.ts"),
                    start_line: 1,
                    end_line: 10,
                    start_col: 0,
                    end_col: 0,
                    fragment: String::new(),
                },
                dead_code_kind: DeadCodeKind::UnusedFile,
                group_index: 0,
            }],
            clones_in_unused_files: 1,
            clones_with_unused_exports: 0,
        };
        let lines = build_cross_reference_lines(&cross_ref, &root);
        let text = plain(&lines);
        assert!(text.contains("Duplicated + Unused (safe to delete)"));
        assert!(text.contains("src/dead.ts:1-10"));
        assert!(text.contains("(entire file is unused)"));
    }

    #[test]
    fn cross_reference_unused_export_reason() {
        let root = PathBuf::from("/project");
        let cross_ref = CrossReferenceResult {
            combined_findings: vec![CombinedFinding {
                clone_instance: CloneInstance {
                    file: root.join("src/utils.ts"),
                    start_line: 5,
                    end_line: 15,
                    start_col: 0,
                    end_col: 0,
                    fragment: String::new(),
                },
                dead_code_kind: DeadCodeKind::UnusedExport {
                    export_name: "processData".to_string(),
                },
                group_index: 0,
            }],
            clones_in_unused_files: 0,
            clones_with_unused_exports: 1,
        };
        let lines = build_cross_reference_lines(&cross_ref, &root);
        let text = plain(&lines);
        assert!(text.contains("export 'processData' is unused"));
    }

    #[test]
    fn cross_reference_unused_type_reason() {
        let root = PathBuf::from("/project");
        let cross_ref = CrossReferenceResult {
            combined_findings: vec![CombinedFinding {
                clone_instance: CloneInstance {
                    file: root.join("src/types.ts"),
                    start_line: 1,
                    end_line: 5,
                    start_col: 0,
                    end_col: 0,
                    fragment: String::new(),
                },
                dead_code_kind: DeadCodeKind::UnusedType {
                    type_name: "OldConfig".to_string(),
                },
                group_index: 0,
            }],
            clones_in_unused_files: 0,
            clones_with_unused_exports: 1,
        };
        let lines = build_cross_reference_lines(&cross_ref, &root);
        let text = plain(&lines);
        assert!(text.contains("type 'OldConfig' is unused"));
    }

    // ── Performance output tests ──

    #[test]
    fn performance_output_contains_all_pipeline_stages() {
        let timings = PipelineTimings {
            discover_files_ms: 12.5,
            file_count: 100,
            workspaces_ms: 3.2,
            workspace_count: 3,
            plugins_ms: 1.0,
            script_analysis_ms: 2.5,
            parse_extract_ms: 45.0,
            module_count: 80,
            cache_hits: 0,
            cache_misses: 80,
            cache_update_ms: 5.0,
            entry_points_ms: 0.5,
            entry_point_count: 10,
            resolve_imports_ms: 8.0,
            build_graph_ms: 15.0,
            analyze_ms: 10.0,
            total_ms: 102.7,
        };
        let lines = build_performance_human_lines(&timings);
        let text = plain(&lines);
        assert!(text.contains("Pipeline Performance"));
        assert!(text.contains("discover files"));
        assert!(text.contains("100 files"));
        assert!(text.contains("workspaces"));
        assert!(text.contains("3 workspaces"));
        assert!(text.contains("plugins"));
        assert!(text.contains("script analysis"));
        assert!(text.contains("parse/extract"));
        assert!(text.contains("80 modules"));
        assert!(text.contains("cache update"));
        assert!(text.contains("entry points"));
        assert!(text.contains("10 entries"));
        assert!(text.contains("resolve imports"));
        assert!(text.contains("build graph"));
        assert!(text.contains("analyze"));
        assert!(text.contains("TOTAL"));
        assert!(text.contains("102.7"));
    }

    #[test]
    fn performance_output_shows_cache_detail_when_cache_hits_nonzero() {
        let timings = PipelineTimings {
            discover_files_ms: 10.0,
            file_count: 50,
            workspaces_ms: 1.0,
            workspace_count: 1,
            plugins_ms: 0.5,
            script_analysis_ms: 1.0,
            parse_extract_ms: 20.0,
            module_count: 40,
            cache_hits: 30,
            cache_misses: 10,
            cache_update_ms: 2.0,
            entry_points_ms: 0.3,
            entry_point_count: 5,
            resolve_imports_ms: 3.0,
            build_graph_ms: 5.0,
            analyze_ms: 4.0,
            total_ms: 46.8,
        };
        let lines = build_performance_human_lines(&timings);
        let text = plain(&lines);
        assert!(text.contains("30 cached"));
        assert!(text.contains("10 parsed"));
    }

    #[test]
    fn performance_output_omits_cache_detail_when_no_cache_hits() {
        let timings = PipelineTimings {
            discover_files_ms: 10.0,
            file_count: 50,
            workspaces_ms: 1.0,
            workspace_count: 1,
            plugins_ms: 0.5,
            script_analysis_ms: 1.0,
            parse_extract_ms: 20.0,
            module_count: 40,
            cache_hits: 0,
            cache_misses: 40,
            cache_update_ms: 2.0,
            entry_points_ms: 0.3,
            entry_point_count: 5,
            resolve_imports_ms: 3.0,
            build_graph_ms: 5.0,
            analyze_ms: 4.0,
            total_ms: 46.8,
        };
        let lines = build_performance_human_lines(&timings);
        let text = plain(&lines);
        assert!(!text.contains("cached"));
        assert!(!text.contains("parsed"));
    }

    // ── strip_ansi utility ──

    #[test]
    fn strip_ansi_removes_color_codes() {
        let colored_str = "hello".red().bold().to_string();
        assert_eq!(strip_ansi(&colored_str), "hello");
    }

    #[test]
    fn strip_ansi_preserves_plain_text() {
        assert_eq!(strip_ansi("plain text"), "plain text");
    }

    #[test]
    fn strip_ansi_handles_empty_string() {
        assert_eq!(strip_ansi(""), "");
    }

    // ── Warn severity renders with correct indicator for section header ──

    #[test]
    fn warn_severity_produces_header_with_bullet() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.type_only_dependencies.push(TypeOnlyDependency {
            package_name: "zod".to_string(),
            path: root.join("package.json"),
            line: 8,
        });
        // type_only_dependencies defaults to Warn
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        // Verify the section appears with the correct title (the styling differs
        // between Warn and Error, but the structural content is the same)
        assert!(text.contains("\u{25cf}"));
        assert!(text.contains("Type-only dependencies"));
    }

    // ── Unlisted dependencies show package name ──

    #[test]
    fn unlisted_deps_show_package_name() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unlisted_dependencies.push(UnlistedDependency {
            package_name: "@scope/unknown-pkg".to_string(),
            imported_from: vec![],
        });
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        assert!(text.contains("@scope/unknown-pkg"));
    }

    // ── Duplication family pluralization ──

    #[test]
    fn duplication_single_group_family_is_suppressed() {
        let root = PathBuf::from("/project");
        let report = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![CloneInstance {
                    file: root.join("src/a.ts"),
                    start_line: 1,
                    end_line: 5,
                    start_col: 0,
                    end_col: 0,
                    fragment: String::new(),
                }],
                token_count: 30,
                line_count: 5,
            }],
            clone_families: vec![CloneFamily {
                files: vec![root.join("src/a.ts")],
                groups: vec![CloneGroup {
                    instances: vec![],
                    token_count: 30,
                    line_count: 5,
                }],
                total_duplicated_lines: 5,
                total_duplicated_tokens: 30,
                suggestions: vec![],
            }],
            stats: DuplicationStats::default(),
        };
        let lines = build_duplication_human_lines(&report, &root);
        let text = plain(&lines);
        // Single-group families are suppressed from output
        assert!(!text.contains("Clone families"));
    }

    #[test]
    fn duplication_multiple_groups_plural() {
        let root = PathBuf::from("/project");
        let dummy_group = CloneGroup {
            instances: vec![],
            token_count: 30,
            line_count: 5,
        };
        let report = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![CloneInstance {
                    file: root.join("src/a.ts"),
                    start_line: 1,
                    end_line: 5,
                    start_col: 0,
                    end_col: 0,
                    fragment: String::new(),
                }],
                token_count: 30,
                line_count: 5,
            }],
            clone_families: vec![CloneFamily {
                files: vec![root.join("src/a.ts")],
                groups: vec![dummy_group.clone(), dummy_group],
                total_duplicated_lines: 10,
                total_duplicated_tokens: 60,
                suggestions: vec![],
            }],
            stats: DuplicationStats::default(),
        };
        let lines = build_duplication_human_lines(&report, &root);
        let text = plain(&lines);
        assert!(text.contains("2 groups,"));
    }

    // ── Single instance connector: only └─, no ├─ ──

    #[test]
    fn single_instance_clone_group_no_connectors() {
        let root = PathBuf::from("/project");
        let report = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![CloneInstance {
                    file: root.join("src/a.ts"),
                    start_line: 1,
                    end_line: 10,
                    start_col: 0,
                    end_col: 0,
                    fragment: String::new(),
                }],
                token_count: 50,
                line_count: 10,
            }],
            clone_families: vec![],
            stats: DuplicationStats::default(),
        };
        let lines = build_duplication_human_lines(&report, &root);
        let text = plain(&lines);
        // No tree connectors — simple indentation
        assert!(!text.contains("\u{2514}\u{2500}"));
        assert!(!text.contains("\u{251c}\u{2500}"));
        assert!(text.contains("a.ts:1-10"));
    }

    // ── Utility function tests ──

    #[test]
    fn thousands_zero() {
        assert_eq!(thousands(0), "0");
    }

    #[test]
    fn thousands_small() {
        assert_eq!(thousands(999), "999");
    }

    #[test]
    fn thousands_boundary() {
        assert_eq!(thousands(1000), "1,000");
    }

    #[test]
    fn thousands_large() {
        assert_eq!(thousands(1_000_000), "1,000,000");
    }

    #[test]
    fn thousands_irregular() {
        assert_eq!(thousands(12345), "12,345");
    }

    #[test]
    fn format_path_with_directory() {
        let result = strip_ansi(&format_path("src/components/Button.tsx"));
        assert!(result.ends_with("Button.tsx"));
        assert!(result.contains("src/components/"));
    }

    #[test]
    fn format_path_no_directory() {
        let result = strip_ansi(&format_path("index.ts"));
        assert_eq!(result, "index.ts");
    }

    // ── Truncation tests ──

    #[test]
    fn flat_section_truncates_at_max() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        for i in 0..15 {
            results.unused_files.push(UnusedFile {
                path: root.join(format!("src/dead{i}.ts")),
            });
        }
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        assert!(text.contains("... and 5 more"));
    }

    #[test]
    fn grouped_section_truncates_files() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        // 15 files with 1 export each
        for i in 0..15 {
            results.unused_exports.push(UnusedExport {
                path: root.join(format!("src/file{i:02}.ts")),
                export_name: format!("fn{i}"),
                is_type_only: false,
                line: 1,
                col: 0,
                span_start: 0,
                is_re_export: false,
            });
        }
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        assert!(text.contains("... and 5 more in 5 files"));
    }

    // ── Hub-grouped circular deps ──

    #[test]
    fn circular_deps_grouped_by_hub() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        // Two cycles sharing the same hub file
        results.circular_dependencies.push(CircularDependency {
            files: vec![root.join("src/hub.ts"), root.join("src/a.ts")],
            length: 2,
            line: 1,
            col: 0,
        });
        results.circular_dependencies.push(CircularDependency {
            files: vec![root.join("src/hub.ts"), root.join("src/b.ts")],
            length: 2,
            line: 5,
            col: 0,
        });
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        // Should show "(2 cycles)" for the hub
        assert!(text.contains("(2 cycles)"));
        // Hub file appears once
        assert_eq!(text.matches("hub.ts").count(), 3); // header + 2 chain endings
    }

    // ── Mirrored directory detection ──

    #[test]
    fn mirrored_dirs_detected() {
        let root = PathBuf::from("/project");
        let mut families = Vec::new();
        // 4 families with same dir pattern (above MIN_MIRROR_FAMILIES threshold of 3)
        for name in &["a.ts", "b.ts", "c.ts", "d.ts"] {
            families.push(CloneFamily {
                files: vec![
                    root.join(format!("src/{name}")),
                    root.join(format!("deno/lib/{name}")),
                ],
                groups: vec![CloneGroup {
                    instances: vec![],
                    token_count: 100,
                    line_count: 50,
                }],
                total_duplicated_lines: 50,
                total_duplicated_tokens: 100,
                suggestions: vec![],
            });
        }
        let (mirrored, non_mirrored) = detect_mirrored_families(&families, &root);
        assert_eq!(mirrored.len(), 1);
        assert_eq!(mirrored[0].file_count, 4);
        assert!(non_mirrored.is_empty());
    }

    #[test]
    fn mirrored_dirs_below_threshold_not_detected() {
        let root = PathBuf::from("/project");
        let families = vec![
            CloneFamily {
                files: vec![root.join("src/a.ts"), root.join("deno/a.ts")],
                groups: vec![],
                total_duplicated_lines: 10,
                total_duplicated_tokens: 50,
                suggestions: vec![],
            },
            CloneFamily {
                files: vec![root.join("src/b.ts"), root.join("deno/b.ts")],
                groups: vec![],
                total_duplicated_lines: 10,
                total_duplicated_tokens: 50,
                suggestions: vec![],
            },
        ];
        let (mirrored, _) = detect_mirrored_families(&families, &root);
        // Only 2 families — below threshold of 3
        assert!(mirrored.is_empty());
    }

    // ── Summary footer ──

    #[test]
    fn summary_footer_uses_short_labels() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let footer = build_summary_footer(&results);
        // Should use short labels, not "unused file" etc.
        assert!(footer.contains("1 file"));
        assert!(footer.contains("1 export"));
        assert!(footer.contains("1 circular"));
        assert!(!footer.contains("unused file"));
    }

    // ── Section footers with docs links ──

    #[test]
    fn section_footer_contains_docs_link() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/dead.ts"),
        });
        let rules = RulesConfig::default();
        let lines = build_human_lines(&results, &root, &rules);
        let text = plain(&lines);
        // Human output always includes section footers with doc links
        assert!(text.contains("docs.fallow.tools/explanations/dead-code"));
        assert!(text.contains("Files not imported or referenced by any entry point"));
    }
}
