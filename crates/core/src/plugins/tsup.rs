//! Tsup TypeScript library bundler plugin.
//!
//! Detects Tsup projects and marks config files as always used.
//! Parses tsup config to extract referenced dependencies.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct TsupPlugin;

const ENABLERS: &[&str] = &["tsup"];

const CONFIG_PATTERNS: &[&str] = &["tsup.config.{ts,js,cjs,mjs}"];

const ALWAYS_USED: &[&str] = &["tsup.config.{ts,js,cjs,mjs}"];

const TOOLING_DEPENDENCIES: &[&str] = &["tsup"];

impl Plugin for TsupPlugin {
    fn name(&self) -> &'static str {
        "tsup"
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

        // entry → source entry points for the library
        let entries = config_parser::extract_config_string_array(source, config_path, &["entry"]);
        result.entry_patterns.extend(entries);

        result
    }
}
