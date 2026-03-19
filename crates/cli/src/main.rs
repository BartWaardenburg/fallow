use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use clap::{CommandFactory, Parser, Subcommand};
use fallow_config::{
    ExternalPluginDef, FallowConfig, OutputFormat, RulesConfig, Severity, discover_workspaces,
};

mod baseline;
mod fix;
mod migrate;
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

    /// Show pipeline performance timing breakdown
    #[arg(long, global = true)]
    performance: bool,
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

        /// Trace why an export is used/unused (format: FILE:EXPORT_NAME)
        #[arg(long, value_name = "FILE:EXPORT")]
        trace: Option<String>,

        /// Trace all edges for a file (imports, exports, importers)
        #[arg(long, value_name = "PATH")]
        trace_file: Option<String>,

        /// Trace where a dependency is used
        #[arg(long, value_name = "PACKAGE")]
        trace_dependency: Option<String>,
    },

    /// Watch for changes and re-run analysis
    Watch,

    /// Auto-fix issues (remove unused exports, dependencies)
    Fix {
        /// Dry run — show what would be changed without modifying files
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt (required in non-TTY environments like CI or AI agents)
        #[arg(long, alias = "force")]
        yes: bool,
    },

    /// Initialize a fallow.jsonc configuration file
    Init {
        /// Generate TOML instead of JSONC
        #[arg(long)]
        toml: bool,
    },

    /// Print the JSON Schema for fallow configuration files
    ConfigSchema,

    /// Print the JSON Schema for external plugin files
    PluginSchema,

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

        /// Trace all clones at a specific location (format: FILE:LINE)
        #[arg(long, value_name = "FILE:LINE")]
        trace: Option<String>,
    },

    /// Dump the CLI interface as machine-readable JSON for agent introspection
    Schema,

    /// Migrate configuration from knip or jscpd to fallow
    Migrate {
        /// Generate TOML instead of JSONC
        #[arg(long)]
        toml: bool,

        /// Only preview the generated config without writing
        #[arg(long)]
        dry_run: bool,

        /// Path to source config file (auto-detect if not specified)
        #[arg(long, value_name = "PATH")]
        from: Option<PathBuf>,
    },
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

struct TraceOptions {
    trace_export: Option<String>,
    trace_file: Option<String>,
    trace_dependency: Option<String>,
    performance: bool,
}

impl TraceOptions {
    fn any_active(&self) -> bool {
        self.trace_export.is_some()
            || self.trace_file.is_some()
            || self.trace_dependency.is_some()
            || self.performance
    }
}

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

/// Reject strings containing control characters (bytes < 0x20) except
/// newline (0x0A) and tab (0x09). This prevents agents from accidentally
/// passing invisible characters in CLI arguments.
fn validate_no_control_chars(s: &str, arg_name: &str) -> Result<(), String> {
    for (i, byte) in s.bytes().enumerate() {
        if byte < 0x20 && byte != b'\n' && byte != b'\t' {
            return Err(format!(
                "{arg_name} contains control character (byte 0x{byte:02x}) at position {i}"
            ));
        }
    }
    Ok(())
}

// ── Structured error output ──────────────────────────────────────

/// Emit an error as structured JSON on stdout when `--format json` is active,
/// then return the given exit code. For non-JSON formats, emit to stderr as usual.
fn emit_error(message: &str, exit_code: u8, output: &OutputFormat) -> ExitCode {
    if matches!(output, OutputFormat::Json) {
        let error_obj = serde_json::json!({
            "error": true,
            "message": message,
            "exit_code": exit_code,
        });
        if let Ok(json) = serde_json::to_string_pretty(&error_obj) {
            println!("{json}");
        }
    } else {
        eprintln!("Error: {message}");
    }
    ExitCode::from(exit_code)
}

// ── Environment variable helpers ─────────────────────────────────

/// Read FALLOW_FORMAT env var and parse it into a Format value.
fn format_from_env() -> Option<Format> {
    let val = std::env::var("FALLOW_FORMAT").ok()?;
    match val.to_lowercase().as_str() {
        "json" => Some(Format::Json),
        "human" => Some(Format::Human),
        "sarif" => Some(Format::Sarif),
        "compact" => Some(Format::Compact),
        _ => None,
    }
}

/// Read FALLOW_QUIET env var: "1" or "true" (case-insensitive) means quiet.
fn quiet_from_env() -> bool {
    std::env::var("FALLOW_QUIET")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
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
    if matches!(cli.command, Some(Command::PluginSchema)) {
        return run_plugin_schema();
    }

    // Resolve output format: CLI flag > FALLOW_FORMAT env var > default ("human").
    // clap sets the default to "human", so we only override with the env var
    // when the user did NOT explicitly pass --format on the CLI.
    let cli_format_was_explicit = std::env::args().any(|a| {
        a == "--format" || a == "--output" || a.starts_with("--format=") || a.starts_with("-f")
    });
    let format: Format = if cli_format_was_explicit {
        cli.format
    } else {
        format_from_env().unwrap_or(cli.format)
    };

    // Resolve quiet: CLI --quiet flag > FALLOW_QUIET env var > false
    let quiet = cli.quiet || quiet_from_env();

    let output: OutputFormat = format.clone().into();

    // Set up tracing
    if !quiet {
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

    // Validate control characters in key string inputs
    if let Some(ref config_path) = cli.config
        && let Some(s) = config_path.to_str()
        && let Err(e) = validate_no_control_chars(s, "--config")
    {
        return emit_error(&e, 2, &output);
    }
    if let Some(ref ws) = cli.workspace
        && let Err(e) = validate_no_control_chars(ws, "--workspace")
    {
        return emit_error(&e, 2, &output);
    }
    if let Some(ref git_ref) = cli.changed_since
        && let Err(e) = validate_no_control_chars(git_ref, "--changed-since")
    {
        return emit_error(&e, 2, &output);
    }

    // Validate and resolve root
    let raw_root = cli
        .root
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));
    let root = match validate_root(&raw_root) {
        Ok(r) => r,
        Err(e) => {
            return emit_error(&e, 2, &output);
        }
    };

    // Validate --changed-since early
    if let Some(ref git_ref) = cli.changed_since
        && let Err(e) = validate_git_ref(git_ref)
    {
        return emit_error(&format!("invalid --changed-since: {e}"), 2, &output);
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
        trace: None,
        trace_file: None,
        trace_dependency: None,
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
            trace,
            trace_file,
            trace_dependency,
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
            let trace_opts = TraceOptions {
                trace_export: trace,
                trace_file,
                trace_dependency,
                performance: cli.performance,
            };
            run_check(&CheckOptions {
                root: &root,
                config_path: &cli.config,
                output,
                no_cache: cli.no_cache,
                threads,
                quiet,
                fail_on_issues,
                filters: &filters,
                changed_since: cli.changed_since.as_deref(),
                baseline: cli.baseline.as_deref(),
                save_baseline: cli.save_baseline.as_deref(),
                sarif_file: sarif_file.as_deref(),
                production: cli.production,
                workspace: cli.workspace.as_deref(),
                include_dupes,
                trace_opts: &trace_opts,
            })
        }
        Command::Watch => run_watch(
            &root,
            &cli.config,
            output,
            cli.no_cache,
            threads,
            quiet,
            cli.production,
        ),
        Command::Fix { dry_run, yes } => fix::run_fix(&fix::FixOptions {
            root: &root,
            config_path: &cli.config,
            output,
            no_cache: cli.no_cache,
            threads,
            quiet,
            dry_run,
            yes,
            production: cli.production,
        }),
        Command::Init { toml } => run_init(&root, toml),
        Command::ConfigSchema => run_config_schema(),
        Command::PluginSchema => run_plugin_schema(),
        Command::List {
            entry_points,
            files,
            plugins,
        } => run_list(&ListOptions {
            root: &root,
            config_path: &cli.config,
            output,
            threads,
            entry_points,
            files,
            plugins,
            production: cli.production,
        }),
        Command::Dupes {
            mode,
            min_tokens,
            min_lines,
            threshold,
            skip_local,
            cross_language,
            trace,
        } => run_dupes(&DupesOptions {
            root: &root,
            config_path: &cli.config,
            output,
            no_cache: cli.no_cache,
            threads,
            quiet,
            mode,
            min_tokens,
            min_lines,
            threshold,
            skip_local,
            cross_language,
            baseline_path: cli.baseline.as_deref(),
            save_baseline_path: cli.save_baseline.as_deref(),
            production: cli.production,
            trace: trace.as_deref(),
        }),
        Command::Schema => unreachable!("handled above"),
        Command::Migrate {
            toml,
            dry_run,
            from,
        } => migrate::run_migrate(&root, toml, dry_run, from),
    }
}

// ── Commands ─────────────────────────────────────────────────────

struct CheckOptions<'a> {
    root: &'a std::path::Path,
    config_path: &'a Option<PathBuf>,
    output: OutputFormat,
    no_cache: bool,
    threads: usize,
    quiet: bool,
    fail_on_issues: bool,
    filters: &'a IssueFilters,
    changed_since: Option<&'a str>,
    baseline: Option<&'a std::path::Path>,
    save_baseline: Option<&'a std::path::Path>,
    sarif_file: Option<&'a std::path::Path>,
    production: bool,
    workspace: Option<&'a str>,
    include_dupes: bool,
    trace_opts: &'a TraceOptions,
}

fn run_check(opts: &CheckOptions<'_>) -> ExitCode {
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
    let changed_files: Option<std::collections::HashSet<std::path::PathBuf>> = opts
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
            let (file_path, export_name) = match trace_spec.rsplit_once(':') {
                Some((f, e)) => (f, e),
                None => {
                    return emit_error(
                        "--trace requires FILE:EXPORT_NAME format (e.g., src/utils.ts:foo)",
                        2,
                        &opts.output,
                    );
                }
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
        eprintln!("Warning: report output failed");
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
                                eprintln!("Warning: report output failed");
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

struct DupesOptions<'a> {
    root: &'a std::path::Path,
    config_path: &'a Option<PathBuf>,
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
    baseline_path: Option<&'a std::path::Path>,
    save_baseline_path: Option<&'a std::path::Path>,
    production: bool,
    trace: Option<&'a str>,
}

fn run_dupes(opts: &DupesOptions<'_>) -> ExitCode {
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

    // Build duplication config: start from fallow.toml, override with CLI args
    let toml_dupes = &config.duplicates;
    let dupes_config = fallow_config::DuplicatesConfig {
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
    };

    // Discover files
    let files = fallow_core::discover::discover_files(&config);

    // Run duplication detection
    let mut report = fallow_core::duplicates::find_duplicates(&config.root, &files, &dupes_config);

    // Handle trace (diagnostic mode — early return)
    if let Some(trace_spec) = opts.trace {
        let (file_path, line_str) = match trace_spec.rsplit_once(':') {
            Some((f, l)) => (f, l),
            None => {
                return emit_error(
                    "--trace requires FILE:LINE format (e.g., src/utils.ts:42)",
                    2,
                    &opts.output,
                );
            }
        };
        let line: usize = match line_str.parse() {
            Ok(l) if l > 0 => l,
            _ => {
                return emit_error("--trace LINE must be a positive integer", 2, &opts.output);
            }
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

    let elapsed = start.elapsed();

    // Print results
    let result =
        report::print_duplication_report(&report, &config, elapsed, opts.quiet, &opts.output);
    if result != ExitCode::SUCCESS {
        return result;
    }

    // Check threshold
    if opts.threshold > 0.0 && report.stats.duplication_percentage > opts.threshold {
        eprintln!(
            "Duplication ({:.1}%) exceeds threshold ({:.1}%)",
            report.stats.duplication_percentage, opts.threshold
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

fn run_plugin_schema() -> ExitCode {
    let schema = ExternalPluginDef::json_schema();
    match serde_json::to_string_pretty(&schema) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: failed to serialize plugin schema: {e}");
            ExitCode::from(2)
        }
    }
}

struct ListOptions<'a> {
    root: &'a std::path::Path,
    config_path: &'a Option<PathBuf>,
    output: OutputFormat,
    threads: usize,
    entry_points: bool,
    files: bool,
    plugins: bool,
    production: bool,
}

fn run_list(opts: &ListOptions<'_>) -> ExitCode {
    let config = match load_config(
        opts.root,
        opts.config_path,
        OutputFormat::Human,
        true,
        opts.threads,
        opts.production,
    ) {
        Ok(c) => c,
        Err(code) => return code,
    };

    let show_all = !opts.entry_points && !opts.files && !opts.plugins;

    // Run plugin detection to find active plugins (including workspace packages)
    let plugin_result = if opts.plugins || show_all {
        let disc = fallow_core::discover::discover_files(&config);
        let file_paths: Vec<std::path::PathBuf> = disc.iter().map(|f| f.path.clone()).collect();
        let registry = fallow_core::plugins::PluginRegistry::new(config.external_plugins.clone());

        let pkg_path = opts.root.join("package.json");
        let mut result = if let Ok(pkg) = fallow_config::PackageJson::load(&pkg_path) {
            registry.run(&pkg, opts.root, &file_paths)
        } else {
            fallow_core::plugins::AggregatedPluginResult::default()
        };

        // Also run plugins for workspace packages
        let workspaces = fallow_config::discover_workspaces(opts.root);
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

    // Discover files once if needed by either files or entry_points
    let need_files = opts.files || show_all || opts.entry_points;
    let discovered = if need_files {
        Some(fallow_core::discover::discover_files(&config))
    } else {
        None
    };

    // Compute entry points once (shared by both JSON and human output branches)
    let all_entry_points = if (opts.entry_points || show_all)
        && let Some(ref disc) = discovered
    {
        let mut entries = fallow_core::discover::discover_entry_points(&config, disc);
        // Add workspace entry points
        let workspaces = fallow_config::discover_workspaces(opts.root);
        for ws in &workspaces {
            let ws_entries =
                fallow_core::discover::discover_workspace_entry_points(&ws.root, &config, disc);
            entries.extend(ws_entries);
        }
        // Add plugin-discovered entry points
        if let Some(ref pr) = plugin_result {
            let plugin_entries =
                fallow_core::discover::discover_plugin_entry_points(pr, &config, disc);
            entries.extend(plugin_entries);
        }
        Some(entries)
    } else {
        None
    };

    match opts.output {
        OutputFormat::Json => {
            let mut result = serde_json::Map::new();

            if (opts.plugins || show_all)
                && let Some(ref pr) = plugin_result
            {
                let pl: Vec<serde_json::Value> = pr
                    .active_plugins
                    .iter()
                    .map(|name| serde_json::json!({ "name": name }))
                    .collect();
                result.insert("plugins".to_string(), serde_json::json!(pl));
            }

            if (opts.files || show_all)
                && let Some(ref disc) = discovered
            {
                let paths: Vec<serde_json::Value> = disc
                    .iter()
                    .map(|f| {
                        let relative = f.path.strip_prefix(opts.root).unwrap_or(&f.path);
                        serde_json::json!(relative.display().to_string())
                    })
                    .collect();
                result.insert("file_count".to_string(), serde_json::json!(paths.len()));
                result.insert("files".to_string(), serde_json::json!(paths));
            }

            if let Some(ref entries) = all_entry_points {
                let eps: Vec<serde_json::Value> = entries
                    .iter()
                    .map(|ep| {
                        let relative = ep.path.strip_prefix(opts.root).unwrap_or(&ep.path);
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
            if (opts.plugins || show_all)
                && let Some(ref pr) = plugin_result
            {
                eprintln!("Active plugins:");
                for name in &pr.active_plugins {
                    eprintln!("  - {name}");
                }
            }

            if (opts.files || show_all)
                && let Some(ref disc) = discovered
            {
                eprintln!("Discovered {} files", disc.len());
                for file in disc {
                    println!("{}", file.path.display());
                }
            }

            if let Some(ref entries) = all_entry_points {
                eprintln!("Found {} entry points", entries.len());
                for ep in entries {
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
            "2": "Error (invalid config, invalid input, etc.). When --format json is active, errors are emitted as structured JSON on stdout: {\"error\": true, \"message\": \"...\", \"exit_code\": 2}"
        },
        "environment_variables": {
            "FALLOW_FORMAT": "Default output format (json/human/sarif/compact). CLI --format flag overrides this.",
            "FALLOW_QUIET": "Set to \"1\" or \"true\" to suppress progress output. CLI --quiet flag overrides this.",
            "FALLOW_BIN": "Path to fallow binary (used by fallow-mcp server)."
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
                let msg = format!("failed to load config '{}': {e}", path.display());
                return Err(emit_error(&msg, 2, &output));
            }
        }
    } else {
        match FallowConfig::find_and_load(root) {
            Ok(found) => found.map(|(c, _)| c),
            Err(e) => {
                return Err(emit_error(&e.to_string(), 2, &output));
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
            plugins: vec![],
        }
        .resolve(root.to_path_buf(), threads, no_cache),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── validate_no_control_chars ────────────────────────────────────

    #[test]
    fn control_chars_rejects_null_byte() {
        let result = validate_no_control_chars("main\x00branch", "--changed-since");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("0x00"));
        assert!(err.contains("--changed-since"));
    }

    #[test]
    fn control_chars_rejects_bell() {
        assert!(validate_no_control_chars("test\x07ref", "--workspace").is_err());
    }

    #[test]
    fn control_chars_rejects_escape() {
        assert!(validate_no_control_chars("\x1b[31mred", "--config").is_err());
    }

    #[test]
    fn control_chars_rejects_carriage_return() {
        assert!(validate_no_control_chars("main\rinjected", "--changed-since").is_err());
    }

    #[test]
    fn control_chars_allows_normal_text() {
        assert!(validate_no_control_chars("main", "--changed-since").is_ok());
    }

    #[test]
    fn control_chars_allows_newline() {
        assert!(validate_no_control_chars("line1\nline2", "--config").is_ok());
    }

    #[test]
    fn control_chars_allows_tab() {
        assert!(validate_no_control_chars("col1\tcol2", "--config").is_ok());
    }

    #[test]
    fn control_chars_allows_empty_string() {
        assert!(validate_no_control_chars("", "--workspace").is_ok());
    }

    #[test]
    fn control_chars_allows_unicode() {
        assert!(validate_no_control_chars("my-package-日本語", "--workspace").is_ok());
    }

    #[test]
    fn control_chars_allows_paths_with_dots_and_slashes() {
        assert!(validate_no_control_chars("./path/to/config.toml", "--config").is_ok());
    }

    // ── emit_error ──────────────────────────────────────────────────

    #[test]
    fn emit_error_returns_given_exit_code() {
        let code = emit_error("test error", 2, &OutputFormat::Human);
        assert_eq!(code, ExitCode::from(2));
    }

    // ── format/quiet parsing logic ─────────────────────────────────
    // Note: format_from_env() and quiet_from_env() read process-global
    // env vars, so we test the underlying parsing logic directly to
    // avoid unsafe set_var/remove_var and parallel test interference.

    #[test]
    fn format_parsing_covers_all_variants() {
        // The format_from_env function lowercases then matches.
        // Test the same logic inline.
        let parse = |s: &str| -> Option<Format> {
            match s.to_lowercase().as_str() {
                "json" => Some(Format::Json),
                "human" => Some(Format::Human),
                "sarif" => Some(Format::Sarif),
                "compact" => Some(Format::Compact),
                _ => None,
            }
        };
        assert!(matches!(parse("json"), Some(Format::Json)));
        assert!(matches!(parse("JSON"), Some(Format::Json)));
        assert!(matches!(parse("human"), Some(Format::Human)));
        assert!(matches!(parse("sarif"), Some(Format::Sarif)));
        assert!(matches!(parse("compact"), Some(Format::Compact)));
        assert!(parse("xml").is_none());
        assert!(parse("").is_none());
    }

    #[test]
    fn quiet_parsing_logic() {
        let parse = |s: &str| -> bool { s == "1" || s.eq_ignore_ascii_case("true") };
        assert!(parse("1"));
        assert!(parse("true"));
        assert!(parse("TRUE"));
        assert!(parse("True"));
        assert!(!parse("0"));
        assert!(!parse("false"));
        assert!(!parse("yes"));
    }

    // ── schema includes env vars ─────────────────────────────────────

    #[test]
    fn schema_includes_environment_variables() {
        let cmd = Cli::command();
        let schema = build_cli_schema(&cmd);
        let env_vars = &schema["environment_variables"];
        assert!(env_vars["FALLOW_FORMAT"].is_string());
        assert!(env_vars["FALLOW_QUIET"].is_string());
        assert!(env_vars["FALLOW_BIN"].is_string());
    }

    #[test]
    fn schema_exit_code_2_mentions_json_errors() {
        let cmd = Cli::command();
        let schema = build_cli_schema(&cmd);
        let exit_2 = schema["exit_codes"]["2"].as_str().unwrap();
        assert!(exit_2.contains("JSON"));
    }
}
