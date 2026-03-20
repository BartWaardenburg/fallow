//! Prettier plugin.
//!
//! Detects Prettier projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["prettier"];

const ALWAYS_USED: &[&str] = &[
    ".prettierrc",
    ".prettierrc.{json,json5,yml,yaml,js,cjs,mjs,ts,cts,toml}",
    "prettier.config.{js,cjs,mjs,ts,cts}",
    ".prettierignore",
];

const TOOLING_DEPENDENCIES: &[&str] = &["prettier"];

define_plugin! {
    struct PrettierPlugin => "prettier",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
