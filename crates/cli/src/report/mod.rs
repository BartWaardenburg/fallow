mod compact;
mod human;
mod json;
mod markdown;
mod sarif;

use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use fallow_config::{OutputFormat, ResolvedConfig, Severity};
use fallow_core::duplicates::DuplicationReport;
use fallow_core::results::AnalysisResults;
use fallow_core::trace::{CloneTrace, DependencyTrace, ExportTrace, FileTrace, PipelineTimings};

/// Strip the project root prefix from a path for display, falling back to the full path.
fn relative_path<'a>(path: &'a Path, root: &Path) -> &'a Path {
    path.strip_prefix(root).unwrap_or(path)
}

/// Compute a SARIF-compatible relative URI from an absolute path and project root.
fn relative_uri(path: &Path, root: &Path) -> String {
    normalize_uri(&relative_path(path, root).display().to_string())
}

/// Normalize a path string to a valid URI: forward slashes and percent-encoded brackets.
///
/// Brackets (`[`, `]`) are not valid in URI path segments per RFC 3986 and cause
/// SARIF validation warnings (e.g., Next.js dynamic routes like `[slug]`).
pub fn normalize_uri(path_str: &str) -> String {
    path_str
        .replace('\\', "/")
        .replace('[', "%5B")
        .replace(']', "%5D")
}

/// Severity level for human-readable output.
#[derive(Clone, Copy)]
enum Level {
    Warn,
    Info,
    Error,
}

const fn severity_to_level(s: Severity) -> Level {
    match s {
        Severity::Error => Level::Error,
        Severity::Warn => Level::Warn,
        // Off issues are filtered before reporting; fall back to Info.
        Severity::Off => Level::Info,
    }
}

/// Print analysis results in the configured format.
/// Returns exit code 2 if serialization fails, SUCCESS otherwise.
pub fn print_results(
    results: &AnalysisResults,
    config: &ResolvedConfig,
    elapsed: Duration,
    quiet: bool,
) -> ExitCode {
    match config.output {
        OutputFormat::Human => {
            human::print_human(results, &config.root, &config.rules, elapsed, quiet);
            ExitCode::SUCCESS
        }
        OutputFormat::Json => json::print_json(results, &config.root, elapsed),
        OutputFormat::Compact => {
            compact::print_compact(results, &config.root);
            ExitCode::SUCCESS
        }
        OutputFormat::Sarif => sarif::print_sarif(results, &config.root, &config.rules),
        OutputFormat::Markdown => {
            markdown::print_markdown(results, &config.root);
            ExitCode::SUCCESS
        }
    }
}

// ── Duplication report ────────────────────────────────────────────

/// Print duplication analysis results in the configured format.
pub fn print_duplication_report(
    report: &DuplicationReport,
    config: &ResolvedConfig,
    elapsed: Duration,
    quiet: bool,
    output: &OutputFormat,
) -> ExitCode {
    match output {
        OutputFormat::Human => {
            human::print_duplication_human(report, &config.root, elapsed, quiet);
            ExitCode::SUCCESS
        }
        OutputFormat::Json => json::print_duplication_json(report, elapsed),
        OutputFormat::Compact => {
            compact::print_duplication_compact(report, &config.root);
            ExitCode::SUCCESS
        }
        OutputFormat::Sarif => sarif::print_duplication_sarif(report, &config.root),
        OutputFormat::Markdown => {
            markdown::print_duplication_markdown(report, &config.root);
            ExitCode::SUCCESS
        }
    }
}

// ── Health / complexity report ─────────────────────────────────────

/// Print health (complexity) analysis results in the configured format.
pub fn print_health_report(
    report: &crate::health_types::HealthReport,
    config: &ResolvedConfig,
    elapsed: Duration,
    quiet: bool,
    output: &OutputFormat,
) -> ExitCode {
    match output {
        OutputFormat::Human => {
            human::print_health_human(report, &config.root, elapsed, quiet);
            ExitCode::SUCCESS
        }
        OutputFormat::Compact => {
            compact::print_health_compact(report, &config.root);
            ExitCode::SUCCESS
        }
        OutputFormat::Markdown => {
            markdown::print_health_markdown(report, &config.root);
            ExitCode::SUCCESS
        }
        OutputFormat::Sarif => sarif::print_health_sarif(report, &config.root),
        OutputFormat::Json => json::print_health_json(report, &config.root, elapsed),
    }
}

/// Print cross-reference findings (duplicated code that is also dead code).
///
/// Only emits output in human format to avoid corrupting structured JSON/SARIF output.
pub fn print_cross_reference_findings(
    cross_ref: &fallow_core::cross_reference::CrossReferenceResult,
    root: &Path,
    quiet: bool,
    output: &OutputFormat,
) {
    human::print_cross_reference_findings(cross_ref, root, quiet, output);
}

// ── Trace output ──────────────────────────────────────────────────

/// Print export trace results.
pub fn print_export_trace(trace: &ExportTrace, format: &OutputFormat) {
    match format {
        OutputFormat::Json => json::print_trace_json(trace),
        _ => human::print_export_trace_human(trace),
    }
}

/// Print file trace results.
pub fn print_file_trace(trace: &FileTrace, format: &OutputFormat) {
    match format {
        OutputFormat::Json => json::print_trace_json(trace),
        _ => human::print_file_trace_human(trace),
    }
}

/// Print dependency trace results.
pub fn print_dependency_trace(trace: &DependencyTrace, format: &OutputFormat) {
    match format {
        OutputFormat::Json => json::print_trace_json(trace),
        _ => human::print_dependency_trace_human(trace),
    }
}

/// Print clone trace results.
pub fn print_clone_trace(trace: &CloneTrace, root: &Path, format: &OutputFormat) {
    match format {
        OutputFormat::Json => json::print_trace_json(trace),
        _ => human::print_clone_trace_human(trace, root),
    }
}

/// Print pipeline performance timings.
/// In JSON mode, outputs to stderr to avoid polluting the JSON analysis output on stdout.
pub fn print_performance(timings: &PipelineTimings, format: &OutputFormat) {
    match format {
        OutputFormat::Json => match serde_json::to_string_pretty(timings) {
            Ok(json) => eprintln!("{json}"),
            Err(e) => eprintln!("Error: failed to serialize timings: {e}"),
        },
        _ => human::print_performance_human(timings),
    }
}

// Re-exported for snapshot testing via the lib target
#[allow(unused_imports)]
pub use compact::build_compact_lines;
#[allow(unused_imports)]
pub use json::build_json;
#[allow(unused_imports)]
pub use markdown::build_duplication_markdown;
#[allow(unused_imports)]
pub use markdown::build_health_markdown;
#[allow(unused_imports)]
pub use markdown::build_markdown;
#[allow(unused_imports)]
pub use sarif::build_health_sarif;
#[allow(unused_imports)]
pub use sarif::build_sarif;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── normalize_uri ────────────────────────────────────────────────

    #[test]
    fn normalize_uri_forward_slashes_unchanged() {
        assert_eq!(normalize_uri("src/utils.ts"), "src/utils.ts");
    }

    #[test]
    fn normalize_uri_backslashes_replaced() {
        assert_eq!(normalize_uri("src\\utils\\index.ts"), "src/utils/index.ts");
    }

    #[test]
    fn normalize_uri_mixed_slashes() {
        assert_eq!(normalize_uri("src\\utils/index.ts"), "src/utils/index.ts");
    }

    #[test]
    fn normalize_uri_path_with_spaces() {
        assert_eq!(
            normalize_uri("src\\my folder\\file.ts"),
            "src/my folder/file.ts"
        );
    }

    #[test]
    fn normalize_uri_empty_string() {
        assert_eq!(normalize_uri(""), "");
    }

    // ── relative_path ────────────────────────────────────────────────

    #[test]
    fn relative_path_strips_root_prefix() {
        let root = Path::new("/project");
        let path = Path::new("/project/src/utils.ts");
        assert_eq!(relative_path(path, root), Path::new("src/utils.ts"));
    }

    #[test]
    fn relative_path_returns_full_path_when_no_prefix() {
        let root = Path::new("/other");
        let path = Path::new("/project/src/utils.ts");
        assert_eq!(relative_path(path, root), path);
    }

    #[test]
    fn relative_path_at_root_returns_empty_or_file() {
        let root = Path::new("/project");
        let path = Path::new("/project/file.ts");
        assert_eq!(relative_path(path, root), Path::new("file.ts"));
    }

    #[test]
    fn relative_path_deeply_nested() {
        let root = Path::new("/project");
        let path = Path::new("/project/packages/ui/src/components/Button.tsx");
        assert_eq!(
            relative_path(path, root),
            Path::new("packages/ui/src/components/Button.tsx")
        );
    }

    // ── relative_uri ─────────────────────────────────────────────────

    #[test]
    fn relative_uri_produces_forward_slash_path() {
        let root = PathBuf::from("/project");
        let path = root.join("src").join("utils.ts");
        let uri = relative_uri(&path, &root);
        assert_eq!(uri, "src/utils.ts");
    }

    #[test]
    fn relative_uri_encodes_brackets() {
        let root = PathBuf::from("/project");
        let path = root.join("src/app/[...slug]/page.tsx");
        let uri = relative_uri(&path, &root);
        assert_eq!(uri, "src/app/%5B...slug%5D/page.tsx");
    }

    #[test]
    fn relative_uri_encodes_nested_dynamic_routes() {
        let root = PathBuf::from("/project");
        let path = root.join("src/app/[slug]/[id]/page.tsx");
        let uri = relative_uri(&path, &root);
        assert_eq!(uri, "src/app/%5Bslug%5D/%5Bid%5D/page.tsx");
    }

    #[test]
    fn relative_uri_no_common_prefix_returns_full() {
        let root = PathBuf::from("/other");
        let path = PathBuf::from("/project/src/utils.ts");
        let uri = relative_uri(&path, &root);
        assert!(uri.contains("project"));
        assert!(uri.contains("utils.ts"));
    }

    // ── severity_to_level ────────────────────────────────────────────

    #[test]
    fn severity_error_maps_to_level_error() {
        assert!(matches!(severity_to_level(Severity::Error), Level::Error));
    }

    #[test]
    fn severity_warn_maps_to_level_warn() {
        assert!(matches!(severity_to_level(Severity::Warn), Level::Warn));
    }

    #[test]
    fn severity_off_maps_to_level_info() {
        assert!(matches!(severity_to_level(Severity::Off), Level::Info));
    }
}
