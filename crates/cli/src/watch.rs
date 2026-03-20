use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use fallow_config::OutputFormat;
use notify_debouncer_mini::{DebouncedEventKind, new_debouncer};

use crate::load_config;
use crate::report;

#[allow(clippy::ref_option, clippy::needless_pass_by_value)] // matches load_config signature
pub fn run_watch(
    root: &Path,
    config_path: &Option<PathBuf>,
    output: OutputFormat,
    no_cache: bool,
    threads: usize,
    quiet: bool,
    production: bool,
) -> ExitCode {
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
                    let Ok(config) = load_config(
                        root,
                        config_path,
                        output.clone(),
                        no_cache,
                        threads,
                        production,
                    ) else {
                        eprintln!("Warning: failed to reload config, using previous configuration");
                        continue;
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
