//! Wrangler / Cloudflare Workers plugin.
//!
//! Detects Cloudflare Workers projects and marks worker entry points
//! and config files.

use super::Plugin;

pub struct WranglerPlugin;

const ENABLERS: &[&str] = &["wrangler"];

const ENTRY_PATTERNS: &[&str] = &[
    "src/index.{ts,js}",
    "src/worker.{ts,js}",
    "functions/**/*.{ts,js}",
];

const ALWAYS_USED: &[&str] = &["wrangler.toml", "wrangler.json", "wrangler.jsonc"];

const TOOLING_DEPENDENCIES: &[&str] = &["wrangler", "@cloudflare/workers-types"];

impl Plugin for WranglerPlugin {
    fn name(&self) -> &'static str {
        "wrangler"
    }

    fn enablers(&self) -> &'static [&'static str] {
        ENABLERS
    }

    fn entry_patterns(&self) -> &'static [&'static str] {
        ENTRY_PATTERNS
    }

    fn always_used(&self) -> &'static [&'static str] {
        ALWAYS_USED
    }

    fn tooling_dependencies(&self) -> &'static [&'static str] {
        TOOLING_DEPENDENCIES
    }
}
