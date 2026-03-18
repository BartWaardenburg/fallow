//! TypeScript plugin.
//!
//! Detects TypeScript projects and marks tsconfig files as always used.

use super::Plugin;

pub struct TypeScriptPlugin;

const ENABLERS: &[&str] = &["typescript"];

const ALWAYS_USED: &[&str] = &["tsconfig.json", "tsconfig.*.json"];

const TOOLING_DEPENDENCIES: &[&str] = &["typescript", "ts-node", "tsx", "ts-loader"];

impl Plugin for TypeScriptPlugin {
    fn name(&self) -> &'static str {
        "typescript"
    }

    fn enablers(&self) -> &'static [&'static str] {
        ENABLERS
    }

    fn always_used(&self) -> &'static [&'static str] {
        ALWAYS_USED
    }

    fn tooling_dependencies(&self) -> &'static [&'static str] {
        TOOLING_DEPENDENCIES
    }
}
