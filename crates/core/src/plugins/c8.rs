//! c8 plugin.
//!
//! Detects c8 (V8 coverage) projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["c8"];

const ALWAYS_USED: &[&str] = &[];

const TOOLING_DEPENDENCIES: &[&str] = &["c8"];

define_plugin! {
    struct C8Plugin => "c8",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
