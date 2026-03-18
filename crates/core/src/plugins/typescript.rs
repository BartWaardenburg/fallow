//! TypeScript plugin.
//!
//! Detects TypeScript projects and marks tsconfig files as always used.
//! Parses tsconfig.json to extract project references, extended configs,
//! type package dependencies, language service plugins, and array extends (TS 5.0+).

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct TypeScriptPlugin;

const ENABLERS: &[&str] = &["typescript"];

const CONFIG_PATTERNS: &[&str] = &["tsconfig.json", "tsconfig.*.json"];

const ALWAYS_USED: &[&str] = &["tsconfig.json", "tsconfig.*.json"];

const TOOLING_DEPENDENCIES: &[&str] = &["typescript", "ts-node", "tsx", "ts-loader"];

impl Plugin for TypeScriptPlugin {
    fn name(&self) -> &'static str {
        "typescript"
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

    fn resolve_config(&self, config_path: &Path, source: &str, root: &Path) -> PluginResult {
        let mut result = PluginResult::default();

        // tsconfig.json is JSON — wrap in parens to make it a valid JS expression for Oxc
        let is_json = config_path.extension().is_some_and(|ext| ext == "json");
        let (parse_source, parse_path_buf);
        let parse_path: &Path;
        if is_json {
            parse_source = format!("({source})");
            parse_path_buf = config_path.with_extension("js");
            parse_path = &parse_path_buf;
        } else {
            parse_source = source.to_string();
            parse_path_buf = config_path.to_path_buf();
            parse_path = &parse_path_buf;
        };

        // extends → referenced dependency or base config file
        if let Some(extends) =
            config_parser::extract_config_string(&parse_source, parse_path, &["extends"])
        {
            if extends.starts_with('.') || extends.starts_with('/') {
                result
                    .setup_files
                    .push(root.join(extends.trim_start_matches("./")));
            } else {
                let dep = crate::resolve::extract_package_name(&extends);
                result.referenced_dependencies.push(dep);
            }
        }

        // extends as array (TypeScript 5.0+)
        // e.g. "extends": ["./tsconfig.base.json", "@tsconfig/node18"]
        let extends_arr =
            config_parser::extract_config_string_array(&parse_source, parse_path, &["extends"]);
        for ext in &extends_arr {
            if ext.starts_with('.') || ext.starts_with('/') {
                result
                    .setup_files
                    .push(root.join(ext.trim_start_matches("./")));
            } else {
                let dep = crate::resolve::extract_package_name(ext);
                result.referenced_dependencies.push(dep);
            }
        }

        // compilerOptions.types → @types/* dependencies
        let types = config_parser::extract_config_string_array(
            &parse_source,
            parse_path,
            &["compilerOptions", "types"],
        );
        for ty in &types {
            let base = crate::resolve::extract_package_name(ty);
            if !base.starts_with('@') {
                result
                    .referenced_dependencies
                    .push(format!("@types/{base}"));
            }
            result.referenced_dependencies.push(base);
        }

        // compilerOptions.jsxImportSource → referenced dependency
        if let Some(jsx_source) = config_parser::extract_config_string(
            &parse_source,
            parse_path,
            &["compilerOptions", "jsxImportSource"],
        ) {
            result.referenced_dependencies.push(jsx_source);
        }

        // compilerOptions.plugins → referenced dependencies (TS language service plugins)
        parse_tsconfig_plugins(&parse_source, parse_path, &mut result);

        // references → project reference paths
        parse_tsconfig_references(&parse_source, parse_path, root, &mut result);

        result
    }
}

/// Extract `compilerOptions.plugins[].name` from a tsconfig as referenced dependencies.
fn parse_tsconfig_plugins(source: &str, path: &Path, result: &mut PluginResult) {
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

    // Navigate to compilerOptions
    let compiler_opts = obj.properties.iter().find_map(|prop| {
        if let ObjectPropertyKind::ObjectProperty(p) = prop {
            let is_compiler_opts = match &p.key {
                PropertyKey::StaticIdentifier(id) => id.name == "compilerOptions",
                PropertyKey::StringLiteral(s) => s.value == "compilerOptions",
                _ => false,
            };
            if is_compiler_opts && let Expression::ObjectExpression(obj) = &p.value {
                return Some(obj);
            }
        }
        None
    });
    let Some(compiler_opts) = compiler_opts else {
        return;
    };

    // Find plugins array
    let plugins_arr = compiler_opts.properties.iter().find_map(|prop| {
        if let ObjectPropertyKind::ObjectProperty(p) = prop {
            let is_plugins = match &p.key {
                PropertyKey::StaticIdentifier(id) => id.name == "plugins",
                PropertyKey::StringLiteral(s) => s.value == "plugins",
                _ => false,
            };
            if is_plugins && let Expression::ArrayExpression(arr) = &p.value {
                return Some(arr);
            }
        }
        None
    });
    let Some(plugins_arr) = plugins_arr else {
        return;
    };

    // Extract "name" from each plugin object
    for el in &plugins_arr.elements {
        if let Some(Expression::ObjectExpression(plugin_obj)) = el.as_expression() {
            for prop in &plugin_obj.properties {
                if let ObjectPropertyKind::ObjectProperty(p) = prop {
                    let is_name = match &p.key {
                        PropertyKey::StaticIdentifier(id) => id.name == "name",
                        PropertyKey::StringLiteral(s) => s.value == "name",
                        _ => false,
                    };
                    if is_name && let Expression::StringLiteral(s) = &p.value {
                        let dep = crate::resolve::extract_package_name(&s.value);
                        result.referenced_dependencies.push(dep);
                    }
                }
            }
        }
    }
}

/// Extract `references[].path` from a tsconfig and add them as setup files.
fn parse_tsconfig_references(source: &str, path: &Path, root: &Path, result: &mut PluginResult) {
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

    for prop in &obj.properties {
        if let ObjectPropertyKind::ObjectProperty(p) = prop {
            let is_references = match &p.key {
                PropertyKey::StaticIdentifier(id) => id.name == "references",
                PropertyKey::StringLiteral(s) => s.value == "references",
                _ => false,
            };
            if !is_references {
                continue;
            }
            if let Expression::ArrayExpression(arr) = &p.value {
                for el in &arr.elements {
                    if let Some(Expression::ObjectExpression(ref_obj)) = el.as_expression() {
                        for ref_prop in &ref_obj.properties {
                            if let ObjectPropertyKind::ObjectProperty(rp) = ref_prop {
                                let is_path = match &rp.key {
                                    PropertyKey::StaticIdentifier(id) => id.name == "path",
                                    PropertyKey::StringLiteral(s) => s.value == "path",
                                    _ => false,
                                };
                                if is_path && let Expression::StringLiteral(s) = &rp.value {
                                    let ref_path = s.value.to_string();
                                    let tsconfig_path = root
                                        .join(ref_path.trim_start_matches("./"))
                                        .join("tsconfig.json");
                                    result.setup_files.push(tsconfig_path);
                                }
                            }
                        }
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
    fn resolve_config_extends_package() {
        let source = r#"{"extends": "@tsconfig/node18/tsconfig.json"}"#;
        let plugin = TypeScriptPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("tsconfig.json"),
            source,
            std::path::Path::new("/project"),
        );
        assert!(
            result
                .referenced_dependencies
                .contains(&"@tsconfig/node18".to_string())
        );
    }

    #[test]
    fn resolve_config_extends_relative_path() {
        let source = r#"{"extends": "./tsconfig.base.json"}"#;
        let plugin = TypeScriptPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("tsconfig.json"),
            source,
            std::path::Path::new("/project"),
        );
        assert!(result.referenced_dependencies.is_empty());
        assert!(
            result
                .setup_files
                .contains(&std::path::PathBuf::from("/project/tsconfig.base.json"))
        );
    }

    #[test]
    fn resolve_config_extends_array() {
        let source = r#"{"extends": ["./tsconfig.base.json", "@tsconfig/node18/tsconfig.json"]}"#;
        let plugin = TypeScriptPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("tsconfig.json"),
            source,
            std::path::Path::new("/project"),
        );
        assert!(
            result
                .setup_files
                .contains(&std::path::PathBuf::from("/project/tsconfig.base.json"))
        );
        assert!(
            result
                .referenced_dependencies
                .contains(&"@tsconfig/node18".to_string())
        );
    }

    #[test]
    fn resolve_config_compiler_options_types() {
        let source = r#"{"compilerOptions": {"types": ["node", "jest"]}}"#;
        let plugin = TypeScriptPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("tsconfig.json"),
            source,
            std::path::Path::new("/project"),
        );
        let deps = &result.referenced_dependencies;
        assert!(deps.contains(&"@types/node".to_string()));
        assert!(deps.contains(&"node".to_string()));
        assert!(deps.contains(&"@types/jest".to_string()));
        assert!(deps.contains(&"jest".to_string()));
    }

    #[test]
    fn resolve_config_jsx_import_source() {
        let source = r#"{"compilerOptions": {"jsxImportSource": "react"}}"#;
        let plugin = TypeScriptPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("tsconfig.json"),
            source,
            std::path::Path::new("/project"),
        );
        assert!(
            result
                .referenced_dependencies
                .contains(&"react".to_string())
        );
    }

    #[test]
    fn resolve_config_compiler_options_plugins() {
        let source =
            r#"{"compilerOptions": {"plugins": [{"name": "typescript-plugin-css-modules"}]}}"#;
        let plugin = TypeScriptPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("tsconfig.json"),
            source,
            std::path::Path::new("/project"),
        );
        assert!(
            result
                .referenced_dependencies
                .contains(&"typescript-plugin-css-modules".to_string())
        );
    }

    #[test]
    fn resolve_config_references() {
        let source = r#"{"references": [{"path": "./packages/core"}, {"path": "./packages/ui"}]}"#;
        let plugin = TypeScriptPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("tsconfig.json"),
            source,
            std::path::Path::new("/project"),
        );
        assert!(result.setup_files.contains(&std::path::PathBuf::from(
            "/project/packages/core/tsconfig.json"
        )));
        assert!(result.setup_files.contains(&std::path::PathBuf::from(
            "/project/packages/ui/tsconfig.json"
        )));
    }

    #[test]
    fn resolve_config_with_comments_and_trailing_commas() {
        let source = r#"{
            // Base config for all packages
            "extends": "@tsconfig/strictest",
            "compilerOptions": {
                "types": ["node"],
            },
        }"#;
        let plugin = TypeScriptPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("tsconfig.json"),
            source,
            std::path::Path::new("/project"),
        );
        assert!(
            result
                .referenced_dependencies
                .contains(&"@tsconfig/strictest".to_string())
        );
        assert!(
            result
                .referenced_dependencies
                .contains(&"@types/node".to_string())
        );
    }
}
