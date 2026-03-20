//! Bun plugin.
//!
//! Detects Bun runtime projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["bun-types"];

const ALWAYS_USED: &[&str] = &["bunfig.toml"];

const TOOLING_DEPENDENCIES: &[&str] = &["bun-types"];

define_plugin! {
    struct BunPlugin => "bun",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
