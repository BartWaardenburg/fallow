use std::process::ExitCode;

use fallow_config::{OutputFormat, ResolvedConfig};
use fallow_core::graph::ModuleGraph;

use super::TraceOptions;
use crate::{emit_error, report};

// ── Trace output ─────────────────────────────────────────────────

/// Handle `--trace`, `--trace-file`, `--trace-dependency` early returns.
///
/// Returns `Some(code)` if a trace was handled (caller should return),
/// `None` if no trace was active and control should continue.
pub(super) fn handle_trace_output(
    graph: &ModuleGraph,
    trace_opts: &TraceOptions,
    root: &std::path::Path,
    output: &OutputFormat,
) -> Option<ExitCode> {
    if let Some(ref trace_spec) = trace_opts.trace_export {
        let Some((file_path, export_name)) = trace_spec.rsplit_once(':') else {
            return Some(emit_error(
                "--trace requires FILE:EXPORT_NAME format (e.g., src/utils.ts:foo)",
                2,
                output,
            ));
        };
        match fallow_core::trace::trace_export(graph, root, file_path, export_name) {
            Some(trace) => {
                report::print_export_trace(&trace, output);
                return Some(ExitCode::SUCCESS);
            }
            None => {
                return Some(emit_error(
                    &format!("export '{export_name}' not found in '{file_path}'"),
                    2,
                    output,
                ));
            }
        }
    }

    if let Some(ref file_path) = trace_opts.trace_file {
        match fallow_core::trace::trace_file(graph, root, file_path) {
            Some(trace) => {
                report::print_file_trace(&trace, output);
                return Some(ExitCode::SUCCESS);
            }
            None => {
                return Some(emit_error(
                    &format!("file '{file_path}' not found in module graph"),
                    2,
                    output,
                ));
            }
        }
    }

    if let Some(ref pkg_name) = trace_opts.trace_dependency {
        let trace = fallow_core::trace::trace_dependency(graph, root, pkg_name);
        report::print_dependency_trace(&trace, output);
        return Some(ExitCode::SUCCESS);
    }

    None
}

// ── SARIF output ─────────────────────────────────────────────────

/// Write SARIF output to a file if `--sarif-file` was specified.
pub fn write_sarif_file(
    results: &fallow_core::results::AnalysisResults,
    config: &ResolvedConfig,
    sarif_path: &std::path::Path,
    quiet: bool,
) {
    let sarif = report::build_sarif(results, &config.root, &config.rules);
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

// ── Cross-reference output ───────────────────────────────────────

/// Run duplication cross-reference and print combined findings.
pub fn run_cross_reference(
    config: &ResolvedConfig,
    unfiltered_results: &fallow_core::results::AnalysisResults,
    quiet: bool,
) {
    let files = fallow_core::discover::discover_files(config);
    let dupe_report =
        fallow_core::duplicates::find_duplicates(&config.root, &files, &config.duplicates);
    let cross_ref = fallow_core::cross_reference::cross_reference(&dupe_report, unfiltered_results);

    if cross_ref.has_findings() {
        report::print_cross_reference_findings(&cross_ref, &config.root, quiet, &config.output);
    }
}
