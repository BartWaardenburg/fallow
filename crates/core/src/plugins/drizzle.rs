//! Drizzle ORM plugin.
//!
//! Detects Drizzle projects and marks migration/schema files as entry points.
//! Parses drizzle.config to extract referenced dependencies.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct DrizzlePlugin;

const ENABLERS: &[&str] = &["drizzle-orm"];

const ENTRY_PATTERNS: &[&str] = &["drizzle/**/*.{ts,js}"];

const CONFIG_PATTERNS: &[&str] = &["drizzle.config.{ts,js,mjs}"];

const ALWAYS_USED: &[&str] = &["drizzle.config.{ts,js,mjs}"];

const TOOLING_DEPENDENCIES: &[&str] = &["drizzle-orm", "drizzle-kit"];

impl Plugin for DrizzlePlugin {
    fn name(&self) -> &'static str {
        "drizzle"
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
