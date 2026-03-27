use std::process::ExitCode;
use std::time::Instant;

use fallow_config::OutputFormat;

use crate::check::{CheckOptions, CheckResult, IssueFilters, TraceOptions};
use crate::dupes::{DupesMode, DupesOptions, DupesResult};
use crate::health::{HealthOptions, HealthResult, SortBy};
use crate::report;
use crate::{AnalysisKind, emit_error};

pub struct CombinedOptions<'a> {
    pub root: &'a std::path::Path,
    pub config_path: &'a Option<std::path::PathBuf>,
    pub output: OutputFormat,
    pub no_cache: bool,
    pub threads: usize,
    pub quiet: bool,
    pub fail_on_issues: bool,
    pub sarif_file: Option<&'a std::path::Path>,
    pub changed_since: Option<&'a str>,
    pub baseline: Option<&'a std::path::Path>,
    pub save_baseline: Option<&'a std::path::Path>,
    pub production: bool,
    pub workspace: Option<&'a str>,
    pub explain: bool,
    pub performance: bool,
    pub run_check: bool,
    pub run_dupes: bool,
    pub run_health: bool,
}

/// Resolve which analyses to run based on --only/--skip flags.
/// Precondition: only and skip must not both be non-empty (validated in main.rs).
pub fn resolve_analyses(only: &[AnalysisKind], skip: &[AnalysisKind]) -> (bool, bool, bool) {
    if !only.is_empty() {
        (
            only.contains(&AnalysisKind::DeadCode),
            only.contains(&AnalysisKind::Dupes),
            only.contains(&AnalysisKind::Health),
        )
    } else if !skip.is_empty() {
        (
            !skip.contains(&AnalysisKind::DeadCode),
            !skip.contains(&AnalysisKind::Dupes),
            !skip.contains(&AnalysisKind::Health),
        )
    } else {
        (true, true, true)
    }
}

pub fn run_combined(opts: &CombinedOptions<'_>) -> ExitCode {
    let start = Instant::now();
    let mut max_exit: u8 = 0;

    let mut check_result: Option<CheckResult> = None;
    let mut dupes_result: Option<DupesResult> = None;
    let mut health_result: Option<HealthResult> = None;

    // Run check (dead code analysis)
    if opts.run_check {
        let filters = IssueFilters::default();
        let trace_opts = TraceOptions {
            trace_export: None,
            trace_file: None,
            trace_dependency: None,
            performance: opts.performance,
        };
        let check_opts = CheckOptions {
            root: opts.root,
            config_path: opts.config_path,
            output: opts.output.clone(),
            no_cache: opts.no_cache,
            threads: opts.threads,
            quiet: opts.quiet,
            fail_on_issues: opts.fail_on_issues,
            filters: &filters,
            changed_since: opts.changed_since,
            baseline: opts.baseline,
            save_baseline: opts.save_baseline,
            sarif_file: opts.sarif_file,
            production: opts.production,
            workspace: opts.workspace,
            include_dupes: false,
            trace_opts: &trace_opts,
            explain: opts.explain,
        };
        match crate::check::execute_check(&check_opts) {
            Ok(result) => {
                check_result = Some(result);
            }
            Err(code) => return code,
        }
    }

    // Run dupes (duplication analysis)
    if opts.run_dupes {
        let dupes_opts = DupesOptions {
            root: opts.root,
            config_path: opts.config_path,
            output: opts.output.clone(),
            no_cache: opts.no_cache,
            threads: opts.threads,
            quiet: opts.quiet,
            mode: DupesMode::Mild,
            min_tokens: 50,
            min_lines: 5,
            threshold: 0.0,
            skip_local: false,
            cross_language: false,
            top: None,
            baseline_path: None,
            save_baseline_path: None,
            production: opts.production,
            trace: None,
            changed_since: opts.changed_since,
            explain: opts.explain,
        };
        match crate::dupes::execute_dupes(&dupes_opts) {
            Ok(result) => {
                dupes_result = Some(result);
            }
            Err(code) => return code,
        }
    }

    // Run health (complexity analysis)
    if opts.run_health {
        let health_opts = build_health_opts(opts);
        match crate::health::execute_health(&health_opts) {
            Ok(result) => {
                health_result = Some(result);
            }
            Err(code) => return code,
        }
    }

    let total_elapsed = start.elapsed();

    // Print combined report
    match opts.output {
        OutputFormat::Json => {
            // JSON: single combined object with check/dupes/health keys
            let code = print_combined_json(
                check_result.as_ref(),
                dupes_result.as_ref(),
                health_result.as_ref(),
                total_elapsed,
                opts.explain,
            );
            if code != ExitCode::SUCCESS {
                return code;
            }
        }
        OutputFormat::Sarif => {
            // SARIF: multi-run document with one run per analysis
            let code = print_combined_sarif(
                check_result.as_ref(),
                dupes_result.as_ref(),
                health_result.as_ref(),
            );
            if code != ExitCode::SUCCESS {
                return code;
            }
        }
        OutputFormat::CodeClimate => {
            // CodeClimate: single JSON array merging all analyses
            let code = print_combined_codeclimate(
                check_result.as_ref(),
                dupes_result.as_ref(),
                health_result.as_ref(),
            );
            if code != ExitCode::SUCCESS {
                return code;
            }
        }
        _ => {
            // Human/Compact/Markdown: print each section sequentially
            let show_headers = matches!(opts.output, OutputFormat::Human) && !opts.quiet;

            if let Some(ref result) = check_result {
                if show_headers {
                    eprintln!();
                    eprintln!("── Dead Code ──────────────────────────────────────");
                }
                let code = crate::check::print_check_result(result, opts.quiet, opts.explain);
                max_exit = max_exit.max(exit_code_to_u8(code));
            }

            if let Some(ref result) = dupes_result {
                if show_headers {
                    eprintln!();
                    eprintln!("── Duplication ────────────────────────────────────");
                }
                let code = crate::dupes::print_dupes_result(result, opts.quiet, opts.explain);
                max_exit = max_exit.max(exit_code_to_u8(code));
            }

            if let Some(ref result) = health_result {
                if show_headers {
                    eprintln!();
                    eprintln!("── Complexity ─────────────────────────────────────");
                }
                let code = crate::health::print_health_result(result, opts.quiet, opts.explain);
                max_exit = max_exit.max(exit_code_to_u8(code));
            }
        }
    }

    // Summary on failure
    if max_exit > 0 && !opts.quiet {
        let mut parts = Vec::new();
        if let Some(ref r) = check_result {
            let issues = r.results.total_issues();
            if issues > 0 {
                parts.push(format!("check ({issues} issues)"));
            }
        }
        if let Some(ref r) = dupes_result {
            let groups = r.report.clone_groups.len();
            if groups > 0 {
                parts.push(format!("dupes ({groups} clone groups)"));
            }
        }
        if !parts.is_empty() {
            eprintln!("\nFailed: {}", parts.join(", "));
        }
    }

    ExitCode::from(max_exit)
}

/// Print combined JSON output wrapping check, dupes, and health results.
fn print_combined_json(
    check: Option<&CheckResult>,
    dupes: Option<&DupesResult>,
    health: Option<&HealthResult>,
    elapsed: std::time::Duration,
    _explain: bool,
) -> ExitCode {
    let mut combined = serde_json::Map::new();
    combined.insert("schema_version".into(), serde_json::Value::Number(3.into()));
    combined.insert(
        "version".into(),
        serde_json::Value::String(env!("CARGO_PKG_VERSION").to_string()),
    );
    combined.insert(
        "elapsed_ms".into(),
        serde_json::Value::Number(serde_json::Number::from(elapsed.as_millis() as u64)),
    );

    if let Some(result) = check {
        match report::build_json(&result.results, &result.config.root, result.elapsed) {
            Ok(json) => {
                combined.insert("check".into(), json);
            }
            Err(e) => {
                return emit_error(
                    &format!("JSON serialization error: {e}"),
                    2,
                    &OutputFormat::Json,
                );
            }
        }
    }

    if let Some(result) = dupes {
        match serde_json::to_value(&result.report) {
            Ok(json) => {
                combined.insert("dupes".into(), json);
            }
            Err(e) => {
                return emit_error(
                    &format!("JSON serialization error: {e}"),
                    2,
                    &OutputFormat::Json,
                );
            }
        }
    }

    if let Some(result) = health {
        match serde_json::to_value(&result.report) {
            Ok(json) => {
                combined.insert("health".into(), json);
            }
            Err(e) => {
                return emit_error(
                    &format!("JSON serialization error: {e}"),
                    2,
                    &OutputFormat::Json,
                );
            }
        }
    }

    match serde_json::to_string_pretty(&serde_json::Value::Object(combined)) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => emit_error(
            &format!("JSON serialization error: {e}"),
            2,
            &OutputFormat::Json,
        ),
    }
}

/// Print combined SARIF with multiple runs (one per analysis).
fn print_combined_sarif(
    check: Option<&CheckResult>,
    dupes: Option<&DupesResult>,
    health: Option<&HealthResult>,
) -> ExitCode {
    let mut all_runs = Vec::new();

    if let Some(result) = check {
        let sarif = report::build_sarif(&result.results, &result.config.root, &result.config.rules);
        if let Some(runs) = sarif.get("runs").and_then(|r| r.as_array()) {
            all_runs.extend(runs.iter().cloned());
        }
    }

    // Duplication SARIF builder is pub(super) — serialize the report as a simple run
    if let Some(result) = dupes.filter(|r| !r.report.clone_groups.is_empty()) {
        let run = serde_json::json!({
            "tool": {
                "driver": {
                    "name": "fallow",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/fallow-rs/fallow",
                }
            },
            "automationDetails": { "id": "fallow/dupes" },
            "results": result.report.clone_groups.iter().enumerate().map(|(i, g)| {
                serde_json::json!({
                    "ruleId": "fallow/code-duplication",
                    "level": "warning",
                    "message": { "text": format!("Clone group {} ({} lines, {} instances)", i + 1, g.line_count, g.instances.len()) },
                })
            }).collect::<Vec<_>>()
        });
        all_runs.push(run);
    }

    if let Some(result) = health {
        let sarif = report::build_health_sarif(&result.report, &result.config.root);
        if let Some(runs) = sarif.get("runs").and_then(|r| r.as_array()) {
            all_runs.extend(runs.iter().cloned());
        }
    }

    let combined = serde_json::json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": all_runs,
    });

    match serde_json::to_string_pretty(&combined) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => emit_error(
            &format!("SARIF serialization error: {e}"),
            2,
            &OutputFormat::Sarif,
        ),
    }
}

/// Print combined `CodeClimate` output merging all analyses into one JSON array.
fn print_combined_codeclimate(
    check: Option<&CheckResult>,
    dupes: Option<&DupesResult>,
    health: Option<&HealthResult>,
) -> ExitCode {
    let mut all_issues = Vec::new();

    if let Some(result) = check
        && let serde_json::Value::Array(items) =
            report::build_codeclimate(&result.results, &result.config.root, &result.config.rules)
    {
        all_issues.extend(items);
    }

    if let Some(result) = dupes
        && let serde_json::Value::Array(items) =
            report::build_duplication_codeclimate(&result.report, &result.config.root)
    {
        all_issues.extend(items);
    }

    if let Some(result) = health
        && let serde_json::Value::Array(items) =
            report::build_health_codeclimate(&result.report, &result.config.root)
    {
        all_issues.extend(items);
    }

    match serde_json::to_string_pretty(&serde_json::Value::Array(all_issues)) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => emit_error(
            &format!("CodeClimate serialization error: {e}"),
            2,
            &OutputFormat::CodeClimate,
        ),
    }
}

fn build_health_opts<'a>(opts: &'a CombinedOptions<'a>) -> HealthOptions<'a> {
    HealthOptions {
        root: opts.root,
        config_path: opts.config_path,
        output: opts.output.clone(),
        no_cache: opts.no_cache,
        threads: opts.threads,
        quiet: opts.quiet,
        max_cyclomatic: None,
        max_cognitive: None,
        top: None,
        sort: SortBy::Cyclomatic,
        production: opts.production,
        changed_since: opts.changed_since,
        workspace: opts.workspace,
        baseline: None,
        save_baseline: None,
        complexity: true,
        file_scores: true,
        hotspots: true,
        targets: true,
        since: None,
        min_commits: None,
        explain: opts.explain,
        save_snapshot: None,
    }
}

/// Convert an ExitCode to u8 for comparison.
/// ExitCode doesn't implement Ord, so we use this workaround.
fn exit_code_to_u8(code: ExitCode) -> u8 {
    u8::from(code != ExitCode::SUCCESS)
}
