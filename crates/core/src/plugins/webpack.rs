//! Webpack bundler plugin.
//!
//! Detects Webpack projects and marks conventional entry points and config files.
//! Parses webpack config to extract entry points, plugin dependencies, loader
//! packages from module.rules, and external dependencies.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct WebpackPlugin;

const ENABLERS: &[&str] = &["webpack"];

const ENTRY_PATTERNS: &[&str] = &["src/index.{ts,tsx,js,jsx}"];

const CONFIG_PATTERNS: &[&str] = &[
    "webpack.config.{ts,js,mjs,cjs}",
    "webpack.*.config.{ts,js,mjs,cjs}",
];

const ALWAYS_USED: &[&str] = &[
    "webpack.config.{ts,js,mjs,cjs}",
    "webpack.*.config.{ts,js,mjs,cjs}",
];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "webpack",
    "webpack-cli",
    "webpack-dev-server",
    "html-webpack-plugin",
];

impl Plugin for WebpackPlugin {
    fn name(&self) -> &'static str {
        "webpack"
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

        let imports = config_parser::extract_imports(source, config_path);
        for imp in &imports {
            let dep = crate::resolve::extract_package_name(imp);
            result.referenced_dependencies.push(dep);
        }

        // entry → entry points (string, array, or object with string values)
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

        // externals → referenced dependencies (string array form)
        let externals =
            config_parser::extract_config_shallow_strings(source, config_path, "externals");
        for ext in &externals {
            result
                .referenced_dependencies
                .push(crate::resolve::extract_package_name(ext));
        }

        // module.rules → extract loader package names
        parse_webpack_loaders(source, config_path, &mut result);

        result
    }
}

/// Extract loader package names from webpack `module.rules` config.
///
/// Handles common patterns:
/// - `{ loader: 'ts-loader' }`
/// - `{ use: ['style-loader', 'css-loader'] }`
/// - `{ use: [{ loader: 'css-loader', options: {} }] }`
/// - `{ oneOf: [...rules] }`
fn parse_webpack_loaders(source: &str, path: &Path, result: &mut PluginResult) {
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

    // Navigate to module.rules
    let Some(module_prop) = find_obj_prop(obj, "module") else {
        return;
    };
    let Expression::ObjectExpression(module_obj) = &module_prop.value else {
        return;
    };
    let Some(rules_prop) = find_obj_prop(module_obj, "rules") else {
        return;
    };
    let Expression::ArrayExpression(rules_arr) = &rules_prop.value else {
        return;
    };

    walk_rules(rules_arr, result);
}

fn find_obj_prop<'a>(
    obj: &'a oxc_ast::ast::ObjectExpression<'a>,
    key: &str,
) -> Option<&'a oxc_ast::ast::ObjectProperty<'a>> {
    use oxc_ast::ast::*;
    for prop in &obj.properties {
        if let ObjectPropertyKind::ObjectProperty(p) = prop {
            let is_match = match &p.key {
                PropertyKey::StaticIdentifier(id) => id.name == key,
                PropertyKey::StringLiteral(s) => s.value == key,
                _ => false,
            };
            if is_match {
                return Some(p);
            }
        }
    }
    None
}

fn walk_rules(rules: &oxc_ast::ast::ArrayExpression, result: &mut PluginResult) {
    use oxc_ast::ast::*;
    for el in &rules.elements {
        if let Some(Expression::ObjectExpression(rule_obj)) = el.as_expression() {
            walk_rule(rule_obj, result);
        }
    }
}

fn walk_rule(rule: &oxc_ast::ast::ObjectExpression, result: &mut PluginResult) {
    use oxc_ast::ast::*;

    for prop in &rule.properties {
        let ObjectPropertyKind::ObjectProperty(p) = prop else {
            continue;
        };
        let key_name = match &p.key {
            PropertyKey::StaticIdentifier(id) => id.name.as_str(),
            PropertyKey::StringLiteral(s) => s.value.as_str(),
            _ => continue,
        };

        match key_name {
            // loader: 'ts-loader'
            "loader" => {
                if let Expression::StringLiteral(s) = &p.value {
                    let dep = crate::resolve::extract_package_name(&s.value);
                    result.referenced_dependencies.push(dep);
                }
            }
            // use: 'babel-loader' or use: ['style-loader', { loader: 'css-loader' }]
            "use" => match &p.value {
                Expression::StringLiteral(s) => {
                    let dep = crate::resolve::extract_package_name(&s.value);
                    result.referenced_dependencies.push(dep);
                }
                Expression::ArrayExpression(arr) => {
                    for use_el in &arr.elements {
                        if let Some(expr) = use_el.as_expression() {
                            match expr {
                                Expression::StringLiteral(s) => {
                                    let dep = crate::resolve::extract_package_name(&s.value);
                                    result.referenced_dependencies.push(dep);
                                }
                                Expression::ObjectExpression(use_obj) => {
                                    if let Some(loader_prop) = find_obj_prop(use_obj, "loader")
                                        && let Expression::StringLiteral(s) = &loader_prop.value
                                    {
                                        let dep = crate::resolve::extract_package_name(&s.value);
                                        result.referenced_dependencies.push(dep);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            },
            // oneOf: [...rules] → recurse
            "oneOf" => {
                if let Expression::ArrayExpression(one_of) = &p.value {
                    walk_rules(one_of, result);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_config_entry_string() {
        let source = r#"module.exports = { entry: "./src/app.js" };"#;
        let plugin = WebpackPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("webpack.config.js"),
            source,
            std::path::Path::new("/project"),
        );
        assert_eq!(result.entry_patterns, vec!["./src/app.js"]);
    }

    #[test]
    fn resolve_config_loaders() {
        let source = r#"
            module.exports = {
                module: {
                    rules: [
                        { test: /\.tsx?$/, loader: 'ts-loader' },
                        { test: /\.css$/, use: ['style-loader', 'css-loader'] },
                        { test: /\.scss$/, use: [
                            'style-loader',
                            { loader: 'css-loader', options: { modules: true } },
                            'sass-loader'
                        ]}
                    ]
                }
            };
        "#;
        let plugin = WebpackPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("webpack.config.js"),
            source,
            std::path::Path::new("/project"),
        );
        let deps = &result.referenced_dependencies;
        assert!(deps.contains(&"ts-loader".to_string()));
        assert!(deps.contains(&"style-loader".to_string()));
        assert!(deps.contains(&"css-loader".to_string()));
        assert!(deps.contains(&"sass-loader".to_string()));
    }

    #[test]
    fn resolve_config_one_of_loaders() {
        let source = r#"
            module.exports = {
                module: {
                    rules: [
                        { oneOf: [
                            { test: /\.svg$/, loader: 'svg-loader' },
                            { test: /\.png$/, use: 'file-loader' }
                        ]}
                    ]
                }
            };
        "#;
        let plugin = WebpackPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("webpack.config.js"),
            source,
            std::path::Path::new("/project"),
        );
        let deps = &result.referenced_dependencies;
        assert!(deps.contains(&"svg-loader".to_string()));
        assert!(deps.contains(&"file-loader".to_string()));
    }

    #[test]
    fn resolve_config_externals() {
        let source = r#"module.exports = { externals: ["react", "react-dom"] };"#;
        let plugin = WebpackPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("webpack.config.js"),
            source,
            std::path::Path::new("/project"),
        );
        let deps = &result.referenced_dependencies;
        assert!(deps.contains(&"react".to_string()));
        assert!(deps.contains(&"react-dom".to_string()));
    }
}
