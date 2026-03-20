//! Rsbuild bundler plugin.
//!
//! Detects Rsbuild projects and marks config files as always used.
//! Parses rsbuild config to extract entry points, plugin dependencies,
//! and import references.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct RsbuildPlugin;

const ENABLERS: &[&str] = &["@rsbuild/core"];

const CONFIG_PATTERNS: &[&str] = &["rsbuild.config.{ts,js,mjs,cjs}"];

const ALWAYS_USED: &[&str] = &["rsbuild.config.{ts,js,mjs,cjs}"];

const TOOLING_DEPENDENCIES: &[&str] = &["@rsbuild/core"];

impl Plugin for RsbuildPlugin {
    fn name(&self) -> &'static str {
        "rsbuild"
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

        // Extract import sources as referenced dependencies
        let imports = config_parser::extract_imports(source, config_path);
        for imp in &imports {
            let dep = crate::resolve::extract_package_name(imp);
            result.referenced_dependencies.push(dep);
        }

        // source.entry → entry points (string or object with string values)
        let entries = config_parser::extract_config_string_or_array(
            source,
            config_path,
            &["source", "entry"],
        );
        result.entry_patterns.extend(entries);

        // plugins → extract plugin package names from imports
        // Rsbuild plugins are typically imported and passed to the plugins array,
        // so the import extraction above already covers them. Additionally extract
        // any string references or require() calls in the plugins array.
        let plugin_requires =
            config_parser::extract_config_require_strings(source, config_path, "plugins");
        for dep in &plugin_requires {
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
    fn resolve_config_entry_string() {
        let source = r#"
            export default {
                source: {
                    entry: "./src/index.tsx"
                }
            };
        "#;
        let plugin = RsbuildPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("rsbuild.config.ts"),
            source,
            std::path::Path::new("/project"),
        );
        assert_eq!(result.entry_patterns, vec!["./src/index.tsx"]);
    }

    #[test]
    fn resolve_config_entry_object() {
        let source = r#"
            export default {
                source: {
                    entry: {
                        main: "./src/main.tsx",
                        admin: "./src/admin.tsx"
                    }
                }
            };
        "#;
        let plugin = RsbuildPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("rsbuild.config.ts"),
            source,
            std::path::Path::new("/project"),
        );
        assert!(
            result
                .entry_patterns
                .contains(&"./src/main.tsx".to_string())
        );
        assert!(
            result
                .entry_patterns
                .contains(&"./src/admin.tsx".to_string())
        );
    }

    #[test]
    fn resolve_config_imports() {
        let source = r#"
            import { defineConfig } from '@rsbuild/core';
            import { pluginReact } from '@rsbuild/plugin-react';
            export default defineConfig({
                plugins: [pluginReact()],
                source: {
                    entry: "./src/index.tsx"
                }
            });
        "#;
        let plugin = RsbuildPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("rsbuild.config.ts"),
            source,
            std::path::Path::new("/project"),
        );
        let deps = &result.referenced_dependencies;
        assert!(deps.contains(&"@rsbuild/core".to_string()));
        assert!(deps.contains(&"@rsbuild/plugin-react".to_string()));
        assert_eq!(result.entry_patterns, vec!["./src/index.tsx"]);
    }

    #[test]
    fn resolve_config_define_config() {
        let source = r#"
            import { defineConfig } from '@rsbuild/core';
            export default defineConfig({
                source: {
                    entry: {
                        index: "./src/index.ts"
                    }
                }
            });
        "#;
        let plugin = RsbuildPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("rsbuild.config.ts"),
            source,
            std::path::Path::new("/project"),
        );
        assert_eq!(result.entry_patterns, vec!["./src/index.ts"]);
    }
}
