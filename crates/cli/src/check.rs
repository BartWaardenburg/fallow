use std::process::ExitCode;
use std::time::Instant;

use fallow_config::{OutputFormat, RulesConfig, Severity, discover_workspaces};

use crate::baseline::{BaselineData, filter_new_issues};
use crate::report;
use crate::{emit_error, load_config};

// ── Issue type filters ──────────────────────────────────────────

#[allow(clippy::struct_excessive_bools)]
pub(crate) struct IssueFilters {
    pub(crate) unused_files: bool,
    pub(crate) unused_exports: bool,
    pub(crate) unused_deps: bool,
    pub(crate) unused_types: bool,
    pub(crate) unused_enum_members: bool,
    pub(crate) unused_class_members: bool,
    pub(crate) unresolved_imports: bool,
    pub(crate) unlisted_deps: bool,
    pub(crate) duplicate_exports: bool,
}

impl IssueFilters {
    pub(crate) fn any_active(&self) -> bool {
        self.unused_files
            || self.unused_exports
            || self.unused_deps
            || self.unused_types
            || self.unused_enum_members
            || self.unused_class_members
            || self.unresolved_imports
            || self.unlisted_deps
            || self.duplicate_exports
    }

    /// When any filter is active, clear issue types that were NOT requested.
    pub(crate) fn apply(&self, results: &mut fallow_core::results::AnalysisResults) {
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
    }
}

// ── Trace options ───────────────────────────────────────────────

pub(crate) struct TraceOptions {
    pub(crate) trace_export: Option<String>,
    pub(crate) trace_file: Option<String>,
    pub(crate) trace_dependency: Option<String>,
    pub(crate) performance: bool,
}

impl TraceOptions {
    pub(crate) fn any_active(&self) -> bool {
        self.trace_export.is_some()
            || self.trace_file.is_some()
            || self.trace_dependency.is_some()
            || self.performance
    }
}

// ── Rules helpers ────────────────────────────────────────────────

/// Remove issues whose severity is `Off` from the results.
fn apply_rules(results: &mut fallow_core::results::AnalysisResults, rules: &RulesConfig) {
    if rules.unused_files == Severity::Off {
        results.unused_files.clear();
    }
    if rules.unused_exports == Severity::Off {
        results.unused_exports.clear();
    }
    if rules.unused_types == Severity::Off {
        results.unused_types.clear();
    }
    if rules.unused_dependencies == Severity::Off {
        results.unused_dependencies.clear();
    }
    if rules.unused_dev_dependencies == Severity::Off {
        results.unused_dev_dependencies.clear();
    }
    if rules.unused_enum_members == Severity::Off {
        results.unused_enum_members.clear();
    }
    if rules.unused_class_members == Severity::Off {
        results.unused_class_members.clear();
    }
    if rules.unresolved_imports == Severity::Off {
        results.unresolved_imports.clear();
    }
    if rules.unlisted_dependencies == Severity::Off {
        results.unlisted_dependencies.clear();
    }
    if rules.duplicate_exports == Severity::Off {
        results.duplicate_exports.clear();
    }
}

/// Check whether any issue type with `Severity::Error` has remaining issues.
fn has_error_severity_issues(
    results: &fallow_core::results::AnalysisResults,
    rules: &RulesConfig,
) -> bool {
    (rules.unused_files == Severity::Error && !results.unused_files.is_empty())
        || (rules.unused_exports == Severity::Error && !results.unused_exports.is_empty())
        || (rules.unused_types == Severity::Error && !results.unused_types.is_empty())
        || (rules.unused_dependencies == Severity::Error && !results.unused_dependencies.is_empty())
        || (rules.unused_dev_dependencies == Severity::Error
            && !results.unused_dev_dependencies.is_empty())
        || (rules.unused_enum_members == Severity::Error && !results.unused_enum_members.is_empty())
        || (rules.unused_class_members == Severity::Error
            && !results.unused_class_members.is_empty())
        || (rules.unresolved_imports == Severity::Error && !results.unresolved_imports.is_empty())
        || (rules.unlisted_dependencies == Severity::Error
            && !results.unlisted_dependencies.is_empty())
        || (rules.duplicate_exports == Severity::Error && !results.duplicate_exports.is_empty())
}

// ── Workspace filtering ──────────────────────────────────────────

/// Scope results to a single workspace package.
///
/// The full cross-workspace graph is still built (so cross-package imports
/// are resolved), but only issues from files under `ws_root` are reported.
fn filter_to_workspace(
    results: &mut fallow_core::results::AnalysisResults,
    ws_root: &std::path::Path,
) {
    // File-scoped issues: retain only those under the workspace root
    results.unused_files.retain(|f| f.path.starts_with(ws_root));
    results
        .unused_exports
        .retain(|e| e.path.starts_with(ws_root));
    results.unused_types.retain(|e| e.path.starts_with(ws_root));
    results
        .unused_enum_members
        .retain(|m| m.path.starts_with(ws_root));
    results
        .unused_class_members
        .retain(|m| m.path.starts_with(ws_root));
    results
        .unresolved_imports
        .retain(|i| i.path.starts_with(ws_root));

    // Dependency issues: scope to workspace's own package.json
    let ws_pkg = ws_root.join("package.json");
    results.unused_dependencies.retain(|d| d.path == ws_pkg);
    results.unused_dev_dependencies.retain(|d| d.path == ws_pkg);
    results.type_only_dependencies.retain(|d| d.path == ws_pkg);

    // Unlisted deps: keep only if any importing file is in this workspace
    results
        .unlisted_dependencies
        .retain(|d| d.imported_from.iter().any(|p| p.starts_with(ws_root)));

    // Duplicate exports: filter locations to workspace, drop groups with < 2
    for dup in &mut results.duplicate_exports {
        dup.locations.retain(|p| p.starts_with(ws_root));
    }
    results.duplicate_exports.retain(|d| d.locations.len() >= 2);
}

/// Resolve `--workspace <name>` to a workspace root path, or exit with an error.
fn resolve_workspace_filter(
    root: &std::path::Path,
    workspace_name: &str,
    output: &OutputFormat,
) -> Result<std::path::PathBuf, ExitCode> {
    let workspaces = discover_workspaces(root);
    if workspaces.is_empty() {
        let msg = format!(
            "--workspace '{workspace_name}' specified but no workspaces found. \
             Ensure root package.json has a \"workspaces\" field or pnpm-workspace.yaml exists."
        );
        return Err(emit_error(&msg, 2, output));
    }

    match workspaces.iter().find(|ws| ws.name == workspace_name) {
        Some(ws) => Ok(ws.root.clone()),
        None => {
            let names: Vec<&str> = workspaces.iter().map(|ws| ws.name.as_str()).collect();
            let msg = format!(
                "workspace '{workspace_name}' not found. Available workspaces: {}",
                names.join(", ")
            );
            Err(emit_error(&msg, 2, output))
        }
    }
}

// ── Changed files ────────────────────────────────────────────────

/// Get files changed since a git ref.
fn get_changed_files(
    root: &std::path::Path,
    git_ref: &str,
) -> Option<rustc_hash::FxHashSet<std::path::PathBuf>> {
    let output = match std::process::Command::new("git")
        .args(["diff", "--name-only", &format!("{git_ref}...HEAD")])
        .current_dir(root)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            // git binary not found or not executable — could be a non-git project
            eprintln!("Warning: --changed-since ignored: failed to run git: {e}");
            return None;
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not a git repository") {
            // Not a git repo — silently skip the filter (could be OK)
            eprintln!("Warning: --changed-since ignored: not a git repository");
        } else {
            // Likely a bad ref — warn the user
            eprintln!(
                "Warning: --changed-since failed for ref '{}': {}",
                git_ref,
                stderr.trim()
            );
        }
        return None;
    }

    let files: rustc_hash::FxHashSet<std::path::PathBuf> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| root.join(line))
        .collect();

    Some(files)
}

// ── Check command ────────────────────────────────────────────────

#[allow(clippy::struct_excessive_bools)]
pub(crate) struct CheckOptions<'a> {
    pub(crate) root: &'a std::path::Path,
    pub(crate) config_path: &'a Option<std::path::PathBuf>,
    pub(crate) output: OutputFormat,
    pub(crate) no_cache: bool,
    pub(crate) threads: usize,
    pub(crate) quiet: bool,
    pub(crate) fail_on_issues: bool,
    pub(crate) filters: &'a IssueFilters,
    pub(crate) changed_since: Option<&'a str>,
    pub(crate) baseline: Option<&'a std::path::Path>,
    pub(crate) save_baseline: Option<&'a std::path::Path>,
    pub(crate) sarif_file: Option<&'a std::path::Path>,
    pub(crate) production: bool,
    pub(crate) workspace: Option<&'a str>,
    pub(crate) include_dupes: bool,
    pub(crate) trace_opts: &'a TraceOptions,
}

pub(crate) fn run_check(opts: &CheckOptions<'_>) -> ExitCode {
    let start = Instant::now();

    let config = match load_config(
        opts.root,
        opts.config_path,
        opts.output.clone(),
        opts.no_cache,
        opts.threads,
        opts.production,
    ) {
        Ok(c) => c,
        Err(code) => return code,
    };

    // Validate --workspace early (before analysis) to fail fast
    let ws_root = if let Some(ws_name) = opts.workspace {
        match resolve_workspace_filter(opts.root, ws_name, &opts.output) {
            Ok(root) => Some(root),
            Err(code) => return code,
        }
    } else {
        None
    };

    // Get changed files if --changed-since is set (already validated)
    let changed_files: Option<rustc_hash::FxHashSet<std::path::PathBuf>> = opts
        .changed_since
        .and_then(|git_ref| get_changed_files(opts.root, git_ref));

    // Use analyze_with_trace when any trace option is active (retains graph + timings)
    let use_trace = opts.trace_opts.any_active();
    let (mut results, trace_graph, trace_timings) = if use_trace {
        match fallow_core::analyze_with_trace(&config) {
            Ok(trace_output) => (
                trace_output.results,
                trace_output.graph,
                trace_output.timings,
            ),
            Err(e) => {
                return emit_error(&format!("Analysis error: {e}"), 2, &opts.output);
            }
        }
    } else {
        match fallow_core::analyze(&config) {
            Ok(r) => (r, None, None),
            Err(e) => {
                return emit_error(&format!("Analysis error: {e}"), 2, &opts.output);
            }
        }
    };
    let elapsed = start.elapsed();

    // Print performance timings first — they should always appear, even combined with --trace*
    if let Some(ref timings) = trace_timings
        && opts.trace_opts.performance
    {
        report::print_performance(timings, &config.output);
    }

    // Handle trace output (trace is a diagnostic mode — early return after output)
    if let Some(ref graph) = trace_graph {
        if let Some(ref trace_spec) = opts.trace_opts.trace_export {
            let Some((file_path, export_name)) = trace_spec.rsplit_once(':') else {
                return emit_error(
                    "--trace requires FILE:EXPORT_NAME format (e.g., src/utils.ts:foo)",
                    2,
                    &opts.output,
                );
            };
            match fallow_core::trace::trace_export(graph, &config.root, file_path, export_name) {
                Some(trace) => {
                    report::print_export_trace(&trace, &config.output);
                    return ExitCode::SUCCESS;
                }
                None => {
                    return emit_error(
                        &format!("export '{export_name}' not found in '{file_path}'"),
                        2,
                        &opts.output,
                    );
                }
            }
        }

        if let Some(ref file_path) = opts.trace_opts.trace_file {
            match fallow_core::trace::trace_file(graph, &config.root, file_path) {
                Some(trace) => {
                    report::print_file_trace(&trace, &config.output);
                    return ExitCode::SUCCESS;
                }
                None => {
                    return emit_error(
                        &format!("file '{file_path}' not found in module graph"),
                        2,
                        &opts.output,
                    );
                }
            }
        }

        if let Some(ref pkg_name) = opts.trace_opts.trace_dependency {
            let trace = fallow_core::trace::trace_dependency(graph, &config.root, pkg_name);
            report::print_dependency_trace(&trace, &config.output);
            return ExitCode::SUCCESS;
        }
    }

    // Scope to workspace package if requested (full graph is built, only output is filtered)
    if let Some(ref ws_root) = ws_root {
        filter_to_workspace(&mut results, ws_root);
    }

    // Filter to only changed files if requested
    if let Some(changed) = &changed_files {
        results.unused_files.retain(|f| changed.contains(&f.path));
        results.unused_exports.retain(|e| changed.contains(&e.path));
        results.unused_types.retain(|e| changed.contains(&e.path));
        results
            .unused_enum_members
            .retain(|m| changed.contains(&m.path));
        results
            .unused_class_members
            .retain(|m| changed.contains(&m.path));
        results
            .unresolved_imports
            .retain(|i| changed.contains(&i.path));
    }

    // Apply rules: remove issues with Severity::Off
    apply_rules(&mut results, &config.rules);

    // Snapshot results for cross-reference AFTER rules/workspace/changed-files filtering
    // but BEFORE CLI issue-type filters (--unused-files etc.), so combined findings
    // respect the user's severity config but aren't limited by per-invocation filters.
    let unfiltered_results = if opts.include_dupes && config.duplicates.enabled {
        Some(results.clone())
    } else {
        None
    };

    // Apply issue type filters (CLI --unused-files etc.)
    opts.filters.apply(&mut results);

    // Save baseline if requested
    if let Some(baseline_path) = opts.save_baseline {
        let baseline_data = BaselineData::from_results(&results);
        if let Ok(json) = serde_json::to_string_pretty(&baseline_data) {
            if let Err(e) = std::fs::write(baseline_path, json) {
                eprintln!("Failed to save baseline: {e}");
            } else if !opts.quiet {
                eprintln!("Baseline saved to {}", baseline_path.display());
            }
        }
    }

    // Compare against baseline if provided
    if let Some(baseline_path) = opts.baseline
        && let Ok(content) = std::fs::read_to_string(baseline_path)
        && let Ok(baseline_data) = serde_json::from_str::<BaselineData>(&content)
    {
        results = filter_new_issues(results, &baseline_data);
        if !opts.quiet {
            eprintln!("Comparing against baseline: {}", baseline_path.display());
        }
    }

    // Write SARIF to file if requested (independent of --format)
    if let Some(sarif_path) = opts.sarif_file {
        let sarif = report::build_sarif(&results, &config.root, &config.rules);
        match serde_json::to_string_pretty(&sarif) {
            Ok(json) => {
                // Ensure parent directories exist
                if let Some(parent) = sarif_path.parent()
                    && !parent.as_os_str().is_empty()
                    && let Err(e) = std::fs::create_dir_all(parent)
                {
                    eprintln!(
                        "Warning: failed to create directory for SARIF file '{}': {e}",
                        sarif_path.display()
                    );
                }
                if let Err(e) = std::fs::write(sarif_path, json) {
                    eprintln!(
                        "Warning: failed to write SARIF file '{}': {e}",
                        sarif_path.display()
                    );
                } else if !opts.quiet {
                    eprintln!("SARIF output written to {}", sarif_path.display());
                }
            }
            Err(e) => {
                eprintln!("Warning: failed to serialize SARIF output: {e}");
            }
        }
    }

    // When --fail-on-issues is set, promote all Warn to Error for this run
    let effective_rules = if opts.fail_on_issues {
        let mut r = config.rules.clone();
        if r.unused_files == Severity::Warn {
            r.unused_files = Severity::Error;
        }
        if r.unused_exports == Severity::Warn {
            r.unused_exports = Severity::Error;
        }
        if r.unused_types == Severity::Warn {
            r.unused_types = Severity::Error;
        }
        if r.unused_dependencies == Severity::Warn {
            r.unused_dependencies = Severity::Error;
        }
        if r.unused_dev_dependencies == Severity::Warn {
            r.unused_dev_dependencies = Severity::Error;
        }
        if r.unused_enum_members == Severity::Warn {
            r.unused_enum_members = Severity::Error;
        }
        if r.unused_class_members == Severity::Warn {
            r.unused_class_members = Severity::Error;
        }
        if r.unresolved_imports == Severity::Warn {
            r.unresolved_imports = Severity::Error;
        }
        if r.unlisted_dependencies == Severity::Warn {
            r.unlisted_dependencies = Severity::Error;
        }
        if r.duplicate_exports == Severity::Warn {
            r.duplicate_exports = Severity::Error;
        }
        r
    } else {
        config.rules.clone()
    };

    let report_code = report::print_results(&results, &config, elapsed, opts.quiet);
    if report_code != ExitCode::SUCCESS {
        return report_code;
    }

    // Cross-reference with duplication analysis if --include-dupes is set.
    // Uses unfiltered results so combined findings reflect all dead code,
    // not just the CLI-filtered subset.
    if let Some(ref unfiltered) = unfiltered_results {
        let files = fallow_core::discover::discover_files(&config);
        let dupe_report =
            fallow_core::duplicates::find_duplicates(&config.root, &files, &config.duplicates);
        let cross_ref = fallow_core::cross_reference::cross_reference(&dupe_report, unfiltered);

        if cross_ref.has_findings() {
            report::print_cross_reference_findings(
                &cross_ref,
                &config.root,
                opts.quiet,
                &config.output,
            );
        }
    }

    if has_error_severity_issues(&results, &effective_rules) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fallow_core::extract::MemberKind;
    use fallow_core::results::*;
    use std::path::PathBuf;

    // ── Helper: build populated AnalysisResults ──────────────────

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
        });
        r.unused_dev_dependencies.push(UnusedDependency {
            package_name: "jest".into(),
            location: DependencyLocation::DevDependencies,
            path: PathBuf::from("/project/package.json"),
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
        });
        r.unlisted_dependencies.push(UnlistedDependency {
            package_name: "chalk".into(),
            imported_from: vec![PathBuf::from("/project/src/g.ts")],
        });
        r.duplicate_exports.push(DuplicateExport {
            export_name: "helper".into(),
            locations: vec![
                PathBuf::from("/project/src/h.ts"),
                PathBuf::from("/project/src/i.ts"),
            ],
        });
        r
    }

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
        }
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

    // ── apply_rules ──────────────────────────────────────────────

    #[test]
    fn apply_rules_default_error_preserves_all() {
        let mut results = make_results();
        let rules = RulesConfig::default();
        let original_total = results.total_issues();
        apply_rules(&mut results, &rules);
        assert_eq!(results.total_issues(), original_total);
    }

    #[test]
    fn apply_rules_off_clears_that_issue_type() {
        let mut results = make_results();
        let mut rules = RulesConfig::default();
        rules.unused_files = Severity::Off;
        apply_rules(&mut results, &rules);
        assert!(results.unused_files.is_empty());
        // Other types are preserved
        assert!(!results.unused_exports.is_empty());
    }

    #[test]
    fn apply_rules_warn_preserves_issues() {
        let mut results = make_results();
        let mut rules = RulesConfig::default();
        rules.unused_exports = Severity::Warn;
        apply_rules(&mut results, &rules);
        assert_eq!(results.unused_exports.len(), 1);
    }

    #[test]
    fn apply_rules_all_off_clears_everything() {
        let mut results = make_results();
        let rules = RulesConfig {
            unused_files: Severity::Off,
            unused_exports: Severity::Off,
            unused_types: Severity::Off,
            unused_dependencies: Severity::Off,
            unused_dev_dependencies: Severity::Off,
            unused_enum_members: Severity::Off,
            unused_class_members: Severity::Off,
            unresolved_imports: Severity::Off,
            unlisted_dependencies: Severity::Off,
            duplicate_exports: Severity::Off,
        };
        apply_rules(&mut results, &rules);
        assert_eq!(results.total_issues(), 0);
    }

    #[test]
    fn apply_rules_off_each_type_individually() {
        // Verify every rule field maps to its corresponding results field
        let field_setters: Vec<(fn(&mut RulesConfig), fn(&AnalysisResults) -> bool)> = vec![
            (
                |r| r.unused_files = Severity::Off,
                |res| res.unused_files.is_empty(),
            ),
            (
                |r| r.unused_exports = Severity::Off,
                |res| res.unused_exports.is_empty(),
            ),
            (
                |r| r.unused_types = Severity::Off,
                |res| res.unused_types.is_empty(),
            ),
            (
                |r| r.unused_dependencies = Severity::Off,
                |res| res.unused_dependencies.is_empty(),
            ),
            (
                |r| r.unused_dev_dependencies = Severity::Off,
                |res| res.unused_dev_dependencies.is_empty(),
            ),
            (
                |r| r.unused_enum_members = Severity::Off,
                |res| res.unused_enum_members.is_empty(),
            ),
            (
                |r| r.unused_class_members = Severity::Off,
                |res| res.unused_class_members.is_empty(),
            ),
            (
                |r| r.unresolved_imports = Severity::Off,
                |res| res.unresolved_imports.is_empty(),
            ),
            (
                |r| r.unlisted_dependencies = Severity::Off,
                |res| res.unlisted_dependencies.is_empty(),
            ),
            (
                |r| r.duplicate_exports = Severity::Off,
                |res| res.duplicate_exports.is_empty(),
            ),
        ];

        for (set_off, check_empty) in field_setters {
            let mut results = make_results();
            let mut rules = RulesConfig::default();
            set_off(&mut rules);
            apply_rules(&mut results, &rules);
            assert!(
                check_empty(&results),
                "Setting a rule to Off should clear the corresponding results"
            );
        }
    }

    // ── has_error_severity_issues ────────────────────────────────

    #[test]
    fn empty_results_no_error_issues() {
        let results = AnalysisResults::default();
        let rules = RulesConfig::default();
        assert!(!has_error_severity_issues(&results, &rules));
    }

    #[test]
    fn error_severity_with_issues_returns_true() {
        let results = make_results();
        let rules = RulesConfig::default(); // all Error
        assert!(has_error_severity_issues(&results, &rules));
    }

    #[test]
    fn warn_severity_with_issues_returns_false() {
        let results = make_results();
        let rules = RulesConfig {
            unused_files: Severity::Warn,
            unused_exports: Severity::Warn,
            unused_types: Severity::Warn,
            unused_dependencies: Severity::Warn,
            unused_dev_dependencies: Severity::Warn,
            unused_enum_members: Severity::Warn,
            unused_class_members: Severity::Warn,
            unresolved_imports: Severity::Warn,
            unlisted_dependencies: Severity::Warn,
            duplicate_exports: Severity::Warn,
        };
        assert!(!has_error_severity_issues(&results, &rules));
    }

    #[test]
    fn mixed_severity_returns_true_for_error_with_issues() {
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: PathBuf::from("/project/src/a.ts"),
        });
        let mut rules = RulesConfig {
            unused_files: Severity::Warn,
            unused_exports: Severity::Warn,
            unused_types: Severity::Warn,
            unused_dependencies: Severity::Warn,
            unused_dev_dependencies: Severity::Warn,
            unused_enum_members: Severity::Warn,
            unused_class_members: Severity::Warn,
            unresolved_imports: Severity::Warn,
            unlisted_dependencies: Severity::Warn,
            duplicate_exports: Severity::Warn,
        };
        // Only unused_files present, but set to Warn — should not trigger
        assert!(!has_error_severity_issues(&results, &rules));

        // Promote unused_files to Error — should now trigger
        rules.unused_files = Severity::Error;
        assert!(has_error_severity_issues(&results, &rules));
    }

    #[test]
    fn off_severity_with_issues_returns_false() {
        let mut results = AnalysisResults::default();
        results.unresolved_imports.push(UnresolvedImport {
            path: PathBuf::from("/project/src/a.ts"),
            specifier: "./missing".into(),
            line: 1,
            col: 0,
        });
        let mut rules = RulesConfig::default();
        rules.unresolved_imports = Severity::Off;
        // Other fields are default (Error) but have no issues
        assert!(!has_error_severity_issues(&results, &rules));
    }

    // ── filter_to_workspace ──────────────────────────────────────

    #[test]
    fn filter_to_workspace_keeps_files_under_ws_root() {
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: PathBuf::from("/project/packages/ui/src/button.ts"),
        });
        results.unused_files.push(UnusedFile {
            path: PathBuf::from("/project/packages/api/src/handler.ts"),
        });

        let ws_root = PathBuf::from("/project/packages/ui");
        filter_to_workspace(&mut results, &ws_root);

        assert_eq!(results.unused_files.len(), 1);
        assert_eq!(
            results.unused_files[0].path,
            PathBuf::from("/project/packages/ui/src/button.ts")
        );
    }

    #[test]
    fn filter_to_workspace_scopes_dependencies_to_ws_package_json() {
        let mut results = AnalysisResults::default();
        results.unused_dependencies.push(UnusedDependency {
            package_name: "lodash".into(),
            location: DependencyLocation::Dependencies,
            path: PathBuf::from("/project/package.json"),
        });
        results.unused_dependencies.push(UnusedDependency {
            package_name: "react".into(),
            location: DependencyLocation::Dependencies,
            path: PathBuf::from("/project/packages/ui/package.json"),
        });
        results.unused_dev_dependencies.push(UnusedDependency {
            package_name: "vitest".into(),
            location: DependencyLocation::DevDependencies,
            path: PathBuf::from("/project/packages/ui/package.json"),
        });

        let ws_root = PathBuf::from("/project/packages/ui");
        filter_to_workspace(&mut results, &ws_root);

        assert_eq!(results.unused_dependencies.len(), 1);
        assert_eq!(results.unused_dependencies[0].package_name, "react");
        assert_eq!(results.unused_dev_dependencies.len(), 1);
        assert_eq!(results.unused_dev_dependencies[0].package_name, "vitest");
    }

    #[test]
    fn filter_to_workspace_scopes_unlisted_deps_by_importer() {
        let mut results = AnalysisResults::default();
        results.unlisted_dependencies.push(UnlistedDependency {
            package_name: "chalk".into(),
            imported_from: vec![PathBuf::from("/project/packages/ui/src/a.ts")],
        });
        results.unlisted_dependencies.push(UnlistedDependency {
            package_name: "debug".into(),
            imported_from: vec![PathBuf::from("/project/packages/api/src/b.ts")],
        });

        let ws_root = PathBuf::from("/project/packages/ui");
        filter_to_workspace(&mut results, &ws_root);

        assert_eq!(results.unlisted_dependencies.len(), 1);
        assert_eq!(results.unlisted_dependencies[0].package_name, "chalk");
    }

    #[test]
    fn filter_to_workspace_drops_duplicate_exports_below_two_locations() {
        let mut results = AnalysisResults::default();
        results.duplicate_exports.push(DuplicateExport {
            export_name: "helper".into(),
            locations: vec![
                PathBuf::from("/project/packages/ui/src/a.ts"),
                PathBuf::from("/project/packages/api/src/b.ts"),
            ],
        });
        results.duplicate_exports.push(DuplicateExport {
            export_name: "utils".into(),
            locations: vec![
                PathBuf::from("/project/packages/ui/src/c.ts"),
                PathBuf::from("/project/packages/ui/src/d.ts"),
            ],
        });

        let ws_root = PathBuf::from("/project/packages/ui");
        filter_to_workspace(&mut results, &ws_root);

        // "helper" had only 1 location in workspace — dropped
        // "utils" had 2 locations in workspace — kept
        assert_eq!(results.duplicate_exports.len(), 1);
        assert_eq!(results.duplicate_exports[0].export_name, "utils");
    }

    #[test]
    fn filter_to_workspace_scopes_exports_and_types() {
        let mut results = AnalysisResults::default();
        results.unused_exports.push(UnusedExport {
            path: PathBuf::from("/project/packages/ui/src/a.ts"),
            export_name: "A".into(),
            is_type_only: false,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        results.unused_exports.push(UnusedExport {
            path: PathBuf::from("/project/packages/api/src/b.ts"),
            export_name: "B".into(),
            is_type_only: false,
            line: 2,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        results.unused_types.push(UnusedExport {
            path: PathBuf::from("/project/packages/ui/src/types.ts"),
            export_name: "T".into(),
            is_type_only: true,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });

        let ws_root = PathBuf::from("/project/packages/ui");
        filter_to_workspace(&mut results, &ws_root);

        assert_eq!(results.unused_exports.len(), 1);
        assert_eq!(results.unused_exports[0].export_name, "A");
        assert_eq!(results.unused_types.len(), 1);
        assert_eq!(results.unused_types[0].export_name, "T");
    }

    #[test]
    fn filter_to_workspace_scopes_type_only_dependencies() {
        let mut results = AnalysisResults::default();
        results.type_only_dependencies.push(TypeOnlyDependency {
            package_name: "zod".into(),
            path: PathBuf::from("/project/packages/ui/package.json"),
        });
        results.type_only_dependencies.push(TypeOnlyDependency {
            package_name: "yup".into(),
            path: PathBuf::from("/project/package.json"),
        });

        let ws_root = PathBuf::from("/project/packages/ui");
        filter_to_workspace(&mut results, &ws_root);

        assert_eq!(results.type_only_dependencies.len(), 1);
        assert_eq!(results.type_only_dependencies[0].package_name, "zod");
    }

    #[test]
    fn filter_to_workspace_scopes_enum_and_class_members() {
        let mut results = AnalysisResults::default();
        results.unused_enum_members.push(UnusedMember {
            path: PathBuf::from("/project/packages/ui/src/enums.ts"),
            parent_name: "Color".into(),
            member_name: "Red".into(),
            kind: MemberKind::EnumMember,
            line: 2,
            col: 0,
        });
        results.unused_enum_members.push(UnusedMember {
            path: PathBuf::from("/project/packages/api/src/enums.ts"),
            parent_name: "Status".into(),
            member_name: "Active".into(),
            kind: MemberKind::EnumMember,
            line: 3,
            col: 0,
        });
        results.unused_class_members.push(UnusedMember {
            path: PathBuf::from("/project/packages/ui/src/service.ts"),
            parent_name: "Svc".into(),
            member_name: "init".into(),
            kind: MemberKind::ClassMethod,
            line: 5,
            col: 0,
        });

        let ws_root = PathBuf::from("/project/packages/ui");
        filter_to_workspace(&mut results, &ws_root);

        assert_eq!(results.unused_enum_members.len(), 1);
        assert_eq!(results.unused_enum_members[0].member_name, "Red");
        assert_eq!(results.unused_class_members.len(), 1);
        assert_eq!(results.unused_class_members[0].member_name, "init");
    }
}
