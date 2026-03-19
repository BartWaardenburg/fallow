use std::collections::{HashMap, HashSet};

use fallow_config::{PackageJson, ResolvedConfig};

use crate::discover::FileId;
use crate::graph::ModuleGraph;
use crate::resolve::ResolvedModule;
use crate::results::*;
use crate::suppress::{self, IssueKind, Suppression};

use super::predicates::{is_builtin_module, is_implicit_dependency, is_path_alias};
use super::{byte_offset_to_line_col, read_source};

/// Find dependencies in package.json that are never imported.
///
/// Checks both the root package.json and each workspace's package.json.
/// For workspace deps, only files within that workspace are considered when
/// determining whether a dependency is used (mirroring `find_unlisted_dependencies`).
pub(crate) fn find_unused_dependencies(
    graph: &ModuleGraph,
    pkg: &PackageJson,
    config: &ResolvedConfig,
    plugin_result: Option<&crate::plugins::AggregatedPluginResult>,
    workspaces: &[fallow_config::WorkspaceInfo],
) -> (Vec<UnusedDependency>, Vec<UnusedDependency>) {
    // Collect deps referenced in config files (discovered by plugins)
    let plugin_referenced: HashSet<&str> = plugin_result
        .map(|pr| {
            pr.referenced_dependencies
                .iter()
                .map(|s| s.as_str())
                .collect()
        })
        .unwrap_or_default();

    // Collect tooling deps from plugins
    let plugin_tooling: HashSet<&str> = plugin_result
        .map(|pr| pr.tooling_dependencies.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();

    // Collect packages used as binaries in package.json scripts
    let script_used: HashSet<&str> = plugin_result
        .map(|pr| pr.script_used_packages.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();

    // Collect workspace package names — these are internal deps, not npm packages
    let workspace_names: HashSet<&str> = workspaces.iter().map(|ws| ws.name.as_str()).collect();

    // Pre-compute ignore deps as HashSet for O(1) lookups instead of O(n) linear scan
    let ignore_deps: HashSet<&str> = config
        .ignore_dependencies
        .iter()
        .map(|s| s.as_str())
        .collect();

    // Build per-package set of files that use it (globally)
    let used_packages: HashSet<&str> = graph.package_usage.keys().map(|s| s.as_str()).collect();

    let root_pkg_path = config.root.join("package.json");

    // --- Root package.json check (existing behavior: any file can satisfy usage) ---
    let mut unused_deps: Vec<UnusedDependency> = pkg
        .production_dependency_names()
        .into_iter()
        .filter(|dep| !used_packages.contains(dep.as_str()))
        .filter(|dep| !script_used.contains(dep.as_str()))
        .filter(|dep| !is_implicit_dependency(dep))
        .filter(|dep| !plugin_referenced.contains(dep.as_str()))
        .filter(|dep| !ignore_deps.contains(dep.as_str()))
        .filter(|dep| !workspace_names.contains(dep.as_str()))
        .map(|dep| UnusedDependency {
            package_name: dep,
            location: DependencyLocation::Dependencies,
            path: root_pkg_path.clone(),
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
        .map(|dep| UnusedDependency {
            package_name: dep,
            location: DependencyLocation::DevDependencies,
            path: root_pkg_path.clone(),
        })
        .collect();

    // --- Workspace package.json checks: scope usage to files within each workspace ---
    // Track which deps are already flagged from root to avoid double-reporting
    let root_flagged: HashSet<String> = unused_deps
        .iter()
        .chain(unused_dev_deps.iter())
        .map(|d| d.package_name.clone())
        .collect();

    for ws in workspaces {
        let ws_pkg_path = ws.root.join("package.json");
        let ws_pkg = match PackageJson::load(&ws_pkg_path) {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Helper: check if a dependency is used by any file within this workspace.
        // Uses raw path comparison (module paths are absolute, workspace root is absolute)
        // to avoid per-file canonicalize() syscalls.
        let ws_root = &ws.root;
        let is_used_in_workspace = |dep: &str| -> bool {
            if let Some(file_ids) = graph.package_usage.get(dep) {
                file_ids.iter().any(|id| {
                    graph
                        .modules
                        .get(id.0 as usize)
                        .is_some_and(|module| module.path.starts_with(ws_root))
                })
            } else {
                false
            }
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
            {
                continue;
            }
            unused_deps.push(UnusedDependency {
                package_name: dep,
                location: DependencyLocation::Dependencies,
                path: ws_pkg_path.clone(),
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
            unused_dev_deps.push(UnusedDependency {
                package_name: dep,
                location: DependencyLocation::DevDependencies,
                path: ws_pkg_path.clone(),
            });
        }
    }

    (unused_deps, unused_dev_deps)
}

/// Check if a dependency should be skipped during unused dependency analysis.
///
/// Shared guard conditions for both production and dev dependency loops:
/// already flagged from root, used in scripts, referenced by plugins, in ignore list,
/// is a workspace package, or used by files in the workspace.
fn should_skip_dependency(
    dep: &str,
    root_flagged: &HashSet<String>,
    script_used: &HashSet<&str>,
    plugin_referenced: &HashSet<&str>,
    ignore_deps: &HashSet<&str>,
    workspace_names: &HashSet<&str>,
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
pub(crate) fn find_type_only_dependencies(
    graph: &ModuleGraph,
    pkg: &PackageJson,
    config: &ResolvedConfig,
    workspaces: &[fallow_config::WorkspaceInfo],
) -> Vec<TypeOnlyDependency> {
    let root_pkg_path = config.root.join("package.json");
    let workspace_names: HashSet<&str> = workspaces.iter().map(|ws| ws.name.as_str()).collect();

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
        let total_count = graph
            .package_usage
            .get(dep.as_str())
            .map(|v| v.len())
            .unwrap_or(0);
        let type_only_count = graph
            .type_only_package_usage
            .get(dep.as_str())
            .map(|v| v.len())
            .unwrap_or(0);

        if has_type_only_usage && type_only_count == total_count {
            type_only_deps.push(TypeOnlyDependency {
                package_name: dep,
                path: root_pkg_path.clone(),
            });
        }
    }

    type_only_deps
}

/// Find dependencies used in imports but not listed in package.json.
pub(crate) fn find_unlisted_dependencies(
    graph: &ModuleGraph,
    pkg: &PackageJson,
    config: &ResolvedConfig,
    workspaces: &[fallow_config::WorkspaceInfo],
    plugin_result: Option<&crate::plugins::AggregatedPluginResult>,
) -> Vec<UnlistedDependency> {
    let all_deps: HashSet<String> = pkg.all_dependency_names().into_iter().collect();

    // Build a set of all deps across all workspace package.json files.
    // In monorepos, imports in workspace files reference deps from that workspace's package.json.
    let mut all_workspace_deps: HashSet<String> = all_deps.clone();
    // Also collect workspace package names — internal workspace deps should not be flagged
    let mut workspace_names: HashSet<String> = HashSet::new();
    // Map: canonical workspace root -> set of dep names (for per-file checks)
    let mut ws_dep_map: Vec<(std::path::PathBuf, HashSet<String>)> = Vec::new();

    for ws in workspaces {
        workspace_names.insert(ws.name.clone());
        let ws_pkg_path = ws.root.join("package.json");
        if let Ok(ws_pkg) = PackageJson::load(&ws_pkg_path) {
            let ws_deps: HashSet<String> = ws_pkg.all_dependency_names().into_iter().collect();
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
    let plugin_tooling: HashSet<&str> = plugin_result
        .map(|pr| pr.tooling_dependencies.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();

    let mut unlisted: HashMap<String, Vec<std::path::PathBuf>> = HashMap::new();

    for (package_name, file_ids) in &graph.package_usage {
        if is_builtin_module(package_name) || is_path_alias(package_name) {
            continue;
        }
        // Skip virtual module imports (e.g., `virtual:pwa-register`, `virtual:emoji-mart-lang-importer`)
        // created by Vite plugins and similar build tools
        if package_name.starts_with("virtual:") {
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
        let mut unlisted_paths: Vec<std::path::PathBuf> = Vec::new();
        for id in file_ids {
            if let Some(module) = graph.modules.get(id.0 as usize) {
                let listed_in_ws = ws_dep_map.iter().any(|(ws_root, ws_deps)| {
                    module.path.starts_with(ws_root) && ws_deps.contains(package_name)
                });
                // Also check root deps
                let listed_in_root = all_deps.contains(package_name);
                if !listed_in_ws && !listed_in_root {
                    unlisted_paths.push(module.path.clone());
                }
            }
        }

        if !unlisted_paths.is_empty() {
            unlisted_paths.sort();
            unlisted_paths.dedup();
            unlisted.insert(package_name.clone(), unlisted_paths);
        }
    }

    let _ = config; // future use
    unlisted
        .into_iter()
        .map(|(name, paths)| UnlistedDependency {
            package_name: name,
            imported_from: paths,
        })
        .collect()
}

/// Find imports that could not be resolved.
pub(crate) fn find_unresolved_imports(
    resolved_modules: &[ResolvedModule],
    _config: &ResolvedConfig,
    suppressions_by_file: &HashMap<FileId, &[Suppression]>,
    virtual_prefixes: &[&str],
) -> Vec<UnresolvedImport> {
    let mut unresolved = Vec::new();

    for module in resolved_modules {
        // Lazily load source content for line/col computation
        let mut source_content: Option<String> = None;

        for import in &module.resolved_imports {
            if let crate::resolve::ResolveResult::Unresolvable(spec) = &import.target {
                // Skip virtual module imports provided by active framework plugins
                // (e.g., Nuxt's #imports, #app, #components, #build).
                if virtual_prefixes
                    .iter()
                    .any(|prefix| spec.starts_with(prefix))
                {
                    continue;
                }

                let source = source_content.get_or_insert_with(|| read_source(&module.path));
                let (line, col) = byte_offset_to_line_col(source, import.info.span.start);

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
        HashSet<String>,
        HashSet<&'static str>,
        HashSet<&'static str>,
        HashSet<&'static str>,
        HashSet<&'static str>,
    );

    /// Helper: build empty sets for should_skip_dependency args.
    fn empty_sets() -> SkipDepSets {
        (
            HashSet::new(),
            HashSet::new(),
            HashSet::new(),
            HashSet::new(),
            HashSet::new(),
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
    fn skip_dep_short_circuits_on_first_match() {
        // If root_flagged matches, the closure should not even be needed.
        // We verify this by providing a dep in root_flagged and a closure that panics.
        let (mut root_flagged, script_used, plugin_referenced, ignore_deps, workspace_names) =
            empty_sets();
        root_flagged.insert("lodash".to_string());
        // This should return true without calling the closure
        let result = should_skip_dependency(
            "lodash",
            &root_flagged,
            &script_used,
            &plugin_referenced,
            &ignore_deps,
            &workspace_names,
            |_| panic!("closure should not be called"),
        );
        assert!(result);
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
        assert!(super::super::predicates::is_implicit_dependency("@next/mdx"));
        assert!(super::super::predicates::is_implicit_dependency(
            "@next/bundle-analyzer"
        ));
        assert!(super::super::predicates::is_implicit_dependency("@next/env"));
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
}
