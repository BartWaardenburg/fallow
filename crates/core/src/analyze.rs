use std::collections::HashSet;

use fallow_config::{PackageJson, ResolvedConfig};

use crate::graph::ModuleGraph;
use crate::results::*;

/// Find all dead code in the project.
pub fn find_dead_code(graph: &ModuleGraph, config: &ResolvedConfig) -> AnalysisResults {
    let _span = tracing::info_span!("find_dead_code").entered();

    let mut results = AnalysisResults::default();

    if config.detect.unused_files {
        results.unused_files = find_unused_files(graph);
    }

    if config.detect.unused_exports || config.detect.unused_types {
        let (exports, types) = find_unused_exports(graph, config);
        if config.detect.unused_exports {
            results.unused_exports = exports;
        }
        if config.detect.unused_types {
            results.unused_types = types;
        }
    }

    if config.detect.unused_dependencies || config.detect.unused_dev_dependencies {
        let pkg_path = config.root.join("package.json");
        if let Ok(pkg) = PackageJson::load(&pkg_path) {
            let (deps, dev_deps) = find_unused_dependencies(graph, &pkg, config);
            if config.detect.unused_dependencies {
                results.unused_dependencies = deps;
            }
            if config.detect.unused_dev_dependencies {
                results.unused_dev_dependencies = dev_deps;
            }
        }
    }

    results
}

/// Find files that are not reachable from any entry point.
fn find_unused_files(graph: &ModuleGraph) -> Vec<UnusedFile> {
    graph
        .modules
        .iter()
        .filter(|m| !m.is_reachable && !m.is_entry_point)
        .map(|m| UnusedFile {
            path: m.path.clone(),
        })
        .collect()
}

/// Find exports that are never imported by other files.
fn find_unused_exports(
    graph: &ModuleGraph,
    config: &ResolvedConfig,
) -> (Vec<UnusedExport>, Vec<UnusedExport>) {
    let mut unused_exports = Vec::new();
    let mut unused_types = Vec::new();

    for module in &graph.modules {
        // Skip unreachable modules (already reported as unused files)
        if !module.is_reachable {
            continue;
        }

        // Skip entry points (their exports are consumed externally)
        if module.is_entry_point {
            continue;
        }

        // Skip CJS modules with module.exports (hard to track individual exports)
        if module.has_cjs_exports && module.exports.is_empty() {
            continue;
        }

        // Check if this file has namespace imports (import * as ns)
        // If so, all exports are conservatively considered used
        if graph.has_namespace_import(module.file_id) {
            continue;
        }

        // Check ignore rules
        let relative_path = module
            .path
            .strip_prefix(&config.root)
            .unwrap_or(&module.path);

        for export in &module.exports {
            if export.references.is_empty() {
                // Check if this export is ignored by config
                if is_export_ignored(config, relative_path, &export.name) {
                    continue;
                }

                // Check if this export is considered "used" by a framework rule
                if is_framework_used_export(config, relative_path, &export.name) {
                    continue;
                }

                let unused = UnusedExport {
                    path: module.path.clone(),
                    export_name: export.name.to_string(),
                    is_type_only: export.is_type_only,
                    line: export.span.start,
                    col: 0,
                };

                if export.is_type_only {
                    unused_types.push(unused);
                } else {
                    unused_exports.push(unused);
                }
            }
        }
    }

    (unused_exports, unused_types)
}

/// Find dependencies in package.json that are never imported.
fn find_unused_dependencies(
    graph: &ModuleGraph,
    pkg: &PackageJson,
    config: &ResolvedConfig,
) -> (Vec<UnusedDependency>, Vec<UnusedDependency>) {
    let used_packages: HashSet<&str> = graph
        .package_usage
        .keys()
        .map(|s| s.as_str())
        .collect();

    let unused_deps: Vec<UnusedDependency> = pkg
        .production_dependency_names()
        .into_iter()
        .filter(|dep| !used_packages.contains(dep.as_str()))
        .filter(|dep| !is_implicit_dependency(dep))
        .filter(|dep| !config.ignore_dependencies.iter().any(|d| d == dep))
        .map(|dep| UnusedDependency {
            package_name: dep,
            location: DependencyLocation::Dependencies,
        })
        .collect();

    let unused_dev_deps: Vec<UnusedDependency> = pkg
        .dev_dependency_names()
        .into_iter()
        .filter(|dep| !used_packages.contains(dep.as_str()))
        .filter(|dep| !is_tooling_dependency(dep))
        .filter(|dep| !config.ignore_dependencies.iter().any(|d| d == dep))
        .map(|dep| UnusedDependency {
            package_name: dep,
            location: DependencyLocation::DevDependencies,
        })
        .collect();

    (unused_deps, unused_dev_deps)
}

/// Check if an export should be ignored based on config rules.
fn is_export_ignored(
    config: &ResolvedConfig,
    file_path: &std::path::Path,
    export_name: &crate::extract::ExportName,
) -> bool {
    let file_str = file_path.to_string_lossy();
    let export_str = export_name.to_string();

    for rule in &config.ignore_export_rules {
        let file_matches = globset::Glob::new(&rule.file)
            .ok()
            .map(|g| g.compile_matcher().is_match(file_str.as_ref()))
            .unwrap_or(false);

        if file_matches {
            if rule.exports.iter().any(|e| e == "*" || e == &export_str) {
                return true;
            }
        }
    }
    false
}

/// Check if a framework rule marks this export as always-used.
fn is_framework_used_export(
    config: &ResolvedConfig,
    file_path: &std::path::Path,
    export_name: &crate::extract::ExportName,
) -> bool {
    let file_str = file_path.to_string_lossy();
    let export_str = export_name.to_string();

    for rule in &config.framework_rules {
        for used in &rule.used_exports {
            let file_matches = globset::Glob::new(&used.file_pattern)
                .ok()
                .map(|g| g.compile_matcher().is_match(file_str.as_ref()))
                .unwrap_or(false);

            if file_matches && used.exports.iter().any(|e| e == &export_str) {
                return true;
            }
        }
    }
    false
}

/// Dependencies that are used implicitly (not via imports).
fn is_implicit_dependency(name: &str) -> bool {
    name.starts_with("@types/")
}

/// Dev dependencies that are tooling (used by CLI, not imported in code).
fn is_tooling_dependency(name: &str) -> bool {
    let tooling_prefixes = [
        "@types/",
        "eslint",
        "prettier",
        "typescript",
        "@typescript-eslint",
        "husky",
        "lint-staged",
        "commitlint",
        "@commitlint",
        "stylelint",
        "postcss",
        "autoprefixer",
        "tailwindcss",
        "@tailwindcss",
    ];

    let exact_matches = [
        "typescript",
        "prettier",
        "turbo",
        "concurrently",
        "cross-env",
        "rimraf",
        "npm-run-all",
        "nodemon",
        "ts-node",
        "tsx",
    ];

    tooling_prefixes.iter().any(|p| name.starts_with(p))
        || exact_matches.iter().any(|m| name == *m)
}
