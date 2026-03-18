//! Rollup module bundler plugin.
//!
//! Detects Rollup projects and marks config files as always used.
//! Parses rollup config to extract imports and plugin references as dependencies.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct RollupPlugin;

const ENABLERS: &[&str] = &["rollup"];

const CONFIG_PATTERNS: &[&str] = &["rollup.config.{js,ts,mjs,cjs}"];

const ALWAYS_USED: &[&str] = &["rollup.config.{js,ts,mjs,cjs}"];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "rollup",
    "@rollup/plugin-node-resolve",
    "@rollup/plugin-commonjs",
    "@rollup/plugin-typescript",
    "@rollup/plugin-babel",
    "@rollup/plugin-terser",
    "@rollup/plugin-json",
    "rollup-plugin-dts",
];

impl Plugin for RollupPlugin {
    fn name(&self) -> &'static str {
        "rollup"
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

        result
    }
}
