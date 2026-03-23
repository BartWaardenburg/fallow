pub mod analyze;
pub mod cache;
pub mod cross_reference;
pub mod discover;
pub mod duplicates;
pub mod errors;
pub mod extract;
pub mod plugins;
pub mod progress;
pub mod results;
pub mod scripts;
pub mod suppress;
pub mod trace;

// Re-export from fallow-graph for backwards compatibility
pub use fallow_graph::graph;
pub use fallow_graph::project;
pub use fallow_graph::resolve;

use std::path::Path;
use std::time::Instant;

use errors::FallowError;
use fallow_config::{PackageJson, ResolvedConfig, discover_workspaces};
use results::AnalysisResults;
use trace::PipelineTimings;

/// Result of the full analysis pipeline, including optional performance timings.
pub struct AnalysisOutput {
    pub results: AnalysisResults,
    pub timings: Option<PipelineTimings>,
    pub graph: Option<graph::ModuleGraph>,
}

/// Update cache: write freshly parsed modules and refresh stale mtime/size entries.
fn update_cache(
    store: &mut cache::CacheStore,
    modules: &[extract::ModuleInfo],
    files: &[discover::DiscoveredFile],
) {
    for module in modules {
        if let Some(file) = files.get(module.file_id.0 as usize) {
            let (mt, sz) = file_mtime_and_size(&file.path);
            // If content hash matches, just refresh mtime/size if stale (e.g. `touch`ed file)
            if let Some(cached) = store.get_by_path_only(&file.path)
                && cached.content_hash == module.content_hash
            {
                if cached.mtime_secs != mt || cached.file_size != sz {
                    store.insert(&file.path, cache::module_to_cached(module, mt, sz));
                }
                continue;
            }
            store.insert(&file.path, cache::module_to_cached(module, mt, sz));
        }
    }
    store.retain_paths(files);
}

/// Extract mtime (seconds since epoch) and file size from a path.
fn file_mtime_and_size(path: &std::path::Path) -> (u64, u64) {
    std::fs::metadata(path)
        .map(|m| {
            let mt = m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::SystemTime::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs());
            (mt, m.len())
        })
        .unwrap_or((0, 0))
}

/// Run the full analysis pipeline.
pub fn analyze(config: &ResolvedConfig) -> Result<AnalysisResults, FallowError> {
    let output = analyze_full(config, false, false)?;
    Ok(output.results)
}

/// Run the full analysis pipeline with export usage collection (for LSP Code Lens).
pub fn analyze_with_usages(config: &ResolvedConfig) -> Result<AnalysisResults, FallowError> {
    let output = analyze_full(config, false, true)?;
    Ok(output.results)
}

/// Run the full analysis pipeline with optional performance timings and graph retention.
pub fn analyze_with_trace(config: &ResolvedConfig) -> Result<AnalysisOutput, FallowError> {
    analyze_full(config, true, false)
}

#[expect(clippy::unnecessary_wraps)] // Result kept for future error handling
fn analyze_full(
    config: &ResolvedConfig,
    retain: bool,
    collect_usages: bool,
) -> Result<AnalysisOutput, FallowError> {
    let _span = tracing::info_span!("fallow_analyze").entered();
    let pipeline_start = Instant::now();

    // Progress bars: enabled when not quiet, stderr is a terminal, and output is human-readable.
    // Structured formats (JSON, SARIF) suppress spinners even on TTY — users piping structured
    // output don't expect progress noise on stderr.
    let show_progress = !config.quiet
        && std::io::IsTerminal::is_terminal(&std::io::stderr())
        && matches!(
            config.output,
            fallow_config::OutputFormat::Human
                | fallow_config::OutputFormat::Compact
                | fallow_config::OutputFormat::Markdown
        );
    let progress = progress::AnalysisProgress::new(show_progress);

    // Warn if node_modules is missing — resolution will be severely degraded
    if !config.root.join("node_modules").is_dir() {
        tracing::warn!(
            "node_modules directory not found. Run `npm install` / `pnpm install` first for accurate results."
        );
    }

    // Discover workspaces if in a monorepo
    let t = Instant::now();
    let workspaces_vec = discover_workspaces(&config.root);
    let workspaces_ms = t.elapsed().as_secs_f64() * 1000.0;
    if !workspaces_vec.is_empty() {
        tracing::info!(count = workspaces_vec.len(), "workspaces discovered");
    }

    // Stage 1: Discover all source files
    let t = Instant::now();
    let pb = progress.stage_spinner("Discovering files...");
    let discovered_files = discover::discover_files(config);
    let discover_ms = t.elapsed().as_secs_f64() * 1000.0;
    pb.finish_and_clear();

    // Build ProjectState: owns the file registry with stable FileIds and workspace metadata.
    // This is the foundation for cross-workspace resolution and future incremental analysis.
    let project = project::ProjectState::new(discovered_files, workspaces_vec);
    let files = project.files();
    let workspaces = project.workspaces();

    // Stage 1.5: Run plugin system — parse config files, discover dynamic entries
    let t = Instant::now();
    let pb = progress.stage_spinner("Detecting plugins...");
    let mut plugin_result = run_plugins(config, files, workspaces);
    let plugins_ms = t.elapsed().as_secs_f64() * 1000.0;
    pb.finish_and_clear();

    // Stage 1.6: Analyze package.json scripts for binary usage and config file refs
    let t = Instant::now();
    let pkg_path = config.root.join("package.json");
    if let Ok(pkg) = PackageJson::load(&pkg_path)
        && let Some(ref pkg_scripts) = pkg.scripts
    {
        // In production mode, only analyze start/build scripts
        let scripts_to_analyze = if config.production {
            scripts::filter_production_scripts(pkg_scripts)
        } else {
            pkg_scripts.clone()
        };
        let script_analysis = scripts::analyze_scripts(&scripts_to_analyze, &config.root);
        plugin_result.script_used_packages = script_analysis.used_packages;

        // Add config files from scripts as entry points (resolved later)
        for config_file in &script_analysis.config_files {
            plugin_result
                .entry_patterns
                .push((config_file.clone(), "scripts".to_string()));
        }
    }
    // Also analyze workspace package.json scripts
    for ws in workspaces {
        let ws_pkg_path = ws.root.join("package.json");
        if let Ok(ws_pkg) = PackageJson::load(&ws_pkg_path)
            && let Some(ref ws_scripts) = ws_pkg.scripts
        {
            let scripts_to_analyze = if config.production {
                scripts::filter_production_scripts(ws_scripts)
            } else {
                ws_scripts.clone()
            };
            let ws_analysis = scripts::analyze_scripts(&scripts_to_analyze, &ws.root);
            plugin_result
                .script_used_packages
                .extend(ws_analysis.used_packages);

            let ws_prefix = ws
                .root
                .strip_prefix(&config.root)
                .unwrap_or(&ws.root)
                .to_string_lossy();
            for config_file in &ws_analysis.config_files {
                plugin_result
                    .entry_patterns
                    .push((format!("{ws_prefix}/{config_file}"), "scripts".to_string()));
            }
        }
    }
    let scripts_ms = t.elapsed().as_secs_f64() * 1000.0;

    // Stage 2: Parse all files in parallel and extract imports/exports
    let t = Instant::now();
    let pb = progress.stage_spinner(&format!("Parsing {} files...", files.len()));
    let mut cache_store = if config.no_cache {
        None
    } else {
        cache::CacheStore::load(&config.cache_dir)
    };

    let parse_result = extract::parse_all_files(files, cache_store.as_ref());
    let modules = parse_result.modules;
    let cache_hits = parse_result.cache_hits;
    let cache_misses = parse_result.cache_misses;
    let parse_ms = t.elapsed().as_secs_f64() * 1000.0;
    pb.finish_and_clear();

    // Update cache with freshly parsed modules and refresh stale mtime/size entries.
    let t = Instant::now();
    if !config.no_cache {
        let store = cache_store.get_or_insert_with(cache::CacheStore::new);
        update_cache(store, &modules, files);
        if let Err(e) = store.save(&config.cache_dir) {
            tracing::warn!("Failed to save cache: {e}");
        }
    }
    let cache_ms = t.elapsed().as_secs_f64() * 1000.0;

    // Stage 3: Discover entry points (static patterns + plugin-discovered patterns)
    let t = Instant::now();
    let mut entry_points = discover::discover_entry_points(config, files);
    for ws in workspaces {
        let ws_entries = discover::discover_workspace_entry_points(&ws.root, config, files);
        entry_points.extend(ws_entries);
    }
    let plugin_entries = discover::discover_plugin_entry_points(&plugin_result, config, files);
    entry_points.extend(plugin_entries);
    let infra_entries = discover::discover_infrastructure_entry_points(&config.root);
    entry_points.extend(infra_entries);
    let entry_points_ms = t.elapsed().as_secs_f64() * 1000.0;

    // Stage 4: Resolve imports to file IDs
    let t = Instant::now();
    let pb = progress.stage_spinner("Resolving imports...");
    let resolved = resolve::resolve_all_imports(
        &modules,
        files,
        workspaces,
        &plugin_result.active_plugins,
        &plugin_result.path_aliases,
        &config.root,
    );
    let resolve_ms = t.elapsed().as_secs_f64() * 1000.0;
    pb.finish_and_clear();

    // Stage 5: Build module graph
    let t = Instant::now();
    let pb = progress.stage_spinner("Building module graph...");
    let graph = graph::ModuleGraph::build(&resolved, &entry_points, files);
    let graph_ms = t.elapsed().as_secs_f64() * 1000.0;
    pb.finish_and_clear();

    // Stage 6: Analyze for dead code (with plugin context and workspace info)
    let t = Instant::now();
    let pb = progress.stage_spinner("Analyzing...");
    let result = analyze::find_dead_code_full(
        &graph,
        config,
        &resolved,
        Some(&plugin_result),
        workspaces,
        &modules,
        collect_usages,
    );
    let analyze_ms = t.elapsed().as_secs_f64() * 1000.0;
    pb.finish_and_clear();
    progress.finish();

    let total_ms = pipeline_start.elapsed().as_secs_f64() * 1000.0;

    let cache_summary = if cache_hits > 0 {
        format!(" ({cache_hits} cached, {cache_misses} parsed)")
    } else {
        String::new()
    };

    tracing::debug!(
        "\n┌─ Pipeline Profile ─────────────────────────────\n\
         │  discover files:   {:>8.1}ms  ({} files)\n\
         │  workspaces:       {:>8.1}ms\n\
         │  plugins:          {:>8.1}ms\n\
         │  script analysis:  {:>8.1}ms\n\
         │  parse/extract:    {:>8.1}ms  ({} modules{})\n\
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
        scripts_ms,
        parse_ms,
        modules.len(),
        cache_summary,
        cache_ms,
        entry_points_ms,
        entry_points.len(),
        resolve_ms,
        graph_ms,
        analyze_ms,
        total_ms,
    );

    let timings = if retain {
        Some(PipelineTimings {
            discover_files_ms: discover_ms,
            file_count: files.len(),
            workspaces_ms,
            workspace_count: workspaces.len(),
            plugins_ms,
            script_analysis_ms: scripts_ms,
            parse_extract_ms: parse_ms,
            module_count: modules.len(),
            cache_hits,
            cache_misses,
            cache_update_ms: cache_ms,
            entry_points_ms,
            entry_point_count: entry_points.len(),
            resolve_imports_ms: resolve_ms,
            build_graph_ms: graph_ms,
            analyze_ms,
            total_ms,
        })
    } else {
        None
    };

    Ok(AnalysisOutput {
        results: result,
        timings,
        graph: if retain { Some(graph) } else { None },
    })
}

/// Run plugins for root project and all workspace packages.
fn run_plugins(
    config: &ResolvedConfig,
    files: &[discover::DiscoveredFile],
    workspaces: &[fallow_config::WorkspaceInfo],
) -> plugins::AggregatedPluginResult {
    let registry = plugins::PluginRegistry::new(config.external_plugins.clone());
    let file_paths: Vec<std::path::PathBuf> = files.iter().map(|f| f.path.clone()).collect();

    // Run plugins for root project (full run with external plugins, inline config, etc.)
    let pkg_path = config.root.join("package.json");
    let mut result = PackageJson::load(&pkg_path).map_or_else(
        |_| plugins::AggregatedPluginResult::default(),
        |pkg| registry.run(&pkg, &config.root, &file_paths),
    );

    if workspaces.is_empty() {
        return result;
    }

    // Pre-compile config matchers and relative files once for all workspace runs.
    // This avoids re-compiling glob patterns and re-computing relative paths per workspace
    // (previously O(workspaces × plugins × files) glob compilations).
    let precompiled_matchers = registry.precompile_config_matchers();
    let relative_files: Vec<(&std::path::PathBuf, String)> = file_paths
        .iter()
        .map(|f| {
            let rel = f
                .strip_prefix(&config.root)
                .unwrap_or(f)
                .to_string_lossy()
                .into_owned();
            (f, rel)
        })
        .collect();

    // Run plugins for each workspace package using the fast path
    for ws in workspaces {
        let ws_pkg_path = ws.root.join("package.json");
        if let Ok(ws_pkg) = PackageJson::load(&ws_pkg_path) {
            let ws_result = registry.run_workspace_fast(
                &ws_pkg,
                &ws.root,
                &config.root,
                &precompiled_matchers,
                &relative_files,
            );

            // Early skip if workspace produced no results (common for leaf packages)
            if ws_result.active_plugins.is_empty() {
                continue;
            }

            // Workspace plugin patterns are relative to the workspace root (e.g., `jest.setup.ts`),
            // but `discover_plugin_entry_points` matches against paths relative to the monorepo root
            // (e.g., `packages/foo/jest.setup.ts`). Prefix workspace patterns with the workspace
            // path to make them matchable from the monorepo root.
            let ws_prefix = ws
                .root
                .strip_prefix(&config.root)
                .unwrap_or(&ws.root)
                .to_string_lossy();

            // Prefix helper: workspace-relative patterns need the workspace prefix
            // to be matchable from the monorepo root. But patterns that are already
            // project-root-relative (e.g., from angular.json which uses absolute paths
            // like "apps/client/src/styles.css") should not be double-prefixed.
            let prefix_if_needed = |pat: &str| -> String {
                if pat.starts_with(ws_prefix.as_ref()) || pat.starts_with('/') {
                    pat.to_string()
                } else {
                    format!("{ws_prefix}/{pat}")
                }
            };

            for (pat, pname) in &ws_result.entry_patterns {
                result
                    .entry_patterns
                    .push((prefix_if_needed(pat), pname.clone()));
            }
            for (pat, pname) in &ws_result.always_used {
                result
                    .always_used
                    .push((prefix_if_needed(pat), pname.clone()));
            }
            for (pat, pname) in &ws_result.discovered_always_used {
                result
                    .discovered_always_used
                    .push((prefix_if_needed(pat), pname.clone()));
            }
            for (file_pat, exports) in &ws_result.used_exports {
                result
                    .used_exports
                    .push((prefix_if_needed(file_pat), exports.clone()));
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
            // Virtual module prefixes (e.g., Docusaurus @theme/, @site/) are
            // package-name prefixes, not file paths — no workspace prefix needed.
            for prefix in &ws_result.virtual_module_prefixes {
                if !result.virtual_module_prefixes.contains(prefix) {
                    result.virtual_module_prefixes.push(prefix.clone());
                }
            }
        }
    }

    result
}

/// Run analysis on a project directory (with export usages for LSP Code Lens).
pub fn analyze_project(root: &Path) -> Result<AnalysisResults, FallowError> {
    let config = default_config(root);
    analyze_with_usages(&config)
}

/// Create a default config for a project root.
pub(crate) fn default_config(root: &Path) -> ResolvedConfig {
    let user_config = fallow_config::FallowConfig::find_and_load(root)
        .ok()
        .flatten();
    match user_config {
        Some((config, _path)) => config.resolve(
            root.to_path_buf(),
            fallow_config::OutputFormat::Human,
            num_cpus(),
            false,
            true, // quiet: LSP/programmatic callers don't need progress bars
        ),
        None => fallow_config::FallowConfig {
            schema: None,
            extends: vec![],
            entry: vec![],
            ignore_patterns: vec![],
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            duplicates: fallow_config::DuplicatesConfig::default(),
            rules: fallow_config::RulesConfig::default(),
            production: false,
            plugins: vec![],
            overrides: vec![],
        }
        .resolve(
            root.to_path_buf(),
            fallow_config::OutputFormat::Human,
            num_cpus(),
            false,
            true,
        ),
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
