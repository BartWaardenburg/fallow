use std::process::ExitCode;
use std::time::{Duration, Instant};

use fallow_config::{OutputFormat, ResolvedConfig};
use fallow_core::results::AnalysisResults;

use crate::baseline::{BaselineData, filter_new_issues};
use crate::report;
use crate::{emit_error, load_config};

mod filtering;
mod output;
mod rules;

pub use filtering::get_changed_files;
pub use filtering::resolve_workspace_filter;

// ── Issue type filters ──────────────────────────────────────────

#[derive(Default)]
pub struct IssueFilters {
    pub unused_files: bool,
    pub unused_exports: bool,
    pub unused_deps: bool,
    pub unused_types: bool,
    pub unused_enum_members: bool,
    pub unused_class_members: bool,
    pub unresolved_imports: bool,
    pub unlisted_deps: bool,
    pub duplicate_exports: bool,
    pub circular_deps: bool,
}

impl IssueFilters {
    pub const fn any_active(&self) -> bool {
        self.unused_files
            || self.unused_exports
            || self.unused_deps
            || self.unused_types
            || self.unused_enum_members
            || self.unused_class_members
            || self.unresolved_imports
            || self.unlisted_deps
            || self.duplicate_exports
            || self.circular_deps
    }

    /// When any filter is active, clear issue types that were NOT requested.
    pub fn apply(&self, results: &mut fallow_core::results::AnalysisResults) {
        if !self.any_active() {
            return;
        }
        if !self.unused_files {
            results.unused_files.clear();
        }
        if !self.unused_exports {
            results.unused_exports.clear();
        }
        if !self.unused_types {
            results.unused_types.clear();
        }
        if !self.unused_deps {
            results.unused_dependencies.clear();
            results.unused_dev_dependencies.clear();
            results.unused_optional_dependencies.clear();
            results.type_only_dependencies.clear();
        }
        if !self.unused_enum_members {
            results.unused_enum_members.clear();
        }
        if !self.unused_class_members {
            results.unused_class_members.clear();
        }
        if !self.unresolved_imports {
            results.unresolved_imports.clear();
        }
        if !self.unlisted_deps {
            results.unlisted_dependencies.clear();
        }
        if !self.duplicate_exports {
            results.duplicate_exports.clear();
        }
        if !self.circular_deps {
            results.circular_dependencies.clear();
        }
    }
}

// ── Trace options ───────────────────────────────────────────────

pub struct TraceOptions {
    pub trace_export: Option<String>,
    pub trace_file: Option<String>,
    pub trace_dependency: Option<String>,
    pub performance: bool,
}

impl TraceOptions {
    pub const fn any_active(&self) -> bool {
        self.trace_export.is_some()
            || self.trace_file.is_some()
            || self.trace_dependency.is_some()
            || self.performance
    }
}

// ── Check command ────────────────────────────────────────────────

pub struct CheckOptions<'a> {
    pub root: &'a std::path::Path,
    pub config_path: &'a Option<std::path::PathBuf>,
    pub output: OutputFormat,
    pub no_cache: bool,
    pub threads: usize,
    pub quiet: bool,
    pub fail_on_issues: bool,
    pub filters: &'a IssueFilters,
    pub changed_since: Option<&'a str>,
    pub baseline: Option<&'a std::path::Path>,
    pub save_baseline: Option<&'a std::path::Path>,
    pub sarif_file: Option<&'a std::path::Path>,
    pub production: bool,
    pub workspace: Option<&'a str>,
    pub include_dupes: bool,
    pub trace_opts: &'a TraceOptions,
    pub explain: bool,
}

/// Result of executing check analysis without printing.
pub struct CheckResult {
    pub results: AnalysisResults,
    pub config: ResolvedConfig,
    pub elapsed: Duration,
    pub fail_on_issues: bool,
}

/// Run analysis, filtering, and baseline handling. Returns results without printing.
pub fn execute_check(opts: &CheckOptions<'_>) -> Result<CheckResult, ExitCode> {
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

    // Workspace filter resolution
    let ws_root = if let Some(ws_name) = opts.workspace {
        Some(filtering::resolve_workspace_filter(
            opts.root,
            ws_name,
            &opts.output,
        )?)
    } else {
        None
    };

    // Changed-files resolution
    let changed_files: Option<rustc_hash::FxHashSet<std::path::PathBuf>> = opts
        .changed_since
        .and_then(|git_ref| filtering::get_changed_files(opts.root, git_ref));

    // Core analysis
    let use_trace = opts.trace_opts.any_active();
    let (mut results, trace_graph, trace_timings) = if use_trace {
        match fallow_core::analyze_with_trace(&config) {
            Ok(trace_output) => (
                trace_output.results,
                trace_output.graph,
                trace_output.timings,
            ),
            Err(e) => {
                return Err(emit_error(&format!("Analysis error: {e}"), 2, &opts.output));
            }
        }
    } else {
        match fallow_core::analyze(&config) {
            Ok(r) => (r, None, None),
            Err(e) => {
                return Err(emit_error(&format!("Analysis error: {e}"), 2, &opts.output));
            }
        }
    };
    let elapsed = start.elapsed();

    // Performance output
    if let Some(ref timings) = trace_timings
        && opts.trace_opts.performance
    {
        report::print_performance(timings, &config.output);
    }

    // Trace early-return
    if let Some(ref graph) = trace_graph
        && let Some(code) =
            output::handle_trace_output(graph, opts.trace_opts, &config.root, &config.output)
    {
        return Err(code);
    }

    // Workspace scoping
    if let Some(ref ws_root) = ws_root {
        filtering::filter_to_workspace(&mut results, ws_root);
    }

    // Changed-file filtering
    if let Some(ref changed) = changed_files {
        filtering::filter_changed_files(&mut results, changed);
    }

    // Rules application
    rules::apply_rules(&mut results, &config);

    // CLI issue-type filters
    opts.filters.apply(&mut results);

    // Baseline handling
    if let Some(exit) = handle_baseline(&mut results, opts.save_baseline, opts.baseline, opts.quiet)
    {
        return Err(exit);
    }

    // SARIF file write
    if let Some(sarif_path) = opts.sarif_file {
        output::write_sarif_file(&results, &config, sarif_path, opts.quiet);
    }

    Ok(CheckResult {
        results,
        config,
        elapsed,
        fail_on_issues: opts.fail_on_issues,
    })
}

/// Print check results and return appropriate exit code.
pub fn print_check_result(result: &CheckResult, quiet: bool, explain: bool) -> ExitCode {
    let effective_rules = if result.fail_on_issues {
        let mut r = result.config.rules.clone();
        rules::promote_warns_to_errors(&mut r);
        r
    } else {
        result.config.rules.clone()
    };

    let ctx = report::ReportContext {
        root: &result.config.root,
        rules: &result.config.rules,
        elapsed: result.elapsed,
        quiet,
        explain,
    };
    let report_code = report::print_results(&result.results, &ctx, &result.config.output);
    if report_code != ExitCode::SUCCESS {
        return report_code;
    }

    if rules::has_error_severity_issues(&result.results, &effective_rules, Some(&result.config)) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

pub fn run_check(opts: &CheckOptions<'_>) -> ExitCode {
    let result = match execute_check(opts) {
        Ok(r) => r,
        Err(code) => return code,
    };

    let exit = print_check_result(&result, opts.quiet, opts.explain);

    // Cross-reference: run duplication analysis on the full results
    // (the combined command handles this separately)
    if opts.include_dupes && result.config.duplicates.enabled {
        output::run_cross_reference(&result.config, &result.results, opts.quiet);
    }

    exit
}

// ── Baseline helpers ────────────────────────────────────────────

/// Save baseline and/or compare against an existing baseline.
///
/// Returns `Some(ExitCode)` on fatal errors (serialization/IO failure),
/// `None` on success.
fn handle_baseline(
    results: &mut fallow_core::results::AnalysisResults,
    save_path: Option<&std::path::Path>,
    load_path: Option<&std::path::Path>,
    quiet: bool,
) -> Option<ExitCode> {
    // Save baseline if requested
    if let Some(baseline_path) = save_path {
        let baseline_data = BaselineData::from_results(results);
        match serde_json::to_string_pretty(&baseline_data) {
            Ok(json) => {
                if let Err(e) = std::fs::write(baseline_path, json) {
                    eprintln!("Error: failed to save baseline: {e}");
                    return Some(ExitCode::from(2));
                }
                if !quiet {
                    eprintln!("Baseline saved to {}", baseline_path.display());
                }
            }
            Err(e) => {
                eprintln!("Error: failed to serialize baseline: {e}");
                return Some(ExitCode::from(2));
            }
        }
    }

    // Compare against baseline if provided
    if let Some(baseline_path) = load_path {
        match std::fs::read_to_string(baseline_path) {
            Ok(content) => match serde_json::from_str::<BaselineData>(&content) {
                Ok(baseline_data) => {
                    *results = filter_new_issues(std::mem::take(results), &baseline_data);
                    if !quiet {
                        eprintln!("Comparing against baseline: {}", baseline_path.display());
                    }
                }
                Err(e) => {
                    eprintln!("Error: failed to parse baseline: {e}");
                    return Some(ExitCode::from(2));
                }
            },
            Err(e) => {
                eprintln!("Error: failed to read baseline: {e}");
                return Some(ExitCode::from(2));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use fallow_core::extract::MemberKind;
    use fallow_core::results::*;
    use std::path::PathBuf;

    fn no_filters() -> IssueFilters {
        IssueFilters {
            unused_files: false,
            unused_exports: false,
            unused_deps: false,
            unused_types: false,
            unused_enum_members: false,
            unused_class_members: false,
            unresolved_imports: false,
            unlisted_deps: false,
            duplicate_exports: false,
            circular_deps: false,
        }
    }

    fn make_results() -> AnalysisResults {
        let mut r = AnalysisResults::default();
        r.unused_files.push(UnusedFile {
            path: PathBuf::from("/project/src/a.ts"),
        });
        r.unused_exports.push(UnusedExport {
            path: PathBuf::from("/project/src/b.ts"),
            export_name: "foo".into(),
            is_type_only: false,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        r.unused_types.push(UnusedExport {
            path: PathBuf::from("/project/src/c.ts"),
            export_name: "MyType".into(),
            is_type_only: true,
            line: 5,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        r.unused_dependencies.push(UnusedDependency {
            package_name: "lodash".into(),
            location: DependencyLocation::Dependencies,
            path: PathBuf::from("/project/package.json"),
            line: 5,
        });
        r.unused_dev_dependencies.push(UnusedDependency {
            package_name: "jest".into(),
            location: DependencyLocation::DevDependencies,
            path: PathBuf::from("/project/package.json"),
            line: 5,
        });
        r.unused_enum_members.push(UnusedMember {
            path: PathBuf::from("/project/src/d.ts"),
            parent_name: "Status".into(),
            member_name: "Pending".into(),
            kind: MemberKind::EnumMember,
            line: 3,
            col: 0,
        });
        r.unused_class_members.push(UnusedMember {
            path: PathBuf::from("/project/src/e.ts"),
            parent_name: "Service".into(),
            member_name: "helper".into(),
            kind: MemberKind::ClassMethod,
            line: 10,
            col: 0,
        });
        r.unresolved_imports.push(UnresolvedImport {
            path: PathBuf::from("/project/src/f.ts"),
            specifier: "./missing".into(),
            line: 1,
            col: 0,
            specifier_col: 0,
        });
        r.unlisted_dependencies.push(UnlistedDependency {
            package_name: "chalk".into(),
            imported_from: vec![ImportSite {
                path: PathBuf::from("/project/src/g.ts"),
                line: 1,
                col: 0,
            }],
        });
        r.duplicate_exports.push(DuplicateExport {
            export_name: "helper".into(),
            locations: vec![
                DuplicateLocation {
                    path: PathBuf::from("/project/src/h.ts"),
                    line: 15,
                    col: 0,
                },
                DuplicateLocation {
                    path: PathBuf::from("/project/src/i.ts"),
                    line: 30,
                    col: 0,
                },
            ],
        });
        r
    }

    // ── IssueFilters::any_active ─────────────────────────────────

    #[test]
    fn no_filters_means_none_active() {
        assert!(!no_filters().any_active());
    }

    #[test]
    fn single_filter_is_active() {
        let mut f = no_filters();
        f.unused_files = true;
        assert!(f.any_active());
    }

    #[test]
    fn each_filter_flag_registers_as_active() {
        let flags: Vec<fn(&mut IssueFilters)> = vec![
            |f| f.unused_files = true,
            |f| f.unused_exports = true,
            |f| f.unused_deps = true,
            |f| f.unused_types = true,
            |f| f.unused_enum_members = true,
            |f| f.unused_class_members = true,
            |f| f.unresolved_imports = true,
            |f| f.unlisted_deps = true,
            |f| f.duplicate_exports = true,
            |f| f.circular_deps = true,
        ];
        for setter in flags {
            let mut f = no_filters();
            setter(&mut f);
            assert!(f.any_active());
        }
    }

    // ── IssueFilters::apply ──────────────────────────────────────

    #[test]
    fn apply_no_active_filters_preserves_all_results() {
        let mut results = make_results();
        let original_total = results.total_issues();
        no_filters().apply(&mut results);
        assert_eq!(results.total_issues(), original_total);
    }

    #[test]
    fn apply_unused_files_filter_keeps_only_unused_files() {
        let mut results = make_results();
        let mut f = no_filters();
        f.unused_files = true;
        f.apply(&mut results);

        assert_eq!(results.unused_files.len(), 1);
        assert!(results.unused_exports.is_empty());
        assert!(results.unused_types.is_empty());
        assert!(results.unused_dependencies.is_empty());
        assert!(results.unused_dev_dependencies.is_empty());
        assert!(results.unused_enum_members.is_empty());
        assert!(results.unused_class_members.is_empty());
        assert!(results.unresolved_imports.is_empty());
        assert!(results.unlisted_dependencies.is_empty());
        assert!(results.duplicate_exports.is_empty());
    }

    #[test]
    fn apply_unused_deps_filter_keeps_both_dep_types() {
        let mut results = make_results();
        let mut f = no_filters();
        f.unused_deps = true;
        f.apply(&mut results);

        assert_eq!(results.unused_dependencies.len(), 1);
        assert_eq!(results.unused_dev_dependencies.len(), 1);
        assert!(results.unused_files.is_empty());
        assert!(results.unused_exports.is_empty());
    }

    #[test]
    fn apply_multiple_filters_keeps_selected_types() {
        let mut results = make_results();
        let mut f = no_filters();
        f.unused_files = true;
        f.unresolved_imports = true;
        f.apply(&mut results);

        assert_eq!(results.unused_files.len(), 1);
        assert_eq!(results.unresolved_imports.len(), 1);
        assert!(results.unused_exports.is_empty());
        assert!(results.unused_types.is_empty());
        assert!(results.duplicate_exports.is_empty());
    }

    #[test]
    fn apply_circular_deps_filter_keeps_only_circular_deps() {
        let mut results = make_results();
        // Add circular dependency to results
        results
            .circular_dependencies
            .push(fallow_core::results::CircularDependency {
                files: vec![
                    PathBuf::from("/project/src/a.ts"),
                    PathBuf::from("/project/src/b.ts"),
                ],
                length: 2,
                line: 1,
                col: 0,
            });
        let mut f = no_filters();
        f.circular_deps = true;
        f.apply(&mut results);

        assert_eq!(results.circular_dependencies.len(), 1);
        assert!(results.unused_files.is_empty());
        assert!(results.unused_exports.is_empty());
        assert!(results.unused_dependencies.is_empty());
    }

    // ── TraceOptions::any_active ─────────────────────────────────

    #[test]
    fn no_trace_options_means_none_active() {
        let t = TraceOptions {
            trace_export: None,
            trace_file: None,
            trace_dependency: None,
            performance: false,
        };
        assert!(!t.any_active());
    }

    #[test]
    fn trace_export_is_active() {
        let t = TraceOptions {
            trace_export: Some("src/foo.ts:bar".into()),
            trace_file: None,
            trace_dependency: None,
            performance: false,
        };
        assert!(t.any_active());
    }

    #[test]
    fn trace_file_is_active() {
        let t = TraceOptions {
            trace_export: None,
            trace_file: Some("src/foo.ts".into()),
            trace_dependency: None,
            performance: false,
        };
        assert!(t.any_active());
    }

    #[test]
    fn trace_dependency_is_active() {
        let t = TraceOptions {
            trace_export: None,
            trace_file: None,
            trace_dependency: Some("lodash".into()),
            performance: false,
        };
        assert!(t.any_active());
    }

    #[test]
    fn performance_flag_is_active() {
        let t = TraceOptions {
            trace_export: None,
            trace_file: None,
            trace_dependency: None,
            performance: true,
        };
        assert!(t.any_active());
    }
}
