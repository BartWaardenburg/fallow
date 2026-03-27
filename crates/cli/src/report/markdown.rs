use std::fmt::Write;
use std::path::Path;

use fallow_core::duplicates::DuplicationReport;
use fallow_core::results::{AnalysisResults, UnusedExport, UnusedMember};

use super::{normalize_uri, plural, relative_path};

/// Escape backticks in user-controlled strings to prevent breaking markdown code spans.
fn escape_backticks(s: &str) -> String {
    s.replace('`', "\\`")
}

pub(super) fn print_markdown(results: &AnalysisResults, root: &Path) {
    println!("{}", build_markdown(results, root));
}

/// Build markdown output for analysis results.
pub fn build_markdown(results: &AnalysisResults, root: &Path) -> String {
    let rel = |p: &Path| {
        escape_backticks(&normalize_uri(
            &relative_path(p, root).display().to_string(),
        ))
    };

    let total = results.total_issues();
    let mut out = String::new();

    if total == 0 {
        out.push_str("## Fallow: no issues found\n");
        return out;
    }

    let _ = write!(out, "## Fallow: {total} issue{} found\n\n", plural(total));

    // ── Unused files ──
    markdown_section(&mut out, &results.unused_files, "Unused files", |file| {
        vec![format!("- `{}`", rel(&file.path))]
    });

    // ── Unused exports ──
    markdown_grouped_section(
        &mut out,
        &results.unused_exports,
        "Unused exports",
        root,
        |e| e.path.as_path(),
        format_export,
    );

    // ── Unused types ──
    markdown_grouped_section(
        &mut out,
        &results.unused_types,
        "Unused type exports",
        root,
        |e| e.path.as_path(),
        format_export,
    );

    // ── Unused dependencies ──
    markdown_section(
        &mut out,
        &results.unused_dependencies,
        "Unused dependencies",
        |dep| format_dependency(&dep.package_name, &dep.path, root),
    );

    // ── Unused devDependencies ──
    markdown_section(
        &mut out,
        &results.unused_dev_dependencies,
        "Unused devDependencies",
        |dep| format_dependency(&dep.package_name, &dep.path, root),
    );

    // ── Unused optionalDependencies ──
    markdown_section(
        &mut out,
        &results.unused_optional_dependencies,
        "Unused optionalDependencies",
        |dep| format_dependency(&dep.package_name, &dep.path, root),
    );

    // ── Unused enum members ──
    markdown_grouped_section(
        &mut out,
        &results.unused_enum_members,
        "Unused enum members",
        root,
        |m| m.path.as_path(),
        format_member,
    );

    // ── Unused class members ──
    markdown_grouped_section(
        &mut out,
        &results.unused_class_members,
        "Unused class members",
        root,
        |m| m.path.as_path(),
        format_member,
    );

    // ── Unresolved imports ──
    markdown_grouped_section(
        &mut out,
        &results.unresolved_imports,
        "Unresolved imports",
        root,
        |i| i.path.as_path(),
        |i| format!(":{} `{}`", i.line, escape_backticks(&i.specifier)),
    );

    // ── Unlisted dependencies ──
    markdown_section(
        &mut out,
        &results.unlisted_dependencies,
        "Unlisted dependencies",
        |dep| vec![format!("- `{}`", escape_backticks(&dep.package_name))],
    );

    // ── Duplicate exports ──
    markdown_section(
        &mut out,
        &results.duplicate_exports,
        "Duplicate exports",
        |dup| {
            let locations: Vec<String> = dup
                .locations
                .iter()
                .map(|loc| format!("`{}`", rel(&loc.path)))
                .collect();
            vec![format!(
                "- `{}` in {}",
                escape_backticks(&dup.export_name),
                locations.join(", ")
            )]
        },
    );

    // ── Type-only dependencies ──
    markdown_section(
        &mut out,
        &results.type_only_dependencies,
        "Type-only dependencies (consider moving to devDependencies)",
        |dep| format_dependency(&dep.package_name, &dep.path, root),
    );

    // ── Circular dependencies ──
    markdown_section(
        &mut out,
        &results.circular_dependencies,
        "Circular dependencies",
        |cycle| {
            let chain: Vec<String> = cycle.files.iter().map(|p| rel(p)).collect();
            let mut display_chain = chain.clone();
            if let Some(first) = chain.first() {
                display_chain.push(first.clone());
            }
            vec![format!(
                "- {}",
                display_chain
                    .iter()
                    .map(|s| format!("`{s}`"))
                    .collect::<Vec<_>>()
                    .join(" \u{2192} ")
            )]
        },
    );

    out
}

fn format_export(e: &UnusedExport) -> String {
    let re = if e.is_re_export { " (re-export)" } else { "" };
    format!(":{} `{}`{re}", e.line, escape_backticks(&e.export_name))
}

fn format_member(m: &UnusedMember) -> String {
    format!(
        ":{} `{}.{}`",
        m.line,
        escape_backticks(&m.parent_name),
        escape_backticks(&m.member_name)
    )
}

fn format_dependency(dep_name: &str, pkg_path: &Path, root: &Path) -> Vec<String> {
    let name = escape_backticks(dep_name);
    let pkg_label = relative_path(pkg_path, root).display().to_string();
    if pkg_label == "package.json" {
        vec![format!("- `{name}`")]
    } else {
        let label = escape_backticks(&pkg_label);
        vec![format!("- `{name}` ({label})")]
    }
}

/// Emit a markdown section with a header and per-item lines. Skipped if empty.
fn markdown_section<T>(
    out: &mut String,
    items: &[T],
    title: &str,
    format_lines: impl Fn(&T) -> Vec<String>,
) {
    if items.is_empty() {
        return;
    }
    let _ = write!(out, "### {title} ({})\n\n", items.len());
    for item in items {
        for line in format_lines(item) {
            out.push_str(&line);
            out.push('\n');
        }
    }
    out.push('\n');
}

/// Emit a markdown section whose items are grouped by file path.
fn markdown_grouped_section<'a, T>(
    out: &mut String,
    items: &'a [T],
    title: &str,
    root: &Path,
    get_path: impl Fn(&'a T) -> &'a Path,
    format_detail: impl Fn(&T) -> String,
) {
    if items.is_empty() {
        return;
    }
    let _ = write!(out, "### {title} ({})\n\n", items.len());

    let mut indices: Vec<usize> = (0..items.len()).collect();
    indices.sort_by(|&a, &b| get_path(&items[a]).cmp(get_path(&items[b])));

    let rel = |p: &Path| normalize_uri(&relative_path(p, root).display().to_string());
    let mut last_file = String::new();
    for &i in &indices {
        let item = &items[i];
        let file_str = rel(get_path(item));
        if file_str != last_file {
            let _ = writeln!(out, "- `{file_str}`");
            last_file = file_str;
        }
        let _ = writeln!(out, "  - {}", format_detail(item));
    }
    out.push('\n');
}

// ── Duplication markdown output ──────────────────────────────────

pub(super) fn print_duplication_markdown(report: &DuplicationReport, root: &Path) {
    println!("{}", build_duplication_markdown(report, root));
}

/// Build markdown output for duplication results.
pub fn build_duplication_markdown(report: &DuplicationReport, root: &Path) -> String {
    let rel = |p: &Path| normalize_uri(&relative_path(p, root).display().to_string());

    let mut out = String::new();

    if report.clone_groups.is_empty() {
        out.push_str("## Fallow: no code duplication found\n");
        return out;
    }

    let stats = &report.stats;
    let _ = write!(
        out,
        "## Fallow: {} clone group{} found ({:.1}% duplication)\n\n",
        stats.clone_groups,
        plural(stats.clone_groups),
        stats.duplication_percentage,
    );

    out.push_str("### Duplicates\n\n");
    for (i, group) in report.clone_groups.iter().enumerate() {
        let instance_count = group.instances.len();
        let _ = write!(
            out,
            "**Clone group {}** ({} lines, {instance_count} instance{})\n\n",
            i + 1,
            group.line_count,
            plural(instance_count)
        );
        for instance in &group.instances {
            let relative = rel(&instance.file);
            let _ = writeln!(
                out,
                "- `{relative}:{}-{}`",
                instance.start_line, instance.end_line
            );
        }
        out.push('\n');
    }

    // Clone families
    if !report.clone_families.is_empty() {
        out.push_str("### Clone Families\n\n");
        for (i, family) in report.clone_families.iter().enumerate() {
            let file_names: Vec<_> = family.files.iter().map(|f| rel(f)).collect();
            let _ = write!(
                out,
                "**Family {}** ({} group{}, {} lines across {})\n\n",
                i + 1,
                family.groups.len(),
                plural(family.groups.len()),
                family.total_duplicated_lines,
                file_names
                    .iter()
                    .map(|s| format!("`{s}`"))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            for suggestion in &family.suggestions {
                let savings = if suggestion.estimated_savings > 0 {
                    format!(" (~{} lines saved)", suggestion.estimated_savings)
                } else {
                    String::new()
                };
                let _ = writeln!(out, "- {}{savings}", suggestion.description);
            }
            out.push('\n');
        }
    }

    // Summary line
    let _ = writeln!(
        out,
        "**Summary:** {} duplicated lines ({:.1}%) across {} file{}",
        stats.duplicated_lines,
        stats.duplication_percentage,
        stats.files_with_clones,
        plural(stats.files_with_clones),
    );

    out
}

// ── Health markdown output ──────────────────────────────────────────

pub(super) fn print_health_markdown(report: &crate::health_types::HealthReport, root: &Path) {
    println!("{}", build_health_markdown(report, root));
}

/// Build markdown output for health (complexity) results.
pub fn build_health_markdown(report: &crate::health_types::HealthReport, root: &Path) -> String {
    let rel = |p: &Path| {
        escape_backticks(&normalize_uri(
            &relative_path(p, root).display().to_string(),
        ))
    };

    let mut out = String::new();

    // Vital signs summary table
    if let Some(ref vs) = report.vital_signs {
        out.push_str("## Vital Signs\n\n");
        out.push_str("| Metric | Value |\n");
        out.push_str("|:-------|------:|\n");
        let _ = writeln!(out, "| Avg Cyclomatic | {:.1} |", vs.avg_cyclomatic);
        let _ = writeln!(out, "| P90 Cyclomatic | {} |", vs.p90_cyclomatic);
        if let Some(v) = vs.dead_file_pct {
            let _ = writeln!(out, "| Dead Files | {v:.1}% |");
        }
        if let Some(v) = vs.dead_export_pct {
            let _ = writeln!(out, "| Dead Exports | {v:.1}% |");
        }
        if let Some(v) = vs.maintainability_avg {
            let _ = writeln!(out, "| Maintainability (avg) | {v:.1} |");
        }
        if let Some(v) = vs.hotspot_count {
            let _ = writeln!(out, "| Hotspots | {v} |");
        }
        if let Some(v) = vs.circular_dep_count {
            let _ = writeln!(out, "| Circular Deps | {v} |");
        }
        if let Some(v) = vs.unused_dep_count {
            let _ = writeln!(out, "| Unused Deps | {v} |");
        }
        out.push('\n');
    }

    if report.findings.is_empty()
        && report.file_scores.is_empty()
        && report.hotspots.is_empty()
        && report.targets.is_empty()
    {
        if report.vital_signs.is_none() {
            let _ = write!(
                out,
                "## Fallow: no functions exceed complexity thresholds\n\n\
                 **{}** functions analyzed (max cyclomatic: {}, max cognitive: {})\n",
                report.summary.functions_analyzed,
                report.summary.max_cyclomatic_threshold,
                report.summary.max_cognitive_threshold,
            );
        }
        return out;
    }

    if !report.findings.is_empty() {
        let count = report.summary.functions_above_threshold;
        let shown = report.findings.len();
        if shown < count {
            let _ = write!(
                out,
                "## Fallow: {count} high complexity function{} ({shown} shown)\n\n",
                plural(count),
            );
        } else {
            let _ = write!(
                out,
                "## Fallow: {count} high complexity function{}\n\n",
                plural(count),
            );
        }

        out.push_str("| File | Function | Cyclomatic | Cognitive | Lines |\n");
        out.push_str("|:-----|:---------|:-----------|:----------|:------|\n");

        for finding in &report.findings {
            let file_str = rel(&finding.path);
            let cyc_marker = if finding.cyclomatic > report.summary.max_cyclomatic_threshold {
                " **!**"
            } else {
                ""
            };
            let cog_marker = if finding.cognitive > report.summary.max_cognitive_threshold {
                " **!**"
            } else {
                ""
            };
            let _ = writeln!(
                out,
                "| `{file_str}:{line}` | `{name}` | {cyc}{cyc_marker} | {cog}{cog_marker} | {lines} |",
                line = finding.line,
                name = escape_backticks(&finding.name),
                cyc = finding.cyclomatic,
                cog = finding.cognitive,
                lines = finding.line_count,
            );
        }

        let s = &report.summary;
        let _ = write!(
            out,
            "\n**{files}** files, **{funcs}** functions analyzed \
             (thresholds: cyclomatic > {cyc}, cognitive > {cog})\n",
            files = s.files_analyzed,
            funcs = s.functions_analyzed,
            cyc = s.max_cyclomatic_threshold,
            cog = s.max_cognitive_threshold,
        );
    }

    // File health scores table
    if !report.file_scores.is_empty() {
        out.push('\n');
        let _ = writeln!(
            out,
            "### File Health Scores ({} files)\n",
            report.file_scores.len(),
        );
        out.push_str("| File | MI | Fan-in | Fan-out | Dead Code | Density |\n");
        out.push_str("|:-----|:---|:-------|:--------|:----------|:--------|\n");

        for score in &report.file_scores {
            let file_str = rel(&score.path);
            let _ = writeln!(
                out,
                "| `{file_str}` | {mi:.1} | {fi} | {fan_out} | {dead:.0}% | {density:.2} |",
                mi = score.maintainability_index,
                fi = score.fan_in,
                fan_out = score.fan_out,
                dead = score.dead_code_ratio * 100.0,
                density = score.complexity_density,
            );
        }

        if let Some(avg) = report.summary.average_maintainability {
            let _ = write!(out, "\n**Average maintainability index:** {avg:.1}/100\n");
        }
    }

    // Hotspot table
    if !report.hotspots.is_empty() {
        out.push('\n');
        let header = if let Some(ref summary) = report.hotspot_summary {
            format!(
                "### Hotspots ({} files, since {})\n",
                report.hotspots.len(),
                summary.since,
            )
        } else {
            format!("### Hotspots ({} files)\n", report.hotspots.len())
        };
        let _ = writeln!(out, "{header}");
        out.push_str("| File | Score | Commits | Churn | Density | Fan-in | Trend |\n");
        out.push_str("|:-----|:------|:--------|:------|:--------|:-------|:------|\n");

        for entry in &report.hotspots {
            let file_str = rel(&entry.path);
            let _ = writeln!(
                out,
                "| `{file_str}` | {score:.1} | {commits} | {churn} | {density:.2} | {fi} | {trend} |",
                score = entry.score,
                commits = entry.commits,
                churn = entry.lines_added + entry.lines_deleted,
                density = entry.complexity_density,
                fi = entry.fan_in,
                trend = entry.trend,
            );
        }

        if let Some(ref summary) = report.hotspot_summary
            && summary.files_excluded > 0
        {
            let _ = write!(
                out,
                "\n*{} file{} excluded (< {} commits)*\n",
                summary.files_excluded,
                plural(summary.files_excluded),
                summary.min_commits,
            );
        }
    }

    // Refactoring targets
    if !report.targets.is_empty() {
        let _ = write!(
            out,
            "\n### Refactoring Targets ({})\n\n",
            report.targets.len()
        );
        out.push_str("| Efficiency | Category | Effort / Confidence | File | Recommendation |\n");
        out.push_str("|:-----------|:---------|:--------------------|:-----|:---------------|\n");
        for target in &report.targets {
            let file_str = normalize_uri(&relative_path(&target.path, root).display().to_string());
            let category = target.category.label();
            let effort = target.effort.label();
            let confidence = target.confidence.label();
            let _ = writeln!(
                out,
                "| {:.1} | {category} | {effort} / {confidence} | `{file_str}` | {} |",
                target.efficiency, target.recommendation,
            );
        }
    }

    // Metric legend — explains abbreviations used in the tables above
    let has_scores = !report.file_scores.is_empty();
    let has_hotspots = !report.hotspots.is_empty();
    let has_targets = !report.targets.is_empty();
    if has_scores || has_hotspots || has_targets {
        out.push_str("\n---\n\n<details><summary>Metric definitions</summary>\n\n");
        if has_scores {
            out.push_str("- **MI** — Maintainability Index (0\u{2013}100, higher is better)\n");
            out.push_str("- **Fan-in** — files that import this file (blast radius)\n");
            out.push_str("- **Fan-out** — files this file imports (coupling)\n");
            out.push_str("- **Dead Code** — % of value exports with zero references\n");
            out.push_str("- **Density** — cyclomatic complexity / lines of code\n");
        }
        if has_hotspots {
            out.push_str(
                "- **Score** — churn \u{00d7} complexity (0\u{2013}100, higher = riskier)\n",
            );
            out.push_str("- **Commits** — commits in the analysis window\n");
            out.push_str("- **Churn** — total lines added + deleted\n");
            out.push_str("- **Trend** — accelerating / stable / cooling\n");
        }
        if has_targets {
            out.push_str("- **Efficiency** — priority / effort (higher = better quick-win value, default sort)\n");
            out.push_str("- **Category** — recommendation type (churn+complexity, high impact, dead code, complexity, coupling, circular dep)\n");
            out.push_str("- **Effort** — estimated effort (low / medium / high) based on file size, function count, and fan-in\n");
            out.push_str("- **Confidence** — recommendation reliability (high = deterministic analysis, medium = heuristic, low = git-dependent)\n");
        }
        out.push_str("\n[Full metric reference](https://docs.fallow.tools/explanations/metrics)\n\n</details>\n");
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::test_helpers::sample_results;
    use fallow_core::duplicates::{
        CloneFamily, CloneGroup, CloneInstance, DuplicationReport, DuplicationStats,
        RefactoringKind, RefactoringSuggestion,
    };
    use fallow_core::results::*;
    use std::path::PathBuf;

    #[test]
    fn markdown_empty_results_no_issues() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let md = build_markdown(&results, &root);
        assert_eq!(md, "## Fallow: no issues found\n");
    }

    #[test]
    fn markdown_contains_header_with_count() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let md = build_markdown(&results, &root);
        assert!(md.starts_with(&format!(
            "## Fallow: {} issues found\n",
            results.total_issues()
        )));
    }

    #[test]
    fn markdown_contains_all_sections() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let md = build_markdown(&results, &root);

        assert!(md.contains("### Unused files (1)"));
        assert!(md.contains("### Unused exports (1)"));
        assert!(md.contains("### Unused type exports (1)"));
        assert!(md.contains("### Unused dependencies (1)"));
        assert!(md.contains("### Unused devDependencies (1)"));
        assert!(md.contains("### Unused enum members (1)"));
        assert!(md.contains("### Unused class members (1)"));
        assert!(md.contains("### Unresolved imports (1)"));
        assert!(md.contains("### Unlisted dependencies (1)"));
        assert!(md.contains("### Duplicate exports (1)"));
        assert!(md.contains("### Type-only dependencies"));
        assert!(md.contains("### Circular dependencies (1)"));
    }

    #[test]
    fn markdown_unused_file_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/dead.ts"),
        });
        let md = build_markdown(&results, &root);
        assert!(md.contains("- `src/dead.ts`"));
    }

    #[test]
    fn markdown_unused_export_grouped_by_file() {
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
        let md = build_markdown(&results, &root);
        assert!(md.contains("- `src/utils.ts`"));
        assert!(md.contains(":10 `helperFn`"));
    }

    #[test]
    fn markdown_re_export_tagged() {
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
        let md = build_markdown(&results, &root);
        assert!(md.contains("(re-export)"));
    }

    #[test]
    fn markdown_unused_dep_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_dependencies.push(UnusedDependency {
            package_name: "lodash".to_string(),
            location: DependencyLocation::Dependencies,
            path: root.join("package.json"),
            line: 5,
        });
        let md = build_markdown(&results, &root);
        assert!(md.contains("- `lodash`"));
    }

    #[test]
    fn markdown_circular_dep_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.circular_dependencies.push(CircularDependency {
            files: vec![root.join("src/a.ts"), root.join("src/b.ts")],
            length: 2,
            line: 3,
            col: 0,
        });
        let md = build_markdown(&results, &root);
        assert!(md.contains("`src/a.ts`"));
        assert!(md.contains("`src/b.ts`"));
        assert!(md.contains("\u{2192}"));
    }

    #[test]
    fn markdown_strips_root_prefix() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: PathBuf::from("/project/src/deep/nested/file.ts"),
        });
        let md = build_markdown(&results, &root);
        assert!(md.contains("`src/deep/nested/file.ts`"));
        assert!(!md.contains("/project/"));
    }

    #[test]
    fn markdown_single_issue_no_plural() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/dead.ts"),
        });
        let md = build_markdown(&results, &root);
        assert!(md.starts_with("## Fallow: 1 issue found\n"));
    }

    #[test]
    fn markdown_type_only_dep_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.type_only_dependencies.push(TypeOnlyDependency {
            package_name: "zod".to_string(),
            path: root.join("package.json"),
            line: 8,
        });
        let md = build_markdown(&results, &root);
        assert!(md.contains("### Type-only dependencies"));
        assert!(md.contains("- `zod`"));
    }

    #[test]
    fn markdown_escapes_backticks_in_export_names() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_exports.push(UnusedExport {
            path: root.join("src/utils.ts"),
            export_name: "foo`bar".to_string(),
            is_type_only: false,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        let md = build_markdown(&results, &root);
        assert!(md.contains("foo\\`bar"));
        assert!(!md.contains("foo`bar`"));
    }

    #[test]
    fn markdown_escapes_backticks_in_package_names() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_dependencies.push(UnusedDependency {
            package_name: "pkg`name".to_string(),
            location: DependencyLocation::Dependencies,
            path: root.join("package.json"),
            line: 5,
        });
        let md = build_markdown(&results, &root);
        assert!(md.contains("pkg\\`name"));
    }

    // ── Duplication markdown ──

    #[test]
    fn duplication_markdown_empty() {
        let report = DuplicationReport::default();
        let root = PathBuf::from("/project");
        let md = build_duplication_markdown(&report, &root);
        assert_eq!(md, "## Fallow: no code duplication found\n");
    }

    #[test]
    fn duplication_markdown_contains_groups() {
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
                total_files: 10,
                files_with_clones: 2,
                total_lines: 500,
                duplicated_lines: 20,
                total_tokens: 2500,
                duplicated_tokens: 100,
                clone_groups: 1,
                clone_instances: 2,
                duplication_percentage: 4.0,
            },
        };
        let md = build_duplication_markdown(&report, &root);
        assert!(md.contains("**Clone group 1**"));
        assert!(md.contains("`src/a.ts:1-10`"));
        assert!(md.contains("`src/b.ts:5-14`"));
        assert!(md.contains("4.0% duplication"));
    }

    #[test]
    fn duplication_markdown_contains_families() {
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
                files: vec![root.join("src/a.ts"), root.join("src/b.ts")],
                groups: vec![],
                total_duplicated_lines: 20,
                total_duplicated_tokens: 100,
                suggestions: vec![RefactoringSuggestion {
                    kind: RefactoringKind::ExtractFunction,
                    description: "Extract shared utility function".to_string(),
                    estimated_savings: 15,
                }],
            }],
            stats: DuplicationStats {
                clone_groups: 1,
                clone_instances: 1,
                duplication_percentage: 2.0,
                ..Default::default()
            },
        };
        let md = build_duplication_markdown(&report, &root);
        assert!(md.contains("### Clone Families"));
        assert!(md.contains("**Family 1**"));
        assert!(md.contains("Extract shared utility function"));
        assert!(md.contains("~15 lines saved"));
    }

    // ── Health markdown ──

    #[test]
    fn health_markdown_empty_no_findings() {
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
            vital_signs: None,
            file_scores: vec![],
            hotspots: vec![],
            hotspot_summary: None,
            targets: vec![],
            target_thresholds: None,
        };
        let md = build_health_markdown(&report, &root);
        assert!(md.contains("no functions exceed complexity thresholds"));
        assert!(md.contains("**50** functions analyzed"));
    }

    #[test]
    fn health_markdown_table_format() {
        let root = PathBuf::from("/project");
        let report = crate::health_types::HealthReport {
            findings: vec![crate::health_types::HealthFinding {
                path: root.join("src/utils.ts"),
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
            vital_signs: None,
            file_scores: vec![],
            hotspots: vec![],
            hotspot_summary: None,
            targets: vec![],
            target_thresholds: None,
        };
        let md = build_health_markdown(&report, &root);
        assert!(md.contains("## Fallow: 1 high complexity function\n"));
        assert!(md.contains("| File | Function |"));
        assert!(md.contains("`src/utils.ts:42`"));
        assert!(md.contains("`parseExpression`"));
        assert!(md.contains("25 **!**"));
        assert!(md.contains("30 **!**"));
        assert!(md.contains("| 80 |"));
    }

    #[test]
    fn health_markdown_no_marker_when_below_threshold() {
        let root = PathBuf::from("/project");
        let report = crate::health_types::HealthReport {
            findings: vec![crate::health_types::HealthFinding {
                path: root.join("src/utils.ts"),
                name: "helper".to_string(),
                line: 10,
                col: 0,
                cyclomatic: 15,
                cognitive: 20,
                line_count: 30,
                exceeded: crate::health_types::ExceededThreshold::Cognitive,
            }],
            summary: crate::health_types::HealthSummary {
                files_analyzed: 5,
                functions_analyzed: 20,
                functions_above_threshold: 1,
                max_cyclomatic_threshold: 20,
                max_cognitive_threshold: 15,
                files_scored: None,
                average_maintainability: None,
            },
            vital_signs: None,
            file_scores: vec![],
            hotspots: vec![],
            hotspot_summary: None,
            targets: vec![],
            target_thresholds: None,
        };
        let md = build_health_markdown(&report, &root);
        // Cyclomatic 15 is below threshold 20, no marker
        assert!(md.contains("| 15 |"));
        // Cognitive 20 exceeds threshold 15, has marker
        assert!(md.contains("20 **!**"));
    }

    #[test]
    fn health_markdown_with_targets() {
        use crate::health_types::*;

        let root = PathBuf::from("/project");
        let report = HealthReport {
            findings: vec![],
            summary: HealthSummary {
                files_analyzed: 10,
                functions_analyzed: 50,
                functions_above_threshold: 0,
                max_cyclomatic_threshold: 20,
                max_cognitive_threshold: 15,
                files_scored: None,
                average_maintainability: None,
            },
            vital_signs: None,
            file_scores: vec![],
            hotspots: vec![],
            hotspot_summary: None,
            targets: vec![
                RefactoringTarget {
                    path: PathBuf::from("/project/src/complex.ts"),
                    priority: 82.5,
                    efficiency: 27.5,
                    recommendation: "Split high-impact file".into(),
                    category: RecommendationCategory::SplitHighImpact,
                    effort: crate::health_types::EffortEstimate::High,
                    confidence: crate::health_types::Confidence::Medium,
                    factors: vec![ContributingFactor {
                        metric: "fan_in",
                        value: 25.0,
                        threshold: 10.0,
                        detail: "25 files depend on this".into(),
                    }],
                    evidence: None,
                },
                RefactoringTarget {
                    path: PathBuf::from("/project/src/legacy.ts"),
                    priority: 45.0,
                    efficiency: 45.0,
                    recommendation: "Remove 5 unused exports".into(),
                    category: RecommendationCategory::RemoveDeadCode,
                    effort: crate::health_types::EffortEstimate::Low,
                    confidence: crate::health_types::Confidence::High,
                    factors: vec![],
                    evidence: None,
                },
            ],
            target_thresholds: None,
        };
        let md = build_health_markdown(&report, &root);

        // Should have refactoring targets section
        assert!(
            md.contains("Refactoring Targets"),
            "should contain targets heading"
        );
        assert!(
            md.contains("src/complex.ts"),
            "should contain target file path"
        );
        assert!(md.contains("27.5"), "should contain efficiency score");
        assert!(
            md.contains("Split high-impact file"),
            "should contain recommendation"
        );
        assert!(md.contains("src/legacy.ts"), "should contain second target");
    }

    // ── Dependency in workspace package ──

    #[test]
    fn markdown_dep_in_workspace_shows_package_label() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_dependencies.push(UnusedDependency {
            package_name: "lodash".to_string(),
            location: DependencyLocation::Dependencies,
            path: root.join("packages/core/package.json"),
            line: 5,
        });
        let md = build_markdown(&results, &root);
        // Non-root package.json should show the label
        assert!(md.contains("(packages/core/package.json)"));
    }

    #[test]
    fn markdown_dep_at_root_no_extra_label() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_dependencies.push(UnusedDependency {
            package_name: "lodash".to_string(),
            location: DependencyLocation::Dependencies,
            path: root.join("package.json"),
            line: 5,
        });
        let md = build_markdown(&results, &root);
        assert!(md.contains("- `lodash`"));
        assert!(!md.contains("(package.json)"));
    }

    // ── Multiple exports same file grouped ──

    #[test]
    fn markdown_exports_grouped_by_file() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_exports.push(UnusedExport {
            path: root.join("src/utils.ts"),
            export_name: "alpha".to_string(),
            is_type_only: false,
            line: 5,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        results.unused_exports.push(UnusedExport {
            path: root.join("src/utils.ts"),
            export_name: "beta".to_string(),
            is_type_only: false,
            line: 10,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        results.unused_exports.push(UnusedExport {
            path: root.join("src/other.ts"),
            export_name: "gamma".to_string(),
            is_type_only: false,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        let md = build_markdown(&results, &root);
        // File header should appear only once for utils.ts
        let utils_count = md.matches("- `src/utils.ts`").count();
        assert_eq!(utils_count, 1, "file header should appear once per file");
        // Both exports should be under it as sub-items
        assert!(md.contains(":5 `alpha`"));
        assert!(md.contains(":10 `beta`"));
    }

    // ── Multiple issues plural header ──

    #[test]
    fn markdown_multiple_issues_plural() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/a.ts"),
        });
        results.unused_files.push(UnusedFile {
            path: root.join("src/b.ts"),
        });
        let md = build_markdown(&results, &root);
        assert!(md.starts_with("## Fallow: 2 issues found\n"));
    }

    // ── Duplication markdown with zero estimated savings ──

    #[test]
    fn duplication_markdown_zero_savings_no_suffix() {
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
                groups: vec![],
                total_duplicated_lines: 5,
                total_duplicated_tokens: 30,
                suggestions: vec![RefactoringSuggestion {
                    kind: RefactoringKind::ExtractFunction,
                    description: "Extract function".to_string(),
                    estimated_savings: 0,
                }],
            }],
            stats: DuplicationStats {
                clone_groups: 1,
                clone_instances: 1,
                duplication_percentage: 1.0,
                ..Default::default()
            },
        };
        let md = build_duplication_markdown(&report, &root);
        assert!(md.contains("Extract function"));
        assert!(!md.contains("lines saved"));
    }

    // ── Health markdown vital signs ──

    #[test]
    fn health_markdown_vital_signs_table() {
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
            vital_signs: Some(crate::health_types::VitalSigns {
                avg_cyclomatic: 3.5,
                p90_cyclomatic: 12,
                dead_file_pct: Some(5.0),
                dead_export_pct: Some(10.2),
                duplication_pct: None,
                maintainability_avg: Some(72.3),
                hotspot_count: Some(3),
                circular_dep_count: Some(1),
                unused_dep_count: Some(2),
            }),
            file_scores: vec![],
            hotspots: vec![],
            hotspot_summary: None,
            targets: vec![],
            target_thresholds: None,
        };
        let md = build_health_markdown(&report, &root);
        assert!(md.contains("## Vital Signs"));
        assert!(md.contains("| Metric | Value |"));
        assert!(md.contains("| Avg Cyclomatic | 3.5 |"));
        assert!(md.contains("| P90 Cyclomatic | 12 |"));
        assert!(md.contains("| Dead Files | 5.0% |"));
        assert!(md.contains("| Dead Exports | 10.2% |"));
        assert!(md.contains("| Maintainability (avg) | 72.3 |"));
        assert!(md.contains("| Hotspots | 3 |"));
        assert!(md.contains("| Circular Deps | 1 |"));
        assert!(md.contains("| Unused Deps | 2 |"));
    }

    // ── Health markdown file scores ──

    #[test]
    fn health_markdown_file_scores_table() {
        let root = PathBuf::from("/project");
        let report = crate::health_types::HealthReport {
            findings: vec![crate::health_types::HealthFinding {
                path: root.join("src/dummy.ts"),
                name: "fn".to_string(),
                line: 1,
                col: 0,
                cyclomatic: 25,
                cognitive: 20,
                line_count: 50,
                exceeded: crate::health_types::ExceededThreshold::Both,
            }],
            summary: crate::health_types::HealthSummary {
                files_analyzed: 5,
                functions_analyzed: 10,
                functions_above_threshold: 1,
                max_cyclomatic_threshold: 20,
                max_cognitive_threshold: 15,
                files_scored: Some(1),
                average_maintainability: Some(65.0),
            },
            vital_signs: None,
            file_scores: vec![crate::health_types::FileHealthScore {
                path: root.join("src/utils.ts"),
                fan_in: 5,
                fan_out: 3,
                dead_code_ratio: 0.25,
                complexity_density: 0.8,
                maintainability_index: 72.5,
                total_cyclomatic: 40,
                total_cognitive: 30,
                function_count: 10,
                lines: 200,
            }],
            hotspots: vec![],
            hotspot_summary: None,
            targets: vec![],
            target_thresholds: None,
        };
        let md = build_health_markdown(&report, &root);
        assert!(md.contains("### File Health Scores (1 files)"));
        assert!(md.contains("| File | MI | Fan-in | Fan-out | Dead Code | Density |"));
        assert!(md.contains("| `src/utils.ts` | 72.5 | 5 | 3 | 25% | 0.80 |"));
        assert!(md.contains("**Average maintainability index:** 65.0/100"));
    }

    // ── Health markdown hotspots ──

    #[test]
    fn health_markdown_hotspots_table() {
        let root = PathBuf::from("/project");
        let report = crate::health_types::HealthReport {
            findings: vec![crate::health_types::HealthFinding {
                path: root.join("src/dummy.ts"),
                name: "fn".to_string(),
                line: 1,
                col: 0,
                cyclomatic: 25,
                cognitive: 20,
                line_count: 50,
                exceeded: crate::health_types::ExceededThreshold::Both,
            }],
            summary: crate::health_types::HealthSummary {
                files_analyzed: 5,
                functions_analyzed: 10,
                functions_above_threshold: 1,
                max_cyclomatic_threshold: 20,
                max_cognitive_threshold: 15,
                files_scored: None,
                average_maintainability: None,
            },
            vital_signs: None,
            file_scores: vec![],
            hotspots: vec![crate::health_types::HotspotEntry {
                path: root.join("src/hot.ts"),
                score: 85.0,
                commits: 42,
                weighted_commits: 35.0,
                lines_added: 500,
                lines_deleted: 200,
                complexity_density: 1.2,
                fan_in: 10,
                trend: fallow_core::churn::ChurnTrend::Accelerating,
            }],
            hotspot_summary: Some(crate::health_types::HotspotSummary {
                since: "6 months".to_string(),
                min_commits: 3,
                files_analyzed: 50,
                files_excluded: 5,
                shallow_clone: false,
            }),
            targets: vec![],
            target_thresholds: None,
        };
        let md = build_health_markdown(&report, &root);
        assert!(md.contains("### Hotspots (1 files, since 6 months)"));
        assert!(md.contains("| `src/hot.ts` | 85.0 | 42 | 700 | 1.20 | 10 | accelerating |"));
        assert!(md.contains("*5 files excluded (< 3 commits)*"));
    }

    // ── Health markdown metric legend ──

    #[test]
    fn health_markdown_metric_legend_with_scores() {
        let root = PathBuf::from("/project");
        let report = crate::health_types::HealthReport {
            findings: vec![crate::health_types::HealthFinding {
                path: root.join("src/x.ts"),
                name: "f".to_string(),
                line: 1,
                col: 0,
                cyclomatic: 25,
                cognitive: 20,
                line_count: 10,
                exceeded: crate::health_types::ExceededThreshold::Both,
            }],
            summary: crate::health_types::HealthSummary {
                files_analyzed: 1,
                functions_analyzed: 1,
                functions_above_threshold: 1,
                max_cyclomatic_threshold: 20,
                max_cognitive_threshold: 15,
                files_scored: Some(1),
                average_maintainability: Some(70.0),
            },
            vital_signs: None,
            file_scores: vec![crate::health_types::FileHealthScore {
                path: root.join("src/x.ts"),
                fan_in: 1,
                fan_out: 1,
                dead_code_ratio: 0.0,
                complexity_density: 0.5,
                maintainability_index: 80.0,
                total_cyclomatic: 10,
                total_cognitive: 8,
                function_count: 2,
                lines: 50,
            }],
            hotspots: vec![],
            hotspot_summary: None,
            targets: vec![],
            target_thresholds: None,
        };
        let md = build_health_markdown(&report, &root);
        assert!(md.contains("<details><summary>Metric definitions</summary>"));
        assert!(md.contains("**MI** \u{2014} Maintainability Index"));
        assert!(md.contains("**Fan-in**"));
        assert!(md.contains("Full metric reference"));
    }

    // ── Health markdown truncated findings ──

    #[test]
    fn health_markdown_truncated_findings_shown_count() {
        let root = PathBuf::from("/project");
        let report = crate::health_types::HealthReport {
            findings: vec![crate::health_types::HealthFinding {
                path: root.join("src/x.ts"),
                name: "f".to_string(),
                line: 1,
                col: 0,
                cyclomatic: 25,
                cognitive: 20,
                line_count: 10,
                exceeded: crate::health_types::ExceededThreshold::Both,
            }],
            summary: crate::health_types::HealthSummary {
                files_analyzed: 10,
                functions_analyzed: 50,
                functions_above_threshold: 5, // 5 total but only 1 shown
                max_cyclomatic_threshold: 20,
                max_cognitive_threshold: 15,
                files_scored: None,
                average_maintainability: None,
            },
            vital_signs: None,
            file_scores: vec![],
            hotspots: vec![],
            hotspot_summary: None,
            targets: vec![],
            target_thresholds: None,
        };
        let md = build_health_markdown(&report, &root);
        assert!(md.contains("5 high complexity functions (1 shown)"));
    }

    // ── escape_backticks ──

    #[test]
    fn escape_backticks_handles_multiple() {
        assert_eq!(escape_backticks("a`b`c"), "a\\`b\\`c");
    }

    #[test]
    fn escape_backticks_no_backticks_unchanged() {
        assert_eq!(escape_backticks("hello"), "hello");
    }

    // ── Unresolved import in markdown ──

    #[test]
    fn markdown_unresolved_import_grouped_by_file() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unresolved_imports.push(UnresolvedImport {
            path: root.join("src/app.ts"),
            specifier: "./missing".to_string(),
            line: 3,
            col: 0,
            specifier_col: 0,
        });
        let md = build_markdown(&results, &root);
        assert!(md.contains("### Unresolved imports (1)"));
        assert!(md.contains("- `src/app.ts`"));
        assert!(md.contains(":3 `./missing`"));
    }

    // ── Markdown optional dep ──

    #[test]
    fn markdown_unused_optional_dep() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_optional_dependencies.push(UnusedDependency {
            package_name: "fsevents".to_string(),
            location: DependencyLocation::OptionalDependencies,
            path: root.join("package.json"),
            line: 12,
        });
        let md = build_markdown(&results, &root);
        assert!(md.contains("### Unused optionalDependencies (1)"));
        assert!(md.contains("- `fsevents`"));
    }

    // ── Health markdown no hotspot exclusion message when 0 excluded ──

    #[test]
    fn health_markdown_hotspots_no_excluded_message() {
        let root = PathBuf::from("/project");
        let report = crate::health_types::HealthReport {
            findings: vec![crate::health_types::HealthFinding {
                path: root.join("src/x.ts"),
                name: "f".to_string(),
                line: 1,
                col: 0,
                cyclomatic: 25,
                cognitive: 20,
                line_count: 10,
                exceeded: crate::health_types::ExceededThreshold::Both,
            }],
            summary: crate::health_types::HealthSummary {
                files_analyzed: 5,
                functions_analyzed: 10,
                functions_above_threshold: 1,
                max_cyclomatic_threshold: 20,
                max_cognitive_threshold: 15,
                files_scored: None,
                average_maintainability: None,
            },
            vital_signs: None,
            file_scores: vec![],
            hotspots: vec![crate::health_types::HotspotEntry {
                path: root.join("src/hot.ts"),
                score: 50.0,
                commits: 10,
                weighted_commits: 8.0,
                lines_added: 100,
                lines_deleted: 50,
                complexity_density: 0.5,
                fan_in: 3,
                trend: fallow_core::churn::ChurnTrend::Stable,
            }],
            hotspot_summary: Some(crate::health_types::HotspotSummary {
                since: "6 months".to_string(),
                min_commits: 3,
                files_analyzed: 50,
                files_excluded: 0,
                shallow_clone: false,
            }),
            targets: vec![],
            target_thresholds: None,
        };
        let md = build_health_markdown(&report, &root);
        assert!(!md.contains("files excluded"));
    }

    // ── Duplication markdown plural ──

    #[test]
    fn duplication_markdown_single_group_no_plural() {
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
            clone_families: vec![],
            stats: DuplicationStats {
                clone_groups: 1,
                clone_instances: 1,
                duplication_percentage: 2.0,
                ..Default::default()
            },
        };
        let md = build_duplication_markdown(&report, &root);
        assert!(md.contains("1 clone group found"));
        assert!(!md.contains("1 clone groups found"));
    }
}
