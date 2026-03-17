use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use clap::{Parser, Subcommand};
use fallow_config::{FallowConfig, OutputFormat};

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

    /// Only report issues in files changed since this git ref (e.g., main, HEAD~5)
    #[arg(long, global = true)]
    changed_since: Option<String>,

    /// Compare against a previously saved baseline file
    #[arg(long, global = true)]
    baseline: Option<PathBuf>,

    /// Save the current results as a baseline file
    #[arg(long, global = true)]
    save_baseline: Option<PathBuf>,
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

    /// Watch for changes and re-run analysis
    Watch,

    /// Auto-fix issues (remove unused exports, dependencies)
    Fix {
        /// Dry run — show what would be changed without modifying files
        #[arg(long)]
        dry_run: bool,
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
        } => run_check(
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
            cli.changed_since.as_deref(),
            cli.baseline.as_deref(),
            cli.save_baseline.as_deref(),
        ),
        Command::Watch => run_watch(
            &root,
            &cli.config,
            cli.format.into(),
            cli.no_cache,
            threads,
            cli.quiet,
        ),
        Command::Fix { dry_run } => run_fix(
            &root,
            &cli.config,
            cli.no_cache,
            threads,
            cli.quiet,
            dry_run,
        ),
        Command::Init => run_init(&root),
        Command::List {
            entry_points,
            files,
            frameworks,
        } => run_list(&root, &cli.config, threads, entry_points, files, frameworks),
    }
}

#[allow(clippy::too_many_arguments)]
fn run_check(
    root: &std::path::Path,
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
    changed_since: Option<&str>,
    baseline: Option<&std::path::Path>,
    save_baseline: Option<&std::path::Path>,
) -> ExitCode {
    let start = Instant::now();

    let config = load_config(root, config_path, output, no_cache, threads);

    // Get changed files if --changed-since is set
    let changed_files: Option<std::collections::HashSet<std::path::PathBuf>> =
        changed_since.and_then(|git_ref| get_changed_files(root, git_ref));

    let mut results = fallow_core::analyze(&config);
    let elapsed = start.elapsed();

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

    report::print_results(&results, &config, elapsed, quiet);

    if fail_on_issues && results.has_issues() {
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
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", git_ref])
        .current_dir(root)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let files: std::collections::HashSet<std::path::PathBuf> =
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|line| root.join(line))
            .collect();

    Some(files)
}

/// Baseline data for comparison.
#[derive(serde::Serialize, serde::Deserialize)]
struct BaselineData {
    unused_files: Vec<String>,
    unused_exports: Vec<String>,
    unused_types: Vec<String>,
    unused_deps: Vec<String>,
    unused_dev_deps: Vec<String>,
}

impl BaselineData {
    fn from_results(results: &fallow_core::results::AnalysisResults) -> Self {
        Self {
            unused_files: results
                .unused_files
                .iter()
                .map(|f| f.path.to_string_lossy().to_string())
                .collect(),
            unused_exports: results
                .unused_exports
                .iter()
                .map(|e| format!("{}:{}", e.path.display(), e.export_name))
                .collect(),
            unused_types: results
                .unused_types
                .iter()
                .map(|e| format!("{}:{}", e.path.display(), e.export_name))
                .collect(),
            unused_deps: results
                .unused_dependencies
                .iter()
                .map(|d| d.package_name.clone())
                .collect(),
            unused_dev_deps: results
                .unused_dev_dependencies
                .iter()
                .map(|d| d.package_name.clone())
                .collect(),
        }
    }
}

/// Filter results to only include issues not present in the baseline.
fn filter_new_issues(
    mut results: fallow_core::results::AnalysisResults,
    baseline: &BaselineData,
) -> fallow_core::results::AnalysisResults {
    results.unused_files.retain(|f| {
        !baseline
            .unused_files
            .contains(&f.path.to_string_lossy().to_string())
    });
    results.unused_exports.retain(|e| {
        !baseline
            .unused_exports
            .contains(&format!("{}:{}", e.path.display(), e.export_name))
    });
    results.unused_types.retain(|e| {
        !baseline
            .unused_types
            .contains(&format!("{}:{}", e.path.display(), e.export_name))
    });
    results
        .unused_dependencies
        .retain(|d| !baseline.unused_deps.contains(&d.package_name));
    results
        .unused_dev_dependencies
        .retain(|d| !baseline.unused_dev_deps.contains(&d.package_name));
    results
}

fn run_watch(
    root: &PathBuf,
    config_path: &Option<PathBuf>,
    output: OutputFormat,
    no_cache: bool,
    threads: usize,
    quiet: bool,
) -> ExitCode {
    use notify_debouncer_mini::{DebouncedEventKind, new_debouncer};
    use std::sync::mpsc;
    use std::time::Duration;

    let config = load_config(root, config_path, output.clone(), no_cache, threads);

    eprintln!("Watching for changes... (press Ctrl+C to stop)");

    // Run initial analysis
    let start = Instant::now();
    let results = fallow_core::analyze(&config);
    let elapsed = start.elapsed();
    report::print_results(&results, &config, elapsed, quiet);

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
                    let config = load_config(root, config_path, output.clone(), no_cache, threads);
                    let start = Instant::now();
                    let results = fallow_core::analyze(&config);
                    let elapsed = start.elapsed();
                    report::print_results(&results, &config, elapsed, quiet);
                }
            }
            Ok(Err(e)) => {
                eprintln!("Watch error: {e:?}");
            }
            Err(e) => {
                eprintln!("Channel error: {e}");
                break;
            }
        }
    }

    ExitCode::SUCCESS
}

fn run_fix(
    root: &PathBuf,
    config_path: &Option<PathBuf>,
    no_cache: bool,
    threads: usize,
    quiet: bool,
    dry_run: bool,
) -> ExitCode {
    let config = load_config(root, config_path, OutputFormat::Human, no_cache, threads);

    let results = fallow_core::analyze(&config);

    if results.total_issues() == 0 {
        if !quiet {
            eprintln!("No issues to fix.");
        }
        return ExitCode::SUCCESS;
    }

    let mut fixed_count = 0;

    // Fix unused exports: remove the `export` keyword
    for export in &results.unused_exports {
        let path = &export.path;
        if let Ok(content) = std::fs::read_to_string(path) {
            let lines: Vec<&str> = content.lines().collect();
            // Find the line containing this export by span offset
            let byte_offset = export.line as usize;
            let mut current_offset = 0;
            let mut target_line = None;
            for (i, line) in lines.iter().enumerate() {
                if current_offset + line.len() >= byte_offset {
                    target_line = Some(i);
                    break;
                }
                current_offset += line.len() + 1; // +1 for newline
            }

            if let Some(line_idx) = target_line {
                let line = lines[line_idx];
                // Simple fix: remove "export " prefix
                if line.trim_start().starts_with("export ") {
                    let indent = line.len() - line.trim_start().len();
                    let new_line = format!(
                        "{}{}",
                        &line[..indent],
                        line.trim_start()
                            .strip_prefix("export ")
                            .unwrap_or(line.trim_start())
                    );

                    if dry_run {
                        let relative = path.strip_prefix(root).unwrap_or(path);
                        eprintln!(
                            "Would remove export from {}:{} `{}`",
                            relative.display(),
                            line_idx + 1,
                            export.export_name
                        );
                    } else {
                        let mut new_lines: Vec<String> =
                            lines.iter().map(|l| l.to_string()).collect();
                        new_lines[line_idx] = new_line;
                        let new_content = new_lines.join("\n");
                        if std::fs::write(path, new_content).is_ok() {
                            fixed_count += 1;
                        }
                    }
                }
            }
        }
    }

    // Fix unused dependencies: remove from package.json
    if !results.unused_dependencies.is_empty() || !results.unused_dev_dependencies.is_empty() {
        let pkg_path = root.join("package.json");
        if let Ok(content) = std::fs::read_to_string(&pkg_path)
            && let Ok(mut pkg_value) = serde_json::from_str::<serde_json::Value>(&content)
        {
            let mut changed = false;

            for dep in &results.unused_dependencies {
                if let Some(deps) = pkg_value.get_mut("dependencies")
                    && let Some(obj) = deps.as_object_mut()
                    && obj.remove(&dep.package_name).is_some()
                {
                    if dry_run {
                        eprintln!("Would remove `{}` from dependencies", dep.package_name);
                    } else {
                        changed = true;
                        fixed_count += 1;
                    }
                }
            }

            for dep in &results.unused_dev_dependencies {
                if let Some(deps) = pkg_value.get_mut("devDependencies")
                    && let Some(obj) = deps.as_object_mut()
                    && obj.remove(&dep.package_name).is_some()
                {
                    if dry_run {
                        eprintln!("Would remove `{}` from devDependencies", dep.package_name);
                    } else {
                        changed = true;
                        fixed_count += 1;
                    }
                }
            }

            if changed
                && !dry_run
                && let Ok(new_json) = serde_json::to_string_pretty(&pkg_value)
            {
                let _ = std::fs::write(&pkg_path, new_json + "\n");
            }
        }
    }

    if !quiet {
        if dry_run {
            eprintln!("Dry run complete. No files were modified.");
        } else {
            eprintln!("Fixed {} issue(s).", fixed_count);
        }
    }

    ExitCode::SUCCESS
}

fn run_init(root: &std::path::Path) -> ExitCode {
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
    root: &std::path::Path,
    config_path: &Option<PathBuf>,
    threads: usize,
    entry_points: bool,
    files: bool,
    frameworks: bool,
) -> ExitCode {
    let config = load_config(root, config_path, OutputFormat::Human, true, threads);

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
    root: &std::path::Path,
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
            config.resolve(root.to_path_buf(), threads, no_cache)
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
        .resolve(root.to_path_buf(), threads, no_cache),
    }
}
