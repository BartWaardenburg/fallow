//! Turborepo monorepo build system plugin.
//!
//! Detects Turborepo projects and marks turbo.json as always used.

use super::Plugin;

pub struct TurborepoPlugin;

const ENABLERS: &[&str] = &["turbo"];

const ALWAYS_USED: &[&str] = &["turbo.json"];

const TOOLING_DEPENDENCIES: &[&str] = &["turbo"];

impl Plugin for TurborepoPlugin {
    fn name(&self) -> &'static str {
        "turborepo"
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
