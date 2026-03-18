//! Jest test runner plugin.
//!
//! Detects Jest projects and marks test files as entry points.
//! Parses jest.config to extract setupFiles, testMatch, transform,
//! reporters, and testEnvironment as referenced dependencies.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct JestPlugin;

const ENABLERS: &[&str] = &["jest"];

const ENTRY_PATTERNS: &[&str] = &[
    "**/*.test.{ts,tsx,js,jsx}",
    "**/*.spec.{ts,tsx,js,jsx}",
    "**/__tests__/**/*.{ts,tsx,js,jsx}",
];

const CONFIG_PATTERNS: &[&str] = &["jest.config.{ts,js,mjs,cjs}", "jest.config.json"];

const ALWAYS_USED: &[&str] = &["jest.config.{ts,js,mjs,cjs}", "jest.setup.{ts,js,tsx,jsx}"];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "jest",
    "@jest/globals",
    "jest-environment-jsdom",
    "ts-jest",
    "babel-jest",
    "@jest/types",
];

/// Built-in Jest reporter names that should not be treated as dependencies.
const BUILTIN_REPORTERS: &[&str] = &["default", "verbose", "summary"];

impl Plugin for JestPlugin {
    fn name(&self) -> &'static str {
        "jest"
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

        // setupFiles → setup files
        let setup_files =
            config_parser::extract_config_string_array(source, config_path, &["setupFiles"]);
        for f in &setup_files {
            result
                .setup_files
                .push(root.join(f.trim_start_matches("./")));
        }

        // setupFilesAfterEnv → setup files
        let setup_after = config_parser::extract_config_string_array(
            source,
            config_path,
            &["setupFilesAfterEnv"],
        );
        for f in &setup_after {
            result
                .setup_files
                .push(root.join(f.trim_start_matches("./")));
        }

        // testMatch → additional entry patterns
        let test_match =
            config_parser::extract_config_string_array(source, config_path, &["testMatch"]);
        result.entry_patterns.extend(test_match);

        // transform values → referenced dependencies (shallow to avoid options objects)
        let transform_values =
            config_parser::extract_config_shallow_strings(source, config_path, "transform");
        for val in &transform_values {
            let dep = crate::resolve::extract_package_name(val);
            result.referenced_dependencies.push(dep);
        }

        // reporters → referenced dependencies (shallow to avoid options objects)
        let reporters =
            config_parser::extract_config_shallow_strings(source, config_path, "reporters");
        for reporter in &reporters {
            if !BUILTIN_REPORTERS.contains(&reporter.as_str()) {
                let dep = crate::resolve::extract_package_name(reporter);
                result.referenced_dependencies.push(dep);
            }
        }

        // testEnvironment → if not built-in, it's a referenced dependency
        // Jest custom environments use the package name `jest-environment-<name>`
        if let Some(env) =
            config_parser::extract_config_string(source, config_path, &["testEnvironment"])
            && !matches!(env.as_str(), "node" | "jsdom")
        {
            result
                .referenced_dependencies
                .push(format!("jest-environment-{env}"));
            // Also push raw name in case the package is named directly
            result.referenced_dependencies.push(env);
        }

        result
    }
}
