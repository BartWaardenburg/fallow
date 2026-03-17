use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use clap::{CommandFactory, Parser, Subcommand};
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

    /// Dump the CLI interface as machine-readable JSON for agent introspection
    Schema,
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

    // Handle schema before tracing setup (no side effects)
    if matches!(cli.command, Some(Command::Schema)) {
        return run_schema();
    }

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
        unused_files: false,
        unused_exports: false,
        unused_deps: false,
        unused_types: false,
        unused_enum_members: false,
        unused_class_members: false,
        unresolved_imports: false,
        unlisted_deps: false,
        duplicate_exports: false,
    }) {
        Command::Check {
            unused_files,
            unused_exports,
            unused_deps,
            unused_types,
            unused_enum_members,
            unused_class_members,
            unresolved_imports,
            unlisted_deps,
            duplicate_exports,
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
                cli.fail_on_issues,
                &filters,
                cli.changed_since.as_deref(),
                cli.baseline.as_deref(),
                cli.save_baseline.as_deref(),
            )
        }
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
            cli.format.into(),
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
        } => run_list(
            &root,
            &cli.config,
            cli.format.into(),
            threads,
            entry_points,
            files,
            frameworks,
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
) -> ExitCode {
    let start = Instant::now();

    let config = load_config(root, config_path, output, no_cache, threads);

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

    // Apply issue type filters
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
    let results = match fallow_core::analyze(&config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Analysis error: {e}");
            return ExitCode::from(2);
        }
    };
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
                    match fallow_core::analyze(&config) {
                        Ok(results) => {
                            let elapsed = start.elapsed();
                            report::print_results(&results, &config, elapsed, quiet);
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
                break;
            }
        }
    }

    ExitCode::SUCCESS
}

fn run_fix(
    root: &PathBuf,
    config_path: &Option<PathBuf>,
    output: OutputFormat,
    no_cache: bool,
    threads: usize,
    quiet: bool,
    dry_run: bool,
) -> ExitCode {
    let config = load_config(root, config_path, OutputFormat::Human, no_cache, threads);

    let results = match fallow_core::analyze(&config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Analysis error: {e}");
            return ExitCode::from(2);
        }
    };

    if results.total_issues() == 0 {
        if matches!(output, OutputFormat::Json) {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "dry_run": dry_run,
                    "fixes": [],
                    "total_fixed": 0
                }))
                .unwrap()
            );
        } else if !quiet {
            eprintln!("No issues to fix.");
        }
        return ExitCode::SUCCESS;
    }

    let mut fixes: Vec<serde_json::Value> = Vec::new();

    // Fix unused exports: remove the `export` keyword
    for export in &results.unused_exports {
        let path = &export.path;
        if let Ok(content) = std::fs::read_to_string(path) {
            let lines: Vec<&str> = content.lines().collect();
            // Find the line containing this export by span offset
            let byte_offset = export.span_start as usize;
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

                    let relative = path.strip_prefix(root).unwrap_or(path);

                    if dry_run {
                        if !matches!(output, OutputFormat::Json) {
                            eprintln!(
                                "Would remove export from {}:{} `{}`",
                                relative.display(),
                                line_idx + 1,
                                export.export_name
                            );
                        }
                        fixes.push(serde_json::json!({
                            "type": "remove_export",
                            "path": relative.display().to_string(),
                            "line": line_idx + 1,
                            "name": export.export_name,
                        }));
                    } else {
                        let mut new_lines: Vec<String> =
                            lines.iter().map(|l| l.to_string()).collect();
                        new_lines[line_idx] = new_line;
                        let new_content = new_lines.join("\n");
                        let success = std::fs::write(path, new_content).is_ok();
                        fixes.push(serde_json::json!({
                            "type": "remove_export",
                            "path": relative.display().to_string(),
                            "line": line_idx + 1,
                            "name": export.export_name,
                            "applied": success,
                        }));
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
                        if !matches!(output, OutputFormat::Json) {
                            eprintln!("Would remove `{}` from dependencies", dep.package_name);
                        }
                        fixes.push(serde_json::json!({
                            "type": "remove_dependency",
                            "package": dep.package_name,
                            "location": "dependencies",
                        }));
                    } else {
                        changed = true;
                        fixes.push(serde_json::json!({
                            "type": "remove_dependency",
                            "package": dep.package_name,
                            "location": "dependencies",
                            "applied": true,
                        }));
                    }
                }
            }

            for dep in &results.unused_dev_dependencies {
                if let Some(deps) = pkg_value.get_mut("devDependencies")
                    && let Some(obj) = deps.as_object_mut()
                    && obj.remove(&dep.package_name).is_some()
                {
                    if dry_run {
                        if !matches!(output, OutputFormat::Json) {
                            eprintln!("Would remove `{}` from devDependencies", dep.package_name);
                        }
                        fixes.push(serde_json::json!({
                            "type": "remove_dependency",
                            "package": dep.package_name,
                            "location": "devDependencies",
                        }));
                    } else {
                        changed = true;
                        fixes.push(serde_json::json!({
                            "type": "remove_dependency",
                            "package": dep.package_name,
                            "location": "devDependencies",
                            "applied": true,
                        }));
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

    if matches!(output, OutputFormat::Json) {
        let applied_count = fixes
            .iter()
            .filter(|f| f.get("applied").and_then(|v| v.as_bool()).unwrap_or(false))
            .count();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "dry_run": dry_run,
                "fixes": fixes,
                "total_fixed": applied_count
            }))
            .unwrap()
        );
    } else if !quiet {
        let fixed_count = fixes.len();
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
# See https://github.com/bartwaardenburg/fallow for documentation

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
    output: OutputFormat,
    threads: usize,
    entry_points: bool,
    files: bool,
    frameworks: bool,
) -> ExitCode {
    let config = load_config(root, config_path, OutputFormat::Human, true, threads);

    let show_all = !entry_points && !files && !frameworks;

    match output {
        OutputFormat::Json => {
            let mut result = serde_json::Map::new();

            if frameworks || show_all {
                let fw: Vec<serde_json::Value> = config
                    .framework_rules
                    .iter()
                    .map(|r| serde_json::json!({ "name": r.name }))
                    .collect();
                result.insert("frameworks".to_string(), serde_json::json!(fw));
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
                let entries = fallow_core::discover::discover_entry_points(&config, disc);
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

            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::Value::Object(result)).unwrap()
            );
        }
        _ => {
            if frameworks || show_all {
                eprintln!("Detected frameworks:");
                for rule in &config.framework_rules {
                    eprintln!("  - {}", rule.name);
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
                let entries = fallow_core::discover::discover_entry_points(&config, disc);
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
    println!(
        "{}",
        serde_json::to_string_pretty(&schema).expect("Failed to serialize schema")
    );
    ExitCode::SUCCESS
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
                "fixable": false
            },
            {
                "id": "unused-export",
                "description": "Export is never imported by other modules",
                "filter_flag": "--unused-exports",
                "fixable": true
            },
            {
                "id": "unused-type",
                "description": "Type export is never imported by other modules",
                "filter_flag": "--unused-types",
                "fixable": false
            },
            {
                "id": "unused-dependency",
                "description": "Package in dependencies is never imported",
                "filter_flag": "--unused-deps",
                "fixable": true,
                "note": "--unused-deps controls both unused-dependency and unused-dev-dependency"
            },
            {
                "id": "unused-dev-dependency",
                "description": "Package in devDependencies is never imported",
                "filter_flag": "--unused-deps",
                "fixable": true,
                "note": "--unused-deps controls both unused-dependency and unused-dev-dependency"
            },
            {
                "id": "unused-enum-member",
                "description": "Enum member is never referenced",
                "filter_flag": "--unused-enum-members",
                "fixable": false
            },
            {
                "id": "unused-class-member",
                "description": "Class member is never referenced",
                "filter_flag": "--unused-class-members",
                "fixable": false
            },
            {
                "id": "unresolved-import",
                "description": "Import specifier could not be resolved to a file",
                "filter_flag": "--unresolved-imports",
                "fixable": false
            },
            {
                "id": "unlisted-dependency",
                "description": "Package is imported but not in package.json",
                "filter_flag": "--unlisted-deps",
                "fixable": false
            },
            {
                "id": "duplicate-export",
                "description": "Same export name appears in multiple modules",
                "filter_flag": "--duplicate-exports",
                "fixable": false
            }
        ],
        "output_formats": ["human", "json", "sarif", "compact"],
        "exit_codes": {
            "0": "Success (or issues found without --fail-on-issues)",
            "1": "Issues found (with --fail-on-issues)",
            "2": "Error (invalid config, invalid input, etc.)"
        }
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
