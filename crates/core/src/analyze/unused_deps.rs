use std::path::Path;

use rustc_hash::{FxHashMap, FxHashSet};

use fallow_config::{PackageJson, ResolvedConfig};

use crate::discover::FileId;
use crate::graph::ModuleGraph;
use crate::resolve::ResolvedModule;
use crate::results::*;
use crate::suppress::{self, IssueKind, Suppression};

use super::predicates::{
    is_builtin_module, is_implicit_dependency, is_path_alias, is_virtual_module,
};
use super::{LineOffsetsMap, byte_offset_to_line_col};

/// Find the 1-based line number of a dependency key in a package.json file.
///
/// Searches the raw file content for `"<package_name>"` followed by `:` on the
/// same line. Skips JSONC comment lines. Returns 1 if not found (safe fallback).
fn find_dep_line_in_json(content: &str, package_name: &str) -> u32 {
    let needle = format!("\"{package_name}\"");
    let mut in_block_comment = false;
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        // Track block comments
        if in_block_comment {
            if let Some(end) = trimmed.find("*/") {
                // Block comment ends on this line — check the remainder after `*/`
                let rest = &trimmed[end + 2..];
                in_block_comment = false;
                if let Some(pos) = rest.find(&*needle) {
                    let after = &rest[pos + needle.len()..];
                    if after.trim_start().starts_with(':') {
                        return (i + 1) as u32;
                    }
                }
            }
            continue;
        }
        // Skip line comments
        if trimmed.starts_with("//") {
            continue;
        }
        // Start of block comment
        if let Some(after_open) = trimmed.strip_prefix("/*") {
            if let Some(end) = after_open.find("*/") {
                // Single-line block comment — check remainder after `*/`
                let rest = &after_open[end + 2..];
                if let Some(pos) = rest.find(&*needle) {
                    let after = &rest[pos + needle.len()..];
                    if after.trim_start().starts_with(':') {
                        return (i + 1) as u32;
                    }
                }
            } else {
                in_block_comment = true;
            }
            continue;
        }
        if let Some(pos) = line.find(&needle) {
            // Verify it's a key (followed by `:` after optional whitespace)
            let after = &line[pos + needle.len()..];
            if after.trim_start().starts_with(':') {
                return (i + 1) as u32;
            }
        }
    }
    1
}

/// Read a package.json file's raw text for line-number scanning.
fn read_pkg_json_content(pkg_path: &Path) -> Option<String> {
    std::fs::read_to_string(pkg_path).ok()
}

/// Find dependencies in package.json that are never imported.
///
/// Checks both the root package.json and each workspace's package.json.
/// For workspace deps, only files within that workspace are considered when
/// determining whether a dependency is used (mirroring `find_unlisted_dependencies`).
pub fn find_unused_dependencies(
    graph: &ModuleGraph,
    pkg: &PackageJson,
    config: &ResolvedConfig,
    plugin_result: Option<&crate::plugins::AggregatedPluginResult>,
    workspaces: &[fallow_config::WorkspaceInfo],
) -> (
    Vec<UnusedDependency>,
    Vec<UnusedDependency>,
    Vec<UnusedDependency>,
) {
    // Collect deps referenced in config files (discovered by plugins)
    let plugin_referenced: FxHashSet<&str> = plugin_result
        .map(|pr| {
            pr.referenced_dependencies
                .iter()
                .map(|s| s.as_str())
                .collect()
        })
        .unwrap_or_default();

    // Collect tooling deps from plugins
    let plugin_tooling: FxHashSet<&str> = plugin_result
        .map(|pr| pr.tooling_dependencies.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();

    // Collect packages used as binaries in package.json scripts
    let script_used: FxHashSet<&str> = plugin_result
        .map(|pr| pr.script_used_packages.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();

    // Collect workspace package names — these are internal deps, not npm packages
    let workspace_names: FxHashSet<&str> = workspaces.iter().map(|ws| ws.name.as_str()).collect();

    // Pre-compute ignore deps as FxHashSet for O(1) lookups instead of O(n) linear scan
    let ignore_deps: FxHashSet<&str> = config
        .ignore_dependencies
        .iter()
        .map(|s| s.as_str())
        .collect();

    // Build per-package set of files that use it (globally)
    let used_packages: FxHashSet<&str> = graph.package_usage.keys().map(|s| s.as_str()).collect();

    let root_pkg_path = config.root.join("package.json");
    let root_pkg_content = read_pkg_json_content(&root_pkg_path);

    // --- Root package.json check (existing behavior: any file can satisfy usage) ---
    let mut unused_deps: Vec<UnusedDependency> = pkg
        .production_dependency_names()
        .into_iter()
        .filter(|dep| !used_packages.contains(dep.as_str()))
        .filter(|dep| !script_used.contains(dep.as_str()))
        .filter(|dep| !is_implicit_dependency(dep))
        .filter(|dep| !plugin_referenced.contains(dep.as_str()))
        .filter(|dep| !plugin_tooling.contains(dep.as_str()))
        .filter(|dep| !ignore_deps.contains(dep.as_str()))
        .filter(|dep| !workspace_names.contains(dep.as_str()))
        .map(|dep| {
            let line = root_pkg_content
                .as_deref()
                .map_or(1, |c| find_dep_line_in_json(c, &dep));
            UnusedDependency {
                package_name: dep,
                location: DependencyLocation::Dependencies,
                path: root_pkg_path.clone(),
                line,
            }
        })
        .collect();

    let mut unused_dev_deps: Vec<UnusedDependency> = pkg
        .dev_dependency_names()
        .into_iter()
        .filter(|dep| !used_packages.contains(dep.as_str()))
        .filter(|dep| !script_used.contains(dep.as_str()))
        .filter(|dep| !crate::plugins::is_known_tooling_dependency(dep))
        .filter(|dep| !plugin_tooling.contains(dep.as_str()))
        .filter(|dep| !plugin_referenced.contains(dep.as_str()))
        .filter(|dep| !ignore_deps.contains(dep.as_str()))
        .filter(|dep| !workspace_names.contains(dep.as_str()))
        .map(|dep| {
            let line = root_pkg_content
                .as_deref()
                .map_or(1, |c| find_dep_line_in_json(c, &dep));
            UnusedDependency {
                package_name: dep,
                location: DependencyLocation::DevDependencies,
                path: root_pkg_path.clone(),
                line,
            }
        })
        .collect();

    let mut unused_optional_deps: Vec<UnusedDependency> = pkg
        .optional_dependency_names()
        .into_iter()
        .filter(|dep| !used_packages.contains(dep.as_str()))
        .filter(|dep| !script_used.contains(dep.as_str()))
        .filter(|dep| !is_implicit_dependency(dep))
        .filter(|dep| !plugin_referenced.contains(dep.as_str()))
        .filter(|dep| !ignore_deps.contains(dep.as_str()))
        .filter(|dep| !workspace_names.contains(dep.as_str()))
        .map(|dep| {
            let line = root_pkg_content
                .as_deref()
                .map_or(1, |c| find_dep_line_in_json(c, &dep));
            UnusedDependency {
                package_name: dep,
                location: DependencyLocation::OptionalDependencies,
                path: root_pkg_path.clone(),
                line,
            }
        })
        .collect();

    // --- Workspace package.json checks: scope usage to files within each workspace ---
    // Track which deps are already flagged from root to avoid double-reporting
    let root_flagged: FxHashSet<String> = unused_deps
        .iter()
        .chain(unused_dev_deps.iter())
        .chain(unused_optional_deps.iter())
        .map(|d| d.package_name.clone())
        .collect();

    for ws in workspaces {
        let ws_pkg_path = ws.root.join("package.json");
        let Ok(ws_pkg) = PackageJson::load(&ws_pkg_path) else {
            continue;
        };
        let ws_pkg_content = read_pkg_json_content(&ws_pkg_path);

        // Helper: check if a dependency is used by any file within this workspace.
        // Uses raw path comparison (module paths are absolute, workspace root is absolute)
        // to avoid per-file canonicalize() syscalls.
        let ws_root = &ws.root;
        let is_used_in_workspace = |dep: &str| -> bool {
            graph.package_usage.get(dep).is_some_and(|file_ids| {
                file_ids.iter().any(|id| {
                    graph
                        .modules
                        .get(id.0 as usize)
                        .is_some_and(|module| module.path.starts_with(ws_root))
                })
            })
        };

        // Check workspace production dependencies
        for dep in ws_pkg.production_dependency_names() {
            if should_skip_dependency(
                &dep,
                &root_flagged,
                &script_used,
                &plugin_referenced,
                &ignore_deps,
                &workspace_names,
                is_used_in_workspace,
            ) || is_implicit_dependency(&dep)
                || plugin_tooling.contains(dep.as_str())
            {
                continue;
            }
            let line = ws_pkg_content
                .as_deref()
                .map_or(1, |c| find_dep_line_in_json(c, &dep));
            unused_deps.push(UnusedDependency {
                package_name: dep,
                location: DependencyLocation::Dependencies,
                path: ws_pkg_path.clone(),
                line,
            });
        }

        // Check workspace dev dependencies
        for dep in ws_pkg.dev_dependency_names() {
            if should_skip_dependency(
                &dep,
                &root_flagged,
                &script_used,
                &plugin_referenced,
                &ignore_deps,
                &workspace_names,
                is_used_in_workspace,
            ) || crate::plugins::is_known_tooling_dependency(&dep)
                || plugin_tooling.contains(dep.as_str())
            {
                continue;
            }
            let line = ws_pkg_content
                .as_deref()
                .map_or(1, |c| find_dep_line_in_json(c, &dep));
            unused_dev_deps.push(UnusedDependency {
                package_name: dep,
                location: DependencyLocation::DevDependencies,
                path: ws_pkg_path.clone(),
                line,
            });
        }

        // Check workspace optional dependencies
        for dep in ws_pkg.optional_dependency_names() {
            if should_skip_dependency(
                &dep,
                &root_flagged,
                &script_used,
                &plugin_referenced,
                &ignore_deps,
                &workspace_names,
                is_used_in_workspace,
            ) || is_implicit_dependency(&dep)
            {
                continue;
            }
            let line = ws_pkg_content
                .as_deref()
                .map_or(1, |c| find_dep_line_in_json(c, &dep));
            unused_optional_deps.push(UnusedDependency {
                package_name: dep,
                location: DependencyLocation::OptionalDependencies,
                path: ws_pkg_path.clone(),
                line,
            });
        }
    }

    (unused_deps, unused_dev_deps, unused_optional_deps)
}

/// Check if a dependency should be skipped during unused dependency analysis.
///
/// Shared guard conditions for both production and dev dependency loops:
/// already flagged from root, used in scripts, referenced by plugins, in ignore list,
/// is a workspace package, or used by files in the workspace.
fn should_skip_dependency(
    dep: &str,
    root_flagged: &FxHashSet<String>,
    script_used: &FxHashSet<&str>,
    plugin_referenced: &FxHashSet<&str>,
    ignore_deps: &FxHashSet<&str>,
    workspace_names: &FxHashSet<&str>,
    is_used_in_workspace: impl Fn(&str) -> bool,
) -> bool {
    root_flagged.contains(dep)
        || script_used.contains(dep)
        || plugin_referenced.contains(dep)
        || ignore_deps.contains(dep)
        || workspace_names.contains(dep)
        || is_used_in_workspace(dep)
}

/// Find production dependencies that are only imported via type-only imports.
///
/// In production mode, `import type { Foo } from 'pkg'` is erased at compile time,
/// meaning the dependency is not needed at runtime. Such dependencies should be
/// moved to devDependencies.
pub fn find_type_only_dependencies(
    graph: &ModuleGraph,
    pkg: &PackageJson,
    config: &ResolvedConfig,
    workspaces: &[fallow_config::WorkspaceInfo],
) -> Vec<TypeOnlyDependency> {
    let root_pkg_path = config.root.join("package.json");
    let root_pkg_content = read_pkg_json_content(&root_pkg_path);
    let workspace_names: FxHashSet<&str> = workspaces.iter().map(|ws| ws.name.as_str()).collect();

    let mut type_only_deps = Vec::new();

    // Check root production dependencies
    for dep in pkg.production_dependency_names() {
        // Skip internal workspace packages
        if workspace_names.contains(dep.as_str()) {
            continue;
        }
        // Skip ignored dependencies
        if config.ignore_dependencies.iter().any(|d| d == &dep) {
            continue;
        }

        let has_any_usage = graph.package_usage.contains_key(dep.as_str());
        let has_type_only_usage = graph.type_only_package_usage.contains_key(dep.as_str());

        if !has_any_usage {
            // Not used at all — this will be caught by unused_dependencies
            continue;
        }

        // Check if ALL usages are type-only: the number of type-only usages must equal
        // the total number of usages for this package
        let total_count = graph.package_usage.get(dep.as_str()).map_or(0, Vec::len);
        let type_only_count = graph
            .type_only_package_usage
            .get(dep.as_str())
            .map_or(0, Vec::len);

        if has_type_only_usage && type_only_count == total_count {
            let line = root_pkg_content
                .as_deref()
                .map_or(1, |c| find_dep_line_in_json(c, &dep));
            type_only_deps.push(TypeOnlyDependency {
                package_name: dep,
                path: root_pkg_path.clone(),
                line,
            });
        }
    }

    type_only_deps
}

/// Find dependencies used in imports but not listed in package.json.
pub fn find_unlisted_dependencies(
    graph: &ModuleGraph,
    pkg: &PackageJson,
    config: &ResolvedConfig,
    workspaces: &[fallow_config::WorkspaceInfo],
    plugin_result: Option<&crate::plugins::AggregatedPluginResult>,
    resolved_modules: &[ResolvedModule],
    line_offsets_by_file: &LineOffsetsMap<'_>,
) -> Vec<UnlistedDependency> {
    let all_deps: FxHashSet<String> = pkg.all_dependency_names().into_iter().collect();

    // Build a set of all deps across all workspace package.json files.
    // In monorepos, imports in workspace files reference deps from that workspace's package.json.
    let mut all_workspace_deps: FxHashSet<String> = all_deps.clone();
    // Also collect workspace package names — internal workspace deps should not be flagged
    let mut workspace_names: FxHashSet<String> = FxHashSet::default();
    // Map: canonical workspace root -> set of dep names (for per-file checks)
    let mut ws_dep_map: Vec<(std::path::PathBuf, FxHashSet<String>)> = Vec::new();

    for ws in workspaces {
        workspace_names.insert(ws.name.clone());
        let ws_pkg_path = ws.root.join("package.json");
        if let Ok(ws_pkg) = PackageJson::load(&ws_pkg_path) {
            let ws_deps: FxHashSet<String> = ws_pkg.all_dependency_names().into_iter().collect();
            all_workspace_deps.extend(ws_deps.iter().cloned());
            // Use raw workspace root path for starts_with checks (avoids per-file canonicalize)
            ws_dep_map.push((ws.root.clone(), ws_deps));
        }
    }

    // Collect virtual module prefixes from active plugins (e.g., Docusaurus @theme/, @site/)
    let virtual_prefixes: Vec<&str> = plugin_result
        .map(|pr| {
            pr.virtual_module_prefixes
                .iter()
                .map(|s| s.as_str())
                .collect()
        })
        .unwrap_or_default();

    // Collect tooling dependencies from active plugins — these are framework-provided
    // packages (e.g., Nuxt provides `ofetch`, `h3`, `vue-router` at runtime) that may
    // be imported in user code without being listed in package.json.
    let plugin_tooling: FxHashSet<&str> = plugin_result
        .map(|pr| pr.tooling_dependencies.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();

    // Build a lookup: FileId -> Vec<(package_name, span_start)> from resolved modules,
    // so we can recover the import location when building UnlistedDependency results.
    let mut import_spans_by_file: FxHashMap<FileId, Vec<(&str, u32)>> = FxHashMap::default();
    for rm in resolved_modules {
        for import in &rm.resolved_imports {
            if let crate::resolve::ResolveResult::NpmPackage(name) = &import.target {
                import_spans_by_file
                    .entry(rm.file_id)
                    .or_default()
                    .push((name.as_str(), import.info.span.start));
            }
        }
        // Re-exports don't have span info on ReExportInfo, so skip them here.
        // The import span lookup will fall back to (1, 0) for re-export-only usages.
    }

    let mut unlisted: FxHashMap<String, Vec<ImportSite>> = FxHashMap::default();

    for (package_name, file_ids) in &graph.package_usage {
        if is_builtin_module(package_name) || is_path_alias(package_name) {
            continue;
        }
        // Skip virtual module imports (e.g., `virtual:pwa-register`, `virtual:uno.css`)
        // created by Vite plugins and similar build tools
        if is_virtual_module(package_name) {
            continue;
        }
        // Skip internal workspace package names
        if workspace_names.contains(package_name) {
            continue;
        }
        // Skip framework-provided dependencies declared by active plugins
        if plugin_tooling.contains(package_name.as_str()) {
            continue;
        }
        // Skip virtual module imports provided by active framework plugins
        if virtual_prefixes
            .iter()
            .any(|prefix| package_name.starts_with(prefix))
        {
            continue;
        }
        // Quick check: if listed in any root or workspace deps, skip
        if all_workspace_deps.contains(package_name) {
            continue;
        }

        // Slower fallback: check if each importing file belongs to a workspace that lists this dep.
        // Uses raw path comparison (module paths are absolute) to avoid per-file canonicalize().
        let mut unlisted_sites: Vec<ImportSite> = Vec::new();
        for id in file_ids {
            if let Some(module) = graph.modules.get(id.0 as usize) {
                let listed_in_ws = ws_dep_map.iter().any(|(ws_root, ws_deps)| {
                    module.path.starts_with(ws_root) && ws_deps.contains(package_name)
                });
                // Also check root deps
                let listed_in_root = all_deps.contains(package_name);
                if !listed_in_ws && !listed_in_root {
                    // Look up the import span for this package in this file
                    let (line, col) = import_spans_by_file
                        .get(id)
                        .and_then(|spans| {
                            spans.iter().find(|(name, _)| *name == package_name).map(
                                |(_, span_start)| {
                                    byte_offset_to_line_col(line_offsets_by_file, *id, *span_start)
                                },
                            )
                        })
                        .unwrap_or((1, 0));

                    unlisted_sites.push(ImportSite {
                        path: module.path.clone(),
                        line,
                        col,
                    });
                }
            }
        }

        if !unlisted_sites.is_empty() {
            unlisted_sites.sort_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));
            unlisted_sites.dedup_by(|a, b| a.path == b.path);
            unlisted.insert(package_name.clone(), unlisted_sites);
        }
    }

    let _ = config; // future use
    unlisted
        .into_iter()
        .map(|(name, sites)| UnlistedDependency {
            package_name: name,
            imported_from: sites,
        })
        .collect()
}

/// Find imports that could not be resolved.
pub fn find_unresolved_imports(
    resolved_modules: &[ResolvedModule],
    _config: &ResolvedConfig,
    suppressions_by_file: &FxHashMap<FileId, &[Suppression]>,
    virtual_prefixes: &[&str],
    line_offsets_by_file: &LineOffsetsMap<'_>,
) -> Vec<UnresolvedImport> {
    let mut unresolved = Vec::new();

    for module in resolved_modules {
        for import in &module.resolved_imports {
            if let crate::resolve::ResolveResult::Unresolvable(spec) = &import.target {
                // Skip virtual module imports using the `virtual:` convention
                // (e.g., `virtual:pwa-register`, `virtual:uno.css`)
                if is_virtual_module(spec) {
                    continue;
                }
                // Skip virtual module imports provided by active framework plugins
                // (e.g., Nuxt's #imports, #app, #components, #build).
                if virtual_prefixes
                    .iter()
                    .any(|prefix| spec.starts_with(prefix))
                {
                    continue;
                }

                let (line, col) = byte_offset_to_line_col(
                    line_offsets_by_file,
                    module.file_id,
                    import.info.span.start,
                );

                // Check inline suppression
                if let Some(supps) = suppressions_by_file.get(&module.file_id)
                    && suppress::is_suppressed(supps, line, IssueKind::UnresolvedImport)
                {
                    continue;
                }

                unresolved.push(UnresolvedImport {
                    path: module.path.clone(),
                    specifier: spec.clone(),
                    line,
                    col,
                });
            }
        }
    }

    unresolved
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- should_skip_dependency tests ----

    type SkipDepSets = (
        FxHashSet<String>,
        FxHashSet<&'static str>,
        FxHashSet<&'static str>,
        FxHashSet<&'static str>,
        FxHashSet<&'static str>,
    );

    /// Helper: build empty sets for should_skip_dependency args.
    fn empty_sets() -> SkipDepSets {
        (
            FxHashSet::default(),
            FxHashSet::default(),
            FxHashSet::default(),
            FxHashSet::default(),
            FxHashSet::default(),
        )
    }

    #[test]
    fn skip_dep_returns_false_when_no_guard_matches() {
        let (root_flagged, script_used, plugin_referenced, ignore_deps, workspace_names) =
            empty_sets();
        let result = should_skip_dependency(
            "some-package",
            &root_flagged,
            &script_used,
            &plugin_referenced,
            &ignore_deps,
            &workspace_names,
            |_| false,
        );
        assert!(!result);
    }

    #[test]
    fn skip_dep_when_root_flagged() {
        let (mut root_flagged, script_used, plugin_referenced, ignore_deps, workspace_names) =
            empty_sets();
        root_flagged.insert("lodash".to_string());
        assert!(should_skip_dependency(
            "lodash",
            &root_flagged,
            &script_used,
            &plugin_referenced,
            &ignore_deps,
            &workspace_names,
            |_| false,
        ));
    }

    #[test]
    fn skip_dep_when_script_used() {
        let (root_flagged, mut script_used, plugin_referenced, ignore_deps, workspace_names) =
            empty_sets();
        script_used.insert("eslint");
        assert!(should_skip_dependency(
            "eslint",
            &root_flagged,
            &script_used,
            &plugin_referenced,
            &ignore_deps,
            &workspace_names,
            |_| false,
        ));
    }

    #[test]
    fn skip_dep_when_plugin_referenced() {
        let (root_flagged, script_used, mut plugin_referenced, ignore_deps, workspace_names) =
            empty_sets();
        plugin_referenced.insert("tailwindcss");
        assert!(should_skip_dependency(
            "tailwindcss",
            &root_flagged,
            &script_used,
            &plugin_referenced,
            &ignore_deps,
            &workspace_names,
            |_| false,
        ));
    }

    #[test]
    fn skip_dep_when_in_ignore_list() {
        let (root_flagged, script_used, plugin_referenced, mut ignore_deps, workspace_names) =
            empty_sets();
        ignore_deps.insert("my-internal-package");
        assert!(should_skip_dependency(
            "my-internal-package",
            &root_flagged,
            &script_used,
            &plugin_referenced,
            &ignore_deps,
            &workspace_names,
            |_| false,
        ));
    }

    #[test]
    fn skip_dep_when_workspace_name() {
        let (root_flagged, script_used, plugin_referenced, ignore_deps, mut workspace_names) =
            empty_sets();
        workspace_names.insert("@myorg/shared");
        assert!(should_skip_dependency(
            "@myorg/shared",
            &root_flagged,
            &script_used,
            &plugin_referenced,
            &ignore_deps,
            &workspace_names,
            |_| false,
        ));
    }

    #[test]
    fn skip_dep_when_used_in_workspace() {
        let (root_flagged, script_used, plugin_referenced, ignore_deps, workspace_names) =
            empty_sets();
        assert!(should_skip_dependency(
            "react",
            &root_flagged,
            &script_used,
            &plugin_referenced,
            &ignore_deps,
            &workspace_names,
            |dep| dep == "react",
        ));
    }

    #[test]
    fn skip_dep_closure_receives_correct_dep_name() {
        let (root_flagged, script_used, plugin_referenced, ignore_deps, workspace_names) =
            empty_sets();
        // Closure that only returns true for "axios"
        let result = should_skip_dependency(
            "axios",
            &root_flagged,
            &script_used,
            &plugin_referenced,
            &ignore_deps,
            &workspace_names,
            |dep| dep == "axios",
        );
        assert!(result);

        // Different dep name should not match
        let result = should_skip_dependency(
            "express",
            &root_flagged,
            &script_used,
            &plugin_referenced,
            &ignore_deps,
            &workspace_names,
            |dep| dep == "axios",
        );
        assert!(!result);
    }

    #[test]
    fn skip_dep_no_match_with_similar_names() {
        let (mut root_flagged, script_used, plugin_referenced, ignore_deps, workspace_names) =
            empty_sets();
        root_flagged.insert("lodash-es".to_string());
        // "lodash" is not the same as "lodash-es"
        assert!(!should_skip_dependency(
            "lodash",
            &root_flagged,
            &script_used,
            &plugin_referenced,
            &ignore_deps,
            &workspace_names,
            |_| false,
        ));
    }

    #[test]
    fn skip_dep_multiple_guards_match() {
        // When multiple guards would match, function still returns true
        let (mut root_flagged, mut script_used, plugin_referenced, ignore_deps, workspace_names) =
            empty_sets();
        root_flagged.insert("eslint".to_string());
        script_used.insert("eslint");
        assert!(should_skip_dependency(
            "eslint",
            &root_flagged,
            &script_used,
            &plugin_referenced,
            &ignore_deps,
            &workspace_names,
            |_| false,
        ));
    }

    // ---- is_builtin_module tests (via predicates, used in find_unlisted_dependencies) ----

    #[test]
    fn builtin_module_subpaths() {
        assert!(super::super::predicates::is_builtin_module("fs/promises"));
        assert!(super::super::predicates::is_builtin_module(
            "stream/consumers"
        ));
        assert!(super::super::predicates::is_builtin_module(
            "node:fs/promises"
        ));
        assert!(super::super::predicates::is_builtin_module(
            "readline/promises"
        ));
    }

    #[test]
    fn builtin_module_cloudflare_workers() {
        assert!(super::super::predicates::is_builtin_module(
            "cloudflare:workers"
        ));
        assert!(super::super::predicates::is_builtin_module(
            "cloudflare:sockets"
        ));
    }

    #[test]
    fn builtin_module_deno_std() {
        assert!(super::super::predicates::is_builtin_module("std"));
        assert!(super::super::predicates::is_builtin_module("std/path"));
    }

    // ---- is_implicit_dependency tests (used in find_unused_dependencies) ----

    #[test]
    fn implicit_dep_react_dom() {
        assert!(super::super::predicates::is_implicit_dependency(
            "react-dom"
        ));
        assert!(super::super::predicates::is_implicit_dependency(
            "react-dom/client"
        ));
    }

    #[test]
    fn implicit_dep_next_packages() {
        assert!(super::super::predicates::is_implicit_dependency(
            "@next/font"
        ));
        assert!(super::super::predicates::is_implicit_dependency(
            "@next/mdx"
        ));
        assert!(super::super::predicates::is_implicit_dependency(
            "@next/bundle-analyzer"
        ));
        assert!(super::super::predicates::is_implicit_dependency(
            "@next/env"
        ));
    }

    #[test]
    fn implicit_dep_websocket_addons() {
        assert!(super::super::predicates::is_implicit_dependency(
            "utf-8-validate"
        ));
        assert!(super::super::predicates::is_implicit_dependency(
            "bufferutil"
        ));
    }

    // ---- is_path_alias tests (used in find_unlisted_dependencies) ----

    #[test]
    fn path_alias_not_reported_as_unlisted() {
        // These should be detected as path aliases and skipped
        assert!(super::super::predicates::is_path_alias("@/components/Foo"));
        assert!(super::super::predicates::is_path_alias("~/utils/helper"));
        assert!(super::super::predicates::is_path_alias("#internal/auth"));
        assert!(super::super::predicates::is_path_alias(
            "@Components/Button"
        ));
    }

    #[test]
    fn scoped_npm_packages_not_path_aliases() {
        assert!(!super::super::predicates::is_path_alias("@angular/core"));
        assert!(!super::super::predicates::is_path_alias("@emotion/react"));
        assert!(!super::super::predicates::is_path_alias("@nestjs/common"));
    }

    // ---- find_dep_line_in_json tests ----

    #[test]
    fn find_dep_line_finds_dependency_key() {
        let content = r#"{
  "name": "my-app",
  "dependencies": {
    "react": "^18.0.0",
    "lodash": "^4.17.21"
  }
}"#;
        assert_eq!(super::find_dep_line_in_json(content, "lodash"), 5);
        assert_eq!(super::find_dep_line_in_json(content, "react"), 4);
    }

    #[test]
    fn find_dep_line_returns_1_when_not_found() {
        let content = r#"{ "dependencies": {} }"#;
        assert_eq!(super::find_dep_line_in_json(content, "missing"), 1);
    }

    #[test]
    fn find_dep_line_handles_scoped_packages() {
        let content = r#"{
  "devDependencies": {
    "@typescript-eslint/parser": "^6.0.0"
  }
}"#;
        assert_eq!(
            super::find_dep_line_in_json(content, "@typescript-eslint/parser"),
            3
        );
    }

    #[test]
    fn find_dep_line_skips_line_comments() {
        let content = r#"{
  // "lodash": "old version",
  "dependencies": {
    "lodash": "^4.17.21"
  }
}"#;
        assert_eq!(super::find_dep_line_in_json(content, "lodash"), 4);
    }

    #[test]
    fn find_dep_line_skips_block_comments() {
        let content = r#"{
  /* "lodash": "old" */
  "dependencies": {
    "lodash": "^4.17.21"
  }
}"#;
        assert_eq!(super::find_dep_line_in_json(content, "lodash"), 4);
    }

    // ---- find_dep_line_in_json: multi-line block comment coverage ----

    #[test]
    fn find_dep_line_skips_multiline_block_comment() {
        let content = r#"{
  /*
    "lodash": "commented out",
    "react": "also commented"
  */
  "dependencies": {
    "lodash": "^4.17.21"
  }
}"#;
        // "lodash" inside the multi-line block comment is skipped; real one is on line 7
        assert_eq!(find_dep_line_in_json(content, "lodash"), 7);
    }

    #[test]
    fn find_dep_line_after_block_comment_end_on_same_line() {
        // Single-line block comment: the remainder after `*/` is scanned for the dep key.
        let content = r#"{
  /* comment */ "lodash": "^4.17.21"
}"#;
        assert_eq!(find_dep_line_in_json(content, "lodash"), 2);
    }

    #[test]
    fn find_dep_line_dep_inside_and_after_block_comment() {
        // The dep name appears inside the comment AND as a real key after it.
        // Must match the post-comment occurrence, not the in-comment one.
        let content = "{\n  /* \"lodash\": \"old\" */ \"lodash\": \"^4.17.21\"\n}";
        assert_eq!(find_dep_line_in_json(content, "lodash"), 2);
    }

    #[test]
    fn find_dep_line_minimal_block_comment() {
        // Minimal block comment `/**/` followed by a dep key.
        let content = "{\n  /**/ \"lodash\": \"^4.17.21\"\n}";
        assert_eq!(find_dep_line_in_json(content, "lodash"), 2);
    }

    #[test]
    fn find_dep_line_multiline_block_comment_end_with_dep_on_remainder() {
        // Tests the branch where a multi-line block comment ends and the dep key
        // appears on the remainder of the same line after "*/"
        let content = "{\n  /* start of comment\n  end */ \"lodash\": \"^4.17.21\"\n}";
        // Line 1: {
        // Line 2: /* start of comment    <-- sets in_block_comment = true
        // Line 3: end */ "lodash": "^4.17.21"  <-- comment ends, remainder has dep
        assert_eq!(find_dep_line_in_json(content, "lodash"), 3);
    }

    #[test]
    fn find_dep_line_block_comment_end_without_dep_on_remainder() {
        // Block comment ends but the remainder does NOT have the dep key
        let content =
            "{\n  /* start\n  end */ \"other\": \"1.0.0\",\n  \"lodash\": \"^4.17.21\"\n}";
        // The dep "lodash" is on line 4, after the block comment ends on line 3
        assert_eq!(find_dep_line_in_json(content, "lodash"), 4);
    }

    #[test]
    fn find_dep_line_value_not_key_is_not_matched() {
        // "lodash" appears as a VALUE, not a key -- should not match
        let content = r#"{
  "dependencies": {
    "my-lodash-wrapper": "lodash"
  }
}"#;
        // "lodash" appears in the value but NOT as a key (not followed by ":")
        // "my-lodash-wrapper" IS a key.
        assert_eq!(find_dep_line_in_json(content, "lodash"), 1);
        assert_eq!(find_dep_line_in_json(content, "my-lodash-wrapper"), 3);
    }

    #[test]
    fn find_dep_line_empty_content() {
        assert_eq!(find_dep_line_in_json("", "lodash"), 1);
    }

    #[test]
    fn find_dep_line_multiple_dep_sections() {
        let content = r#"{
  "dependencies": {
    "react": "^18.0.0"
  },
  "devDependencies": {
    "react": "^18.0.0"
  }
}"#;
        // Should find the FIRST occurrence (line 3)
        assert_eq!(find_dep_line_in_json(content, "react"), 3);
    }

    // ---- Integration tests for find_unused_dependencies ----
    //
    // These tests construct a ModuleGraph via ModuleGraph::build() with
    // minimal resolved modules, then call find_unused_dependencies to
    // verify that the correct deps are flagged.

    use std::path::PathBuf;

    use fallow_config::{FallowConfig, OutputFormat, WorkspaceInfo};
    use fallow_types::discover::{DiscoveredFile, EntryPoint, EntryPointSource, FileId};
    use fallow_types::extract::{ImportInfo, ImportedName};

    use crate::graph::ModuleGraph;
    use crate::plugins::AggregatedPluginResult;
    use crate::resolve::{ResolveResult, ResolvedImport, ResolvedModule};

    /// Build a minimal ResolvedConfig for testing.
    fn test_config(root: PathBuf) -> ResolvedConfig {
        FallowConfig {
            schema: None,
            extends: vec![],
            entry: vec![],
            ignore_patterns: vec![],
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            duplicates: fallow_config::DuplicatesConfig::default(),
            health: fallow_config::HealthConfig::default(),
            rules: fallow_config::RulesConfig::default(),
            production: false,
            plugins: vec![],
            overrides: vec![],
        }
        .resolve(root, OutputFormat::Human, 1, true, true)
    }

    /// Build a PackageJson with specific dependency fields via JSON deserialization.
    /// This avoids directly constructing `std::collections::HashMap` (clippy disallowed type).
    fn make_pkg(deps: &[&str], dev_deps: &[&str], optional_deps: &[&str]) -> PackageJson {
        let to_obj = |names: &[&str]| -> serde_json::Value {
            let map: serde_json::Map<String, serde_json::Value> = names
                .iter()
                .map(|n| {
                    (
                        n.to_string(),
                        serde_json::Value::String("^1.0.0".to_string()),
                    )
                })
                .collect();
            serde_json::Value::Object(map)
        };

        let mut obj = serde_json::Map::new();
        obj.insert(
            "name".to_string(),
            serde_json::Value::String("test-project".to_string()),
        );
        if !deps.is_empty() {
            obj.insert("dependencies".to_string(), to_obj(deps));
        }
        if !dev_deps.is_empty() {
            obj.insert("devDependencies".to_string(), to_obj(dev_deps));
        }
        if !optional_deps.is_empty() {
            obj.insert("optionalDependencies".to_string(), to_obj(optional_deps));
        }
        serde_json::from_value(serde_json::Value::Object(obj))
            .expect("test PackageJson should deserialize")
    }

    /// Build a minimal graph where a single entry file imports given npm packages.
    fn build_graph_with_npm_imports(
        npm_packages: &[(&str, bool)], // (package_name, is_type_only)
    ) -> (ModuleGraph, Vec<ResolvedModule>) {
        let files = vec![DiscoveredFile {
            id: FileId(0),
            path: PathBuf::from("/project/src/index.ts"),
            size_bytes: 100,
        }];

        let entry_points = vec![EntryPoint {
            path: PathBuf::from("/project/src/index.ts"),
            source: EntryPointSource::PackageJsonMain,
        }];

        let resolved_imports: Vec<ResolvedImport> = npm_packages
            .iter()
            .enumerate()
            .map(|(i, (name, is_type_only))| ResolvedImport {
                info: ImportInfo {
                    source: name.to_string(),
                    imported_name: ImportedName::Named("default".to_string()),
                    local_name: format!("import_{i}"),
                    is_type_only: *is_type_only,
                    span: oxc_span::Span::new((i * 20) as u32, (i * 20 + 15) as u32),
                },
                target: ResolveResult::NpmPackage(name.to_string()),
            })
            .collect();

        let resolved_modules = vec![ResolvedModule {
            file_id: FileId(0),
            path: PathBuf::from("/project/src/index.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports,
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        }];

        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);
        (graph, resolved_modules)
    }

    // ---- find_unused_dependencies integration tests ----

    #[test]
    fn unused_dep_flagged_when_never_imported() {
        let (graph, _) = build_graph_with_npm_imports(&[("react", false)]);
        let pkg = make_pkg(&["react", "lodash"], &[], &[]);
        let config = test_config(PathBuf::from("/project"));

        let (unused, unused_dev, unused_optional) =
            find_unused_dependencies(&graph, &pkg, &config, None, &[]);

        assert!(
            unused.iter().any(|d| d.package_name == "lodash"),
            "lodash is never imported and should be flagged"
        );
        assert!(
            !unused.iter().any(|d| d.package_name == "react"),
            "react is imported and should NOT be flagged"
        );
        assert!(unused_dev.is_empty());
        assert!(unused_optional.is_empty());
    }

    #[test]
    fn known_tooling_dev_deps_not_flagged_as_unused() {
        let (graph, _) = build_graph_with_npm_imports(&[]);
        let pkg = make_pkg(&[], &["jest", "vitest"], &[]);
        let config = test_config(PathBuf::from("/project"));

        let (unused, unused_dev, _) = find_unused_dependencies(&graph, &pkg, &config, None, &[]);

        assert!(unused.is_empty());
        // "jest" and "vitest" are known tooling deps, so they should NOT be flagged
        assert!(
            !unused_dev.iter().any(|d| d.package_name == "jest"),
            "jest is a known tooling dep and should be filtered"
        );
        assert!(
            !unused_dev.iter().any(|d| d.package_name == "vitest"),
            "vitest is a known tooling dep and should be filtered"
        );
    }

    #[test]
    fn unused_dev_dep_non_tooling_is_flagged() {
        let (graph, _) = build_graph_with_npm_imports(&[]);
        let pkg = make_pkg(&[], &["my-custom-lib"], &[]);
        let config = test_config(PathBuf::from("/project"));

        let (_, unused_dev, _) = find_unused_dependencies(&graph, &pkg, &config, None, &[]);

        assert!(
            unused_dev.iter().any(|d| d.package_name == "my-custom-lib"),
            "non-tooling dev dep should be flagged as unused"
        );
    }

    #[test]
    fn unused_optional_dep_flagged_when_never_imported() {
        let (graph, _) = build_graph_with_npm_imports(&[]);
        let pkg = make_pkg(&[], &[], &["sharp"]);
        let config = test_config(PathBuf::from("/project"));

        let (_, _, unused_optional) = find_unused_dependencies(&graph, &pkg, &config, None, &[]);

        assert!(
            unused_optional.iter().any(|d| d.package_name == "sharp"),
            "unused optional dep should be flagged"
        );
    }

    #[test]
    fn implicit_deps_not_flagged_as_unused() {
        // react-dom, @types/node, etc. are implicit and should be filtered
        let (graph, _) = build_graph_with_npm_imports(&[]);
        let pkg = make_pkg(&["react-dom", "@types/node"], &[], &[]);
        let config = test_config(PathBuf::from("/project"));

        let (unused, _, _) = find_unused_dependencies(&graph, &pkg, &config, None, &[]);

        assert!(
            !unused.iter().any(|d| d.package_name == "react-dom"),
            "react-dom is implicit and should not be flagged"
        );
        assert!(
            !unused.iter().any(|d| d.package_name == "@types/node"),
            "@types/node is implicit and should not be flagged"
        );
    }

    #[test]
    fn workspace_package_names_not_flagged() {
        let (graph, _) = build_graph_with_npm_imports(&[]);
        let pkg = make_pkg(&["@myorg/shared"], &[], &[]);
        let config = test_config(PathBuf::from("/project"));

        let workspaces = vec![WorkspaceInfo {
            root: PathBuf::from("/project/packages/shared"),
            name: "@myorg/shared".to_string(),
            is_internal_dependency: false,
        }];

        let (unused, _, _) = find_unused_dependencies(&graph, &pkg, &config, None, &workspaces);

        assert!(
            !unused.iter().any(|d| d.package_name == "@myorg/shared"),
            "workspace packages should not be flagged as unused"
        );
    }

    #[test]
    fn ignore_dependencies_config_filters_deps() {
        let (graph, _) = build_graph_with_npm_imports(&[]);
        let pkg = make_pkg(&["my-internal-pkg"], &[], &[]);

        let config = FallowConfig {
            schema: None,
            extends: vec![],
            entry: vec![],
            ignore_patterns: vec![],
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec!["my-internal-pkg".to_string()],
            ignore_exports: vec![],
            duplicates: fallow_config::DuplicatesConfig::default(),
            health: fallow_config::HealthConfig::default(),
            rules: fallow_config::RulesConfig::default(),
            production: false,
            plugins: vec![],
            overrides: vec![],
        }
        .resolve(
            PathBuf::from("/project"),
            OutputFormat::Human,
            1,
            true,
            true,
        );

        let (unused, _, _) = find_unused_dependencies(&graph, &pkg, &config, None, &[]);

        assert!(
            !unused.iter().any(|d| d.package_name == "my-internal-pkg"),
            "deps in ignoreDependencies should not be flagged"
        );
    }

    #[test]
    fn plugin_referenced_deps_not_flagged() {
        let (graph, _) = build_graph_with_npm_imports(&[]);
        let pkg = make_pkg(&["tailwindcss"], &[], &[]);
        let config = test_config(PathBuf::from("/project"));

        let mut plugin_result = AggregatedPluginResult::default();
        plugin_result
            .referenced_dependencies
            .push("tailwindcss".to_string());

        let (unused, _, _) =
            find_unused_dependencies(&graph, &pkg, &config, Some(&plugin_result), &[]);

        assert!(
            !unused.iter().any(|d| d.package_name == "tailwindcss"),
            "plugin-referenced deps should not be flagged"
        );
    }

    #[test]
    fn plugin_tooling_deps_not_flagged() {
        let (graph, _) = build_graph_with_npm_imports(&[]);
        let pkg = make_pkg(&["my-framework-runtime"], &[], &[]);
        let config = test_config(PathBuf::from("/project"));

        let mut plugin_result = AggregatedPluginResult::default();
        plugin_result
            .tooling_dependencies
            .push("my-framework-runtime".to_string());

        let (unused, _, _) =
            find_unused_dependencies(&graph, &pkg, &config, Some(&plugin_result), &[]);

        assert!(
            !unused
                .iter()
                .any(|d| d.package_name == "my-framework-runtime"),
            "plugin tooling deps should not be flagged"
        );
    }

    #[test]
    fn script_used_packages_not_flagged() {
        let (graph, _) = build_graph_with_npm_imports(&[]);
        let pkg = make_pkg(&["concurrently"], &[], &[]);
        let config = test_config(PathBuf::from("/project"));

        let mut plugin_result = AggregatedPluginResult::default();
        plugin_result
            .script_used_packages
            .insert("concurrently".to_string());

        let (unused, _, _) =
            find_unused_dependencies(&graph, &pkg, &config, Some(&plugin_result), &[]);

        assert!(
            !unused.iter().any(|d| d.package_name == "concurrently"),
            "packages used in scripts should not be flagged"
        );
    }

    #[test]
    fn unused_dep_location_is_correct() {
        let (graph, _) = build_graph_with_npm_imports(&[]);
        let pkg = make_pkg(&["unused-dep"], &["unused-dev"], &["unused-opt"]);
        let config = test_config(PathBuf::from("/project"));

        let (unused, unused_dev, unused_optional) =
            find_unused_dependencies(&graph, &pkg, &config, None, &[]);

        assert!(unused.iter().any(|d| d.package_name == "unused-dep"
            && matches!(d.location, DependencyLocation::Dependencies)));
        assert!(unused_dev.iter().any(|d| d.package_name == "unused-dev"
            && matches!(d.location, DependencyLocation::DevDependencies)));
        assert!(
            unused_optional
                .iter()
                .any(|d| d.package_name == "unused-opt"
                    && matches!(d.location, DependencyLocation::OptionalDependencies))
        );
    }

    // ---- find_type_only_dependencies tests ----

    #[test]
    fn type_only_dep_detected_when_all_imports_are_type_only() {
        let (graph, _) = build_graph_with_npm_imports(&[("zod", true)]);
        let pkg = make_pkg(&["zod"], &[], &[]);
        let config = test_config(PathBuf::from("/project"));

        let type_only = find_type_only_dependencies(&graph, &pkg, &config, &[]);

        assert!(
            type_only.iter().any(|d| d.package_name == "zod"),
            "dep used only via `import type` should be flagged as type-only"
        );
    }

    #[test]
    fn type_only_dep_not_detected_when_runtime_import_exists() {
        // One runtime import + one type-only import => not type-only
        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: PathBuf::from("/project/src/index.ts"),
                size_bytes: 100,
            },
            DiscoveredFile {
                id: FileId(1),
                path: PathBuf::from("/project/src/other.ts"),
                size_bytes: 100,
            },
        ];

        let entry_points = vec![
            EntryPoint {
                path: PathBuf::from("/project/src/index.ts"),
                source: EntryPointSource::PackageJsonMain,
            },
            EntryPoint {
                path: PathBuf::from("/project/src/other.ts"),
                source: EntryPointSource::PackageJsonMain,
            },
        ];

        let resolved_modules = vec![
            ResolvedModule {
                file_id: FileId(0),
                path: PathBuf::from("/project/src/index.ts"),
                exports: vec![],
                re_exports: vec![],
                resolved_imports: vec![ResolvedImport {
                    info: ImportInfo {
                        source: "zod".to_string(),
                        imported_name: ImportedName::Named("z".to_string()),
                        local_name: "z".to_string(),
                        is_type_only: true,
                        span: oxc_span::Span::new(0, 20),
                    },
                    target: ResolveResult::NpmPackage("zod".to_string()),
                }],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
                unused_import_bindings: vec![],
            },
            ResolvedModule {
                file_id: FileId(1),
                path: PathBuf::from("/project/src/other.ts"),
                exports: vec![],
                re_exports: vec![],
                resolved_imports: vec![ResolvedImport {
                    info: ImportInfo {
                        source: "zod".to_string(),
                        imported_name: ImportedName::Named("z".to_string()),
                        local_name: "z".to_string(),
                        is_type_only: false, // runtime import
                        span: oxc_span::Span::new(0, 20),
                    },
                    target: ResolveResult::NpmPackage("zod".to_string()),
                }],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
                unused_import_bindings: vec![],
            },
        ];

        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);
        let pkg = make_pkg(&["zod"], &[], &[]);
        let config = test_config(PathBuf::from("/project"));

        let type_only = find_type_only_dependencies(&graph, &pkg, &config, &[]);

        assert!(
            type_only.is_empty(),
            "dep with mixed type-only and runtime imports should NOT be flagged"
        );
    }

    #[test]
    fn type_only_dep_not_detected_when_unused() {
        // Dep is not imported at all => caught by unused_dependencies, not type_only
        let (graph, _) = build_graph_with_npm_imports(&[]);
        let pkg = make_pkg(&["zod"], &[], &[]);
        let config = test_config(PathBuf::from("/project"));

        let type_only = find_type_only_dependencies(&graph, &pkg, &config, &[]);

        assert!(
            type_only.is_empty(),
            "completely unused deps should not appear in type_only results"
        );
    }

    #[test]
    fn type_only_dep_skips_workspace_packages() {
        let (graph, _) = build_graph_with_npm_imports(&[("@myorg/types", true)]);
        let pkg = make_pkg(&["@myorg/types"], &[], &[]);
        let config = test_config(PathBuf::from("/project"));

        let workspaces = vec![WorkspaceInfo {
            root: PathBuf::from("/project/packages/types"),
            name: "@myorg/types".to_string(),
            is_internal_dependency: false,
        }];

        let type_only = find_type_only_dependencies(&graph, &pkg, &config, &workspaces);

        assert!(
            type_only.is_empty(),
            "workspace packages should not be flagged as type-only deps"
        );
    }

    #[test]
    fn type_only_dep_skips_ignored_deps() {
        let (graph, _) = build_graph_with_npm_imports(&[("zod", true)]);
        let pkg = make_pkg(&["zod"], &[], &[]);

        let config = FallowConfig {
            schema: None,
            extends: vec![],
            entry: vec![],
            ignore_patterns: vec![],
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec!["zod".to_string()],
            ignore_exports: vec![],
            duplicates: fallow_config::DuplicatesConfig::default(),
            health: fallow_config::HealthConfig::default(),
            rules: fallow_config::RulesConfig::default(),
            production: false,
            plugins: vec![],
            overrides: vec![],
        }
        .resolve(
            PathBuf::from("/project"),
            OutputFormat::Human,
            1,
            true,
            true,
        );

        let type_only = find_type_only_dependencies(&graph, &pkg, &config, &[]);

        assert!(
            type_only.is_empty(),
            "ignored deps should not be flagged as type-only"
        );
    }

    // ---- find_unlisted_dependencies tests ----

    #[test]
    fn unlisted_dep_detected_when_not_in_package_json() {
        let (graph, resolved_modules) = build_graph_with_npm_imports(&[("axios", false)]);
        let pkg = make_pkg(&["react"], &[], &[]); // axios is NOT listed
        let config = test_config(PathBuf::from("/project"));
        let line_offsets: super::LineOffsetsMap<'_> = FxHashMap::default();

        let unlisted = find_unlisted_dependencies(
            &graph,
            &pkg,
            &config,
            &[],
            None,
            &resolved_modules,
            &line_offsets,
        );

        assert!(
            unlisted.iter().any(|d| d.package_name == "axios"),
            "axios is imported but not listed, should be unlisted"
        );
    }

    #[test]
    fn listed_dep_not_reported_as_unlisted() {
        let (graph, resolved_modules) = build_graph_with_npm_imports(&[("react", false)]);
        let pkg = make_pkg(&["react"], &[], &[]);
        let config = test_config(PathBuf::from("/project"));
        let line_offsets: super::LineOffsetsMap<'_> = FxHashMap::default();

        let unlisted = find_unlisted_dependencies(
            &graph,
            &pkg,
            &config,
            &[],
            None,
            &resolved_modules,
            &line_offsets,
        );

        assert!(
            unlisted.is_empty(),
            "dep listed in dependencies should not be flagged as unlisted"
        );
    }

    #[test]
    fn dev_dep_not_reported_as_unlisted() {
        let (graph, resolved_modules) = build_graph_with_npm_imports(&[("jest", false)]);
        let pkg = make_pkg(&[], &["jest"], &[]);
        let config = test_config(PathBuf::from("/project"));
        let line_offsets: super::LineOffsetsMap<'_> = FxHashMap::default();

        let unlisted = find_unlisted_dependencies(
            &graph,
            &pkg,
            &config,
            &[],
            None,
            &resolved_modules,
            &line_offsets,
        );

        assert!(
            unlisted.is_empty(),
            "dep listed in devDependencies should not be unlisted"
        );
    }

    #[test]
    fn builtin_modules_not_reported_as_unlisted() {
        // Import "fs" (a Node.js builtin) - should never be unlisted
        let files = vec![DiscoveredFile {
            id: FileId(0),
            path: PathBuf::from("/project/src/index.ts"),
            size_bytes: 100,
        }];
        let entry_points = vec![EntryPoint {
            path: PathBuf::from("/project/src/index.ts"),
            source: EntryPointSource::PackageJsonMain,
        }];
        // NpmPackage("fs") would be the resolve result if it were npm.
        // But in practice, builtins are tracked as NpmPackage in package_usage.
        // The key filter is is_builtin_module in find_unlisted_dependencies.
        let resolved_modules = vec![ResolvedModule {
            file_id: FileId(0),
            path: PathBuf::from("/project/src/index.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![ResolvedImport {
                info: ImportInfo {
                    source: "node:fs".to_string(),
                    imported_name: ImportedName::Named("readFile".to_string()),
                    local_name: "readFile".to_string(),
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 25),
                },
                target: ResolveResult::NpmPackage("node:fs".to_string()),
            }],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        }];
        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);
        let pkg = make_pkg(&[], &[], &[]);
        let config = test_config(PathBuf::from("/project"));
        let line_offsets: super::LineOffsetsMap<'_> = FxHashMap::default();

        let unlisted = find_unlisted_dependencies(
            &graph,
            &pkg,
            &config,
            &[],
            None,
            &resolved_modules,
            &line_offsets,
        );

        assert!(
            !unlisted.iter().any(|d| d.package_name == "node:fs"),
            "node:fs builtin should not be flagged as unlisted"
        );
    }

    #[test]
    fn virtual_modules_not_reported_as_unlisted() {
        let files = vec![DiscoveredFile {
            id: FileId(0),
            path: PathBuf::from("/project/src/index.ts"),
            size_bytes: 100,
        }];
        let entry_points = vec![EntryPoint {
            path: PathBuf::from("/project/src/index.ts"),
            source: EntryPointSource::PackageJsonMain,
        }];
        let resolved_modules = vec![ResolvedModule {
            file_id: FileId(0),
            path: PathBuf::from("/project/src/index.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![ResolvedImport {
                info: ImportInfo {
                    source: "virtual:pwa-register".to_string(),
                    imported_name: ImportedName::Named("register".to_string()),
                    local_name: "register".to_string(),
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 30),
                },
                target: ResolveResult::NpmPackage("virtual:pwa-register".to_string()),
            }],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        }];
        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);
        let pkg = make_pkg(&[], &[], &[]);
        let config = test_config(PathBuf::from("/project"));
        let line_offsets: super::LineOffsetsMap<'_> = FxHashMap::default();

        let unlisted = find_unlisted_dependencies(
            &graph,
            &pkg,
            &config,
            &[],
            None,
            &resolved_modules,
            &line_offsets,
        );

        assert!(
            unlisted.is_empty(),
            "virtual: modules should not be flagged as unlisted"
        );
    }

    #[test]
    fn workspace_package_names_not_reported_as_unlisted() {
        let (graph, resolved_modules) = build_graph_with_npm_imports(&[("@myorg/utils", false)]);
        let pkg = make_pkg(&[], &[], &[]); // @myorg/utils NOT listed
        let config = test_config(PathBuf::from("/project"));
        let line_offsets: super::LineOffsetsMap<'_> = FxHashMap::default();

        let workspaces = vec![WorkspaceInfo {
            root: PathBuf::from("/project/packages/utils"),
            name: "@myorg/utils".to_string(),
            is_internal_dependency: false,
        }];

        let unlisted = find_unlisted_dependencies(
            &graph,
            &pkg,
            &config,
            &workspaces,
            None,
            &resolved_modules,
            &line_offsets,
        );

        assert!(
            !unlisted.iter().any(|d| d.package_name == "@myorg/utils"),
            "workspace package names should not be flagged as unlisted"
        );
    }

    #[test]
    fn plugin_virtual_prefixes_not_reported_as_unlisted() {
        let pkg = make_pkg(&[], &[], &[]);
        let config = test_config(PathBuf::from("/project"));
        let line_offsets: super::LineOffsetsMap<'_> = FxHashMap::default();

        // Use a non-path-alias virtual prefix (not "#" which is_path_alias catches)
        let (graph2, resolved_modules2) = build_graph_with_npm_imports(&[("@theme/Layout", false)]);

        let mut plugin_result2 = AggregatedPluginResult::default();
        plugin_result2
            .virtual_module_prefixes
            .push("@theme/".to_string());

        let unlisted = find_unlisted_dependencies(
            &graph2,
            &pkg,
            &config,
            &[],
            Some(&plugin_result2),
            &resolved_modules2,
            &line_offsets,
        );

        assert!(
            !unlisted.iter().any(|d| d.package_name == "@theme/Layout"),
            "imports matching virtual module prefixes should not be unlisted"
        );
    }

    #[test]
    fn plugin_tooling_deps_not_reported_as_unlisted() {
        let (graph, resolved_modules) = build_graph_with_npm_imports(&[("h3", false)]);
        let pkg = make_pkg(&[], &[], &[]);
        let config = test_config(PathBuf::from("/project"));
        let line_offsets: super::LineOffsetsMap<'_> = FxHashMap::default();

        let mut plugin_result = AggregatedPluginResult::default();
        plugin_result.tooling_dependencies.push("h3".to_string());

        let unlisted = find_unlisted_dependencies(
            &graph,
            &pkg,
            &config,
            &[],
            Some(&plugin_result),
            &resolved_modules,
            &line_offsets,
        );

        assert!(
            !unlisted.iter().any(|d| d.package_name == "h3"),
            "plugin tooling deps should not be flagged as unlisted"
        );
    }

    #[test]
    fn peer_dep_not_reported_as_unlisted() {
        let (graph, resolved_modules) = build_graph_with_npm_imports(&[("react", false)]);
        // react is listed as a peer dep only, not in deps/devDeps
        let pkg: PackageJson =
            serde_json::from_str(r#"{"peerDependencies": {"react": "^18.0.0"}}"#)
                .expect("test pkg json");

        let config = test_config(PathBuf::from("/project"));
        let line_offsets: super::LineOffsetsMap<'_> = FxHashMap::default();

        let unlisted = find_unlisted_dependencies(
            &graph,
            &pkg,
            &config,
            &[],
            None,
            &resolved_modules,
            &line_offsets,
        );

        assert!(
            unlisted.is_empty(),
            "peer dependencies should not be flagged as unlisted"
        );
    }

    // ---- find_unresolved_imports tests ----

    #[test]
    fn unresolved_import_detected() {
        let resolved_modules = vec![ResolvedModule {
            file_id: FileId(0),
            path: PathBuf::from("/project/src/index.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![ResolvedImport {
                info: ImportInfo {
                    source: "./missing-file".to_string(),
                    imported_name: ImportedName::Named("foo".to_string()),
                    local_name: "foo".to_string(),
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 30),
                },
                target: ResolveResult::Unresolvable("./missing-file".to_string()),
            }],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        }];

        let config = test_config(PathBuf::from("/project"));
        let suppressions: FxHashMap<FileId, &[Suppression]> = FxHashMap::default();
        let line_offsets: super::LineOffsetsMap<'_> = FxHashMap::default();

        let unresolved = find_unresolved_imports(
            &resolved_modules,
            &config,
            &suppressions,
            &[],
            &line_offsets,
        );

        assert_eq!(unresolved.len(), 1);
        assert_eq!(unresolved[0].specifier, "./missing-file");
    }

    #[test]
    fn unresolved_virtual_module_not_reported() {
        let resolved_modules = vec![ResolvedModule {
            file_id: FileId(0),
            path: PathBuf::from("/project/src/index.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![ResolvedImport {
                info: ImportInfo {
                    source: "virtual:generated-pages".to_string(),
                    imported_name: ImportedName::Named("pages".to_string()),
                    local_name: "pages".to_string(),
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 40),
                },
                target: ResolveResult::Unresolvable("virtual:generated-pages".to_string()),
            }],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        }];

        let config = test_config(PathBuf::from("/project"));
        let suppressions: FxHashMap<FileId, &[Suppression]> = FxHashMap::default();
        let line_offsets: super::LineOffsetsMap<'_> = FxHashMap::default();

        let unresolved = find_unresolved_imports(
            &resolved_modules,
            &config,
            &suppressions,
            &[],
            &line_offsets,
        );

        assert!(
            unresolved.is_empty(),
            "virtual: module imports should not be flagged as unresolved"
        );
    }

    #[test]
    fn unresolved_import_with_virtual_prefix_not_reported() {
        let resolved_modules = vec![ResolvedModule {
            file_id: FileId(0),
            path: PathBuf::from("/project/src/index.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![ResolvedImport {
                info: ImportInfo {
                    source: "#imports".to_string(),
                    imported_name: ImportedName::Named("useRouter".to_string()),
                    local_name: "useRouter".to_string(),
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 25),
                },
                target: ResolveResult::Unresolvable("#imports".to_string()),
            }],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        }];

        let config = test_config(PathBuf::from("/project"));
        let suppressions: FxHashMap<FileId, &[Suppression]> = FxHashMap::default();
        let line_offsets: super::LineOffsetsMap<'_> = FxHashMap::default();

        let unresolved = find_unresolved_imports(
            &resolved_modules,
            &config,
            &suppressions,
            &["#"], // Nuxt-style virtual prefix
            &line_offsets,
        );

        assert!(
            unresolved.is_empty(),
            "imports matching virtual_prefixes should not be flagged as unresolved"
        );
    }

    #[test]
    fn unresolved_import_suppressed_by_inline_comment() {
        use crate::suppress::Suppression;

        let resolved_modules = vec![ResolvedModule {
            file_id: FileId(0),
            path: PathBuf::from("/project/src/index.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![ResolvedImport {
                info: ImportInfo {
                    source: "./broken".to_string(),
                    imported_name: ImportedName::Named("thing".to_string()),
                    local_name: "thing".to_string(),
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 20),
                },
                target: ResolveResult::Unresolvable("./broken".to_string()),
            }],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        }];

        let config = test_config(PathBuf::from("/project"));
        // Suppress unresolved imports on line 1 (byte offset 0 => line 1 without offsets)
        let supps = vec![Suppression {
            line: 1,
            kind: Some(suppress::IssueKind::UnresolvedImport),
        }];
        let mut suppressions: FxHashMap<FileId, &[Suppression]> = FxHashMap::default();
        suppressions.insert(FileId(0), &supps);
        let line_offsets: super::LineOffsetsMap<'_> = FxHashMap::default();

        let unresolved = find_unresolved_imports(
            &resolved_modules,
            &config,
            &suppressions,
            &[],
            &line_offsets,
        );

        assert!(
            unresolved.is_empty(),
            "suppressed unresolved import should not be reported"
        );
    }

    #[test]
    fn unresolved_import_file_level_suppression() {
        use crate::suppress::Suppression;

        let resolved_modules = vec![ResolvedModule {
            file_id: FileId(0),
            path: PathBuf::from("/project/src/index.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![ResolvedImport {
                info: ImportInfo {
                    source: "./nonexistent".to_string(),
                    imported_name: ImportedName::Named("x".to_string()),
                    local_name: "x".to_string(),
                    is_type_only: false,
                    span: oxc_span::Span::new(0, 25),
                },
                target: ResolveResult::Unresolvable("./nonexistent".to_string()),
            }],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        }];

        let config = test_config(PathBuf::from("/project"));
        // File-level suppression (line 0)
        let supps = vec![Suppression {
            line: 0,
            kind: Some(suppress::IssueKind::UnresolvedImport),
        }];
        let mut suppressions: FxHashMap<FileId, &[Suppression]> = FxHashMap::default();
        suppressions.insert(FileId(0), &supps);
        let line_offsets: super::LineOffsetsMap<'_> = FxHashMap::default();

        let unresolved = find_unresolved_imports(
            &resolved_modules,
            &config,
            &suppressions,
            &[],
            &line_offsets,
        );

        assert!(
            unresolved.is_empty(),
            "file-level suppression should suppress all unresolved imports in the file"
        );
    }

    #[test]
    fn resolved_import_not_reported_as_unresolved() {
        let resolved_modules = vec![ResolvedModule {
            file_id: FileId(0),
            path: PathBuf::from("/project/src/index.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![
                ResolvedImport {
                    info: ImportInfo {
                        source: "react".to_string(),
                        imported_name: ImportedName::Named("useState".to_string()),
                        local_name: "useState".to_string(),
                        is_type_only: false,
                        span: oxc_span::Span::new(0, 20),
                    },
                    target: ResolveResult::NpmPackage("react".to_string()),
                },
                ResolvedImport {
                    info: ImportInfo {
                        source: "./utils".to_string(),
                        imported_name: ImportedName::Named("helper".to_string()),
                        local_name: "helper".to_string(),
                        is_type_only: false,
                        span: oxc_span::Span::new(25, 50),
                    },
                    target: ResolveResult::InternalModule(FileId(1)),
                },
            ],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        }];

        let config = test_config(PathBuf::from("/project"));
        let suppressions: FxHashMap<FileId, &[Suppression]> = FxHashMap::default();
        let line_offsets: super::LineOffsetsMap<'_> = FxHashMap::default();

        let unresolved = find_unresolved_imports(
            &resolved_modules,
            &config,
            &suppressions,
            &[],
            &line_offsets,
        );

        assert!(
            unresolved.is_empty(),
            "resolved imports should never appear as unresolved"
        );
    }

    // ---- Scoped package / subpath import edge cases ----

    #[test]
    fn scoped_package_subpath_import_recognized_as_used() {
        // import { Button } from '@chakra-ui/react/button'
        // should recognize '@chakra-ui/react' as the package name
        let (graph, _resolved_modules) =
            build_graph_with_npm_imports(&[("@chakra-ui/react", false)]);
        let pkg = make_pkg(&["@chakra-ui/react"], &[], &[]);
        let config = test_config(PathBuf::from("/project"));

        let (unused, _, _) = find_unused_dependencies(&graph, &pkg, &config, None, &[]);

        assert!(
            unused.is_empty(),
            "@chakra-ui/react should be recognized as used via subpath import"
        );
    }

    #[test]
    fn optional_dep_in_peer_deps_also_counts() {
        // An optional dep that is also used should not be flagged
        let (graph, _) = build_graph_with_npm_imports(&[("sharp", false)]);
        let pkg = make_pkg(&[], &[], &["sharp"]);
        let config = test_config(PathBuf::from("/project"));

        let (_, _, unused_optional) = find_unused_dependencies(&graph, &pkg, &config, None, &[]);

        assert!(
            unused_optional.is_empty(),
            "optional dep that is imported should not be flagged as unused"
        );
    }

    // ---- Empty / edge case scenarios ----

    #[test]
    fn no_deps_produces_no_unused() {
        let (graph, _) = build_graph_with_npm_imports(&[]);
        let pkg = make_pkg(&[], &[], &[]);
        let config = test_config(PathBuf::from("/project"));

        let (unused, unused_dev, unused_optional) =
            find_unused_dependencies(&graph, &pkg, &config, None, &[]);

        assert!(unused.is_empty());
        assert!(unused_dev.is_empty());
        assert!(unused_optional.is_empty());
    }

    #[test]
    fn no_imports_flags_all_non_implicit_deps() {
        let (graph, _) = build_graph_with_npm_imports(&[]);
        let pkg = make_pkg(&["lodash", "axios"], &[], &[]);
        let config = test_config(PathBuf::from("/project"));

        let (unused, _, _) = find_unused_dependencies(&graph, &pkg, &config, None, &[]);

        assert!(unused.iter().any(|d| d.package_name == "lodash"));
        assert!(unused.iter().any(|d| d.package_name == "axios"));
    }

    #[test]
    fn unlisted_dep_has_import_sites() {
        let (graph, resolved_modules) = build_graph_with_npm_imports(&[("unlisted-pkg", false)]);
        let pkg = make_pkg(&[], &[], &[]);
        let config = test_config(PathBuf::from("/project"));
        let line_offsets: super::LineOffsetsMap<'_> = FxHashMap::default();

        let unlisted = find_unlisted_dependencies(
            &graph,
            &pkg,
            &config,
            &[],
            None,
            &resolved_modules,
            &line_offsets,
        );

        assert_eq!(unlisted.len(), 1);
        assert_eq!(unlisted[0].package_name, "unlisted-pkg");
        assert!(
            !unlisted[0].imported_from.is_empty(),
            "unlisted dep should have at least one import site"
        );
        assert_eq!(
            unlisted[0].imported_from[0].path,
            PathBuf::from("/project/src/index.ts")
        );
    }

    #[test]
    fn path_alias_imports_not_reported_as_unlisted() {
        // @/components and ~/utils are path aliases, not npm packages
        let files = vec![DiscoveredFile {
            id: FileId(0),
            path: PathBuf::from("/project/src/index.ts"),
            size_bytes: 100,
        }];
        let entry_points = vec![EntryPoint {
            path: PathBuf::from("/project/src/index.ts"),
            source: EntryPointSource::PackageJsonMain,
        }];
        let resolved_modules = vec![ResolvedModule {
            file_id: FileId(0),
            path: PathBuf::from("/project/src/index.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![
                ResolvedImport {
                    info: ImportInfo {
                        source: "@/components/Button".to_string(),
                        imported_name: ImportedName::Named("Button".to_string()),
                        local_name: "Button".to_string(),
                        is_type_only: false,
                        span: oxc_span::Span::new(0, 30),
                    },
                    target: ResolveResult::NpmPackage("@/components/Button".to_string()),
                },
                ResolvedImport {
                    info: ImportInfo {
                        source: "~/utils/helper".to_string(),
                        imported_name: ImportedName::Named("helper".to_string()),
                        local_name: "helper".to_string(),
                        is_type_only: false,
                        span: oxc_span::Span::new(35, 60),
                    },
                    target: ResolveResult::NpmPackage("~/utils/helper".to_string()),
                },
            ],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        }];
        let graph = ModuleGraph::build(&resolved_modules, &entry_points, &files);
        let pkg = make_pkg(&[], &[], &[]);
        let config = test_config(PathBuf::from("/project"));
        let line_offsets: super::LineOffsetsMap<'_> = FxHashMap::default();

        let unlisted = find_unlisted_dependencies(
            &graph,
            &pkg,
            &config,
            &[],
            None,
            &resolved_modules,
            &line_offsets,
        );

        assert!(
            unlisted.is_empty(),
            "path aliases should never be flagged as unlisted dependencies"
        );
    }

    #[test]
    fn multiple_unresolved_imports_collected() {
        let resolved_modules = vec![ResolvedModule {
            file_id: FileId(0),
            path: PathBuf::from("/project/src/index.ts"),
            exports: vec![],
            re_exports: vec![],
            resolved_imports: vec![
                ResolvedImport {
                    info: ImportInfo {
                        source: "./missing-a".to_string(),
                        imported_name: ImportedName::Named("a".to_string()),
                        local_name: "a".to_string(),
                        is_type_only: false,
                        span: oxc_span::Span::new(0, 20),
                    },
                    target: ResolveResult::Unresolvable("./missing-a".to_string()),
                },
                ResolvedImport {
                    info: ImportInfo {
                        source: "./missing-b".to_string(),
                        imported_name: ImportedName::Named("b".to_string()),
                        local_name: "b".to_string(),
                        is_type_only: false,
                        span: oxc_span::Span::new(25, 45),
                    },
                    target: ResolveResult::Unresolvable("./missing-b".to_string()),
                },
            ],
            resolved_dynamic_imports: vec![],
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        }];

        let config = test_config(PathBuf::from("/project"));
        let suppressions: FxHashMap<FileId, &[Suppression]> = FxHashMap::default();
        let line_offsets: super::LineOffsetsMap<'_> = FxHashMap::default();

        let unresolved = find_unresolved_imports(
            &resolved_modules,
            &config,
            &suppressions,
            &[],
            &line_offsets,
        );

        assert_eq!(unresolved.len(), 2);
        assert!(unresolved.iter().any(|u| u.specifier == "./missing-a"));
        assert!(unresolved.iter().any(|u| u.specifier == "./missing-b"));
    }
}
