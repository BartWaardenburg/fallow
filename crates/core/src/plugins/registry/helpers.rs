//! Helper functions for plugin registry orchestration.
//!
//! Contains pattern aggregation, external plugin processing, config file discovery,
//! config result merging, and plugin detection logic.

use std::path::{Path, PathBuf};

use rustc_hash::FxHashSet;

use fallow_config::{ExternalPluginDef, PluginDetection};

use super::super::{Plugin, PluginResult};
use super::AggregatedPluginResult;

/// Collect static patterns from a single plugin into the aggregated result.
pub fn process_static_patterns(
    plugin: &dyn Plugin,
    root: &Path,
    result: &mut AggregatedPluginResult,
) {
    result.active_plugins.push(plugin.name().to_string());

    let pname = plugin.name().to_string();
    for pat in plugin.entry_patterns() {
        result
            .entry_patterns
            .push(((*pat).to_string(), pname.clone()));
    }
    for pat in plugin.config_patterns() {
        result.config_patterns.push((*pat).to_string());
    }
    for pat in plugin.always_used() {
        result.always_used.push(((*pat).to_string(), pname.clone()));
    }
    for (file_pat, exports) in plugin.used_exports() {
        result.used_exports.push((
            file_pat.to_string(),
            exports.iter().map(|s| s.to_string()).collect(),
        ));
    }
    for dep in plugin.tooling_dependencies() {
        result.tooling_dependencies.push((*dep).to_string());
    }
    for prefix in plugin.virtual_module_prefixes() {
        result.virtual_module_prefixes.push((*prefix).to_string());
    }
    for (prefix, replacement) in plugin.path_aliases(root) {
        result.path_aliases.push((prefix.to_string(), replacement));
    }
}

/// Process external plugin definitions, checking activation and aggregating patterns.
pub fn process_external_plugins(
    external_plugins: &[ExternalPluginDef],
    all_deps: &[String],
    root: &Path,
    discovered_files: &[PathBuf],
    result: &mut AggregatedPluginResult,
) {
    let all_dep_refs: Vec<&str> = all_deps.iter().map(|s| s.as_str()).collect();
    for ext in external_plugins {
        let is_active = if let Some(detection) = &ext.detection {
            check_plugin_detection(detection, &all_dep_refs, root, discovered_files)
        } else if !ext.enablers.is_empty() {
            ext.enablers.iter().any(|enabler| {
                if enabler.ends_with('/') {
                    all_deps.iter().any(|d| d.starts_with(enabler))
                } else {
                    all_deps.iter().any(|d| d == enabler)
                }
            })
        } else {
            false
        };
        if is_active {
            result.active_plugins.push(ext.name.clone());
            result.entry_patterns.extend(
                ext.entry_points
                    .iter()
                    .map(|p| (p.clone(), ext.name.clone())),
            );
            // Track config patterns for introspection (not used for AST parsing —
            // external plugins cannot do resolve_config())
            result.config_patterns.extend(ext.config_patterns.clone());
            result.always_used.extend(
                ext.config_patterns
                    .iter()
                    .chain(ext.always_used.iter())
                    .map(|p| (p.clone(), ext.name.clone())),
            );
            result
                .tooling_dependencies
                .extend(ext.tooling_dependencies.clone());
            for ue in &ext.used_exports {
                result
                    .used_exports
                    .push((ue.pattern.clone(), ue.exports.clone()));
            }
        }
    }
}

/// Discover JSON config files on the filesystem for plugins that weren't matched against
/// discovered source files. Returns `(path, plugin)` pairs.
pub fn discover_json_config_files<'a>(
    config_matchers: &[(&'a dyn Plugin, Vec<globset::GlobMatcher>)],
    resolved_plugins: &FxHashSet<&str>,
    relative_files: &[(&PathBuf, String)],
    root: &Path,
) -> Vec<(PathBuf, &'a dyn Plugin)> {
    let mut json_configs: Vec<(PathBuf, &'a dyn Plugin)> = Vec::new();
    for (plugin, _) in config_matchers {
        if resolved_plugins.contains(plugin.name()) {
            continue;
        }
        for pat in plugin.config_patterns() {
            let has_glob = pat.contains("**") || pat.contains('*') || pat.contains('?');
            if !has_glob {
                // Simple pattern (e.g., "angular.json") — check at root
                let abs_path = root.join(pat);
                if abs_path.is_file() {
                    json_configs.push((abs_path, *plugin));
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
                    for (abs_path, _) in relative_files {
                        if let Some(parent) = abs_path.parent() {
                            checked_dirs.insert(parent);
                        }
                    }
                    for dir in checked_dirs {
                        let candidate = dir.join(filename);
                        if candidate.is_file() {
                            let rel = candidate
                                .strip_prefix(root)
                                .map(|p| p.to_string_lossy())
                                .unwrap_or_default();
                            if matcher.is_match(rel.as_ref()) {
                                json_configs.push((candidate, *plugin));
                            }
                        }
                    }
                }
            }
        }
    }
    json_configs
}

/// Merge a `PluginResult` from config parsing into the aggregated result.
pub fn process_config_result(
    plugin_name: &str,
    plugin_result: PluginResult,
    result: &mut AggregatedPluginResult,
) {
    let pname = plugin_name.to_string();
    result.entry_patterns.extend(
        plugin_result
            .entry_patterns
            .into_iter()
            .map(|p| (p, pname.clone())),
    );
    result
        .referenced_dependencies
        .extend(plugin_result.referenced_dependencies);
    result.discovered_always_used.extend(
        plugin_result
            .always_used_files
            .into_iter()
            .map(|p| (p, pname.clone())),
    );
    result.setup_files.extend(
        plugin_result
            .setup_files
            .into_iter()
            .map(|p| (p, pname.clone())),
    );
}

/// Check if a plugin already has a config file matched against discovered files.
pub fn check_has_config_file(
    plugin: &dyn Plugin,
    config_matchers: &[(&dyn Plugin, Vec<globset::GlobMatcher>)],
    relative_files: &[(&PathBuf, String)],
) -> bool {
    !plugin.config_patterns().is_empty()
        && config_matchers.iter().any(|(p, matchers)| {
            p.name() == plugin.name()
                && relative_files
                    .iter()
                    .any(|(_, rel)| matchers.iter().any(|m| m.is_match(rel.as_str())))
        })
}

/// Check if a `PluginDetection` condition is satisfied.
pub fn check_plugin_detection(
    detection: &PluginDetection,
    all_deps: &[&str],
    root: &Path,
    discovered_files: &[PathBuf],
) -> bool {
    match detection {
        PluginDetection::Dependency { package } => all_deps.iter().any(|d| *d == package),
        PluginDetection::FileExists { pattern } => {
            // Check against discovered files first (fast path)
            if let Ok(matcher) = globset::Glob::new(pattern).map(|g| g.compile_matcher()) {
                for file in discovered_files {
                    let relative = file.strip_prefix(root).unwrap_or(file);
                    if matcher.is_match(relative) {
                        return true;
                    }
                }
            }
            // Fall back to glob on disk for non-source files (e.g., config files)
            let full_pattern = root.join(pattern).to_string_lossy().to_string();
            glob::glob(&full_pattern)
                .ok()
                .is_some_and(|mut g| g.next().is_some())
        }
        PluginDetection::All { conditions } => conditions
            .iter()
            .all(|c| check_plugin_detection(c, all_deps, root, discovered_files)),
        PluginDetection::Any { conditions } => conditions
            .iter()
            .any(|c| check_plugin_detection(c, all_deps, root, discovered_files)),
    }
}
