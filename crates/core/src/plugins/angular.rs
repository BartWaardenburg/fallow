//! Angular framework plugin.
//!
//! Detects Angular projects and marks component, module, service, guard,
//! pipe, directive, resolver, and interceptor files as entry points.

use super::Plugin;

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
    "@angular/compiler",
    "zone.js",
    "tslib",
];

define_plugin! {
    struct AngularPlugin => "angular",
    enablers: ENABLERS,
    entry_patterns: ENTRY_PATTERNS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
