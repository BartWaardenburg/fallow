//! Commitizen plugin.
//!
//! Detects Commitizen projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["commitizen"];

const ALWAYS_USED: &[&str] = &[".czrc", ".cz.json"];

const TOOLING_DEPENDENCIES: &[&str] = &["commitizen", "cz-conventional-changelog"];

define_plugin! {
    struct CommitizenPlugin => "commitizen",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
