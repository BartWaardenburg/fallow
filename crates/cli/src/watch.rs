use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use colored::Colorize;
use fallow_config::OutputFormat;
use notify_debouncer_mini::{DebouncedEventKind, new_debouncer};
use rustc_hash::FxHashSet;

use crate::load_config;
use crate::report;

/// ANSI escape: clear screen + scrollback + move cursor home (same sequence as tsc --watch).
const CLEAR_SCREEN: &str = "\x1B[2J\x1B[3J\x1B[H";

pub struct WatchOptions<'a> {
    pub root: &'a Path,
    pub config_path: &'a Option<PathBuf>,
    pub output: OutputFormat,
    pub no_cache: bool,
    pub threads: usize,
    pub quiet: bool,
    pub production: bool,
    pub clear_screen: bool,
}

fn is_relevant_source(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| fallow_core::discover::SOURCE_EXTENSIONS.contains(&ext))
}

fn is_relevant_config(path: &Path) -> bool {
    path.file_name()
        .and_then(|s| s.to_str())
        .is_some_and(|name| {
            matches!(
                name,
                "package.json"
                    | ".fallowrc.json"
                    | "fallow.toml"
                    | ".fallow.toml"
                    | "tsconfig.json"
            )
        })
}

/// Collect changed file paths from debounced events, deduplicating and stripping the root prefix.
fn collect_changed_paths(
    events: &[notify_debouncer_mini::DebouncedEvent],
    root: &Path,
) -> Vec<String> {
    let mut seen = FxHashSet::default();
    let mut paths = Vec::new();
    for event in events {
        if !matches!(event.kind, DebouncedEventKind::Any) {
            continue;
        }
        if !is_relevant_source(&event.path) && !is_relevant_config(&event.path) {
            continue;
        }
        let display = event
            .path
            .strip_prefix(root)
            .unwrap_or(&event.path)
            .display()
            .to_string();
        if seen.insert(display.clone()) {
            paths.push(display);
        }
    }
    paths
}

fn print_waiting() {
    eprintln!(
        "\n{}",
        "Watching for changes... (press Ctrl+C to stop)".dimmed()
    );
}

pub fn run_watch(opts: &WatchOptions<'_>) -> ExitCode {
    use std::sync::mpsc;
    use std::time::Duration;

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
    let report_code = report::print_results(&results, &config, elapsed, opts.quiet);
    if report_code != ExitCode::SUCCESS {
        eprintln!("Warning: report output failed");
    }
    print_waiting();

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
        .watch(opts.root.as_ref(), notify::RecursiveMode::Recursive)
    {
        eprintln!("Failed to watch directory: {e}");
        return ExitCode::from(2);
    }

    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                let changed = collect_changed_paths(&events, opts.root);
                if changed.is_empty() {
                    continue;
                }

                if opts.clear_screen && std::io::stderr().is_terminal() {
                    eprint!("{CLEAR_SCREEN}");
                }

                // Show which files changed
                for path in &changed {
                    eprintln!("{} {path}", "Changed:".dimmed());
                }
                eprintln!();

                let Ok(config) = load_config(
                    opts.root,
                    opts.config_path,
                    opts.output.clone(),
                    opts.no_cache,
                    opts.threads,
                    opts.production,
                    opts.quiet,
                ) else {
                    eprintln!("Warning: failed to reload config, using previous configuration");
                    continue;
                };
                let start = Instant::now();
                match fallow_core::analyze(&config) {
                    Ok(results) => {
                        let elapsed = start.elapsed();
                        let report_code =
                            report::print_results(&results, &config, elapsed, opts.quiet);
                        if report_code != ExitCode::SUCCESS {
                            eprintln!("Warning: report output failed");
                        }
                    }
                    Err(e) => {
                        eprintln!("Analysis error: {e}");
                    }
                }
                print_waiting();
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
