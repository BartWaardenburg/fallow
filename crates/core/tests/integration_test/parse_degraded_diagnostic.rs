//! Integration test for the `source-parse-degraded` diagnostic.
//!
//! A file that does not parse is not a neutral event. Its imports credit
//! nothing, so every module it was the only consumer of becomes an
//! `unused-file` with a `delete-file` action, and the run gives no hint that
//! the verdict rests on a partial parse. These tests pin that the degradation
//! is now recorded, that it survives a warm cache, and that it never withholds
//! a finding: a broken file must still be analyzed as far as the parser got.

use std::path::Path;

use fallow_config::{WorkspaceDiagnostic, WorkspaceDiagnosticKind};
use fallow_types::output_dead_code::ReachabilityConfidenceFlag;

use super::common::create_config_with_cache;

fn degraded_diagnostics(root: &Path) -> Vec<WorkspaceDiagnostic> {
    fallow_config::workspace_diagnostics_for(root)
        .into_iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.kind,
                WorkspaceDiagnosticKind::SourceParseDegraded { .. }
            )
        })
        .collect()
}

/// Write a project whose entry file has an unclosed call expression.
fn write_broken_project(root: &Path) {
    std::fs::create_dir_all(root.join("src")).expect("create project src");
    std::fs::write(
        root.join("package.json"),
        r#"{ "name": "parse-degraded", "version": "1.0.0", "main": "src/index.ts" }"#,
    )
    .expect("write manifest");
    std::fs::write(
        root.join("src/helper.ts"),
        "export const helper = (): string => \"helper\";\n",
    )
    .expect("write helper module");
    std::fs::write(
        root.join("src/index.ts"),
        "import { helper } from \"./helper\";\n\nexport const run = (): string => {\n  return helper(\n};\n",
    )
    .expect("write broken entry module");
}

#[test]
fn a_file_that_fails_to_parse_is_reported_as_degraded() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let root = temp.path().join("project");
    write_broken_project(&root);
    let config = create_config_with_cache(root, temp.path().join("cache"));

    let results = fallow_core::analyze(&config).expect("analysis succeeds on a broken source");

    let degraded = degraded_diagnostics(&config.root);
    assert_eq!(
        degraded.len(),
        1,
        "exactly the broken file should be reported, got {degraded:?}"
    );
    assert!(
        degraded[0].path.ends_with("src/index.ts"),
        "the diagnostic must anchor at the file that failed to parse, got {:?}",
        degraded[0].path
    );
    let WorkspaceDiagnosticKind::SourceParseDegraded { error_count, .. } = degraded[0].kind else {
        panic!("expected a source-parse-degraded kind");
    };
    assert!(error_count > 0, "a degraded parse must report its errors");

    // The unusable import is exactly why this has to be reported: without the
    // diagnostic the run confidently offers to delete a file the source imports.
    assert!(
        results
            .unused_files
            .iter()
            .any(|issue| issue.file.path.ends_with("src/helper.ts")),
        "the finding itself is unchanged; the degradation is reported, never used to gate"
    );
}

#[test]
fn a_degraded_parse_is_still_reported_from_a_warm_cache() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let root = temp.path().join("project");
    write_broken_project(&root);
    let config = create_config_with_cache(root, temp.path().join("cache"));

    let _ = fallow_core::analyze(&config).expect("cold analysis succeeds");
    let _ = fallow_core::analyze(&config).expect("warm analysis succeeds");

    let degraded = degraded_diagnostics(&config.root);
    assert_eq!(
        degraded.len(),
        1,
        "a cache hit must not lose the degradation, got {degraded:?}"
    );
}

#[test]
fn a_project_that_parses_cleanly_reports_no_degradation() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let root = temp.path().join("project");
    write_broken_project(&root);
    std::fs::write(
        root.join("src/index.ts"),
        "import { helper } from \"./helper\";\n\nexport const run = (): string => helper();\n",
    )
    .expect("write valid entry module");
    let config = create_config_with_cache(root, temp.path().join("cache"));

    let results = fallow_core::analyze(&config).expect("analysis succeeds");

    assert!(
        degraded_diagnostics(&config.root).is_empty(),
        "a clean parse must not be reported as degraded"
    );
    assert!(
        results
            .unused_files
            .iter()
            .all(|issue| issue.confidence.is_empty())
            && results
                .unused_exports
                .iter()
                .all(|issue| issue.confidence.is_empty()),
        "a project that parses cleanly must carry no confidence marker anywhere"
    );
}

/// The diagnostic alone leaves the caveat at the top of the envelope while the
/// finding it distorts sits far away with a `delete-file` action on it. The
/// finding has to carry the caveat itself.
#[test]
fn a_finding_a_degraded_parse_can_distort_carries_the_caveat() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let root = temp.path().join("project");
    write_broken_project(&root);
    let config = create_config_with_cache(root, temp.path().join("cache"));

    let results = fallow_core::analyze(&config).expect("analysis succeeds on a broken source");

    let helper = results
        .unused_files
        .iter()
        .find(|issue| issue.file.path.ends_with("src/helper.ts"))
        .expect("the file the broken entry imports is still reported unused");
    assert_eq!(
        helper.confidence,
        vec![ReachabilityConfidenceFlag::IncompleteImportGraph],
        "the entry file that failed to parse is reachable, so the verdict on the file it \
         imports rests on an import graph fallow knows is incomplete"
    );

    // Report-only stands: the caveat is advisory provenance, not a gate.
    assert_eq!(
        helper.actions.len(),
        2,
        "the marker must not withhold or trim the finding's actions"
    );
}
