//! Gatsby framework plugin.
//!
//! Detects Gatsby projects and marks pages, templates, and config files
//! as entry points. Parses gatsby-config to extract plugin dependencies.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct GatsbyPlugin;

const ENABLERS: &[&str] = &["gatsby"];

const ENTRY_PATTERNS: &[&str] = &[
    // Filesystem routing
    "src/pages/**/*.{ts,tsx,js,jsx}",
    // Templates (used by createPage in gatsby-node)
    "src/templates/**/*.{ts,tsx,js,jsx}",
    // API routes (Gatsby 4+)
    "src/api/**/*.{ts,js}",
];

const CONFIG_PATTERNS: &[&str] = &[
    "gatsby-config.{ts,js,mjs}",
    "gatsby-node.{ts,js,mjs}",
    "gatsby-browser.{ts,tsx,js,jsx}",
    "gatsby-ssr.{ts,tsx,js,jsx}",
];

const ALWAYS_USED: &[&str] = &[
    "gatsby-config.{ts,js,mjs}",
    "gatsby-node.{ts,js,mjs}",
    "gatsby-browser.{ts,tsx,js,jsx}",
    "gatsby-ssr.{ts,tsx,js,jsx}",
];

const TOOLING_DEPENDENCIES: &[&str] = &["gatsby", "gatsby-cli"];

// Gatsby page exports
const PAGE_EXPORTS: &[&str] = &["default", "Head", "query", "getServerData"];

impl Plugin for GatsbyPlugin {
    fn name(&self) -> &'static str {
        "gatsby"
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

    fn used_exports(&self) -> Vec<(&'static str, &'static [&'static str])> {
        vec![
            ("src/pages/**/*.{ts,tsx,js,jsx}", PAGE_EXPORTS),
            ("src/templates/**/*.{ts,tsx,js,jsx}", PAGE_EXPORTS),
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

        // Extract plugins array -- plugins can be strings or { resolve: "plugin-name" } objects
        // Simple string plugins
        let plugins = config_parser::extract_config_shallow_strings(source, config_path, "plugins");
        for plugin in &plugins {
            let dep = crate::resolve::extract_package_name(plugin);
            result.referenced_dependencies.push(dep);
        }

        // require() calls in plugins array
        let require_deps =
            config_parser::extract_config_require_strings(source, config_path, "plugins");
        for dep in &require_deps {
            result
                .referenced_dependencies
                .push(crate::resolve::extract_package_name(dep));
        }

        // Extract "resolve" property values from plugin objects
        // e.g., plugins: [{ resolve: "gatsby-plugin-image", options: {} }]
        extract_gatsby_plugin_resolves(source, config_path, &mut result);

        result
    }
}

/// Extract `resolve` string values from Gatsby plugin objects in the plugins array.
///
/// Handles: `plugins: [{ resolve: "gatsby-plugin-x", options: {} }]`
fn extract_gatsby_plugin_resolves(source: &str, path: &Path, result: &mut PluginResult) {
    use oxc_allocator::Allocator;
    use oxc_ast::ast::*;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    let source_type = SourceType::from_path(path).unwrap_or_default();
    let alloc = Allocator::default();
    let parsed = Parser::new(&alloc, source, source_type).parse();

    let Some(obj) = config_parser::find_config_object_pub(&parsed.program) else {
        return;
    };

    // Find the plugins property
    let Some(plugins_prop) = obj.properties.iter().find_map(|prop| {
        if let ObjectPropertyKind::ObjectProperty(p) = prop {
            let is_match = match &p.key {
                PropertyKey::StaticIdentifier(id) => id.name == "plugins",
                PropertyKey::StringLiteral(s) => s.value == "plugins",
                _ => false,
            };
            if is_match {
                return Some(p);
            }
        }
        None
    }) else {
        return;
    };

    let Expression::ArrayExpression(arr) = &plugins_prop.value else {
        return;
    };

    for el in &arr.elements {
        if let Some(Expression::ObjectExpression(plugin_obj)) = el.as_expression() {
            // Look for { resolve: "plugin-name" }
            for prop in &plugin_obj.properties {
                if let ObjectPropertyKind::ObjectProperty(p) = prop {
                    let is_resolve = match &p.key {
                        PropertyKey::StaticIdentifier(id) => id.name == "resolve",
                        PropertyKey::StringLiteral(s) => s.value == "resolve",
                        _ => false,
                    };
                    if is_resolve && let Expression::StringLiteral(s) = &p.value {
                        let dep = crate::resolve::extract_package_name(&s.value);
                        result.referenced_dependencies.push(dep);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_config_string_plugins() {
        let source = r#"
            module.exports = {
                plugins: ["gatsby-plugin-image", "gatsby-plugin-sharp"]
            };
        "#;
        let plugin = GatsbyPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("gatsby-config.js"),
            source,
            std::path::Path::new("/project"),
        );
        let deps = &result.referenced_dependencies;
        assert!(deps.contains(&"gatsby-plugin-image".to_string()));
        assert!(deps.contains(&"gatsby-plugin-sharp".to_string()));
    }

    #[test]
    fn resolve_config_object_plugins() {
        let source = r#"
            module.exports = {
                plugins: [
                    {
                        resolve: "gatsby-source-filesystem",
                        options: { name: "images", path: "./src/images" }
                    },
                    "gatsby-plugin-sharp"
                ]
            };
        "#;
        let plugin = GatsbyPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("gatsby-config.js"),
            source,
            std::path::Path::new("/project"),
        );
        let deps = &result.referenced_dependencies;
        assert!(deps.contains(&"gatsby-source-filesystem".to_string()));
        assert!(deps.contains(&"gatsby-plugin-sharp".to_string()));
    }

    #[test]
    fn resolve_config_imports() {
        let source = r#"
            import type { GatsbyConfig } from "gatsby";
            export default {
                plugins: ["gatsby-plugin-postcss"]
            };
        "#;
        let plugin = GatsbyPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("gatsby-config.ts"),
            source,
            std::path::Path::new("/project"),
        );
        let deps = &result.referenced_dependencies;
        assert!(deps.contains(&"gatsby".to_string()));
        assert!(deps.contains(&"gatsby-plugin-postcss".to_string()));
    }
}
