use std::path::Path;
use std::time::Duration;

use colored::Colorize;

use super::{MAX_FLAT_ITEMS, format_path, relative_path, split_dir_filename};

/// Docs base URL for health explanations.
const DOCS_HEALTH: &str = "https://docs.fallow.tools/explanations/health";

pub(in crate::report) fn print_health_human(
    report: &crate::health_types::HealthReport,
    root: &Path,
    elapsed: Duration,
    quiet: bool,
) {
    if !quiet {
        eprintln!();
    }

    if report.findings.is_empty()
        && report.file_scores.is_empty()
        && report.hotspots.is_empty()
        && report.targets.is_empty()
    {
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
pub(in crate::report) fn build_health_human_lines(
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

    // Refactoring targets (last section — synthesis of data above)
    if !report.targets.is_empty() {
        lines.push(format!(
            "{} {}",
            "\u{25cf}".cyan(),
            format!("Refactoring targets ({})", report.targets.len())
                .cyan()
                .bold()
        ));
        lines.push(String::new());

        let shown_targets = report.targets.len().min(MAX_FLAT_ITEMS);
        for target in &report.targets[..shown_targets] {
            let file_str = relative_path(&target.path, root).display().to_string();

            // Priority score: color-coded by urgency
            let score_str = format!("{:>5.1}", target.priority);
            let score_colored = if target.priority >= 70.0 {
                score_str.red().bold().to_string()
            } else if target.priority >= 40.0 {
                score_str.yellow().to_string()
            } else {
                score_str.green().to_string()
            };

            // Path: dim directory, normal filename
            let (dir, filename) = split_dir_filename(&file_str);

            // Line 1: priority score + path
            lines.push(format!(
                "  {}    {}{}",
                score_colored,
                dir.dimmed(),
                filename,
            ));

            // Line 2: category label (yellow) + recommendation (dimmed)
            let label = target.category.label();
            lines.push(format!(
                "         {}  {}",
                label.yellow(),
                target.recommendation.dimmed(),
            ));

            // Blank line between entries
            lines.push(String::new());
        }
        if report.targets.len() > MAX_FLAT_ITEMS {
            lines.push(format!(
                "  {}",
                format!(
                    "... and {} more targets",
                    report.targets.len() - MAX_FLAT_ITEMS
                )
                .dimmed()
            ));
            lines.push(String::new());
        }
        lines.push(format!(
            "  {}",
            format!(
                "Prioritized refactoring recommendations based on complexity, churn, and coupling signals \u{2014} {DOCS_HEALTH}#refactoring-targets"
            )
            .dimmed()
        ));
        lines.push(String::new());
    }

    lines
}
