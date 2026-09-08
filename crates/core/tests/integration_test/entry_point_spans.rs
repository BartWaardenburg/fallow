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
///
/// The workspace fixture activates no plugins, so its glob set is empty and the
/// matching loop is skipped entirely. Asserting a positive match time here
/// measured nothing but the gap between two adjacent clock reads, and failed
/// whenever they landed in the same tick. What this fixture can prove is that
/// the two sub-spans stay inside the stage they subdivide and that neither one
/// borrows time it did not spend.
#[test]
fn plugin_glob_work_is_split_into_compile_and_match() {
    let config = create_config(fixture_path("workspace-project"));
    let output = fallow_core::analyze_with_trace(&config).expect("workspace analysis");
    let spans = output
        .timings
        .expect("trace timings retained")
        .entry_point_spans;

    assert!(
        spans.plugin_glob_build_ms >= 0.0 && spans.plugin_glob_match_ms >= 0.0,
        "sub-spans are elapsed times, not differences: {} compile, {} match",
        spans.plugin_glob_build_ms,
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

/// Files in the plugin-glob project. Enough that matching the compiled set
/// against them is real work rather than clock noise.
const PLUGIN_GLOB_PROJECT_FILES: usize = 300;

/// The split only means something on a project whose plugins contribute globs:
/// there, matching scales with files times patterns and is the half that grows.
/// A `vitest.config.ts` activates the test-runner plugin from the file alone,
/// so the project needs no installed dependencies to carry a real glob set.
#[test]
fn plugin_glob_match_time_is_reported_when_a_plugin_contributes_globs() {
    let project = tempfile::tempdir().expect("create project");
    let root = project.path();
    std::fs::write(
        root.join("package.json"),
        r#"{"name":"plugin-glob-project","version":"1.0.0"}"#,
    )
    .expect("write package.json");
    std::fs::write(
        root.join("vitest.config.ts"),
        "export default { test: { include: ['src/**/*.test.ts'] } };\n",
    )
    .expect("write vitest config");
    let src = root.join("src");
    std::fs::create_dir_all(&src).expect("create src");
    for index in 0..PLUGIN_GLOB_PROJECT_FILES {
        std::fs::write(
            src.join(format!("module{index}.ts")),
            format!("export const value{index} = {index};\n"),
        )
        .expect("write module");
    }

    let config = create_config(root.to_path_buf());
    let output = fallow_core::analyze_with_trace(&config).expect("plugin glob analysis");
    let spans = output
        .timings
        .expect("trace timings retained")
        .entry_point_spans;

    assert!(
        spans.plugin_glob_match_ms > 0.0,
        "matching {PLUGIN_GLOB_PROJECT_FILES} files against an active plugin glob set is timeable \
         work, got {}ms",
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
