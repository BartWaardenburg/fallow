//! GraphQL Codegen plugin.
//!
//! Detects GraphQL Codegen projects and marks config files as always used.
//! Parses codegen config to extract referenced dependencies.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct GraphqlCodegenPlugin;

const ENABLERS: &[&str] = &["@graphql-codegen/cli"];

const CONFIG_PATTERNS: &[&str] = &["codegen.{ts,js}", "graphql.config.{ts,js}"];

const ALWAYS_USED: &[&str] = &[
    "codegen.{ts,js,yml,yaml}",
    "graphql.config.{ts,js,yml,yaml}",
];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "@graphql-codegen/cli",
    "@graphql-codegen/typescript",
    "@graphql-codegen/typescript-operations",
    "@graphql-codegen/typescript-react-query",
];

impl Plugin for GraphqlCodegenPlugin {
    fn name(&self) -> &'static str {
        "graphql-codegen"
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
