//! Step 6: Result filtering and deduplication.
//!
//! Converts raw clone groups into `CloneGroup` structs with line info,
//! applies token-level and line-level subset removal, min_lines filter,
//! and skip_local filter.

use std::path::PathBuf;

use rustc_hash::{FxHashMap, FxHashSet};

use super::FileData;
use super::extraction::RawGroup;
use super::utils::build_clone_instance_fast;
use crate::duplicates::types::{CloneGroup, CloneInstance};

// ── Interval index ──────────────────────────────────────────────
//
// Sorted, non-overlapping `(start, end)` intervals per slot with
// O(log N) lookup and merge-on-insert. Used for both token-level
// and line-level subset removal.

/// Sorted interval lists indexed by a numeric slot (file id or path index).
struct IntervalIndex {
    slots: Vec<Vec<(usize, usize)>>,
}

impl IntervalIndex {
    fn new(num_slots: usize) -> Self {
        Self {
            slots: vec![Vec::new(); num_slots],
        }
    }

    /// Return `true` if `[start, start+len)` is fully contained within an
    /// existing interval in `slot`.
    fn is_covered(&self, slot: usize, start: usize, len: usize) -> bool {
        let intervals = &self.slots[slot];
        let idx = intervals.partition_point(|&(s, _)| s <= start);
        if idx == 0 {
            return false;
        }
        let (s, e) = intervals[idx - 1];
        start >= s && start + len <= e
    }

    /// Insert `[start, end)` into `slot`, merging with the preceding interval
    /// when they overlap.
    fn insert(&mut self, slot: usize, start: usize, end: usize) {
        let intervals = &mut self.slots[slot];
        let idx = intervals.partition_point(|&(s, _)| s < start);
        if idx > 0 {
            let prev = &mut intervals[idx - 1];
            if prev.1 >= start {
                if end > prev.1 {
                    prev.1 = end;
                }
                return;
            }
        }
        intervals.insert(idx, (start, end));
    }
}

// ── Token-level subset removal ──────────────────────────────────

/// Remove raw groups whose token spans are fully contained within a larger
/// group's spans. Groups must arrive sorted by length descending.
fn remove_token_subsets(mut raw_groups: Vec<RawGroup>, num_files: usize) -> Vec<RawGroup> {
    let raw_count = raw_groups.len();
    raw_groups.sort_by(|a, b| b.length.cmp(&a.length));

    let mut covered = IntervalIndex::new(num_files);
    let mut surviving = Vec::new();

    for rg in raw_groups {
        let len = rg.length;
        let all_covered = rg
            .instances
            .iter()
            .all(|&(fid, offset)| covered.is_covered(fid, offset, len));

        if all_covered {
            continue;
        }

        for &(fid, offset) in &rg.instances {
            covered.insert(fid, offset, offset + len);
        }
        surviving.push(rg);
    }

    tracing::trace!(
        raw = raw_count,
        surviving = surviving.len(),
        "token-level subset removal"
    );

    surviving
}

// ── Line table construction ─────────────────────────────────────

/// Build a sorted vec of newline byte positions per file for O(log L) lookup.
fn build_line_tables(files: &[FileData]) -> Vec<Vec<usize>> {
    files
        .iter()
        .map(|f| {
            f.file_tokens
                .source
                .bytes()
                .enumerate()
                .filter_map(|(i, b)| if b == b'\n' { Some(i) } else { None })
                .collect()
        })
        .collect()
}

// ── Single clone group construction ─────────────────────────────

/// Convert a single `RawGroup` into a `CloneGroup`, returning `None` when
/// the group should be filtered out (too few instances, below min_lines,
/// or same-directory when skip_local is set).
fn build_clone_group(
    rg: &RawGroup,
    files: &[FileData],
    line_tables: &[Vec<usize>],
    min_lines: usize,
    skip_local: bool,
) -> Option<CloneGroup> {
    let mut seen: FxHashSet<(usize, usize)> = FxHashSet::default();
    let mut instances: Vec<CloneInstance> = Vec::new();

    for &(file_id, offset) in &rg.instances {
        if !seen.insert((file_id, offset)) {
            continue;
        }
        let file = &files[file_id];
        if let Some(inst) =
            build_clone_instance_fast(file, offset, rg.length, &line_tables[file_id])
        {
            instances.push(inst);
        }
    }

    // Apply skip_local: only keep cross-directory clones.
    if skip_local && instances.len() >= 2 {
        let dirs: FxHashSet<_> = instances
            .iter()
            .filter_map(|inst| inst.file.parent().map(|p| p.to_path_buf()))
            .collect();
        if dirs.len() < 2 {
            return None;
        }
    }

    if instances.len() < 2 {
        return None;
    }

    let line_count = instances
        .iter()
        .map(|inst| inst.end_line.saturating_sub(inst.start_line) + 1)
        .max()
        .unwrap_or(0);

    if line_count < min_lines {
        return None;
    }

    // Sort instances by file path then start line for stable output.
    instances.sort_by(|a, b| a.file.cmp(&b.file).then(a.start_line.cmp(&b.start_line)));

    // Deduplicate instances that map to overlapping line ranges within
    // the same file (different token offsets can resolve to overlapping
    // source spans). When two instances overlap, keep the wider one.
    instances.dedup_by(|b, a| {
        if a.file != b.file {
            return false;
        }
        // Instances are sorted by start_line. `b` starts at or after `a`.
        // If b's start overlaps with a's range, merge by extending a.
        if b.start_line <= a.end_line {
            if b.end_line > a.end_line {
                a.end_line = b.end_line;
                a.end_col = b.end_col;
            }
            true
        } else {
            false
        }
    });

    if instances.len() < 2 {
        return None;
    }

    Some(CloneGroup {
        instances,
        token_count: rg.length,
        line_count,
    })
}

// ── Line-level subset removal ───────────────────────────────────

/// Remove groups whose line ranges are fully contained within a larger
/// group's line ranges. Groups must arrive sorted by token count descending.
///
/// Uses a per-file interval index to avoid O(g^2 x m x n): iterate from
/// largest to smallest, registering kept groups' spans and checking smaller
/// groups against the index in O(instances x log(intervals)).
fn remove_line_subsets(clone_groups: Vec<CloneGroup>) -> Vec<CloneGroup> {
    // Build file path -> slot index mapping.
    let mut path_to_idx: FxHashMap<PathBuf, usize> = FxHashMap::default();
    let mut next_idx = 0usize;
    for group in &clone_groups {
        for inst in &group.instances {
            path_to_idx.entry(inst.file.clone()).or_insert_with(|| {
                let idx = next_idx;
                next_idx += 1;
                idx
            });
        }
    }

    let mut index = IntervalIndex::new(next_idx);
    let mut kept = Vec::new();

    for group in clone_groups {
        let all_contained = group.instances.iter().all(|inst| {
            let fidx = path_to_idx[&inst.file];
            let intervals = &index.slots[fidx];
            let idx = intervals.partition_point(|&(s, _)| s <= inst.start_line);
            idx > 0 && {
                let (s, e) = intervals[idx - 1];
                inst.start_line >= s && inst.end_line <= e
            }
        });

        if all_contained {
            continue;
        }

        for inst in &group.instances {
            let fidx = path_to_idx[&inst.file];
            index.insert(fidx, inst.start_line, inst.end_line);
        }
        kept.push(group);
    }

    kept
}

// ── Main orchestrator ───────────────────────────────────────────

/// Convert raw groups into `CloneGroup` structs, applying `min_lines` and
/// `skip_local` filters, deduplication, and subset removal.
pub(super) fn build_groups(
    raw_groups: Vec<RawGroup>,
    files: &[FileData],
    min_lines: usize,
    skip_local: bool,
) -> Vec<CloneGroup> {
    if raw_groups.is_empty() {
        return Vec::new();
    }

    // Step 1: Token-level subset removal (cheap, before line calculation).
    let surviving = remove_token_subsets(raw_groups, files.len());

    // Step 2: Pre-compute line offset tables for O(log L) byte-to-line lookup.
    let line_tables = build_line_tables(files);

    // Step 3: Convert surviving raw groups into CloneGroups with filtering.
    let mut clone_groups: Vec<CloneGroup> = surviving
        .iter()
        .filter_map(|rg| build_clone_group(rg, files, &line_tables, min_lines, skip_local))
        .collect();

    // Step 4: Sort by token count desc, then instance count desc so that
    // N-way groups come before M-way (M<N) subsets at equal token counts.
    clone_groups.sort_by(|a, b| {
        b.token_count
            .cmp(&a.token_count)
            .then(b.instances.len().cmp(&a.instances.len()))
    });

    // Step 5: Line-level subset removal.
    remove_line_subsets(clone_groups)
}
