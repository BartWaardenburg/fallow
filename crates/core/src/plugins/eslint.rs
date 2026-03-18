//! ESLint plugin.
//!
//! Detects ESLint projects and marks config files as always used.
//! Parses ESLint config to extract plugin/config imports as referenced dependencies.
//! Also covers Prettier and lint-staged config files.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct EslintPlugin;

const ENABLERS: &[&str] = &["eslint", "@eslint/js"];

const CONFIG_PATTERNS: &[&str] = &[
    "eslint.config.{js,mjs,cjs,ts,mts,cts}",
    ".eslintrc.{js,cjs,mjs,json,yaml,yml}",
];

const ALWAYS_USED: &[&str] = &[
    "eslint.config.{js,mjs,cjs,ts,mts,cts}",
    ".eslintrc.{js,cjs,mjs,json,yaml,yml}",
    ".prettierrc.{js,cjs,mjs,json,yaml,yml}",
    "prettier.config.{js,mjs,cjs,ts}",
    ".lintstagedrc.{js,cjs,mjs,json}",
    "lint-staged.config.{js,mjs,cjs}",
];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "eslint",
    "@eslint/js",
    "@eslint/eslintrc",
    "eslint-config-next",
    "eslint-config-prettier",
    "eslint-plugin-react",
    "eslint-plugin-react-hooks",
    "eslint-plugin-jsx-a11y",
    "eslint-plugin-import",
    "@typescript-eslint/parser",
    "@typescript-eslint/eslint-plugin",
    "prettier",
    "eslint-plugin-prettier",
    "eslint-plugin-sonarjs",
    "eslint-plugin-storybook",
];

const ESLINT_CONFIG_EXPORTS: &[&str] = &["default"];

impl Plugin for EslintPlugin {
    fn name(&self) -> &'static str {
        "eslint"
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

    fn used_exports(&self) -> Vec<(&'static str, &'static [&'static str])> {
        vec![(
            "eslint.config.{js,mjs,cjs,ts,mts,cts}",
            ESLINT_CONFIG_EXPORTS,
        )]
    }

    fn resolve_config(&self, config_path: &Path, source: &str, _root: &Path) -> PluginResult {
        let mut result = PluginResult::default();

        // Extract import sources as referenced dependencies (eslint plugins, configs)
        let imports = config_parser::extract_imports(source, config_path);
        for imp in &imports {
            let dep = crate::resolve::extract_package_name(imp);
            result.referenced_dependencies.push(dep);
        }

        result
    }
}
