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

    let ignore_set = build_ignore_set(&config.health.ignore);

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

    let (mut findings, files_analyzed, total_functions) = collect_findings(
        &parse_result.modules,
        &file_paths,
        &config.root,
        &ignore_set,
        changed_files.as_ref(),
        max_cyclomatic,
        max_cognitive,
    );

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
    // Uses analyze_with_parse_result to reuse the already-parsed modules from above,
    // avoiding re-parsing. Discovery, resolution, and graph construction still run.
    let needs_file_scores = opts.file_scores || opts.hotspots || opts.targets;
    let (score_output, files_scored, average_maintainability) = if needs_file_scores {
        let analysis_output =
            fallow_core::analyze_with_parse_result(&config, &parse_result.modules).map_err(
                |e| {
                    eprintln!("Error: analysis failed: {e}");
                    ExitCode::from(2)
                },
            )?;
        match compute_file_scores(
            &parse_result.modules,
            &file_paths,
            changed_files.as_ref(),
            analysis_output,
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
                (Some(output), Some(total_scored), avg)
            }
            Err(e) => {
                eprintln!("Warning: failed to compute file scores: {e}");
                (None, Some(0), None)
            }
        }
    } else {
        (None, None, None)
    };

    let file_scores_slice = score_output
        .as_ref()
        .map_or(&[] as &[_], |o| o.scores.as_slice());

    // Compute hotspot analysis when requested (or when targets need churn data).
    let (hotspots, hotspot_summary) = if opts.hotspots || opts.targets {
        compute_hotspots(
            opts,
            &config,
            file_scores_slice,
            &ignore_set,
            ws_root.as_deref(),
        )
    } else {
        (Vec::new(), None)
    };

    // Compute refactoring targets when requested.
    let (targets, target_thresholds) = if opts.targets {
        if let Some(ref output) = score_output {
            let target_aux = TargetAuxData::from(output);
            let (mut tgts, thresholds) =
                compute_refactoring_targets(file_scores_slice, &target_aux, &hotspots);
            if let Some(ref baseline) = loaded_baseline {
                tgts = filter_new_health_targets(tgts, baseline, &config.root);
            }
            if let Some(top) = opts.top {
                tgts.truncate(top);
            }
            (tgts, Some(thresholds))
        } else {
            (Vec::new(), None)
        }
    } else {
        (Vec::new(), None)
    };

    if let Some(save_path) = opts.save_baseline {
        save_health_baseline(save_path, &findings, &targets, &config.root, opts.quiet)?;
    }

    // Compute vital signs from available data (always, for the report summary)
    let analysis_counts = score_output
        .as_ref()
        .map(|o| crate::vital_signs::AnalysisCounts {
            total_exports: o.analysis_counts.total_exports,
            dead_files: o.analysis_counts.dead_files,
            dead_exports: o.analysis_counts.dead_exports,
            unused_deps: o.analysis_counts.unused_deps,
            circular_deps: o.analysis_counts.circular_deps,
            total_deps: o.analysis_counts.total_deps,
        });
    let vs_input = vital_signs::VitalSignsInput {
        modules: &parse_result.modules,
        file_scores: if needs_file_scores {
            Some(file_scores_slice)
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

    // Extract file scores for the report (apply --top after hotspot/target computation)
    let file_scores = if opts.file_scores {
        let mut scores = score_output.map(|o| o.scores).unwrap_or_default();
        if let Some(top) = opts.top {
            scores.truncate(top);
        }
        scores
    } else {
        Vec::new()
    };

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
        target_thresholds,
    };

    let elapsed = start.elapsed();

    Ok(HealthResult {
        report,
        config,
        elapsed,
    })
}

/// Build a glob set from health ignore patterns.
fn build_ignore_set(patterns: &[String]) -> globset::GlobSet {
    let mut builder = globset::GlobSetBuilder::new();
    for pattern in patterns {
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
}

/// Collect health findings from parsed modules, applying ignore and changed-since filters.
fn collect_findings(
    modules: &[fallow_core::extract::ModuleInfo],
    file_paths: &rustc_hash::FxHashMap<fallow_core::discover::FileId, &std::path::PathBuf>,
    config_root: &std::path::Path,
    ignore_set: &globset::GlobSet,
    changed_files: Option<&rustc_hash::FxHashSet<std::path::PathBuf>>,
    max_cyclomatic: u16,
    max_cognitive: u16,
) -> (Vec<HealthFinding>, usize, usize) {
    let mut files_analyzed = 0usize;
    let mut total_functions = 0usize;
    let mut findings: Vec<HealthFinding> = Vec::new();

    for module in modules {
        let Some(&path) = file_paths.get(&module.file_id) else {
            continue;
        };

        let relative = path.strip_prefix(config_root).unwrap_or(path);
        if ignore_set.is_match(relative) {
            continue;
        }

        if let Some(changed) = changed_files
            && !changed.contains(path)
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
                    path: path.clone(),
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

    (findings, files_analyzed, total_functions)
}

/// Save health baseline to disk.
fn save_health_baseline(
    save_path: &std::path::Path,
    findings: &[HealthFinding],
    targets: &[RefactoringTarget],
    config_root: &std::path::Path,
    quiet: bool,
) -> Result<(), ExitCode> {
    let baseline = HealthBaselineData::from_findings(findings, targets, config_root);
    match serde_json::to_string_pretty(&baseline) {
        Ok(json) => {
            if let Err(e) = std::fs::write(save_path, json) {
                eprintln!("Error: failed to save health baseline: {e}");
                return Err(ExitCode::from(2));
            }
            if !quiet {
                eprintln!("Saved health baseline to {}", save_path.display());
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("Error: failed to serialize health baseline: {e}");
            Err(ExitCode::from(2))
        }
    }
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
    let ctx = report::ReportContext {
        root: &result.config.root,
        rules: &result.config.rules,
        elapsed: result.elapsed,
        quiet,
        explain,
    };
    let report_code = report::print_health_report(&result.report, &ctx, &result.config.output);
    if report_code != ExitCode::SUCCESS {
        return report_code;
    }

    if !result.report.findings.is_empty() {
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use fallow_core::extract::ModuleInfo;
    use fallow_types::discover::FileId;
    use fallow_types::extract::FunctionComplexity;
    use rustc_hash::{FxHashMap, FxHashSet};
    use std::path::{Path, PathBuf};

    /// Build a minimal `ModuleInfo` with only the fields `collect_findings` needs.
    fn make_module(file_id: FileId, complexity: Vec<FunctionComplexity>) -> ModuleInfo {
        ModuleInfo {
            file_id,
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
            line_offsets: vec![0],
            complexity,
        }
    }

    fn make_fc(name: &str, cyclomatic: u16, cognitive: u16, line_count: u32) -> FunctionComplexity {
        FunctionComplexity {
            name: name.to_string(),
            line: 1,
            col: 0,
            cyclomatic,
            cognitive,
            line_count,
        }
    }

    // ── build_ignore_set ────────────────────────────────────────

    #[test]
    fn build_ignore_set_empty_patterns() {
        let set = build_ignore_set(&[]);
        assert!(set.is_empty());
    }

    #[test]
    fn build_ignore_set_matches_glob() {
        let patterns = vec!["src/generated/**".to_string()];
        let set = build_ignore_set(&patterns);
        assert!(set.is_match(Path::new("src/generated/types.ts")));
        assert!(!set.is_match(Path::new("src/utils.ts")));
    }

    #[test]
    fn build_ignore_set_multiple_patterns() {
        let patterns = vec!["*.test.ts".to_string(), "dist/**".to_string()];
        let set = build_ignore_set(&patterns);
        assert!(set.is_match(Path::new("foo.test.ts")));
        assert!(set.is_match(Path::new("dist/index.js")));
        assert!(!set.is_match(Path::new("src/index.ts")));
    }

    #[test]
    fn build_ignore_set_skips_invalid_patterns() {
        // "[invalid" is not a valid glob — should be skipped, not panic
        let patterns = vec!["[invalid".to_string(), "*.js".to_string()];
        let set = build_ignore_set(&patterns);
        // The valid pattern should still work
        assert!(set.is_match(Path::new("foo.js")));
    }

    // ── collect_findings ────────────────────────────────────────

    #[test]
    fn collect_findings_empty_modules() {
        let (findings, files, functions) = collect_findings(
            &[],
            &FxHashMap::default(),
            Path::new("/project"),
            &globset::GlobSet::empty(),
            None,
            20,
            15,
        );
        assert!(findings.is_empty());
        assert_eq!(files, 0);
        assert_eq!(functions, 0);
    }

    #[test]
    fn collect_findings_below_threshold() {
        let path = PathBuf::from("/project/src/a.ts");
        let modules = vec![make_module(FileId(0), vec![make_fc("doStuff", 5, 3, 10)])];
        let mut file_paths = FxHashMap::default();
        file_paths.insert(FileId(0), &path);

        let (findings, files, functions) = collect_findings(
            &modules,
            &file_paths,
            Path::new("/project"),
            &globset::GlobSet::empty(),
            None,
            20,
            15,
        );
        assert!(findings.is_empty());
        assert_eq!(files, 1);
        assert_eq!(functions, 1);
    }

    #[test]
    fn collect_findings_exceeds_cyclomatic_only() {
        let path = PathBuf::from("/project/src/a.ts");
        let modules = vec![make_module(
            FileId(0),
            vec![make_fc("complexFn", 25, 5, 50)],
        )];
        let mut file_paths = FxHashMap::default();
        file_paths.insert(FileId(0), &path);

        let (findings, _, _) = collect_findings(
            &modules,
            &file_paths,
            Path::new("/project"),
            &globset::GlobSet::empty(),
            None,
            20,
            15,
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].cyclomatic, 25);
        assert!(matches!(
            findings[0].exceeded,
            ExceededThreshold::Cyclomatic
        ));
    }

    #[test]
    fn collect_findings_exceeds_cognitive_only() {
        let path = PathBuf::from("/project/src/a.ts");
        let modules = vec![make_module(FileId(0), vec![make_fc("nestedFn", 5, 20, 30)])];
        let mut file_paths = FxHashMap::default();
        file_paths.insert(FileId(0), &path);

        let (findings, _, _) = collect_findings(
            &modules,
            &file_paths,
            Path::new("/project"),
            &globset::GlobSet::empty(),
            None,
            20,
            15,
        );
        assert_eq!(findings.len(), 1);
        assert!(matches!(findings[0].exceeded, ExceededThreshold::Cognitive));
    }

    #[test]
    fn collect_findings_exceeds_both() {
        let path = PathBuf::from("/project/src/a.ts");
        let modules = vec![make_module(
            FileId(0),
            vec![make_fc("terribleFn", 25, 20, 100)],
        )];
        let mut file_paths = FxHashMap::default();
        file_paths.insert(FileId(0), &path);

        let (findings, _, _) = collect_findings(
            &modules,
            &file_paths,
            Path::new("/project"),
            &globset::GlobSet::empty(),
            None,
            20,
            15,
        );
        assert_eq!(findings.len(), 1);
        assert!(matches!(findings[0].exceeded, ExceededThreshold::Both));
    }

    #[test]
    fn collect_findings_multiple_functions_per_file() {
        let path = PathBuf::from("/project/src/a.ts");
        let modules = vec![make_module(
            FileId(0),
            vec![
                make_fc("ok", 5, 3, 10),
                make_fc("bad", 25, 20, 50),
                make_fc("also_bad", 21, 5, 30),
            ],
        )];
        let mut file_paths = FxHashMap::default();
        file_paths.insert(FileId(0), &path);

        let (findings, files, functions) = collect_findings(
            &modules,
            &file_paths,
            Path::new("/project"),
            &globset::GlobSet::empty(),
            None,
            20,
            15,
        );
        assert_eq!(findings.len(), 2);
        assert_eq!(files, 1);
        assert_eq!(functions, 3);
    }

    #[test]
    fn collect_findings_ignores_matching_files() {
        let path = PathBuf::from("/project/src/generated/types.ts");
        let modules = vec![make_module(FileId(0), vec![make_fc("genFn", 25, 20, 50)])];
        let mut file_paths = FxHashMap::default();
        file_paths.insert(FileId(0), &path);

        let ignore_set = build_ignore_set(&["src/generated/**".to_string()]);
        let (findings, files, _) = collect_findings(
            &modules,
            &file_paths,
            Path::new("/project"),
            &ignore_set,
            None,
            20,
            15,
        );
        assert!(findings.is_empty());
        assert_eq!(files, 0);
    }

    #[test]
    fn collect_findings_filters_by_changed_files() {
        let path_a = PathBuf::from("/project/src/a.ts");
        let path_b = PathBuf::from("/project/src/b.ts");
        let modules = vec![
            make_module(FileId(0), vec![make_fc("fnA", 25, 20, 50)]),
            make_module(FileId(1), vec![make_fc("fnB", 25, 20, 50)]),
        ];
        let mut file_paths = FxHashMap::default();
        file_paths.insert(FileId(0), &path_a);
        file_paths.insert(FileId(1), &path_b);

        let mut changed = FxHashSet::default();
        changed.insert(PathBuf::from("/project/src/a.ts"));

        let (findings, files, _) = collect_findings(
            &modules,
            &file_paths,
            Path::new("/project"),
            &globset::GlobSet::empty(),
            Some(&changed),
            20,
            15,
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].name, "fnA");
        assert_eq!(files, 1);
    }

    #[test]
    fn collect_findings_skips_module_without_path() {
        // Module with FileId(99) has no entry in file_paths
        let modules = vec![make_module(FileId(99), vec![make_fc("orphan", 25, 20, 50)])];
        let file_paths = FxHashMap::default();

        let (findings, files, _) = collect_findings(
            &modules,
            &file_paths,
            Path::new("/project"),
            &globset::GlobSet::empty(),
            None,
            20,
            15,
        );
        assert!(findings.is_empty());
        assert_eq!(files, 0);
    }

    #[test]
    fn collect_findings_at_exact_threshold_not_reported() {
        let path = PathBuf::from("/project/src/a.ts");
        let modules = vec![make_module(
            FileId(0),
            // Exactly at thresholds — should NOT be reported (> not >=)
            vec![make_fc("borderline", 20, 15, 20)],
        )];
        let mut file_paths = FxHashMap::default();
        file_paths.insert(FileId(0), &path);

        let (findings, _, _) = collect_findings(
            &modules,
            &file_paths,
            Path::new("/project"),
            &globset::GlobSet::empty(),
            None,
            20,
            15,
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn collect_findings_preserves_function_metadata() {
        let path = PathBuf::from("/project/src/a.ts");
        let modules = vec![make_module(
            FileId(0),
            vec![FunctionComplexity {
                name: "processData".to_string(),
                line: 42,
                col: 8,
                cyclomatic: 25,
                cognitive: 18,
                line_count: 75,
            }],
        )];
        let mut file_paths = FxHashMap::default();
        file_paths.insert(FileId(0), &path);

        let (findings, _, _) = collect_findings(
            &modules,
            &file_paths,
            Path::new("/project"),
            &globset::GlobSet::empty(),
            None,
            20,
            15,
        );
        assert_eq!(findings.len(), 1);
        let f = &findings[0];
        assert_eq!(f.name, "processData");
        assert_eq!(f.line, 42);
        assert_eq!(f.col, 8);
        assert_eq!(f.cyclomatic, 25);
        assert_eq!(f.cognitive, 18);
        assert_eq!(f.line_count, 75);
        assert_eq!(f.path, PathBuf::from("/project/src/a.ts"));
    }
}
