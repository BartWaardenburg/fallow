//! Syncpack plugin.
//!
//! Detects Syncpack projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["syncpack"];

const ALWAYS_USED: &[&str] = &[".syncpackrc", ".syncpackrc.{json,js,cjs}"];

const TOOLING_DEPENDENCIES: &[&str] = &["syncpack"];

define_plugin! {
    struct SyncpackPlugin => "syncpack",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
