pub mod analyze;
pub mod discover;
pub mod extract;
pub mod graph;
pub mod resolve;
pub mod results;

use std::path::Path;

use fallow_config::{ResolvedConfig, discover_workspaces};
use results::AnalysisResults;

/// Run the full analysis pipeline.
pub fn analyze(config: &ResolvedConfig) -> AnalysisResults {
    let _span = tracing::info_span!("fallow_analyze").entered();

    // Discover workspaces if in a monorepo
    let workspaces = discover_workspaces(&config.root);
    if !workspaces.is_empty() {
        tracing::info!(count = workspaces.len(), "workspaces discovered");
    }

    // Stage 1: Discover all source files (across all workspaces)
    tracing::info!("discovering files...");
    let mut files = discover::discover_files(config);
    // Also discover files in workspaces
    for ws in &workspaces {
        let ws_files = discover::discover_workspace_files(&ws.root, config, files.len());
        files.extend(ws_files);
    }
    tracing::info!(count = files.len(), "files discovered");

    // Stage 2: Parse all files in parallel and extract imports/exports
    tracing::info!("parsing files...");
    let modules = extract::parse_all_files(&files, config);
    tracing::info!(count = modules.len(), "modules parsed");

    // Stage 3: Discover entry points
    tracing::info!("discovering entry points...");
    let mut entry_points = discover::discover_entry_points(config, &files);
    // Also discover workspace entry points
    for ws in &workspaces {
        let ws_entries = discover::discover_workspace_entry_points(&ws.root, config, &files);
        entry_points.extend(ws_entries);
    }
    tracing::info!(count = entry_points.len(), "entry points found");

    // Stage 4: Resolve imports to file IDs
    tracing::info!("resolving imports...");
    let resolved = resolve::resolve_all_imports(&modules, config, &files);

    // Stage 5: Build module graph
    tracing::info!("building module graph...");
    let graph = graph::ModuleGraph::build(&resolved, &entry_points, &files);
    tracing::info!(
        modules = graph.module_count(),
        edges = graph.edge_count(),
        "graph built"
    );

    // Stage 6: Analyze for dead code
    tracing::info!("analyzing...");
    let results = analyze::find_dead_code_with_resolved(&graph, config, &resolved);
    tracing::info!(
        unused_files = results.unused_files.len(),
        unused_exports = results.unused_exports.len(),
        unused_deps = results.unused_dependencies.len(),
        "analysis complete"
    );

    results
}

/// Run analysis on a project directory.
pub fn analyze_project(root: &Path) -> AnalysisResults {
    let config = default_config(root);
    analyze(&config)
}

/// Create a default config for a project root.
fn default_config(root: &Path) -> ResolvedConfig {
    let user_config = fallow_config::FallowConfig::find_and_load(root);
    match user_config {
        Some((config, _path)) => config.resolve(root.to_path_buf(), num_cpus(), false),
        None => fallow_config::FallowConfig {
            root: None,
            entry: vec![],
            ignore: vec![],
            detect: fallow_config::DetectConfig::default(),
            frameworks: None,
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            output: fallow_config::OutputFormat::Human,
        }
        .resolve(root.to_path_buf(), num_cpus(), false),
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
