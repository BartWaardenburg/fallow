//! Plugin registry: discovers active plugins, collects patterns, parses configs.
#![expect(clippy::excessive_nesting)]

use rustc_hash::FxHashSet;
use std::path::{Path, PathBuf};

use fallow_config::{ExternalPluginDef, PackageJson};

use super::Plugin;

mod builtin;
mod helpers;

use helpers::{
    check_has_config_file, discover_json_config_files, process_config_result,
    process_external_plugins, process_static_patterns,
};

/// Registry of all available plugins (built-in + external).
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
    external_plugins: Vec<ExternalPluginDef>,
}

/// Aggregated results from all active plugins for a project.
#[derive(Debug, Default)]
pub struct AggregatedPluginResult {
    /// All entry point patterns from active plugins: (pattern, plugin_name).
    pub entry_patterns: Vec<(String, String)>,
    /// All config file patterns from active plugins.
    pub config_patterns: Vec<String>,
    /// All always-used file patterns from active plugins: (pattern, plugin_name).
    pub always_used: Vec<(String, String)>,
    /// All used export rules from active plugins.
    pub used_exports: Vec<(String, Vec<String>)>,
    /// Dependencies referenced in config files (should not be flagged unused).
    pub referenced_dependencies: Vec<String>,
    /// Additional always-used files discovered from config parsing: (pattern, plugin_name).
    pub discovered_always_used: Vec<(String, String)>,
    /// Setup files discovered from config parsing: (path, plugin_name).
    pub setup_files: Vec<(PathBuf, String)>,
    /// Tooling dependencies (should not be flagged as unused devDeps).
    pub tooling_dependencies: Vec<String>,
    /// Package names discovered as used in package.json scripts (binary invocations).
    pub script_used_packages: FxHashSet<String>,
    /// Import prefixes for virtual modules provided by active frameworks.
    /// Imports matching these prefixes should not be flagged as unlisted dependencies.
    pub virtual_module_prefixes: Vec<String>,
    /// Path alias mappings from active plugins (prefix → replacement directory).
    /// Used by the resolver to substitute import prefixes before re-resolving.
    pub path_aliases: Vec<(String, String)>,
    /// Names of active plugins.
    pub active_plugins: Vec<String>,
}

impl PluginRegistry {
    /// Create a registry with all built-in plugins and optional external plugins.
    pub fn new(external: Vec<ExternalPluginDef>) -> Self {
        Self {
            plugins: builtin::create_builtin_plugins(),
            external_plugins: external,
        }
    }

    /// Run all plugins against a project, returning aggregated results.
    ///
    /// This discovers which plugins are active, collects their static patterns,
    /// then parses any config files to extract dynamic information.
    pub fn run(
        &self,
        pkg: &PackageJson,
        root: &Path,
        discovered_files: &[PathBuf],
    ) -> AggregatedPluginResult {
        let _span = tracing::info_span!("run_plugins").entered();
        let mut result = AggregatedPluginResult::default();

        // Phase 1: Determine which plugins are active
        // Compute deps once to avoid repeated Vec<String> allocation per plugin
        let all_deps = pkg.all_dependency_names();
        let active: Vec<&dyn Plugin> = self
            .plugins
            .iter()
            .filter(|p| p.is_enabled_with_deps(&all_deps, root))
            .map(|p| p.as_ref())
            .collect();

        tracing::info!(
            plugins = active
                .iter()
                .map(|p| p.name())
                .collect::<Vec<_>>()
                .join(", "),
            "active plugins"
        );

        // Phase 2: Collect static patterns from active plugins
        for plugin in &active {
            process_static_patterns(*plugin, root, &mut result);
        }

        // Phase 2b: Process external plugins (includes inline framework definitions)
        process_external_plugins(
            &self.external_plugins,
            &all_deps,
            root,
            discovered_files,
            &mut result,
        );

        // Phase 3: Find and parse config files for dynamic resolution
        // Pre-compile all config patterns
        let config_matchers: Vec<(&dyn Plugin, Vec<globset::GlobMatcher>)> = active
            .iter()
            .filter(|p| !p.config_patterns().is_empty())
            .map(|p| {
                let matchers: Vec<globset::GlobMatcher> = p
                    .config_patterns()
                    .iter()
                    .filter_map(|pat| globset::Glob::new(pat).ok().map(|g| g.compile_matcher()))
                    .collect();
                (*p, matchers)
            })
            .collect();

        // Build relative paths for matching (used by Phase 3 and 4)
        let relative_files: Vec<(&PathBuf, String)> = discovered_files
            .iter()
            .map(|f| {
                let rel = f
                    .strip_prefix(root)
                    .unwrap_or(f)
                    .to_string_lossy()
                    .into_owned();
                (f, rel)
            })
            .collect();

        if !config_matchers.is_empty() {
            // Phase 3a: Match config files from discovered source files
            let mut resolved_plugins: FxHashSet<&str> = FxHashSet::default();

            for (plugin, matchers) in &config_matchers {
                for (abs_path, rel_path) in &relative_files {
                    if matchers.iter().any(|m| m.is_match(rel_path.as_str())) {
                        // Mark as resolved regardless of result to prevent Phase 3b
                        // from re-parsing a JSON config for the same plugin.
                        resolved_plugins.insert(plugin.name());
                        if let Ok(source) = std::fs::read_to_string(abs_path) {
                            let plugin_result = plugin.resolve_config(abs_path, &source, root);
                            if !plugin_result.is_empty() {
                                tracing::debug!(
                                    plugin = plugin.name(),
                                    config = rel_path.as_str(),
                                    entries = plugin_result.entry_patterns.len(),
                                    deps = plugin_result.referenced_dependencies.len(),
                                    "resolved config"
                                );
                                process_config_result(plugin.name(), plugin_result, &mut result);
                            }
                        }
                    }
                }
            }

            // Phase 3b: Filesystem fallback for JSON config files.
            // JSON files (angular.json, project.json) are not in the discovered file set
            // because fallow only discovers JS/TS/CSS/Vue/etc. files.
            let json_configs = discover_json_config_files(
                &config_matchers,
                &resolved_plugins,
                &relative_files,
                root,
            );
            for (abs_path, plugin) in &json_configs {
                if let Ok(source) = std::fs::read_to_string(abs_path) {
                    let plugin_result = plugin.resolve_config(abs_path, &source, root);
                    if !plugin_result.is_empty() {
                        let rel = abs_path
                            .strip_prefix(root)
                            .map(|p| p.to_string_lossy())
                            .unwrap_or_default();
                        tracing::debug!(
                            plugin = plugin.name(),
                            config = %rel,
                            entries = plugin_result.entry_patterns.len(),
                            deps = plugin_result.referenced_dependencies.len(),
                            "resolved config (filesystem fallback)"
                        );
                        process_config_result(plugin.name(), plugin_result, &mut result);
                    }
                }
            }
        }

        // Phase 4: Package.json inline config fallback
        // For plugins that define `package_json_config_key()`, check if the root
        // package.json contains that key and no standalone config file was found.
        for plugin in &active {
            if let Some(key) = plugin.package_json_config_key()
                && !check_has_config_file(*plugin, &config_matchers, &relative_files)
            {
                // Try to extract the key from package.json
                let pkg_path = root.join("package.json");
                if let Ok(content) = std::fs::read_to_string(&pkg_path)
                    && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
                    && let Some(config_value) = json.get(key)
                {
                    let config_json = serde_json::to_string(config_value).unwrap_or_default();
                    let fake_path = root.join(format!("{key}.config.json"));
                    let plugin_result = plugin.resolve_config(&fake_path, &config_json, root);
                    if !plugin_result.is_empty() {
                        tracing::debug!(
                            plugin = plugin.name(),
                            key = key,
                            "resolved inline package.json config"
                        );
                        process_config_result(plugin.name(), plugin_result, &mut result);
                    }
                }
            }
        }

        result
    }

    /// Fast variant of `run()` for workspace packages.
    ///
    /// Reuses pre-compiled config matchers and pre-computed relative files from the root
    /// project run, avoiding repeated glob compilation and path computation per workspace.
    /// Skips external plugins (they only activate at root level) and package.json inline
    /// config (workspace packages rarely have inline configs).
    pub fn run_workspace_fast(
        &self,
        pkg: &PackageJson,
        root: &Path,
        project_root: &Path,
        precompiled_config_matchers: &[(&dyn Plugin, Vec<globset::GlobMatcher>)],
        relative_files: &[(&PathBuf, String)],
    ) -> AggregatedPluginResult {
        let _span = tracing::info_span!("run_plugins").entered();
        let mut result = AggregatedPluginResult::default();

        // Phase 1: Determine which plugins are active (with pre-computed deps)
        let all_deps = pkg.all_dependency_names();
        let active: Vec<&dyn Plugin> = self
            .plugins
            .iter()
            .filter(|p| p.is_enabled_with_deps(&all_deps, root))
            .map(|p| p.as_ref())
            .collect();

        tracing::info!(
            plugins = active
                .iter()
                .map(|p| p.name())
                .collect::<Vec<_>>()
                .join(", "),
            "active plugins"
        );

        // Early exit if no plugins are active (common for leaf workspace packages)
        if active.is_empty() {
            return result;
        }

        // Phase 2: Collect static patterns from active plugins
        for plugin in &active {
            process_static_patterns(*plugin, root, &mut result);
        }

        // Phase 3: Find and parse config files using pre-compiled matchers
        // Only check matchers for plugins that are active in this workspace
        let active_names: FxHashSet<&str> = active.iter().map(|p| p.name()).collect();
        let workspace_matchers: Vec<_> = precompiled_config_matchers
            .iter()
            .filter(|(p, _)| active_names.contains(p.name()))
            .collect();

        let mut resolved_ws_plugins: FxHashSet<&str> = FxHashSet::default();
        if !workspace_matchers.is_empty() {
            for (plugin, matchers) in &workspace_matchers {
                for (abs_path, rel_path) in relative_files {
                    if matchers.iter().any(|m| m.is_match(rel_path.as_str()))
                        && let Ok(source) = std::fs::read_to_string(abs_path)
                    {
                        // Mark resolved regardless of result to prevent Phase 3b
                        // from re-parsing a JSON config for the same plugin.
                        resolved_ws_plugins.insert(plugin.name());
                        let plugin_result = plugin.resolve_config(abs_path, &source, root);
                        if !plugin_result.is_empty() {
                            tracing::debug!(
                                plugin = plugin.name(),
                                config = rel_path.as_str(),
                                entries = plugin_result.entry_patterns.len(),
                                deps = plugin_result.referenced_dependencies.len(),
                                "resolved config"
                            );
                            process_config_result(plugin.name(), plugin_result, &mut result);
                        }
                    }
                }
            }
        }

        // Phase 3b: Filesystem fallback for JSON config files at the project root.
        // Config files like angular.json live at the monorepo root, but Angular is
        // only active in workspace packages. Check the project root for unresolved
        // config patterns.
        let mut ws_json_configs: Vec<(PathBuf, &dyn Plugin)> = Vec::new();
        let mut ws_seen_paths: FxHashSet<PathBuf> = FxHashSet::default();
        for plugin in &active {
            if resolved_ws_plugins.contains(plugin.name()) || plugin.config_patterns().is_empty() {
                continue;
            }
            for pat in plugin.config_patterns() {
                let has_glob = pat.contains("**") || pat.contains('*') || pat.contains('?');
                if !has_glob {
                    // Check both workspace root and project root (deduplicate when equal)
                    let check_roots: Vec<&Path> = if root == project_root {
                        vec![root]
                    } else {
                        vec![root, project_root]
                    };
                    for check_root in check_roots {
                        let abs_path = check_root.join(pat);
                        if abs_path.is_file() && ws_seen_paths.insert(abs_path.clone()) {
                            ws_json_configs.push((abs_path, *plugin));
                            break; // Found it — don't check other roots for this pattern
                        }
                    }
                } else {
                    // Glob pattern (e.g., "**/project.json") — check directories
                    // that contain discovered source files
                    let filename = std::path::Path::new(pat)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(pat);
                    let matcher = globset::Glob::new(pat).ok().map(|g| g.compile_matcher());
                    if let Some(matcher) = matcher {
                        let mut checked_dirs: FxHashSet<&Path> = FxHashSet::default();
                        checked_dirs.insert(root);
                        if root != project_root {
                            checked_dirs.insert(project_root);
                        }
                        for (abs_path, _) in relative_files {
                            if let Some(parent) = abs_path.parent() {
                                checked_dirs.insert(parent);
                            }
                        }
                        for dir in checked_dirs {
                            let candidate = dir.join(filename);
                            if candidate.is_file() && ws_seen_paths.insert(candidate.clone()) {
                                let rel = candidate
                                    .strip_prefix(project_root)
                                    .map(|p| p.to_string_lossy())
                                    .unwrap_or_default();
                                if matcher.is_match(rel.as_ref()) {
                                    ws_json_configs.push((candidate, *plugin));
                                }
                            }
                        }
                    }
                }
            }
        }
        // Parse discovered JSON config files
        for (abs_path, plugin) in &ws_json_configs {
            if let Ok(source) = std::fs::read_to_string(abs_path) {
                let plugin_result = plugin.resolve_config(abs_path, &source, root);
                if !plugin_result.is_empty() {
                    let rel = abs_path
                        .strip_prefix(project_root)
                        .map(|p| p.to_string_lossy())
                        .unwrap_or_default();
                    tracing::debug!(
                        plugin = plugin.name(),
                        config = %rel,
                        entries = plugin_result.entry_patterns.len(),
                        deps = plugin_result.referenced_dependencies.len(),
                        "resolved config (workspace filesystem fallback)"
                    );
                    process_config_result(plugin.name(), plugin_result, &mut result);
                }
            }
        }

        result
    }

    /// Pre-compile config pattern glob matchers for all plugins that have config patterns.
    /// Returns a vec of (plugin, matchers) pairs that can be reused across multiple `run_workspace_fast` calls.
    pub fn precompile_config_matchers(&self) -> Vec<(&dyn Plugin, Vec<globset::GlobMatcher>)> {
        self.plugins
            .iter()
            .filter(|p| !p.config_patterns().is_empty())
            .map(|p| {
                let matchers: Vec<globset::GlobMatcher> = p
                    .config_patterns()
                    .iter()
                    .filter_map(|pat| globset::Glob::new(pat).ok().map(|g| g.compile_matcher()))
                    .collect();
                (p.as_ref(), matchers)
            })
            .collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new(vec![])
    }
}

#[cfg(test)]
#[expect(clippy::disallowed_types)]
mod tests;
