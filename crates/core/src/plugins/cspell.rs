//! `CSpell` plugin.
//!
//! Detects `CSpell` projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["cspell"];

const ALWAYS_USED: &[&str] = &[
    ".cspell.{json,jsonc,yml,yaml}",
    "cspell.{json,jsonc}",
    "cspell.config.{js,cjs,mjs,yaml,yml}",
];

const TOOLING_DEPENDENCIES: &[&str] = &["cspell"];

define_plugin! {
    struct CspellPlugin => "cspell",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
