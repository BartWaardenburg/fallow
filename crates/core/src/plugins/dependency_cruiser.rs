//! dependency-cruiser plugin.
//!
//! Detects dependency-cruiser projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["dependency-cruiser"];

const ALWAYS_USED: &[&str] = &[".dependency-cruiser.{js,cjs,mjs,json}"];

const TOOLING_DEPENDENCIES: &[&str] = &["dependency-cruiser"];

define_plugin! {
    struct DependencyCruiserPlugin => "dependency-cruiser",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
