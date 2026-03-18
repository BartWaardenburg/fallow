//! Ava test runner plugin.
//!
//! Detects Ava projects and marks test files as entry points.

use super::Plugin;

pub struct AvaPlugin;

const ENABLERS: &[&str] = &["ava"];

const ENTRY_PATTERNS: &[&str] = &[
    "test/**/*.{ts,tsx,js,jsx}",
    "tests/**/*.{ts,tsx,js,jsx}",
    "**/*.test.{ts,tsx,js,jsx}",
    "**/*.spec.{ts,tsx,js,jsx}",
];

const ALWAYS_USED: &[&str] = &["ava.config.{js,cjs,mjs}"];

const TOOLING_DEPENDENCIES: &[&str] = &["ava", "@ava/typescript"];

impl Plugin for AvaPlugin {
    fn name(&self) -> &'static str {
        "ava"
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
