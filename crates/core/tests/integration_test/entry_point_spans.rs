//! Attribution gate for the entry-point discovery stage.
//!
//! `PipelineTimings::entry_points_ms` used to be a single opaque number, so a
//! slow discovery stage could only be guessed at. These tests assert that the
//! sub-spans are a real partition of that stage: they are populated on a run
//! that does discovery work, they never exceed the stage they subdivide, and a
//! section that did not run reports zero rather than borrowing another
//! section's time.

use fallow_types::trace::EntryPointSpans;

use super::common::{create_config, fixture_path};

fn spans_total(spans: EntryPointSpans) -> f64 {
    spans.root_ms
        + spans.workspaces_ms
        + spans.plugins_ms
        + spans.infrastructure_ms
        + spans.dynamic_ms
        + spans.dedup_ms
}

#[test]
fn entry_point_spans_partition_the_stage_they_subdivide() {
    let config = create_config(fixture_path("workspace-project"));
    let output = fallow_core::analyze_with_trace(&config).expect("workspace analysis");
    let timings = output.timings.expect("trace timings retained");
    let spans = timings.entry_point_spans;

    assert!(
        spans.root_ms > 0.0,
        "root discovery always runs, got {}ms",
        spans.root_ms
    );
    assert!(
        spans_total(spans) <= timings.entry_points_ms,
        "sub-spans must partition the stage: {} sub-span ms vs {} stage ms",
        spans_total(spans),
        timings.entry_points_ms
    );
    assert!(
        timings.entry_points_ms > 0.0,
        "the stage itself must still be timed"
    );
}

/// Plugin glob work dominates the stage on every real project measured, so the
/// report has to say which half of it paid: compiling the pattern set once, or
/// matching it against every discovered file.
#[test]
fn plugin_glob_work_is_split_into_compile_and_match() {
    let config = create_config(fixture_path("workspace-project"));
    let output = fallow_core::analyze_with_trace(&config).expect("workspace analysis");
    let spans = output
        .timings
        .expect("trace timings retained")
        .entry_point_spans;

    assert!(
        spans.plugin_glob_match_ms > 0.0,
        "matching runs on every project, got {}ms",
        spans.plugin_glob_match_ms
    );
    assert!(
        spans.plugin_glob_build_ms + spans.plugin_glob_match_ms <= spans.plugins_ms,
        "compile plus match must fit inside the plugin span: {} + {} vs {}",
        spans.plugin_glob_build_ms,
        spans.plugin_glob_match_ms,
        spans.plugins_ms
    );
}

#[test]
fn unconfigured_dynamic_globs_report_zero_rather_than_borrowed_time() {
    let config = create_config(fixture_path("workspace-project"));
    assert!(
        config.dynamically_loaded.is_empty(),
        "fixture must not configure dynamically-loaded globs"
    );

    let output = fallow_core::analyze_with_trace(&config).expect("workspace analysis");
    let timings = output.timings.expect("trace timings retained");

    assert!(
        timings.entry_point_spans.dynamic_ms < 1.0,
        "a section that never ran must not carry another section's time, got {}ms",
        timings.entry_point_spans.dynamic_ms
    );
}
