mod package_json_utils;
mod predicates;
mod unused_deps;
mod unused_exports;
mod unused_files;
mod unused_members;

use rustc_hash::FxHashMap;

use fallow_config::{PackageJson, ResolvedConfig, Severity};

use crate::discover::FileId;
use crate::extract::ModuleInfo;
use crate::graph::ModuleGraph;
use crate::resolve::ResolvedModule;
use crate::results::*;
use crate::suppress::{self, IssueKind, Suppression};

use unused_deps::{
    find_type_only_dependencies, find_unlisted_dependencies, find_unresolved_imports,
    find_unused_dependencies,
};
use unused_exports::{collect_export_usages, find_duplicate_exports, find_unused_exports};
use unused_files::find_unused_files;
use unused_members::find_unused_members;

/// Pre-computed line offset tables indexed by `FileId`, built during parse and
/// carried through the cache. Eliminates redundant file reads during analysis.
pub(crate) type LineOffsetsMap<'a> = FxHashMap<FileId, &'a [u32]>;

/// Convert a byte offset to (line, col) using pre-computed line offsets.
/// Falls back to `(1, byte_offset)` when no line table is available.
pub(crate) fn byte_offset_to_line_col(
    line_offsets_map: &LineOffsetsMap<'_>,
    file_id: FileId,
    byte_offset: u32,
) -> (u32, u32) {
    line_offsets_map
        .get(&file_id)
        .map_or((1, byte_offset), |offsets| {
            fallow_types::extract::byte_offset_to_line_col(offsets, byte_offset)
        })
}

/// Read source content from disk, returning empty string on failure.
/// Only used for LSP Code Lens reference resolution where the referencing
/// file may not be in the line offsets map.
fn read_source(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

/// Find all dead code in the project.
pub fn find_dead_code(graph: &ModuleGraph, config: &ResolvedConfig) -> AnalysisResults {
    find_dead_code_with_resolved(graph, config, &[], None)
}

/// Find all dead code, with optional resolved module data and plugin context.
pub fn find_dead_code_with_resolved(
    graph: &ModuleGraph,
    config: &ResolvedConfig,
    resolved_modules: &[ResolvedModule],
    plugin_result: Option<&crate::plugins::AggregatedPluginResult>,
) -> AnalysisResults {
    find_dead_code_full(
        graph,
        config,
        resolved_modules,
        plugin_result,
        &[],
        &[],
        false,
    )
}

/// Find all dead code, with optional resolved module data, plugin context, and workspace info.
pub fn find_dead_code_full(
    graph: &ModuleGraph,
    config: &ResolvedConfig,
    resolved_modules: &[ResolvedModule],
    plugin_result: Option<&crate::plugins::AggregatedPluginResult>,
    workspaces: &[fallow_config::WorkspaceInfo],
    modules: &[ModuleInfo],
    collect_usages: bool,
) -> AnalysisResults {
    let _span = tracing::info_span!("find_dead_code").entered();

    // Build suppression index: FileId -> suppressions
    let suppressions_by_file: FxHashMap<FileId, &[Suppression]> = modules
        .iter()
        .filter(|m| !m.suppressions.is_empty())
        .map(|m| (m.file_id, m.suppressions.as_slice()))
        .collect();

    // Build line offset index: FileId -> pre-computed line start offsets.
    // Eliminates redundant file reads for byte-to-line/col conversion.
    let line_offsets_by_file: LineOffsetsMap<'_> = modules
        .iter()
        .filter(|m| !m.line_offsets.is_empty())
        .map(|m| (m.file_id, m.line_offsets.as_slice()))
        .collect();

    let mut results = AnalysisResults::default();

    if config.rules.unused_files != Severity::Off {
        results.unused_files = find_unused_files(graph, &suppressions_by_file);
    }

    if config.rules.unused_exports != Severity::Off || config.rules.unused_types != Severity::Off {
        let (exports, types) = find_unused_exports(
            graph,
            config,
            plugin_result,
            &suppressions_by_file,
            &line_offsets_by_file,
        );
        if config.rules.unused_exports != Severity::Off {
            results.unused_exports = exports;
        }
        if config.rules.unused_types != Severity::Off {
            results.unused_types = types;
        }
    }

    if config.rules.unused_enum_members != Severity::Off
        || config.rules.unused_class_members != Severity::Off
    {
        let (enum_members, class_members) = find_unused_members(
            graph,
            config,
            resolved_modules,
            &suppressions_by_file,
            &line_offsets_by_file,
        );
        if config.rules.unused_enum_members != Severity::Off {
            results.unused_enum_members = enum_members;
        }
        if config.rules.unused_class_members != Severity::Off {
            results.unused_class_members = class_members;
        }
    }

    // Build merged dependency set from root + all workspace package.json files
    let pkg_path = config.root.join("package.json");
    let pkg = PackageJson::load(&pkg_path).ok();
    if let Some(ref pkg) = pkg {
        if config.rules.unused_dependencies != Severity::Off
            || config.rules.unused_dev_dependencies != Severity::Off
            || config.rules.unused_optional_dependencies != Severity::Off
        {
            let (deps, dev_deps, optional_deps) =
                find_unused_dependencies(graph, pkg, config, plugin_result, workspaces);
            if config.rules.unused_dependencies != Severity::Off {
                results.unused_dependencies = deps;
            }
            if config.rules.unused_dev_dependencies != Severity::Off {
                results.unused_dev_dependencies = dev_deps;
            }
            if config.rules.unused_optional_dependencies != Severity::Off {
                results.unused_optional_dependencies = optional_deps;
            }
        }

        if config.rules.unlisted_dependencies != Severity::Off {
            results.unlisted_dependencies = find_unlisted_dependencies(
                graph,
                pkg,
                config,
                workspaces,
                plugin_result,
                resolved_modules,
                &line_offsets_by_file,
            );
        }
    }

    if config.rules.unresolved_imports != Severity::Off && !resolved_modules.is_empty() {
        let virtual_prefixes: Vec<&str> = plugin_result
            .map(|pr| {
                pr.virtual_module_prefixes
                    .iter()
                    .map(|s| s.as_str())
                    .collect()
            })
            .unwrap_or_default();
        results.unresolved_imports = find_unresolved_imports(
            resolved_modules,
            config,
            &suppressions_by_file,
            &virtual_prefixes,
            &line_offsets_by_file,
        );
    }

    if config.rules.duplicate_exports != Severity::Off {
        results.duplicate_exports =
            find_duplicate_exports(graph, config, &suppressions_by_file, &line_offsets_by_file);
    }

    // In production mode, detect dependencies that are only used via type-only imports
    if config.production
        && let Some(ref pkg) = pkg
    {
        results.type_only_dependencies =
            find_type_only_dependencies(graph, pkg, config, workspaces);
    }

    // Detect circular dependencies
    if config.rules.circular_dependencies != Severity::Off {
        let cycles = graph.find_cycles();
        results.circular_dependencies = cycles
            .into_iter()
            .filter(|cycle| {
                // Skip cycles where any participating file has a file-level suppression
                !cycle.iter().any(|&id| {
                    suppressions_by_file.get(&id).is_some_and(|supps| {
                        suppress::is_file_suppressed(supps, IssueKind::CircularDependency)
                    })
                })
            })
            .map(|cycle| {
                let files: Vec<std::path::PathBuf> = cycle
                    .iter()
                    .map(|&id| graph.modules[id.0 as usize].path.clone())
                    .collect();
                let length = files.len();
                // Look up the import span from cycle[0] → cycle[1] for precise location
                let (line, col) = if cycle.len() >= 2 {
                    graph
                        .find_import_span_start(cycle[0], cycle[1])
                        .map_or((1, 0), |span_start| {
                            byte_offset_to_line_col(&line_offsets_by_file, cycle[0], span_start)
                        })
                } else {
                    (1, 0)
                };
                CircularDependency {
                    files,
                    length,
                    line,
                    col,
                }
            })
            .collect();
    }

    // Collect export usage counts for Code Lens (LSP feature).
    // Skipped in CLI mode since the field is #[serde(skip)] in all output formats.
    if collect_usages {
        results.export_usages = collect_export_usages(graph, &line_offsets_by_file);
    }

    results
}

#[cfg(test)]
mod tests {
    use fallow_types::extract::{byte_offset_to_line_col, compute_line_offsets};

    // Helper: compute line offsets from source and convert byte offset
    fn line_col(source: &str, byte_offset: u32) -> (u32, u32) {
        let offsets = compute_line_offsets(source);
        byte_offset_to_line_col(&offsets, byte_offset)
    }

    // ── compute_line_offsets ─────────────────────────────────────

    #[test]
    fn compute_offsets_empty() {
        assert_eq!(compute_line_offsets(""), vec![0]);
    }

    #[test]
    fn compute_offsets_single_line() {
        assert_eq!(compute_line_offsets("hello"), vec![0]);
    }

    #[test]
    fn compute_offsets_multiline() {
        assert_eq!(compute_line_offsets("abc\ndef\nghi"), vec![0, 4, 8]);
    }

    #[test]
    fn compute_offsets_trailing_newline() {
        assert_eq!(compute_line_offsets("abc\n"), vec![0, 4]);
    }

    #[test]
    fn compute_offsets_crlf() {
        assert_eq!(compute_line_offsets("ab\r\ncd"), vec![0, 4]);
    }

    #[test]
    fn compute_offsets_consecutive_newlines() {
        assert_eq!(compute_line_offsets("\n\n"), vec![0, 1, 2]);
    }

    // ── byte_offset_to_line_col ─────────────────────────────────

    #[test]
    fn byte_offset_empty_source() {
        assert_eq!(line_col("", 0), (1, 0));
    }

    #[test]
    fn byte_offset_single_line_start() {
        assert_eq!(line_col("hello", 0), (1, 0));
    }

    #[test]
    fn byte_offset_single_line_middle() {
        assert_eq!(line_col("hello", 4), (1, 4));
    }

    #[test]
    fn byte_offset_multiline_start_of_line2() {
        assert_eq!(line_col("line1\nline2\nline3", 6), (2, 0));
    }

    #[test]
    fn byte_offset_multiline_middle_of_line3() {
        assert_eq!(line_col("line1\nline2\nline3", 14), (3, 2));
    }

    #[test]
    fn byte_offset_at_newline_boundary() {
        assert_eq!(line_col("line1\nline2", 5), (1, 5));
    }

    #[test]
    fn byte_offset_multibyte_utf8() {
        let source = "hi\n\u{1F600}x";
        assert_eq!(line_col(source, 3), (2, 0));
        assert_eq!(line_col(source, 7), (2, 4));
    }

    #[test]
    fn byte_offset_multibyte_accented_chars() {
        let source = "caf\u{00E9}\nbar";
        assert_eq!(line_col(source, 6), (2, 0));
        assert_eq!(line_col(source, 3), (1, 3));
    }

    #[test]
    fn byte_offset_via_map_fallback() {
        use super::*;
        let map: LineOffsetsMap<'_> = FxHashMap::default();
        assert_eq!(
            super::byte_offset_to_line_col(&map, FileId(99), 42),
            (1, 42)
        );
    }

    #[test]
    fn byte_offset_via_map_lookup() {
        use super::*;
        let offsets = compute_line_offsets("abc\ndef\nghi");
        let mut map: LineOffsetsMap<'_> = FxHashMap::default();
        map.insert(FileId(0), &offsets);
        assert_eq!(super::byte_offset_to_line_col(&map, FileId(0), 5), (2, 1));
    }
}
