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
    pub targets: bool,
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

    // Compute file-level health scores when requested, when hotspots need them,
    // or when targets need them.
    // NOTE: This runs the full analysis pipeline (discovery, parsing, graph, dead code detection)
    // a second time because there is no API to inject pre-parsed modules into the analysis
    // pipeline. The cache mitigates re-parsing cost but the discovery and graph construction
    // are repeated. Future optimization: expose a lower-level API that accepts ParseResult.
    let needs_file_scores = opts.file_scores || opts.hotspots || opts.targets;
    let (mut file_scores, files_scored, average_maintainability, score_aux) = if needs_file_scores {
        match compute_file_scores(
            &config,
            &parse_result.modules,
            &file_paths,
            changed_files.as_ref(),
        ) {
            Ok(mut output) => {
                // Apply the same filters that findings get: workspace, ignore globs
                if let Some(ref ws) = ws_root {
                    output.scores.retain(|s| s.path.starts_with(ws));
                }
                if !ignore_set.is_empty() {
                    output.scores.retain(|s| {
                        let relative = s.path.strip_prefix(&config.root).unwrap_or(&s.path);
                        !ignore_set.is_match(relative)
                    });
                }
                // Compute average BEFORE --top truncation so it reflects the full project
                let total_scored = output.scores.len();
                let avg = if total_scored > 0 {
                    let sum: f64 = output.scores.iter().map(|s| s.maintainability_index).sum();
                    Some((sum / total_scored as f64 * 10.0).round() / 10.0)
                } else {
                    None
                };
                let aux = (
                    output.circular_files,
                    output.top_complex_fns,
                    output.entry_points,
                    output.value_export_counts,
                );
                (output.scores, Some(total_scored), avg, Some(aux))
            }
            Err(e) => {
                eprintln!("Warning: failed to compute file scores: {e}");
                // Use Some(0) so JSON consumers can distinguish "flag not set" (field absent)
                // from "flag set but failed" (files_scored: 0).
                (Vec::new(), Some(0), None, None)
            }
        }
    } else {
        (Vec::new(), None, None, None)
    };

    // Compute hotspot analysis when requested (or when targets need churn data).
    let (hotspots, hotspot_summary) = if opts.hotspots || opts.targets {
        compute_hotspots(opts, &config, &file_scores, &ignore_set, ws_root.as_deref())
    } else {
        (Vec::new(), None)
    };

    // Compute refactoring targets when requested.
    let targets = if opts.targets {
        if let Some((
            ref circular_files,
            ref top_complex_fns,
            ref entry_points,
            ref value_export_counts,
        )) = score_aux
        {
            let mut tgts = compute_refactoring_targets(
                &file_scores,
                circular_files,
                top_complex_fns,
                entry_points,
                value_export_counts,
                &hotspots,
            );
            if let Some(top) = opts.top {
                tgts.truncate(top);
            }
            tgts
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Apply --top to file scores (after hotspot and target computation which use the full list)
    if opts.file_scores {
        if let Some(top) = opts.top {
            file_scores.truncate(top);
        }
    } else {
        // If file_scores was only computed for hotspots/targets, don't include it in the report
        file_scores.clear();
    }

    // If hotspots were only computed for targets, don't include them in the report
    let (report_hotspots, report_hotspot_summary) = if opts.hotspots {
        (hotspots, hotspot_summary)
    } else {
        (Vec::new(), None)
    };

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
        hotspots: report_hotspots,
        hotspot_summary: report_hotspot_summary,
        targets,
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

/// Output from `compute_file_scores`, including auxiliary data for refactoring targets.
struct FileScoreOutput {
    scores: Vec<FileHealthScore>,
    /// Files participating in circular dependencies (absolute paths).
    circular_files: rustc_hash::FxHashSet<std::path::PathBuf>,
    /// Top 3 functions by cognitive complexity per file (name, cognitive score).
    top_complex_fns: rustc_hash::FxHashMap<std::path::PathBuf, Vec<(String, u16)>>,
    /// Files that are configured entry points.
    entry_points: rustc_hash::FxHashSet<std::path::PathBuf>,
    /// Total number of value exports per file (for dead code gate: total_value_exports ≥ 3).
    value_export_counts: rustc_hash::FxHashMap<std::path::PathBuf, usize>,
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
) -> Result<FileScoreOutput, String> {
    // Run full analysis to get the graph and dead code results
    let output = fallow_core::analyze_with_trace(config).map_err(|e| format!("{e}"))?;
    let graph = output.graph.ok_or("graph not available")?;
    let results = &output.results;

    // Build auxiliary data for refactoring targets
    let circular_files: rustc_hash::FxHashSet<std::path::PathBuf> = results
        .circular_dependencies
        .iter()
        .flat_map(|c| c.files.iter().cloned())
        .collect();

    let mut top_complex_fns: rustc_hash::FxHashMap<std::path::PathBuf, Vec<(String, u16)>> =
        rustc_hash::FxHashMap::default();
    for module in modules {
        if module.complexity.is_empty() {
            continue;
        }
        let Some(path) = file_paths.get(&module.file_id) else {
            continue;
        };
        let mut funcs: Vec<(String, u16)> = module
            .complexity
            .iter()
            .map(|f| (f.name.clone(), f.cognitive))
            .collect();
        funcs.sort_by(|a, b| b.1.cmp(&a.1));
        funcs.truncate(3);
        if funcs[0].1 > 0 {
            top_complex_fns.insert((*path).clone(), funcs);
        }
    }

    let mut entry_points: rustc_hash::FxHashSet<std::path::PathBuf> =
        rustc_hash::FxHashSet::default();
    let mut value_export_counts: rustc_hash::FxHashMap<std::path::PathBuf, usize> =
        rustc_hash::FxHashMap::default();

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

        // Track entry points for refactoring target exclusion
        if node.is_entry_point {
            entry_points.insert((*path).clone());
        }

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

        // Track value export count for dead code gate
        let value_exports = node.exports.iter().filter(|e| !e.is_type_only).count();
        value_export_counts.insert((*path).clone(), value_exports);

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

    Ok(FileScoreOutput {
        scores,
        circular_files,
        top_complex_fns,
        entry_points,
        value_export_counts,
    })
}

/// Compute the refactoring priority score for a file.
///
/// Formula (avoids double-counting with MI):
/// ```text
/// priority = min(density, 1) × 30 + hotspot_boost × 25 + dead_code × 20 + fan_in_norm × 15 + fan_out_norm × 10
/// ```
/// All inputs are clamped to \[0, 1\] so each weight is a true percentage share.
fn compute_target_priority(score: &FileHealthScore, hotspot_score: Option<f64>) -> f64 {
    // Normalize all inputs to [0, 1] so each weight is a true percentage share.
    let density_norm = score.complexity_density.min(1.0);
    let fan_in_norm = (score.fan_in as f64 / 20.0).min(1.0);
    let fan_out_norm = (score.fan_out as f64 / 30.0).min(1.0);
    let hotspot_boost = hotspot_score.map_or(0.0, |s| s / 100.0);

    // Keep the formula readable — it matches the documented specification.
    #[expect(clippy::suboptimal_flops)]
    let priority = density_norm * 30.0
        + hotspot_boost * 25.0
        + score.dead_code_ratio * 20.0
        + fan_in_norm * 15.0
        + fan_out_norm * 10.0;

    (priority.clamp(0.0, 100.0) * 10.0).round() / 10.0
}

/// Compute refactoring targets by applying rules to file scores and auxiliary data.
///
/// Rules are evaluated in priority order; first match determines the category and
/// recommendation. All contributing factors are collected regardless of which rule wins.
/// Files matching no rule are skipped.
fn compute_refactoring_targets(
    file_scores: &[FileHealthScore],
    circular_files: &rustc_hash::FxHashSet<std::path::PathBuf>,
    top_complex_fns: &rustc_hash::FxHashMap<std::path::PathBuf, Vec<(String, u16)>>,
    entry_points: &rustc_hash::FxHashSet<std::path::PathBuf>,
    value_export_counts: &rustc_hash::FxHashMap<std::path::PathBuf, usize>,
    hotspots: &[HotspotEntry],
) -> Vec<RefactoringTarget> {
    // Build hotspot lookup by path for O(1) access
    let hotspot_map: rustc_hash::FxHashMap<&std::path::Path, &HotspotEntry> =
        hotspots.iter().map(|h| (h.path.as_path(), h)).collect();

    let mut targets = Vec::new();

    for score in file_scores {
        let hotspot = hotspot_map.get(score.path.as_path());
        let hotspot_score = hotspot.map(|h| h.score);
        let is_circular = circular_files.contains(&score.path);
        let is_entry = entry_points.contains(&score.path);
        let top_fns = top_complex_fns.get(&score.path);
        let value_exports = value_export_counts.get(&score.path).copied().unwrap_or(0);

        // Collect all contributing factors
        let mut factors = Vec::new();

        if score.complexity_density > 0.3 {
            factors.push(ContributingFactor {
                metric: "complexity_density",
                value: score.complexity_density,
                threshold: 0.3,
                detail: format!("density {:.2} exceeds 0.3", score.complexity_density),
            });
        }
        if score.fan_in >= 10 {
            factors.push(ContributingFactor {
                metric: "fan_in",
                value: score.fan_in as f64,
                threshold: 10.0,
                detail: format!("{} files depend on this", score.fan_in),
            });
        }
        if score.dead_code_ratio >= 0.5 && value_exports >= 3 {
            let unused_count = (score.dead_code_ratio * value_exports as f64)
                .round()
                .min(value_exports as f64) as usize;
            factors.push(ContributingFactor {
                metric: "dead_code_ratio",
                value: score.dead_code_ratio,
                threshold: 0.5,
                detail: format!(
                    "{} unused of {} value exports ({:.0}%)",
                    unused_count,
                    value_exports,
                    score.dead_code_ratio * 100.0
                ),
            });
        }
        if score.fan_out >= 15 {
            factors.push(ContributingFactor {
                metric: "fan_out",
                value: score.fan_out as f64,
                threshold: 15.0,
                detail: format!("imports {} modules", score.fan_out),
            });
        }
        if is_circular {
            factors.push(ContributingFactor {
                metric: "circular_dependency",
                value: 1.0,
                threshold: 1.0,
                detail: "participates in an import cycle".into(),
            });
        }
        if let Some(h) = hotspot
            && h.score >= 30.0
        {
            factors.push(ContributingFactor {
                metric: "hotspot_score",
                value: h.score,
                threshold: 30.0,
                detail: format!(
                    "hotspot score {:.0} ({} commits, {} trend)",
                    h.score,
                    h.commits,
                    match h.trend {
                        fallow_core::churn::ChurnTrend::Accelerating => "accelerating",
                        fallow_core::churn::ChurnTrend::Cooling => "cooling",
                        fallow_core::churn::ChurnTrend::Stable => "stable",
                    }
                ),
            });
        }
        if let Some(fns) = top_fns
            && let Some((name, cog)) = fns.first()
            && *cog >= 30
        {
            factors.push(ContributingFactor {
                metric: "cognitive_complexity",
                value: f64::from(*cog),
                threshold: 30.0,
                detail: format!("{name} has cognitive complexity {cog}"),
            });
        }

        // Skip if no factors triggered
        if factors.is_empty() {
            continue;
        }

        // Evaluate rules in priority order — first match determines category + recommendation
        let matched = try_match_rules(
            score,
            hotspot.copied(),
            is_circular,
            is_entry,
            top_fns,
            value_exports,
        );

        let Some((category, recommendation)) = matched else {
            continue;
        };

        let priority = compute_target_priority(score, hotspot_score);

        targets.push(RefactoringTarget {
            path: score.path.clone(),
            priority,
            recommendation,
            category,
            factors,
        });
    }

    // Sort by priority descending, break ties by path for determinism
    targets.sort_by(|a, b| {
        b.priority
            .partial_cmp(&a.priority)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.path.cmp(&b.path))
    });

    targets
}

/// Try to match a file against refactoring rules in priority order.
///
/// Returns the first matching `(category, recommendation)`, or `None` if no rule matches.
fn try_match_rules(
    score: &FileHealthScore,
    hotspot: Option<&HotspotEntry>,
    is_circular: bool,
    is_entry: bool,
    top_fns: Option<&Vec<(String, u16)>>,
    value_exports: usize,
) -> Option<(RecommendationCategory, String)> {
    // Rule 1: Urgent churn + complexity
    if let Some(h) = hotspot
        && h.score >= 50.0
        && matches!(h.trend, fallow_core::churn::ChurnTrend::Accelerating)
        && score.complexity_density > 0.5
    {
        return Some((
            RecommendationCategory::UrgentChurnComplexity,
            "Actively-changing file with growing complexity \u{2014} stabilize before adding features".into(),
        ));
    }

    // Rule 2: Circular dependency with high fan-in
    if is_circular && score.fan_in >= 5 {
        return Some((
            RecommendationCategory::BreakCircularDependency,
            format!(
                "Break import cycle \u{2014} {} files depend on this, changes cascade through the cycle",
                score.fan_in
            ),
        ));
    }

    // Rule 3: Split high-impact file
    if score.complexity_density > 0.3
        && (score.fan_in >= 20 || (score.fan_in >= 10 && score.function_count >= 5))
    {
        return Some((
            RecommendationCategory::SplitHighImpact,
            format!(
                "Split high-impact file \u{2014} {} dependents amplify every change",
                score.fan_in
            ),
        ));
    }

    // Rule 4: Remove dead code (gate: ≥3 value exports)
    if score.dead_code_ratio >= 0.5 && value_exports >= 3 {
        let unused_count = (score.dead_code_ratio * value_exports as f64).round() as usize;
        return Some((
            RecommendationCategory::RemoveDeadCode,
            format!(
                "Remove {} unused exports to reduce surface area ({:.0}% dead)",
                unused_count,
                score.dead_code_ratio * 100.0
            ),
        ));
    }

    // Rule 5: Extract complex functions (cognitive ≥ 30)
    if let Some(fns) = top_fns {
        let high: Vec<&(String, u16)> = fns.iter().filter(|(_, cog)| *cog >= 30).collect();
        if !high.is_empty() {
            let desc = match high.len() {
                1 => format!(
                    "Extract {} (cognitive: {}) into smaller functions",
                    high[0].0, high[0].1
                ),
                _ => format!(
                    "Extract {} (cognitive: {}) and {} (cognitive: {}) into smaller functions",
                    high[0].0, high[0].1, high[1].0, high[1].1
                ),
            };
            return Some((RecommendationCategory::ExtractComplexFunctions, desc));
        }
    }

    // Rule 6: Extract dependencies (not for entry points)
    if !is_entry && score.fan_out >= 15 && score.maintainability_index < 60.0 {
        return Some((
            RecommendationCategory::ExtractDependencies,
            format!(
                "Reduce coupling \u{2014} this file imports {} modules, limiting testability",
                score.fan_out
            ),
        ));
    }

    // Rule 7: Circular dependency (low fan-in fallback)
    if is_circular {
        return Some((
            RecommendationCategory::BreakCircularDependency,
            "Break import cycle to reduce change cascade risk".into(),
        ));
    }

    None
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

    // --- compute_hotspot_score ---

    #[test]
    fn hotspot_score_both_maxima_zero() {
        // When both maxima are zero, avoid division by zero → score 0
        assert!((compute_hotspot_score(0.0, 0.0, 0.0, 0.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn hotspot_score_max_weighted_zero() {
        // Churn dimension zero, complexity present → score 0
        assert!((compute_hotspot_score(5.0, 0.0, 0.5, 1.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn hotspot_score_max_density_zero() {
        // Complexity dimension zero, churn present → score 0
        assert!((compute_hotspot_score(5.0, 10.0, 0.0, 0.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn hotspot_score_equal_normalization() {
        // File equals both maxima → normalized values both 1.0 → score 100
        let score = compute_hotspot_score(10.0, 10.0, 2.0, 2.0);
        assert!((score - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn hotspot_score_half_values() {
        // Half of each maximum → 0.5 * 0.5 * 100 = 25.0
        let score = compute_hotspot_score(5.0, 10.0, 1.0, 2.0);
        assert!((score - 25.0).abs() < f64::EPSILON);
    }

    // --- compute_complexity_density ---

    #[test]
    fn complexity_density_zero_lines() {
        assert!((compute_complexity_density(10, 0)).abs() < f64::EPSILON);
    }

    #[test]
    fn complexity_density_normal() {
        // 10 cyclomatic / 100 lines = 0.1
        let result = compute_complexity_density(10, 100);
        assert!((result - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn complexity_density_high() {
        // 50 cyclomatic / 10 lines = 5.0
        let result = compute_complexity_density(50, 10);
        assert!((result - 5.0).abs() < f64::EPSILON);
    }

    // --- compute_dead_code_ratio ---

    #[test]
    fn dead_code_ratio_no_exports() {
        let unused_files = rustc_hash::FxHashSet::default();
        let unused_map = rustc_hash::FxHashMap::default();
        let path = std::path::Path::new("/src/foo.ts");
        let exports: Vec<fallow_core::graph::ExportSymbol> = vec![];

        let ratio = compute_dead_code_ratio(path, &exports, &unused_files, &unused_map);
        assert!((ratio).abs() < f64::EPSILON);
    }

    #[test]
    fn dead_code_ratio_all_unused_file() {
        let mut unused_files: rustc_hash::FxHashSet<&std::path::Path> =
            rustc_hash::FxHashSet::default();
        let path = std::path::Path::new("/src/foo.ts");
        unused_files.insert(path);
        let unused_map = rustc_hash::FxHashMap::default();
        let exports: Vec<fallow_core::graph::ExportSymbol> = vec![];

        let ratio = compute_dead_code_ratio(path, &exports, &unused_files, &unused_map);
        assert!((ratio - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn dead_code_ratio_mix() {
        let unused_files = rustc_hash::FxHashSet::default();
        let path = std::path::Path::new("/src/foo.ts");

        // 3 value exports, 1 type-only export
        let exports = vec![
            fallow_core::graph::ExportSymbol {
                name: fallow_core::extract::ExportName::Named("a".into()),
                is_type_only: false,
                is_public: false,
                span: oxc_span::Span::empty(0),
                references: vec![],
                members: vec![],
            },
            fallow_core::graph::ExportSymbol {
                name: fallow_core::extract::ExportName::Named("b".into()),
                is_type_only: false,
                is_public: false,
                span: oxc_span::Span::empty(0),
                references: vec![],
                members: vec![],
            },
            fallow_core::graph::ExportSymbol {
                name: fallow_core::extract::ExportName::Named("c".into()),
                is_type_only: false,
                is_public: false,
                span: oxc_span::Span::empty(0),
                references: vec![],
                members: vec![],
            },
            fallow_core::graph::ExportSymbol {
                name: fallow_core::extract::ExportName::Named("MyType".into()),
                is_type_only: true,
                is_public: false,
                span: oxc_span::Span::empty(0),
                references: vec![],
                members: vec![],
            },
        ];

        // 2 of 3 value exports are unused
        let mut unused_map: rustc_hash::FxHashMap<&std::path::Path, usize> =
            rustc_hash::FxHashMap::default();
        unused_map.insert(path, 2);

        let ratio = compute_dead_code_ratio(path, &exports, &unused_files, &unused_map);
        // 2/3 ≈ 0.6667
        assert!((ratio - 2.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn dead_code_ratio_all_type_only_exports() {
        let unused_files = rustc_hash::FxHashSet::default();
        let path = std::path::Path::new("/src/types.ts");

        // Only type exports → value_exports = 0 → ratio 0.0
        let exports = vec![fallow_core::graph::ExportSymbol {
            name: fallow_core::extract::ExportName::Named("Foo".into()),
            is_type_only: true,
            is_public: false,
            span: oxc_span::Span::empty(0),
            references: vec![],
            members: vec![],
        }];
        let unused_map = rustc_hash::FxHashMap::default();

        let ratio = compute_dead_code_ratio(path, &exports, &unused_files, &unused_map);
        assert!((ratio).abs() < f64::EPSILON);
    }

    // --- aggregate_complexity ---

    #[test]
    fn aggregate_complexity_empty_module() {
        let module = fallow_core::extract::ModuleInfo {
            file_id: fallow_core::discover::FileId(0),
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            dynamic_import_patterns: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            content_hash: 0,
            suppressions: vec![],
            unused_import_bindings: vec![],
            line_offsets: vec![],
            complexity: vec![],
        };

        let (cyc, cog, funcs, lines) = aggregate_complexity(&module);
        assert_eq!(cyc, 0);
        assert_eq!(cog, 0);
        assert_eq!(funcs, 0);
        assert_eq!(lines, 0);
    }

    #[test]
    fn aggregate_complexity_single_function() {
        let module = fallow_core::extract::ModuleInfo {
            file_id: fallow_core::discover::FileId(0),
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            dynamic_import_patterns: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            content_hash: 0,
            suppressions: vec![],
            unused_import_bindings: vec![],
            line_offsets: vec![0, 10, 20, 30, 40], // 5 lines
            complexity: vec![fallow_types::extract::FunctionComplexity {
                name: "doStuff".into(),
                line: 1,
                col: 0,
                cyclomatic: 7,
                cognitive: 4,
                line_count: 5,
            }],
        };

        let (cyc, cog, funcs, lines) = aggregate_complexity(&module);
        assert_eq!(cyc, 7);
        assert_eq!(cog, 4);
        assert_eq!(funcs, 1);
        assert_eq!(lines, 5);
    }

    #[test]
    fn aggregate_complexity_multiple_functions() {
        let module = fallow_core::extract::ModuleInfo {
            file_id: fallow_core::discover::FileId(0),
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            dynamic_import_patterns: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            content_hash: 0,
            suppressions: vec![],
            unused_import_bindings: vec![],
            line_offsets: vec![0, 10, 20], // 3 lines
            complexity: vec![
                fallow_types::extract::FunctionComplexity {
                    name: "a".into(),
                    line: 1,
                    col: 0,
                    cyclomatic: 3,
                    cognitive: 2,
                    line_count: 1,
                },
                fallow_types::extract::FunctionComplexity {
                    name: "b".into(),
                    line: 2,
                    col: 0,
                    cyclomatic: 5,
                    cognitive: 8,
                    line_count: 2,
                },
            ],
        };

        let (cyc, cog, funcs, lines) = aggregate_complexity(&module);
        assert_eq!(cyc, 8);
        assert_eq!(cog, 10);
        assert_eq!(funcs, 2);
        assert_eq!(lines, 3);
    }

    // --- is_excluded_from_hotspots ---

    #[test]
    fn excluded_no_filters() {
        let path = std::path::Path::new("/project/src/foo.ts");
        let root = std::path::Path::new("/project");
        let ignore_set = globset::GlobSet::empty();

        assert!(!is_excluded_from_hotspots(path, root, &ignore_set, None));
    }

    #[test]
    fn excluded_workspace_filter_mismatch() {
        let path = std::path::Path::new("/project/packages/b/src/foo.ts");
        let root = std::path::Path::new("/project");
        let ws_root = std::path::Path::new("/project/packages/a");
        let ignore_set = globset::GlobSet::empty();

        assert!(is_excluded_from_hotspots(
            path,
            root,
            &ignore_set,
            Some(ws_root)
        ));
    }

    #[test]
    fn excluded_workspace_filter_match() {
        let path = std::path::Path::new("/project/packages/a/src/foo.ts");
        let root = std::path::Path::new("/project");
        let ws_root = std::path::Path::new("/project/packages/a");
        let ignore_set = globset::GlobSet::empty();

        assert!(!is_excluded_from_hotspots(
            path,
            root,
            &ignore_set,
            Some(ws_root)
        ));
    }

    #[test]
    fn excluded_matching_glob() {
        let path = std::path::Path::new("/project/src/generated/types.ts");
        let root = std::path::Path::new("/project");
        let mut builder = globset::GlobSetBuilder::new();
        builder.add(globset::Glob::new("src/generated/**").unwrap());
        let ignore_set = builder.build().unwrap();

        assert!(is_excluded_from_hotspots(path, root, &ignore_set, None));
    }

    #[test]
    fn excluded_non_matching_glob() {
        let path = std::path::Path::new("/project/src/components/Button.tsx");
        let root = std::path::Path::new("/project");
        let mut builder = globset::GlobSetBuilder::new();
        builder.add(globset::Glob::new("src/generated/**").unwrap());
        let ignore_set = builder.build().unwrap();

        assert!(!is_excluded_from_hotspots(path, root, &ignore_set, None));
    }

    // --- compute_normalization_maxima ---

    #[test]
    fn normalization_maxima_empty_input() {
        let scores: Vec<FileHealthScore> = vec![];
        let churn_files = rustc_hash::FxHashMap::default();

        let (max_w, max_d) = compute_normalization_maxima(&scores, &churn_files, 3);
        assert!((max_w).abs() < f64::EPSILON);
        assert!((max_d).abs() < f64::EPSILON);
    }

    #[test]
    fn normalization_maxima_single_file() {
        let scores = vec![FileHealthScore {
            path: std::path::PathBuf::from("/src/foo.ts"),
            fan_in: 0,
            fan_out: 0,
            dead_code_ratio: 0.0,
            complexity_density: 0.75,
            maintainability_index: 80.0,
            total_cyclomatic: 15,
            total_cognitive: 10,
            function_count: 3,
            lines: 20,
        }];
        let mut churn_files: rustc_hash::FxHashMap<
            std::path::PathBuf,
            fallow_core::churn::FileChurn,
        > = rustc_hash::FxHashMap::default();
        churn_files.insert(
            std::path::PathBuf::from("/src/foo.ts"),
            fallow_core::churn::FileChurn {
                path: std::path::PathBuf::from("/src/foo.ts"),
                commits: 5,
                weighted_commits: 4.2,
                lines_added: 100,
                lines_deleted: 20,
                trend: fallow_core::churn::ChurnTrend::Stable,
            },
        );

        let (max_w, max_d) = compute_normalization_maxima(&scores, &churn_files, 3);
        assert!((max_w - 4.2).abs() < f64::EPSILON);
        assert!((max_d - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn normalization_maxima_below_min_commits() {
        let scores = vec![FileHealthScore {
            path: std::path::PathBuf::from("/src/foo.ts"),
            fan_in: 0,
            fan_out: 0,
            dead_code_ratio: 0.0,
            complexity_density: 0.75,
            maintainability_index: 80.0,
            total_cyclomatic: 15,
            total_cognitive: 10,
            function_count: 3,
            lines: 20,
        }];
        let mut churn_files: rustc_hash::FxHashMap<
            std::path::PathBuf,
            fallow_core::churn::FileChurn,
        > = rustc_hash::FxHashMap::default();
        churn_files.insert(
            std::path::PathBuf::from("/src/foo.ts"),
            fallow_core::churn::FileChurn {
                path: std::path::PathBuf::from("/src/foo.ts"),
                commits: 2, // below min_commits=3
                weighted_commits: 4.2,
                lines_added: 100,
                lines_deleted: 20,
                trend: fallow_core::churn::ChurnTrend::Stable,
            },
        );

        // File has only 2 commits, below min_commits=3 → excluded
        let (max_w, max_d) = compute_normalization_maxima(&scores, &churn_files, 3);
        assert!((max_w).abs() < f64::EPSILON);
        assert!((max_d).abs() < f64::EPSILON);
    }

    #[test]
    fn normalization_maxima_all_zeros() {
        let scores = vec![FileHealthScore {
            path: std::path::PathBuf::from("/src/foo.ts"),
            fan_in: 0,
            fan_out: 0,
            dead_code_ratio: 0.0,
            complexity_density: 0.0,
            maintainability_index: 100.0,
            total_cyclomatic: 0,
            total_cognitive: 0,
            function_count: 1,
            lines: 10,
        }];
        let mut churn_files: rustc_hash::FxHashMap<
            std::path::PathBuf,
            fallow_core::churn::FileChurn,
        > = rustc_hash::FxHashMap::default();
        churn_files.insert(
            std::path::PathBuf::from("/src/foo.ts"),
            fallow_core::churn::FileChurn {
                path: std::path::PathBuf::from("/src/foo.ts"),
                commits: 5,
                weighted_commits: 0.0,
                lines_added: 0,
                lines_deleted: 0,
                trend: fallow_core::churn::ChurnTrend::Stable,
            },
        );

        let (max_w, max_d) = compute_normalization_maxima(&scores, &churn_files, 3);
        assert!((max_w).abs() < f64::EPSILON);
        assert!((max_d).abs() < f64::EPSILON);
    }

    // --- count_unused_exports_by_path ---

    #[test]
    fn count_unused_exports_empty() {
        let exports: Vec<fallow_core::results::UnusedExport> = vec![];
        let map = count_unused_exports_by_path(&exports);
        assert!(map.is_empty());
    }

    #[test]
    fn count_unused_exports_groups_by_path() {
        let exports = vec![
            fallow_core::results::UnusedExport {
                path: std::path::PathBuf::from("/src/a.ts"),
                export_name: "foo".into(),
                is_type_only: false,
                line: 1,
                col: 0,
                span_start: 0,
                is_re_export: false,
            },
            fallow_core::results::UnusedExport {
                path: std::path::PathBuf::from("/src/a.ts"),
                export_name: "bar".into(),
                is_type_only: false,
                line: 5,
                col: 0,
                span_start: 40,
                is_re_export: false,
            },
            fallow_core::results::UnusedExport {
                path: std::path::PathBuf::from("/src/b.ts"),
                export_name: "baz".into(),
                is_type_only: false,
                line: 1,
                col: 0,
                span_start: 0,
                is_re_export: false,
            },
        ];
        let map = count_unused_exports_by_path(&exports);
        assert_eq!(map.get(std::path::Path::new("/src/a.ts")).copied(), Some(2));
        assert_eq!(map.get(std::path::Path::new("/src/b.ts")).copied(), Some(1));
    }

    // --- compute_target_priority ---

    fn make_score(overrides: impl FnOnce(&mut FileHealthScore)) -> FileHealthScore {
        let mut s = FileHealthScore {
            path: std::path::PathBuf::from("/src/foo.ts"),
            fan_in: 0,
            fan_out: 0,
            dead_code_ratio: 0.0,
            complexity_density: 0.0,
            maintainability_index: 100.0,
            total_cyclomatic: 0,
            total_cognitive: 0,
            function_count: 1,
            lines: 100,
        };
        overrides(&mut s);
        s
    }

    #[test]
    fn target_priority_all_zero() {
        let score = make_score(|_| {});
        let priority = compute_target_priority(&score, None);
        assert!((priority).abs() < f64::EPSILON);
    }

    #[test]
    fn target_priority_max_all_inputs() {
        let score = make_score(|s| {
            s.complexity_density = 2.0; // clamped to 1.0
            s.fan_in = 40; // clamped to 1.0
            s.fan_out = 60; // clamped to 1.0
            s.dead_code_ratio = 1.0;
        });
        let priority = compute_target_priority(&score, Some(100.0));
        assert!((priority - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn target_priority_complexity_density_weight() {
        // density=1.0, all else zero → 30 points
        let score = make_score(|s| s.complexity_density = 1.0);
        let priority = compute_target_priority(&score, None);
        assert!((priority - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn target_priority_hotspot_weight() {
        // hotspot_score=100 → boost=1.0 → 25 points
        let score = make_score(|_| {});
        let priority = compute_target_priority(&score, Some(100.0));
        assert!((priority - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn target_priority_dead_code_weight() {
        // dead_code_ratio=1.0 → 20 points
        let score = make_score(|s| s.dead_code_ratio = 1.0);
        let priority = compute_target_priority(&score, None);
        assert!((priority - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn target_priority_fan_in_weight() {
        // fan_in=20 → norm=1.0 → 15 points
        let score = make_score(|s| s.fan_in = 20);
        let priority = compute_target_priority(&score, None);
        assert!((priority - 15.0).abs() < f64::EPSILON);
    }

    #[test]
    fn target_priority_fan_out_weight() {
        // fan_out=30 → norm=1.0 → 10 points
        let score = make_score(|s| s.fan_out = 30);
        let priority = compute_target_priority(&score, None);
        assert!((priority - 10.0).abs() < f64::EPSILON);
    }

    // --- try_match_rules ---

    #[test]
    fn rule_no_match_clean_file() {
        let score = make_score(|_| {});
        let result = try_match_rules(&score, None, false, false, None, 0);
        assert!(result.is_none());
    }

    #[test]
    fn rule_circular_dep_high_fan_in() {
        let score = make_score(|s| s.fan_in = 5);
        let result = try_match_rules(&score, None, true, false, None, 0);
        assert!(result.is_some());
        let (cat, _) = result.unwrap();
        assert!(matches!(
            cat,
            RecommendationCategory::BreakCircularDependency
        ));
    }

    #[test]
    fn rule_circular_dep_low_fan_in_fallback() {
        let score = make_score(|s| s.fan_in = 1);
        let result = try_match_rules(&score, None, true, false, None, 0);
        assert!(result.is_some());
        let (cat, _) = result.unwrap();
        assert!(matches!(
            cat,
            RecommendationCategory::BreakCircularDependency
        ));
    }

    #[test]
    fn rule_split_high_impact() {
        let score = make_score(|s| {
            s.complexity_density = 0.5;
            s.fan_in = 20;
        });
        let result = try_match_rules(&score, None, false, false, None, 0);
        assert!(result.is_some());
        let (cat, _) = result.unwrap();
        assert!(matches!(cat, RecommendationCategory::SplitHighImpact));
    }

    #[test]
    fn rule_remove_dead_code() {
        let score = make_score(|s| s.dead_code_ratio = 0.6);
        let result = try_match_rules(&score, None, false, false, None, 5);
        assert!(result.is_some());
        let (cat, _) = result.unwrap();
        assert!(matches!(cat, RecommendationCategory::RemoveDeadCode));
    }

    #[test]
    fn rule_dead_code_gate_too_few_exports() {
        // dead_code_ratio high but only 2 value exports — below gate of 3
        let score = make_score(|s| s.dead_code_ratio = 0.8);
        let result = try_match_rules(&score, None, false, false, None, 2);
        // Should NOT match dead code rule
        assert!(result.is_none());
    }

    #[test]
    fn rule_extract_complex_functions() {
        let score = make_score(|_| {});
        let fns = vec![("handleSubmit".to_string(), 35u16)];
        let result = try_match_rules(&score, None, false, false, Some(&fns), 0);
        assert!(result.is_some());
        let (cat, rec) = result.unwrap();
        assert!(matches!(
            cat,
            RecommendationCategory::ExtractComplexFunctions
        ));
        assert!(rec.contains("handleSubmit"));
    }

    #[test]
    fn rule_extract_dependencies_not_entry() {
        let score = make_score(|s| {
            s.fan_out = 20;
            s.maintainability_index = 50.0;
        });
        let result = try_match_rules(&score, None, false, false, None, 0);
        assert!(result.is_some());
        let (cat, _) = result.unwrap();
        assert!(matches!(cat, RecommendationCategory::ExtractDependencies));
    }

    #[test]
    fn rule_extract_dependencies_skipped_for_entry() {
        let score = make_score(|s| {
            s.fan_out = 20;
            s.maintainability_index = 50.0;
        });
        // is_entry=true → rule 6 should not match
        let result = try_match_rules(&score, None, false, true, None, 0);
        assert!(result.is_none());
    }

    #[test]
    fn rule_urgent_churn_complexity() {
        let score = make_score(|s| s.complexity_density = 0.8);
        let hotspot = HotspotEntry {
            path: std::path::PathBuf::from("/src/foo.ts"),
            score: 60.0,
            commits: 20,
            weighted_commits: 15.0,
            lines_added: 500,
            lines_deleted: 100,
            complexity_density: 0.8,
            fan_in: 5,
            trend: fallow_core::churn::ChurnTrend::Accelerating,
        };
        let result = try_match_rules(&score, Some(&hotspot), false, false, None, 0);
        assert!(result.is_some());
        let (cat, _) = result.unwrap();
        assert!(matches!(cat, RecommendationCategory::UrgentChurnComplexity));
    }
}
