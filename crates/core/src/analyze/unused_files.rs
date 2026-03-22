use rustc_hash::FxHashMap;

use crate::discover::FileId;
use crate::graph::ModuleGraph;
use crate::results::*;
use crate::suppress::{self, IssueKind, Suppression};

use super::predicates::{is_barrel_with_reachable_sources, is_config_file, is_declaration_file};

/// Find files that are not reachable from any entry point.
///
/// TypeScript declaration files (`.d.ts`) are excluded because they are consumed
/// by the TypeScript compiler via `tsconfig.json` includes, not via explicit
/// import statements. Flagging them as unused is a false positive.
///
/// Configuration files (e.g., `babel.config.js`, `.eslintrc.js`, `knip.config.ts`)
/// are also excluded because they are consumed by tools, not via imports.
///
/// Barrel files (index.ts that only re-export) are excluded when their re-export
/// sources are reachable — they serve an organizational purpose even if consumers
/// import directly from the source files rather than through the barrel.
pub fn find_unused_files(
    graph: &ModuleGraph,
    suppressions_by_file: &FxHashMap<FileId, &[Suppression]>,
) -> Vec<UnusedFile> {
    graph
        .modules
        .iter()
        .filter(|m| !m.is_reachable && !m.is_entry_point)
        .filter(|m| !is_declaration_file(&m.path))
        .filter(|m| !is_config_file(&m.path))
        .filter(|m| !is_barrel_with_reachable_sources(m, graph))
        // Safety net: don't report as unused if any reachable module imports this file.
        // BFS reachability should already cover this, but this guard catches edge cases
        // where import resolution or re-export chain propagation creates edges that BFS
        // doesn't fully follow (e.g., path alias resolution inconsistencies).
        .filter(|m| !has_reachable_importer(m.file_id, graph))
        // Don't report as unused if any export actually has references from other modules.
        // Re-export chain propagation (Phase 4) can add references after BFS (Phase 3),
        // so a file may have referenced exports despite being "unreachable" by BFS alone.
        .filter(|m| m.exports.iter().all(|e| e.references.is_empty()))
        // Guard against phantom files: don't report files that no longer exist on disk.
        // This can happen if a file was deleted between discovery and analysis, or if
        // a stale cache entry references a path that no longer exists.
        .filter(|m| m.path.exists())
        .filter(|m| {
            !suppressions_by_file
                .get(&m.file_id)
                .is_some_and(|supps| suppress::is_file_suppressed(supps, IssueKind::UnusedFile))
        })
        .map(|m| UnusedFile {
            path: m.path.clone(),
        })
        .collect()
}

/// Check if any reachable module has an edge to this file.
fn has_reachable_importer(file_id: FileId, graph: &ModuleGraph) -> bool {
    let idx = file_id.0 as usize;
    if idx >= graph.reverse_deps.len() {
        return false;
    }
    graph.reverse_deps[idx].iter().any(|&dep_id| {
        let dep_idx = dep_id.0 as usize;
        dep_idx < graph.modules.len() && graph.modules[dep_idx].is_reachable
    })
}
