//! simple-git-hooks plugin.
//!
//! Detects simple-git-hooks projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["simple-git-hooks"];

const ALWAYS_USED: &[&str] = &["simple-git-hooks.{js,cjs}"];

const TOOLING_DEPENDENCIES: &[&str] = &["simple-git-hooks"];

define_plugin! {
    struct SimpleGitHooksPlugin => "simple-git-hooks",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
