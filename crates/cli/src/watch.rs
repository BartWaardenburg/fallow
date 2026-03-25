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
    pub explain: bool,
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

#[cfg(test)]
mod tests {
    use super::*;
    use notify_debouncer_mini::{DebouncedEvent, DebouncedEventKind};

    // ── is_relevant_source ───────────────────────────────────────────

    #[test]
    fn relevant_source_ts_extensions() {
        assert!(is_relevant_source(Path::new("src/index.ts")));
        assert!(is_relevant_source(Path::new("app.tsx")));
        assert!(is_relevant_source(Path::new("lib/utils.mts")));
        assert!(is_relevant_source(Path::new("lib/utils.cts")));
    }

    #[test]
    fn relevant_source_js_extensions() {
        assert!(is_relevant_source(Path::new("src/index.js")));
        assert!(is_relevant_source(Path::new("app.jsx")));
        assert!(is_relevant_source(Path::new("lib/utils.mjs")));
        assert!(is_relevant_source(Path::new("lib/utils.cjs")));
    }

    #[test]
    fn relevant_source_framework_extensions() {
        assert!(is_relevant_source(Path::new("App.vue")));
        assert!(is_relevant_source(Path::new("Page.svelte")));
        assert!(is_relevant_source(Path::new("page.astro")));
        assert!(is_relevant_source(Path::new("doc.mdx")));
    }

    #[test]
    fn relevant_source_style_extensions() {
        assert!(is_relevant_source(Path::new("styles.css")));
        assert!(is_relevant_source(Path::new("theme.scss")));
    }

    #[test]
    fn not_relevant_source() {
        assert!(!is_relevant_source(Path::new("README.md")));
        assert!(!is_relevant_source(Path::new("image.png")));
        assert!(!is_relevant_source(Path::new("data.json")));
        assert!(!is_relevant_source(Path::new("script.py")));
        assert!(!is_relevant_source(Path::new("Cargo.toml")));
        assert!(!is_relevant_source(Path::new("no_extension")));
    }

    // ── is_relevant_config ───────────────────────────────────────────

    #[test]
    fn relevant_config_files() {
        assert!(is_relevant_config(Path::new("package.json")));
        assert!(is_relevant_config(Path::new("/project/package.json")));
        assert!(is_relevant_config(Path::new(".fallowrc.json")));
        assert!(is_relevant_config(Path::new("fallow.toml")));
        assert!(is_relevant_config(Path::new(".fallow.toml")));
        assert!(is_relevant_config(Path::new("tsconfig.json")));
    }

    #[test]
    fn not_relevant_config() {
        assert!(!is_relevant_config(Path::new("eslint.config.js")));
        assert!(!is_relevant_config(Path::new("jest.config.ts")));
        assert!(!is_relevant_config(Path::new("package-lock.json")));
        assert!(!is_relevant_config(Path::new("tsconfig.build.json")));
        assert!(!is_relevant_config(Path::new("README.md")));
    }

    // ── collect_changed_paths ────────────────────────────────────────

    fn make_event(path: &str, kind: DebouncedEventKind) -> DebouncedEvent {
        DebouncedEvent {
            path: PathBuf::from(path),
            kind,
        }
    }

    #[test]
    fn collect_changed_paths_filters_non_source() {
        let root = PathBuf::from("/project");
        let events = vec![
            make_event("/project/src/index.ts", DebouncedEventKind::Any),
            make_event("/project/README.md", DebouncedEventKind::Any),
            make_event("/project/image.png", DebouncedEventKind::Any),
        ];
        let paths = collect_changed_paths(&events, &root);
        assert_eq!(paths, vec!["src/index.ts"]);
    }

    #[test]
    fn collect_changed_paths_includes_config() {
        let root = PathBuf::from("/project");
        let events = vec![
            make_event("/project/package.json", DebouncedEventKind::Any),
            make_event("/project/.fallowrc.json", DebouncedEventKind::Any),
        ];
        let paths = collect_changed_paths(&events, &root);
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&"package.json".to_string()));
        assert!(paths.contains(&".fallowrc.json".to_string()));
    }

    #[test]
    fn collect_changed_paths_deduplicates() {
        let root = PathBuf::from("/project");
        let events = vec![
            make_event("/project/src/index.ts", DebouncedEventKind::Any),
            make_event("/project/src/index.ts", DebouncedEventKind::Any),
            make_event("/project/src/index.ts", DebouncedEventKind::Any),
        ];
        let paths = collect_changed_paths(&events, &root);
        assert_eq!(paths, vec!["src/index.ts"]);
    }

    #[test]
    fn collect_changed_paths_ignores_non_any_events() {
        let root = PathBuf::from("/project");
        let events = vec![make_event(
            "/project/src/index.ts",
            DebouncedEventKind::AnyContinuous,
        )];
        let paths = collect_changed_paths(&events, &root);
        assert!(paths.is_empty());
    }

    #[test]
    fn collect_changed_paths_empty_events() {
        let root = PathBuf::from("/project");
        let paths = collect_changed_paths(&[], &root);
        assert!(paths.is_empty());
    }

    #[test]
    fn collect_changed_paths_strips_root_prefix() {
        let root = PathBuf::from("/project");
        let events = vec![make_event(
            "/project/src/deep/nested/file.tsx",
            DebouncedEventKind::Any,
        )];
        let paths = collect_changed_paths(&events, &root);
        assert_eq!(paths, vec!["src/deep/nested/file.tsx"]);
    }
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
    let report_code = report::print_results(&results, &config, elapsed, opts.quiet, opts.explain);
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
                        let report_code = report::print_results(
                            &results,
                            &config,
                            elapsed,
                            opts.quiet,
                            opts.explain,
                        );
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
