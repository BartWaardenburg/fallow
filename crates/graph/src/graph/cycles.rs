//! Circular dependency detection via Tarjan's SCC algorithm + elementary cycle enumeration.
#![expect(clippy::excessive_nesting)]

use std::ops::Range;

use fixedbitset::FixedBitSet;
use rustc_hash::FxHashSet;

use fallow_types::discover::FileId;

use super::ModuleGraph;
use super::types::ModuleNode;

impl ModuleGraph {
    /// Find all circular dependency cycles in the module graph.
    ///
    /// Uses an iterative implementation of Tarjan's strongly connected components
    /// algorithm (O(V + E)) to find all SCCs with 2 or more nodes. Each such SCC
    /// represents a set of files involved in a circular dependency.
    ///
    /// Returns cycles sorted by length (shortest first), with files within each
    /// cycle sorted by path for deterministic output.
    pub fn find_cycles(&self) -> Vec<Vec<FileId>> {
        let n = self.modules.len();
        if n == 0 {
            return Vec::new();
        }

        // Tarjan's SCC state
        let mut index_counter: u32 = 0;
        let mut indices: Vec<u32> = vec![u32::MAX; n]; // u32::MAX = undefined
        let mut lowlinks: Vec<u32> = vec![0; n];
        let mut on_stack = FixedBitSet::with_capacity(n);
        let mut stack: Vec<usize> = Vec::new();
        let mut sccs: Vec<Vec<FileId>> = Vec::new();

        // Iterative DFS stack frame
        struct Frame {
            node: usize,
            succ_pos: usize,
            succ_end: usize,
        }

        // Pre-collect all successors (deduplicated) into a flat vec for cache-friendly access.
        let mut all_succs: Vec<usize> = Vec::with_capacity(self.edges.len());
        let mut succ_ranges: Vec<Range<usize>> = Vec::with_capacity(n);
        let mut seen_set = FxHashSet::default();
        for module in &self.modules {
            let start = all_succs.len();
            seen_set.clear();
            for edge in &self.edges[module.edge_range.clone()] {
                let target = edge.target.0 as usize;
                if target < n && seen_set.insert(target) {
                    all_succs.push(target);
                }
            }
            let end = all_succs.len();
            succ_ranges.push(start..end);
        }

        let mut dfs_stack: Vec<Frame> = Vec::new();

        for start_node in 0..n {
            if indices[start_node] != u32::MAX {
                continue;
            }

            // Push the starting node
            indices[start_node] = index_counter;
            lowlinks[start_node] = index_counter;
            index_counter += 1;
            on_stack.insert(start_node);
            stack.push(start_node);

            let range = &succ_ranges[start_node];
            dfs_stack.push(Frame {
                node: start_node,
                succ_pos: range.start,
                succ_end: range.end,
            });

            while let Some(frame) = dfs_stack.last_mut() {
                if frame.succ_pos < frame.succ_end {
                    let w = all_succs[frame.succ_pos];
                    frame.succ_pos += 1;

                    if indices[w] == u32::MAX {
                        // Tree edge: push w onto the DFS stack
                        indices[w] = index_counter;
                        lowlinks[w] = index_counter;
                        index_counter += 1;
                        on_stack.insert(w);
                        stack.push(w);

                        let range = &succ_ranges[w];
                        dfs_stack.push(Frame {
                            node: w,
                            succ_pos: range.start,
                            succ_end: range.end,
                        });
                    } else if on_stack.contains(w) {
                        // Back edge: update lowlink
                        let v = frame.node;
                        lowlinks[v] = lowlinks[v].min(indices[w]);
                    }
                } else {
                    // All successors processed — pop this frame
                    let v = frame.node;
                    let v_lowlink = lowlinks[v];
                    let v_index = indices[v];
                    dfs_stack.pop();

                    // Update parent's lowlink
                    if let Some(parent) = dfs_stack.last_mut() {
                        lowlinks[parent.node] = lowlinks[parent.node].min(v_lowlink);
                    }

                    // If v is a root node, pop the SCC
                    if v_lowlink == v_index {
                        let mut scc = Vec::new();
                        loop {
                            let w = stack.pop().expect("SCC stack should not be empty");
                            on_stack.set(w, false);
                            scc.push(FileId(w as u32));
                            if w == v {
                                break;
                            }
                        }
                        // Only report cycles of length >= 2
                        if scc.len() >= 2 {
                            sccs.push(scc);
                        }
                    }
                }
            }
        }

        // Phase 2: Enumerate individual elementary cycles within each SCC.
        // For small SCCs (len == 2), there's exactly one cycle.
        // For larger SCCs, use bounded DFS to find up to MAX_CYCLES_PER_SCC cycles.
        const MAX_CYCLES_PER_SCC: usize = 20;

        let mut result: Vec<Vec<FileId>> = Vec::new();
        let mut seen_cycles: FxHashSet<Vec<u32>> = FxHashSet::default();

        for scc in &sccs {
            if scc.len() == 2 {
                let mut cycle = vec![scc[0].0 as usize, scc[1].0 as usize];
                // Canonical: smallest path first
                if self.modules[cycle[1]].path < self.modules[cycle[0]].path {
                    cycle.swap(0, 1);
                }
                let key: Vec<u32> = cycle.iter().map(|&n| n as u32).collect();
                if seen_cycles.insert(key) {
                    result.push(cycle.into_iter().map(|n| FileId(n as u32)).collect());
                }
                continue;
            }

            let scc_nodes: Vec<usize> = scc.iter().map(|id| id.0 as usize).collect();
            let elementary = enumerate_elementary_cycles(
                &scc_nodes,
                &all_succs,
                &succ_ranges,
                MAX_CYCLES_PER_SCC,
                &self.modules,
            );

            for cycle in elementary {
                let key: Vec<u32> = cycle.iter().map(|&n| n as u32).collect();
                if seen_cycles.insert(key) {
                    result.push(cycle.into_iter().map(|n| FileId(n as u32)).collect());
                }
            }
        }

        // Sort: shortest first, then by first file path
        result.sort_by(|a, b| {
            a.len().cmp(&b.len()).then_with(|| {
                self.modules[a[0].0 as usize]
                    .path
                    .cmp(&self.modules[b[0].0 as usize].path)
            })
        });

        result
    }
}

/// Rotate a cycle so the node with the smallest path is first (canonical form for dedup).
fn canonical_cycle(cycle: &[usize], modules: &[ModuleNode]) -> Vec<usize> {
    if cycle.is_empty() {
        return Vec::new();
    }
    let min_pos = cycle
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| modules[**a].path.cmp(&modules[**b].path))
        .map_or(0, |(i, _)| i);
    let mut result = cycle[min_pos..].to_vec();
    result.extend_from_slice(&cycle[..min_pos]);
    result
}

/// Enumerate individual elementary cycles within an SCC using depth-limited DFS.
///
/// Uses iterative deepening: first finds all 2-node cycles, then 3-node, etc.
/// This ensures the shortest, most actionable cycles are always found first.
/// Stops after `max_cycles` total cycles to bound work on dense SCCs.
fn enumerate_elementary_cycles(
    scc_nodes: &[usize],
    all_succs: &[usize],
    succ_ranges: &[Range<usize>],
    max_cycles: usize,
    modules: &[ModuleNode],
) -> Vec<Vec<usize>> {
    let scc_set: FxHashSet<usize> = scc_nodes.iter().copied().collect();
    let mut cycles: Vec<Vec<usize>> = Vec::new();
    let mut seen: FxHashSet<Vec<u32>> = FxHashSet::default();

    // Sort start nodes by path for deterministic enumeration order
    let mut sorted_nodes: Vec<usize> = scc_nodes.to_vec();
    sorted_nodes.sort_by(|a, b| modules[*a].path.cmp(&modules[*b].path));

    // DFS frame for iterative cycle finding
    struct CycleFrame {
        succ_pos: usize,
        succ_end: usize,
    }

    // Iterative deepening: increase max depth from 2 up to SCC size
    let max_depth = scc_nodes.len().min(12); // Cap depth to avoid very long cycles
    for depth_limit in 2..=max_depth {
        if cycles.len() >= max_cycles {
            break;
        }

        for &start in &sorted_nodes {
            if cycles.len() >= max_cycles {
                break;
            }

            let mut path: Vec<usize> = vec![start];
            let mut path_set = FixedBitSet::with_capacity(modules.len());
            path_set.insert(start);

            let range = &succ_ranges[start];
            let mut dfs: Vec<CycleFrame> = vec![CycleFrame {
                succ_pos: range.start,
                succ_end: range.end,
            }];

            while let Some(frame) = dfs.last_mut() {
                if cycles.len() >= max_cycles {
                    break;
                }

                if frame.succ_pos >= frame.succ_end {
                    // Backtrack
                    dfs.pop();
                    if path.len() > 1 {
                        let last = path.pop().unwrap();
                        path_set.set(last, false);
                    }
                    continue;
                }

                let w = all_succs[frame.succ_pos];
                frame.succ_pos += 1;

                // Only follow edges within this SCC
                if !scc_set.contains(&w) {
                    continue;
                }

                if w == start && path.len() >= 2 && path.len() == depth_limit {
                    // Found an elementary cycle at exactly this depth
                    let canonical = canonical_cycle(&path, modules);
                    let key: Vec<u32> = canonical.iter().map(|&n| n as u32).collect();
                    if seen.insert(key) {
                        cycles.push(canonical);
                    }
                } else if !path_set.contains(w) && path.len() < depth_limit {
                    // Extend path (only if within depth limit)
                    path.push(w);
                    path_set.insert(w);

                    let range = &succ_ranges[w];
                    dfs.push(CycleFrame {
                        succ_pos: range.start,
                        succ_end: range.end,
                    });
                }
            }
        }
    }

    cycles
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::resolve::{ResolveResult, ResolvedImport, ResolvedModule};
    use fallow_types::discover::{DiscoveredFile, EntryPoint, EntryPointSource, FileId};
    use fallow_types::extract::{ExportName, ImportInfo, ImportedName};

    use super::ModuleGraph;

    /// Helper: build a graph from files+edges, no entry points needed for cycle detection.
    fn build_cycle_graph(file_count: usize, edges_spec: &[(u32, u32)]) -> ModuleGraph {
        let files: Vec<DiscoveredFile> = (0..file_count)
            .map(|i| DiscoveredFile {
                id: FileId(i as u32),
                path: PathBuf::from(format!("/project/file{i}.ts")),
                size_bytes: 100,
            })
            .collect();

        let resolved_modules: Vec<ResolvedModule> = (0..file_count)
            .map(|i| {
                let imports: Vec<ResolvedImport> = edges_spec
                    .iter()
                    .filter(|(src, _)| *src == i as u32)
                    .map(|(_, tgt)| ResolvedImport {
                        info: ImportInfo {
                            source: format!("./file{tgt}"),
                            imported_name: ImportedName::Named("x".to_string()),
                            local_name: "x".to_string(),
                            is_type_only: false,
                            span: oxc_span::Span::new(0, 10),
                        },
                        target: ResolveResult::InternalModule(FileId(*tgt)),
                    })
                    .collect();

                ResolvedModule {
                    file_id: FileId(i as u32),
                    path: PathBuf::from(format!("/project/file{i}.ts")),
                    exports: vec![fallow_types::extract::ExportInfo {
                        name: ExportName::Named("x".to_string()),
                        local_name: Some("x".to_string()),
                        is_type_only: false,
                        is_public: false,
                        span: oxc_span::Span::new(0, 20),
                        members: vec![],
                    }],
                    re_exports: vec![],
                    resolved_imports: imports,
                    resolved_dynamic_imports: vec![],
                    resolved_dynamic_patterns: vec![],
                    member_accesses: vec![],
                    whole_object_uses: vec![],
                    has_cjs_exports: false,
                    unused_import_bindings: vec![],
                }
            })
            .collect();

        let entry_points = vec![EntryPoint {
            path: PathBuf::from("/project/file0.ts"),
            source: EntryPointSource::PackageJsonMain,
        }];

        ModuleGraph::build(&resolved_modules, &entry_points, &files)
    }

    #[test]
    fn find_cycles_empty_graph() {
        let graph = ModuleGraph::build(&[], &[], &[]);
        assert!(graph.find_cycles().is_empty());
    }

    #[test]
    fn find_cycles_no_cycles() {
        // A -> B -> C (no back edges)
        let graph = build_cycle_graph(3, &[(0, 1), (1, 2)]);
        assert!(graph.find_cycles().is_empty());
    }

    #[test]
    fn find_cycles_simple_two_node_cycle() {
        // A -> B -> A
        let graph = build_cycle_graph(2, &[(0, 1), (1, 0)]);
        let cycles = graph.find_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 2);
    }

    #[test]
    fn find_cycles_three_node_cycle() {
        // A -> B -> C -> A
        let graph = build_cycle_graph(3, &[(0, 1), (1, 2), (2, 0)]);
        let cycles = graph.find_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 3);
    }

    #[test]
    fn find_cycles_self_import_ignored() {
        // A -> A (self-import, should NOT be reported)
        let graph = build_cycle_graph(1, &[(0, 0)]);
        let cycles = graph.find_cycles();
        assert!(
            cycles.is_empty(),
            "self-imports should not be reported as cycles"
        );
    }

    #[test]
    fn find_cycles_multiple_independent_cycles() {
        // Cycle 1: A -> B -> A
        // Cycle 2: C -> D -> C
        // No connection between cycles
        let graph = build_cycle_graph(4, &[(0, 1), (1, 0), (2, 3), (3, 2)]);
        let cycles = graph.find_cycles();
        assert_eq!(cycles.len(), 2);
        // Both cycles should have length 2
        assert!(cycles.iter().all(|c| c.len() == 2));
    }

    #[test]
    fn find_cycles_linear_chain_with_back_edge() {
        // A -> B -> C -> D -> B (cycle is B-C-D)
        let graph = build_cycle_graph(4, &[(0, 1), (1, 2), (2, 3), (3, 1)]);
        let cycles = graph.find_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 3);
        // The cycle should contain files 1, 2, 3
        let ids: Vec<u32> = cycles[0].iter().map(|f| f.0).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
        assert!(ids.contains(&3));
        assert!(!ids.contains(&0));
    }

    #[test]
    fn find_cycles_overlapping_cycles_enumerated() {
        // A -> B -> A, B -> C -> B => SCC is {A, B, C} but should report 2 elementary cycles
        let graph = build_cycle_graph(3, &[(0, 1), (1, 0), (1, 2), (2, 1)]);
        let cycles = graph.find_cycles();
        assert_eq!(
            cycles.len(),
            2,
            "should find 2 elementary cycles, not 1 SCC"
        );
        assert!(
            cycles.iter().all(|c| c.len() == 2),
            "both cycles should have length 2"
        );
    }

    #[test]
    fn find_cycles_deterministic_ordering() {
        // Run twice with the same graph — results should be identical
        let graph1 = build_cycle_graph(3, &[(0, 1), (1, 2), (2, 0)]);
        let graph2 = build_cycle_graph(3, &[(0, 1), (1, 2), (2, 0)]);
        let cycles1 = graph1.find_cycles();
        let cycles2 = graph2.find_cycles();
        assert_eq!(cycles1.len(), cycles2.len());
        for (c1, c2) in cycles1.iter().zip(cycles2.iter()) {
            let paths1: Vec<&PathBuf> = c1
                .iter()
                .map(|f| &graph1.modules[f.0 as usize].path)
                .collect();
            let paths2: Vec<&PathBuf> = c2
                .iter()
                .map(|f| &graph2.modules[f.0 as usize].path)
                .collect();
            assert_eq!(paths1, paths2);
        }
    }

    #[test]
    fn find_cycles_sorted_by_length() {
        // Two cycles: A-B (len 2) and C-D-E (len 3)
        let graph = build_cycle_graph(5, &[(0, 1), (1, 0), (2, 3), (3, 4), (4, 2)]);
        let cycles = graph.find_cycles();
        assert_eq!(cycles.len(), 2);
        assert!(
            cycles[0].len() <= cycles[1].len(),
            "cycles should be sorted by length"
        );
    }

    #[test]
    fn find_cycles_large_cycle() {
        // Chain of 10 nodes forming a single cycle: 0->1->2->...->9->0
        let edges: Vec<(u32, u32)> = (0..10).map(|i| (i, (i + 1) % 10)).collect();
        let graph = build_cycle_graph(10, &edges);
        let cycles = graph.find_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 10);
    }

    #[test]
    fn find_cycles_complex_scc_multiple_elementary() {
        // A square: A->B, B->C, C->D, D->A, plus diagonal A->C
        // Elementary cycles: A->B->C->D->A, A->C->D->A, and A->B->C->...
        let graph = build_cycle_graph(4, &[(0, 1), (1, 2), (2, 3), (3, 0), (0, 2)]);
        let cycles = graph.find_cycles();
        // Should find multiple elementary cycles, not just one SCC of 4
        assert!(
            cycles.len() >= 2,
            "should find at least 2 elementary cycles, got {}",
            cycles.len()
        );
        // All cycles should be shorter than the full SCC
        assert!(cycles.iter().all(|c| c.len() <= 4));
    }

    #[test]
    fn find_cycles_no_duplicate_cycles() {
        // Triangle: A->B->C->A — should find exactly 1 cycle, not duplicates
        // from different DFS start points
        let graph = build_cycle_graph(3, &[(0, 1), (1, 2), (2, 0)]);
        let cycles = graph.find_cycles();
        assert_eq!(cycles.len(), 1, "triangle should produce exactly 1 cycle");
        assert_eq!(cycles[0].len(), 3);
    }
}
