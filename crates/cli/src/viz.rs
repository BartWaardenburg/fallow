use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use rustc_hash::FxHashSet;
use serde::Serialize;

use fallow_config::OutputFormat;
use fallow_core::graph::ModuleGraph;
use fallow_core::project::ProjectState;
use fallow_core::results::AnalysisResults;

use crate::{emit_error, load_config};

// ── Embedded viz assets ─────────────────────────────────────────

const VIZ_JS: &str = include_str!("../viz-assets/viz.js");
const VIZ_CSS: &str = include_str!("../viz-assets/viz.css");

// ── CLI types ───────────────────────────────────────────────────

#[derive(Clone, clap::ValueEnum)]
pub enum VizFormat {
    /// Interactive HTML treemap (default)
    Html,
    /// Graphviz DOT format
    Dot,
    /// Mermaid diagram format
    Mermaid,
}

pub struct VizOptions<'a> {
    pub root: &'a Path,
    pub config_path: &'a Option<PathBuf>,
    pub no_cache: bool,
    pub threads: usize,
    pub quiet: bool,
    pub production: bool,
    pub output_path: Option<&'a Path>,
    pub no_open: bool,
    pub format: VizFormat,
}

// ── Serializable viz data ───────────────────────────────────────

#[derive(Serialize)]
struct VizData {
    root: String,
    files: Vec<VizFile>,
    edges: Vec<[u32; 2]>,
    summary: VizSummary,
    workspaces: Vec<VizWorkspace>,
    cycles: Vec<Vec<u32>>,
}

#[derive(Serialize)]
struct VizFile {
    path: String,
    size: u64,
    status: VizFileStatus,
    export_count: u16,
    unused_export_count: u16,
    is_entry: bool,
    importer_count: u16,
    import_count: u16,
    workspace: String,
    /// Names of unused exports in this file (for actionable tooltips)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    unused_exports: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum VizFileStatus {
    Clean,
    HasUnusedExports,
    Unused,
    EntryPoint,
}

#[derive(Serialize)]
struct VizSummary {
    total_files: usize,
    total_size: u64,
    unused_files: usize,
    unused_exports: usize,
    unused_types: usize,
    unused_deps: usize,
    unresolved_imports: usize,
    circular_deps: usize,
}

#[derive(Serialize)]
struct VizWorkspace {
    name: String,
    root: String,
}

// ── Entry point ─────────────────────────────────────────────────

pub fn run_viz(opts: &VizOptions<'_>) -> ExitCode {
    let start = Instant::now();

    let config = match load_config(
        opts.root,
        opts.config_path,
        OutputFormat::Human,
        opts.no_cache,
        opts.threads,
        opts.production,
        opts.quiet,
    ) {
        Ok(c) => c,
        Err(code) => return code,
    };

    // Always use trace mode to retain the graph and project state
    let output = match fallow_core::analyze_with_trace(&config) {
        Ok(o) => o,
        Err(e) => {
            return emit_error(&format!("Analysis error: {e}"), 2, &OutputFormat::Human);
        }
    };

    let Some(graph) = output.graph else {
        return emit_error("Graph not available", 2, &OutputFormat::Human);
    };

    let Some(project) = output.project else {
        return emit_error("Project state not available", 2, &OutputFormat::Human);
    };

    let data = build_viz_data(&graph, &project, &output.results, &config.root);
    let elapsed = start.elapsed();

    match opts.format {
        VizFormat::Html => write_html(opts, &data, elapsed),
        VizFormat::Dot => {
            let dot = generate_dot(&data);
            println!("{dot}");
            ExitCode::SUCCESS
        }
        VizFormat::Mermaid => {
            let mermaid = generate_mermaid(&data);
            println!("{mermaid}");
            ExitCode::SUCCESS
        }
    }
}

// ── Build VizData from pipeline output ──────────────────────────

fn build_viz_data(
    graph: &ModuleGraph,
    project: &ProjectState,
    results: &AnalysisResults,
    root: &Path,
) -> VizData {
    let files = project.files();

    // Build lookup sets for status classification
    let unused_file_paths: FxHashSet<&Path> = results
        .unused_files
        .iter()
        .map(|f| f.path.as_path())
        .collect();

    // Collect unused export names per file
    let mut unused_exports_by_file: rustc_hash::FxHashMap<&Path, Vec<String>> =
        rustc_hash::FxHashMap::default();
    for export in &results.unused_exports {
        unused_exports_by_file
            .entry(export.path.as_path())
            .or_default()
            .push(export.export_name.clone());
    }
    for export in &results.unused_types {
        unused_exports_by_file
            .entry(export.path.as_path())
            .or_default()
            .push(export.export_name.clone());
    }

    // Build file entries
    let root_display = root.file_name().map_or_else(
        || root.to_string_lossy().into_owned(),
        |n| n.to_string_lossy().into_owned(),
    );

    let viz_files: Vec<VizFile> = files
        .iter()
        .map(|f| {
            let idx = f.id.0 as usize;
            let rel_path = f
                .path
                .strip_prefix(root)
                .unwrap_or(&f.path)
                .to_string_lossy()
                .into_owned();

            let is_entry = idx < graph.modules.len() && graph.modules[idx].is_entry_point;
            let export_count = if idx < graph.modules.len() {
                graph.modules[idx].exports.len().min(u16::MAX as usize) as u16
            } else {
                0
            };
            let unused_export_names = unused_exports_by_file
                .remove(f.path.as_path())
                .unwrap_or_default();
            let unused_export_count = unused_export_names.len().min(u16::MAX as usize) as u16;
            let import_count = if idx < graph.modules.len() {
                graph.edges_for(f.id).len().min(u16::MAX as usize) as u16
            } else {
                0
            };
            let importer_count = if idx < graph.reverse_deps.len() {
                graph.reverse_deps[idx].len().min(u16::MAX as usize) as u16
            } else {
                0
            };

            // Status priority: Unused > HasUnusedExports > EntryPoint > Clean
            let status = if unused_file_paths.contains(f.path.as_path()) {
                VizFileStatus::Unused
            } else if unused_export_count > 0 {
                VizFileStatus::HasUnusedExports
            } else if is_entry {
                VizFileStatus::EntryPoint
            } else {
                VizFileStatus::Clean
            };

            let workspace = project
                .workspace_for_file(f.id)
                .map_or_else(String::new, |ws| ws.name.clone());

            VizFile {
                path: rel_path,
                size: f.size_bytes,
                status,
                export_count,
                unused_export_count,
                is_entry,
                importer_count,
                import_count,
                workspace,
                unused_exports: unused_export_names,
            }
        })
        .collect();

    // Build edges as [source_idx, target_idx] pairs
    let mut edges = Vec::new();
    for file in files {
        let source = file.id.0;
        for target_id in graph.edges_for(file.id) {
            edges.push([source, target_id.0]);
        }
    }

    // Build cycles from analysis results (avoids re-running expensive cycle detection)
    let cycles: Vec<Vec<u32>> = results
        .circular_dependencies
        .iter()
        .filter_map(|cd| {
            let ids: Vec<u32> = cd
                .files
                .iter()
                .filter_map(|p| project.id_for_path(p).map(|id| id.0))
                .collect();
            if ids.len() == cd.files.len() {
                Some(ids)
            } else {
                None
            }
        })
        .collect();

    // Build workspace list
    let workspaces: Vec<VizWorkspace> = project
        .workspaces()
        .iter()
        .map(|ws| VizWorkspace {
            name: ws.name.clone(),
            root: ws
                .root
                .strip_prefix(root)
                .unwrap_or(&ws.root)
                .to_string_lossy()
                .into_owned(),
        })
        .collect();

    // Build summary
    let summary = VizSummary {
        total_files: files.len(),
        total_size: files.iter().map(|f| f.size_bytes).sum(),
        unused_files: results.unused_files.len(),
        unused_exports: results.unused_exports.len() + results.unused_types.len(),
        unused_types: results.unused_types.len(),
        unused_deps: results.unused_dependencies.len()
            + results.unused_dev_dependencies.len()
            + results.unused_optional_dependencies.len(),
        unresolved_imports: results.unresolved_imports.len(),
        circular_deps: results.circular_dependencies.len(),
    };

    VizData {
        root: root_display,
        files: viz_files,
        edges,
        summary,
        workspaces,
        cycles,
    }
}

// ── HTML generation ─────────────────────────────────────────────

/// Escape HTML special characters to prevent injection in text content.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn write_html(opts: &VizOptions<'_>, data: &VizData, elapsed: std::time::Duration) -> ExitCode {
    let json = match serde_json::to_string(data) {
        Ok(j) => j,
        Err(e) => {
            return emit_error(
                &format!("Failed to serialize viz data: {e}"),
                2,
                &OutputFormat::Human,
            );
        }
    };

    // Prevent </script> in JSON string values from closing the script tag
    let json_safe = json.replace("</", "<\\/");
    let title = html_escape(&data.root);

    let css = VIZ_CSS;
    let js = VIZ_JS;
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>fallow — {title}</title>
<style>{css}</style>
</head>
<body>
<script>window.__FALLOW_DATA__={json_safe};</script>
<script>{js}</script>
</body>
</html>"#,
    );

    let output_path = opts
        .output_path
        .map_or_else(|| opts.root.join("fallow-viz.html"), Path::to_path_buf);

    if let Err(e) = std::fs::write(&output_path, &html) {
        return emit_error(
            &format!("Failed to write HTML: {e}"),
            2,
            &OutputFormat::Human,
        );
    }

    if !opts.quiet {
        let file_count = data.summary.total_files;
        let issue_count = data.summary.unused_files
            + data.summary.unused_exports
            + data.summary.unused_deps
            + data.summary.unresolved_imports
            + data.summary.circular_deps;
        eprintln!(
            "Visualization generated in {:.0}ms ({file_count} files, {issue_count} issues)",
            elapsed.as_secs_f64() * 1000.0,
        );
        eprintln!("  → {}", output_path.display());
    }

    if !opts.no_open
        && let Err(e) = open::that(&output_path)
    {
        eprintln!("Could not open browser: {e}");
        eprintln!("Open manually: {}", output_path.display());
    }

    ExitCode::SUCCESS
}

// ── DOT generation ──────────────────────────────────────────────

fn generate_dot(data: &VizData) -> String {
    let mut out =
        String::from("digraph fallow {\n  rankdir=LR;\n  node [shape=box, style=filled];\n\n");

    for (i, f) in data.files.iter().enumerate() {
        let color = match f.status {
            VizFileStatus::Unused => "#EF4444",
            VizFileStatus::HasUnusedExports => "#F59E0B",
            VizFileStatus::EntryPoint => "#10B981",
            VizFileStatus::Clean => "#3B82F6",
        };
        let escaped_path = f.path.replace('\\', "\\\\").replace('"', "\\\"");
        let _ = writeln!(
            out,
            "  n{i} [label=\"{escaped_path}\", fillcolor=\"{color}\", fontcolor=\"white\"];",
        );
    }

    out.push('\n');

    for [src, tgt] in &data.edges {
        let _ = writeln!(out, "  n{src} -> n{tgt};");
    }

    out.push_str("}\n");
    out
}

// ── Mermaid generation ──────────────────────────────────────────

fn generate_mermaid(data: &VizData) -> String {
    let mut out = String::from("graph LR\n");

    for (i, f) in data.files.iter().enumerate() {
        let escaped_path = f.path.replace('"', "#quot;").replace(']', "#93;");
        let _ = writeln!(out, "  n{i}[\"{escaped_path}\"]");
    }

    out.push('\n');

    for [src, tgt] in &data.edges {
        let _ = writeln!(out, "  n{src} --> n{tgt}");
    }

    // Style classes for file status
    out.push('\n');
    let mut unused: Vec<usize> = Vec::new();
    let mut has_unused: Vec<usize> = Vec::new();
    let mut entry: Vec<usize> = Vec::new();

    for (i, f) in data.files.iter().enumerate() {
        match f.status {
            VizFileStatus::Unused => unused.push(i),
            VizFileStatus::HasUnusedExports => has_unused.push(i),
            VizFileStatus::EntryPoint => entry.push(i),
            VizFileStatus::Clean => {}
        }
    }

    if !unused.is_empty() {
        let nodes: Vec<String> = unused.iter().map(|i| format!("n{i}")).collect();
        let _ = writeln!(out, "  style {} fill:#EF4444,color:#fff", nodes.join(","));
    }
    if !has_unused.is_empty() {
        let nodes: Vec<String> = has_unused.iter().map(|i| format!("n{i}")).collect();
        let _ = writeln!(out, "  style {} fill:#F59E0B,color:#fff", nodes.join(","));
    }
    if !entry.is_empty() {
        let nodes: Vec<String> = entry.iter().map(|i| format!("n{i}")).collect();
        let _ = writeln!(out, "  style {} fill:#10B981,color:#fff", nodes.join(","));
    }

    out
}
