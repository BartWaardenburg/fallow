//! Docusaurus documentation framework plugin.
//!
//! Detects Docusaurus projects and marks docs, blog, pages, and config as entry points.
//! Parses docusaurus.config to extract referenced dependencies.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct DocusaurusPlugin;

const ENABLERS: &[&str] = &["@docusaurus/core"];

const ENTRY_PATTERNS: &[&str] = &[
    "docs/**/*.{md,mdx}",
    "blog/**/*.{md,mdx}",
    "src/pages/**/*.{ts,tsx,js,jsx,md,mdx}",
    "sidebars.{js,ts}",
];

const CONFIG_PATTERNS: &[&str] = &["docusaurus.config.{js,ts,mjs}"];

const ALWAYS_USED: &[&str] = &["docusaurus.config.{js,ts,mjs}", "sidebars.{js,ts}"];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "@docusaurus/core",
    "@docusaurus/preset-classic",
    "@docusaurus/plugin-content-docs",
    "@docusaurus/plugin-content-blog",
    "@docusaurus/theme-classic",
];

impl Plugin for DocusaurusPlugin {
    fn name(&self) -> &'static str {
        "docusaurus"
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
