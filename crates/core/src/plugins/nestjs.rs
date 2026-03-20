//! `NestJS` backend framework plugin.
//!
//! Detects `NestJS` projects and marks module, controller, service, guard,
//! interceptor, pipe, filter, middleware, gateway, and resolver files as entry points.

use super::Plugin;

const ENABLERS: &[&str] = &["@nestjs/core"];

const ENTRY_PATTERNS: &[&str] = &[
    "src/main.ts",
    "src/**/*.module.ts",
    "src/**/*.controller.ts",
    "src/**/*.service.ts",
    "src/**/*.guard.ts",
    "src/**/*.interceptor.ts",
    "src/**/*.pipe.ts",
    "src/**/*.filter.ts",
    "src/**/*.middleware.ts",
    "src/**/*.decorator.ts",
    "src/**/*.gateway.ts",
    "src/**/*.resolver.ts",
];

const ALWAYS_USED: &[&str] = &["nest-cli.json"];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "@nestjs/core",
    "@nestjs/common",
    "@nestjs/cli",
    "@nestjs/testing",
    "@nestjs/platform-express",
    "@nestjs/platform-fastify",
    "@nestjs/swagger",
    "@nestjs/config",
    "@nestjs/typeorm",
    "@nestjs/mongoose",
    "reflect-metadata",
];

define_plugin! {
    struct NestJsPlugin => "nestjs",
    enablers: ENABLERS,
    entry_patterns: ENTRY_PATTERNS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
