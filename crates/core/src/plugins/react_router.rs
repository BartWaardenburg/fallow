//! React Router (v7+) framework plugin.
//!
//! Detects React Router projects and marks route files, root layout, and entry points.
//! Recognizes conventional route exports (loader, action, meta, etc.).

use super::Plugin;

pub struct ReactRouterPlugin;

const ENABLERS: &[&str] = &["@react-router/dev"];

const ENTRY_PATTERNS: &[&str] = &[
    "app/routes/**/*.{ts,tsx,js,jsx}",
    "app/root.{ts,tsx,js,jsx}",
    "app/entry.client.{ts,tsx,js,jsx}",
    "app/entry.server.{ts,tsx,js,jsx}",
];

const ALWAYS_USED: &[&str] = &["react-router.config.{ts,js}"];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "@react-router/dev",
    "@react-router/serve",
    "@react-router/node",
];

const ROUTE_EXPORTS: &[&str] = &[
    "default",
    "loader",
    "clientLoader",
    "action",
    "clientAction",
    "meta",
    "links",
    "headers",
    "handle",
    "ErrorBoundary",
    "HydrateFallback",
    "shouldRevalidate",
];

impl Plugin for ReactRouterPlugin {
    fn name(&self) -> &'static str {
        "react-router"
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
