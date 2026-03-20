//! Remark plugin.
//!
//! Detects remark projects and marks config files as always used.
//! Parses JS/CJS config files to extract referenced dependencies.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct RemarkPlugin;

const ENABLERS: &[&str] = &["remark", "remark-cli"];

const CONFIG_PATTERNS: &[&str] = &[".remarkrc.{js,cjs,mjs}"];

const ALWAYS_USED: &[&str] = &[".remarkrc", ".remarkrc.{js,cjs,mjs,json,yml,yaml}"];

const TOOLING_DEPENDENCIES: &[&str] = &["remark", "remark-cli"];

impl Plugin for RemarkPlugin {
    fn name(&self) -> &'static str {
        "remark"
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

        result
    }
}
