//! Karma plugin.
//!
//! Detects Karma test runner projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["karma"];

const ALWAYS_USED: &[&str] = &["karma.conf.{js,ts}"];

const TOOLING_DEPENDENCIES: &[&str] = &["karma"];

define_plugin! {
    struct KarmaPlugin => "karma",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
