//! Oxlint plugin.
//!
//! Detects Oxlint projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["oxlint"];

const ALWAYS_USED: &[&str] = &[".oxlintrc.json"];

const TOOLING_DEPENDENCIES: &[&str] = &["oxlint"];

define_plugin! {
    struct OxlintPlugin => "oxlint",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
