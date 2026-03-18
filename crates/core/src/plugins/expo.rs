//! Expo framework plugin.
//!
//! Detects Expo projects and marks app entry points and config files.

use super::Plugin;

pub struct ExpoPlugin;

const ENABLERS: &[&str] = &["expo"];

const ENTRY_PATTERNS: &[&str] = &[
    "App.{ts,tsx,js,jsx}",
    "app/**/*.{ts,tsx,js,jsx}",
    "src/App.{ts,tsx,js,jsx}",
];

const ALWAYS_USED: &[&str] = &[
    "app.json",
    "app.config.{ts,js}",
    "metro.config.{ts,js}",
    "babel.config.{ts,js}",
];

const TOOLING_DEPENDENCIES: &[&str] = &["expo", "expo-cli", "@expo/webpack-config"];

impl Plugin for ExpoPlugin {
    fn name(&self) -> &'static str {
        "expo"
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
