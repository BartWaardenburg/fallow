//! Module graph construction and import resolution for the fallow codebase analyzer.
//!
//! This crate builds the dependency graph from parsed modules, resolves import
//! specifiers to their targets, and tracks export usage through re-export chains.

#![warn(missing_docs)]

pub mod graph;
pub mod project;
pub mod resolve;
