//! Commitlint plugin.
//!
//! Detects Commitlint projects and marks config files as always used.
//! Parses config to extract referenced dependencies.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct CommitlintPlugin;

const ENABLERS: &[&str] = &["@commitlint/cli"];

const CONFIG_PATTERNS: &[&str] = &[
    "commitlint.config.{js,cjs,mjs,ts}",
    ".commitlintrc.{js,cjs}",
];

const ALWAYS_USED: &[&str] = &[
    "commitlint.config.{js,cjs,mjs,ts}",
    ".commitlintrc.{json,yaml,yml,js,cjs}",
];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "@commitlint/cli",
    "@commitlint/config-conventional",
    "@commitlint/config-angular",
];

impl Plugin for CommitlintPlugin {
    fn name(&self) -> &'static str {
        "commitlint"
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
