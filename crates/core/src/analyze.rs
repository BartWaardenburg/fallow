use std::collections::{HashMap, HashSet};

use fallow_config::{PackageJson, ResolvedConfig};

use crate::extract::MemberKind;
use crate::graph::ModuleGraph;
use crate::resolve::ResolvedModule;
use crate::results::*;

/// Convert a byte offset in source text to a 1-based line and 0-based column (byte offset from
/// start of the line). Uses byte counting to stay consistent with Oxc's byte-offset spans.
fn byte_offset_to_line_col(source: &str, byte_offset: u32) -> (u32, u32) {
    let byte_offset = byte_offset as usize;
    let prefix = &source[..byte_offset.min(source.len())];
    let line = prefix.bytes().filter(|&b| b == b'\n').count() as u32 + 1;
    let col = prefix
        .rfind('\n')
        .map(|pos| byte_offset - pos - 1)
        .unwrap_or(byte_offset) as u32;
    (line, col)
}

/// Read source content from disk, returning empty string on failure.
fn read_source(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

/// Find all dead code in the project.
pub fn find_dead_code(graph: &ModuleGraph, config: &ResolvedConfig) -> AnalysisResults {
    find_dead_code_with_resolved(graph, config, &[], None)
}

/// Find all dead code, with optional resolved module data and plugin context.
pub fn find_dead_code_with_resolved(
    graph: &ModuleGraph,
    config: &ResolvedConfig,
    resolved_modules: &[ResolvedModule],
    plugin_result: Option<&crate::plugins::AggregatedPluginResult>,
) -> AnalysisResults {
    let _span = tracing::info_span!("find_dead_code").entered();

    let mut results = AnalysisResults::default();

    if config.detect.unused_files {
        results.unused_files = find_unused_files(graph);
    }

    if config.detect.unused_exports || config.detect.unused_types {
        let (exports, types) = find_unused_exports(graph, config, plugin_result);
        if config.detect.unused_exports {
            results.unused_exports = exports;
        }
        if config.detect.unused_types {
            results.unused_types = types;
        }
    }

    if config.detect.unused_enum_members || config.detect.unused_class_members {
        let (enum_members, class_members) = find_unused_members(graph, config, resolved_modules);
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
            let (deps, dev_deps) = find_unused_dependencies(graph, &pkg, config, plugin_result);
            if config.detect.unused_dependencies {
                results.unused_dependencies = deps;
            }
            if config.detect.unused_dev_dependencies {
                results.unused_dev_dependencies = dev_deps;
            }
        }

        if config.detect.unlisted_dependencies {
            results.unlisted_dependencies = find_unlisted_dependencies(graph, &pkg);
        }
    }

    if config.detect.unresolved_imports && !resolved_modules.is_empty() {
        results.unresolved_imports = find_unresolved_imports(resolved_modules, config);
    }

    if config.detect.duplicate_exports {
        results.duplicate_exports = find_duplicate_exports(graph, config);
    }

    results
}

/// Find files that are not reachable from any entry point.
///
/// TypeScript declaration files (`.d.ts`) are excluded because they are consumed
/// by the TypeScript compiler via `tsconfig.json` includes, not via explicit
/// import statements. Flagging them as unused is a false positive.
///
/// Configuration files (e.g., `babel.config.js`, `.eslintrc.js`, `knip.config.ts`)
/// are also excluded because they are consumed by tools, not via imports.
///
/// Barrel files (index.ts that only re-export) are excluded when their re-export
/// sources are reachable — they serve an organizational purpose even if consumers
/// import directly from the source files rather than through the barrel.
fn find_unused_files(graph: &ModuleGraph) -> Vec<UnusedFile> {
    graph
        .modules
        .iter()
        .filter(|m| !m.is_reachable && !m.is_entry_point)
        .filter(|m| !is_declaration_file(&m.path))
        .filter(|m| !is_config_file(&m.path))
        .filter(|m| !is_barrel_with_reachable_sources(m, graph))
        .map(|m| UnusedFile {
            path: m.path.clone(),
        })
        .collect()
}

/// Check if a path is a TypeScript declaration file (`.d.ts`, `.d.mts`, `.d.cts`).
fn is_declaration_file(path: &std::path::Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.ends_with(".d.ts") || name.ends_with(".d.mts") || name.ends_with(".d.cts")
}

/// Check if a module is a barrel file (only re-exports) whose sources are reachable.
///
/// A barrel file like `index.ts` that only contains `export { Foo } from './source'`
/// lines serves an organizational purpose. If the source modules are reachable,
/// the barrel file should not be reported as unused — consumers may have bypassed
/// it with direct imports, but the barrel still provides valid re-exports.
fn is_barrel_with_reachable_sources(
    module: &crate::graph::ModuleNode,
    graph: &ModuleGraph,
) -> bool {
    // Must have re-exports
    if module.re_exports.is_empty() {
        return false;
    }

    // Must be a pure barrel: no local exports with real spans (only re-export-generated
    // exports have span 0..0) and no CJS exports
    let has_local_exports = module
        .exports
        .iter()
        .any(|e| e.span.start != 0 || e.span.end != 0);
    if has_local_exports || module.has_cjs_exports {
        return false;
    }

    // At least one re-export source must be reachable
    module.re_exports.iter().any(|re| {
        let source_idx = re.source_file.0 as usize;
        graph
            .modules
            .get(source_idx)
            .is_some_and(|m| m.is_reachable)
    })
}

/// Check if a file is a configuration file consumed by tooling, not via imports.
///
/// These files should never be reported as unused because they are loaded by
/// their respective tools (e.g., Babel reads `babel.config.js`, ESLint reads
/// `eslint.config.ts`, etc.) rather than being imported by application code.
fn is_config_file(path: &std::path::Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Dotfiles with "rc" suffix pattern (e.g., .secretlintrc.cjs, .commitlintrc.js, .prettierrc.js)
    // Only match files with "rc." before the extension — avoids false matches on arbitrary dotfiles.
    if name.starts_with('.') && !name.starts_with("..") {
        let lower = name.to_ascii_lowercase();
        // .foorc.{ext} pattern — standard for tool configs
        if lower.contains("rc.") {
            return true;
        }
    }

    // Files matching common config naming patterns.
    // Each pattern is a prefix — the file must start with it.
    let config_patterns = [
        // Build tools
        "babel.config.",
        "rollup.config.",
        "webpack.config.",
        "postcss.config.",
        "stencil.config.",
        "remotion.config.",
        "metro.config.",
        // Testing
        "jest.config.",
        "jest.setup.",
        "vitest.config.",
        "vitest.ci.config.",
        "vitest.setup.",
        "vitest.workspace.",
        "playwright.config.",
        "cypress.config.",
        // Linting & formatting
        "eslint.config.",
        "prettier.config.",
        "stylelint.config.",
        "lint-staged.config.",
        "commitlint.config.",
        // Frameworks / CMS
        "next.config.",
        "next-sitemap.config.",
        "nuxt.config.",
        "astro.config.",
        "sanity.config.",
        "vite.config.",
        "tailwind.config.",
        "drizzle.config.",
        "knexfile.",
        "sentry.client.config.",
        "sentry.server.config.",
        "sentry.edge.config.",
        "react-router.config.",
        // Analysis & misc
        "knip.config.",
        "fallow.config.",
        "i18next-parser.config.",
        "codegen.config.",
        "graphql.config.",
        "npmpackagejsonlint.config.",
        // Environment declarations
        "next-env.d.",
        "env.d.",
        "vite-env.d.",
    ];

    config_patterns.iter().any(|p| name.starts_with(p))
}

/// Find exports that are never imported by other files.
fn find_unused_exports(
    graph: &ModuleGraph,
    config: &ResolvedConfig,
    plugin_result: Option<&crate::plugins::AggregatedPluginResult>,
) -> (Vec<UnusedExport>, Vec<UnusedExport>) {
    let mut unused_exports = Vec::new();
    let mut unused_types = Vec::new();

    // Pre-compile glob matchers for ignore rules and framework rules
    let ignore_matchers: Vec<(globset::GlobMatcher, &[String])> = config
        .ignore_export_rules
        .iter()
        .filter_map(|rule| {
            globset::Glob::new(&rule.file)
                .ok()
                .map(|g| (g.compile_matcher(), rule.exports.as_slice()))
        })
        .collect();

    let framework_matchers: Vec<(globset::GlobMatcher, &[String])> = config
        .framework_rules
        .iter()
        .flat_map(|rule| &rule.used_exports)
        .filter_map(|used| {
            globset::Glob::new(&used.file_pattern)
                .ok()
                .map(|g| (g.compile_matcher(), used.exports.as_slice()))
        })
        .collect();

    // Also compile plugin-discovered used_exports rules
    let plugin_matchers: Vec<(globset::GlobMatcher, Vec<&str>)> = plugin_result
        .map(|pr| {
            pr.used_exports
                .iter()
                .filter_map(|(file_pat, exports)| {
                    globset::Glob::new(file_pat).ok().map(|g| {
                        (
                            g.compile_matcher(),
                            exports.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                        )
                    })
                })
                .collect()
        })
        .unwrap_or_default();

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
        // If so, all exports are conservatively considered used — O(1) lookup
        if graph.has_namespace_import(module.file_id) {
            continue;
        }

        // Check ignore rules — compute relative path and string once per module
        let relative_path = module
            .path
            .strip_prefix(&config.root)
            .unwrap_or(&module.path);
        let file_str = relative_path.to_string_lossy();

        // Pre-check which ignore/framework matchers match this file
        let matching_ignore: Vec<&[String]> = ignore_matchers
            .iter()
            .filter(|(m, _)| m.is_match(file_str.as_ref()))
            .map(|(_, exports)| *exports)
            .collect();

        let matching_framework: Vec<&[String]> = framework_matchers
            .iter()
            .filter(|(m, _)| m.is_match(file_str.as_ref()))
            .map(|(_, exports)| *exports)
            .collect();

        // Check plugin-discovered used_exports rules
        let matching_plugin: Vec<&Vec<&str>> = plugin_matchers
            .iter()
            .filter(|(m, _)| m.is_match(file_str.as_ref()))
            .map(|(_, exports)| exports)
            .collect();

        // Lazily load source content for line/col computation
        let mut source_content: Option<String> = None;

        for export in &module.exports {
            if export.references.is_empty() {
                let export_str = export.name.to_string();

                // Check if this export is ignored by config
                if matching_ignore
                    .iter()
                    .any(|exports| exports.iter().any(|e| e == "*" || e == &export_str))
                {
                    continue;
                }

                // Check if this export is considered "used" by a framework rule
                if matching_framework
                    .iter()
                    .any(|exports| exports.iter().any(|e| e == &export_str))
                {
                    continue;
                }

                // Check if this export is considered "used" by a plugin rule
                if matching_plugin
                    .iter()
                    .any(|exports| exports.iter().any(|e| *e == export_str))
                {
                    continue;
                }

                let source = source_content.get_or_insert_with(|| read_source(&module.path));
                let (line, col) = byte_offset_to_line_col(source, export.span.start);

                let unused = UnusedExport {
                    path: module.path.clone(),
                    export_name: export_str,
                    is_type_only: export.is_type_only,
                    line,
                    col,
                    span_start: export.span.start,
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
    plugin_result: Option<&crate::plugins::AggregatedPluginResult>,
) -> (Vec<UnusedDependency>, Vec<UnusedDependency>) {
    let used_packages: HashSet<&str> = graph.package_usage.keys().map(|s| s.as_str()).collect();

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

    let unused_deps: Vec<UnusedDependency> = pkg
        .production_dependency_names()
        .into_iter()
        .filter(|dep| !used_packages.contains(dep.as_str()))
        .filter(|dep| !is_implicit_dependency(dep))
        .filter(|dep| !plugin_referenced.contains(dep.as_str()))
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
        .filter(|dep| !plugin_tooling.contains(dep.as_str()))
        .filter(|dep| !plugin_referenced.contains(dep.as_str()))
        .filter(|dep| !config.ignore_dependencies.iter().any(|d| d == dep))
        .map(|dep| UnusedDependency {
            package_name: dep,
            location: DependencyLocation::DevDependencies,
        })
        .collect();

    (unused_deps, unused_dev_deps)
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

        // Lazily load source content for line/col computation
        let mut source_content: Option<String> = None;

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

                // Skip React class component lifecycle methods — they are called by the
                // React runtime, not user code, so they should never be flagged as unused.
                // Also skip Angular lifecycle hooks (OnInit, OnDestroy, etc.).
                if matches!(
                    member.kind,
                    MemberKind::ClassMethod | MemberKind::ClassProperty
                ) && (is_react_lifecycle_method(&member.name)
                    || is_angular_lifecycle_method(&member.name))
                {
                    continue;
                }

                let source = source_content.get_or_insert_with(|| read_source(&module.path));
                let (line, col) = byte_offset_to_line_col(source, member.span.start);

                let unused = UnusedMember {
                    path: module.path.clone(),
                    parent_name: export_name.clone(),
                    member_name: member.name.clone(),
                    kind: member.kind.clone(),
                    line,
                    col,
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
        if !all_deps.contains(package_name)
            && !is_builtin_module(package_name)
            && !is_path_alias(package_name)
        {
            let mut paths: Vec<std::path::PathBuf> = file_ids
                .iter()
                .filter_map(|id| graph.modules.get(id.0 as usize).map(|m| m.path.clone()))
                .collect();
            paths.sort();
            paths.dedup();
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

/// Check if a package name looks like a TypeScript path alias rather than an npm package.
///
/// Common patterns: `@/components`, `@app/utils`, `~/lib`, `#internal/module`,
/// `@Components/Button` (PascalCase tsconfig paths).
/// These are typically defined in tsconfig.json `paths` or package.json `imports`.
fn is_path_alias(name: &str) -> bool {
    // `#` prefix is Node.js imports maps (package.json "imports" field)
    if name.starts_with('#') {
        return true;
    }
    // `~/` prefix is a common alias convention (e.g., Nuxt, custom tsconfig)
    if name.starts_with("~/") {
        return true;
    }
    // `@/` is a very common path alias (e.g., `@/components/Foo`)
    if name.starts_with("@/") {
        return true;
    }
    // npm scoped packages MUST be lowercase (npm registry requirement).
    // PascalCase `@Scope` or `@Scope/path` patterns are tsconfig path aliases,
    // not npm packages. E.g., `@Components`, `@Hooks/useApi`, `@Services/auth`.
    if name.starts_with('@') {
        let scope = name.split('/').next().unwrap_or(name);
        if scope.len() > 1 && scope.chars().nth(1).is_some_and(|c| c.is_ascii_uppercase()) {
            return true;
        }
    }

    false
}

/// Find imports that could not be resolved.
fn find_unresolved_imports(
    resolved_modules: &[ResolvedModule],
    _config: &ResolvedConfig,
) -> Vec<UnresolvedImport> {
    let mut unresolved = Vec::new();

    for module in resolved_modules {
        // Lazily load source content for line/col computation
        let mut source_content: Option<String> = None;

        for import in &module.resolved_imports {
            if let crate::resolve::ResolveResult::Unresolvable(spec) = &import.target {
                let source = source_content.get_or_insert_with(|| read_source(&module.path));
                let (line, col) = byte_offset_to_line_col(source, import.info.span.start);

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

/// Find exports that appear with the same name in multiple files (potential duplicates).
///
/// Barrel re-exports (files that only re-export from other modules via `export { X } from './source'`)
/// are excluded — having an index.ts re-export the same name as the source module is the normal
/// barrel file pattern, not a true duplicate.
fn find_duplicate_exports(graph: &ModuleGraph, _config: &ResolvedConfig) -> Vec<DuplicateExport> {
    // Pre-compute which modules are pure barrel files (only re-exports, no local exports with spans)
    let barrel_modules: HashSet<usize> = graph
        .modules
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            !m.re_exports.is_empty()
                && m.exports
                    .iter()
                    .all(|e| e.span.start == 0 && e.span.end == 0)
        })
        .map(|(i, _)| i)
        .collect();

    let mut export_locations: HashMap<String, Vec<std::path::PathBuf>> = HashMap::new();

    for (idx, module) in graph.modules.iter().enumerate() {
        if !module.is_reachable || module.is_entry_point {
            continue;
        }

        // Skip barrel files — their re-exported names are not true duplicates
        if barrel_modules.contains(&idx) {
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
        "assert/strict",
        "async_hooks",
        "buffer",
        "child_process",
        "cluster",
        "console",
        "constants",
        "crypto",
        "dgram",
        "diagnostics_channel",
        "dns",
        "dns/promises",
        "domain",
        "events",
        "fs",
        "fs/promises",
        "http",
        "http2",
        "https",
        "inspector",
        "inspector/promises",
        "module",
        "net",
        "os",
        "path",
        "path/posix",
        "path/win32",
        "perf_hooks",
        "process",
        "punycode",
        "querystring",
        "readline",
        "readline/promises",
        "repl",
        "stream",
        "stream/consumers",
        "stream/promises",
        "stream/web",
        "string_decoder",
        "sys",
        "test",
        "test/reporters",
        "timers",
        "timers/promises",
        "tls",
        "trace_events",
        "tty",
        "url",
        "util",
        "util/types",
        "v8",
        "vm",
        "wasi",
        "worker_threads",
        "zlib",
    ];
    let stripped = name.strip_prefix("node:").unwrap_or(name);
    // Check exact match or subpath (e.g., "fs/promises" matches "fs/promises",
    // "assert/strict" matches "assert/strict")
    builtins.contains(&stripped) || {
        // Handle deep subpaths like "stream/consumers" or "test/reporters"
        stripped
            .split('/')
            .next()
            .is_some_and(|root| builtins.contains(&root))
    }
}

/// Dependencies that are used implicitly (not via imports).
fn is_implicit_dependency(name: &str) -> bool {
    if name.starts_with("@types/") {
        return true;
    }

    // Framework runtime dependencies that are used implicitly (e.g., JSX runtime,
    // bundler injection) and never appear as explicit imports in source code.
    let implicit_deps = [
        "react-dom",
        "react-dom/client",
        "react-native",
        "@next/font",
        "@next/mdx",
        "@next/bundle-analyzer",
        "@next/env",
        // WebSocket optional native addons (peer deps of ws)
        "utf-8-validate",
        "bufferutil",
    ];
    implicit_deps.contains(&name)
}

/// Dev dependencies that are tooling (used by CLI, not imported in code).
fn is_tooling_dependency(name: &str) -> bool {
    let tooling_prefixes = [
        "@types/",
        "eslint",
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
        "@vitest/",
        "@jest/",
        "@testing-library/",
        "@playwright/",
        "@storybook/",
        "storybook",
        "@babel/",
        "babel-",
        "@react-native-community/cli",
        "@react-native/",
        "secretlint",
        "@secretlint/",
        "oxlint",
        // Release & publishing tooling
        "@semantic-release/",
        "semantic-release",
        "@release-it/",
        "@lerna-lite/",
        // Build tool plugins (used in config)
        "@graphql-codegen/",
        "@rollup/",
    ];

    let exact_matches = [
        "typescript",
        "prettier",
        "turbo",
        "concurrently",
        "cross-env",
        "rimraf",
        "npm-run-all",
        "npm-run-all2",
        "nodemon",
        "ts-node",
        "tsx",
        "knip",
        "fallow",
        "jest",
        "vitest",
        "happy-dom",
        "jsdom",
        "vite",
        "sass",
        "sass-embedded",
        "webpack",
        "webpack-cli",
        "webpack-dev-server",
        "esbuild",
        "rollup",
        "swc",
        "@swc/core",
        "@swc/jest",
        "terser",
        "cssnano",
        "sharp",
        // Release & publishing
        "release-it",
        "lerna",
        // Dotenv CLI tools
        "dotenv-cli",
        "dotenv-flow",
        // Code quality & analysis
        "oxfmt",
        "jscpd",
        "npm-check-updates",
        "markdownlint-cli",
        "npm-package-json-lint",
        "synp",
        "flow-bin",
        // i18n tooling
        "i18next-parser",
        "i18next-conv",
        // Bundle analysis & build tooling
        "webpack-bundle-analyzer",
        // Vite plugins (used in config, not imported)
        "vite-plugin-svgr",
        "vite-plugin-eslint",
        "@vitejs/plugin-vue",
        "@vitejs/plugin-react",
        // Site generation / SEO
        "next-sitemap",
        // Monorepo tools
        "nx",
        // Vue tooling
        "vue-tsc",
        "@vue/tsconfig",
        "@tsconfig/node20",
        "@tsconfig/react-native",
        // TypeScript experimental
        "@typescript/native-preview",
        // CSS-only deps (not imported in JS)
        "tw-animate-css",
    ];

    tooling_prefixes.iter().any(|p| name.starts_with(p)) || exact_matches.contains(&name)
}

/// Angular lifecycle hooks and framework-invoked methods.
///
/// These should never be flagged as unused class members because they are
/// called by the Angular framework, not user code.
fn is_angular_lifecycle_method(name: &str) -> bool {
    matches!(
        name,
        "ngOnInit"
            | "ngOnDestroy"
            | "ngOnChanges"
            | "ngDoCheck"
            | "ngAfterContentInit"
            | "ngAfterContentChecked"
            | "ngAfterViewInit"
            | "ngAfterViewChecked"
            | "ngAcceptInputType"
            // Angular guard/resolver/interceptor methods
            | "canActivate"
            | "canDeactivate"
            | "canActivateChild"
            | "canMatch"
            | "resolve"
            | "intercept"
            | "transform"
            // Angular form-related methods
            | "validate"
            | "registerOnChange"
            | "registerOnTouched"
            | "writeValue"
            | "setDisabledState"
    )
}

fn is_react_lifecycle_method(name: &str) -> bool {
    matches!(
        name,
        "render"
            | "componentDidMount"
            | "componentDidUpdate"
            | "componentWillUnmount"
            | "shouldComponentUpdate"
            | "getSnapshotBeforeUpdate"
            | "getDerivedStateFromProps"
            | "getDerivedStateFromError"
            | "componentDidCatch"
            | "componentWillMount"
            | "componentWillReceiveProps"
            | "componentWillUpdate"
            | "UNSAFE_componentWillMount"
            | "UNSAFE_componentWillReceiveProps"
            | "UNSAFE_componentWillUpdate"
            | "getChildContext"
            | "contextType"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // is_builtin_module tests
    #[test]
    fn builtin_module_fs() {
        assert!(is_builtin_module("fs"));
    }

    #[test]
    fn builtin_module_path() {
        assert!(is_builtin_module("path"));
    }

    #[test]
    fn builtin_module_with_node_prefix() {
        assert!(is_builtin_module("node:fs"));
        assert!(is_builtin_module("node:path"));
        assert!(is_builtin_module("node:crypto"));
    }

    #[test]
    fn builtin_module_all_known() {
        let known = [
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
        for name in &known {
            assert!(is_builtin_module(name), "{name} should be a builtin module");
        }
    }

    #[test]
    fn not_builtin_module() {
        assert!(!is_builtin_module("react"));
        assert!(!is_builtin_module("lodash"));
        assert!(!is_builtin_module("express"));
        assert!(!is_builtin_module("@scope/pkg"));
    }

    #[test]
    fn not_builtin_similar_names() {
        assert!(!is_builtin_module("filesystem"));
        assert!(!is_builtin_module("pathlib"));
        assert!(!is_builtin_module("node:react"));
    }

    // is_implicit_dependency tests
    #[test]
    fn implicit_dep_types_packages() {
        assert!(is_implicit_dependency("@types/node"));
        assert!(is_implicit_dependency("@types/react"));
        assert!(is_implicit_dependency("@types/jest"));
    }

    #[test]
    fn not_implicit_dep() {
        assert!(!is_implicit_dependency("react"));
        assert!(!is_implicit_dependency("@scope/types"));
        assert!(!is_implicit_dependency("types"));
        assert!(!is_implicit_dependency("typescript"));
        assert!(!is_implicit_dependency("prettier"));
        assert!(!is_implicit_dependency("eslint"));
    }

    // is_tooling_dependency tests
    #[test]
    fn tooling_dep_prefixes() {
        assert!(is_tooling_dependency("@types/node"));
        assert!(is_tooling_dependency("eslint"));
        assert!(is_tooling_dependency("eslint-plugin-react"));
        assert!(is_tooling_dependency("prettier"));
        assert!(is_tooling_dependency("@typescript-eslint/parser"));
        assert!(is_tooling_dependency("husky"));
        assert!(is_tooling_dependency("lint-staged"));
        assert!(is_tooling_dependency("commitlint"));
        assert!(is_tooling_dependency("@commitlint/config-conventional"));
        assert!(is_tooling_dependency("stylelint"));
        assert!(is_tooling_dependency("postcss"));
        assert!(is_tooling_dependency("autoprefixer"));
        assert!(is_tooling_dependency("tailwindcss"));
        assert!(is_tooling_dependency("@tailwindcss/forms"));
    }

    #[test]
    fn tooling_dep_exact_matches() {
        assert!(is_tooling_dependency("typescript"));
        assert!(is_tooling_dependency("prettier"));
        assert!(is_tooling_dependency("turbo"));
        assert!(is_tooling_dependency("concurrently"));
        assert!(is_tooling_dependency("cross-env"));
        assert!(is_tooling_dependency("rimraf"));
        assert!(is_tooling_dependency("npm-run-all"));
        assert!(is_tooling_dependency("nodemon"));
        assert!(is_tooling_dependency("ts-node"));
        assert!(is_tooling_dependency("tsx"));
    }

    #[test]
    fn not_tooling_dep() {
        assert!(!is_tooling_dependency("react"));
        assert!(!is_tooling_dependency("next"));
        assert!(!is_tooling_dependency("lodash"));
        assert!(!is_tooling_dependency("express"));
        assert!(!is_tooling_dependency("@emotion/react"));
    }

    // New tooling dependency tests (Issue 2)
    #[test]
    fn tooling_dep_testing_frameworks() {
        assert!(is_tooling_dependency("jest"));
        assert!(is_tooling_dependency("vitest"));
        assert!(is_tooling_dependency("@jest/globals"));
        assert!(is_tooling_dependency("@vitest/coverage-v8"));
        assert!(is_tooling_dependency("@testing-library/react"));
        assert!(is_tooling_dependency("@testing-library/jest-dom"));
        assert!(is_tooling_dependency("@playwright/test"));
    }

    #[test]
    fn tooling_dep_environments_and_cli() {
        assert!(is_tooling_dependency("happy-dom"));
        assert!(is_tooling_dependency("jsdom"));
        assert!(is_tooling_dependency("knip"));
    }

    // React lifecycle method tests (Issue 1)
    #[test]
    fn react_lifecycle_standard_methods() {
        assert!(is_react_lifecycle_method("render"));
        assert!(is_react_lifecycle_method("componentDidMount"));
        assert!(is_react_lifecycle_method("componentDidUpdate"));
        assert!(is_react_lifecycle_method("componentWillUnmount"));
        assert!(is_react_lifecycle_method("shouldComponentUpdate"));
        assert!(is_react_lifecycle_method("getSnapshotBeforeUpdate"));
    }

    #[test]
    fn react_lifecycle_static_methods() {
        assert!(is_react_lifecycle_method("getDerivedStateFromProps"));
        assert!(is_react_lifecycle_method("getDerivedStateFromError"));
    }

    #[test]
    fn react_lifecycle_error_boundary() {
        assert!(is_react_lifecycle_method("componentDidCatch"));
    }

    #[test]
    fn react_lifecycle_deprecated_and_unsafe() {
        assert!(is_react_lifecycle_method("componentWillMount"));
        assert!(is_react_lifecycle_method("componentWillReceiveProps"));
        assert!(is_react_lifecycle_method("componentWillUpdate"));
        assert!(is_react_lifecycle_method("UNSAFE_componentWillMount"));
        assert!(is_react_lifecycle_method(
            "UNSAFE_componentWillReceiveProps"
        ));
        assert!(is_react_lifecycle_method("UNSAFE_componentWillUpdate"));
    }

    #[test]
    fn react_lifecycle_context_methods() {
        assert!(is_react_lifecycle_method("getChildContext"));
        assert!(is_react_lifecycle_method("contextType"));
    }

    #[test]
    fn not_react_lifecycle_method() {
        assert!(!is_react_lifecycle_method("handleClick"));
        assert!(!is_react_lifecycle_method("fetchData"));
        assert!(!is_react_lifecycle_method("constructor"));
        assert!(!is_react_lifecycle_method("setState"));
        assert!(!is_react_lifecycle_method("forceUpdate"));
        assert!(!is_react_lifecycle_method("customMethod"));
    }

    // Declaration file tests (Issue 4)
    #[test]
    fn declaration_file_dts() {
        assert!(is_declaration_file(std::path::Path::new("styled.d.ts")));
        assert!(is_declaration_file(std::path::Path::new(
            "src/types/styled.d.ts"
        )));
        assert!(is_declaration_file(std::path::Path::new("env.d.ts")));
    }

    #[test]
    fn declaration_file_dmts_dcts() {
        assert!(is_declaration_file(std::path::Path::new("module.d.mts")));
        assert!(is_declaration_file(std::path::Path::new("module.d.cts")));
    }

    #[test]
    fn not_declaration_file() {
        assert!(!is_declaration_file(std::path::Path::new("index.ts")));
        assert!(!is_declaration_file(std::path::Path::new("component.tsx")));
        assert!(!is_declaration_file(std::path::Path::new("utils.js")));
        assert!(!is_declaration_file(std::path::Path::new("styles.d.css")));
    }

    // byte_offset_to_line_col tests
    #[test]
    fn byte_offset_empty_source() {
        assert_eq!(byte_offset_to_line_col("", 0), (1, 0));
    }

    #[test]
    fn byte_offset_single_line_start() {
        assert_eq!(byte_offset_to_line_col("hello", 0), (1, 0));
    }

    #[test]
    fn byte_offset_single_line_middle() {
        assert_eq!(byte_offset_to_line_col("hello", 4), (1, 4));
    }

    #[test]
    fn byte_offset_multiline_start_of_line2() {
        // "line1\nline2\nline3"
        //  01234 5 678901 2
        // offset 6 = start of "line2"
        let source = "line1\nline2\nline3";
        assert_eq!(byte_offset_to_line_col(source, 6), (2, 0));
    }

    #[test]
    fn byte_offset_multiline_middle_of_line3() {
        // "line1\nline2\nline3"
        //  01234 5 67890 1 23456
        //                1 12345
        // offset 14 = 'n' in "line3" (col 2)
        let source = "line1\nline2\nline3";
        assert_eq!(byte_offset_to_line_col(source, 14), (3, 2));
    }

    #[test]
    fn byte_offset_at_newline_boundary() {
        // "line1\nline2"
        // offset 5 = the '\n' character itself
        let source = "line1\nline2";
        assert_eq!(byte_offset_to_line_col(source, 5), (1, 5));
    }

    #[test]
    fn byte_offset_beyond_source_length() {
        // Line count is clamped (prefix is sliced to source.len()), but the
        // byte-offset column is passed through unclamped because the function
        // uses the raw byte_offset for the column fallback.
        let source = "hello";
        assert_eq!(byte_offset_to_line_col(source, 100), (1, 100));
    }

    #[test]
    fn byte_offset_multibyte_utf8() {
        // Emoji is 4 bytes: "hi\n" (3 bytes) + emoji (4 bytes) + "x" (1 byte)
        let source = "hi\n\u{1F600}x";
        // offset 3 = start of line 2, col 0
        assert_eq!(byte_offset_to_line_col(source, 3), (2, 0));
        // offset 7 = 'x' (after 4-byte emoji), col 4 (byte-based)
        assert_eq!(byte_offset_to_line_col(source, 7), (2, 4));
    }

    #[test]
    fn byte_offset_multibyte_accented_chars() {
        // 'e' with accent (U+00E9) is 2 bytes in UTF-8
        let source = "caf\u{00E9}\nbar";
        // "caf\u{00E9}" = 3 + 2 = 5 bytes, then '\n' at offset 5
        // 'b' at offset 6 → line 2, col 0
        assert_eq!(byte_offset_to_line_col(source, 6), (2, 0));
        // '\u{00E9}' starts at offset 3, col 3 (byte-based)
        assert_eq!(byte_offset_to_line_col(source, 3), (1, 3));
    }

    // is_path_alias tests
    #[test]
    fn path_alias_at_slash() {
        assert!(is_path_alias("@/components"));
    }

    #[test]
    fn path_alias_tilde() {
        assert!(is_path_alias("~/lib"));
    }

    #[test]
    fn path_alias_hash_imports_map() {
        assert!(is_path_alias("#internal/module"));
    }

    #[test]
    fn path_alias_pascal_case_scope() {
        assert!(is_path_alias("@Components/Button"));
    }

    #[test]
    fn not_path_alias_regular_package() {
        assert!(!is_path_alias("react"));
    }

    #[test]
    fn not_path_alias_scoped_npm_package() {
        assert!(!is_path_alias("@scope/pkg"));
    }

    #[test]
    fn not_path_alias_emotion_react() {
        assert!(!is_path_alias("@emotion/react"));
    }

    #[test]
    fn not_path_alias_lodash() {
        assert!(!is_path_alias("lodash"));
    }

    #[test]
    fn not_path_alias_lowercase_short_scope() {
        assert!(!is_path_alias("@s/lowercase"));
    }

    // is_angular_lifecycle_method tests
    #[test]
    fn angular_lifecycle_core_hooks() {
        assert!(is_angular_lifecycle_method("ngOnInit"));
        assert!(is_angular_lifecycle_method("ngOnDestroy"));
        assert!(is_angular_lifecycle_method("ngOnChanges"));
        assert!(is_angular_lifecycle_method("ngAfterViewInit"));
    }

    #[test]
    fn angular_lifecycle_check_hooks() {
        assert!(is_angular_lifecycle_method("ngDoCheck"));
        assert!(is_angular_lifecycle_method("ngAfterContentChecked"));
        assert!(is_angular_lifecycle_method("ngAfterViewChecked"));
    }

    #[test]
    fn angular_lifecycle_content_hooks() {
        assert!(is_angular_lifecycle_method("ngAfterContentInit"));
        assert!(is_angular_lifecycle_method("ngAcceptInputType"));
    }

    #[test]
    fn angular_lifecycle_guard_resolver_methods() {
        assert!(is_angular_lifecycle_method("canActivate"));
        assert!(is_angular_lifecycle_method("canDeactivate"));
        assert!(is_angular_lifecycle_method("canActivateChild"));
        assert!(is_angular_lifecycle_method("canMatch"));
        assert!(is_angular_lifecycle_method("resolve"));
        assert!(is_angular_lifecycle_method("intercept"));
        assert!(is_angular_lifecycle_method("transform"));
    }

    #[test]
    fn angular_lifecycle_form_methods() {
        assert!(is_angular_lifecycle_method("validate"));
        assert!(is_angular_lifecycle_method("registerOnChange"));
        assert!(is_angular_lifecycle_method("registerOnTouched"));
        assert!(is_angular_lifecycle_method("writeValue"));
        assert!(is_angular_lifecycle_method("setDisabledState"));
    }

    #[test]
    fn not_angular_lifecycle_method() {
        assert!(!is_angular_lifecycle_method("onClick"));
        assert!(!is_angular_lifecycle_method("handleSubmit"));
        assert!(!is_angular_lifecycle_method("render"));
    }
}
