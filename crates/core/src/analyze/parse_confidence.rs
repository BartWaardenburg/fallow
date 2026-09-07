//! Attach reachability-confidence flags to the dead-code findings a degraded
//! parse can distort.
//!
//! A file that fails to parse is reported through `workspace_diagnostics[]` as
//! `source-parse-degraded`, and that stays report-only: oxc emits recoverable
//! errors for valid syntax newer than the parser, so gating findings on parser
//! errors would mute real results project-wide. But the diagnostic sits at the
//! top of the envelope while the finding it distorts sits far away in
//! `unused_files[]` carrying a `delete-file` action, and nothing connects the
//! two. This pass carries the caveat to the finding.
//!
//! What can actually be computed. The imports a parser never saw cannot be
//! attributed to a target: there is no record of them anywhere, so an exact
//! "this degraded file would have imported that path" link does not exist
//! without re-reading and re-parsing the source. Two things ARE computable
//! from data the run already holds, and each covers one of the two routes a
//! degraded parse takes into a reachability verdict:
//!
//! 1. The finding's own file parsed degraded, so the export list read by the
//!    "is any export of this file referenced from a reachable module" test may
//!    be truncated.
//! 2. Some module that is observed reachable parsed degraded, so the import
//!    graph the verdict rests on is incomplete.
//!
//! The second is a run-level condition, not a per-finding one, and its wire
//! documentation says so. It does discriminate: if every degraded module is
//! itself unreachable, no missing edge attributable to a degraded parse can
//! change a reachability verdict, so no flag is emitted. That holds because
//! the FIRST missing edge along any entry-point path leaves from a module
//! whose every predecessor edge was observed, so that module is observed
//! reachable, and it is degraded precisely because the edge went missing.
//!
//! Nothing here suppresses, filters, reorders, or downgrades a finding.

use std::path::Path;

use rustc_hash::FxHashSet;

use fallow_types::output_dead_code::ReachabilityConfidenceFlag;

use crate::extract::ModuleInfo;
use crate::graph::ModuleGraph;
use crate::results::AnalysisResults;

/// The degraded-parse facts of one run, resolved to graph paths once so the
/// annotation pass is a lookup per finding rather than a scan per finding.
pub(super) struct DegradedParseContext<'a> {
    /// Paths of the modules whose parse reported diagnostics.
    degraded_paths: FxHashSet<&'a Path>,
    /// Whether any degraded module is observed reachable from an entry point.
    reachable_degraded: bool,
}

impl<'a> DegradedParseContext<'a> {
    /// Collect the degraded modules of this run and whether any of them is
    /// reachable. `modules` carries the per-file parse error count; the graph
    /// supplies the path and the reachability flag for the same `FileId`.
    pub(super) fn new(graph: &'a ModuleGraph, modules: &[ModuleInfo]) -> Self {
        let mut degraded_paths = FxHashSet::default();
        let mut reachable_degraded = false;
        for module in modules {
            if module.parse_error_count == 0 {
                continue;
            }
            let Some(node) = graph.modules.get(module.file_id.0 as usize) else {
                continue;
            };
            degraded_paths.insert(node.path.as_path());
            reachable_degraded = reachable_degraded || node.is_reachable() || node.is_entry_point();
        }
        Self {
            degraded_paths,
            reachable_degraded,
        }
    }

    /// Whether this run parsed everything it read cleanly, in which case the
    /// annotation pass has nothing to do and every finding stays byte-identical.
    fn is_clean(&self) -> bool {
        self.degraded_paths.is_empty()
    }

    /// The flags that apply to a finding reported on `path`. Returned in enum
    /// declaration order, so the result is sorted and deduplicated by
    /// construction.
    fn flags_for(&self, path: &Path) -> Vec<ReachabilityConfidenceFlag> {
        let mut flags = Vec::new();
        if self.degraded_paths.contains(path) {
            flags.push(ReachabilityConfidenceFlag::SourceParseDegraded);
        }
        if self.reachable_degraded {
            flags.push(ReachabilityConfidenceFlag::IncompleteImportGraph);
        }
        flags
    }

    /// Stamp the flags onto the two verdicts a lost import edge can distort.
    /// A clean run returns without touching a finding.
    pub(super) fn annotate(&self, results: &mut AnalysisResults) {
        if self.is_clean() {
            return;
        }
        for finding in &mut results.unused_files {
            finding.confidence = self.flags_for(&finding.file.path);
        }
        for finding in &mut results.unused_exports {
            finding.confidence = self.flags_for(&finding.export.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::{DiscoveredFile, EntryPoint, EntryPointSource, FileId};
    use crate::resolve::ResolvedModule;
    use crate::results::UnusedFile;
    use fallow_types::output_dead_code::UnusedFileFinding;
    use std::path::PathBuf;

    const INDEX: &str = "/p/src/index.ts";
    const HELPER: &str = "/p/src/helper.ts";
    const ORPHAN: &str = "/p/src/orphan.ts";

    /// A three-file graph whose only entry point is `index.ts`; nothing imports
    /// anything, so `helper.ts` and `orphan.ts` are both unreachable.
    fn graph() -> ModuleGraph {
        let paths = [INDEX, HELPER, ORPHAN];
        let files: Vec<DiscoveredFile> = paths
            .iter()
            .enumerate()
            .map(|(index, path)| DiscoveredFile {
                id: FileId(u32::try_from(index).expect("test file count fits u32")),
                path: PathBuf::from(path),
                size_bytes: 0,
            })
            .collect();
        let entry_points = vec![EntryPoint {
            path: PathBuf::from(INDEX),
            source: EntryPointSource::ManualEntry,
        }];
        let resolved: Vec<ResolvedModule> = files
            .iter()
            .map(|file| ResolvedModule {
                file_id: file.id,
                path: file.path.clone(),
                ..Default::default()
            })
            .collect();
        ModuleGraph::build(&resolved, &entry_points, &files)
    }

    /// `ModuleInfo` values carrying the given parse error count per `FileId`.
    fn modules(error_counts: [u32; 3]) -> Vec<ModuleInfo> {
        error_counts
            .into_iter()
            .enumerate()
            .map(|(index, errors)| ModuleInfo {
                parse_error_count: errors,
                ..ModuleInfo::empty(FileId(
                    u32::try_from(index).expect("test file count fits u32"),
                ))
            })
            .collect()
    }

    fn unused(paths: &[&str]) -> AnalysisResults {
        AnalysisResults {
            unused_files: paths
                .iter()
                .map(|path| {
                    UnusedFileFinding::with_actions(UnusedFile {
                        path: PathBuf::from(path),
                    })
                })
                .collect(),
            ..AnalysisResults::default()
        }
    }

    #[test]
    fn a_run_that_parsed_cleanly_flags_nothing() {
        let graph = graph();
        let mut results = unused(&[HELPER, ORPHAN]);

        DegradedParseContext::new(&graph, &modules([0, 0, 0])).annotate(&mut results);

        assert!(
            results
                .unused_files
                .iter()
                .all(|finding| finding.confidence.is_empty()),
            "a healthy project must carry no marker at all"
        );
    }

    #[test]
    fn a_degraded_reachable_module_flags_the_findings_its_lost_imports_could_reach() {
        let graph = graph();
        let mut results = unused(&[HELPER, ORPHAN]);

        // The entry file is the one that failed to parse: the imports it never
        // credited are exactly why the other two read as unused.
        DegradedParseContext::new(&graph, &modules([3, 0, 0])).annotate(&mut results);

        for finding in &results.unused_files {
            assert_eq!(
                finding.confidence,
                vec![ReachabilityConfidenceFlag::IncompleteImportGraph],
                "{} should carry the incomplete-graph caveat",
                finding.file.path.display()
            );
        }
    }

    #[test]
    fn a_degraded_unreachable_module_flags_only_itself() {
        let graph = graph();
        let mut results = unused(&[HELPER, ORPHAN]);

        // Only `helper.ts` degraded, and it is unreachable: no missing edge of
        // its can change any verdict but its own, whose export list is short.
        DegradedParseContext::new(&graph, &modules([0, 2, 0])).annotate(&mut results);

        let helper = results
            .unused_files
            .iter()
            .find(|finding| finding.file.path.ends_with("helper.ts"))
            .expect("helper finding present");
        assert_eq!(
            helper.confidence,
            vec![ReachabilityConfidenceFlag::SourceParseDegraded],
            "the degraded file's own truncated export list is the only caveat"
        );

        let orphan = results
            .unused_files
            .iter()
            .find(|finding| finding.file.path.ends_with("orphan.ts"))
            .expect("orphan finding present");
        assert!(
            orphan.confidence.is_empty(),
            "an unreachable degraded module must not cast a caveat over the whole run"
        );
    }
}
