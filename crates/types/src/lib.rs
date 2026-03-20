//! Shared types for the fallow dead code analyzer.
//!
//! This crate contains type definitions used across multiple fallow crates
//! (core, CLI, LSP). It has no analysis logic — only data structures.

pub mod discover;
pub mod extract;
pub mod results;
pub mod suppress;
