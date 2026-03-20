//! `TanStack` Router plugin.
//!
//! Detects `TanStack` Router projects and marks route files as entry points.

use super::Plugin;

const ENABLERS: &[&str] = &[
    "@tanstack/react-router",
    "@tanstack/start",
    "@tanstack/react-start",
];

const ENTRY_PATTERNS: &[&str] = &[
    "src/routes/**/*.{ts,tsx,js,jsx}",
    "app/routes/**/*.{ts,tsx,js,jsx}",
    "src/routeTree.gen.ts",
    "src/server.{ts,tsx}",
    "src/client.{ts,tsx}",
    "src/router.{ts,tsx}",
    "src/routes/__root.{ts,tsx}",
];

const ALWAYS_USED: &[&str] = &["tsr.config.json", "app.config.{ts,js}"];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "@tanstack/react-router",
    "@tanstack/react-router-devtools",
    "@tanstack/start",
    "@tanstack/react-start",
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
