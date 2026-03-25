mod hotspots;
mod scoring;
mod targets;

use std::process::ExitCode;
use std::time::{Duration, Instant};

use fallow_config::{OutputFormat, ResolvedConfig};

use crate::baseline::{HealthBaselineData, filter_new_health_findings, filter_new_health_targets};
use crate::check::{get_changed_files, resolve_workspace_filter};
pub use crate::health_types::*;
use crate::load_config;
use crate::report;
use crate::vital_signs;

use hotspots::compute_hotspots;
use scoring::compute_file_scores;
use targets::{TargetAuxData, compute_refactoring_targets};

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
    pub save_snapshot: Option<std::path::PathBuf>,
}

/// Run health analysis and return results without printing.
pub fn execute_health(opts: &HealthOptions<'_>) -> Result<HealthResult, ExitCode> {
    let start = Instant::now();

    let config = load_config(
        opts.root,
        opts.config_path,
        opts.output.clone(),
        opts.no_cache,
        opts.threads,
        opts.production,
        opts.quiet,
    )?;

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
        Some(resolve_workspace_filter(opts.root, ws_name, &opts.output)?)
    } else {
        None
    };

    // Build FileId -> path lookup for O(1) access
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

    // Capture total above threshold before baseline filtering
    let total_above_threshold = findings.len();

    // Load baseline for filtering (save happens after targets are computed)
    let loaded_baseline = if let Some(load_path) = opts.baseline {
        match std::fs::read_to_string(load_path) {
            Ok(json) => match serde_json::from_str::<HealthBaselineData>(&json) {
                Ok(baseline) => {
                    findings = filter_new_health_findings(findings, &baseline, &config.root);
                    Some(baseline)
                }
                Err(e) => {
                    eprintln!("Error: failed to parse health baseline: {e}");
                    return Err(ExitCode::from(2));
                }
            },
            Err(e) => {
                eprintln!("Error: failed to read health baseline: {e}");
                return Err(ExitCode::from(2));
            }
        }
    } else {
        None
    };

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
    let (mut file_scores, files_scored, average_maintainability, score_aux, analysis_counts) =
        if needs_file_scores {
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
                    let counts = output.analysis_counts;
                    let aux = (
                        output.circular_files,
                        output.top_complex_fns,
                        output.entry_points,
                        output.value_export_counts,
                        output.unused_export_names,
                        output.cycle_members,
                    );
                    (
                        output.scores,
                        Some(total_scored),
                        avg,
                        Some(aux),
                        Some(counts),
                    )
                }
                Err(e) => {
                    eprintln!("Warning: failed to compute file scores: {e}");
                    // Use Some(0) so JSON consumers can distinguish "flag not set" (field absent)
                    // from "flag set but failed" (files_scored: 0).
                    (Vec::new(), Some(0), None, None, None)
                }
            }
        } else {
            (Vec::new(), None, None, None, None)
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
            ref unused_export_names,
            ref cycle_members,
        )) = score_aux
        {
            let target_aux = TargetAuxData {
                circular_files,
                top_complex_fns,
                entry_points,
                value_export_counts,
                unused_export_names,
                cycle_members,
            };
            let mut tgts = compute_refactoring_targets(&file_scores, &target_aux, &hotspots);
            // Filter targets against baseline (before --top truncation)
            if let Some(ref baseline) = loaded_baseline {
                tgts = filter_new_health_targets(tgts, baseline, &config.root);
            }
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

    // Save baseline (after targets are computed, captures full state)
    if let Some(save_path) = opts.save_baseline {
        let baseline = HealthBaselineData::from_findings(&findings, &targets, &config.root);
        match serde_json::to_string_pretty(&baseline) {
            Ok(json) => {
                if let Err(e) = std::fs::write(save_path, json) {
                    eprintln!("Error: failed to save health baseline: {e}");
                    return Err(ExitCode::from(2));
                }
                if !opts.quiet {
                    eprintln!("Saved health baseline to {}", save_path.display());
                }
            }
            Err(e) => {
                eprintln!("Error: failed to serialize health baseline: {e}");
                return Err(ExitCode::from(2));
            }
        }
    }

    // Compute vital signs from available data (always, for the report summary)
    let vs_input = vital_signs::VitalSignsInput {
        modules: &parse_result.modules,
        file_scores: if needs_file_scores {
            Some(&file_scores)
        } else {
            None
        },
        // Some(&[]) when pipeline ran but returned 0 results (→ hotspot_count: 0),
        // None when pipeline was not invoked (→ hotspot_count: null in snapshot).
        hotspots: if opts.hotspots || opts.targets {
            Some(&hotspots)
        } else {
            None
        },
        total_files: files.len(),
        analysis_counts,
    };
    let vital_signs = vital_signs::compute_vital_signs(&vs_input);

    // Save snapshot if requested
    if let Some(ref snapshot_path) = opts.save_snapshot {
        let counts = vital_signs::build_counts(&vs_input);
        let shallow = hotspot_summary.as_ref().is_some_and(|s| s.shallow_clone);
        let snapshot = vital_signs::build_snapshot(vital_signs.clone(), counts, opts.root, shallow);
        let explicit = if snapshot_path.as_os_str().is_empty() {
            None
        } else {
            Some(snapshot_path.as_path())
        };
        match vital_signs::save_snapshot(&snapshot, opts.root, explicit) {
            Ok(saved_path) => {
                if !opts.quiet {
                    eprintln!("Saved vital signs snapshot to {}", saved_path.display());
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
                return Err(ExitCode::from(2));
            }
        }
    }

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
        vital_signs: Some(vital_signs),
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

    Ok(HealthResult {
        report,
        config,
        elapsed,
    })
}

/// Run health analysis, print results, and return exit code.
pub fn run_health(opts: &HealthOptions<'_>) -> ExitCode {
    match execute_health(opts) {
        Ok(result) => print_health_result(&result, opts.quiet, opts.explain),
        Err(code) => code,
    }
}

/// Result of executing health analysis without printing.
pub struct HealthResult {
    pub report: HealthReport,
    pub config: ResolvedConfig,
    pub elapsed: Duration,
}

/// Print health results and return appropriate exit code.
pub fn print_health_result(result: &HealthResult, quiet: bool, explain: bool) -> ExitCode {
    let report_code = report::print_health_report(
        &result.report,
        &result.config,
        result.elapsed,
        quiet,
        &result.config.output,
        explain,
    );
    if report_code != ExitCode::SUCCESS {
        return report_code;
    }

    if !result.report.findings.is_empty() {
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}
