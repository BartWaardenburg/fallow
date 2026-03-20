use rustc_hash::{FxHashMap, FxHashSet};

use fallow_config::ResolvedConfig;

use crate::discover::FileId;
use crate::graph::ModuleGraph;
use crate::results::*;
use crate::suppress::{self, IssueKind, Suppression};

use super::{LineOffsetsMap, byte_offset_to_line_col, read_source};

/// Find exports that are never imported by other files.
pub fn find_unused_exports(
    graph: &ModuleGraph,
    config: &ResolvedConfig,
    plugin_result: Option<&crate::plugins::AggregatedPluginResult>,
    suppressions_by_file: &FxHashMap<FileId, &[Suppression]>,
    line_offsets_by_file: &LineOffsetsMap<'_>,
) -> (Vec<UnusedExport>, Vec<UnusedExport>) {
    let mut unused_exports = Vec::new();
    let mut unused_types = Vec::new();

    // Pre-compile glob matchers for ignore rules
    let ignore_matchers: Vec<(globset::GlobMatcher, &[String])> = config
        .ignore_export_rules
        .iter()
        .filter_map(|rule| {
            globset::Glob::new(&rule.file)
                .ok()
                .map(|g| (g.compile_matcher(), rule.exports.as_slice()))
        })
        .collect();

    // Compile plugin-discovered used_exports rules (includes framework preset rules)
    let plugin_matchers: Vec<(globset::GlobMatcher, Vec<&str>)> = plugin_result
        .map(|pr| {
            pr.used_exports
                .iter()
                .filter_map(|(file_pat, exports)| {
                    globset::Glob::new(file_pat).ok().map(|g| {
                        (
                            g.compile_matcher(),
                            exports.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                        )
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    for module in &graph.modules {
        // Skip unreachable modules (already reported as unused files)
        if !module.is_reachable {
            continue;
        }

        // Skip entry points (their exports are consumed externally)
        if module.is_entry_point {
            continue;
        }

        // Skip CJS modules with module.exports (hard to track individual exports)
        if module.has_cjs_exports && module.exports.is_empty() {
            continue;
        }

        // Namespace imports are now handled with member-access narrowing in graph.rs:
        // only specific accessed members get references populated. No blanket skip needed.

        // Svelte files use `export let`/`export const` for component props, which are
        // consumed by the Svelte runtime rather than imported by other modules. Since we
        // can't distinguish props from utility exports in the `<script>` block without
        // Svelte compiler semantics, we skip export analysis entirely for reachable
        // .svelte files. Unreachable Svelte files are still caught by `find_unused_files`.
        if module.path.extension().is_some_and(|ext| ext == "svelte") {
            continue;
        }

        // Check ignore rules — compute relative path and string once per module
        let relative_path = module
            .path
            .strip_prefix(&config.root)
            .unwrap_or(&module.path);
        let file_str = relative_path.to_string_lossy();

        // Pre-check which ignore/plugin matchers match this file
        let matching_ignore: Vec<&[String]> = ignore_matchers
            .iter()
            .filter(|(m, _)| m.is_match(file_str.as_ref()))
            .map(|(_, exports)| *exports)
            .collect();

        // Check plugin-discovered used_exports rules (includes framework preset rules)
        let matching_plugin: Vec<&Vec<&str>> = plugin_matchers
            .iter()
            .filter(|(m, _)| m.is_match(file_str.as_ref()))
            .map(|(_, exports)| exports)
            .collect();

        for export in &module.exports {
            if export.references.is_empty() {
                let export_str = export.name.to_string();

                // Check if this export is ignored by config
                if matching_ignore
                    .iter()
                    .any(|exports| exports.iter().any(|e| e == "*" || e == &export_str))
                {
                    continue;
                }

                // Check if this export is considered "used" by a plugin/framework rule
                if matching_plugin
                    .iter()
                    .any(|exports| exports.iter().any(|e| *e == export_str))
                {
                    continue;
                }

                let (line, col) = byte_offset_to_line_col(
                    line_offsets_by_file,
                    module.file_id,
                    export.span.start,
                );

                // Barrel re-exports are synthesized in graph.rs with Span::new(0, 0) as a sentinel.
                let is_re_export = export.span.start == 0 && export.span.end == 0;

                // Check inline suppression
                let issue_kind = if export.is_type_only {
                    IssueKind::UnusedType
                } else {
                    IssueKind::UnusedExport
                };
                if let Some(supps) = suppressions_by_file.get(&module.file_id)
                    && suppress::is_suppressed(supps, line, issue_kind)
                {
                    continue;
                }

                let unused = UnusedExport {
                    path: module.path.clone(),
                    export_name: export_str,
                    is_type_only: export.is_type_only,
                    line,
                    col,
                    span_start: export.span.start,
                    is_re_export,
                };

                if export.is_type_only {
                    unused_types.push(unused);
                } else {
                    unused_exports.push(unused);
                }
            }
        }
    }

    (unused_exports, unused_types)
}

/// Find exports that appear with the same name in multiple files (potential duplicates).
///
/// Barrel re-exports (files that only re-export from other modules via `export { X } from './source'`)
/// are excluded — having an index.ts re-export the same name as the source module is the normal
/// barrel file pattern, not a true duplicate.
pub fn find_duplicate_exports(
    graph: &ModuleGraph,
    config: &ResolvedConfig,
    suppressions_by_file: &FxHashMap<FileId, &[Suppression]>,
) -> Vec<DuplicateExport> {
    // Build a set of re-export relationships: (re-exporting module idx) -> set of (source module idx)
    let mut re_export_sources: FxHashMap<usize, FxHashSet<usize>> = FxHashMap::default();
    for (idx, module) in graph.modules.iter().enumerate() {
        for re in &module.re_exports {
            re_export_sources
                .entry(idx)
                .or_default()
                .insert(re.source_file.0 as usize);
        }
    }

    let mut export_locations: FxHashMap<String, Vec<(usize, std::path::PathBuf)>> =
        FxHashMap::default();

    for (idx, module) in graph.modules.iter().enumerate() {
        if !module.is_reachable || module.is_entry_point {
            continue;
        }

        // Skip files with file-wide duplicate-export suppression
        if suppressions_by_file
            .get(&module.file_id)
            .is_some_and(|supps| suppress::is_file_suppressed(supps, IssueKind::DuplicateExport))
        {
            continue;
        }

        for export in &module.exports {
            if matches!(export.name, crate::extract::ExportName::Default) {
                continue; // Skip default exports
            }
            // Skip synthetic re-export entries (span 0..0) — these are generated by
            // graph construction for re-exports, not real local declarations
            if export.span.start == 0 && export.span.end == 0 {
                continue;
            }
            let name = export.name.to_string();
            export_locations
                .entry(name)
                .or_default()
                .push((idx, module.path.clone()));
        }
    }

    // Filter: only keep truly independent duplicates (not re-export chains)
    let _ = config; // used for consistency
    // Sort by export name for deterministic output order
    let mut sorted_locations: Vec<_> = export_locations.into_iter().collect();
    sorted_locations.sort_by(|a, b| a.0.cmp(&b.0));

    sorted_locations
        .into_iter()
        .filter_map(|(name, locations)| {
            if locations.len() <= 1 {
                return None;
            }
            // Remove entries where one module re-exports from another in the set.
            // For each pair (A, B), if A re-exports from B or B re-exports from A,
            // they are part of the same export chain, not true duplicates.
            let module_indices: FxHashSet<usize> = locations.iter().map(|(idx, _)| *idx).collect();
            let independent: Vec<std::path::PathBuf> = locations
                .into_iter()
                .filter(|(idx, _)| {
                    // Keep this module only if it doesn't re-export from another module in the set
                    // AND no other module in the set re-exports from it (unless both are sources)
                    let sources = re_export_sources.get(idx);
                    let has_source_in_set =
                        sources.is_some_and(|s| s.iter().any(|src| module_indices.contains(src)));
                    !has_source_in_set
                })
                .map(|(_, path)| path)
                .collect();

            if independent.len() > 1 {
                Some(DuplicateExport {
                    export_name: name,
                    locations: independent,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Collect usage counts for all exports in the module graph.
///
/// Iterates every module and every export, producing an `ExportUsage` entry with the
/// reference count and reference locations. This data is used by the LSP server to show
/// Code Lens annotations (e.g., "3 references") above export declarations, with
/// click-to-navigate support via `editor.action.showReferences`.
pub fn collect_export_usages(
    graph: &ModuleGraph,
    line_offsets_by_file: &LineOffsetsMap<'_>,
) -> Vec<ExportUsage> {
    let mut usages = Vec::new();

    // Build FileId -> path index for resolving reference locations
    let file_paths: FxHashMap<FileId, &std::path::Path> = graph
        .modules
        .iter()
        .map(|m| (m.file_id, m.path.as_path()))
        .collect();

    // Fallback source cache for reference locations not in the line offsets map.
    // Only populated when a referencing file's line offsets are unavailable.
    let mut source_cache: FxHashMap<FileId, String> = FxHashMap::default();

    for module in &graph.modules {
        // Skip unreachable modules — no point showing Code Lens for files
        // that aren't reachable from any entry point
        if !module.is_reachable {
            continue;
        }

        for export in &module.exports {
            // Skip synthetic re-export entries (span 0..0) — these are generated
            // by graph construction, not real local declarations in the source
            if export.span.start == 0 && export.span.end == 0 {
                continue;
            }

            let (line, col) =
                byte_offset_to_line_col(line_offsets_by_file, module.file_id, export.span.start);

            // Resolve reference locations for Code Lens navigation
            let reference_locations: Vec<ReferenceLocation> = export
                .references
                .iter()
                .filter_map(|r| {
                    // Skip references with no span (e.g. from dynamic import patterns)
                    if r.import_span.start == 0 && r.import_span.end == 0 {
                        return None;
                    }
                    let ref_path = file_paths.get(&r.from_file)?;
                    // Use pre-computed line offsets when available, fall back to disk read
                    let (ref_line, ref_col) = if line_offsets_by_file.contains_key(&r.from_file) {
                        byte_offset_to_line_col(
                            line_offsets_by_file,
                            r.from_file,
                            r.import_span.start,
                        )
                    } else {
                        let ref_source = source_cache
                            .entry(r.from_file)
                            .or_insert_with(|| read_source(ref_path));
                        let offsets = fallow_types::extract::compute_line_offsets(ref_source);
                        fallow_types::extract::byte_offset_to_line_col(
                            &offsets,
                            r.import_span.start,
                        )
                    };
                    Some(ReferenceLocation {
                        path: ref_path.to_path_buf(),
                        line: ref_line,
                        col: ref_col,
                    })
                })
                .collect();

            usages.push(ExportUsage {
                path: module.path.clone(),
                export_name: export.name.to_string(),
                line,
                col,
                reference_count: export.references.len(),
                reference_locations,
            });
        }
    }

    usages
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::{DiscoveredFile, EntryPoint, EntryPointSource, FileId};
    use crate::extract::ExportName;
    use crate::graph::{ExportSymbol, ModuleGraph, ReExportEdge, SymbolReference};
    use crate::resolve::ResolvedModule;
    use oxc_span::Span;
    use std::path::PathBuf;

    /// Build a minimal ModuleGraph via the build() constructor.
    fn build_graph(file_specs: &[(&str, bool)]) -> ModuleGraph {
        let files: Vec<DiscoveredFile> = file_specs
            .iter()
            .enumerate()
            .map(|(i, (path, _))| DiscoveredFile {
                id: FileId(i as u32),
                path: PathBuf::from(path),
                size_bytes: 0,
            })
            .collect();

        let entry_points: Vec<EntryPoint> = file_specs
            .iter()
            .filter(|(_, is_entry)| *is_entry)
            .map(|(path, _)| EntryPoint {
                path: PathBuf::from(path),
                source: EntryPointSource::ManualEntry,
            })
            .collect();

        let resolved_modules: Vec<ResolvedModule> = files
            .iter()
            .map(|f| ResolvedModule {
                file_id: f.id,
                path: f.path.clone(),
                exports: vec![],
                re_exports: vec![],
                resolved_imports: vec![],
                resolved_dynamic_imports: vec![],
                resolved_dynamic_patterns: vec![],
                member_accesses: vec![],
                whole_object_uses: vec![],
                has_cjs_exports: false,
            })
            .collect();

        ModuleGraph::build(&resolved_modules, &entry_points, &files)
    }

    /// Build a default ResolvedConfig for tests.
    fn test_config() -> ResolvedConfig {
        fallow_config::FallowConfig {
            schema: None,
            extends: vec![],
            entry: vec![],
            ignore_patterns: vec![],
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            duplicates: fallow_config::DuplicatesConfig::default(),
            rules: fallow_config::RulesConfig::default(),
            production: false,
            plugins: vec![],
            overrides: vec![],
        }
        .resolve(
            PathBuf::from("/tmp/test"),
            fallow_config::OutputFormat::Human,
            1,
            true,
        )
    }

    fn make_export(name: &str, span_start: u32, span_end: u32) -> ExportSymbol {
        ExportSymbol {
            name: ExportName::Named(name.to_string()),
            is_type_only: false,
            span: Span::new(span_start, span_end),
            references: vec![],
            members: vec![],
        }
    }

    fn make_referenced_export(
        name: &str,
        span_start: u32,
        span_end: u32,
        from: u32,
    ) -> ExportSymbol {
        ExportSymbol {
            name: ExportName::Named(name.to_string()),
            is_type_only: false,
            span: Span::new(span_start, span_end),
            references: vec![SymbolReference {
                from_file: FileId(from),
                kind: crate::graph::ReferenceKind::NamedImport,
                import_span: Span::new(0, 10),
            }],
            members: vec![],
        }
    }

    // ---- find_duplicate_exports tests ----

    #[test]
    fn duplicate_exports_empty_graph() {
        let graph = build_graph(&[]);
        let config = test_config();
        let suppressions = FxHashMap::default();
        let result = find_duplicate_exports(&graph, &config, &suppressions);
        assert!(result.is_empty());
    }

    #[test]
    fn duplicate_exports_no_duplicates_single_module() {
        let mut graph = build_graph(&[("/src/entry.ts", true), ("/src/utils.ts", false)]);
        graph.modules[1].is_reachable = true;
        graph.modules[1].exports = vec![make_export("foo", 10, 20), make_export("bar", 30, 40)];
        let config = test_config();
        let suppressions = FxHashMap::default();
        let result = find_duplicate_exports(&graph, &config, &suppressions);
        assert!(result.is_empty());
    }

    #[test]
    fn duplicate_exports_detects_same_name_in_two_modules() {
        let mut graph = build_graph(&[
            ("/src/entry.ts", true),
            ("/src/a.ts", false),
            ("/src/b.ts", false),
        ]);
        graph.modules[1].is_reachable = true;
        graph.modules[1].exports = vec![make_export("helper", 10, 20)];
        graph.modules[2].is_reachable = true;
        graph.modules[2].exports = vec![make_export("helper", 10, 20)];
        let config = test_config();
        let suppressions = FxHashMap::default();
        let result = find_duplicate_exports(&graph, &config, &suppressions);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].export_name, "helper");
        assert_eq!(result[0].locations.len(), 2);
    }

    #[test]
    fn duplicate_exports_skips_default_exports() {
        let mut graph = build_graph(&[
            ("/src/entry.ts", true),
            ("/src/a.ts", false),
            ("/src/b.ts", false),
        ]);
        graph.modules[1].is_reachable = true;
        graph.modules[1].exports = vec![ExportSymbol {
            name: ExportName::Default,
            is_type_only: false,
            span: Span::new(10, 20),
            references: vec![],
            members: vec![],
        }];
        graph.modules[2].is_reachable = true;
        graph.modules[2].exports = vec![ExportSymbol {
            name: ExportName::Default,
            is_type_only: false,
            span: Span::new(10, 20),
            references: vec![],
            members: vec![],
        }];
        let config = test_config();
        let suppressions = FxHashMap::default();
        let result = find_duplicate_exports(&graph, &config, &suppressions);
        assert!(result.is_empty());
    }

    #[test]
    fn duplicate_exports_skips_synthetic_re_export_entries() {
        let mut graph = build_graph(&[
            ("/src/entry.ts", true),
            ("/src/a.ts", false),
            ("/src/b.ts", false),
        ]);
        graph.modules[1].is_reachable = true;
        graph.modules[1].exports = vec![make_export("helper", 0, 0)]; // synthetic
        graph.modules[2].is_reachable = true;
        graph.modules[2].exports = vec![make_export("helper", 10, 20)]; // real
        let config = test_config();
        let suppressions = FxHashMap::default();
        let result = find_duplicate_exports(&graph, &config, &suppressions);
        assert!(result.is_empty());
    }

    #[test]
    fn duplicate_exports_skips_unreachable_modules() {
        let mut graph = build_graph(&[
            ("/src/entry.ts", true),
            ("/src/a.ts", false),
            ("/src/b.ts", false),
        ]);
        graph.modules[1].is_reachable = true;
        graph.modules[1].exports = vec![make_export("helper", 10, 20)];
        // Module 2 stays unreachable
        graph.modules[2].exports = vec![make_export("helper", 10, 20)];
        let config = test_config();
        let suppressions = FxHashMap::default();
        let result = find_duplicate_exports(&graph, &config, &suppressions);
        assert!(result.is_empty());
    }

    #[test]
    fn duplicate_exports_skips_entry_points() {
        let mut graph = build_graph(&[("/src/entry.ts", true), ("/src/b.ts", false)]);
        graph.modules[0].exports = vec![make_export("helper", 10, 20)];
        graph.modules[1].is_reachable = true;
        graph.modules[1].exports = vec![make_export("helper", 10, 20)];
        let config = test_config();
        let suppressions = FxHashMap::default();
        let result = find_duplicate_exports(&graph, &config, &suppressions);
        assert!(result.is_empty());
    }

    #[test]
    fn duplicate_exports_filters_re_export_chains() {
        let mut graph = build_graph(&[
            ("/src/entry.ts", true),
            ("/src/index.ts", false),
            ("/src/helper.ts", false),
        ]);
        graph.modules[1].is_reachable = true;
        graph.modules[1].exports = vec![make_export("helper", 10, 20)];
        graph.modules[1].re_exports = vec![ReExportEdge {
            source_file: FileId(2),
            imported_name: "helper".to_string(),
            exported_name: "helper".to_string(),
            is_type_only: false,
        }];
        graph.modules[2].is_reachable = true;
        graph.modules[2].exports = vec![make_export("helper", 5, 15)];
        let config = test_config();
        let suppressions = FxHashMap::default();
        let result = find_duplicate_exports(&graph, &config, &suppressions);
        assert!(result.is_empty());
    }

    #[test]
    fn duplicate_exports_suppressed_file_wide() {
        let mut graph = build_graph(&[
            ("/src/entry.ts", true),
            ("/src/a.ts", false),
            ("/src/b.ts", false),
        ]);
        graph.modules[1].is_reachable = true;
        graph.modules[1].exports = vec![make_export("helper", 10, 20)];
        graph.modules[2].is_reachable = true;
        graph.modules[2].exports = vec![make_export("helper", 10, 20)];
        let config = test_config();

        let supp = vec![Suppression {
            line: 0,
            kind: Some(IssueKind::DuplicateExport),
        }];
        let mut suppressions: FxHashMap<FileId, &[Suppression]> = FxHashMap::default();
        suppressions.insert(FileId(2), &supp);

        let result = find_duplicate_exports(&graph, &config, &suppressions);
        assert!(result.is_empty());
    }

    #[test]
    fn duplicate_exports_three_modules_same_name() {
        let mut graph = build_graph(&[
            ("/src/entry.ts", true),
            ("/src/a.ts", false),
            ("/src/b.ts", false),
            ("/src/c.ts", false),
        ]);
        for i in 1..=3 {
            graph.modules[i].is_reachable = true;
            graph.modules[i].exports = vec![make_export("sharedFn", 10, 20)];
        }
        let config = test_config();
        let suppressions = FxHashMap::default();
        let result = find_duplicate_exports(&graph, &config, &suppressions);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].export_name, "sharedFn");
        assert_eq!(result[0].locations.len(), 3);
    }

    #[test]
    fn duplicate_exports_different_names_not_duplicated() {
        let mut graph = build_graph(&[
            ("/src/entry.ts", true),
            ("/src/a.ts", false),
            ("/src/b.ts", false),
        ]);
        graph.modules[1].is_reachable = true;
        graph.modules[1].exports = vec![make_export("foo", 10, 20)];
        graph.modules[2].is_reachable = true;
        graph.modules[2].exports = vec![make_export("bar", 10, 20)];
        let config = test_config();
        let suppressions = FxHashMap::default();
        let result = find_duplicate_exports(&graph, &config, &suppressions);
        assert!(result.is_empty());
    }

    // ---- collect_export_usages tests ----

    #[test]
    fn collect_usages_empty_graph() {
        let graph = build_graph(&[]);
        let result = collect_export_usages(&graph, &FxHashMap::default());
        assert!(result.is_empty());
    }

    #[test]
    fn collect_usages_skips_unreachable_modules() {
        let mut graph = build_graph(&[("/src/dead.ts", false)]);
        graph.modules[0].exports = vec![make_export("unused", 10, 20)];
        let result = collect_export_usages(&graph, &FxHashMap::default());
        assert!(result.is_empty());
    }

    #[test]
    fn collect_usages_skips_synthetic_exports() {
        let mut graph = build_graph(&[("/src/barrel.ts", true)]);
        graph.modules[0].exports = vec![make_export("reexported", 0, 0)];
        let result = collect_export_usages(&graph, &FxHashMap::default());
        assert!(result.is_empty());
    }

    #[test]
    fn collect_usages_counts_references() {
        let mut graph = build_graph(&[("/src/utils.ts", true), ("/src/app.ts", false)]);
        graph.modules[0].exports = vec![make_referenced_export("helper", 10, 20, 1)];
        let result = collect_export_usages(&graph, &FxHashMap::default());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].export_name, "helper");
        assert_eq!(result[0].reference_count, 1);
    }

    #[test]
    fn collect_usages_zero_references_still_reported() {
        let mut graph = build_graph(&[("/src/utils.ts", true)]);
        graph.modules[0].exports = vec![make_export("unused", 10, 20)];
        let result = collect_export_usages(&graph, &FxHashMap::default());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].export_name, "unused");
        assert_eq!(result[0].reference_count, 0);
        assert!(result[0].reference_locations.is_empty());
    }

    #[test]
    fn collect_usages_multiple_exports_same_module() {
        let mut graph = build_graph(&[("/src/utils.ts", true)]);
        graph.modules[0].exports = vec![make_export("alpha", 10, 20), make_export("beta", 30, 40)];
        let result = collect_export_usages(&graph, &FxHashMap::default());
        assert_eq!(result.len(), 2);
        let names: FxHashSet<&str> = result.iter().map(|u| u.export_name.as_str()).collect();
        assert!(names.contains("alpha"));
        assert!(names.contains("beta"));
    }
}
