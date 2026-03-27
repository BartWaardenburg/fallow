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

/// Check if an import specifier is a virtual module that does not correspond to a real file.
///
/// The `virtual:` prefix is a convention established by Vite and widely adopted across
/// the JS/TS bundler ecosystem. Plugins create virtual modules with this prefix
/// (e.g., `virtual:pwa-register`, `virtual:uno.css`, `virtual:generated-pages`).
/// These should never be flagged as unlisted dependencies or unresolved imports.
pub fn is_virtual_module(name: &str) -> bool {
    name.starts_with("virtual:")
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
    // All known builtins and their subpaths (fs/promises, path/posix, test/reporters,
    // stream/consumers, etc.) are listed explicitly in the array above.
    // No fallback root-segment matching — it would false-positive on npm packages
    // like test-utils, url-parse, path-browserify, stream-browserify, events-emitter.
    builtins.contains(&stripped)
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

    /// Regression: npm packages whose name starts with a Node builtin name
    /// (e.g., "test-utils", "url-parse") must not be classified as builtins.
    #[test]
    fn not_builtin_npm_packages_with_builtin_prefix() {
        assert!(!is_builtin_module("test-utils/helpers"));
        assert!(!is_builtin_module("url-parse"));
        assert!(!is_builtin_module("path-browserify"));
        assert!(!is_builtin_module("stream-browserify"));
        assert!(!is_builtin_module("events-emitter"));
        assert!(!is_builtin_module("util-deprecate"));
        assert!(!is_builtin_module("os-tmpdir"));
        assert!(!is_builtin_module("net-ping"));
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

    // is_virtual_module tests
    #[test]
    fn virtual_module_vite_convention() {
        assert!(is_virtual_module("virtual:pwa-register"));
        assert!(is_virtual_module("virtual:pwa-register/react"));
        assert!(is_virtual_module("virtual:uno.css"));
        assert!(is_virtual_module("virtual:unocss"));
        assert!(is_virtual_module("virtual:generated-layouts"));
        assert!(is_virtual_module("virtual:generated-pages"));
        assert!(is_virtual_module("virtual:icons/mdi/home"));
        assert!(is_virtual_module("virtual:windi.css"));
        assert!(is_virtual_module("virtual:windi-devtools"));
        assert!(is_virtual_module("virtual:svg-icons-register"));
        assert!(is_virtual_module("virtual:remix/server-build"));
        assert!(is_virtual_module("virtual:emoji-mart-lang-importer"));
    }

    #[test]
    fn not_virtual_module() {
        assert!(!is_virtual_module("react"));
        assert!(!is_virtual_module("lodash"));
        assert!(!is_virtual_module("@scope/pkg"));
        assert!(!is_virtual_module("node:fs"));
        assert!(!is_virtual_module("cloudflare:workers"));
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

    // ---------------------------------------------------------------
    // is_config_file edge cases
    // ---------------------------------------------------------------

    #[test]
    fn config_file_testing_tool_configs() {
        assert!(is_config_file(std::path::Path::new("jest.config.ts")));
        assert!(is_config_file(std::path::Path::new("jest.config.js")));
        assert!(is_config_file(std::path::Path::new("jest.config.cjs")));
        assert!(is_config_file(std::path::Path::new("jest.setup.ts")));
        assert!(is_config_file(std::path::Path::new("vitest.config.ts")));
        assert!(is_config_file(std::path::Path::new("vitest.config.mts")));
        assert!(is_config_file(std::path::Path::new("vitest.setup.ts")));
        assert!(is_config_file(std::path::Path::new("vitest.workspace.ts")));
        assert!(is_config_file(std::path::Path::new("cypress.config.ts")));
        assert!(is_config_file(std::path::Path::new("playwright.config.ts")));
    }

    #[test]
    fn config_file_bundler_configs() {
        assert!(is_config_file(std::path::Path::new("webpack.config.js")));
        assert!(is_config_file(std::path::Path::new("webpack.config.mjs")));
        assert!(is_config_file(std::path::Path::new("rollup.config.mjs")));
        assert!(is_config_file(std::path::Path::new("rollup.config.js")));
        assert!(is_config_file(std::path::Path::new("tsup.config.ts")));
        assert!(is_config_file(std::path::Path::new("esbuild.config.js")));
        assert!(is_config_file(std::path::Path::new("swc.config.json")));
        assert!(is_config_file(std::path::Path::new("unbuild.config.ts")));
    }

    /// Nested config patterns like `vitest.ci.config.ts` are explicitly listed
    /// in the patterns array and match correctly.
    #[test]
    fn config_file_nested_patterns_listed() {
        assert!(is_config_file(std::path::Path::new("vitest.ci.config.ts")));
    }

    /// Config files with extra qualifiers (e.g., `webpack.prod.config.js`) do NOT
    /// match because `webpack.prod.config.js` does not start with `webpack.config.`.
    /// Only explicitly listed nested patterns (like `vitest.ci.config.`) are recognized.
    #[test]
    fn config_file_unlisted_nested_patterns_do_not_match() {
        assert!(!is_config_file(std::path::Path::new(
            "webpack.prod.config.js"
        )));
        assert!(!is_config_file(std::path::Path::new(
            "webpack.dev.config.js"
        )));
        assert!(!is_config_file(std::path::Path::new("jest.e2e.config.ts")));
        assert!(!is_config_file(std::path::Path::new(
            "rollup.lib.config.mjs"
        )));
    }

    #[test]
    fn config_file_rc_files_with_extensions() {
        assert!(is_config_file(std::path::Path::new(".eslintrc.js")));
        assert!(is_config_file(std::path::Path::new(".eslintrc.cjs")));
        assert!(is_config_file(std::path::Path::new(".eslintrc.json")));
        assert!(is_config_file(std::path::Path::new(".eslintrc.yaml")));
        assert!(is_config_file(std::path::Path::new(".prettierrc.json")));
        assert!(is_config_file(std::path::Path::new(".prettierrc.js")));
        assert!(is_config_file(std::path::Path::new(".prettierrc.cjs")));
        assert!(is_config_file(std::path::Path::new(".babelrc.json")));
        assert!(is_config_file(std::path::Path::new(".secretlintrc.cjs")));
        assert!(is_config_file(std::path::Path::new(".commitlintrc.js")));
    }

    /// Bare RC files without an extension (e.g., `.babelrc`, `.prettierrc`) do NOT
    /// match because the dotrc pattern requires `rc.` (with a dot before the extension).
    #[test]
    fn config_file_bare_rc_files_do_not_match() {
        assert!(!is_config_file(std::path::Path::new(".babelrc")));
        assert!(!is_config_file(std::path::Path::new(".prettierrc")));
        assert!(!is_config_file(std::path::Path::new(".eslintrc")));
    }

    /// Files that look like configs but aren't in the patterns list.
    #[test]
    fn not_config_file_similar_names() {
        assert!(!is_config_file(std::path::Path::new("config.ts")));
        assert!(!is_config_file(std::path::Path::new("my-config.js")));
        assert!(!is_config_file(std::path::Path::new("app.config.ts")));
        assert!(!is_config_file(std::path::Path::new("database.config.js")));
        assert!(!is_config_file(std::path::Path::new("firebase.config.ts")));
    }

    #[test]
    fn config_file_next_js_specific() {
        assert!(is_config_file(std::path::Path::new("next-env.d.ts")));
        assert!(is_config_file(std::path::Path::new("next.config.mjs")));
        assert!(is_config_file(std::path::Path::new("next.config.js")));
        assert!(is_config_file(std::path::Path::new("next.config.ts")));
    }

    #[test]
    fn config_file_environment_declarations() {
        assert!(is_config_file(std::path::Path::new("next-env.d.ts")));
        assert!(is_config_file(std::path::Path::new("env.d.ts")));
        assert!(is_config_file(std::path::Path::new("vite-env.d.ts")));
    }

    /// Dotenv files (`.env`, `.env.local`, `.env.production`) are NOT config files
    /// in this context — they are environment variable files, not JS/TS tool configs.
    #[test]
    fn not_config_file_dotenv_files() {
        assert!(!is_config_file(std::path::Path::new(".env")));
        assert!(!is_config_file(std::path::Path::new(".env.local")));
        assert!(!is_config_file(std::path::Path::new(".env.production")));
        assert!(!is_config_file(std::path::Path::new(".env.development")));
        assert!(!is_config_file(std::path::Path::new(".env.staging")));
    }

    #[test]
    fn config_file_framework_configs() {
        assert!(is_config_file(std::path::Path::new("astro.config.mjs")));
        assert!(is_config_file(std::path::Path::new("nuxt.config.ts")));
        assert!(is_config_file(std::path::Path::new("vite.config.ts")));
        assert!(is_config_file(std::path::Path::new("tailwind.config.js")));
        assert!(is_config_file(std::path::Path::new("tailwind.config.ts")));
        assert!(is_config_file(std::path::Path::new("drizzle.config.ts")));
        assert!(is_config_file(std::path::Path::new("postcss.config.js")));
    }

    #[test]
    fn config_file_sentry_configs() {
        assert!(is_config_file(std::path::Path::new(
            "sentry.client.config.ts"
        )));
        assert!(is_config_file(std::path::Path::new(
            "sentry.server.config.ts"
        )));
        assert!(is_config_file(std::path::Path::new(
            "sentry.edge.config.ts"
        )));
    }

    #[test]
    fn config_file_linting_and_formatting() {
        assert!(is_config_file(std::path::Path::new("eslint.config.mjs")));
        assert!(is_config_file(std::path::Path::new("prettier.config.js")));
        assert!(is_config_file(std::path::Path::new("stylelint.config.js")));
        assert!(is_config_file(std::path::Path::new(
            "lint-staged.config.js"
        )));
        assert!(is_config_file(std::path::Path::new("commitlint.config.js")));
    }

    /// Config file detection only considers the filename, not the directory path.
    #[test]
    fn config_file_ignores_directory_path() {
        assert!(is_config_file(std::path::Path::new(
            "src/config/jest.config.ts"
        )));
        assert!(is_config_file(std::path::Path::new(
            "packages/app/vite.config.ts"
        )));
        assert!(!is_config_file(std::path::Path::new(
            "jest.config/index.ts"
        )));
    }

    // ---------------------------------------------------------------
    // is_path_alias edge cases
    // ---------------------------------------------------------------

    #[test]
    fn path_alias_pascal_case_scopes() {
        assert!(is_path_alias("@Components/Button"));
        assert!(is_path_alias("@Hooks/useApi"));
        assert!(is_path_alias("@Services/auth"));
        assert!(is_path_alias("@Utils/format"));
        assert!(is_path_alias("@Lib/helpers"));
    }

    #[test]
    fn path_alias_hash_imports() {
        // All hash-prefixed imports are treated as path aliases
        // (Node.js package.json "imports" field or custom aliases)
        assert!(is_path_alias("#/utils"));
        assert!(is_path_alias("#subpath"));
        assert!(is_path_alias("#internal/module"));
        assert!(is_path_alias("#lib"));
        assert!(is_path_alias("#components/Button"));
    }

    #[test]
    fn path_alias_tilde_imports() {
        assert!(is_path_alias("~/components"));
        assert!(is_path_alias("~/lib/helpers"));
        assert!(is_path_alias("~/utils/format"));
        assert!(is_path_alias("~/styles/theme"));
    }

    /// Tilde without slash is NOT a path alias — it's a bare specifier.
    #[test]
    fn not_path_alias_bare_tilde() {
        assert!(!is_path_alias("~some-package"));
        assert!(!is_path_alias("~"));
    }

    #[test]
    fn path_alias_at_slash_subpaths() {
        assert!(is_path_alias("@/components"));
        assert!(is_path_alias("@/utils/helpers"));
        assert!(is_path_alias("@/lib/api/client"));
        assert!(is_path_alias("@/styles"));
    }

    #[test]
    fn not_path_alias_regular_npm_packages() {
        assert!(!is_path_alias("lodash"));
        assert!(!is_path_alias("react"));
        assert!(!is_path_alias("express"));
        assert!(!is_path_alias("next"));
        assert!(!is_path_alias("typescript"));
        assert!(!is_path_alias("zod"));
    }

    #[test]
    fn not_path_alias_scoped_npm_packages() {
        assert!(!is_path_alias("@types/node"));
        assert!(!is_path_alias("@types/react"));
        assert!(!is_path_alias("@babel/core"));
        assert!(!is_path_alias("@babel/preset-env"));
        assert!(!is_path_alias("@emotion/react"));
        assert!(!is_path_alias("@emotion/styled"));
        assert!(!is_path_alias("@tanstack/react-query"));
        assert!(!is_path_alias("@testing-library/react"));
        assert!(!is_path_alias("@nestjs/core"));
        assert!(!is_path_alias("@prisma/client"));
    }

    /// The PascalCase heuristic checks the second character (after `@`).
    /// Single-char scope names like `@s/pkg` are lowercase and thus not aliases.
    #[test]
    fn not_path_alias_edge_case_scopes() {
        assert!(!is_path_alias("@s/lowercase"));
        assert!(!is_path_alias("@a/package"));
        assert!(!is_path_alias("@x/something"));
    }

    /// Bare `@` without a slash is not a valid npm scope — but it's also
    /// not detected as a path alias because `@` alone has no uppercase second char.
    #[test]
    fn not_path_alias_bare_at_sign() {
        assert!(!is_path_alias("@"));
    }

    // ---------------------------------------------------------------
    // Angular lifecycle methods — exhaustive and negative edge cases
    // ---------------------------------------------------------------

    /// Verify every Angular lifecycle hook is recognized.
    #[test]
    fn angular_lifecycle_all_hooks_exhaustive() {
        let all_hooks = [
            "ngOnInit",
            "ngOnDestroy",
            "ngOnChanges",
            "ngDoCheck",
            "ngAfterContentInit",
            "ngAfterContentChecked",
            "ngAfterViewInit",
            "ngAfterViewChecked",
            "ngAcceptInputType",
        ];
        for hook in &all_hooks {
            assert!(
                is_angular_lifecycle_method(hook),
                "{hook} should be recognized as Angular lifecycle"
            );
        }
    }

    /// Verify every Angular guard/resolver/interceptor method is recognized.
    #[test]
    fn angular_lifecycle_all_guards_exhaustive() {
        let all_guards = [
            "canActivate",
            "canDeactivate",
            "canActivateChild",
            "canMatch",
            "resolve",
            "intercept",
            "transform",
        ];
        for guard in &all_guards {
            assert!(
                is_angular_lifecycle_method(guard),
                "{guard} should be recognized as Angular lifecycle"
            );
        }
    }

    /// Verify every Angular form method is recognized.
    #[test]
    fn angular_lifecycle_all_form_methods_exhaustive() {
        let all_form = [
            "validate",
            "registerOnChange",
            "registerOnTouched",
            "writeValue",
            "setDisabledState",
        ];
        for method in &all_form {
            assert!(
                is_angular_lifecycle_method(method),
                "{method} should be recognized as Angular lifecycle"
            );
        }
    }

    /// Methods that look similar to Angular lifecycle hooks but are NOT.
    #[test]
    fn not_angular_lifecycle_similar_names() {
        assert!(!is_angular_lifecycle_method("ngOnInit2"));
        assert!(!is_angular_lifecycle_method("onInit"));
        assert!(!is_angular_lifecycle_method("ngInit"));
        assert!(!is_angular_lifecycle_method("onDestroy"));
        assert!(!is_angular_lifecycle_method("afterViewInit"));
        assert!(!is_angular_lifecycle_method("doCheck"));
        assert!(!is_angular_lifecycle_method("ngOnInitialize"));
        assert!(!is_angular_lifecycle_method("ngonInit")); // wrong case
    }

    /// Angular methods should be case-sensitive.
    #[test]
    fn angular_lifecycle_case_sensitivity() {
        assert!(!is_angular_lifecycle_method("ngoninit"));
        assert!(!is_angular_lifecycle_method("NGONINIT"));
        assert!(!is_angular_lifecycle_method("NgOnInit"));
        assert!(!is_angular_lifecycle_method("canactivate"));
        assert!(!is_angular_lifecycle_method("CANACTIVATE"));
    }

    // ---------------------------------------------------------------
    // React lifecycle methods — exhaustive and negative edge cases
    // ---------------------------------------------------------------

    /// Verify every React lifecycle method is recognized (complete list).
    #[test]
    fn react_lifecycle_all_methods_exhaustive() {
        let all_methods = [
            "render",
            "componentDidMount",
            "componentDidUpdate",
            "componentWillUnmount",
            "shouldComponentUpdate",
            "getSnapshotBeforeUpdate",
            "getDerivedStateFromProps",
            "getDerivedStateFromError",
            "componentDidCatch",
            "componentWillMount",
            "componentWillReceiveProps",
            "componentWillUpdate",
            "UNSAFE_componentWillMount",
            "UNSAFE_componentWillReceiveProps",
            "UNSAFE_componentWillUpdate",
            "getChildContext",
            "contextType",
        ];
        for method in &all_methods {
            assert!(
                is_react_lifecycle_method(method),
                "{method} should be recognized as React lifecycle"
            );
        }
    }

    /// Methods that look similar to React lifecycle methods but are NOT.
    #[test]
    fn not_react_lifecycle_similar_names() {
        assert!(!is_react_lifecycle_method("componentDidMounted"));
        assert!(!is_react_lifecycle_method("onComponentDidMount"));
        assert!(!is_react_lifecycle_method("didMount"));
        assert!(!is_react_lifecycle_method("willUnmount"));
        assert!(!is_react_lifecycle_method("shouldUpdate"));
        assert!(!is_react_lifecycle_method("getDerivedState"));
        assert!(!is_react_lifecycle_method("UNSAFE_render"));
        assert!(!is_react_lifecycle_method("unsafe_componentWillMount"));
    }

    /// React lifecycle methods should be case-sensitive.
    #[test]
    fn react_lifecycle_case_sensitivity() {
        assert!(!is_react_lifecycle_method("Render"));
        assert!(!is_react_lifecycle_method("RENDER"));
        assert!(!is_react_lifecycle_method("componentdidmount"));
        assert!(!is_react_lifecycle_method("COMPONENTDIDMOUNT"));
        assert!(!is_react_lifecycle_method("ComponentDidMount"));
    }

    /// Common class methods that should never match lifecycle detection.
    #[test]
    fn not_lifecycle_common_class_methods() {
        let common_methods = [
            "constructor",
            "setState",
            "forceUpdate",
            "handleClick",
            "handleSubmit",
            "fetchData",
            "toString",
            "valueOf",
            "toJSON",
            "init",
            "destroy",
            "update",
            "mount",
            "unmount",
        ];
        for method in &common_methods {
            assert!(
                !is_react_lifecycle_method(method),
                "{method} should NOT be a React lifecycle method"
            );
            assert!(
                !is_angular_lifecycle_method(method),
                "{method} should NOT be an Angular lifecycle method"
            );
        }
    }

    // ---------------------------------------------------------------
    // Builtin module edge cases
    // ---------------------------------------------------------------

    /// Subpath imports of builtins should be recognized.
    #[test]
    fn builtin_module_subpath_imports() {
        assert!(is_builtin_module("assert/strict"));
        assert!(is_builtin_module("dns/promises"));
        assert!(is_builtin_module("fs/promises"));
        assert!(is_builtin_module("path/posix"));
        assert!(is_builtin_module("path/win32"));
        assert!(is_builtin_module("readline/promises"));
        assert!(is_builtin_module("stream/consumers"));
        assert!(is_builtin_module("stream/promises"));
        assert!(is_builtin_module("stream/web"));
        assert!(is_builtin_module("timers/promises"));
        assert!(is_builtin_module("util/types"));
        assert!(is_builtin_module("inspector/promises"));
        assert!(is_builtin_module("test/reporters"));
    }

    /// Subpath builtins with `node:` prefix.
    #[test]
    fn builtin_module_subpath_with_node_prefix() {
        assert!(is_builtin_module("node:fs/promises"));
        assert!(is_builtin_module("node:path/posix"));
        assert!(is_builtin_module("node:stream/web"));
        assert!(is_builtin_module("node:timers/promises"));
        assert!(is_builtin_module("node:util/types"));
        assert!(is_builtin_module("node:test/reporters"));
    }

    /// Cloudflare Workers built-in modules.
    #[test]
    fn builtin_module_cloudflare_workers() {
        assert!(is_builtin_module("cloudflare:workers"));
        assert!(is_builtin_module("cloudflare:sockets"));
        assert!(is_builtin_module("cloudflare:email"));
    }

    /// Deno standard library.
    #[test]
    fn builtin_module_deno_std() {
        assert!(is_builtin_module("std"));
        assert!(is_builtin_module("std/path"));
        assert!(is_builtin_module("std/fs"));
    }

    /// Non-existent subpath builtins should not match.
    #[test]
    fn not_builtin_module_fake_subpaths() {
        assert!(!is_builtin_module("fs/extra"));
        assert!(!is_builtin_module("path/utils"));
        assert!(!is_builtin_module("stream/transform"));
    }

    // ---------------------------------------------------------------
    // is_virtual_module edge cases
    // ---------------------------------------------------------------

    /// Empty string and prefix-only edge cases.
    #[test]
    fn virtual_module_edge_cases() {
        assert!(is_virtual_module("virtual:"));
        assert!(!is_virtual_module(""));
        assert!(!is_virtual_module("Virtual:something"));
        assert!(!is_virtual_module("VIRTUAL:something"));
    }

    // ---------------------------------------------------------------
    // is_implicit_dependency edge cases
    // ---------------------------------------------------------------

    #[test]
    fn implicit_dep_react_dom_and_native() {
        assert!(is_implicit_dependency("react-dom"));
        assert!(is_implicit_dependency("react-dom/client"));
        assert!(is_implicit_dependency("react-native"));
    }

    #[test]
    fn implicit_dep_next_packages() {
        assert!(is_implicit_dependency("@next/font"));
        assert!(is_implicit_dependency("@next/mdx"));
        assert!(is_implicit_dependency("@next/bundle-analyzer"));
        assert!(is_implicit_dependency("@next/env"));
    }

    #[test]
    fn implicit_dep_websocket_native_addons() {
        assert!(is_implicit_dependency("utf-8-validate"));
        assert!(is_implicit_dependency("bufferutil"));
    }

    /// Packages that look similar to implicit deps but are NOT.
    #[test]
    fn not_implicit_dep_similar_names() {
        assert!(!is_implicit_dependency("react"));
        assert!(!is_implicit_dependency("react-dom-extra"));
        assert!(!is_implicit_dependency("@next/swc"));
        assert!(!is_implicit_dependency("react-native-web"));
        assert!(!is_implicit_dependency("@types"));
    }

    // ---------------------------------------------------------------
    // is_declaration_file edge cases
    // ---------------------------------------------------------------

    /// Declaration files in deeply nested paths.
    #[test]
    fn declaration_file_nested_paths() {
        assert!(is_declaration_file(std::path::Path::new(
            "packages/ui/src/types/global.d.ts"
        )));
        assert!(is_declaration_file(std::path::Path::new(
            "node_modules/@types/react/index.d.ts"
        )));
    }

    /// Files ending with `.d.` but not valid declaration extensions.
    #[test]
    fn not_declaration_file_invalid_d_extensions() {
        assert!(!is_declaration_file(std::path::Path::new("file.d.js")));
        assert!(!is_declaration_file(std::path::Path::new("file.d.jsx")));
        assert!(!is_declaration_file(std::path::Path::new("file.d.css")));
        assert!(!is_declaration_file(std::path::Path::new("file.d.json")));
    }

    /// Files with `.d.ts` in the middle of the name (not at the end).
    #[test]
    fn not_declaration_file_d_ts_in_middle() {
        assert!(!is_declaration_file(std::path::Path::new("my.d.ts.backup")));
    }

    // ---------------------------------------------------------------
    // is_barrel_with_reachable_sources tests
    // ---------------------------------------------------------------

    use crate::discover::{DiscoveredFile, EntryPoint, EntryPointSource, FileId};
    use crate::graph::{ExportSymbol, ModuleGraph, ReExportEdge};
    use crate::resolve::ResolvedModule;

    fn build_graph(file_specs: &[(&str, bool)]) -> ModuleGraph {
        let files: Vec<DiscoveredFile> = file_specs
            .iter()
            .enumerate()
            .map(|(i, (path, _))| DiscoveredFile {
                id: FileId(i as u32),
                path: std::path::PathBuf::from(path),
                size_bytes: 0,
            })
            .collect();

        let entry_points: Vec<EntryPoint> = file_specs
            .iter()
            .filter(|(_, is_entry)| *is_entry)
            .map(|(path, _)| EntryPoint {
                path: std::path::PathBuf::from(path),
                source: EntryPointSource::ManualEntry,
            })
            .collect();

        let resolved_modules: Vec<ResolvedModule> = files
            .iter()
            .map(|f| ResolvedModule {
                file_id: f.id,
                path: f.path.clone(),
                exports: vec![],
                re_exports: vec![],
                resolved_imports: vec![],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
                unused_import_bindings: rustc_hash::FxHashSet::default(),
            })
            .collect();

        ModuleGraph::build(&resolved_modules, &entry_points, &files)
    }

    /// Module with no re-exports is not a barrel.
    #[test]
    fn barrel_no_re_exports_returns_false() {
        let graph = build_graph(&[("/src/entry.ts", true), ("/src/utils.ts", false)]);
        let module = &graph.modules[1];
        assert!(!is_barrel_with_reachable_sources(module, &graph));
    }

    /// Module with re-exports but also local exports is not a pure barrel.
    #[test]
    fn barrel_with_local_exports_returns_false() {
        let mut graph = build_graph(&[
            ("/src/entry.ts", true),
            ("/src/index.ts", false),
            ("/src/utils.ts", false),
        ]);
        graph.modules[2].is_reachable = true;
        // Add a re-export
        graph.modules[1].re_exports = vec![ReExportEdge {
            source_file: FileId(2),
            imported_name: "helper".to_string(),
            exported_name: "helper".to_string(),
            is_type_only: false,
        }];
        // Add a local export with a real span (non-zero)
        graph.modules[1].exports = vec![ExportSymbol {
            name: crate::extract::ExportName::Named("localFn".to_string()),
            is_type_only: false,
            is_public: false,
            span: oxc_span::Span::new(10, 50),
            references: vec![],
            members: vec![],
        }];
        assert!(!is_barrel_with_reachable_sources(&graph.modules[1], &graph));
    }

    /// Module with re-exports and CJS exports is not a pure barrel.
    #[test]
    fn barrel_with_cjs_exports_returns_false() {
        let mut graph = build_graph(&[
            ("/src/entry.ts", true),
            ("/src/index.ts", false),
            ("/src/utils.ts", false),
        ]);
        graph.modules[2].is_reachable = true;
        graph.modules[1].re_exports = vec![ReExportEdge {
            source_file: FileId(2),
            imported_name: "helper".to_string(),
            exported_name: "helper".to_string(),
            is_type_only: false,
        }];
        graph.modules[1].has_cjs_exports = true;
        assert!(!is_barrel_with_reachable_sources(&graph.modules[1], &graph));
    }

    /// Pure barrel with reachable source returns true.
    #[test]
    fn barrel_pure_with_reachable_source_returns_true() {
        let mut graph = build_graph(&[
            ("/src/entry.ts", true),
            ("/src/index.ts", false),
            ("/src/utils.ts", false),
        ]);
        graph.modules[2].is_reachable = true;
        // Only re-exports, no local exports, no CJS
        graph.modules[1].re_exports = vec![ReExportEdge {
            source_file: FileId(2),
            imported_name: "helper".to_string(),
            exported_name: "helper".to_string(),
            is_type_only: false,
        }];
        // Only synthetic exports (span 0..0), which are from re-exports
        graph.modules[1].exports = vec![ExportSymbol {
            name: crate::extract::ExportName::Named("helper".to_string()),
            is_type_only: false,
            is_public: false,
            span: oxc_span::Span::new(0, 0),
            references: vec![],
            members: vec![],
        }];
        assert!(is_barrel_with_reachable_sources(&graph.modules[1], &graph));
    }

    /// Pure barrel where all sources are unreachable returns false.
    #[test]
    fn barrel_all_sources_unreachable_returns_false() {
        let mut graph = build_graph(&[
            ("/src/entry.ts", true),
            ("/src/index.ts", false),
            ("/src/utils.ts", false),
        ]);
        // utils (source) is NOT reachable
        graph.modules[1].re_exports = vec![ReExportEdge {
            source_file: FileId(2),
            imported_name: "helper".to_string(),
            exported_name: "helper".to_string(),
            is_type_only: false,
        }];
        assert!(!is_barrel_with_reachable_sources(&graph.modules[1], &graph));
    }

    /// Barrel with out-of-bounds source FileId doesn't panic, returns false.
    #[test]
    fn barrel_out_of_bounds_source_returns_false() {
        let mut graph = build_graph(&[("/src/entry.ts", true), ("/src/index.ts", false)]);
        graph.modules[1].re_exports = vec![ReExportEdge {
            source_file: FileId(999), // out of bounds
            imported_name: "helper".to_string(),
            exported_name: "helper".to_string(),
            is_type_only: false,
        }];
        // Should not panic, should return false
        assert!(!is_barrel_with_reachable_sources(&graph.modules[1], &graph));
    }
}
