#![expect(clippy::print_stdout, clippy::print_stderr)]

/// Metric and rule definitions for explainable CLI output.
pub mod explain;

/// Health / complexity analysis report types.
pub mod health_types;

/// Regression detection: baseline comparison and tolerance checking.
pub mod regression;

/// Report formatting utilities for analysis results.
///
/// Exposed for snapshot testing of output formats.
pub mod report;
