mod predicates;
mod unused_deps;
mod unused_exports;
mod unused_files;
mod unused_members;

use rustc_hash::FxHashMap;

use fallow_config::{PackageJson, ResolvedConfig};

use crate::discover::FileId;
use crate::extract::ModuleInfo;
use crate::graph::ModuleGraph;
use crate::resolve::ResolvedModule;
use crate::results::*;
use crate::suppress::Suppression;

use unused_deps::{
    find_type_only_dependencies, find_unlisted_dependencies, find_unresolved_imports,
    find_unused_dependencies,
};
use unused_exports::{collect_export_usages, find_duplicate_exports, find_unused_exports};
use unused_files::find_unused_files;
use unused_members::find_unused_members;

/// Convert a byte offset in source text to a 1-based line and 0-based column (byte offset from
/// start of the line). Uses byte counting to stay consistent with Oxc's byte-offset spans.
fn byte_offset_to_line_col(source: &str, byte_offset: u32) -> (u32, u32) {
    let byte_offset = byte_offset as usize;
    let prefix = &source[..byte_offset.min(source.len())];
    let line = prefix.bytes().filter(|&b| b == b'\n').count() as u32 + 1;
    let col = prefix
        .rfind('\n')
        .map_or(byte_offset, |pos| byte_offset - pos - 1) as u32;
    (line, col)
}

/// Read source content from disk, returning empty string on failure.
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
    find_dead_code_full(graph, config, resolved_modules, plugin_result, &[], &[])
}

/// Find all dead code, with optional resolved module data, plugin context, and workspace info.
pub fn find_dead_code_full(
    graph: &ModuleGraph,
    config: &ResolvedConfig,
    resolved_modules: &[ResolvedModule],
    plugin_result: Option<&crate::plugins::AggregatedPluginResult>,
    workspaces: &[fallow_config::WorkspaceInfo],
    modules: &[ModuleInfo],
) -> AnalysisResults {
    let _span = tracing::info_span!("find_dead_code").entered();

    // Build suppression index: FileId -> suppressions
    let suppressions_by_file: FxHashMap<FileId, &[Suppression]> = modules
        .iter()
        .filter(|m| !m.suppressions.is_empty())
        .map(|m| (m.file_id, m.suppressions.as_slice()))
        .collect();

    let mut results = AnalysisResults::default();

    if config.detect.unused_files {
        results.unused_files = find_unused_files(graph, &suppressions_by_file);
    }

    if config.detect.unused_exports || config.detect.unused_types {
        let (exports, types) =
            find_unused_exports(graph, config, plugin_result, &suppressions_by_file);
        if config.detect.unused_exports {
            results.unused_exports = exports;
        }
        if config.detect.unused_types {
            results.unused_types = types;
        }
    }

    if config.detect.unused_enum_members || config.detect.unused_class_members {
        let (enum_members, class_members) =
            find_unused_members(graph, config, resolved_modules, &suppressions_by_file);
        if config.detect.unused_enum_members {
            results.unused_enum_members = enum_members;
        }
        if config.detect.unused_class_members {
            results.unused_class_members = class_members;
        }
    }

    // Build merged dependency set from root + all workspace package.json files
    let pkg_path = config.root.join("package.json");
    let pkg = PackageJson::load(&pkg_path).ok();
    if let Some(ref pkg) = pkg {
        if config.detect.unused_dependencies || config.detect.unused_dev_dependencies {
            let (deps, dev_deps) =
                find_unused_dependencies(graph, pkg, config, plugin_result, workspaces);
            if config.detect.unused_dependencies {
                results.unused_dependencies = deps;
            }
            if config.detect.unused_dev_dependencies {
                results.unused_dev_dependencies = dev_deps;
            }
        }

        if config.detect.unlisted_dependencies {
            results.unlisted_dependencies =
                find_unlisted_dependencies(graph, pkg, config, workspaces, plugin_result);
        }
    }

    if config.detect.unresolved_imports && !resolved_modules.is_empty() {
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
        );
    }

    if config.detect.duplicate_exports {
        results.duplicate_exports = find_duplicate_exports(graph, config, &suppressions_by_file);
    }

    // In production mode, detect dependencies that are only used via type-only imports
    if config.production
        && let Some(ref pkg) = pkg
    {
        results.type_only_dependencies =
            find_type_only_dependencies(graph, pkg, config, workspaces);
    }

    // Collect export usage counts for Code Lens (LSP feature)
    results.export_usages = collect_export_usages(graph);

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    // byte_offset_to_line_col tests
    #[test]
    fn byte_offset_empty_source() {
        assert_eq!(byte_offset_to_line_col("", 0), (1, 0));
    }

    #[test]
    fn byte_offset_single_line_start() {
        assert_eq!(byte_offset_to_line_col("hello", 0), (1, 0));
    }

    #[test]
    fn byte_offset_single_line_middle() {
        assert_eq!(byte_offset_to_line_col("hello", 4), (1, 4));
    }

    #[test]
    fn byte_offset_multiline_start_of_line2() {
        // "line1\nline2\nline3"
        //  01234 5 678901 2
        // offset 6 = start of "line2"
        let source = "line1\nline2\nline3";
        assert_eq!(byte_offset_to_line_col(source, 6), (2, 0));
    }

    #[test]
    fn byte_offset_multiline_middle_of_line3() {
        // "line1\nline2\nline3"
        //  01234 5 67890 1 23456
        //                1 12345
        // offset 14 = 'n' in "line3" (col 2)
        let source = "line1\nline2\nline3";
        assert_eq!(byte_offset_to_line_col(source, 14), (3, 2));
    }

    #[test]
    fn byte_offset_at_newline_boundary() {
        // "line1\nline2"
        // offset 5 = the '\n' character itself
        let source = "line1\nline2";
        assert_eq!(byte_offset_to_line_col(source, 5), (1, 5));
    }

    #[test]
    fn byte_offset_beyond_source_length() {
        // Line count is clamped (prefix is sliced to source.len()), but the
        // byte-offset column is passed through unclamped because the function
        // uses the raw byte_offset for the column fallback.
        let source = "hello";
        assert_eq!(byte_offset_to_line_col(source, 100), (1, 100));
    }

    #[test]
    fn byte_offset_multibyte_utf8() {
        // Emoji is 4 bytes: "hi\n" (3 bytes) + emoji (4 bytes) + "x" (1 byte)
        let source = "hi\n\u{1F600}x";
        // offset 3 = start of line 2, col 0
        assert_eq!(byte_offset_to_line_col(source, 3), (2, 0));
        // offset 7 = 'x' (after 4-byte emoji), col 4 (byte-based)
        assert_eq!(byte_offset_to_line_col(source, 7), (2, 4));
    }

    #[test]
    fn byte_offset_multibyte_accented_chars() {
        // 'e' with accent (U+00E9) is 2 bytes in UTF-8
        let source = "caf\u{00E9}\nbar";
        // "caf\u{00E9}" = 3 + 2 = 5 bytes, then '\n' at offset 5
        // 'b' at offset 6 -> line 2, col 0
        assert_eq!(byte_offset_to_line_col(source, 6), (2, 0));
        // '\u{00E9}' starts at offset 3, col 3 (byte-based)
        assert_eq!(byte_offset_to_line_col(source, 3), (1, 3));
    }
}
