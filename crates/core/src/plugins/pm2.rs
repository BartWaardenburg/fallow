//! PM2 plugin.
//!
//! Detects PM2 projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["pm2"];

const ALWAYS_USED: &[&str] = &["ecosystem.config.{js,cjs}", "pm2.config.{js,cjs}"];

const TOOLING_DEPENDENCIES: &[&str] = &["pm2"];

define_plugin! {
    struct Pm2Plugin => "pm2",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
