//! next-intl plugin.
//!
//! Detects next-intl projects and marks i18n config files as entry points.

use super::Plugin;

const ENABLERS: &[&str] = &["next-intl"];

const ENTRY_PATTERNS: &[&str] = &[
    "src/i18n.{ts,js}",
    "src/i18n/request.{ts,js}",
    "i18n.{ts,js}",
    "i18n/request.{ts,js}",
    "messages/**/*.json",
];

const ALWAYS_USED: &[&str] = &["messages/**/*.json"];

const TOOLING_DEPENDENCIES: &[&str] = &["next-intl"];

define_plugin! {
    struct NextIntlPlugin => "next-intl",
    enablers: ENABLERS,
    entry_patterns: ENTRY_PATTERNS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
