//! SWC plugin.
//!
//! Detects SWC projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["@swc/core", "@swc/cli"];

const ALWAYS_USED: &[&str] = &[".swcrc", "swc.config.{js,cjs,mjs,ts}"];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "@swc/core",
    "@swc/cli",
    "@swc/jest",
    "@swc/register",
    "swc-loader",
];

define_plugin! {
    struct SwcPlugin => "swc",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
