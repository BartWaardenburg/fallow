use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use clap::{CommandFactory, Parser, Subcommand};
use fallow_config::{FallowConfig, OutputFormat, RulesConfig, Severity, discover_workspaces};

mod baseline;
mod fix;
mod report;

use baseline::{BaselineData, DuplicationBaselineData, filter_new_clone_groups, filter_new_issues};

// ── CLI definition ───────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "fallow",
    about = "Find unused files, exports, and dependencies in JavaScript/TypeScript projects",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Project root directory
    #[arg(short, long, global = true)]
    root: Option<PathBuf>,

    /// Path to config file (fallow.jsonc, fallow.json, or fallow.toml)
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Output format (alias: --output)
    #[arg(
        short,
        long,
        visible_alias = "output",
        global = true,
        default_value = "human"
    )]
    format: Format,

    /// Suppress progress output
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Disable incremental caching
    #[arg(long, global = true)]
    no_cache: bool,

    /// Number of parser threads
    #[arg(long, global = true)]
    threads: Option<usize>,

    /// Only report issues in files changed since this git ref (e.g., main, HEAD~5)
    #[arg(long, global = true)]
    changed_since: Option<String>,

    /// Compare against a previously saved baseline file
    #[arg(long, global = true)]
    baseline: Option<PathBuf>,

    /// Save the current results as a baseline file
    #[arg(long, global = true)]
    save_baseline: Option<PathBuf>,

    /// Production mode: exclude test/story/dev files, only start/build scripts,
    /// report type-only dependencies
    #[arg(long, global = true)]
    production: bool,

    /// Scope output to a single workspace package (by package name).
    /// The full cross-workspace graph is still built, but only issues within
    /// the specified package are reported.
    #[arg(short, long, global = true)]
    workspace: Option<String>,
}

#[derive(Subcommand)]
enum Command {
    /// Run dead code analysis (default)
    Check {
        /// Exit with code 1 if issues are found
        #[arg(long)]
        fail_on_issues: bool,

        /// Write SARIF output to a file (in addition to the primary --format output)
        #[arg(long, value_name = "PATH")]
        sarif_file: Option<PathBuf>,

        /// Only report unused files
        #[arg(long)]
        unused_files: bool,

        /// Only report unused exports
        #[arg(long)]
        unused_exports: bool,

        /// Only report unused dependencies
        #[arg(long)]
        unused_deps: bool,

        /// Only report unused type exports
        #[arg(long)]
        unused_types: bool,

        /// Only report unused enum members
        #[arg(long)]
        unused_enum_members: bool,

        /// Only report unused class members
        #[arg(long)]
        unused_class_members: bool,

        /// Only report unresolved imports
        #[arg(long)]
        unresolved_imports: bool,

        /// Only report unlisted dependencies
        #[arg(long)]
        unlisted_deps: bool,

        /// Only report duplicate exports
        #[arg(long)]
        duplicate_exports: bool,

        /// Also run duplication analysis and cross-reference with dead code
        #[arg(long)]
        include_dupes: bool,
    },

    /// Watch for changes and re-run analysis
    Watch,

    /// Auto-fix issues (remove unused exports, dependencies)
    Fix {
        /// Dry run — show what would be changed without modifying files
        #[arg(long)]
        dry_run: bool,
    },

    /// Initialize a fallow.jsonc configuration file
    Init {
        /// Generate TOML instead of JSONC
        #[arg(long)]
        toml: bool,
    },

    /// Print the JSON Schema for fallow configuration files
    ConfigSchema,

    /// List discovered entry points and files
    List {
        /// Show entry points
        #[arg(long)]
        entry_points: bool,

        /// Show all discovered files
        #[arg(long)]
        files: bool,

        /// Show active plugins
        #[arg(long)]
        plugins: bool,
    },

    /// Find code duplication / clones across the project
    Dupes {
        /// Detection mode: strict, mild, weak, or semantic
        #[arg(long, default_value = "mild")]
        mode: DupesMode,

        /// Minimum token count for a clone
        #[arg(long, default_value = "50")]
        min_tokens: usize,

        /// Minimum line count for a clone
        #[arg(long, default_value = "5")]
        min_lines: usize,

        /// Fail if duplication exceeds this percentage (0 = no limit)
        #[arg(long, default_value = "0")]
        threshold: f64,

        /// Only report cross-directory duplicates
        #[arg(long)]
        skip_local: bool,

        /// Enable cross-language detection (strip TS type annotations for TS↔JS matching)
        #[arg(long)]
        cross_language: bool,
    },

    /// Dump the CLI interface as machine-readable JSON for agent introspection
    Schema,
}

#[derive(Clone, clap::ValueEnum)]
enum DupesMode {
    Strict,
    Mild,
    Weak,
    Semantic,
}

#[derive(Clone, clap::ValueEnum)]
enum Format {
    Human,
    Json,
    Sarif,
    Compact,
}

impl From<Format> for OutputFormat {
    fn from(f: Format) -> Self {
        match f {
            Format::Human => OutputFormat::Human,
            Format::Json => OutputFormat::Json,
            Format::Sarif => OutputFormat::Sarif,
            Format::Compact => OutputFormat::Compact,
        }
    }
}

// ── Issue type filters ──────────────────────────────────────────

struct IssueFilters {
    unused_files: bool,
    unused_exports: bool,
    unused_deps: bool,
    unused_types: bool,
    unused_enum_members: bool,
    unused_class_members: bool,
    unresolved_imports: bool,
    unlisted_deps: bool,
    duplicate_exports: bool,
}

impl IssueFilters {
    fn any_active(&self) -> bool {
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
    fn apply(&self, results: &mut fallow_core::results::AnalysisResults) {
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
) -> Result<std::path::PathBuf, ExitCode> {
    let workspaces = discover_workspaces(root);
    if workspaces.is_empty() {
        eprintln!(
            "Error: --workspace '{workspace_name}' specified but no workspaces found.\n\
             Ensure root package.json has a \"workspaces\" field or pnpm-workspace.yaml exists."
        );
        return Err(ExitCode::from(2));
    }

    match workspaces.iter().find(|ws| ws.name == workspace_name) {
        Some(ws) => Ok(ws.root.clone()),
        None => {
            let names: Vec<&str> = workspaces.iter().map(|ws| ws.name.as_str()).collect();
            eprintln!(
                "Error: workspace '{workspace_name}' not found.\nAvailable workspaces: {}",
                names.join(", ")
            );
            Err(ExitCode::from(2))
        }
    }
}

// ── Input validation ─────────────────────────────────────────────

fn validate_git_ref(s: &str) -> Result<&str, String> {
    if s.is_empty() {
        return Err("git ref cannot be empty".to_string());
    }
    // Reject refs starting with '-' to prevent argument injection
    if s.starts_with('-') {
        return Err("git ref cannot start with '-'".to_string());
    }
    // Allowlist: only permit safe characters in git refs
    // Covers branches, tags, HEAD~N, HEAD^N, @{n}, commit SHAs
    if !s.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || matches!(c, '.' | '_' | '-' | '/' | '~' | '^' | '@' | '{' | '}')
    }) {
        return Err("git ref contains disallowed characters".to_string());
    }
    Ok(s)
}

fn validate_root(root: &std::path::Path) -> Result<PathBuf, String> {
    let canonical = root
        .canonicalize()
        .map_err(|e| format!("invalid root path '{}': {e}", root.display()))?;
    if !canonical.is_dir() {
        return Err(format!("root path '{}' is not a directory", root.display()));
    }
    Ok(canonical)
}

// ── Main ─────────────────────────────────────────────────────────

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Handle schema commands before tracing setup (no side effects)
    if matches!(cli.command, Some(Command::Schema)) {
        return run_schema();
    }
    if matches!(cli.command, Some(Command::ConfigSchema)) {
        return run_config_schema();
    }

    // Set up tracing
    if !cli.quiet {
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive(tracing::Level::INFO.into()),
            )
            .with_target(false)
            .with_timer(tracing_subscriber::fmt::time::uptime())
            .init();
    }

    // Validate and resolve root
    let raw_root = cli
        .root
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));
    let root = match validate_root(&raw_root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::from(2);
        }
    };

    // Validate --changed-since early
    if let Some(ref git_ref) = cli.changed_since
        && let Err(e) = validate_git_ref(git_ref)
    {
        eprintln!("Error: invalid --changed-since: {e}");
        return ExitCode::from(2);
    }

    let threads = cli.threads.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    });

    match cli.command.unwrap_or(Command::Check {
        fail_on_issues: false,
        sarif_file: None,
        unused_files: false,
        unused_exports: false,
        unused_deps: false,
        unused_types: false,
        unused_enum_members: false,
        unused_class_members: false,
        unresolved_imports: false,
        unlisted_deps: false,
        duplicate_exports: false,
        include_dupes: false,
    }) {
        Command::Check {
            fail_on_issues,
            sarif_file,
            unused_files,
            unused_exports,
            unused_deps,
            unused_types,
            unused_enum_members,
            unused_class_members,
            unresolved_imports,
            unlisted_deps,
            duplicate_exports,
            include_dupes,
        } => {
            let filters = IssueFilters {
                unused_files,
                unused_exports,
                unused_deps,
                unused_types,
                unused_enum_members,
                unused_class_members,
                unresolved_imports,
                unlisted_deps,
                duplicate_exports,
            };
            run_check(
                &root,
                &cli.config,
                cli.format.into(),
                cli.no_cache,
                threads,
                cli.quiet,
                fail_on_issues,
                &filters,
                cli.changed_since.as_deref(),
                cli.baseline.as_deref(),
                cli.save_baseline.as_deref(),
                sarif_file.as_deref(),
                cli.production,
                cli.workspace.as_deref(),
                include_dupes,
            )
        }
        Command::Watch => run_watch(
            &root,
            &cli.config,
            cli.format.into(),
            cli.no_cache,
            threads,
            cli.quiet,
            cli.production,
        ),
        Command::Fix { dry_run } => fix::run_fix(
            &root,
            &cli.config,
            cli.format.into(),
            cli.no_cache,
            threads,
            cli.quiet,
            dry_run,
            cli.production,
        ),
        Command::Init { toml } => run_init(&root, toml),
        Command::ConfigSchema => run_config_schema(),
        Command::List {
            entry_points,
            files,
            plugins,
        } => run_list(
            &root,
            &cli.config,
            cli.format.into(),
            threads,
            entry_points,
            files,
            plugins,
            cli.production,
        ),
        Command::Dupes {
            mode,
            min_tokens,
            min_lines,
            threshold,
            skip_local,
            cross_language,
        } => run_dupes(
            &root,
            &cli.config,
            cli.format.into(),
            cli.no_cache,
            threads,
            cli.quiet,
            mode,
            min_tokens,
            min_lines,
            threshold,
            skip_local,
            cross_language,
            cli.baseline.as_deref(),
            cli.save_baseline.as_deref(),
            cli.production,
        ),
        Command::Schema => unreachable!("handled above"),
    }
}

// ── Commands ─────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn run_check(
    root: &std::path::Path,
    config_path: &Option<PathBuf>,
    output: OutputFormat,
    no_cache: bool,
    threads: usize,
    quiet: bool,
    fail_on_issues: bool,
    filters: &IssueFilters,
    changed_since: Option<&str>,
    baseline: Option<&std::path::Path>,
    save_baseline: Option<&std::path::Path>,
    sarif_file: Option<&std::path::Path>,
    production: bool,
    workspace: Option<&str>,
    include_dupes: bool,
) -> ExitCode {
    let start = Instant::now();

    let config = match load_config(root, config_path, output, no_cache, threads, production) {
        Ok(c) => c,
        Err(code) => return code,
    };

    // Validate --workspace early (before analysis) to fail fast
    let ws_root = if let Some(ws_name) = workspace {
        match resolve_workspace_filter(root, ws_name) {
            Ok(root) => Some(root),
            Err(code) => return code,
        }
    } else {
        None
    };

    // Get changed files if --changed-since is set (already validated)
    let changed_files: Option<std::collections::HashSet<std::path::PathBuf>> =
        changed_since.and_then(|git_ref| get_changed_files(root, git_ref));

    let mut results = match fallow_core::analyze(&config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Analysis error: {e}");
            return ExitCode::from(2);
        }
    };
    let elapsed = start.elapsed();

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
    let unfiltered_results = if include_dupes && config.duplicates.enabled {
        Some(results.clone())
    } else {
        None
    };

    // Apply issue type filters (CLI --unused-files etc.)
    filters.apply(&mut results);

    // Save baseline if requested
    if let Some(baseline_path) = save_baseline {
        let baseline_data = BaselineData::from_results(&results);
        if let Ok(json) = serde_json::to_string_pretty(&baseline_data) {
            if let Err(e) = std::fs::write(baseline_path, json) {
                eprintln!("Failed to save baseline: {e}");
            } else if !quiet {
                eprintln!("Baseline saved to {}", baseline_path.display());
            }
        }
    }

    // Compare against baseline if provided
    if let Some(baseline_path) = baseline
        && let Ok(content) = std::fs::read_to_string(baseline_path)
        && let Ok(baseline_data) = serde_json::from_str::<BaselineData>(&content)
    {
        results = filter_new_issues(results, &baseline_data);
        if !quiet {
            eprintln!("Comparing against baseline: {}", baseline_path.display());
        }
    }

    // Write SARIF to file if requested (independent of --format)
    if let Some(sarif_path) = sarif_file {
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
                } else if !quiet {
                    eprintln!("SARIF output written to {}", sarif_path.display());
                }
            }
            Err(e) => {
                eprintln!("Warning: failed to serialize SARIF output: {e}");
            }
        }
    }

    // When --fail-on-issues is set, promote all Warn to Error for this run
    let effective_rules = if fail_on_issues {
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

    let report_code = report::print_results(&results, &config, elapsed, quiet);
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
            report::print_cross_reference_findings(&cross_ref, &config.root, quiet, &config.output);
        }
    }

    if has_error_severity_issues(&results, &effective_rules) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// Get files changed since a git ref.
fn get_changed_files(
    root: &std::path::Path,
    git_ref: &str,
) -> Option<std::collections::HashSet<std::path::PathBuf>> {
    let output = match std::process::Command::new("git")
        .args(["diff", "--name-only", &format!("{}...HEAD", git_ref)])
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

    let files: std::collections::HashSet<std::path::PathBuf> =
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|line| root.join(line))
            .collect();

    Some(files)
}

fn run_watch(
    root: &PathBuf,
    config_path: &Option<PathBuf>,
    output: OutputFormat,
    no_cache: bool,
    threads: usize,
    quiet: bool,
    production: bool,
) -> ExitCode {
    use notify_debouncer_mini::{DebouncedEventKind, new_debouncer};
    use std::sync::mpsc;
    use std::time::Duration;

    let config = match load_config(
        root,
        config_path,
        output.clone(),
        no_cache,
        threads,
        production,
    ) {
        Ok(c) => c,
        Err(code) => return code,
    };

    eprintln!("Watching for changes... (press Ctrl+C to stop)");

    // Run initial analysis
    let start = Instant::now();
    let results = match fallow_core::analyze(&config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Analysis error: {e}");
            return ExitCode::from(2);
        }
    };
    let elapsed = start.elapsed();
    let report_code = report::print_results(&results, &config, elapsed, quiet);
    if report_code != ExitCode::SUCCESS {
        return report_code;
    }

    // Set up file watcher
    let (tx, rx) = mpsc::channel();
    let mut debouncer = match new_debouncer(Duration::from_millis(500), tx) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to create file watcher: {e}");
            return ExitCode::from(2);
        }
    };

    if let Err(e) = debouncer
        .watcher()
        .watch(root.as_ref(), notify::RecursiveMode::Recursive)
    {
        eprintln!("Failed to watch directory: {e}");
        return ExitCode::from(2);
    }

    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                // Filter to only source file changes
                let has_source_changes = events.iter().any(|e| {
                    matches!(e.kind, DebouncedEventKind::Any) && {
                        let path_str = e.path.to_string_lossy();
                        path_str.ends_with(".ts")
                            || path_str.ends_with(".tsx")
                            || path_str.ends_with(".js")
                            || path_str.ends_with(".jsx")
                            || path_str.ends_with(".mts")
                            || path_str.ends_with(".cts")
                            || path_str.ends_with(".mjs")
                            || path_str.ends_with(".cjs")
                    }
                });

                if has_source_changes {
                    eprintln!("\nFile changed, re-analyzing...");
                    let config = match load_config(
                        root,
                        config_path,
                        output.clone(),
                        no_cache,
                        threads,
                        production,
                    ) {
                        Ok(c) => c,
                        Err(_) => {
                            eprintln!(
                                "Warning: failed to reload config, using previous configuration"
                            );
                            continue;
                        }
                    };
                    let start = Instant::now();
                    match fallow_core::analyze(&config) {
                        Ok(results) => {
                            let elapsed = start.elapsed();
                            let report_code =
                                report::print_results(&results, &config, elapsed, quiet);
                            if report_code != ExitCode::SUCCESS {
                                return report_code;
                            }
                        }
                        Err(e) => {
                            eprintln!("Analysis error: {e}");
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                eprintln!("Watch error: {e:?}");
            }
            Err(e) => {
                eprintln!("Channel error: {e}");
                return ExitCode::from(2);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_dupes(
    root: &std::path::Path,
    config_path: &Option<PathBuf>,
    output: OutputFormat,
    no_cache: bool,
    threads: usize,
    quiet: bool,
    mode: DupesMode,
    min_tokens: usize,
    min_lines: usize,
    threshold: f64,
    skip_local: bool,
    cross_language: bool,
    baseline_path: Option<&std::path::Path>,
    save_baseline_path: Option<&std::path::Path>,
    production: bool,
) -> ExitCode {
    let start = Instant::now();

    let config = match load_config(
        root,
        config_path,
        output.clone(),
        no_cache,
        threads,
        production,
    ) {
        Ok(c) => c,
        Err(code) => return code,
    };

    // Build duplication config: start from fallow.toml, override with CLI args
    let toml_dupes = &config.duplicates;
    let dupes_config = fallow_config::DuplicatesConfig {
        enabled: true,
        mode: match mode {
            DupesMode::Strict => fallow_config::DetectionMode::Strict,
            DupesMode::Mild => fallow_config::DetectionMode::Mild,
            DupesMode::Weak => fallow_config::DetectionMode::Weak,
            DupesMode::Semantic => fallow_config::DetectionMode::Semantic,
        },
        min_tokens,
        min_lines,
        threshold,
        ignore: toml_dupes.ignore.clone(),
        skip_local,
        cross_language: cross_language || toml_dupes.cross_language,
        normalization: toml_dupes.normalization.clone(),
    };

    // Discover files
    let files = fallow_core::discover::discover_files(&config);

    // Run duplication detection
    let mut report = fallow_core::duplicates::find_duplicates(&config.root, &files, &dupes_config);

    // Save baseline if requested (before filtering)
    if let Some(path) = save_baseline_path {
        let baseline_data = DuplicationBaselineData::from_report(&report, &config.root);
        match serde_json::to_string_pretty(&baseline_data) {
            Ok(json) => {
                if let Err(e) = std::fs::write(path, json) {
                    eprintln!("Error: failed to write duplication baseline: {e}");
                    return ExitCode::from(2);
                }
                if !quiet {
                    eprintln!("Saved duplication baseline to {}", path.display());
                }
            }
            Err(e) => {
                eprintln!("Error: failed to serialize duplication baseline: {e}");
                return ExitCode::from(2);
            }
        }
    }

    // Filter against baseline if provided
    if let Some(path) = baseline_path {
        match std::fs::read_to_string(path) {
            Ok(json) => match serde_json::from_str::<DuplicationBaselineData>(&json) {
                Ok(baseline_data) => {
                    report = filter_new_clone_groups(report, &baseline_data, &config.root);
                }
                Err(e) => {
                    eprintln!("Error: failed to parse duplication baseline: {e}");
                    return ExitCode::from(2);
                }
            },
            Err(e) => {
                eprintln!("Error: failed to read duplication baseline: {e}");
                return ExitCode::from(2);
            }
        }
    }

    let elapsed = start.elapsed();

    // Print results
    let result = report::print_duplication_report(&report, &config, elapsed, quiet, &output);
    if result != ExitCode::SUCCESS {
        return result;
    }

    // Check threshold
    if threshold > 0.0 && report.stats.duplication_percentage > threshold {
        eprintln!(
            "Duplication ({:.1}%) exceeds threshold ({:.1}%)",
            report.stats.duplication_percentage, threshold
        );
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}

fn run_init(root: &std::path::Path, use_toml: bool) -> ExitCode {
    // Check if any config file already exists
    let existing_names = ["fallow.jsonc", "fallow.json", "fallow.toml", ".fallow.toml"];
    for name in &existing_names {
        let path = root.join(name);
        if path.exists() {
            eprintln!("{name} already exists");
            return ExitCode::from(2);
        }
    }

    if use_toml {
        let config_path = root.join("fallow.toml");
        let default_config = r#"# fallow.toml - Dead code analysis configuration
# See https://github.com/fallow-rs/fallow for documentation

# Additional entry points (beyond auto-detected ones)
# entry = ["src/workers/*.ts"]

# Patterns to ignore
# ignore = ["**/*.generated.ts"]

# Dependencies to ignore (always considered used)
# ignoreDependencies = ["autoprefixer"]

[detect]
unusedFiles = true
unusedExports = true
unusedDependencies = true
unusedDevDependencies = true
unusedTypes = true

# Per-issue-type severity: "error" (fail CI), "warn" (report only), "off" (ignore)
# All default to "error" when omitted.
# [rules]
# unusedFiles = "error"
# unusedExports = "warn"
# unusedTypes = "off"
# unresolvedImports = "error"
"#;
        if let Err(e) = std::fs::write(&config_path, default_config) {
            eprintln!("Error: Failed to write fallow.toml: {e}");
            return ExitCode::from(2);
        }
        eprintln!("Created fallow.toml");
    } else {
        let config_path = root.join("fallow.jsonc");
        let default_config = r#"{
  // fallow.jsonc - Dead code analysis configuration
  // See https://github.com/fallow-rs/fallow for documentation
  "$schema": "https://raw.githubusercontent.com/fallow-rs/fallow/main/schema.json",

  // Additional entry points (beyond auto-detected ones)
  // "entry": ["src/workers/*.ts"],

  // Patterns to ignore
  // "ignore": ["**/*.generated.ts"],

  // Dependencies to ignore (always considered used)
  // "ignoreDependencies": ["autoprefixer"],

  "detect": {
    "unusedFiles": true,
    "unusedExports": true,
    "unusedDependencies": true,
    "unusedDevDependencies": true,
    "unusedTypes": true
  }

  // Per-issue-type severity: "error" (fail CI), "warn" (report only), "off" (ignore)
  // All default to "error" when omitted.
  // "rules": {
  //   "unusedFiles": "error",
  //   "unusedExports": "warn",
  //   "unusedTypes": "off",
  //   "unresolvedImports": "error"
  // }
}
"#;
        if let Err(e) = std::fs::write(&config_path, default_config) {
            eprintln!("Error: Failed to write fallow.jsonc: {e}");
            return ExitCode::from(2);
        }
        eprintln!("Created fallow.jsonc");
    }
    ExitCode::SUCCESS
}

fn run_config_schema() -> ExitCode {
    let schema = FallowConfig::json_schema();
    match serde_json::to_string_pretty(&schema) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: failed to serialize schema: {e}");
            ExitCode::from(2)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_list(
    root: &std::path::Path,
    config_path: &Option<PathBuf>,
    output: OutputFormat,
    threads: usize,
    entry_points: bool,
    files: bool,
    plugins: bool,
    production: bool,
) -> ExitCode {
    let config = match load_config(
        root,
        config_path,
        OutputFormat::Human,
        true,
        threads,
        production,
    ) {
        Ok(c) => c,
        Err(code) => return code,
    };

    let show_all = !entry_points && !files && !plugins;

    // Run plugin detection to find active plugins (including workspace packages)
    let plugin_result = if plugins || show_all {
        let disc = fallow_core::discover::discover_files(&config);
        let file_paths: Vec<std::path::PathBuf> = disc.iter().map(|f| f.path.clone()).collect();
        let registry = fallow_core::plugins::PluginRegistry::new();

        let pkg_path = root.join("package.json");
        let mut result = if let Ok(pkg) = fallow_config::PackageJson::load(&pkg_path) {
            registry.run(&pkg, root, &file_paths)
        } else {
            fallow_core::plugins::AggregatedPluginResult::default()
        };

        // Also run plugins for workspace packages
        let workspaces = fallow_config::discover_workspaces(root);
        for ws in &workspaces {
            let ws_pkg_path = ws.root.join("package.json");
            if let Ok(ws_pkg) = fallow_config::PackageJson::load(&ws_pkg_path) {
                let ws_result = registry.run(&ws_pkg, &ws.root, &file_paths);
                for plugin_name in &ws_result.active_plugins {
                    if !result.active_plugins.contains(plugin_name) {
                        result.active_plugins.push(plugin_name.clone());
                    }
                }
            }
        }
        Some(result)
    } else {
        None
    };

    match output {
        OutputFormat::Json => {
            let mut result = serde_json::Map::new();

            if (plugins || show_all)
                && let Some(ref pr) = plugin_result
            {
                let pl: Vec<serde_json::Value> = pr
                    .active_plugins
                    .iter()
                    .map(|name| serde_json::json!({ "name": name }))
                    .collect();
                result.insert("plugins".to_string(), serde_json::json!(pl));
            }

            // Discover files once if needed by either files or entry_points
            let need_files = files || show_all || entry_points;
            let discovered = if need_files {
                Some(fallow_core::discover::discover_files(&config))
            } else {
                None
            };

            if (files || show_all)
                && let Some(ref disc) = discovered
            {
                let paths: Vec<serde_json::Value> = disc
                    .iter()
                    .map(|f| {
                        let relative = f.path.strip_prefix(root).unwrap_or(&f.path);
                        serde_json::json!(relative.display().to_string())
                    })
                    .collect();
                result.insert("file_count".to_string(), serde_json::json!(paths.len()));
                result.insert("files".to_string(), serde_json::json!(paths));
            }

            if (entry_points || show_all)
                && let Some(ref disc) = discovered
            {
                let mut entries = fallow_core::discover::discover_entry_points(&config, disc);
                // Add workspace entry points
                let workspaces = fallow_config::discover_workspaces(root);
                for ws in &workspaces {
                    let ws_entries = fallow_core::discover::discover_workspace_entry_points(
                        &ws.root, &config, disc,
                    );
                    entries.extend(ws_entries);
                }
                // Add plugin-discovered entry points
                if let Some(ref pr) = plugin_result {
                    let plugin_entries =
                        fallow_core::discover::discover_plugin_entry_points(pr, &config, disc);
                    entries.extend(plugin_entries);
                }
                let eps: Vec<serde_json::Value> = entries
                    .iter()
                    .map(|ep| {
                        let relative = ep.path.strip_prefix(root).unwrap_or(&ep.path);
                        serde_json::json!({
                            "path": relative.display().to_string(),
                            "source": format!("{:?}", ep.source),
                        })
                    })
                    .collect();
                result.insert(
                    "entry_point_count".to_string(),
                    serde_json::json!(eps.len()),
                );
                result.insert("entry_points".to_string(), serde_json::json!(eps));
            }

            match serde_json::to_string_pretty(&serde_json::Value::Object(result)) {
                Ok(json) => println!("{json}"),
                Err(e) => {
                    eprintln!("Error: failed to serialize list output: {e}");
                    return ExitCode::from(2);
                }
            }
        }
        _ => {
            if (plugins || show_all)
                && let Some(ref pr) = plugin_result
            {
                eprintln!("Active plugins:");
                for name in &pr.active_plugins {
                    eprintln!("  - {name}");
                }
            }

            // Discover files once for both files and entry_points
            let need_discover = files || entry_points || show_all;
            let discovered = if need_discover {
                Some(fallow_core::discover::discover_files(&config))
            } else {
                None
            };

            if (files || show_all)
                && let Some(ref disc) = discovered
            {
                eprintln!("Discovered {} files", disc.len());
                for file in disc {
                    println!("{}", file.path.display());
                }
            }

            if (entry_points || show_all)
                && let Some(ref disc) = discovered
            {
                let mut entries = fallow_core::discover::discover_entry_points(&config, disc);
                // Add workspace entry points
                let workspaces = fallow_config::discover_workspaces(root);
                for ws in &workspaces {
                    let ws_entries = fallow_core::discover::discover_workspace_entry_points(
                        &ws.root, &config, disc,
                    );
                    entries.extend(ws_entries);
                }
                // Add plugin-discovered entry points
                if let Some(ref pr) = plugin_result {
                    let plugin_entries =
                        fallow_core::discover::discover_plugin_entry_points(pr, &config, disc);
                    entries.extend(plugin_entries);
                }
                eprintln!("Found {} entry points", entries.len());
                for ep in &entries {
                    println!("{} ({:?})", ep.path.display(), ep.source);
                }
            }
        }
    }

    ExitCode::SUCCESS
}

// ── Schema command ───────────────────────────────────────────────

fn run_schema() -> ExitCode {
    let cmd = Cli::command();
    let schema = build_cli_schema(&cmd);
    match serde_json::to_string_pretty(&schema) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: failed to serialize schema: {e}");
            ExitCode::from(2)
        }
    }
}

fn build_cli_schema(cmd: &clap::Command) -> serde_json::Value {
    let mut global_flags = Vec::new();
    for arg in cmd.get_arguments() {
        if arg.get_id() == "help" || arg.get_id() == "version" {
            continue;
        }
        global_flags.push(build_arg_schema(arg));
    }

    let mut commands = Vec::new();
    for sub in cmd.get_subcommands() {
        if sub.get_name() == "help" {
            continue;
        }
        let mut flags = Vec::new();
        for arg in sub.get_arguments() {
            if arg.get_id() == "help" || arg.get_id() == "version" {
                continue;
            }
            flags.push(build_arg_schema(arg));
        }
        commands.push(serde_json::json!({
            "name": sub.get_name(),
            "description": sub.get_about().map(|s| s.to_string()),
            "flags": flags,
        }));
    }

    serde_json::json!({
        "name": cmd.get_name(),
        "version": env!("CARGO_PKG_VERSION"),
        "description": cmd.get_about().map(|s| s.to_string()),
        "global_flags": global_flags,
        "commands": commands,
        "default_command": "check",
        "issue_types": [
            {
                "id": "unused-file",
                "description": "File is not reachable from any entry point",
                "filter_flag": "--unused-files",
                "fixable": false,
                "suppressible": true,
                "suppress_comment": "// fallow-ignore-file unused-file"
            },
            {
                "id": "unused-export",
                "description": "Export is never imported by other modules",
                "filter_flag": "--unused-exports",
                "fixable": true,
                "suppressible": true,
                "suppress_comment": "// fallow-ignore-next-line unused-export"
            },
            {
                "id": "unused-type",
                "description": "Type export is never imported by other modules",
                "filter_flag": "--unused-types",
                "fixable": false,
                "suppressible": true,
                "suppress_comment": "// fallow-ignore-next-line unused-type"
            },
            {
                "id": "unused-dependency",
                "description": "Package in dependencies is never imported",
                "filter_flag": "--unused-deps",
                "fixable": true,
                "suppressible": false,
                "note": "--unused-deps controls both unused-dependency and unused-dev-dependency"
            },
            {
                "id": "unused-dev-dependency",
                "description": "Package in devDependencies is never imported",
                "filter_flag": "--unused-deps",
                "fixable": true,
                "suppressible": false,
                "note": "--unused-deps controls both unused-dependency and unused-dev-dependency"
            },
            {
                "id": "unused-enum-member",
                "description": "Enum member is never referenced",
                "filter_flag": "--unused-enum-members",
                "fixable": false,
                "suppressible": true,
                "suppress_comment": "// fallow-ignore-next-line unused-enum-member"
            },
            {
                "id": "unused-class-member",
                "description": "Class member is never referenced",
                "filter_flag": "--unused-class-members",
                "fixable": false,
                "suppressible": true,
                "suppress_comment": "// fallow-ignore-next-line unused-class-member"
            },
            {
                "id": "unresolved-import",
                "description": "Import specifier could not be resolved to a file",
                "filter_flag": "--unresolved-imports",
                "fixable": false,
                "suppressible": true,
                "suppress_comment": "// fallow-ignore-next-line unresolved-import"
            },
            {
                "id": "unlisted-dependency",
                "description": "Package is imported but not in package.json",
                "filter_flag": "--unlisted-deps",
                "fixable": false,
                "suppressible": false
            },
            {
                "id": "duplicate-export",
                "description": "Same export name appears in multiple modules",
                "filter_flag": "--duplicate-exports",
                "fixable": false,
                "suppressible": true,
                "suppress_comment": "// fallow-ignore-file duplicate-export"
            }
        ],
        "suppression_comments": {
            "next_line": "// fallow-ignore-next-line [issue-type]",
            "file": "// fallow-ignore-file [issue-type]",
            "note": "Omit [issue-type] to suppress all issue types. Unknown tokens are silently ignored."
        },
        "output_formats": ["human", "json", "sarif", "compact"],
        "exit_codes": {
            "0": "Success (no error-severity issues found)",
            "1": "Error-severity issues found (per rules config, or --fail-on-issues promotes warn→error)",
            "2": "Error (invalid config, invalid input, etc.)"
        },
        "severity_levels": ["error", "warn", "off"]
    })
}

fn build_arg_schema(arg: &clap::Arg) -> serde_json::Value {
    let name = arg
        .get_long()
        .map(|l| format!("--{l}"))
        .unwrap_or_else(|| arg.get_id().to_string());

    let arg_type = match arg.get_action() {
        clap::ArgAction::SetTrue | clap::ArgAction::SetFalse => "bool",
        clap::ArgAction::Count => "count",
        _ => "string",
    };

    let possible: Vec<String> = arg
        .get_possible_values()
        .iter()
        .map(|v| v.get_name().to_string())
        .collect();

    let mut schema = serde_json::json!({
        "name": name,
        "type": arg_type,
        "required": arg.is_required_set(),
        "description": arg.get_help().map(|s| s.to_string()),
    });

    if let Some(short) = arg.get_short() {
        schema["short"] = serde_json::json!(format!("-{short}"));
    }

    if let Some(default) = arg.get_default_values().first() {
        schema["default"] = serde_json::json!(default.to_str());
    }

    if !possible.is_empty() {
        schema["possible_values"] = serde_json::json!(possible);
    }

    schema
}

// ── Config loading ───────────────────────────────────────────────

fn load_config(
    root: &std::path::Path,
    config_path: &Option<PathBuf>,
    output: OutputFormat,
    no_cache: bool,
    threads: usize,
    production: bool,
) -> Result<fallow_config::ResolvedConfig, ExitCode> {
    let user_config = if let Some(path) = config_path {
        // Explicit --config: propagate errors
        match FallowConfig::load(path) {
            Ok(c) => Some(c),
            Err(e) => {
                eprintln!("Error: failed to load config '{}': {e}", path.display());
                return Err(ExitCode::from(2));
            }
        }
    } else {
        match FallowConfig::find_and_load(root) {
            Ok(found) => found.map(|(c, _)| c),
            Err(e) => {
                eprintln!("Error: {e}");
                return Err(ExitCode::from(2));
            }
        }
    };

    Ok(match user_config {
        Some(mut config) => {
            config.output = output;
            // CLI --production flag overrides config
            if production {
                config.production = true;
            }
            config.resolve(root.to_path_buf(), threads, no_cache)
        }
        None => FallowConfig {
            schema: None,
            entry: vec![],
            ignore: vec![],
            detect: fallow_config::DetectConfig::default(),
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            output,
            duplicates: fallow_config::DuplicatesConfig::default(),
            rules: fallow_config::RulesConfig::default(),
            production,
        }
        .resolve(root.to_path_buf(), threads, no_cache),
    })
}
