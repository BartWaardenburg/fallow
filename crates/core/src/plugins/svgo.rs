//! SVGO plugin.
//!
//! Detects SVGO projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["svgo"];

const ALWAYS_USED: &[&str] = &["svgo.config.{js,cjs,mjs,ts}"];

const TOOLING_DEPENDENCIES: &[&str] = &["svgo"];

define_plugin! {
    struct SvgoPlugin => "svgo",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
