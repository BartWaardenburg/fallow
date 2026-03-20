//! Capacitor plugin.
//!
//! Detects Capacitor cross-platform mobile projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["@capacitor/core", "@capacitor/cli"];

const ALWAYS_USED: &[&str] = &["capacitor.config.{ts,js,json}"];

const TOOLING_DEPENDENCIES: &[&str] = &["@capacitor/core", "@capacitor/cli"];

define_plugin! {
    struct CapacitorPlugin => "capacitor",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
