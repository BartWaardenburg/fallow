//! SVGR plugin.
//!
//! Detects SVGR projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &[
    "@svgr/core",
    "@svgr/cli",
    "@svgr/webpack",
    "@svgr/rollup",
    "@svgr/vite",
];

const ALWAYS_USED: &[&str] = &[
    ".svgrrc",
    ".svgrrc.{js,json}",
    "svgr.config.{js,cjs,mjs,ts}",
];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "@svgr/core",
    "@svgr/cli",
    "@svgr/webpack",
    "@svgr/rollup",
    "@svgr/vite",
];

define_plugin! {
    struct SvgrPlugin => "svgr",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
