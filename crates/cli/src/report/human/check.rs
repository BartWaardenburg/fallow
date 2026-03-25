use std::fmt::Write as _;
use std::path::Path;
use std::time::Duration;

use colored::Colorize;
use fallow_config::RulesConfig;
use fallow_core::results::{AnalysisResults, UnusedExport, UnusedMember};

use super::{
    MAX_FLAT_ITEMS, build_grouped_by_file, build_section_header, format_path, push_section_footer,
};
use crate::report::{
    Level, elide_common_prefix, relative_path, severity_to_level, split_dir_filename,
};

/// Maximum files shown per grouped section (unused exports, types, etc.).
const MAX_GROUPED_FILES: usize = 10;
/// Maximum detail items shown per file within a grouped section.
const MAX_ITEMS_PER_FILE: usize = 5;

pub(in crate::report) fn print_human(
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
pub(in crate::report) fn build_human_lines(
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
    build_grouped_by_file(
        lines,
        items,
        root,
        get_path,
        format_detail,
        MAX_GROUPED_FILES,
        MAX_ITEMS_PER_FILE,
    );
    push_section_footer(lines, title);
    lines.push(String::new());
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

        // Vertical layout: file_a on line 1, <-> file_b on line 2, exports on line 3
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

#[cfg(test)]
mod tests {
    use super::*;
    use fallow_config::{RulesConfig, Severity};
    use fallow_core::extract::MemberKind;
    use fallow_core::results::*;
    use std::path::PathBuf;

    /// Strip ANSI escape sequences from a string, leaving only the printable text.
    fn strip_ansi(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
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
            specifier_col: 0,
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
}
