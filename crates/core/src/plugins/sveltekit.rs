//! `SvelteKit` framework plugin.
//!
//! Detects `SvelteKit` projects and marks route files, hooks, and convention files
//! as entry points. Parses svelte.config.js to extract adapter dependencies.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct SvelteKitPlugin;

const ENABLERS: &[&str] = &["@sveltejs/kit"];

const ENTRY_PATTERNS: &[&str] = &[
    // Route files (split svelte/ts for correct used_exports matching)
    "src/routes/**/+page.svelte",
    "src/routes/**/+page.{ts,js}",
    "src/routes/**/+page.server.{ts,js}",
    "src/routes/**/+layout.svelte",
    "src/routes/**/+layout.{ts,js}",
    "src/routes/**/+layout.server.{ts,js}",
    "src/routes/**/+server.{ts,js}",
    "src/routes/**/+error.svelte",
    // Hooks
    "src/hooks.server.{ts,js}",
    "src/hooks.client.{ts,js}",
    "src/hooks.{ts,js}",
    // Service worker
    "src/service-worker.{ts,js}",
    // Params matchers
    "src/params/**/*.{ts,js}",
];

const CONFIG_PATTERNS: &[&str] = &["svelte.config.{js,cjs,mjs,ts}"];

const ALWAYS_USED: &[&str] = &[
    "svelte.config.{js,cjs,mjs,ts}",
    "src/app.html",
    "src/app.d.ts",
    "src/app.{css,scss,less}",
];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "svelte",
    "@sveltejs/kit",
    "@sveltejs/adapter-auto",
    "@sveltejs/adapter-node",
    "@sveltejs/adapter-static",
    "@sveltejs/adapter-vercel",
    "@sveltejs/adapter-netlify",
    "@sveltejs/adapter-cloudflare",
    "@sveltejs/vite-plugin-svelte",
    "svelte-check",
    "svelte-preprocess",
];

/// Virtual module prefixes provided by SvelteKit at build time.
/// `$app/` provides runtime modules (environment, forms, navigation, paths, server, state).
/// `$env/` provides environment variable access (static/dynamic, public/private).
/// `$lib/` is an alias for the `src/lib` directory.
/// `$service-worker` provides service worker build info.
const VIRTUAL_MODULE_PREFIXES: &[&str] = &["$app/", "$env/", "$lib/", "$service-worker"];

// SvelteKit route convention exports
const PAGE_EXPORTS: &[&str] = &["default"];
const PAGE_LOAD_EXPORTS: &[&str] = &[
    "load",
    "prerender",
    "csr",
    "ssr",
    "trailingSlash",
    "entries",
];
const PAGE_SERVER_EXPORTS: &[&str] = &[
    "load",
    "prerender",
    "csr",
    "ssr",
    "trailingSlash",
    "entries",
    "actions",
];
const LAYOUT_EXPORTS: &[&str] = &["default"];
const LAYOUT_LOAD_EXPORTS: &[&str] = &["load", "prerender", "csr", "ssr", "trailingSlash"];
const SERVER_EXPORTS: &[&str] = &[
    "GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS", "fallback",
];
const HOOKS_SERVER_EXPORTS: &[&str] = &["handle", "handleError", "handleFetch", "init"];
const HOOKS_CLIENT_EXPORTS: &[&str] = &["handleError", "init"];
const HOOKS_SHARED_EXPORTS: &[&str] = &["reroute", "transport", "handleError", "init"];

impl Plugin for SvelteKitPlugin {
    fn name(&self) -> &'static str {
        "sveltekit"
    }

    fn enablers(&self) -> &'static [&'static str] {
        ENABLERS
    }

    fn entry_patterns(&self) -> &'static [&'static str] {
        ENTRY_PATTERNS
    }

    fn config_patterns(&self) -> &'static [&'static str] {
        CONFIG_PATTERNS
    }

    fn always_used(&self) -> &'static [&'static str] {
        ALWAYS_USED
    }

    fn tooling_dependencies(&self) -> &'static [&'static str] {
        TOOLING_DEPENDENCIES
    }

    fn virtual_module_prefixes(&self) -> &'static [&'static str] {
        VIRTUAL_MODULE_PREFIXES
    }

    fn used_exports(&self) -> Vec<(&'static str, &'static [&'static str])> {
        vec![
            ("src/routes/**/+page.svelte", PAGE_EXPORTS),
            ("src/routes/**/+page.{ts,js}", PAGE_LOAD_EXPORTS),
            ("src/routes/**/+page.server.{ts,js}", PAGE_SERVER_EXPORTS),
            ("src/routes/**/+layout.svelte", LAYOUT_EXPORTS),
            ("src/routes/**/+layout.{ts,js}", LAYOUT_LOAD_EXPORTS),
            ("src/routes/**/+layout.server.{ts,js}", LAYOUT_LOAD_EXPORTS),
            ("src/routes/**/+server.{ts,js}", SERVER_EXPORTS),
            ("src/hooks.server.{ts,js}", HOOKS_SERVER_EXPORTS),
            ("src/hooks.client.{ts,js}", HOOKS_CLIENT_EXPORTS),
            ("src/hooks.{ts,js}", HOOKS_SHARED_EXPORTS),
        ]
    }

    fn resolve_config(&self, config_path: &Path, source: &str, _root: &Path) -> PluginResult {
        let mut result = PluginResult::default();

        // Extract import sources as referenced dependencies
        let imports = config_parser::extract_imports(source, config_path);
        for imp in &imports {
            let dep = crate::resolve::extract_package_name(imp);
            result.referenced_dependencies.push(dep);
        }

        // Extract require() calls (CJS configs)
        let require_deps =
            config_parser::extract_config_require_strings(source, config_path, "adapter");
        for dep in &require_deps {
            result
                .referenced_dependencies
                .push(crate::resolve::extract_package_name(dep));
        }

        // Extract preprocess plugins
        let preprocess_deps =
            config_parser::extract_config_require_strings(source, config_path, "preprocess");
        for dep in &preprocess_deps {
            result
                .referenced_dependencies
                .push(crate::resolve::extract_package_name(dep));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_config_adapter_import() {
        let source = r#"
            import adapter from '@sveltejs/adapter-node';
            export default { kit: { adapter: adapter() } };
        "#;
        let plugin = SvelteKitPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("svelte.config.js"),
            source,
            std::path::Path::new("/project"),
        );
        assert!(
            result
                .referenced_dependencies
                .contains(&"@sveltejs/adapter-node".to_string())
        );
    }

    #[test]
    fn resolve_config_preprocess_import() {
        let source = r#"
            import adapter from '@sveltejs/adapter-auto';
            import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
            export default { preprocess: vitePreprocess(), kit: { adapter: adapter() } };
        "#;
        let plugin = SvelteKitPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("svelte.config.js"),
            source,
            std::path::Path::new("/project"),
        );
        assert!(
            result
                .referenced_dependencies
                .contains(&"@sveltejs/adapter-auto".to_string())
        );
        assert!(
            result
                .referenced_dependencies
                .contains(&"@sveltejs/vite-plugin-svelte".to_string())
        );
    }

    #[test]
    fn virtual_module_prefixes_includes_sveltekit_builtins() {
        let plugin = SvelteKitPlugin;
        let prefixes = plugin.virtual_module_prefixes();
        assert!(prefixes.contains(&"$app/"));
        assert!(prefixes.contains(&"$env/"));
        assert!(prefixes.contains(&"$lib/"));
        assert!(prefixes.contains(&"$service-worker"));
    }
}
