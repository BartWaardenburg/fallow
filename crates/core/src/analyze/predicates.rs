/// Check if a path is a TypeScript declaration file (`.d.ts`, `.d.mts`, `.d.cts`).
pub(super) fn is_declaration_file(path: &std::path::Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.ends_with(".d.ts") || name.ends_with(".d.mts") || name.ends_with(".d.cts")
}

/// Check if a file is a configuration file consumed by tooling, not via imports.
///
/// These files should never be reported as unused because they are loaded by
/// their respective tools (e.g., Babel reads `babel.config.js`, `ESLint` reads
/// `eslint.config.ts`, etc.) rather than being imported by application code.
pub(super) fn is_config_file(path: &std::path::Path) -> bool {
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
        "tsup.config.",
        "unbuild.config.",
        "esbuild.config.",
        "swc.config.",
        "turbo.",
        // Testing
        "jest.config.",
        "jest.setup.",
        "vitest.config.",
        "vitest.ci.config.",
        "vitest.setup.",
        "vitest.workspace.",
        "playwright.config.",
        "cypress.config.",
        "karma.conf.",
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
        // Documentation
        "typedoc.",
        // Analysis & misc
        "knip.config.",
        "fallow.config.",
        "i18next-parser.config.",
        "codegen.config.",
        "graphql.config.",
        "npmpackagejsonlint.config.",
        "release-it.",
        "release.config.",
        "contentlayer.config.",
        // Environment declarations
        "next-env.d.",
        "env.d.",
        "vite-env.d.",
    ];

    config_patterns.iter().any(|p| name.starts_with(p))
}

/// Check if a package name is a platform built-in module (Node.js, Deno, Cloudflare Workers).
pub fn is_builtin_module(name: &str) -> bool {
    // Cloudflare Workers built-in modules (e.g., `cloudflare:workers`, `cloudflare:sockets`)
    if name.starts_with("cloudflare:") {
        return true;
    }
    // Deno standard library — imported as bare `std` or subpaths like `std/path`
    // (Deno also uses `jsr:@std/` but that would be extracted differently)
    if name == "std" || name.starts_with("std/") {
        return true;
    }
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
pub(super) fn is_implicit_dependency(name: &str) -> bool {
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

/// Check if a package name looks like a TypeScript path alias rather than an npm package.
///
/// Common patterns: `@/components`, `@app/utils`, `~/lib`, `#internal/module`,
/// `@Components/Button` (`PascalCase` tsconfig paths).
/// These are typically defined in tsconfig.json `paths` or package.json `imports`.
pub(super) fn is_path_alias(name: &str) -> bool {
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

/// Angular lifecycle hooks and framework-invoked methods.
///
/// These should never be flagged as unused class members because they are
/// called by the Angular framework, not user code.
pub(super) fn is_angular_lifecycle_method(name: &str) -> bool {
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

pub(super) fn is_react_lifecycle_method(name: &str) -> bool {
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

/// Check if a module is a barrel file (only re-exports) whose sources are reachable.
///
/// A barrel file like `index.ts` that only contains `export { Foo } from './source'`
/// lines serves an organizational purpose. If the source modules are reachable,
/// the barrel file should not be reported as unused — consumers may have bypassed
/// it with direct imports, but the barrel still provides valid re-exports.
pub(super) fn is_barrel_with_reachable_sources(
    module: &crate::graph::ModuleNode,
    graph: &crate::graph::ModuleGraph,
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
        assert!(crate::plugins::is_known_tooling_dependency("@types/node"));
        assert!(crate::plugins::is_known_tooling_dependency("eslint"));
        assert!(crate::plugins::is_known_tooling_dependency(
            "eslint-plugin-react"
        ));
        assert!(crate::plugins::is_known_tooling_dependency("prettier"));
        assert!(crate::plugins::is_known_tooling_dependency(
            "@typescript-eslint/parser"
        ));
        assert!(crate::plugins::is_known_tooling_dependency("husky"));
        assert!(crate::plugins::is_known_tooling_dependency("lint-staged"));
        assert!(crate::plugins::is_known_tooling_dependency("commitlint"));
        assert!(crate::plugins::is_known_tooling_dependency(
            "@commitlint/config-conventional"
        ));
        assert!(crate::plugins::is_known_tooling_dependency("stylelint"));
        assert!(crate::plugins::is_known_tooling_dependency("postcss"));
        assert!(crate::plugins::is_known_tooling_dependency("autoprefixer"));
        assert!(crate::plugins::is_known_tooling_dependency("tailwindcss"));
        assert!(crate::plugins::is_known_tooling_dependency(
            "@tailwindcss/forms"
        ));
    }

    #[test]
    fn tooling_dep_exact_matches() {
        assert!(crate::plugins::is_known_tooling_dependency("typescript"));
        assert!(crate::plugins::is_known_tooling_dependency("prettier"));
        assert!(crate::plugins::is_known_tooling_dependency("turbo"));
        assert!(crate::plugins::is_known_tooling_dependency("concurrently"));
        assert!(crate::plugins::is_known_tooling_dependency("cross-env"));
        assert!(crate::plugins::is_known_tooling_dependency("rimraf"));
        assert!(crate::plugins::is_known_tooling_dependency("npm-run-all"));
        assert!(crate::plugins::is_known_tooling_dependency("nodemon"));
        assert!(crate::plugins::is_known_tooling_dependency("ts-node"));
        assert!(crate::plugins::is_known_tooling_dependency("tsx"));
    }

    #[test]
    fn not_tooling_dep() {
        assert!(!crate::plugins::is_known_tooling_dependency("react"));
        assert!(!crate::plugins::is_known_tooling_dependency("next"));
        assert!(!crate::plugins::is_known_tooling_dependency("lodash"));
        assert!(!crate::plugins::is_known_tooling_dependency("express"));
        assert!(!crate::plugins::is_known_tooling_dependency(
            "@emotion/react"
        ));
    }

    // New tooling dependency tests (Issue 2)
    #[test]
    fn tooling_dep_testing_frameworks() {
        assert!(crate::plugins::is_known_tooling_dependency("jest"));
        assert!(crate::plugins::is_known_tooling_dependency("vitest"));
        assert!(crate::plugins::is_known_tooling_dependency("@jest/globals"));
        assert!(crate::plugins::is_known_tooling_dependency(
            "@vitest/coverage-v8"
        ));
        assert!(crate::plugins::is_known_tooling_dependency(
            "@testing-library/react"
        ));
        assert!(crate::plugins::is_known_tooling_dependency(
            "@testing-library/jest-dom"
        ));
        assert!(crate::plugins::is_known_tooling_dependency(
            "@playwright/test"
        ));
    }

    #[test]
    fn tooling_dep_environments_and_cli() {
        assert!(crate::plugins::is_known_tooling_dependency("happy-dom"));
        assert!(crate::plugins::is_known_tooling_dependency("jsdom"));
        assert!(crate::plugins::is_known_tooling_dependency("knip"));
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

    // Config file tests
    #[test]
    fn config_file_known_patterns() {
        assert!(is_config_file(std::path::Path::new("webpack.config.js")));
        assert!(is_config_file(std::path::Path::new("jest.config.ts")));
        assert!(is_config_file(std::path::Path::new("karma.conf.js")));
        assert!(is_config_file(std::path::Path::new("vite.config.mts")));
        assert!(is_config_file(std::path::Path::new("playwright.config.ts")));
        assert!(is_config_file(std::path::Path::new("eslint.config.mjs")));
    }

    #[test]
    fn config_file_dotrc_pattern() {
        assert!(is_config_file(std::path::Path::new(".eslintrc.js")));
        assert!(is_config_file(std::path::Path::new(".babelrc.json")));
    }

    #[test]
    fn not_config_file() {
        assert!(!is_config_file(std::path::Path::new("index.ts")));
        assert!(!is_config_file(std::path::Path::new("utils.js")));
        assert!(!is_config_file(std::path::Path::new("config.ts")));
        assert!(!is_config_file(std::path::Path::new(
            "src/webpack-plugin.js"
        )));
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
