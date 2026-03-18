//! Mocha test runner plugin.
//!
//! Detects Mocha projects and marks test files as entry points.

use super::Plugin;

pub struct MochaPlugin;

const ENABLERS: &[&str] = &["mocha"];

const ENTRY_PATTERNS: &[&str] = &[
    "test/**/*.{ts,tsx,js,jsx}",
    "tests/**/*.{ts,tsx,js,jsx}",
    "spec/**/*.{ts,tsx,js,jsx}",
    "**/*.test.{ts,tsx,js,jsx}",
    "**/*.spec.{ts,tsx,js,jsx}",
];

const ALWAYS_USED: &[&str] = &[".mocharc.{json,yaml,yml,js,cjs,mjs}"];

const TOOLING_DEPENDENCIES: &[&str] = &["mocha", "@types/mocha", "ts-mocha"];

impl Plugin for MochaPlugin {
    fn name(&self) -> &'static str {
        "mocha"
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
