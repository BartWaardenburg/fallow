//! Biome linter/formatter plugin.
//!
//! Detects Biome projects and marks config files as always used.

use super::Plugin;

pub struct BiomePlugin;

const ENABLERS: &[&str] = &["@biomejs/biome"];

const ALWAYS_USED: &[&str] = &["biome.json", "biome.jsonc"];

const TOOLING_DEPENDENCIES: &[&str] = &["@biomejs/biome"];

impl Plugin for BiomePlugin {
    fn name(&self) -> &'static str {
        "biome"
    }

    fn enablers(&self) -> &'static [&'static str] {
        ENABLERS
    }

    fn always_used(&self) -> &'static [&'static str] {
        ALWAYS_USED
    }

    fn tooling_dependencies(&self) -> &'static [&'static str] {
        TOOLING_DEPENDENCIES
    }
}
