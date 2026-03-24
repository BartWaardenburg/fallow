use std::process::ExitCode;
use std::time::Instant;

use fallow_config::OutputFormat;

use crate::baseline::{HealthBaselineData, filter_new_health_findings};
use crate::check::{get_changed_files, resolve_workspace_filter};
pub use crate::health_types::*;
use crate::load_config;
use crate::report;

/// Sort criteria for complexity output.
#[derive(Clone, clap::ValueEnum)]
pub enum SortBy {
    Cyclomatic,
    Cognitive,
    Lines,
}

pub struct HealthOptions<'a> {
    pub root: &'a std::path::Path,
    pub config_path: &'a Option<std::path::PathBuf>,
    pub output: OutputFormat,
    pub no_cache: bool,
    pub threads: usize,
    pub quiet: bool,
    pub max_cyclomatic: Option<u16>,
    pub max_cognitive: Option<u16>,
    pub top: Option<usize>,
    pub sort: SortBy,
    pub production: bool,
    pub changed_since: Option<&'a str>,
    pub workspace: Option<&'a str>,
    pub baseline: Option<&'a std::path::Path>,
    pub save_baseline: Option<&'a std::path::Path>,
    pub complexity: bool,
    pub file_scores: bool,
    pub hotspots: bool,
    pub since: Option<&'a str>,
    pub min_commits: Option<u32>,
    pub explain: bool,
}

pub fn run_health(opts: &HealthOptions<'_>) -> ExitCode {
    let start = Instant::now();

    let config = match load_config(
        opts.root,
        opts.config_path,
        opts.output.clone(),
        opts.no_cache,
        opts.threads,
        opts.production,
        opts.quiet,
    ) {
        Ok(c) => c,
        Err(code) => return code,
    };

    // Resolve thresholds: CLI flags override config
    let max_cyclomatic = opts.max_cyclomatic.unwrap_or(config.health.max_cyclomatic);
    let max_cognitive = opts.max_cognitive.unwrap_or(config.health.max_cognitive);

    // Discover files
    let files = fallow_core::discover::discover_files(&config);

    // Parse all files (complexity is computed during parsing)
    let cache = if config.no_cache {
        None
    } else {
        fallow_core::cache::CacheStore::load(&config.cache_dir)
    };
    let parse_result = fallow_core::extract::parse_all_files(&files, cache.as_ref());

    // Build ignore globs from config (using globset for consistency with the rest of the codebase)
    let ignore_set = {
        let mut builder = globset::GlobSetBuilder::new();
        for pattern in &config.health.ignore {
            match globset::Glob::new(pattern) {
                Ok(glob) => {
                    builder.add(glob);
                }
                Err(e) => {
                    eprintln!("Warning: Invalid health ignore pattern '{pattern}': {e}");
                }
            }
        }
        builder
            .build()
            .unwrap_or_else(|_| globset::GlobSet::empty())
    };

    // Get changed files for --changed-since filtering
    let changed_files = opts
        .changed_since
        .and_then(|git_ref| get_changed_files(opts.root, git_ref));

    // Resolve workspace filter once — reused for both findings and file scores
    let ws_root = if let Some(ws_name) = opts.workspace {
        match resolve_workspace_filter(opts.root, ws_name, &opts.output) {
            Ok(root) => Some(root),
            Err(code) => return code,
        }
    } else {
        None
    };

    // Build FileId → path lookup for O(1) access
    let file_paths: rustc_hash::FxHashMap<_, _> = files.iter().map(|f| (f.id, &f.path)).collect();

    // Collect findings
    let mut files_analyzed = 0usize;
    let mut total_functions = 0usize;
    let mut findings: Vec<HealthFinding> = Vec::new();

    for module in &parse_result.modules {
        let Some(path) = file_paths.get(&module.file_id) else {
            continue;
        };

        // Apply ignore patterns
        let relative = path.strip_prefix(&config.root).unwrap_or(path);
        if ignore_set.is_match(relative) {
            continue;
        }

        // Apply changed-since filter
        if let Some(ref changed) = changed_files
            && !changed.contains(*path)
        {
            continue;
        }

        files_analyzed += 1;
        for fc in &module.complexity {
            total_functions += 1;
            let exceeds_cyclomatic = fc.cyclomatic > max_cyclomatic;
            let exceeds_cognitive = fc.cognitive > max_cognitive;
            if exceeds_cyclomatic || exceeds_cognitive {
                let exceeded = match (exceeds_cyclomatic, exceeds_cognitive) {
                    (true, true) => ExceededThreshold::Both,
                    (true, false) => ExceededThreshold::Cyclomatic,
                    (false, true) => ExceededThreshold::Cognitive,
                    (false, false) => unreachable!(),
                };
                findings.push(HealthFinding {
                    path: (*path).clone(),
                    name: fc.name.clone(),
                    line: fc.line,
                    col: fc.col,
                    cyclomatic: fc.cyclomatic,
                    cognitive: fc.cognitive,
                    line_count: fc.line_count,
                    exceeded,
                });
            }
        }
    }

    // Apply workspace filter (resolved once above, reused for file scores too)
    if let Some(ref ws) = ws_root {
        findings.retain(|f| f.path.starts_with(ws));
    }

    // Sort findings
    match opts.sort {
        SortBy::Cyclomatic => findings.sort_by(|a, b| b.cyclomatic.cmp(&a.cyclomatic)),
        SortBy::Cognitive => findings.sort_by(|a, b| b.cognitive.cmp(&a.cognitive)),
        SortBy::Lines => findings.sort_by(|a, b| b.line_count.cmp(&a.line_count)),
    }

    // Save baseline (before filtering, captures full state)
    if let Some(save_path) = opts.save_baseline {
        let baseline = HealthBaselineData::from_findings(&findings, &config.root);
        match serde_json::to_string_pretty(&baseline) {
            Ok(json) => {
                if let Err(e) = std::fs::write(save_path, json) {
                    eprintln!("Error: failed to save health baseline: {e}");
                    return ExitCode::from(2);
                }
                if !opts.quiet {
                    eprintln!("Saved health baseline to {}", save_path.display());
                }
            }
            Err(e) => {
                eprintln!("Error: failed to serialize health baseline: {e}");
                return ExitCode::from(2);
            }
        }
    }

    // Capture total above threshold before baseline filtering
    let total_above_threshold = findings.len();

    // Filter against baseline
    if let Some(load_path) = opts.baseline {
        match std::fs::read_to_string(load_path) {
            Ok(json) => match serde_json::from_str::<HealthBaselineData>(&json) {
                Ok(baseline) => {
                    findings = filter_new_health_findings(findings, &baseline, &config.root);
                }
                Err(e) => {
                    eprintln!("Error: failed to parse health baseline: {e}");
                    return ExitCode::from(2);
                }
            },
            Err(e) => {
                eprintln!("Error: failed to read health baseline: {e}");
                return ExitCode::from(2);
            }
        }
    }

    // Apply --top limit
    if let Some(top) = opts.top {
        findings.truncate(top);
    }

    // Compute file-level health scores when requested or when hotspots need them.
    // NOTE: This runs the full analysis pipeline (discovery, parsing, graph, dead code detection)
    // a second time because there is no API to inject pre-parsed modules into the analysis
    // pipeline. The cache mitigates re-parsing cost but the discovery and graph construction
    // are repeated. Future optimization: expose a lower-level API that accepts ParseResult.
    let needs_file_scores = opts.file_scores || opts.hotspots;
    let (mut file_scores, files_scored, average_maintainability) = if needs_file_scores {
        match compute_file_scores(
            &config,
            &parse_result.modules,
            &file_paths,
            changed_files.as_ref(),
        ) {
            Ok(mut scores) => {
                // Apply the same filters that findings get: workspace, ignore globs
                if let Some(ref ws) = ws_root {
                    scores.retain(|s| s.path.starts_with(ws));
                }
                if !ignore_set.is_empty() {
                    scores.retain(|s| {
                        let relative = s.path.strip_prefix(&config.root).unwrap_or(&s.path);
                        !ignore_set.is_match(relative)
                    });
                }
                // Compute average BEFORE --top truncation so it reflects the full project
                let total_scored = scores.len();
                let avg = if total_scored > 0 {
                    let sum: f64 = scores.iter().map(|s| s.maintainability_index).sum();
                    Some((sum / total_scored as f64 * 10.0).round() / 10.0)
                } else {
                    None
                };
                (scores, Some(total_scored), avg)
            }
            Err(e) => {
                eprintln!("Warning: failed to compute file scores: {e}");
                // Use Some(0) so JSON consumers can distinguish "flag not set" (field absent)
                // from "flag set but failed" (files_scored: 0).
                (Vec::new(), Some(0), None)
            }
        }
    } else {
        (Vec::new(), None, None)
    };

    // Compute hotspot analysis when requested.
    let (hotspots, hotspot_summary) = if opts.hotspots {
        compute_hotspots(opts, &config, &file_scores, &ignore_set, ws_root.as_deref())
    } else {
        (Vec::new(), None)
    };

    // Apply --top to file scores (after hotspot computation which uses the full list)
    if opts.file_scores {
        if let Some(top) = opts.top {
            file_scores.truncate(top);
        }
    } else {
        // If file_scores was only computed for hotspots, don't include it in the report
        file_scores.clear();
    }

    let report = HealthReport {
        summary: HealthSummary {
            files_analyzed,
            functions_analyzed: total_functions,
            functions_above_threshold: total_above_threshold,
            max_cyclomatic_threshold: max_cyclomatic,
            max_cognitive_threshold: max_cognitive,
            files_scored: if opts.file_scores { files_scored } else { None },
            average_maintainability: if opts.file_scores {
                average_maintainability
            } else {
                None
            },
        },
        findings: if opts.complexity {
            findings
        } else {
            Vec::new()
        },
        file_scores,
        hotspots,
        hotspot_summary,
    };

    let elapsed = start.elapsed();

    // Print report
    let result = report::print_health_report(
        &report,
        &config,
        elapsed,
        opts.quiet,
        &opts.output,
        opts.explain,
    );
    if result != ExitCode::SUCCESS {
        return result;
    }

    // Exit code 1 if there are findings
    if !report.findings.is_empty() {
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}

/// Validate git prerequisites and return churn data for hotspot analysis.
///
/// Returns `None` (with an error printed) if the repo is invalid, `--since` is
/// malformed, or git analysis fails.
fn fetch_churn_data(
    opts: &HealthOptions<'_>,
) -> Option<(
    fallow_core::churn::ChurnResult,
    fallow_core::churn::SinceDuration,
)> {
    use fallow_core::churn;

    if !churn::is_git_repo(opts.root) {
        eprintln!("Error: hotspot analysis requires a git repository");
        return None;
    }

    let since_input = opts.since.unwrap_or("6m");
    if let Err(e) = crate::validate::validate_no_control_chars(since_input, "--since") {
        eprintln!("Error: {e}");
        return None;
    }
    let since = match churn::parse_since(since_input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: invalid --since: {e}");
            return None;
        }
    };

    let churn_result = churn::analyze_churn(opts.root, &since)?;
    Some((churn_result, since))
}

/// Find the maximum weighted-commits and complexity-density across eligible files.
///
/// Used to normalize hotspot scores into the 0–100 range.
fn compute_normalization_maxima(
    file_scores: &[FileHealthScore],
    churn_files: &rustc_hash::FxHashMap<std::path::PathBuf, fallow_core::churn::FileChurn>,
    min_commits: u32,
) -> (f64, f64) {
    let mut max_weighted: f64 = 0.0;
    let mut max_density: f64 = 0.0;
    for score in file_scores {
        if let Some(churn) = churn_files.get(&score.path)
            && churn.commits >= min_commits
        {
            max_weighted = max_weighted.max(churn.weighted_commits);
            max_density = max_density.max(score.complexity_density);
        }
    }
    (max_weighted, max_density)
}

/// Check whether a file should be excluded from hotspot results
/// based on workspace filter and ignore patterns.
fn is_excluded_from_hotspots(
    path: &std::path::Path,
    root: &std::path::Path,
    ignore_set: &globset::GlobSet,
    ws_root: Option<&std::path::Path>,
) -> bool {
    if let Some(ws) = ws_root
        && !path.starts_with(ws)
    {
        return true;
    }
    if !ignore_set.is_empty() {
        let relative = path.strip_prefix(root).unwrap_or(path);
        if ignore_set.is_match(relative) {
            return true;
        }
    }
    false
}

/// Compute a normalized hotspot score from churn and complexity data.
///
/// Both inputs are normalized against their respective maxima so the result
/// falls in the 0–100 range (rounded to one decimal).
fn compute_hotspot_score(
    weighted_commits: f64,
    max_weighted: f64,
    complexity_density: f64,
    max_density: f64,
) -> f64 {
    let norm_churn = if max_weighted > 0.0 {
        weighted_commits / max_weighted
    } else {
        0.0
    };
    let norm_complexity = if max_density > 0.0 {
        complexity_density / max_density
    } else {
        0.0
    };
    (norm_churn * norm_complexity * 100.0 * 10.0).round() / 10.0
}

/// Compute hotspot entries by combining git churn data with file health scores.
fn compute_hotspots(
    opts: &HealthOptions<'_>,
    config: &fallow_config::ResolvedConfig,
    file_scores: &[FileHealthScore],
    ignore_set: &globset::GlobSet,
    ws_root: Option<&std::path::Path>,
) -> (Vec<HotspotEntry>, Option<HotspotSummary>) {
    let Some((churn_result, since)) = fetch_churn_data(opts) else {
        return (Vec::new(), None);
    };

    // Warn about shallow clones (read from churn result to avoid redundant git call)
    let shallow_clone = churn_result.shallow_clone;
    if shallow_clone && !opts.quiet {
        eprintln!(
            "Warning: shallow clone detected. Hotspot analysis may be incomplete. \
             Use `git fetch --unshallow` for full history."
        );
    }

    let min_commits = opts.min_commits.unwrap_or(3);
    let (max_weighted, max_density) =
        compute_normalization_maxima(file_scores, &churn_result.files, min_commits);

    // Build hotspot entries
    let mut hotspot_entries = Vec::new();
    let mut files_excluded: usize = 0;

    for score in file_scores {
        if is_excluded_from_hotspots(&score.path, &config.root, ignore_set, ws_root) {
            continue;
        }

        let Some(churn) = churn_result.files.get(&score.path) else {
            continue;
        };
        if churn.commits < min_commits {
            files_excluded += 1;
            continue;
        }

        hotspot_entries.push(HotspotEntry {
            path: score.path.clone(),
            score: compute_hotspot_score(
                churn.weighted_commits,
                max_weighted,
                score.complexity_density,
                max_density,
            ),
            commits: churn.commits,
            weighted_commits: churn.weighted_commits,
            lines_added: churn.lines_added,
            lines_deleted: churn.lines_deleted,
            complexity_density: score.complexity_density,
            fan_in: score.fan_in,
            trend: churn.trend,
        });
    }

    // Sort by score descending (highest risk first)
    hotspot_entries.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Compute summary BEFORE --top truncation
    let files_analyzed = hotspot_entries.len();
    let summary = HotspotSummary {
        since: since.display,
        min_commits,
        files_analyzed,
        files_excluded,
        shallow_clone,
    };

    // Apply --top to hotspots
    if let Some(top) = opts.top {
        hotspot_entries.truncate(top);
    }

    (hotspot_entries, Some(summary))
}

/// Aggregate complexity totals from a parsed module.
///
/// Returns `(total_cyclomatic, total_cognitive, function_count, lines)`.
fn aggregate_complexity(module: &fallow_core::extract::ModuleInfo) -> (u32, u32, usize, u32) {
    let cyc: u32 = module
        .complexity
        .iter()
        .map(|f| u32::from(f.cyclomatic))
        .sum();
    let cog: u32 = module
        .complexity
        .iter()
        .map(|f| u32::from(f.cognitive))
        .sum();
    let funcs = module.complexity.len();
    // line_offsets length = number of lines in the file
    let lines = module.line_offsets.len() as u32;
    (cyc, cog, funcs, lines)
}

/// Compute the dead code ratio for a single file.
///
/// Returns the fraction of VALUE exports with zero references (0.0–1.0).
/// Type-only exports (interfaces, type aliases) are excluded from both
/// numerator and denominator to avoid inflating the ratio for well-typed
/// codebases. Returns 1.0 if the entire file is unused, 0.0 if it has no
/// value exports.
fn compute_dead_code_ratio(
    path: &std::path::Path,
    exports: &[fallow_core::graph::ExportSymbol],
    unused_files: &rustc_hash::FxHashSet<&std::path::Path>,
    unused_exports_by_path: &rustc_hash::FxHashMap<&std::path::Path, usize>,
) -> f64 {
    if unused_files.contains(path) {
        return 1.0;
    }
    let value_exports = exports.iter().filter(|e| !e.is_type_only).count();
    if value_exports == 0 {
        return 0.0;
    }
    let unused = unused_exports_by_path.get(path).copied().unwrap_or(0);
    (unused as f64 / value_exports as f64).min(1.0)
}

/// Compute complexity density: total cyclomatic / lines of code.
///
/// Returns 0.0 when the file has no lines.
fn compute_complexity_density(total_cyclomatic: u32, lines: u32) -> f64 {
    if lines > 0 {
        f64::from(total_cyclomatic) / f64::from(lines)
    } else {
        0.0
    }
}

/// Count unused VALUE exports per file path for O(1) lookup.
///
/// Type-only exports (interfaces, type aliases) are intentionally excluded —
/// they are a different concern than unused functions/components.
fn count_unused_exports_by_path(
    unused_exports: &[fallow_core::results::UnusedExport],
) -> rustc_hash::FxHashMap<&std::path::Path, usize> {
    let mut map: rustc_hash::FxHashMap<&std::path::Path, usize> = rustc_hash::FxHashMap::default();
    for exp in unused_exports {
        *map.entry(exp.path.as_path()).or_default() += 1;
    }
    map
}

/// Compute per-file health scores by running the full analysis pipeline.
///
/// This builds the module graph and runs dead code detection to obtain
/// fan-in, fan-out, and dead code ratio per file. Complexity density is
/// derived from the already-parsed modules.
fn compute_file_scores(
    config: &fallow_config::ResolvedConfig,
    modules: &[fallow_core::extract::ModuleInfo],
    file_paths: &rustc_hash::FxHashMap<fallow_core::discover::FileId, &std::path::PathBuf>,
    changed_files: Option<&rustc_hash::FxHashSet<std::path::PathBuf>>,
) -> Result<Vec<FileHealthScore>, String> {
    // Run full analysis to get the graph and dead code results
    let output = fallow_core::analyze_with_trace(config).map_err(|e| format!("{e}"))?;
    let graph = output.graph.ok_or("graph not available")?;
    let results = &output.results;

    // Build a set of unused file paths for O(1) lookup
    let unused_files: rustc_hash::FxHashSet<&std::path::Path> = results
        .unused_files
        .iter()
        .map(|f| f.path.as_path())
        .collect();

    let unused_exports_by_path = count_unused_exports_by_path(&results.unused_exports);

    // Build FileId → ModuleInfo lookup
    let module_by_id: rustc_hash::FxHashMap<
        fallow_core::discover::FileId,
        &fallow_core::extract::ModuleInfo,
    > = modules.iter().map(|m| (m.file_id, m)).collect();

    let mut scores = Vec::with_capacity(graph.modules.len());

    for node in &graph.modules {
        let Some(path) = file_paths.get(&node.file_id) else {
            continue;
        };

        // Fan-in: number of files that import this file
        let fan_in = graph
            .reverse_deps
            .get(node.file_id.0 as usize)
            .map_or(0, Vec::len);

        // Fan-out: number of files this file imports (from edge_range)
        let fan_out = node.edge_range.len();

        let (total_cyclomatic, total_cognitive, function_count, lines) =
            match module_by_id.get(&node.file_id) {
                Some(module) => aggregate_complexity(module),
                None => (0, 0, 0, 0),
            };

        let dead_code_ratio = compute_dead_code_ratio(
            (*path).as_path(),
            &node.exports,
            &unused_files,
            &unused_exports_by_path,
        );
        let complexity_density = compute_complexity_density(total_cyclomatic, lines);

        // Round intermediate values first so the MI in JSON is reproducible
        // from the other rounded fields in the same JSON object.
        let dead_code_ratio_rounded = (dead_code_ratio * 100.0).round() / 100.0;
        let complexity_density_rounded = (complexity_density * 100.0).round() / 100.0;

        let maintainability_index = compute_maintainability_index(
            complexity_density_rounded,
            dead_code_ratio_rounded,
            fan_out,
        );

        scores.push(FileHealthScore {
            path: (*path).clone(),
            fan_in,
            fan_out,
            dead_code_ratio: dead_code_ratio_rounded,
            complexity_density: complexity_density_rounded,
            maintainability_index: (maintainability_index * 10.0).round() / 10.0,
            total_cyclomatic,
            total_cognitive,
            function_count,
            lines,
        });
    }

    // Apply --changed-since filter to keep scores consistent with findings
    if let Some(changed) = changed_files {
        scores.retain(|s| changed.contains(&s.path));
    }

    // Exclude zero-function files (barrel/re-export files) by default.
    // These have zero complexity density and can only be penalized by dead_code_ratio
    // and fan-out, making their MI a dead-code detector rather than a maintainability
    // metric. They pollute the rankings and obscure actually complex files.
    scores.retain(|s| s.function_count > 0);

    // Sort by maintainability index ascending (worst files first)
    scores.sort_by(|a, b| {
        a.maintainability_index
            .partial_cmp(&b.maintainability_index)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(scores)
}

/// Compute the maintainability index for a single file.
///
/// Formula:
/// ```text
/// fan_out_penalty = min(ln(fan_out + 1) × 4, 15)
/// MI = 100 - (complexity_density × 30) - (dead_code_ratio × 20) - fan_out_penalty
/// ```
///
/// Fan-out uses logarithmic scaling capped at 15 points to reflect diminishing
/// marginal risk (the 30th import is less concerning than the 5th) and prevent
/// composition-root files from being unfairly penalized.
///
/// Clamped to \[0, 100\]. Higher is better.
fn compute_maintainability_index(
    complexity_density: f64,
    dead_code_ratio: f64,
    fan_out: usize,
) -> f64 {
    let fan_out_penalty = ((fan_out as f64).ln_1p() * 4.0).min(15.0);
    // Keep the formula readable — it matches the documented specification.
    #[expect(clippy::suboptimal_flops)]
    let score = 100.0 - (complexity_density * 30.0) - (dead_code_ratio * 20.0) - fan_out_penalty;
    score.clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maintainability_perfect_score() {
        // No complexity, no dead code, no fan-out → 100
        assert!((compute_maintainability_index(0.0, 0.0, 0) - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn maintainability_clamped_at_zero() {
        // Very high complexity density → clamped to 0
        assert!((compute_maintainability_index(10.0, 1.0, 100) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn maintainability_formula_correct() {
        // complexity_density=0.5, dead_code_ratio=0.3, fan_out=10
        // fan_out_penalty = min(ln(11) * 4, 15) = min(9.59, 15) = 9.59
        // 100 - 15 - 6 - 9.59 = 69.41
        let result = compute_maintainability_index(0.5, 0.3, 10);
        let expected = 100.0 - 15.0 - 6.0 - (11.0_f64.ln() * 4.0);
        assert!((result - expected).abs() < 0.01);
    }

    #[test]
    fn maintainability_dead_file_penalty() {
        // Fully dead file: dead_code_ratio=1.0, fan_out=0
        // fan_out_penalty = min(ln(1) * 4, 15) = 0
        // 100 - 0 - 20 - 0 = 80
        let result = compute_maintainability_index(0.0, 1.0, 0);
        assert!((result - 80.0).abs() < f64::EPSILON);
    }

    #[test]
    fn maintainability_fan_out_is_logarithmic() {
        // fan_out=10: penalty = min(ln(11) * 4, 15) ≈ 9.59
        let result_10 = compute_maintainability_index(0.0, 0.0, 10);
        // fan_out=100: penalty = min(ln(101) * 4, 15) = 15 (capped)
        let result_100 = compute_maintainability_index(0.0, 0.0, 100);
        // fan_out=200: also capped at 15
        let result_200 = compute_maintainability_index(0.0, 0.0, 200);

        // Logarithmic: 10→100 jump is much less than 10× the penalty
        assert!(result_10 > 90.0); // ~90.4
        assert!(result_100 > 84.0); // 85.0 (capped)
        // Capped: 100 and 200 should score the same
        assert!((result_100 - result_200).abs() < f64::EPSILON);
    }

    #[test]
    fn maintainability_fan_out_capped_at_15() {
        // Very high fan-out should not push score below 65 (100 - 0 - 20 - 15)
        // even with full dead code
        let result = compute_maintainability_index(0.0, 1.0, 1000);
        assert!((result - 65.0).abs() < f64::EPSILON);
    }
}
