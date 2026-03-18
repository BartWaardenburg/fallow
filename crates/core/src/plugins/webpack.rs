//! Webpack bundler plugin.
//!
//! Detects Webpack projects and marks conventional entry points and config files.

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

        result
    }
}
