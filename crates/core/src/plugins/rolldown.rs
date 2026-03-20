//! Rolldown bundler plugin.
//!
//! Detects Rolldown projects (Rust-based Rollup replacement) and marks config
//! files as always used. Parses rolldown config to extract imports and entry
//! point references as dependencies.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct RolldownPlugin;

const ENABLERS: &[&str] = &["rolldown"];

const CONFIG_PATTERNS: &[&str] = &["rolldown.config.{js,cjs,mjs,ts,mts,cts}"];

const ALWAYS_USED: &[&str] = &["rolldown.config.{js,cjs,mjs,ts,mts,cts}"];

const TOOLING_DEPENDENCIES: &[&str] = &["rolldown", "@rolldown/pluginutils"];

impl Plugin for RolldownPlugin {
    fn name(&self) -> &'static str {
        "rolldown"
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

        // input → entry points (string, array, or object)
        let inputs = config_parser::extract_config_string_or_array(source, config_path, &["input"]);
        result.entry_patterns.extend(inputs);

        // external → referenced dependencies (string array)
        let external =
            config_parser::extract_config_shallow_strings(source, config_path, "external");
        for ext in &external {
            result
                .referenced_dependencies
                .push(crate::resolve::extract_package_name(ext));
        }

        result
    }
}
