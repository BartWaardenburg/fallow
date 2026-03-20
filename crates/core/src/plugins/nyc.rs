//! nyc plugin.
//!
//! Detects nyc (Istanbul coverage) projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["nyc"];

const ALWAYS_USED: &[&str] = &[".nycrc", ".nycrc.{json,yml,yaml}"];

const TOOLING_DEPENDENCIES: &[&str] = &["nyc"];

define_plugin! {
    struct NycPlugin => "nyc",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
