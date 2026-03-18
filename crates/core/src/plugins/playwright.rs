//! Playwright test runner plugin.
//!
//! Detects Playwright projects and marks test files and config as entry points.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct PlaywrightPlugin;

const ENABLERS: &[&str] = &["@playwright/test"];

const ENTRY_PATTERNS: &[&str] = &[
    "**/*.spec.{ts,tsx,js,jsx}",
    "**/*.test.{ts,tsx,js,jsx}",
    "tests/**/*.{ts,tsx,js,jsx}",
    "e2e/**/*.{ts,tsx,js,jsx}",
];

const CONFIG_PATTERNS: &[&str] = &["playwright.config.{ts,js}"];

const ALWAYS_USED: &[&str] = &["playwright.config.{ts,js}"];

const TOOLING_DEPENDENCIES: &[&str] = &["@playwright/test", "playwright"];

impl Plugin for PlaywrightPlugin {
    fn name(&self) -> &'static str {
        "playwright"
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

    fn resolve_config(&self, config_path: &Path, source: &str, root: &Path) -> PluginResult {
        let mut result = PluginResult::default();

        let imports = config_parser::extract_imports(source, config_path);
        for imp in &imports {
            let dep = crate::resolve::extract_package_name(imp);
            result.referenced_dependencies.push(dep);
        }

        // globalSetup / globalTeardown → setup files
        if let Some(setup) =
            config_parser::extract_config_string(source, config_path, &["globalSetup"])
        {
            result
                .setup_files
                .push(root.join(setup.trim_start_matches("./")));
        }
        if let Some(teardown) =
            config_parser::extract_config_string(source, config_path, &["globalTeardown"])
        {
            result
                .setup_files
                .push(root.join(teardown.trim_start_matches("./")));
        }

        result
    }
}
