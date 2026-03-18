//! Knex.js query builder plugin.
//!
//! Detects Knex projects and marks migration and seed files as entry points.

use super::Plugin;

pub struct KnexPlugin;

const ENABLERS: &[&str] = &["knex"];

const ENTRY_PATTERNS: &[&str] = &["migrations/**/*.{ts,js}", "seeds/**/*.{ts,js}"];

const ALWAYS_USED: &[&str] = &["knexfile.{ts,js}"];

const TOOLING_DEPENDENCIES: &[&str] = &["knex"];

impl Plugin for KnexPlugin {
    fn name(&self) -> &'static str {
        "knex"
    }

    fn enablers(&self) -> &'static [&'static str] {
        ENABLERS
    }

    fn entry_patterns(&self) -> &'static [&'static str] {
        ENTRY_PATTERNS
    }

    fn always_used(&self) -> &'static [&'static str] {
        ALWAYS_USED
    }

    fn tooling_dependencies(&self) -> &'static [&'static str] {
        TOOLING_DEPENDENCIES
    }
}
