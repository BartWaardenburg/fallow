//! Shared types for the fallow codebase analyzer.
//!
//! This crate contains type definitions used across multiple fallow crates
//! (core, CLI, LSP). It has no analysis logic — only data structures.

#![warn(missing_docs)]

/// File discovery types: discovered files, file IDs, and entry points.
pub mod discover;
/// Module extraction types: exports, imports, re-exports, and member info.
pub mod extract;
/// Analysis result types: unused files, exports, dependencies, and members.
pub mod results;
/// Custom serde serializers for cross-platform path output.
pub mod serde_path;
/// Inline suppression comment types and issue kind definitions.
pub mod suppress;
