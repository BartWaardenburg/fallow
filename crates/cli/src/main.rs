use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use clap::{Parser, Subcommand};
use fallow_config::{FallowConfig, OutputFormat};
use fallow_core::results::AnalysisResults;

mod report;

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

    /// Path to fallow.toml configuration file
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Output format
    #[arg(short, long, global = true, default_value = "human")]
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

    /// Exit with code 1 if issues are found
    #[arg(long, global = true)]
    fail_on_issues: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Run dead code analysis (default)
    Check {
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
    },

    /// Initialize a fallow.toml configuration file
    Init,

    /// List discovered entry points and files
    List {
        /// Show entry points
        #[arg(long)]
        entry_points: bool,

        /// Show all discovered files
        #[arg(long)]
        files: bool,

        /// Show detected frameworks
        #[arg(long)]
        frameworks: bool,
    },
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

// ── Main ─────────────────────────────────────────────────────────

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Set up tracing
    if !cli.quiet {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive(tracing::Level::INFO.into()),
            )
            .with_target(false)
            .with_timer(tracing_subscriber::fmt::time::uptime())
            .init();
    }

    let root = cli
        .root
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    let threads = cli.threads.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    });

    match cli.command.unwrap_or(Command::Check {
        unused_files: false,
        unused_exports: false,
        unused_deps: false,
        unused_types: false,
    }) {
        Command::Check {
            unused_files,
            unused_exports,
            unused_deps,
            unused_types,
        } => {
            run_check(
                &root,
                &cli.config,
                cli.format.into(),
                cli.no_cache,
                threads,
                cli.quiet,
                cli.fail_on_issues,
                unused_files,
                unused_exports,
                unused_deps,
                unused_types,
            )
        }
        Command::Init => run_init(&root),
        Command::List {
            entry_points,
            files,
            frameworks,
        } => run_list(&root, &cli.config, threads, entry_points, files, frameworks),
    }
}

fn run_check(
    root: &PathBuf,
    config_path: &Option<PathBuf>,
    output: OutputFormat,
    no_cache: bool,
    threads: usize,
    quiet: bool,
    fail_on_issues: bool,
    _only_files: bool,
    _only_exports: bool,
    _only_deps: bool,
    _only_types: bool,
) -> ExitCode {
    let start = Instant::now();

    let config = load_config(root, config_path, output, no_cache, threads);

    let results = fallow_core::analyze(&config);
    let elapsed = start.elapsed();

    report::print_results(&results, &config, elapsed, quiet);

    if fail_on_issues && results.has_issues() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn run_init(root: &PathBuf) -> ExitCode {
    let config_path = root.join("fallow.toml");
    if config_path.exists() {
        eprintln!("fallow.toml already exists");
        return ExitCode::from(2);
    }

    let default_config = r#"# fallow.toml - Dead code analysis configuration
# See https://github.com/nicholasgasior/fallow for documentation

# Additional entry points (beyond auto-detected ones)
# entry = ["src/workers/*.ts"]

# Patterns to ignore
# ignore = ["**/*.generated.ts"]

# Dependencies to ignore (always considered used)
# ignore_dependencies = ["autoprefixer"]

[detect]
unused_files = true
unused_exports = true
unused_dependencies = true
unused_dev_dependencies = true
unused_types = true
"#;

    std::fs::write(&config_path, default_config).expect("Failed to write fallow.toml");
    eprintln!("Created fallow.toml");
    ExitCode::SUCCESS
}

fn run_list(
    root: &PathBuf,
    config_path: &Option<PathBuf>,
    threads: usize,
    entry_points: bool,
    files: bool,
    frameworks: bool,
) -> ExitCode {
    let config = load_config(
        root,
        config_path,
        OutputFormat::Human,
        true,
        threads,
    );

    if frameworks || (!entry_points && !files) {
        eprintln!("Detected frameworks:");
        for rule in &config.framework_rules {
            eprintln!("  - {}", rule.name);
        }
    }

    if files || (!entry_points && !frameworks) {
        let discovered = fallow_core::discover::discover_files(&config);
        eprintln!("Discovered {} files", discovered.len());
        for file in &discovered {
            println!("{}", file.path.display());
        }
    }

    if entry_points || (!files && !frameworks) {
        let discovered = fallow_core::discover::discover_files(&config);
        let entries = fallow_core::discover::discover_entry_points(&config, &discovered);
        eprintln!("Found {} entry points", entries.len());
        for ep in &entries {
            println!("{} ({:?})", ep.path.display(), ep.source);
        }
    }

    ExitCode::SUCCESS
}

fn load_config(
    root: &PathBuf,
    config_path: &Option<PathBuf>,
    output: OutputFormat,
    no_cache: bool,
    threads: usize,
) -> fallow_config::ResolvedConfig {
    let user_config = if let Some(path) = config_path {
        FallowConfig::load(path).ok()
    } else {
        FallowConfig::find_and_load(root).map(|(c, _)| c)
    };

    match user_config {
        Some(mut config) => {
            config.output = output;
            config.resolve(root.clone(), threads, no_cache)
        }
        None => FallowConfig {
            root: None,
            entry: vec![],
            ignore: vec![],
            detect: fallow_config::DetectConfig::default(),
            frameworks: None,
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            output,
        }
        .resolve(root.clone(), threads, no_cache),
    }
}
