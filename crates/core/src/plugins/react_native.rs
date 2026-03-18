//! React Native plugin.
//!
//! Detects React Native projects and marks app entry points and
//! Metro/Babel config files as always used.

use super::Plugin;

pub struct ReactNativePlugin;

const ENABLERS: &[&str] = &["react-native"];

const ENTRY_PATTERNS: &[&str] = &[
    "index.{ts,tsx,js,jsx}",
    "App.{ts,tsx,js,jsx}",
    "src/App.{ts,tsx,js,jsx}",
    "app.config.{ts,js}",
];

const ALWAYS_USED: &[&str] = &[
    "metro.config.{ts,js}",
    "react-native.config.{ts,js}",
    "babel.config.{ts,js}",
    "app.json",
];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "react-native",
    "metro",
    "metro-config",
    "@react-native-community/cli",
    "@react-native/metro-config",
];

impl Plugin for ReactNativePlugin {
    fn name(&self) -> &'static str {
        "react-native"
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
