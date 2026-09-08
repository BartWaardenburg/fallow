//! Integration test for the caveat a file the run never READ has to raise.
//!
//! `source-parse-degraded` was the first door onto this hole and it was closed.
//! This is the second: the per-file size guard drops a file at discovery, so it
//! is never read, never parsed, has no `ModuleInfo` and no graph node, and the
//! import on its first line credits nothing. The module it imported then reads
//! as unused, and the export it imported reads as an unused export with an
//! auto-fixable `remove-export` action on it. `fallow fix` would strip that
//! export while the skipped file still imports it, breaking the build.
//!
//! The assertions are written on the observable outcome (the finding carries a
//! caveat) rather than on the size skip specifically, because the withholding
//! in `fallow fix` follows the caveat and every kind
//! `WorkspaceDiagnosticKind::source_never_analyzed` accepts raises it.

use std::path::Path;

use fallow_config::WorkspaceDiagnosticKind;
use fallow_types::output_dead_code::ReachabilityCaveat;

use super::common::create_config_with_cache;

/// A file whose FIRST line imports `target`, padded past `min_bytes` so source
/// discovery drops it before reading it.
fn write_oversized_importer(path: &Path, import_line: &str, min_bytes: usize) {
    let mut source = String::from(import_line);
    source.push('\n');
    source.push_str("export const pad = [\n");
    while source.len() < min_bytes {
        source.push_str("  \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\n");
    }
    source.push_str("];\n");
    std::fs::write(path, source).expect("write oversized module");
}

fn skipped_large_file_diagnostics(root: &Path) -> Vec<std::path::PathBuf> {
    fallow_config::workspace_diagnostics_for(root)
        .into_iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.kind,
                WorkspaceDiagnosticKind::SkippedLargeFile { .. }
            )
        })
        .map(|diagnostic| diagnostic.path)
        .collect()
}

fn config_with_small_size_limit(
    root: std::path::PathBuf,
    cache: std::path::PathBuf,
) -> fallow_config::ResolvedConfig {
    let mut config = create_config_with_cache(root, cache);
    config.max_file_size_bytes = Some(64 * 1024);
    config
}

/// `src/huge.ts` is the only importer of `src/lib.ts`, and the size guard means
/// the run never sees that import. `lib.ts` is reported unused with a
/// `delete-file` action, so the finding has to say the verdict rests on an
/// import graph the run knows is incomplete.
#[test]
fn a_file_only_a_skipped_file_imports_carries_the_caveat() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let root = temp.path().join("project");
    std::fs::create_dir_all(root.join("src")).expect("create project src");
    std::fs::write(
        root.join("package.json"),
        r#"{ "name": "skipped-importer", "version": "1.0.0", "main": "src/index.ts" }"#,
    )
    .expect("write manifest");
    std::fs::write(root.join("src/lib.ts"), "export const needed = 1;\n").expect("write library");
    write_oversized_importer(
        &root.join("src/huge.ts"),
        "import { needed } from \"./lib\";",
        128 * 1024,
    );
    std::fs::write(
        root.join("src/index.ts"),
        "import \"./huge\";\n\nexport const run = (): void => {};\n",
    )
    .expect("write entry module");
    let config = config_with_small_size_limit(root, temp.path().join("cache"));

    let results = fallow_core::analyze(&config).expect("analysis succeeds");

    let skipped = skipped_large_file_diagnostics(&config.root);
    assert_eq!(
        skipped.len(),
        1,
        "the size guard must have dropped exactly the oversized file, got {skipped:?}"
    );

    let library = results
        .unused_files
        .iter()
        .find(|issue| issue.file.path.ends_with("src/lib.ts"))
        .expect("the file only the skipped module imports is reported unused");
    assert_eq!(
        library.reachability_caveats,
        vec![ReachabilityCaveat::IncompleteImportGraph],
        "a file the run never read can hold the import that credits this one"
    );

    // Report-only stands: the caveat is advisory provenance, not a gate.
    assert_eq!(
        library.actions.len(),
        2,
        "the caveat must not withhold or trim the finding's actions"
    );
}

/// The auto-fixable variant, and the dangerous one. `lib.ts` is reachable from
/// the entry point, so only the export `huge.ts` imports reads as unused, and
/// `remove-export` is applied without confirmation by `fallow fix --yes`.
#[test]
fn an_export_only_a_skipped_file_imports_carries_the_caveat() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let root = temp.path().join("project");
    std::fs::create_dir_all(root.join("src")).expect("create project src");
    std::fs::write(
        root.join("package.json"),
        r#"{ "name": "skipped-export-importer", "version": "1.0.0", "main": "src/index.ts" }"#,
    )
    .expect("write manifest");
    std::fs::write(
        root.join("src/lib.ts"),
        "export const needed = 1;\nexport const alsoUsed = 2;\n",
    )
    .expect("write library");
    write_oversized_importer(
        &root.join("src/huge.ts"),
        "import { needed } from \"./lib\";",
        128 * 1024,
    );
    std::fs::write(
        root.join("src/index.ts"),
        "import \"./huge\";\nimport { alsoUsed } from \"./lib\";\n\nexport const run = (): number => alsoUsed;\n",
    )
    .expect("write entry module");
    let config = config_with_small_size_limit(root, temp.path().join("cache"));

    let results = fallow_core::analyze(&config).expect("analysis succeeds");

    let export = results
        .unused_exports
        .iter()
        .find(|issue| issue.export.export_name == "needed")
        .expect("the export only the skipped module imports is reported unused");
    assert_eq!(
        export.reachability_caveats,
        vec![ReachabilityCaveat::IncompleteImportGraph],
        "removing this export would break the file the run never read"
    );
}
