//! Cucumber plugin.
//!
//! Detects Cucumber BDD projects and marks step definitions as entry points.

use super::Plugin;

const ENABLERS: &[&str] = &["@cucumber/cucumber"];

const ENTRY_PATTERNS: &[&str] = &[
    "features/**/*.{ts,js}",
    "**/*.steps.{ts,js}",
    "step_definitions/**/*.{ts,js}",
    "support/**/*.{ts,js}",
];

const ALWAYS_USED: &[&str] = &["cucumber.{js,cjs,mjs}", ".cucumber.{js,yml}"];

const TOOLING_DEPENDENCIES: &[&str] = &["@cucumber/cucumber"];

define_plugin! {
    struct CucumberPlugin => "cucumber",
    enablers: ENABLERS,
    entry_patterns: ENTRY_PATTERNS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
