//! i18next plugin.
//!
//! Detects i18next projects and marks i18n setup and locale files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["i18next", "react-i18next", "vue-i18n"];

const ENTRY_PATTERNS: &[&str] = &[
    "src/i18n.{ts,js,mjs}",
    "src/i18n/index.{ts,js}",
    "i18n.{ts,js,mjs}",
    "i18n/index.{ts,js}",
];

const ALWAYS_USED: &[&str] = &[
    "src/i18n.{ts,js,mjs}",
    "src/i18n/index.{ts,js}",
    "i18n.{ts,js,mjs}",
    "i18n/index.{ts,js}",
    "i18next.config.{js,ts,mjs}",
    "next-i18next.config.{js,mjs}",
    "locales/**/*.json",
    "public/locales/**/*.json",
    "src/locales/**/*.json",
];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "i18next",
    "react-i18next",
    "vue-i18n",
    "i18next-parser",
    "i18next-scanner",
];

define_plugin! {
    struct I18nextPlugin => "i18next",
    enablers: ENABLERS,
    entry_patterns: ENTRY_PATTERNS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
