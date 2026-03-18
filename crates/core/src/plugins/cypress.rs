//! Cypress test runner plugin.
//!
//! Detects Cypress projects and marks test files and support files as entry points.
//! Parses cypress.config to extract referenced dependencies.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct CypressPlugin;

const ENABLERS: &[&str] = &["cypress"];

const ENTRY_PATTERNS: &[&str] = &[
    "cypress/**/*.{ts,tsx,js,jsx}",
    "cypress/support/**/*.{ts,js}",
];

const CONFIG_PATTERNS: &[&str] = &["cypress.config.{ts,js,mjs,cjs}"];

const ALWAYS_USED: &[&str] = &["cypress.config.{ts,js,mjs,cjs}"];

const TOOLING_DEPENDENCIES: &[&str] = &["cypress", "@cypress/react", "@cypress/vue"];

impl Plugin for CypressPlugin {
    fn name(&self) -> &'static str {
        "cypress"
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
