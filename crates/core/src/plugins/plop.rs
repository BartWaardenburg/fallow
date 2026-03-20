//! Plop plugin.
//!
//! Detects Plop code generator projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["plop"];

const ALWAYS_USED: &[&str] = &["plopfile.{js,cjs,mjs,ts}"];

const TOOLING_DEPENDENCIES: &[&str] = &["plop"];

define_plugin! {
    struct PlopPlugin => "plop",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
