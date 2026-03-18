//! Tailwind CSS plugin.
//!
//! Detects Tailwind projects and marks config files as always used.

use super::Plugin;

pub struct TailwindPlugin;

const ENABLERS: &[&str] = &["tailwindcss", "@tailwindcss/postcss"];

const ALWAYS_USED: &[&str] = &["tailwind.config.{ts,js,cjs,mjs}"];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "tailwindcss",
    "@tailwindcss/postcss",
    "@tailwindcss/typography",
    "@tailwindcss/forms",
    "autoprefixer",
];

impl Plugin for TailwindPlugin {
    fn name(&self) -> &'static str {
        "tailwind"
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
