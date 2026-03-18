//! PostCSS plugin.
//!
//! Detects PostCSS projects and marks config files as always used.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct PostCssPlugin;

const ENABLERS: &[&str] = &["postcss"];

const CONFIG_PATTERNS: &[&str] = &["postcss.config.{ts,js,cjs,mjs}"];

const ALWAYS_USED: &[&str] = &["postcss.config.{ts,js,cjs,mjs}"];

const TOOLING_DEPENDENCIES: &[&str] = &["postcss", "postcss-cli"];

impl Plugin for PostCssPlugin {
    fn name(&self) -> &'static str {
        "postcss"
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
