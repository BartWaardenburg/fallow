//! `TanStack` Router plugin.
//!
//! Detects `TanStack` Router projects and marks route files as entry points.

use super::Plugin;

const ENABLERS: &[&str] = &["@tanstack/react-router", "@tanstack/start"];

const ENTRY_PATTERNS: &[&str] = &[
    "src/routes/**/*.{ts,tsx,js,jsx}",
    "app/routes/**/*.{ts,tsx,js,jsx}",
    "src/routeTree.gen.ts",
];

const ALWAYS_USED: &[&str] = &["tsr.config.json"];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "@tanstack/react-router",
    "@tanstack/react-router-devtools",
    "@tanstack/start",
    "@tanstack/router-cli",
    "@tanstack/router-vite-plugin",
];

const ROUTE_EXPORTS: &[&str] = &[
    "default",
    "Route",
    "loader",
    "action",
    "component",
    "errorComponent",
    "pendingComponent",
    "notFoundComponent",
    "beforeLoad",
];

define_plugin! {
    struct TanstackRouterPlugin => "tanstack-router",
    enablers: ENABLERS,
    entry_patterns: ENTRY_PATTERNS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
    used_exports: [
        ("src/routes/**/*.{ts,tsx,js,jsx}", ROUTE_EXPORTS),
        ("app/routes/**/*.{ts,tsx,js,jsx}", ROUTE_EXPORTS),
    ],
}
