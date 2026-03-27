//! Rspack bundler plugin.
//!
//! Detects Rspack projects and marks config files as entry points.
//! Parses rspack.config to extract entry points, loader dependencies,
//! and plugin packages -- using the same webpack-compatible config format.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct RspackPlugin;

const ENABLERS: &[&str] = &["@rspack/core", "@rspack/cli"];

const ENTRY_PATTERNS: &[&str] = &["src/index.{ts,tsx,js,jsx}"];

const CONFIG_PATTERNS: &[&str] = &[
    "rspack.config.{ts,js,mjs,cjs}",
    "rspack.*.config.{ts,js,mjs,cjs}",
];

const ALWAYS_USED: &[&str] = &[
    "rspack.config.{ts,js,mjs,cjs}",
    "rspack.*.config.{ts,js,mjs,cjs}",
];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "@rspack/core",
    "@rspack/cli",
    "@rspack/dev-server",
    "@rspack/plugin-react-refresh",
    "@rspack/plugin-minify",
    "@rspack/plugin-html",
];

impl Plugin for RspackPlugin {
    fn name(&self) -> &'static str {
        "rspack"
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

    fn resolve_config(&self, config_path: &Path, source: &str, _root: &Path) -> PluginResult {
        let mut result = PluginResult::default();

        // Extract import sources as referenced dependencies
        let imports = config_parser::extract_imports(source, config_path);
        for imp in &imports {
            let dep = crate::resolve::extract_package_name(imp);
            result.referenced_dependencies.push(dep);
        }

        // entry -> entry points (string, array, or object with string values)
        let entries =
            config_parser::extract_config_string_or_array(source, config_path, &["entry"]);
        result.entry_patterns.extend(entries);

        // require() calls for loaders/plugins in CJS configs
        let require_deps =
            config_parser::extract_config_require_strings(source, config_path, "plugins");
        for dep in &require_deps {
            result
                .referenced_dependencies
                .push(crate::resolve::extract_package_name(dep));
        }

        // externals -> referenced dependencies (string array form)
        let externals =
            config_parser::extract_config_shallow_strings(source, config_path, "externals");
        for ext in &externals {
            result
                .referenced_dependencies
                .push(crate::resolve::extract_package_name(ext));
        }

        // module.rules -> extract loader package names (reuse webpack's loader parsing)
        super::webpack::parse_webpack_loaders(source, config_path, &mut result);

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_config_entry_string() {
        let source = r#"module.exports = { entry: "./src/app.tsx" };"#;
        let plugin = RspackPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("rspack.config.js"),
            source,
            std::path::Path::new("/project"),
        );
        assert_eq!(result.entry_patterns, vec!["./src/app.tsx"]);
    }

    #[test]
    fn resolve_config_imports() {
        let source = r#"
            import { defineConfig } from '@rspack/cli';
            export default defineConfig({ entry: "./src/main.ts" });
        "#;
        let plugin = RspackPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("rspack.config.ts"),
            source,
            std::path::Path::new("/project"),
        );
        assert!(
            result
                .referenced_dependencies
                .contains(&"@rspack/cli".to_string())
        );
        assert_eq!(result.entry_patterns, vec!["./src/main.ts"]);
    }

    #[test]
    fn resolve_config_loaders() {
        let source = r"
            module.exports = {
                module: {
                    rules: [
                        { test: /\.css$/, use: ['style-loader', 'css-loader'] },
                        { test: /\.svg$/, loader: 'svgr-loader' },
                    ]
                }
            };
        ";
        let plugin = RspackPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("rspack.config.js"),
            source,
            std::path::Path::new("/project"),
        );
        let deps = &result.referenced_dependencies;
        assert!(deps.contains(&"style-loader".to_string()));
        assert!(deps.contains(&"css-loader".to_string()));
        assert!(deps.contains(&"svgr-loader".to_string()));
    }
}
