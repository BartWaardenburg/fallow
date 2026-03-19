//! Lefthook git hooks plugin.
//!
//! Detects Lefthook projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["lefthook", "@evilmartians/lefthook"];

const ALWAYS_USED: &[&str] = &[
    "lefthook.yml",
    "lefthook.yaml",
    ".lefthook.yml",
    ".lefthook.yaml",
    "lefthook.toml",
    ".lefthook.toml",
    ".lefthook/**/*",
    ".lefthook-local/**/*",
];

const TOOLING_DEPENDENCIES: &[&str] = &["lefthook", "@evilmartians/lefthook"];

define_plugin! {
    struct LefthookPlugin => "lefthook",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
