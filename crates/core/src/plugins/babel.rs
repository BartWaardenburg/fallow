//! Babel transpiler plugin.
//!
//! Detects Babel projects and marks config files as always used.
//! Parses babel config to extract presets, plugins, and imports as referenced dependencies.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct BabelPlugin;

const ENABLERS: &[&str] = &["@babel/core"];

const CONFIG_PATTERNS: &[&str] = &[
    "babel.config.{js,cjs,mjs,ts,cts}",
    ".babelrc",
    ".babelrc.{js,cjs,mjs,json}",
];

const ALWAYS_USED: &[&str] = &[
    "babel.config.{js,cjs,mjs,ts,cts}",
    ".babelrc",
    ".babelrc.{js,cjs,mjs,json}",
];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "@babel/core",
    "@babel/cli",
    "@babel/preset-env",
    "@babel/preset-react",
    "@babel/preset-typescript",
    "@babel/plugin-transform-runtime",
    "@babel/runtime",
    "babel-loader",
    "babel-jest",
];

impl Plugin for BabelPlugin {
    fn name(&self) -> &'static str {
        "babel"
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

        // presets → referenced dependencies (shallow to avoid options objects)
        let presets = config_parser::extract_config_shallow_strings(source, config_path, "presets");
        for preset in &presets {
            let dep = crate::resolve::extract_package_name(preset);
            result.referenced_dependencies.push(dep);
        }

        // plugins → referenced dependencies (shallow to avoid options objects)
        let plugins = config_parser::extract_config_shallow_strings(source, config_path, "plugins");
        for plugin in &plugins {
            let dep = crate::resolve::extract_package_name(plugin);
            result.referenced_dependencies.push(dep);
        }

        result
    }
}
