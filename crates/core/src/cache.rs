//! Re-exports from `fallow-extract::cache`.
//!
//! The cache module has been moved to the `fallow-extract` crate since it is
//! tightly coupled with the parsing/extraction pipeline. This module provides
//! backwards-compatible re-exports so that `fallow_core::cache::*` paths
//! continue to resolve.

pub use fallow_extract::cache::*;
