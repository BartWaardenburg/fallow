use std::collections::{HashMap, HashSet};

use fallow_config::{PackageJson, ResolvedConfig};

use crate::extract::MemberKind;
use crate::graph::ModuleGraph;
use crate::resolve::ResolvedModule;
use crate::results::*;

/// Find all dead code in the project.
pub fn find_dead_code(graph: &ModuleGraph, config: &ResolvedConfig) -> AnalysisResults {
    find_dead_code_with_resolved(graph, config, &[])
}

/// Find all dead code, with optional resolved module data for additional analyses.
pub fn find_dead_code_with_resolved(
    graph: &ModuleGraph,
    config: &ResolvedConfig,
    resolved_modules: &[ResolvedModule],
) -> AnalysisResults {
    let _span = tracing::info_span!("find_dead_code").entered();

    let mut results = AnalysisResults::default();

    if config.detect.unused_files {
        let t = std::time::Instant::now();
        results.unused_files = find_unused_files(graph);
        eprintln!("[perf]   unused_files: {:?}", t.elapsed());
    }

    if config.detect.unused_exports || config.detect.unused_types {
        let t = std::time::Instant::now();
        let (exports, types) = find_unused_exports(graph, config);
        eprintln!("[perf]   unused_exports: {:?}", t.elapsed());
        if config.detect.unused_exports {
            results.unused_exports = exports;
        }
        if config.detect.unused_types {
            results.unused_types = types;
        }
    }

    if config.detect.unused_enum_members || config.detect.unused_class_members {
        let t = std::time::Instant::now();
        let (enum_members, class_members) = find_unused_members(graph, config, resolved_modules);
        eprintln!("[perf]   unused_members: {:?}", t.elapsed());
        if config.detect.unused_enum_members {
            results.unused_enum_members = enum_members;
        }
        if config.detect.unused_class_members {
            results.unused_class_members = class_members;
        }
    }

    let pkg_path = config.root.join("package.json");
    if let Ok(pkg) = PackageJson::load(&pkg_path) {
        if config.detect.unused_dependencies || config.detect.unused_dev_dependencies {
            let t = std::time::Instant::now();
            let (deps, dev_deps) = find_unused_dependencies(graph, &pkg, config);
            eprintln!("[perf]   unused_deps: {:?}", t.elapsed());
            if config.detect.unused_dependencies {
                results.unused_dependencies = deps;
            }
            if config.detect.unused_dev_dependencies {
                results.unused_dev_dependencies = dev_deps;
            }
        }

        if config.detect.unlisted_dependencies {
            let t = std::time::Instant::now();
            results.unlisted_dependencies = find_unlisted_dependencies(graph, &pkg);
            eprintln!("[perf]   unlisted_deps: {:?}", t.elapsed());
        }
    }

    if config.detect.unresolved_imports && !resolved_modules.is_empty() {
        let t = std::time::Instant::now();
        results.unresolved_imports = find_unresolved_imports(resolved_modules, config);
        eprintln!("[perf]   unresolved: {:?}", t.elapsed());
    }

    if config.detect.duplicate_exports {
        let t = std::time::Instant::now();
        results.duplicate_exports = find_duplicate_exports(graph, config);
        eprintln!("[perf]   duplicates: {:?}", t.elapsed());
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
    let used_packages: HashSet<&str> = graph.package_usage.keys().map(|s| s.as_str()).collect();

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

        if file_matches && rule.exports.iter().any(|e| e == "*" || e == &export_str) {
            return true;
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

/// Find unused enum and class members in exported symbols.
///
/// Collects all `Identifier.member` static member accesses from all modules,
/// maps them to their imported names, and filters out members that are accessed.
fn find_unused_members(
    graph: &ModuleGraph,
    _config: &ResolvedConfig,
    resolved_modules: &[ResolvedModule],
) -> (Vec<UnusedMember>, Vec<UnusedMember>) {
    let mut unused_enum_members = Vec::new();
    let mut unused_class_members = Vec::new();

    // Build a set of (export_name, member_name) pairs that are accessed across all modules.
    // We map local import names back to the original imported names.
    let mut accessed_members: HashSet<(String, String)> = HashSet::new();

    for resolved in resolved_modules {
        // Build a map from local name -> imported name for this module's imports
        let local_to_imported: HashMap<&str, &str> = resolved
            .resolved_imports
            .iter()
            .filter_map(|imp| match &imp.info.imported_name {
                crate::extract::ImportedName::Named(name) => {
                    Some((imp.info.local_name.as_str(), name.as_str()))
                }
                crate::extract::ImportedName::Default => {
                    Some((imp.info.local_name.as_str(), "default"))
                }
                _ => None,
            })
            .collect();

        for access in &resolved.member_accesses {
            // If the object is a local name for an import, map it to the original export name
            let export_name = local_to_imported
                .get(access.object.as_str())
                .copied()
                .unwrap_or(access.object.as_str());
            accessed_members.insert((export_name.to_string(), access.member.clone()));
        }
    }

    for module in &graph.modules {
        if !module.is_reachable || module.is_entry_point {
            continue;
        }

        for export in &module.exports {
            if export.members.is_empty() {
                continue;
            }

            // If the export itself is unused, skip member analysis (whole export is dead)
            if export.references.is_empty() && !graph.has_namespace_import(module.file_id) {
                continue;
            }

            let export_name = export.name.to_string();

            for member in &export.members {
                // Check if this member is accessed anywhere
                if accessed_members.contains(&(export_name.clone(), member.name.clone())) {
                    continue;
                }

                let unused = UnusedMember {
                    path: module.path.clone(),
                    parent_name: export_name.clone(),
                    member_name: member.name.clone(),
                    kind: match member.kind {
                        MemberKind::EnumMember => "enum_member".to_string(),
                        MemberKind::ClassMethod => "class_method".to_string(),
                        MemberKind::ClassProperty => "class_property".to_string(),
                    },
                    line: member.span.start,
                    col: 0,
                };

                match member.kind {
                    MemberKind::EnumMember => unused_enum_members.push(unused),
                    MemberKind::ClassMethod | MemberKind::ClassProperty => {
                        unused_class_members.push(unused);
                    }
                }
            }
        }
    }

    (unused_enum_members, unused_class_members)
}

/// Find dependencies used in imports but not listed in package.json.
fn find_unlisted_dependencies(graph: &ModuleGraph, pkg: &PackageJson) -> Vec<UnlistedDependency> {
    let all_deps: HashSet<String> = pkg.all_dependency_names().into_iter().collect();

    let mut unlisted: HashMap<String, Vec<std::path::PathBuf>> = HashMap::new();

    for (package_name, file_ids) in &graph.package_usage {
        if !all_deps.contains(package_name) && !is_builtin_module(package_name) {
            let paths: Vec<std::path::PathBuf> = file_ids
                .iter()
                .filter_map(|id| graph.modules.get(id.0 as usize).map(|m| m.path.clone()))
                .collect();
            unlisted.insert(package_name.clone(), paths);
        }
    }

    unlisted
        .into_iter()
        .map(|(name, paths)| UnlistedDependency {
            package_name: name,
            imported_from: paths,
        })
        .collect()
}

/// Find imports that could not be resolved.
fn find_unresolved_imports(
    resolved_modules: &[ResolvedModule],
    _config: &ResolvedConfig,
) -> Vec<UnresolvedImport> {
    let mut unresolved = Vec::new();

    for module in resolved_modules {
        for import in &module.resolved_imports {
            if let crate::resolve::ResolveResult::Unresolvable(spec) = &import.target {
                unresolved.push(UnresolvedImport {
                    path: module.path.clone(),
                    specifier: spec.clone(),
                    line: import.info.span.start,
                    col: 0,
                });
            }
        }
    }

    unresolved
}

/// Find exports that appear with the same name in multiple files (potential duplicates).
fn find_duplicate_exports(graph: &ModuleGraph, _config: &ResolvedConfig) -> Vec<DuplicateExport> {
    let mut export_locations: HashMap<String, Vec<std::path::PathBuf>> = HashMap::new();

    for module in &graph.modules {
        if !module.is_reachable || module.is_entry_point {
            continue;
        }

        for export in &module.exports {
            if matches!(export.name, crate::extract::ExportName::Default) {
                continue; // Skip default exports
            }
            let name = export.name.to_string();
            export_locations
                .entry(name)
                .or_default()
                .push(module.path.clone());
        }
    }

    export_locations
        .into_iter()
        .filter(|(_, locations)| locations.len() > 1)
        .map(|(name, locations)| DuplicateExport {
            export_name: name,
            locations,
        })
        .collect()
}

/// Check if a package name is a Node.js built-in module.
fn is_builtin_module(name: &str) -> bool {
    let builtins = [
        "assert",
        "buffer",
        "child_process",
        "cluster",
        "console",
        "constants",
        "crypto",
        "dgram",
        "dns",
        "domain",
        "events",
        "fs",
        "http",
        "http2",
        "https",
        "module",
        "net",
        "os",
        "path",
        "perf_hooks",
        "process",
        "punycode",
        "querystring",
        "readline",
        "repl",
        "stream",
        "string_decoder",
        "sys",
        "timers",
        "tls",
        "tty",
        "url",
        "util",
        "v8",
        "vm",
        "wasi",
        "worker_threads",
        "zlib",
    ];
    let stripped = name.strip_prefix("node:").unwrap_or(name);
    builtins.contains(&stripped)
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

    tooling_prefixes.iter().any(|p| name.starts_with(p)) || exact_matches.contains(&name)
}
