use std::process::ExitCode;
use std::time::Instant;

use fallow_config::OutputFormat;

use crate::baseline::{DuplicationBaselineData, filter_new_clone_groups, recompute_stats};
use crate::check::get_changed_files;
use crate::report;
use crate::{emit_error, load_config};

#[derive(Clone, clap::ValueEnum)]
pub enum DupesMode {
    Strict,
    Mild,
    Weak,
    Semantic,
}

pub struct DupesOptions<'a> {
    pub root: &'a std::path::Path,
    pub config_path: &'a Option<std::path::PathBuf>,
    pub output: OutputFormat,
    pub no_cache: bool,
    pub threads: usize,
    pub quiet: bool,
    pub mode: DupesMode,
    pub min_tokens: usize,
    pub min_lines: usize,
    pub threshold: f64,
    pub skip_local: bool,
    pub cross_language: bool,
    pub top: Option<usize>,
    pub baseline_path: Option<&'a std::path::Path>,
    pub save_baseline_path: Option<&'a std::path::Path>,
    pub production: bool,
    pub trace: Option<&'a str>,
    pub changed_since: Option<&'a str>,
    pub explain: bool,
}

/// Parse a `--trace` spec string into (file_path, line_number).
///
/// Returns `Err` with a human-readable message on invalid input.
fn parse_trace_spec(spec: &str) -> Result<(&str, usize), &'static str> {
    let (file_path, line_str) = spec
        .rsplit_once(':')
        .ok_or("--trace requires FILE:LINE format (e.g., src/utils.ts:42)")?;
    let line: usize = match line_str.parse() {
        Ok(l) if l > 0 => l,
        _ => return Err("--trace LINE must be a positive integer"),
    };
    Ok((file_path, line))
}

/// Build a `DuplicatesConfig` from CLI options, merging with values from the config file.
fn build_dupes_config(
    opts: &DupesOptions<'_>,
    toml_dupes: &fallow_config::DuplicatesConfig,
) -> fallow_config::DuplicatesConfig {
    fallow_config::DuplicatesConfig {
        enabled: true,
        mode: match opts.mode {
            DupesMode::Strict => fallow_config::DetectionMode::Strict,
            DupesMode::Mild => fallow_config::DetectionMode::Mild,
            DupesMode::Weak => fallow_config::DetectionMode::Weak,
            DupesMode::Semantic => fallow_config::DetectionMode::Semantic,
        },
        min_tokens: opts.min_tokens,
        min_lines: opts.min_lines,
        threshold: opts.threshold,
        ignore: toml_dupes.ignore.clone(),
        skip_local: opts.skip_local,
        cross_language: opts.cross_language || toml_dupes.cross_language,
        normalization: toml_dupes.normalization.clone(),
    }
}

/// Check whether duplication percentage exceeds the configured threshold.
///
/// Returns `true` if the threshold is positive and the duplication percentage exceeds it.
fn exceeds_threshold(threshold: f64, duplication_percentage: f64) -> bool {
    threshold > 0.0 && duplication_percentage > threshold
}

/// Filter a duplication report to only retain clone groups where at least one
/// instance belongs to a changed file. Families and stats are rebuilt from the
/// surviving groups.
fn filter_by_changed_files(
    report: &mut fallow_core::duplicates::DuplicationReport,
    changed: &rustc_hash::FxHashSet<std::path::PathBuf>,
) {
    report
        .clone_groups
        .retain(|g| g.instances.iter().any(|i| changed.contains(&i.file)));
    report.clone_families =
        fallow_core::duplicates::families::group_into_families(&report.clone_groups);
    report.stats = recompute_stats(report);
}

pub fn run_dupes(opts: &DupesOptions<'_>) -> ExitCode {
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

    // Build duplication config: start from fallow.toml, override with CLI args
    let dupes_config = build_dupes_config(opts, &config.duplicates);

    // Discover files
    let files = fallow_core::discover::discover_files(&config);

    // Run duplication detection
    let mut report = fallow_core::duplicates::find_duplicates(&config.root, &files, &dupes_config);

    // Handle trace (diagnostic mode — early return)
    if let Some(trace_spec) = opts.trace {
        let (file_path, line) = match parse_trace_spec(trace_spec) {
            Ok(parsed) => parsed,
            Err(msg) => return emit_error(msg, 2, &opts.output),
        };
        let trace_result = fallow_core::trace::trace_clone(&report, &config.root, file_path, line);
        if trace_result.matched_instance.is_none() {
            return emit_error(
                &format!("no clone found at {file_path}:{line}"),
                2,
                &opts.output,
            );
        }
        report::print_clone_trace(&trace_result, &config.root, &opts.output);
        return ExitCode::SUCCESS;
    }

    // Save baseline if requested (before filtering)
    if let Some(path) = opts.save_baseline_path {
        let baseline_data = DuplicationBaselineData::from_report(&report, &config.root);
        match serde_json::to_string_pretty(&baseline_data) {
            Ok(json) => {
                if let Err(e) = std::fs::write(path, json) {
                    return emit_error(
                        &format!("failed to write duplication baseline: {e}"),
                        2,
                        &opts.output,
                    );
                }
                if !opts.quiet {
                    eprintln!("Saved duplication baseline to {}", path.display());
                }
            }
            Err(e) => {
                return emit_error(
                    &format!("failed to serialize duplication baseline: {e}"),
                    2,
                    &opts.output,
                );
            }
        }
    }

    // Filter against baseline if provided
    if let Some(path) = opts.baseline_path {
        match std::fs::read_to_string(path) {
            Ok(json) => match serde_json::from_str::<DuplicationBaselineData>(&json) {
                Ok(baseline_data) => {
                    report = filter_new_clone_groups(report, &baseline_data, &config.root);
                }
                Err(e) => {
                    return emit_error(
                        &format!("failed to parse duplication baseline: {e}"),
                        2,
                        &opts.output,
                    );
                }
            },
            Err(e) => {
                return emit_error(
                    &format!("failed to read duplication baseline: {e}"),
                    2,
                    &opts.output,
                );
            }
        }
    }

    // Filter to only changed files if requested
    if let Some(git_ref) = opts.changed_since
        && let Some(changed) = get_changed_files(opts.root, git_ref)
    {
        filter_by_changed_files(&mut report, &changed);
    }

    // Apply --top: truncate display to N largest groups but keep stats over the full set
    if let Some(n) = opts.top {
        report.clone_groups.truncate(n);
        report.clone_families =
            fallow_core::duplicates::families::group_into_families(&report.clone_groups);
    }

    let elapsed = start.elapsed();

    // Print results
    let result = report::print_duplication_report(
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

    // Check threshold
    if exceeds_threshold(opts.threshold, report.stats.duplication_percentage) {
        eprintln!(
            "Duplication ({:.1}%) exceeds threshold ({:.1}%)",
            report.stats.duplication_percentage, opts.threshold
        );
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline::{DuplicationBaselineData, filter_new_clone_groups, recompute_stats};
    use fallow_config::{DetectionMode, DuplicatesConfig, NormalizationConfig};
    use fallow_core::duplicates::{CloneGroup, CloneInstance, DuplicationReport, DuplicationStats};
    use std::path::{Path, PathBuf};

    // ── Helpers ──────────────────────────────────────────────────────

    fn instance(file: &str, start: usize, end: usize) -> CloneInstance {
        CloneInstance {
            file: PathBuf::from(file),
            start_line: start,
            end_line: end,
            start_col: 0,
            end_col: 0,
            fragment: String::new(),
        }
    }

    fn make_group(instances: Vec<CloneInstance>, tokens: usize, lines: usize) -> CloneGroup {
        CloneGroup {
            instances,
            token_count: tokens,
            line_count: lines,
        }
    }

    fn make_report(
        groups: Vec<CloneGroup>,
        total_files: usize,
        total_lines: usize,
    ) -> DuplicationReport {
        let clone_instances: usize = groups.iter().map(|g| g.instances.len()).sum();
        DuplicationReport {
            clone_groups: groups,
            clone_families: vec![],
            stats: DuplicationStats {
                total_files,
                files_with_clones: 0,
                total_lines,
                duplicated_lines: 0,
                total_tokens: 0,
                duplicated_tokens: 0,
                clone_groups: 0,
                clone_instances,
                duplication_percentage: 0.0,
            },
        }
    }

    fn default_opts_for_config<'a>(root: &'a Path, mode: DupesMode) -> DupesOptions<'a> {
        DupesOptions {
            root,
            config_path: &None,
            output: OutputFormat::Human,
            no_cache: true,
            threads: 1,
            quiet: true,
            mode,
            min_tokens: 50,
            min_lines: 5,
            threshold: 0.0,
            skip_local: false,
            cross_language: false,
            top: None,
            baseline_path: None,
            save_baseline_path: None,
            production: false,
            trace: None,
            changed_since: None,
            explain: false,
        }
    }

    // ── parse_trace_spec ─────────────────────────────────────────────

    #[test]
    fn parse_trace_spec_valid() {
        let (file, line) = parse_trace_spec("src/utils.ts:42").unwrap();
        assert_eq!(file, "src/utils.ts");
        assert_eq!(line, 42);
    }

    #[test]
    fn parse_trace_spec_windows_path_with_drive() {
        // The rsplit_once(':') should split on the LAST colon, so
        // C:\path\file.ts:10 -> file = "C:\path\file.ts", line = 10
        let (file, line) = parse_trace_spec("C:\\path\\file.ts:10").unwrap();
        assert_eq!(file, "C:\\path\\file.ts");
        assert_eq!(line, 10);
    }

    #[test]
    fn parse_trace_spec_no_colon() {
        let err = parse_trace_spec("src/utils.ts").unwrap_err();
        assert!(
            err.contains("FILE:LINE"),
            "error should mention FILE:LINE format"
        );
    }

    #[test]
    fn parse_trace_spec_line_zero() {
        let err = parse_trace_spec("src/utils.ts:0").unwrap_err();
        assert!(err.contains("positive integer"));
    }

    #[test]
    fn parse_trace_spec_negative_line() {
        // "-1" cannot parse as usize, so it hits the catch-all error
        let err = parse_trace_spec("src/utils.ts:-1").unwrap_err();
        assert!(err.contains("positive integer"));
    }

    #[test]
    fn parse_trace_spec_non_numeric_line() {
        let err = parse_trace_spec("src/utils.ts:abc").unwrap_err();
        assert!(err.contains("positive integer"));
    }

    #[test]
    fn parse_trace_spec_empty_line() {
        // "src/utils.ts:" -> line_str = ""
        let err = parse_trace_spec("src/utils.ts:").unwrap_err();
        assert!(err.contains("positive integer"));
    }

    #[test]
    fn parse_trace_spec_large_line_number() {
        let (file, line) = parse_trace_spec("src/app.ts:999999").unwrap();
        assert_eq!(file, "src/app.ts");
        assert_eq!(line, 999_999);
    }

    #[test]
    fn parse_trace_spec_file_with_colons_in_path() {
        // Edge case: file path contains colons (e.g., absolute path on Windows or unusual naming)
        // rsplit_once splits at the LAST colon, so "a:b:c:10" -> ("a:b:c", "10")
        let (file, line) = parse_trace_spec("a:b:c:10").unwrap();
        assert_eq!(file, "a:b:c");
        assert_eq!(line, 10);
    }

    // ── exceeds_threshold ────────────────────────────────────────────

    #[test]
    fn threshold_zero_never_fails() {
        // When threshold is 0.0 (disabled), even 100% duplication should pass
        assert!(!exceeds_threshold(0.0, 100.0));
    }

    #[test]
    fn threshold_negative_never_fails() {
        // Negative threshold is nonsensical but should not trigger failure
        assert!(!exceeds_threshold(-1.0, 50.0));
    }

    #[test]
    fn threshold_exceeded() {
        assert!(exceeds_threshold(5.0, 10.0));
    }

    #[test]
    fn threshold_exactly_at_boundary() {
        // Duplication == threshold should NOT exceed (the condition is strict >)
        assert!(!exceeds_threshold(5.0, 5.0));
    }

    #[test]
    fn threshold_just_below() {
        assert!(!exceeds_threshold(5.0, 4.9));
    }

    #[test]
    fn threshold_just_above() {
        assert!(exceeds_threshold(5.0, 5.01));
    }

    #[test]
    fn threshold_zero_duplication_with_positive_threshold() {
        assert!(!exceeds_threshold(5.0, 0.0));
    }

    // ── build_dupes_config ───────────────────────────────────────────

    #[test]
    fn build_config_maps_all_modes() {
        let root = PathBuf::from("/project");
        let toml = DuplicatesConfig::default();
        for (cli_mode, expected) in [
            (DupesMode::Strict, DetectionMode::Strict),
            (DupesMode::Mild, DetectionMode::Mild),
            (DupesMode::Weak, DetectionMode::Weak),
            (DupesMode::Semantic, DetectionMode::Semantic),
        ] {
            let opts = default_opts_for_config(&root, cli_mode);
            let config = build_dupes_config(&opts, &toml);
            assert_eq!(config.mode, expected);
        }
    }

    #[test]
    fn build_config_always_enabled() {
        let root = PathBuf::from("/project");
        let opts = default_opts_for_config(&root, DupesMode::Mild);
        let mut toml = DuplicatesConfig::default();
        toml.enabled = false;
        let config = build_dupes_config(&opts, &toml);
        // The dupes command always enables duplication detection
        assert!(config.enabled);
    }

    #[test]
    fn build_config_cross_language_cli_true_overrides_toml_false() {
        let root = PathBuf::from("/project");
        let mut opts = default_opts_for_config(&root, DupesMode::Mild);
        opts.cross_language = true;
        let toml = DuplicatesConfig::default(); // cross_language = false
        let config = build_dupes_config(&opts, &toml);
        assert!(config.cross_language);
    }

    #[test]
    fn build_config_cross_language_toml_true_with_cli_false() {
        let root = PathBuf::from("/project");
        let opts = default_opts_for_config(&root, DupesMode::Mild); // cross_language = false
        let mut toml = DuplicatesConfig::default();
        toml.cross_language = true;
        let config = build_dupes_config(&opts, &toml);
        // OR semantics: toml.cross_language || opts.cross_language
        assert!(config.cross_language);
    }

    #[test]
    fn build_config_cross_language_both_false() {
        let root = PathBuf::from("/project");
        let opts = default_opts_for_config(&root, DupesMode::Mild);
        let toml = DuplicatesConfig::default();
        let config = build_dupes_config(&opts, &toml);
        assert!(!config.cross_language);
    }

    #[test]
    fn build_config_inherits_ignore_from_toml() {
        let root = PathBuf::from("/project");
        let opts = default_opts_for_config(&root, DupesMode::Mild);
        let mut toml = DuplicatesConfig::default();
        toml.ignore = vec!["**/*.generated.ts".to_string()];
        let config = build_dupes_config(&opts, &toml);
        assert_eq!(config.ignore, vec!["**/*.generated.ts"]);
    }

    #[test]
    fn build_config_inherits_normalization_from_toml() {
        let root = PathBuf::from("/project");
        let opts = default_opts_for_config(&root, DupesMode::Mild);
        let mut toml = DuplicatesConfig::default();
        toml.normalization = NormalizationConfig {
            ignore_identifiers: Some(true),
            ignore_string_values: None,
            ignore_numeric_values: Some(false),
        };
        let config = build_dupes_config(&opts, &toml);
        assert_eq!(config.normalization.ignore_identifiers, Some(true));
        assert!(config.normalization.ignore_string_values.is_none());
        assert_eq!(config.normalization.ignore_numeric_values, Some(false));
    }

    #[test]
    fn build_config_uses_cli_min_tokens_and_lines() {
        let root = PathBuf::from("/project");
        let mut opts = default_opts_for_config(&root, DupesMode::Mild);
        opts.min_tokens = 100;
        opts.min_lines = 10;
        let toml = DuplicatesConfig::default();
        let config = build_dupes_config(&opts, &toml);
        assert_eq!(config.min_tokens, 100);
        assert_eq!(config.min_lines, 10);
    }

    #[test]
    fn build_config_uses_cli_threshold() {
        let root = PathBuf::from("/project");
        let mut opts = default_opts_for_config(&root, DupesMode::Mild);
        opts.threshold = 7.5;
        let toml = DuplicatesConfig::default();
        let config = build_dupes_config(&opts, &toml);
        assert!((config.threshold - 7.5).abs() < f64::EPSILON);
    }

    #[test]
    fn build_config_uses_cli_skip_local() {
        let root = PathBuf::from("/project");
        let mut opts = default_opts_for_config(&root, DupesMode::Mild);
        opts.skip_local = true;
        let toml = DuplicatesConfig::default();
        let config = build_dupes_config(&opts, &toml);
        assert!(config.skip_local);
    }

    // ── DuplicationBaselineData integration ──────────────────────────

    #[test]
    fn baseline_save_load_round_trip() {
        let root = Path::new("/project");
        let group = make_group(
            vec![
                instance("/project/src/a.ts", 1, 10),
                instance("/project/src/b.ts", 1, 10),
            ],
            50,
            10,
        );
        let report = make_report(vec![group], 10, 1000);
        let baseline = DuplicationBaselineData::from_report(&report, root);

        // Serialize and deserialize
        let json = serde_json::to_string_pretty(&baseline).unwrap();
        let loaded: DuplicationBaselineData = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.clone_groups, baseline.clone_groups);
    }

    #[test]
    fn baseline_filters_matching_groups_completely() {
        let root = Path::new("/project");
        let group = make_group(
            vec![
                instance("/project/src/a.ts", 1, 10),
                instance("/project/src/b.ts", 1, 10),
            ],
            50,
            10,
        );
        let report = make_report(vec![group.clone()], 10, 1000);
        let baseline = DuplicationBaselineData::from_report(&report, root);

        let filtered = filter_new_clone_groups(report, &baseline, root);
        assert!(filtered.clone_groups.is_empty());
        assert_eq!(filtered.stats.clone_groups, 0);
        assert_eq!(filtered.stats.clone_instances, 0);
    }

    #[test]
    fn baseline_keeps_groups_not_in_baseline() {
        let root = Path::new("/project");
        let old_group = make_group(
            vec![
                instance("/project/src/a.ts", 1, 10),
                instance("/project/src/b.ts", 1, 10),
            ],
            50,
            10,
        );
        let new_group = make_group(
            vec![
                instance("/project/src/c.ts", 20, 30),
                instance("/project/src/d.ts", 20, 30),
            ],
            60,
            11,
        );

        let baseline_report = make_report(vec![old_group.clone()], 10, 1000);
        let baseline = DuplicationBaselineData::from_report(&baseline_report, root);

        let report = make_report(vec![old_group, new_group], 10, 1000);
        let filtered = filter_new_clone_groups(report, &baseline, root);
        assert_eq!(filtered.clone_groups.len(), 1);
        // The remaining group should be the new one (c.ts, d.ts)
        assert_eq!(filtered.clone_groups[0].instances.len(), 2);
        assert!(
            filtered.clone_groups[0]
                .instances
                .iter()
                .any(|i| i.file == PathBuf::from("/project/src/c.ts"))
        );
    }

    // ── recompute_stats ──────────────────────────────────────────────

    #[test]
    fn recompute_stats_empty_report() {
        let report = DuplicationReport::default();
        let stats = recompute_stats(&report);
        assert_eq!(stats.clone_groups, 0);
        assert_eq!(stats.clone_instances, 0);
        assert_eq!(stats.duplicated_lines, 0);
        assert_eq!(stats.duplication_percentage, 0.0);
    }

    #[test]
    fn recompute_stats_basic() {
        let group = make_group(
            vec![
                instance("/project/src/a.ts", 1, 5),
                instance("/project/src/b.ts", 1, 5),
            ],
            30,
            5,
        );
        let mut report = make_report(vec![group], 10, 100);
        report.stats.total_lines = 100;
        let stats = recompute_stats(&report);
        assert_eq!(stats.clone_groups, 1);
        assert_eq!(stats.clone_instances, 2);
        // 5 lines in a.ts + 5 lines in b.ts = 10 duplicated lines
        assert_eq!(stats.duplicated_lines, 10);
        assert!((stats.duplication_percentage - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn recompute_stats_deduplicates_overlapping_lines_in_same_file() {
        // Two groups both mark lines 3-7 as cloned in the same file
        let group1 = make_group(
            vec![
                instance("/project/src/a.ts", 1, 5),
                instance("/project/src/b.ts", 1, 5),
            ],
            30,
            5,
        );
        let group2 = make_group(
            vec![
                instance("/project/src/a.ts", 3, 7),
                instance("/project/src/c.ts", 10, 14),
            ],
            30,
            5,
        );
        let mut report = make_report(vec![group1, group2], 10, 100);
        report.stats.total_lines = 100;
        let stats = recompute_stats(&report);
        // a.ts: lines 1-5 + lines 3-7 = lines 1-7 = 7 unique lines
        // b.ts: lines 1-5 = 5 unique lines
        // c.ts: lines 10-14 = 5 unique lines
        assert_eq!(stats.duplicated_lines, 17);
        assert_eq!(stats.files_with_clones, 3);
    }

    #[test]
    fn recompute_stats_zero_total_lines_no_division_by_zero() {
        let mut report = DuplicationReport::default();
        report.stats.total_lines = 0;
        let stats = recompute_stats(&report);
        assert_eq!(stats.duplication_percentage, 0.0);
    }

    #[test]
    fn recompute_stats_computes_all_fields_from_groups() {
        let group1 = make_group(
            vec![
                instance("/project/src/a.ts", 1, 10),
                instance("/project/src/b.ts", 1, 10),
            ],
            50,
            10,
        );
        let group2 = make_group(
            vec![
                instance("/project/src/c.ts", 20, 25),
                instance("/project/src/d.ts", 20, 25),
            ],
            30,
            6,
        );
        let mut report = make_report(vec![group1, group2], 20, 500);
        report.stats.total_lines = 500;
        report.stats.total_tokens = 10000;
        let stats = recompute_stats(&report);
        // Computed: 2 groups
        assert_eq!(stats.clone_groups, 2);
        // Computed: 4 instances total
        assert_eq!(stats.clone_instances, 4);
        // Computed: a.ts 10 + b.ts 10 + c.ts 6 + d.ts 6 = 32 duplicated lines
        assert_eq!(stats.duplicated_lines, 32);
        // Computed: (50*2) + (30*2) = 160 duplicated tokens
        assert_eq!(stats.duplicated_tokens, 160);
        // Computed: 4 unique files with clones
        assert_eq!(stats.files_with_clones, 4);
        // Computed: 32/500 * 100 = 6.4%
        assert!((stats.duplication_percentage - 6.4).abs() < f64::EPSILON);
    }

    // ── filter_by_changed_files ─────────────────────────────────────

    #[test]
    fn filter_by_changed_files_retains_groups_with_at_least_one_changed_instance() {
        let group = make_group(
            vec![instance("src/a.ts", 1, 10), instance("src/b.ts", 1, 10)],
            50,
            10,
        );
        let mut report = make_report(vec![group], 10, 1000);
        let changed: rustc_hash::FxHashSet<PathBuf> =
            [PathBuf::from("src/a.ts")].into_iter().collect();

        filter_by_changed_files(&mut report, &changed);

        assert_eq!(report.clone_groups.len(), 1);
        assert_eq!(
            report.clone_families.len(),
            1,
            "families should be rebuilt after filtering"
        );
    }

    #[test]
    fn filter_by_changed_files_removes_groups_with_no_changed_instances() {
        let group = make_group(
            vec![instance("src/a.ts", 1, 10), instance("src/b.ts", 1, 10)],
            50,
            10,
        );
        let mut report = make_report(vec![group], 10, 1000);
        let changed: rustc_hash::FxHashSet<PathBuf> =
            [PathBuf::from("src/c.ts")].into_iter().collect();

        filter_by_changed_files(&mut report, &changed);

        assert!(report.clone_groups.is_empty());
    }

    #[test]
    fn filter_by_changed_files_partial_group_retention() {
        // Group 1: a.ts <-> b.ts (a.ts is changed)
        let group1 = make_group(
            vec![instance("src/a.ts", 1, 10), instance("src/b.ts", 1, 10)],
            50,
            10,
        );
        // Group 2: c.ts <-> d.ts (neither is changed)
        let group2 = make_group(
            vec![instance("src/c.ts", 1, 10), instance("src/d.ts", 1, 10)],
            50,
            10,
        );
        let mut report = make_report(vec![group1, group2], 10, 1000);
        let changed: rustc_hash::FxHashSet<PathBuf> =
            [PathBuf::from("src/a.ts")].into_iter().collect();

        filter_by_changed_files(&mut report, &changed);

        assert_eq!(report.clone_groups.len(), 1);
        // The retained group should be the one containing a.ts
        assert!(
            report.clone_groups[0]
                .instances
                .iter()
                .any(|i| i.file == PathBuf::from("src/a.ts"))
        );
    }

    #[test]
    fn filter_by_changed_files_empty_changed_set_removes_all() {
        let group = make_group(
            vec![instance("src/a.ts", 1, 10), instance("src/b.ts", 1, 10)],
            50,
            10,
        );
        let mut report = make_report(vec![group], 10, 1000);
        let changed: rustc_hash::FxHashSet<PathBuf> = rustc_hash::FxHashSet::default();

        filter_by_changed_files(&mut report, &changed);

        assert!(report.clone_groups.is_empty());
    }

    #[test]
    fn baseline_empty_json_object_uses_defaults() {
        // An empty JSON object should deserialize with empty clone_groups
        // (this tests that the format is forward-compatible)
        let result = serde_json::from_str::<DuplicationBaselineData>(r#"{"clone_groups": []}"#);
        assert!(result.is_ok());
        assert!(result.unwrap().clone_groups.is_empty());
    }

    // ── Families rebuilt after filtering ──────────────────────────────

    #[test]
    fn families_rebuilt_after_baseline_filter() {
        let root = Path::new("/project");
        let group1 = make_group(
            vec![
                instance("/project/src/a.ts", 1, 10),
                instance("/project/src/b.ts", 1, 10),
            ],
            50,
            10,
        );
        let group2 = make_group(
            vec![
                instance("/project/src/c.ts", 20, 30),
                instance("/project/src/d.ts", 20, 30),
            ],
            60,
            11,
        );

        // Baseline only knows about group1
        let baseline_report = make_report(vec![group1.clone()], 10, 1000);
        let baseline = DuplicationBaselineData::from_report(&baseline_report, root);

        // Full report has both groups
        let report = make_report(vec![group1, group2], 10, 1000);
        let filtered = filter_new_clone_groups(report, &baseline, root);

        // Families should be rebuilt from the remaining group(s)
        assert_eq!(filtered.clone_groups.len(), 1);
        assert_eq!(filtered.clone_families.len(), 1);
        assert_eq!(filtered.clone_families[0].groups.len(), 1);
    }

    // ── Stats after changed_since filter ─────────────────────────────

    #[test]
    fn stats_recomputed_after_changed_since_filter() {
        let group = make_group(
            vec![instance("src/a.ts", 1, 5), instance("src/b.ts", 1, 5)],
            30,
            5,
        );
        let mut report = make_report(vec![group], 10, 100);
        report.stats.total_lines = 100;
        report.stats.total_tokens = 5000;
        report.stats.total_files = 10;

        let changed: rustc_hash::FxHashSet<PathBuf> =
            [PathBuf::from("src/x.ts")].into_iter().collect();

        filter_by_changed_files(&mut report, &changed);

        // All groups filtered out, stats should reflect that
        assert_eq!(report.stats.clone_groups, 0);
        assert_eq!(report.stats.clone_instances, 0);
        assert_eq!(report.stats.duplicated_lines, 0);
        assert_eq!(report.stats.duplication_percentage, 0.0);
        // Pass-through fields are preserved from the original stats
        assert_eq!(report.stats.total_lines, 100);
        assert_eq!(report.stats.total_tokens, 5000);
        assert_eq!(report.stats.total_files, 10);
    }

    // ── recompute_stats token counting ───────────────────────────────

    #[test]
    fn recompute_stats_counts_tokens_per_instance() {
        let group = make_group(
            vec![
                instance("/project/src/a.ts", 1, 5),
                instance("/project/src/b.ts", 1, 5),
                instance("/project/src/c.ts", 1, 5),
            ],
            40,
            5,
        );
        let mut report = make_report(vec![group], 10, 100);
        report.stats.total_lines = 100;
        let stats = recompute_stats(&report);
        // 40 tokens * 3 instances = 120
        assert_eq!(stats.duplicated_tokens, 120);
    }
}
