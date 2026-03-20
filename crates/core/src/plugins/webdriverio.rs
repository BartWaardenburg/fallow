//! `WebdriverIO` plugin.
//!
//! Detects `WebdriverIO` E2E testing projects and marks test files as entry points.

use super::Plugin;

const ENABLERS: &[&str] = &["webdriverio", "@wdio/"];

const ENTRY_PATTERNS: &[&str] = &["test/**/*.{ts,js}", "e2e/**/*.{ts,js}", "**/*.e2e.{ts,js}"];

const ALWAYS_USED: &[&str] = &["wdio.conf.{ts,js}"];

const TOOLING_DEPENDENCIES: &[&str] = &["webdriverio"];

define_plugin! {
    struct WebdriverioPlugin => "webdriverio",
    enablers: ENABLERS,
    entry_patterns: ENTRY_PATTERNS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
