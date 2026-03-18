//! PostCSS plugin.
//!
//! Detects PostCSS projects and marks config files as always used.
//! Parses config to extract plugin dependencies from object keys, require() calls,
//! and string array forms.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct PostCssPlugin;

const ENABLERS: &[&str] = &["postcss"];

const CONFIG_PATTERNS: &[&str] = &["postcss.config.{ts,js,cjs,mjs}"];

const ALWAYS_USED: &[&str] = &["postcss.config.{ts,js,cjs,mjs}"];

const TOOLING_DEPENDENCIES: &[&str] = &["postcss", "postcss-cli"];

impl Plugin for PostCssPlugin {
    fn name(&self) -> &'static str {
        "postcss"
    }

    fn enablers(&self) -> &'static [&'static str] {
        ENABLERS
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

    fn resolve_config(&self, config_path: &Path, source: &str, _root: &Path) -> PluginResult {
        let mut result = PluginResult::default();

        let imports = config_parser::extract_imports(source, config_path);
        for imp in &imports {
            let dep = crate::resolve::extract_package_name(imp);
            result.referenced_dependencies.push(dep);
        }

        // plugins as object keys: { plugins: { autoprefixer: {}, tailwindcss: {} } }
        let plugin_keys =
            config_parser::extract_config_object_keys(source, config_path, &["plugins"]);
        for key in &plugin_keys {
            result
                .referenced_dependencies
                .push(crate::resolve::extract_package_name(key));
        }

        // plugins as require() calls: { plugins: [require('autoprefixer')] }
        let require_deps =
            config_parser::extract_config_require_strings(source, config_path, "plugins");
        for dep in &require_deps {
            result
                .referenced_dependencies
                .push(crate::resolve::extract_package_name(dep));
        }

        // plugins as string array: { plugins: ["autoprefixer", ["postcss-preset-env", {}]] }
        let plugin_strings =
            config_parser::extract_config_shallow_strings(source, config_path, "plugins");
        for plugin in &plugin_strings {
            result
                .referenced_dependencies
                .push(crate::resolve::extract_package_name(plugin));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_config_string_array_plugins() {
        let source = r#"
            module.exports = {
                plugins: ["autoprefixer", ["postcss-preset-env", { stage: 3 }]]
            };
        "#;
        let plugin = PostCssPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("postcss.config.js"),
            source,
            std::path::Path::new("/project"),
        );
        let deps = &result.referenced_dependencies;
        assert!(deps.contains(&"autoprefixer".to_string()));
        assert!(deps.contains(&"postcss-preset-env".to_string()));
    }

    #[test]
    fn resolve_config_object_and_require() {
        let source = r#"
            module.exports = {
                plugins: {
                    autoprefixer: {},
                    tailwindcss: {}
                }
            };
        "#;
        let plugin = PostCssPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("postcss.config.js"),
            source,
            std::path::Path::new("/project"),
        );
        let deps = &result.referenced_dependencies;
        assert!(deps.contains(&"autoprefixer".to_string()));
        assert!(deps.contains(&"tailwindcss".to_string()));
    }
}
