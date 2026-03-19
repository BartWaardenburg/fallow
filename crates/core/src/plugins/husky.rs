//! Husky git hooks plugin.
//!
//! Detects Husky projects and marks hook files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["husky"];

const ALWAYS_USED: &[&str] = &[".husky/**/*"];

const TOOLING_DEPENDENCIES: &[&str] = &["husky"];

define_plugin! {
    struct HuskyPlugin => "husky",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
