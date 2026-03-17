pub mod analyze;
pub mod cache;
pub mod discover;
pub mod errors;
pub mod extract;
pub mod graph;
pub mod progress;
pub mod resolve;
pub mod results;

use std::path::Path;

use errors::FallowError;
use fallow_config::{ResolvedConfig, discover_workspaces};
use results::AnalysisResults;

/// Run the full analysis pipeline.
pub fn analyze(config: &ResolvedConfig) -> Result<AnalysisResults, FallowError> {
    let _span = tracing::info_span!("fallow_analyze").entered();
    // Discover workspaces if in a monorepo
    let workspaces = discover_workspaces(&config.root);
    if !workspaces.is_empty() {
        tracing::info!(count = workspaces.len(), "workspaces discovered");
    }

    // Stage 1: Discover all source files
    // The root walk already discovers files in nested workspace directories,
    // so no separate workspace walk is needed.
    let files = discover::discover_files(config);

    // Stage 2: Parse all files in parallel and extract imports/exports
    // Load cache if available
    let mut cache_store = if config.no_cache {
        None
    } else {
        cache::CacheStore::load(&config.cache_dir)
    };

    let modules = extract::parse_all_files(&files, config, cache_store.as_ref());

    // Update cache with parsed results
    if !config.no_cache {
        let store = cache_store.get_or_insert_with(cache::CacheStore::new);
        for module in &modules {
            if let Some(file) = files.get(module.file_id.0 as usize) {
                store.insert(&file.path, cache::module_to_cached(module));
            }
        }
        if let Err(e) = store.save(&config.cache_dir) {
            tracing::warn!("Failed to save cache: {e}");
        }
    }

    // Stage 3: Discover entry points
    let mut entry_points = discover::discover_entry_points(config, &files);
    // Also discover workspace entry points
    for ws in &workspaces {
        let ws_entries = discover::discover_workspace_entry_points(&ws.root, config, &files);
        entry_points.extend(ws_entries);
    }

    // Stage 4: Resolve imports to file IDs
    let resolved = resolve::resolve_all_imports(&modules, config, &files);

    // Stage 5: Build module graph
    let graph = graph::ModuleGraph::build(&resolved, &entry_points, &files);

    // Stage 6: Analyze for dead code
    Ok(analyze::find_dead_code_with_resolved(
        &graph, config, &resolved,
    ))
}

/// Run analysis on a project directory.
pub fn analyze_project(root: &Path) -> Result<AnalysisResults, FallowError> {
    let config = default_config(root);
    analyze(&config)
}

/// Create a default config for a project root.
fn default_config(root: &Path) -> ResolvedConfig {
    let user_config = fallow_config::FallowConfig::find_and_load(root)
        .ok()
        .flatten();
    match user_config {
        Some((config, _path)) => config.resolve(root.to_path_buf(), num_cpus(), false),
        None => fallow_config::FallowConfig {
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
