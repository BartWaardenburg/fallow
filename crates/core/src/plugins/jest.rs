//! Jest test runner plugin.
//!
//! Detects Jest projects and marks test files as entry points.
//! Parses jest.config to extract setupFiles, testMatch, transform,
//! reporters, testEnvironment, preset, globalSetup/Teardown, watchPlugins,
//! resolver, snapshotSerializers, testRunner, and runner as referenced dependencies.

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

        // Handle JSON configs (jest.config.json)
        let is_json = config_path.extension().is_some_and(|ext| ext == "json");
        let (parse_source, parse_path_buf);
        let parse_path: &Path;
        if is_json {
            parse_source = format!("({source})");
            parse_path_buf = config_path.with_extension("js");
            parse_path = &parse_path_buf;
        } else {
            parse_source = source.to_string();
            parse_path_buf = config_path.to_path_buf();
            parse_path = &parse_path_buf;
        };

        // Extract import sources as referenced dependencies
        let imports = config_parser::extract_imports(&parse_source, parse_path);
        for imp in &imports {
            let dep = crate::resolve::extract_package_name(imp);
            result.referenced_dependencies.push(dep);
        }

        // preset → referenced dependency (e.g., "ts-jest", "react-native")
        if let Some(preset) =
            config_parser::extract_config_string(&parse_source, parse_path, &["preset"])
        {
            result
                .referenced_dependencies
                .push(crate::resolve::extract_package_name(&preset));
        }

        // setupFiles → setup files
        let setup_files =
            config_parser::extract_config_string_array(&parse_source, parse_path, &["setupFiles"]);
        for f in &setup_files {
            result
                .setup_files
                .push(root.join(f.trim_start_matches("./")));
        }

        // setupFilesAfterEnv → setup files
        let setup_after = config_parser::extract_config_string_array(
            &parse_source,
            parse_path,
            &["setupFilesAfterEnv"],
        );
        for f in &setup_after {
            result
                .setup_files
                .push(root.join(f.trim_start_matches("./")));
        }

        // globalSetup → setup file
        if let Some(setup) =
            config_parser::extract_config_string(&parse_source, parse_path, &["globalSetup"])
        {
            result
                .setup_files
                .push(root.join(setup.trim_start_matches("./")));
        }

        // globalTeardown → setup file
        if let Some(teardown) =
            config_parser::extract_config_string(&parse_source, parse_path, &["globalTeardown"])
        {
            result
                .setup_files
                .push(root.join(teardown.trim_start_matches("./")));
        }

        // testMatch → additional entry patterns
        let test_match =
            config_parser::extract_config_string_array(&parse_source, parse_path, &["testMatch"]);
        result.entry_patterns.extend(test_match);

        // transform values → referenced dependencies (shallow to avoid options objects)
        let transform_values =
            config_parser::extract_config_shallow_strings(&parse_source, parse_path, "transform");
        for val in &transform_values {
            let dep = crate::resolve::extract_package_name(val);
            result.referenced_dependencies.push(dep);
        }

        // reporters → referenced dependencies (shallow to avoid options objects)
        let reporters =
            config_parser::extract_config_shallow_strings(&parse_source, parse_path, "reporters");
        for reporter in &reporters {
            if !BUILTIN_REPORTERS.contains(&reporter.as_str()) {
                let dep = crate::resolve::extract_package_name(reporter);
                result.referenced_dependencies.push(dep);
            }
        }

        // testEnvironment → if not built-in, it's a referenced dependency
        if let Some(env) =
            config_parser::extract_config_string(&parse_source, parse_path, &["testEnvironment"])
            && !matches!(env.as_str(), "node" | "jsdom")
        {
            result
                .referenced_dependencies
                .push(format!("jest-environment-{env}"));
            result.referenced_dependencies.push(env);
        }

        // watchPlugins → referenced dependencies
        let watch_plugins = config_parser::extract_config_shallow_strings(
            &parse_source,
            parse_path,
            "watchPlugins",
        );
        for plugin in &watch_plugins {
            result
                .referenced_dependencies
                .push(crate::resolve::extract_package_name(plugin));
        }

        // resolver → referenced dependency (only if it's a package, not a relative path)
        if let Some(resolver) =
            config_parser::extract_config_string(&parse_source, parse_path, &["resolver"])
            && !resolver.starts_with('.')
            && !resolver.starts_with('/')
        {
            result
                .referenced_dependencies
                .push(crate::resolve::extract_package_name(&resolver));
        }

        // snapshotSerializers → referenced dependencies
        let serializers = config_parser::extract_config_string_array(
            &parse_source,
            parse_path,
            &["snapshotSerializers"],
        );
        for s in &serializers {
            result
                .referenced_dependencies
                .push(crate::resolve::extract_package_name(s));
        }

        // testRunner → referenced dependency (filter built-in runners)
        if let Some(runner) =
            config_parser::extract_config_string(&parse_source, parse_path, &["testRunner"])
            && !matches!(
                runner.as_str(),
                "jest-jasmine2" | "jest-circus" | "jest-circus/runner"
            )
        {
            result
                .referenced_dependencies
                .push(crate::resolve::extract_package_name(&runner));
        }

        // runner → referenced dependency (process runner, not test runner)
        if let Some(runner) =
            config_parser::extract_config_string(&parse_source, parse_path, &["runner"])
            && runner != "jest-runner"
        {
            result
                .referenced_dependencies
                .push(crate::resolve::extract_package_name(&runner));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_config_preset() {
        let source = r#"module.exports = { preset: "ts-jest" };"#;
        let plugin = JestPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("jest.config.js"),
            source,
            std::path::Path::new("/project"),
        );
        assert!(
            result
                .referenced_dependencies
                .contains(&"ts-jest".to_string())
        );
    }

    #[test]
    fn resolve_config_global_setup_teardown() {
        let source = r#"
            module.exports = {
                globalSetup: "./test/global-setup.ts",
                globalTeardown: "./test/global-teardown.ts"
            };
        "#;
        let plugin = JestPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("jest.config.js"),
            source,
            std::path::Path::new("/project"),
        );
        assert!(
            result
                .setup_files
                .contains(&std::path::PathBuf::from("/project/test/global-setup.ts"))
        );
        assert!(result.setup_files.contains(&std::path::PathBuf::from(
            "/project/test/global-teardown.ts"
        )));
    }

    #[test]
    fn resolve_config_watch_plugins() {
        let source = r#"
            module.exports = {
                watchPlugins: [
                    "jest-watch-typeahead/filename",
                    "jest-watch-typeahead/testname"
                ]
            };
        "#;
        let plugin = JestPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("jest.config.js"),
            source,
            std::path::Path::new("/project"),
        );
        let deps = &result.referenced_dependencies;
        assert!(deps.contains(&"jest-watch-typeahead".to_string()));
    }

    #[test]
    fn resolve_config_resolver() {
        let source = r#"module.exports = { resolver: "jest-resolver-enhanced" };"#;
        let plugin = JestPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("jest.config.js"),
            source,
            std::path::Path::new("/project"),
        );
        assert!(
            result
                .referenced_dependencies
                .contains(&"jest-resolver-enhanced".to_string())
        );
    }

    #[test]
    fn resolve_config_resolver_relative_not_added() {
        let source = r#"module.exports = { resolver: "./custom-resolver.js" };"#;
        let plugin = JestPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("jest.config.js"),
            source,
            std::path::Path::new("/project"),
        );
        assert!(
            !result
                .referenced_dependencies
                .iter()
                .any(|d| d.contains("custom-resolver"))
        );
    }

    #[test]
    fn resolve_config_snapshot_serializers() {
        let source = r#"
            module.exports = {
                snapshotSerializers: ["enzyme-to-json/serializer"]
            };
        "#;
        let plugin = JestPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("jest.config.js"),
            source,
            std::path::Path::new("/project"),
        );
        assert!(
            result
                .referenced_dependencies
                .contains(&"enzyme-to-json".to_string())
        );
    }

    #[test]
    fn resolve_config_test_runner_builtin() {
        let source = r#"module.exports = { testRunner: "jest-circus/runner" };"#;
        let plugin = JestPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("jest.config.js"),
            source,
            std::path::Path::new("/project"),
        );
        assert!(
            !result
                .referenced_dependencies
                .iter()
                .any(|d| d.contains("jest-circus"))
        );
    }

    #[test]
    fn resolve_config_custom_runner() {
        let source = r#"module.exports = { runner: "jest-runner-eslint" };"#;
        let plugin = JestPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("jest.config.js"),
            source,
            std::path::Path::new("/project"),
        );
        assert!(
            result
                .referenced_dependencies
                .contains(&"jest-runner-eslint".to_string())
        );
    }

    #[test]
    fn resolve_config_json() {
        let source = r#"{"preset": "ts-jest", "testEnvironment": "jsdom"}"#;
        let plugin = JestPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("jest.config.json"),
            source,
            std::path::Path::new("/project"),
        );
        assert!(
            result
                .referenced_dependencies
                .contains(&"ts-jest".to_string())
        );
    }
}
