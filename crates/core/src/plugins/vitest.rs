//! Vitest test runner plugin.
//!
//! Detects Vitest projects and marks test/bench files as entry points.
//! Parses vitest.config to extract test.include, setupFiles, globalSetup,
//! and custom test environments as referenced dependencies.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct VitestPlugin;

const ENABLERS: &[&str] = &["vitest"];

const ENTRY_PATTERNS: &[&str] = &[
    "**/*.test.{ts,tsx,js,jsx}",
    "**/*.spec.{ts,tsx,js,jsx}",
    "**/__tests__/**/*.{ts,tsx,js,jsx}",
    "**/*.bench.{ts,tsx,js,jsx}",
];

const CONFIG_PATTERNS: &[&str] = &["vitest.config.{ts,js,mts,mjs}", "vitest.workspace.{ts,js}"];

const ALWAYS_USED: &[&str] = &[
    "vitest.config.{ts,js,mts,mjs}",
    "vitest.setup.{ts,js}",
    "vitest.workspace.{ts,js}",
];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "vitest",
    "@vitest/coverage-v8",
    "@vitest/coverage-istanbul",
    "@vitest/ui",
    "@vitest/browser",
];

impl Plugin for VitestPlugin {
    fn name(&self) -> &'static str {
        "vitest"
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

        // Extract import sources as referenced dependencies
        let imports = config_parser::extract_imports(source, config_path);
        for imp in &imports {
            let dep = crate::resolve::extract_package_name(imp);
            result.referenced_dependencies.push(dep);
        }

        // test.include → additional entry patterns
        let includes =
            config_parser::extract_config_string_array(source, config_path, &["test", "include"]);
        result.entry_patterns.extend(includes);

        // test.setupFiles → setup files
        let setup_files = config_parser::extract_config_string_array(
            source,
            config_path,
            &["test", "setupFiles"],
        );
        for f in &setup_files {
            result
                .setup_files
                .push(root.join(f.trim_start_matches("./")));
        }

        // test.globalSetup → setup files
        let global_setup = config_parser::extract_config_string_array(
            source,
            config_path,
            &["test", "globalSetup"],
        );
        for f in &global_setup {
            result
                .setup_files
                .push(root.join(f.trim_start_matches("./")));
        }
        // globalSetup can also be a single string
        if global_setup.is_empty()
            && let Some(gs) =
                config_parser::extract_config_string(source, config_path, &["test", "globalSetup"])
        {
            result
                .setup_files
                .push(root.join(gs.trim_start_matches("./")));
        }

        // test.environment → if custom, it's a referenced dependency
        // Vitest custom environments use the package name `vitest-environment-<name>`
        if let Some(env) =
            config_parser::extract_config_string(source, config_path, &["test", "environment"])
            && !matches!(env.as_str(), "node" | "jsdom" | "happy-dom")
        {
            result
                .referenced_dependencies
                .push(format!("vitest-environment-{env}"));
            result.referenced_dependencies.push(env);
        }

        result
    }
}
