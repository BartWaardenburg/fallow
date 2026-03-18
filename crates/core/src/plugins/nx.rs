//! Nx monorepo plugin.
//!
//! Detects Nx projects and marks workspace config files as always used.

use super::Plugin;

pub struct NxPlugin;

const ENABLERS: &[&str] = &["nx"];

const ALWAYS_USED: &[&str] = &["nx.json", "**/project.json"];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "nx",
    "@nx/workspace",
    "@nx/js",
    "@nx/react",
    "@nx/next",
    "@nx/node",
    "@nx/web",
    "@nx/vite",
    "@nx/jest",
    "@nx/eslint",
];

impl Plugin for NxPlugin {
    fn name(&self) -> &'static str {
        "nx"
    }

    fn enablers(&self) -> &'static [&'static str] {
        ENABLERS
    }

    fn always_used(&self) -> &'static [&'static str] {
        ALWAYS_USED
    }

    fn tooling_dependencies(&self) -> &'static [&'static str] {
        TOOLING_DEPENDENCIES
    }
}
