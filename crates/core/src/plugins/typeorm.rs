//! `TypeORM` plugin.
//!
//! Detects `TypeORM` projects and marks entity, migration, and config files as entry points.

use super::Plugin;

const ENABLERS: &[&str] = &["typeorm"];

const ENTRY_PATTERNS: &[&str] = &[
    "src/entity/**/*.{ts,js}",
    "src/entities/**/*.{ts,js}",
    "src/migration/**/*.{ts,js}",
    "src/migrations/**/*.{ts,js}",
    "src/subscriber/**/*.{ts,js}",
    "src/subscribers/**/*.{ts,js}",
    "entity/**/*.{ts,js}",
    "entities/**/*.{ts,js}",
    "migration/**/*.{ts,js}",
    "migrations/**/*.{ts,js}",
    "subscriber/**/*.{ts,js}",
    "subscribers/**/*.{ts,js}",
];

const ALWAYS_USED: &[&str] = &[
    "ormconfig.{json,js,ts,yml,yaml}",
    "data-source.{ts,js}",
    "src/data-source.{ts,js}",
];

const TOOLING_DEPENDENCIES: &[&str] = &["typeorm"];

define_plugin! {
    struct TypeormPlugin => "typeorm",
    enablers: ENABLERS,
    entry_patterns: ENTRY_PATTERNS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
