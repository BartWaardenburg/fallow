//! Electron plugin.
//!
//! Detects Electron projects and marks main/preload entry points and
//! build tool config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &[
    "electron",
    "electron-builder",
    "@electron-forge/cli",
    "electron-vite",
];

const ENTRY_PATTERNS: &[&str] = &["src/main/**/*.{ts,js}", "src/preload/**/*.{ts,js}"];

const ALWAYS_USED: &[&str] = &[
    "electron-builder.{yml,yaml,json,json5,toml}",
    "forge.config.{ts,js,cjs}",
    "electron.vite.config.{ts,js,mjs}",
];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "electron",
    "electron-builder",
    "electron-vite",
    "@electron/rebuild",
];

define_plugin! {
    struct ElectronPlugin => "electron",
    enablers: ENABLERS,
    entry_patterns: ENTRY_PATTERNS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
