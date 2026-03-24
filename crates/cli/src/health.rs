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

    // Apply workspace filter
    if let Some(ws_name) = opts.workspace {
        match resolve_workspace_filter(opts.root, ws_name, &opts.output) {
            Ok(ws_root) => {
                findings.retain(|f| f.path.starts_with(&ws_root));
            }
            Err(code) => return code,
        }
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

    let report = HealthReport {
        summary: HealthSummary {
            files_analyzed,
            functions_analyzed: total_functions,
            functions_above_threshold: total_above_threshold,
            max_cyclomatic_threshold: max_cyclomatic,
            max_cognitive_threshold: max_cognitive,
        },
        findings,
    };

    let elapsed = start.elapsed();

    // Print report
    let result = report::print_health_report(&report, &config, elapsed, opts.quiet, &opts.output);
    if result != ExitCode::SUCCESS {
        return result;
    }

    // Exit code 1 if there are findings
    if !report.findings.is_empty() {
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}
