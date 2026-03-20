//! markdownlint plugin.
//!
//! Detects markdownlint projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["markdownlint", "markdownlint-cli", "markdownlint-cli2"];

const ALWAYS_USED: &[&str] = &[
    ".markdownlint.{json,jsonc,yml,yaml}",
    ".markdownlint-cli2.{jsonc,yaml,cjs,mjs}",
];

const TOOLING_DEPENDENCIES: &[&str] = &["markdownlint", "markdownlint-cli", "markdownlint-cli2"];

define_plugin! {
    struct MarkdownlintPlugin => "markdownlint",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
