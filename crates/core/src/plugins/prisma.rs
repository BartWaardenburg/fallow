//! Prisma ORM plugin.
//!
//! Detects Prisma projects and marks seed files as entry points
//! and schema files as always used.

use super::Plugin;

pub struct PrismaPlugin;

const ENABLERS: &[&str] = &["prisma", "@prisma/client"];

const ENTRY_PATTERNS: &[&str] = &["prisma/seed.{ts,js}"];

const ALWAYS_USED: &[&str] = &["prisma/schema.prisma"];

const TOOLING_DEPENDENCIES: &[&str] = &["prisma", "@prisma/client"];

impl Plugin for PrismaPlugin {
    fn name(&self) -> &'static str {
        "prisma"
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
