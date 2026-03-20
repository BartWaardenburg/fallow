//! Astro framework plugin.
//!
//! Detects Astro projects and marks pages, layouts, content, and middleware
//! as entry points. Parses astro.config to extract referenced dependencies.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct AstroPlugin;

const ENABLERS: &[&str] = &["astro"];

const ENTRY_PATTERNS: &[&str] = &[
    "src/pages/**/*.{astro,ts,tsx,js,jsx,md,mdx}",
    "src/layouts/**/*.astro",
    "src/content/**/*.{ts,js,md,mdx}",
    "src/middleware.{js,ts}",
];

const CONFIG_PATTERNS: &[&str] = &["astro.config.{ts,js,mjs}"];

const ALWAYS_USED: &[&str] = &["astro.config.{ts,js,mjs}"];

const TOOLING_DEPENDENCIES: &[&str] = &["astro", "@astrojs/check", "@astrojs/ts-plugin"];

/// Virtual module prefixes provided by Astro at build time.
/// `astro:` provides built-in modules (content, transitions, env, actions, assets,
/// i18n, middleware, container, schema).
const VIRTUAL_MODULE_PREFIXES: &[&str] = &["astro:"];

impl Plugin for AstroPlugin {
    fn name(&self) -> &'static str {
        "astro"
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

    fn resolve_config(&self, config_path: &Path, source: &str, _root: &Path) -> PluginResult {
        let mut result = PluginResult::default();

        let imports = config_parser::extract_imports(source, config_path);
        for imp in &imports {
            let dep = crate::resolve::extract_package_name(imp);
            result.referenced_dependencies.push(dep);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_module_prefixes_includes_astro_builtins() {
        let plugin = AstroPlugin;
        let prefixes = plugin.virtual_module_prefixes();
        assert!(prefixes.contains(&"astro:"));
    }
}
