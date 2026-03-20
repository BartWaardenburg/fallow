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
