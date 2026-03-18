//! Changesets versioning plugin.
//!
//! Detects Changesets projects and marks config files as always used.

use super::Plugin;

pub struct ChangesetsPlugin;

const ENABLERS: &[&str] = &["@changesets/cli"];

const ALWAYS_USED: &[&str] = &[".changeset/config.json"];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "@changesets/cli",
    "@changesets/changelog-github",
    "@changesets/changelog-git",
];

impl Plugin for ChangesetsPlugin {
    fn name(&self) -> &'static str {
        "changesets"
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
