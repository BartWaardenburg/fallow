//! Storybook plugin.
//!
//! Detects Storybook projects and marks story files and config as entry points.
//! Parses .storybook/main config to extract addons, framework, and stories
//! patterns as referenced dependencies and additional entry patterns.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct StorybookPlugin;

const ENABLERS: &[&str] = &["storybook", "@storybook/"];

const ENTRY_PATTERNS: &[&str] = &[
    "**/*.stories.{ts,tsx,js,jsx,mdx}",
    ".storybook/**/*.{ts,tsx,js,jsx}",
];

const CONFIG_PATTERNS: &[&str] = &[".storybook/main.{ts,js,mjs,cjs}"];

const ALWAYS_USED: &[&str] = &[
    ".storybook/main.{ts,js,mjs,cjs}",
    ".storybook/preview.{ts,tsx,js,jsx}",
    ".storybook/manager.{ts,tsx,js,jsx}",
];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "storybook",
    "@storybook/react",
    "@storybook/vue3",
    "@storybook/angular",
    "@storybook/svelte",
    "@storybook/web-components",
    "@storybook/html",
    "@storybook/server",
    "@storybook/addon-essentials",
    "@storybook/addon-interactions",
    "@storybook/addon-links",
    "@storybook/addon-a11y",
    "@storybook/addon-docs",
    "@storybook/addon-actions",
    "@storybook/addon-viewport",
    "@storybook/addon-controls",
    "@storybook/addon-backgrounds",
    "@storybook/addon-toolbars",
    "@storybook/addon-measure",
    "@storybook/addon-outline",
    "@storybook/blocks",
    "@storybook/testing-library",
    "@storybook/test",
    "@storybook/manager-api",
    "@storybook/preview-api",
    "@storybook/builder-vite",
    "@storybook/builder-webpack5",
    "@chromatic-com/storybook",
];

impl Plugin for StorybookPlugin {
    fn name(&self) -> &'static str {
        "storybook"
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

        // Extract import sources as referenced dependencies
        let imports = config_parser::extract_imports(source, config_path);
        for imp in &imports {
            let dep = crate::resolve::extract_package_name(imp);
            result.referenced_dependencies.push(dep);
        }

        // addons → referenced dependencies (shallow to avoid options objects)
        let addons = config_parser::extract_config_shallow_strings(source, config_path, "addons");
        for addon in &addons {
            let dep = crate::resolve::extract_package_name(addon);
            result.referenced_dependencies.push(dep);
        }

        // framework → referenced dependency
        // Can be a string or an object with a `.name` property
        if let Some(framework) =
            config_parser::extract_config_string(source, config_path, &["framework"])
        {
            let dep = crate::resolve::extract_package_name(&framework);
            result.referenced_dependencies.push(dep);
        } else if let Some(framework_name) =
            config_parser::extract_config_string(source, config_path, &["framework", "name"])
        {
            let dep = crate::resolve::extract_package_name(&framework_name);
            result.referenced_dependencies.push(dep);
        }

        // stories → additional entry patterns (if string values)
        let stories = config_parser::extract_config_string_array(source, config_path, &["stories"]);
        result.entry_patterns.extend(stories);

        result
    }
}
