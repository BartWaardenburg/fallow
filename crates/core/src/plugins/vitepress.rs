//! `VitePress` plugin.
//!
//! Detects `VitePress` projects and marks config/theme files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["vitepress"];

const ENTRY_PATTERNS: &[&str] = &[
    ".vitepress/theme/index.{ts,js,mts,mjs}",
    ".vitepress/theme/**/*.{vue,ts,js}",
];

const ALWAYS_USED: &[&str] = &[
    ".vitepress/config.{ts,js,mts,mjs}",
    ".vitepress/theme/index.{ts,js,mts,mjs}",
];

const TOOLING_DEPENDENCIES: &[&str] = &["vitepress"];

define_plugin! {
    struct VitePressPlugin => "vitepress",
    enablers: ENABLERS,
    entry_patterns: ENTRY_PATTERNS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
