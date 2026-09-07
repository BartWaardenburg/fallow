//! `fallow trace-error [FILE|-]`: resolve a runtime stack trace's frames
//! against the project graph.
//!
//! Its own surface (`kind: "trace-error"`, `schema_version: "1"`), like the
//! other trace shapes: never folded into the ranked brief and never an input to
//! the focus map. A trace nothing in it resolves is an ANSWER, not an error, so
//! it exits 0 and publishes the counts that say so; only unreadable input or a
//! failed analysis exits 2.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fallow_config::{OutputFormat, ProductionAnalysis};
use fallow_engine::trace_error::MAX_STACK_TRACE_BYTES;
use fallow_types::trace_error::{ErrorTrace, FrameResolution};

use crate::error::emit_error;
use crate::report;
use crate::report::sink::outln;
use crate::{ConfigLoadOptions, load_config_for_analysis};

/// The stdin sentinel, matching `--diff-file -`.
const STDIN_SENTINEL: &str = "-";

/// Options for `fallow trace-error`.
pub struct TraceErrorOptions<'a> {
    pub root: &'a Path,
    pub config_path: &'a Option<PathBuf>,
    pub output: OutputFormat,
    pub json_style: crate::json_style::JsonStyle,
    pub no_cache: bool,
    pub threads: usize,
    pub quiet: bool,
    pub allow_remote_extends: bool,
    /// The trace file, or `-` / `None` to read stdin.
    pub trace_file: Option<&'a str>,
}

/// Read the stack trace, resolve its frames, and emit the result.
pub fn run_trace_error(opts: &TraceErrorOptions<'_>) -> ExitCode {
    let (input, source) = match read_trace(opts.root, opts.trace_file) {
        Ok(pair) => pair,
        Err(message) => return emit_error(&message, 2, opts.output),
    };

    let config = match load_config_for_analysis(
        opts.root,
        opts.config_path,
        ConfigLoadOptions {
            output: opts.output,
            no_cache: opts.no_cache,
            threads: opts.threads,
            production_override: None,
            quiet: opts.quiet,
            allow_remote_extends: opts.allow_remote_extends,
        },
        ProductionAnalysis::DeadCode,
    ) {
        Ok(config) => config,
        Err(code) => return code,
    };

    let session = match fallow_engine::session::AnalysisSession::from_resolved_config(config) {
        Ok(session) => session,
        Err(err) => return emit_error(&format!("Analysis error: {err}"), 2, opts.output),
    };

    let trace = match fallow_engine::trace_error::trace_error_with_session(&session, &input, source)
    {
        Ok(trace) => trace,
        Err(err) => return emit_error(&format!("Analysis error: {err}"), 2, opts.output),
    };

    emit_trace_error(trace, opts)
}

/// Read the trace from a file or from stdin, returning the text and the label
/// the payload reports as its `source`.
///
/// A relative path is resolved against the project root, matching how
/// `--diff-file` resolves its input. The reported `source` keeps the caller's
/// own spelling rather than the resolved absolute path.
fn read_trace(root: &Path, trace_file: Option<&str>) -> Result<(String, String), String> {
    let path = match trace_file {
        None | Some(STDIN_SENTINEL) => {
            let mut buffer = Vec::new();
            std::io::stdin()
                .take(MAX_STACK_TRACE_BYTES + 1)
                .read_to_end(&mut buffer)
                .map_err(|err| format!("failed to read stack trace from stdin: {err}"))?;
            if buffer.len() as u64 > MAX_STACK_TRACE_BYTES {
                return Err(format!(
                    "stack trace from stdin exceeds the {MAX_STACK_TRACE_BYTES}-byte limit"
                ));
            }
            let text = String::from_utf8(buffer)
                .map_err(|_| "stack trace from stdin is not valid UTF-8".to_string())?;
            return Ok((text, "stdin".to_string()));
        }
        Some(path) => path,
    };

    let resolved = {
        let candidate = Path::new(path);
        if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            root.join(candidate)
        }
    };
    let metadata = std::fs::metadata(&resolved)
        .map_err(|err| format!("failed to read stack trace from '{path}': {err}"))?;
    if metadata.len() > MAX_STACK_TRACE_BYTES {
        return Err(format!(
            "stack trace '{path}' exceeds the {MAX_STACK_TRACE_BYTES}-byte limit"
        ));
    }
    let text = std::fs::read_to_string(&resolved)
        .map_err(|err| format!("failed to read stack trace from '{path}': {err}"))?;
    Ok((text, path.to_string()))
}

fn emit_trace_error(trace: ErrorTrace, opts: &TraceErrorOptions<'_>) -> ExitCode {
    match opts.output {
        OutputFormat::Json => {
            let value = match fallow_output::serialize_trace_error_json_output(
                trace,
                crate::output_runtime::current_root_envelope_mode(),
                crate::output_runtime::telemetry_analysis_run_id().as_deref(),
            ) {
                Ok(value) => value,
                Err(err) => {
                    return emit_error(
                        &format!("failed to serialize trace-error output: {err}"),
                        2,
                        opts.output,
                    );
                }
            };
            report::emit_report_json(&value, "trace-error", opts.json_style)
        }
        OutputFormat::Human => {
            print_human(&trace, opts.quiet);
            ExitCode::SUCCESS
        }
        _ => emit_error(
            "trace-error supports --format json or human",
            2,
            opts.output,
        ),
    }
}

fn print_human(trace: &ErrorTrace, quiet: bool) {
    outln!("Stack-trace frames (syntactic; OFF the ranked path)");
    outln!();
    outln!("  source: {}", trace.source);
    if let Some(header) = &trace.header {
        outln!("  error:  {header}");
    }
    outln!();
    if trace.frames.is_empty() {
        outln!("No stack frames recognised.");
    }
    for frame in &trace.frames {
        let location = match (&frame.file, frame.line) {
            (Some(file), Some(line)) => format!("{file}:{line}"),
            (Some(file), None) => file.clone(),
            (None, _) => "<no location>".to_string(),
        };
        outln!(
            "  [{}] {} {} [{}/{}]",
            frame.index,
            frame.function.as_deref().unwrap_or("<anonymous>"),
            location,
            frame.origin.label(),
            frame.resolution.label()
        );
        for candidate in &frame.candidates {
            // An ambiguous frame prints every candidate. Printing only the
            // first would restate the exact overclaim the payload refuses.
            let member = candidate
                .member
                .as_ref()
                .map_or_else(String::new, |member| format!(".{member}"));
            let line = candidate
                .line
                .map_or_else(String::new, |line| format!(":{line}"));
            outln!(
                "        -> {}{} {}{} ({})",
                candidate.file,
                line,
                candidate.symbol,
                member,
                candidate.kind
            );
        }
        if frame.candidates_omitted > 0 {
            outln!(
                "        -> {} further matches omitted",
                frame.candidates_omitted
            );
        }
        if frame.resolution != FrameResolution::Resolved {
            outln!("        {}", frame.reason);
        }
    }
    outln!();
    outln!(
        "  frames {} | resolved {} | ambiguous {} | not found {} | not attempted {}",
        trace.counts.frames,
        trace.counts.resolved,
        trace.counts.ambiguous,
        trace.counts.not_found,
        trace.counts.not_attempted
    );
    // Prose, like the other trace surfaces: `--quiet` drops the explanation from
    // human output while the JSON payload keeps it either way.
    if !quiet {
        outln!();
        outln!("{}", trace.reason);
    }
}
