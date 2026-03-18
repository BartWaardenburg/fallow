//! semantic-release plugin.
//!
//! Detects semantic-release projects and marks config files as always used.
//! Parses config to extract plugin references as dependencies.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct SemanticReleasePlugin;

const ENABLERS: &[&str] = &["semantic-release"];

const CONFIG_PATTERNS: &[&str] = &["release.config.{js,cjs,mjs}", ".releaserc.{js,cjs}"];

const ALWAYS_USED: &[&str] = &[
    "release.config.{js,cjs,mjs}",
    ".releaserc.{json,yaml,yml,js,cjs}",
];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "semantic-release",
    "@semantic-release/commit-analyzer",
    "@semantic-release/release-notes-generator",
    "@semantic-release/changelog",
    "@semantic-release/npm",
    "@semantic-release/github",
    "@semantic-release/git",
];

impl Plugin for SemanticReleasePlugin {
    fn name(&self) -> &'static str {
        "semantic-release"
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

        // plugins → referenced dependencies (shallow to avoid options objects)
        let plugins = config_parser::extract_config_shallow_strings(source, config_path, "plugins");
        for plugin in &plugins {
            let dep = crate::resolve::extract_package_name(plugin);
            result.referenced_dependencies.push(dep);
        }

        result
    }
}
