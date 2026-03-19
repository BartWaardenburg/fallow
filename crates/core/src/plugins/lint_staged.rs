//! lint-staged plugin.
//!
//! Detects lint-staged projects and marks config files as always used.
//! Parses JS/CJS config files to extract referenced dependencies.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct LintStagedPlugin;

const ENABLERS: &[&str] = &["lint-staged"];

const CONFIG_PATTERNS: &[&str] = &[
    "lint-staged.config.{js,cjs,mjs,ts}",
    ".lintstagedrc.{js,cjs,mjs,ts}",
];

const ALWAYS_USED: &[&str] = &[
    "lint-staged.config.{js,cjs,mjs,ts}",
    ".lintstagedrc",
    ".lintstagedrc.{json,yaml,yml,js,cjs,mjs,ts}",
];

const TOOLING_DEPENDENCIES: &[&str] = &["lint-staged"];

impl Plugin for LintStagedPlugin {
    fn name(&self) -> &'static str {
        "lint-staged"
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
