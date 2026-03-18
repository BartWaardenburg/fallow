//! Remix framework plugin.
//!
//! Detects Remix projects and marks route files, root layout, and entry points.
//! Recognizes conventional route exports (loader, action, meta, etc.).

use super::Plugin;

pub struct RemixPlugin;

const ENABLERS: &[&str] = &[
    "@remix-run/node",
    "@remix-run/react",
    "@remix-run/cloudflare",
    "@remix-run/cloudflare-pages",
    "@remix-run/deno",
];

const ENTRY_PATTERNS: &[&str] = &[
    "app/routes/**/*.{ts,tsx,js,jsx}",
    "app/root.{ts,tsx,js,jsx}",
    "app/entry.client.{ts,tsx,js,jsx}",
    "app/entry.server.{ts,tsx,js,jsx}",
];

const ALWAYS_USED: &[&str] = &["remix.config.{ts,js,mjs}"];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "@remix-run/dev",
    "@remix-run/node",
    "@remix-run/react",
    "@remix-run/cloudflare",
    "@remix-run/serve",
];

const ROUTE_EXPORTS: &[&str] = &[
    "default",
    "loader",
    "action",
    "meta",
    "links",
    "headers",
    "handle",
    "ErrorBoundary",
    "HydrateFallback",
];

impl Plugin for RemixPlugin {
    fn name(&self) -> &'static str {
        "remix"
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

    fn used_exports(&self) -> Vec<(&'static str, &'static [&'static str])> {
        vec![("app/routes/**/*.{ts,tsx,js,jsx}", ROUTE_EXPORTS)]
    }
}
