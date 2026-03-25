//! Phase 4: Re-export chain resolution — propagate references through barrel files.

mod propagate;
#[cfg(test)]
mod tests;

use rustc_hash::FxHashSet;

use fallow_types::discover::FileId;

use super::ModuleGraph;

use propagate::{propagate_named_re_export, propagate_star_re_export};

impl ModuleGraph {
    /// Resolve re-export chains: when module A re-exports from B,
    /// any reference to A's re-exported symbol should also count as a reference
    /// to B's original export (and transitively through the chain).
    pub(super) fn resolve_re_export_chains(&mut self) {
        // Collect re-export info: (barrel_file_id, source_file_id, imported_name, exported_name)
        let re_export_info: Vec<(FileId, FileId, String, String)> = self
            .modules
            .iter()
            .flat_map(|m| {
                m.re_exports.iter().map(move |re| {
                    (
                        m.file_id,
                        re.source_file,
                        re.imported_name.clone(),
                        re.exported_name.clone(),
                    )
                })
            })
            .collect();

        if re_export_info.is_empty() {
            return;
        }

        // For each re-export, if the barrel's exported symbol has references,
        // propagate those references to the source module's original export.
        // We iterate until no new references are added (handles chains).
        let mut changed = true;
        let max_iterations = 20; // prevent infinite loops on cycles
        let mut iteration = 0;
        // Reuse a single HashSet across iterations to avoid repeated allocations.
        // In barrel-heavy monorepos, this loop can run up to max_iterations × re_export_info.len()
        // × target_exports.len() times — reusing with .clear() avoids O(n) allocations.
        let mut existing_refs: FxHashSet<FileId> = FxHashSet::default();

        while changed && iteration < max_iterations {
            changed = false;
            iteration += 1;

            for &(barrel_id, source_id, ref imported_name, ref exported_name) in &re_export_info {
                let barrel_idx = barrel_id.0 as usize;
                let source_idx = source_id.0 as usize;

                if barrel_idx >= self.modules.len() || source_idx >= self.modules.len() {
                    continue;
                }

                if exported_name == "*" {
                    changed |= propagate_star_re_export(
                        &mut self.modules,
                        &self.edges,
                        barrel_id,
                        barrel_idx,
                        source_idx,
                    );
                } else {
                    changed |= propagate_named_re_export(
                        &mut self.modules,
                        barrel_id,
                        barrel_idx,
                        source_idx,
                        imported_name,
                        exported_name,
                        &mut existing_refs,
                    );
                }
            }
        }

        if iteration >= max_iterations {
            tracing::warn!(
                iterations = max_iterations,
                "Re-export chain resolution hit iteration limit, some chains may be incomplete"
            );
        }
    }
}
