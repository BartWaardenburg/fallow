//! NestJS backend framework plugin.
//!
//! Detects NestJS projects and marks module, controller, service, guard,
//! interceptor, pipe, filter, middleware, gateway, and resolver files as entry points.

use super::Plugin;

pub struct NestJsPlugin;

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
];

impl Plugin for NestJsPlugin {
    fn name(&self) -> &'static str {
        "nestjs"
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
