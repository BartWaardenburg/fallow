// CLI binary legitimately prints to stdout/stderr
#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use fallow_config::FallowConfig;

mod baseline;
mod check;
mod dupes;
mod fix;
mod init;
mod list;
mod migrate;
mod report;
mod schema;
mod validate;
mod watch;

use check::{CheckOptions, IssueFilters, TraceOptions};
use dupes::{DupesMode, DupesOptions};
use list::ListOptions;

// ── CLI definition ───────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "fallow",
    about = "Find unused files, exports, and dependencies in JavaScript/TypeScript projects",
    version
)]
#[allow(clippy::struct_excessive_bools)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Project root directory
    #[arg(short, long, global = true)]
    root: Option<PathBuf>,

    /// Path to config file (.fallowrc.json or fallow.toml)
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

        /// Trace why an export is used/unused (format: `FILE:EXPORT_NAME`)
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

    /// Auto-fix issues (remove unused exports, dependencies, enum members)
    Fix {
        /// Dry run — show what would be changed without modifying files
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt (required in non-TTY environments like CI or AI agents)
        #[arg(long, alias = "force")]
        yes: bool,
    },

    /// Initialize a .fallowrc.json configuration file
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

        /// Trace all clones at a specific location (format: `FILE:LINE`)
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
enum Format {
    Human,
    Json,
    Sarif,
    Compact,
}

impl From<Format> for fallow_config::OutputFormat {
    fn from(f: Format) -> Self {
        match f {
            Format::Human => Self::Human,
            Format::Json => Self::Json,
            Format::Sarif => Self::Sarif,
            Format::Compact => Self::Compact,
        }
    }
}

// ── Structured error output ──────────────────────────────────────

/// Emit an error as structured JSON on stdout when `--format json` is active,
/// then return the given exit code. For non-JSON formats, emit to stderr as usual.
fn emit_error(message: &str, exit_code: u8, output: &fallow_config::OutputFormat) -> ExitCode {
    if matches!(output, fallow_config::OutputFormat::Json) {
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

/// Read `FALLOW_FORMAT` env var and parse it into a Format value.
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

/// Read `FALLOW_QUIET` env var: "1" or "true" (case-insensitive) means quiet.
fn quiet_from_env() -> bool {
    std::env::var("FALLOW_QUIET")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

// ── Config loading ───────────────────────────────────────────────

#[allow(clippy::ref_option)] // &Option matches clap's field type
fn load_config(
    root: &std::path::Path,
    config_path: &Option<PathBuf>,
    output: fallow_config::OutputFormat,
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
                return Err(emit_error(&e, 2, &output));
            }
        }
    };

    Ok(match user_config {
        Some(mut config) => {
            // CLI --production flag overrides config
            if production {
                config.production = true;
            }
            config.resolve(root.to_path_buf(), output, threads, no_cache)
        }
        None => FallowConfig {
            schema: None,
            extends: vec![],
            entry: vec![],
            ignore_patterns: vec![],
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            duplicates: fallow_config::DuplicatesConfig::default(),
            rules: fallow_config::RulesConfig::default(),
            production,
            plugins: vec![],
            overrides: vec![],
        }
        .resolve(root.to_path_buf(), output, threads, no_cache),
    })
}

// ── Main ─────────────────────────────────────────────────────────

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Handle schema commands before tracing setup (no side effects)
    if matches!(cli.command, Some(Command::Schema)) {
        return schema::run_schema();
    }
    if matches!(cli.command, Some(Command::ConfigSchema)) {
        return init::run_config_schema();
    }
    if matches!(cli.command, Some(Command::PluginSchema)) {
        return init::run_plugin_schema();
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

    let output: fallow_config::OutputFormat = format.clone().into();

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
        && let Err(e) = validate::validate_no_control_chars(s, "--config")
    {
        return emit_error(&e, 2, &output);
    }
    if let Some(ref ws) = cli.workspace
        && let Err(e) = validate::validate_no_control_chars(ws, "--workspace")
    {
        return emit_error(&e, 2, &output);
    }
    if let Some(ref git_ref) = cli.changed_since
        && let Err(e) = validate::validate_no_control_chars(git_ref, "--changed-since")
    {
        return emit_error(&e, 2, &output);
    }

    // Validate and resolve root
    let raw_root = cli
        .root
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));
    let root = match validate::validate_root(&raw_root) {
        Ok(r) => r,
        Err(e) => {
            return emit_error(&e, 2, &output);
        }
    };

    // Validate --changed-since early
    if let Some(ref git_ref) = cli.changed_since
        && let Err(e) = validate::validate_git_ref(git_ref)
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
            check::run_check(&CheckOptions {
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
        Command::Watch => watch::run_watch(
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
        Command::Init { toml } => init::run_init(&root, toml),
        Command::ConfigSchema => init::run_config_schema(),
        Command::PluginSchema => init::run_plugin_schema(),
        Command::List {
            entry_points,
            files,
            plugins,
        } => list::run_list(&ListOptions {
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
        } => dupes::run_dupes(&DupesOptions {
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
        } => migrate::run_migrate(&root, toml, dry_run, from.as_deref()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── emit_error ──────────────────────────────────────────────────

    #[test]
    fn emit_error_returns_given_exit_code() {
        let code = emit_error("test error", 2, &fallow_config::OutputFormat::Human);
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
}
