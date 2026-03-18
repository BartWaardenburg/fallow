//! Angular framework plugin.
//!
//! Detects Angular projects and marks component, module, service, guard,
//! pipe, directive, resolver, and interceptor files as entry points.

use super::Plugin;

pub struct AngularPlugin;

const ENABLERS: &[&str] = &["@angular/core"];

const ENTRY_PATTERNS: &[&str] = &[
    "src/main.ts",
    "src/app/**/*.component.ts",
    "src/app/**/*.module.ts",
    "src/app/**/*.service.ts",
    "src/app/**/*.guard.ts",
    "src/app/**/*.pipe.ts",
    "src/app/**/*.directive.ts",
    "src/app/**/*.resolver.ts",
    "src/app/**/*.interceptor.ts",
];

const ALWAYS_USED: &[&str] = &[
    "angular.json",
    "src/polyfills.ts",
    "src/environments/**/*.ts",
];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "@angular/cli",
    "@angular-devkit/build-angular",
    "@angular/compiler-cli",
];

impl Plugin for AngularPlugin {
    fn name(&self) -> &'static str {
        "angular"
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
