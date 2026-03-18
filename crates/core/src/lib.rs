pub mod analyze;
pub mod cache;
pub mod discover;
pub mod duplicates;
pub mod errors;
pub mod extract;
pub mod graph;
pub mod plugins;
pub mod progress;
pub mod resolve;
pub mod results;

use std::path::Path;
use std::time::Instant;

use errors::FallowError;
use fallow_config::{PackageJson, ResolvedConfig, discover_workspaces};
use results::AnalysisResults;

/// Run the full analysis pipeline.
pub fn analyze(config: &ResolvedConfig) -> Result<AnalysisResults, FallowError> {
    let _span = tracing::info_span!("fallow_analyze").entered();
    let pipeline_start = Instant::now();

    // Warn if node_modules is missing — resolution will be severely degraded
    if !config.root.join("node_modules").is_dir() {
        tracing::warn!(
            "node_modules directory not found. Run `npm install` / `pnpm install` first for accurate results."
        );
    }

    // Discover workspaces if in a monorepo
    let t = Instant::now();
    let workspaces = discover_workspaces(&config.root);
    let workspaces_ms = t.elapsed().as_secs_f64() * 1000.0;
    if !workspaces.is_empty() {
        tracing::info!(count = workspaces.len(), "workspaces discovered");
    }

    // Stage 1: Discover all source files
    let t = Instant::now();
    let files = discover::discover_files(config);
    let discover_ms = t.elapsed().as_secs_f64() * 1000.0;

    // Stage 1.5: Run plugin system — parse config files, discover dynamic entries
    let t = Instant::now();
    let plugin_result = run_plugins(config, &files, &workspaces);
    let plugins_ms = t.elapsed().as_secs_f64() * 1000.0;

    // Stage 2: Parse all files in parallel and extract imports/exports
    let t = Instant::now();
    let mut cache_store = if config.no_cache {
        None
    } else {
        cache::CacheStore::load(&config.cache_dir)
    };

    let modules = extract::parse_all_files(&files, config, cache_store.as_ref());
    let parse_ms = t.elapsed().as_secs_f64() * 1000.0;

    // Update cache with parsed results
    let t = Instant::now();
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
    let cache_ms = t.elapsed().as_secs_f64() * 1000.0;

    // Stage 3: Discover entry points (static patterns + plugin-discovered patterns)
    let t = Instant::now();
    let mut entry_points = discover::discover_entry_points(config, &files);
    for ws in &workspaces {
        let ws_entries = discover::discover_workspace_entry_points(&ws.root, config, &files);
        entry_points.extend(ws_entries);
    }
    let plugin_entries = discover::discover_plugin_entry_points(&plugin_result, config, &files);
    entry_points.extend(plugin_entries);
    let entry_points_ms = t.elapsed().as_secs_f64() * 1000.0;

    // Stage 4: Resolve imports to file IDs
    let t = Instant::now();
    let resolved = resolve::resolve_all_imports(&modules, config, &files);
    let resolve_ms = t.elapsed().as_secs_f64() * 1000.0;

    // Stage 5: Build module graph
    let t = Instant::now();
    let graph = graph::ModuleGraph::build(&resolved, &entry_points, &files);
    let graph_ms = t.elapsed().as_secs_f64() * 1000.0;

    // Stage 6: Analyze for dead code (with plugin context and workspace info)
    let t = Instant::now();
    let result =
        analyze::find_dead_code_full(&graph, config, &resolved, Some(&plugin_result), &workspaces);
    let analyze_ms = t.elapsed().as_secs_f64() * 1000.0;

    let total_ms = pipeline_start.elapsed().as_secs_f64() * 1000.0;

    tracing::debug!(
        "\n┌─ Pipeline Profile ─────────────────────────────\n\
         │  discover files:   {:>8.1}ms  ({} files)\n\
         │  workspaces:       {:>8.1}ms\n\
         │  plugins:          {:>8.1}ms\n\
         │  parse/extract:    {:>8.1}ms  ({} modules)\n\
         │  cache update:     {:>8.1}ms\n\
         │  entry points:     {:>8.1}ms  ({} entries)\n\
         │  resolve imports:  {:>8.1}ms\n\
         │  build graph:      {:>8.1}ms\n\
         │  analyze:          {:>8.1}ms\n\
         │  ────────────────────────────────────────────\n\
         │  TOTAL:            {:>8.1}ms\n\
         └─────────────────────────────────────────────────",
        discover_ms,
        files.len(),
        workspaces_ms,
        plugins_ms,
        parse_ms,
        modules.len(),
        cache_ms,
        entry_points_ms,
        entry_points.len(),
        resolve_ms,
        graph_ms,
        analyze_ms,
        total_ms,
    );

    Ok(result)
}

/// Run plugins for root project and all workspace packages.
fn run_plugins(
    config: &ResolvedConfig,
    files: &[discover::DiscoveredFile],
    workspaces: &[fallow_config::WorkspaceInfo],
) -> plugins::AggregatedPluginResult {
    let registry = plugins::PluginRegistry::new();
    let file_paths: Vec<std::path::PathBuf> = files.iter().map(|f| f.path.clone()).collect();

    // Run plugins for root project
    let pkg_path = config.root.join("package.json");
    let mut result = if let Ok(pkg) = PackageJson::load(&pkg_path) {
        registry.run(&pkg, &config.root, &file_paths)
    } else {
        plugins::AggregatedPluginResult::default()
    };

    // Run plugins for each workspace package too
    for ws in workspaces {
        let ws_pkg_path = ws.root.join("package.json");
        if let Ok(ws_pkg) = PackageJson::load(&ws_pkg_path) {
            let ws_result = registry.run(&ws_pkg, &ws.root, &file_paths);

            // Workspace plugin patterns are relative to the workspace root (e.g., `jest.setup.ts`),
            // but `discover_plugin_entry_points` matches against paths relative to the monorepo root
            // (e.g., `packages/foo/jest.setup.ts`). Prefix workspace patterns with the workspace
            // path to make them matchable from the monorepo root.
            let ws_prefix = ws
                .root
                .strip_prefix(&config.root)
                .unwrap_or(&ws.root)
                .to_string_lossy();

            for pat in &ws_result.entry_patterns {
                result.entry_patterns.push(format!("{ws_prefix}/{pat}"));
            }
            for pat in &ws_result.always_used {
                result.always_used.push(format!("{ws_prefix}/{pat}"));
            }
            for pat in &ws_result.discovered_always_used {
                result
                    .discovered_always_used
                    .push(format!("{ws_prefix}/{pat}"));
            }
            for (file_pat, exports) in &ws_result.used_exports {
                result
                    .used_exports
                    .push((format!("{ws_prefix}/{file_pat}"), exports.clone()));
            }
            // Merge active plugin names (deduplicated)
            for plugin_name in &ws_result.active_plugins {
                if !result.active_plugins.contains(plugin_name) {
                    result.active_plugins.push(plugin_name.clone());
                }
            }
            // These don't need prefixing (absolute paths / package names)
            result
                .referenced_dependencies
                .extend(ws_result.referenced_dependencies);
            result.setup_files.extend(ws_result.setup_files);
            result
                .tooling_dependencies
                .extend(ws_result.tooling_dependencies);
        }
    }

    result
}

/// Run analysis on a project directory.
pub fn analyze_project(root: &Path) -> Result<AnalysisResults, FallowError> {
    let config = default_config(root);
    analyze(&config)
}

/// Create a default config for a project root.
pub(crate) fn default_config(root: &Path) -> ResolvedConfig {
    let user_config = fallow_config::FallowConfig::find_and_load(root)
        .ok()
        .flatten();
    match user_config {
        Some((config, _path)) => config.resolve(root.to_path_buf(), num_cpus(), false),
        None => fallow_config::FallowConfig {
            entry: vec![],
            ignore: vec![],
            detect: fallow_config::DetectConfig::default(),
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            output: fallow_config::OutputFormat::Human,
            duplicates: fallow_config::DuplicatesConfig::default(),
        }
        .resolve(root.to_path_buf(), num_cpus(), false),
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
