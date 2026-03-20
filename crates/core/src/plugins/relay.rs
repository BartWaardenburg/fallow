//! Relay plugin.
//!
//! Detects Relay GraphQL projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["relay-runtime", "react-relay", "relay-compiler"];

const ALWAYS_USED: &[&str] = &["relay.config.{js,json}"];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "relay-runtime",
    "relay-compiler",
    "babel-plugin-relay",
    "react-relay",
    "relay-test-utils",
];

define_plugin! {
    struct RelayPlugin => "relay",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
