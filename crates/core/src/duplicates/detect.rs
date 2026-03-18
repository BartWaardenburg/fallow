//! Suffix Array + LCP based clone detection engine.
//!
//! Uses an O(N log N) prefix-doubling suffix array construction (with radix
//! sort) followed by an O(N) LCP scan. This avoids quadratic pairwise
//! comparisons and naturally finds all maximal clones in a single linear pass.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::normalize::HashedToken;
use super::tokenize::FileTokens;
use super::types::{CloneGroup, CloneInstance, DuplicationReport, DuplicationStats};

/// Data for a single file being analyzed.
struct FileData {
    path: PathBuf,
    hashed_tokens: Vec<HashedToken>,
    file_tokens: FileTokens,
}

/// Suffix Array + LCP based clone detection engine.
///
/// Concatenates all files' token sequences (separated by unique sentinels),
/// builds a suffix array and LCP array, then extracts maximal clone groups
/// from contiguous LCP intervals.
pub struct CloneDetector {
    /// Minimum clone size in tokens.
    min_tokens: usize,
    /// Minimum clone size in lines.
    min_lines: usize,
    /// Only report cross-directory duplicates.
    skip_local: bool,
}

impl CloneDetector {
    /// Create a new detector with the given thresholds.
    pub fn new(min_tokens: usize, min_lines: usize, skip_local: bool) -> Self {
        Self {
            min_tokens,
            min_lines,
            skip_local,
        }
    }

    /// Run clone detection across all files.
    ///
    /// `file_data` is a list of `(path, hashed_tokens, file_tokens)` tuples,
    /// one per analyzed file.
    pub fn detect(
        &self,
        file_data: Vec<(PathBuf, Vec<HashedToken>, FileTokens)>,
    ) -> DuplicationReport {
        let _span = tracing::info_span!("clone_detect").entered();

        if file_data.is_empty() || self.min_tokens == 0 {
            return empty_report(0);
        }

        let files: Vec<FileData> = file_data
            .into_iter()
            .map(|(path, hashed_tokens, file_tokens)| FileData {
                path,
                hashed_tokens,
                file_tokens,
            })
            .collect();

        // Compute total stats.
        let total_files = files.len();
        let total_lines: usize = files.iter().map(|f| f.file_tokens.line_count).sum();
        let total_tokens: usize = files.iter().map(|f| f.hashed_tokens.len()).sum();

        tracing::debug!(
            total_files,
            total_tokens,
            total_lines,
            "clone detection input"
        );

        // Step 1: Rank reduction — map u64 hashes to consecutive u32 ranks.
        let t0 = std::time::Instant::now();
        let ranked_files = rank_reduce(&files);
        let rank_time = t0.elapsed();
        let unique_ranks: usize = ranked_files
            .iter()
            .flat_map(|f| f.iter())
            .copied()
            .max()
            .map_or(0, |m| m as usize + 1);
        tracing::debug!(
            elapsed_us = rank_time.as_micros(),
            unique_ranks,
            "step1_rank_reduce"
        );

        // Step 2: Concatenate with sentinels.
        let t0 = std::time::Instant::now();
        let (text, file_of, file_offsets) = concatenate_with_sentinels(&ranked_files);
        let concat_time = t0.elapsed();
        tracing::debug!(
            elapsed_us = concat_time.as_micros(),
            concat_len = text.len(),
            "step2_concatenate"
        );

        if text.is_empty() {
            return empty_report(total_files);
        }

        // Step 3: Build suffix array.
        let t0 = std::time::Instant::now();
        let sa = build_suffix_array(&text);
        let sa_time = t0.elapsed();
        tracing::debug!(
            elapsed_us = sa_time.as_micros(),
            n = text.len(),
            "step3_suffix_array"
        );

        // Step 4: Build LCP array (Kasai's algorithm, sentinel-aware).
        let t0 = std::time::Instant::now();
        let lcp = build_lcp(&text, &sa);
        let lcp_time = t0.elapsed();
        tracing::debug!(elapsed_us = lcp_time.as_micros(), "step4_lcp_array");

        // Step 5: Extract clone groups from LCP intervals.
        let t0 = std::time::Instant::now();
        let raw_groups =
            extract_clone_groups(&sa, &lcp, &file_of, &file_offsets, self.min_tokens, &files);
        let extract_time = t0.elapsed();
        tracing::debug!(
            elapsed_us = extract_time.as_micros(),
            raw_groups = raw_groups.len(),
            "step5_extract_groups"
        );

        // Step 6: Build CloneGroup structs with line info, apply filters.
        let t0 = std::time::Instant::now();
        let clone_groups = self.build_groups(raw_groups, &files);
        let build_time = t0.elapsed();
        tracing::debug!(
            elapsed_us = build_time.as_micros(),
            final_groups = clone_groups.len(),
            "step6_build_groups"
        );

        // Step 7: Compute stats.
        let t0 = std::time::Instant::now();
        let stats = compute_stats(&clone_groups, total_files, total_lines, total_tokens);
        let stats_time = t0.elapsed();
        tracing::debug!(elapsed_us = stats_time.as_micros(), "step7_compute_stats");

        tracing::info!(
            total_us = (rank_time
                + concat_time
                + sa_time
                + lcp_time
                + extract_time
                + build_time
                + stats_time)
                .as_micros(),
            rank_us = rank_time.as_micros(),
            sa_us = sa_time.as_micros(),
            lcp_us = lcp_time.as_micros(),
            extract_us = extract_time.as_micros(),
            build_us = build_time.as_micros(),
            stats_us = stats_time.as_micros(),
            total_tokens,
            clone_groups = clone_groups.len(),
            "clone detection complete"
        );

        DuplicationReport {
            clone_groups,
            stats,
        }
    }

    /// Convert raw groups into `CloneGroup` structs, applying min_lines and
    /// skip_local filters, deduplication, and subset removal.
    fn build_groups(&self, mut raw_groups: Vec<RawGroup>, files: &[FileData]) -> Vec<CloneGroup> {
        if raw_groups.is_empty() {
            return Vec::new();
        }

        // ── Token-level subset removal (cheap) ────────────────
        //
        // Sort raw groups by length desc. For each instance (file_id, offset),
        // track non-overlapping covered intervals per file. A smaller group is
        // skipped if all its instances fall within already-covered intervals.
        // This eliminates the vast majority of raw groups before the expensive
        // line-calculation step.
        let raw_count = raw_groups.len();
        raw_groups.sort_by(|a, b| b.length.cmp(&a.length));

        // covered[file_id] is a sorted vec of non-overlapping (start, end)
        // intervals. Kept sorted by start for binary search.
        let mut covered: Vec<Vec<(usize, usize)>> = vec![Vec::new(); files.len()];
        let mut surviving_groups: Vec<RawGroup> = Vec::new();

        for rg in raw_groups {
            let len = rg.length;
            // Check if all instances are fully covered by existing intervals.
            let all_covered = rg.instances.iter().all(|&(fid, offset)| {
                let intervals = &covered[fid];
                // Binary search for the interval that could contain [offset, offset+len).
                let idx = intervals.partition_point(|&(s, _)| s <= offset);
                if idx > 0 {
                    let (s, e) = intervals[idx - 1];
                    offset >= s && offset + len <= e
                } else {
                    false
                }
            });

            if !all_covered {
                // Insert covered intervals for this group's instances.
                for &(fid, offset) in &rg.instances {
                    let end = offset + len;
                    let intervals = &mut covered[fid];
                    let idx = intervals.partition_point(|&(s, _)| s < offset);
                    // Check if the new interval merges with an existing one.
                    if idx > 0 {
                        let prev = &mut intervals[idx - 1];
                        if prev.1 >= offset {
                            // Extend previous interval if needed.
                            if end > prev.1 {
                                prev.1 = end;
                            }
                            continue;
                        }
                    }
                    intervals.insert(idx, (offset, end));
                }
                surviving_groups.push(rg);
            }
        }

        tracing::trace!(
            raw = raw_count,
            surviving = surviving_groups.len(),
            "token-level subset removal"
        );

        // ── Pre-compute line offset tables ────────────────────
        //
        // For each file, build a sorted vec of newline byte positions so that
        // byte_offset_to_line_col can use binary search (O(log L)) instead of
        // linear scan (O(L)).
        let line_tables: Vec<Vec<usize>> = files
            .iter()
            .map(|f| {
                f.file_tokens
                    .source
                    .bytes()
                    .enumerate()
                    .filter_map(|(i, b)| if b == b'\n' { Some(i) } else { None })
                    .collect()
            })
            .collect();

        // ── Build CloneGroups for survivors ────────────────────
        let mut clone_groups: Vec<CloneGroup> = Vec::new();

        for rg in surviving_groups {
            // Build instances, deduplicating by (file_id, offset).
            let mut seen: HashSet<(usize, usize)> = HashSet::new();
            let mut group_instances: Vec<CloneInstance> = Vec::new();

            for &(file_id, offset) in &rg.instances {
                if !seen.insert((file_id, offset)) {
                    continue;
                }

                let file = &files[file_id];
                if let Some(inst) =
                    build_clone_instance_fast(file, offset, rg.length, &line_tables[file_id])
                {
                    group_instances.push(inst);
                }
            }

            // Apply skip_local: only keep cross-directory clones.
            if self.skip_local && group_instances.len() >= 2 {
                let dirs: std::collections::HashSet<_> = group_instances
                    .iter()
                    .filter_map(|inst| inst.file.parent().map(|p| p.to_path_buf()))
                    .collect();
                if dirs.len() < 2 {
                    continue;
                }
            }

            if group_instances.len() < 2 {
                continue;
            }

            // Calculate line count from the instances.
            let line_count = group_instances
                .iter()
                .map(|inst| inst.end_line.saturating_sub(inst.start_line) + 1)
                .max()
                .unwrap_or(0);

            // Apply minimum line filter.
            if line_count < self.min_lines {
                continue;
            }

            // Sort instances by file path then start line for stable output.
            group_instances
                .sort_by(|a, b| a.file.cmp(&b.file).then(a.start_line.cmp(&b.start_line)));

            // Deduplicate instances that map to overlapping line ranges within
            // the same file (different token offsets can resolve to overlapping
            // source spans). When two instances overlap, keep the wider one.
            group_instances.dedup_by(|b, a| {
                if a.file != b.file {
                    return false;
                }
                // Instances are sorted by start_line. `b` starts at or after `a`.
                // If b's start overlaps with a's range, merge by extending a.
                if b.start_line <= a.end_line {
                    // Keep the wider range in `a`.
                    if b.end_line > a.end_line {
                        a.end_line = b.end_line;
                        a.end_col = b.end_col;
                    }
                    true
                } else {
                    false
                }
            });

            if group_instances.len() < 2 {
                continue;
            }

            clone_groups.push(CloneGroup {
                instances: group_instances,
                token_count: rg.length,
                line_count,
            });
        }

        // Sort groups by token count (largest first), breaking ties by instance
        // count (most instances first). This ensures that for equal token counts,
        // N-way groups come before M-way (M<N) subsets, so subset removal works
        // correctly regardless of the suffix array's extraction order.
        clone_groups.sort_by(|a, b| {
            b.token_count
                .cmp(&a.token_count)
                .then(b.instances.len().cmp(&a.instances.len()))
        });

        // Remove groups whose line ranges are fully contained within another
        // group's line ranges. Uses a per-file interval index to avoid O(g²×m×n).
        //
        // Strategy: iterate groups from largest to smallest. For each kept group,
        // register its (file, start_line, end_line) spans into a spatial index.
        // Smaller groups are checked against this index in O(instances × log(intervals)).

        // Build file path → index mapping for interval tracking
        let mut path_to_idx: HashMap<PathBuf, usize> = HashMap::new();
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
        let mut file_intervals: Vec<Vec<(usize, usize)>> = vec![Vec::new(); next_idx];
        let mut kept_groups: Vec<CloneGroup> = Vec::new();

        for group in clone_groups {
            // Check if ALL instances of this group are contained within existing intervals
            let all_contained = group.instances.iter().all(|inst| {
                let fidx = path_to_idx[&inst.file];
                let intervals = &file_intervals[fidx];
                let idx = intervals.partition_point(|&(s, _)| s <= inst.start_line);
                idx > 0 && {
                    let (s, e) = intervals[idx - 1];
                    inst.start_line >= s && inst.end_line <= e
                }
            });

            if !all_contained {
                // Register this group's instances into the spatial index
                for inst in &group.instances {
                    let fidx = path_to_idx[&inst.file];
                    let intervals = &mut file_intervals[fidx];
                    let idx = intervals.partition_point(|&(s, _)| s < inst.start_line);
                    if idx > 0 && intervals[idx - 1].1 >= inst.start_line {
                        if inst.end_line > intervals[idx - 1].1 {
                            intervals[idx - 1].1 = inst.end_line;
                        }
                    } else {
                        intervals.insert(idx, (inst.start_line, inst.end_line));
                    }
                }
                kept_groups.push(group);
            }
        }

        kept_groups
    }
}

// ── Raw group from LCP extraction ──────────────────────────

/// A raw clone group before conversion to `CloneGroup`.
struct RawGroup {
    /// List of (file_id, token_offset) instances.
    instances: Vec<(usize, usize)>,
    /// Clone length in tokens.
    length: usize,
}

// ── Step 1: Rank reduction ─────────────────────────────────

/// Map all unique token hashes (u64) to consecutive integer ranks (u32).
///
/// Returns `ranked_files` where `ranked_files[i]` contains the rank
/// sequence for `files[i]`.
fn rank_reduce(files: &[FileData]) -> Vec<Vec<u32>> {
    // Single-pass: assign ranks on first encounter. The exact rank values
    // don't matter as long as equal hashes get equal ranks. Skipping the
    // sort+dedup saves O(N log N) and a second allocation.
    let total: usize = files.iter().map(|f| f.hashed_tokens.len()).sum();
    let mut hash_to_rank: HashMap<u64, u32> = HashMap::with_capacity(total / 2);
    let mut next_rank: u32 = 0;

    files
        .iter()
        .map(|file| {
            file.hashed_tokens
                .iter()
                .map(|ht| {
                    *hash_to_rank.entry(ht.hash).or_insert_with(|| {
                        let r = next_rank;
                        next_rank += 1;
                        r
                    })
                })
                .collect()
        })
        .collect()
}

// ── Step 2: Concatenation with sentinels ───────────────────

/// Concatenate all ranked token sequences into a single `Vec<i64>`,
/// inserting unique negative sentinel values between files.
///
/// Returns `(text, file_of, file_offsets)` where:
/// - `text` is the concatenated sequence
/// - `file_of[pos]` maps a position in `text` to a file index
///   (`usize::MAX` for sentinel positions)
/// - `file_offsets[file_id]` is the starting position of file `file_id`
///   in `text`
fn concatenate_with_sentinels(ranked_files: &[Vec<u32>]) -> (Vec<i64>, Vec<usize>, Vec<usize>) {
    let sentinel_count = ranked_files.len().saturating_sub(1);
    let total_len: usize = ranked_files.iter().map(|f| f.len()).sum::<usize>() + sentinel_count;

    let mut text = Vec::with_capacity(total_len);
    let mut file_of = Vec::with_capacity(total_len);
    let mut file_offsets = Vec::with_capacity(ranked_files.len());

    let mut sentinel: i64 = -1;

    for (file_id, ranks) in ranked_files.iter().enumerate() {
        file_offsets.push(text.len());

        for &r in ranks {
            text.push(r as i64);
            file_of.push(file_id);
        }

        // Insert sentinel between files (not after the last one).
        if file_id + 1 < ranked_files.len() {
            text.push(sentinel);
            file_of.push(usize::MAX);
            sentinel -= 1;
        }
    }

    (text, file_of, file_offsets)
}

// ── Step 3: Suffix array construction (prefix doubling) ────

/// Build a suffix array using the O(N log N) prefix-doubling algorithm with
/// radix sort.
///
/// Returns `sa` where `sa[i]` is the starting position of the i-th
/// lexicographically smallest suffix in `text`.
fn build_suffix_array(text: &[i64]) -> Vec<usize> {
    let n = text.len();
    if n == 0 {
        return vec![];
    }

    // Initial ranks based on raw values. Shift so sentinels (negative) sort
    // before all real tokens.
    let min_val = text.iter().copied().min().unwrap_or(0);
    let mut rank: Vec<i64> = text.iter().map(|&v| v - min_val).collect();
    let mut sa: Vec<usize> = (0..n).collect();
    let mut tmp: Vec<i64> = vec![0; n];
    let mut k: usize = 1;
    let mut iterations = 0u32;

    // Scratch buffers for radix sort (reused across iterations).
    let mut sa_tmp: Vec<usize> = vec![0; n];

    while k < n {
        iterations += 1;
        let max_rank = rank.iter().copied().max().unwrap_or(0) as usize;

        // Two-pass radix sort: sort by secondary key (rank[i+k]) first,
        // then by primary key (rank[i]). Each pass is O(N + K) where
        // K = max_rank + 2 (including the -1 sentinel rank).
        let bucket_count = max_rank + 2; // ranks 0..=max_rank plus -1 mapped to 0

        // Pass 1: sort by secondary key (rank at offset k).
        let mut counts = vec![0usize; bucket_count + 1];
        for &i in &sa {
            let r2 = if i + k < n {
                rank[i + k] as usize + 1
            } else {
                0
            };
            counts[r2] += 1;
        }
        // Prefix sum.
        let mut sum = 0;
        for c in counts.iter_mut() {
            let v = *c;
            *c = sum;
            sum += v;
        }
        for &i in &sa {
            let r2 = if i + k < n {
                rank[i + k] as usize + 1
            } else {
                0
            };
            sa_tmp[counts[r2]] = i;
            counts[r2] += 1;
        }

        // Pass 2: sort by primary key (rank[i]), stable.
        // No +1 offset needed here: rank[i] is always >= 0 because the
        // initial ranks are shifted by min_val, and subsequent iterations
        // assign ranks starting from 0.
        counts.iter_mut().for_each(|c| *c = 0);
        counts.resize(bucket_count + 1, 0);
        for &i in &sa_tmp {
            let r1 = rank[i] as usize;
            counts[r1] += 1;
        }
        sum = 0;
        for c in counts.iter_mut() {
            let v = *c;
            *c = sum;
            sum += v;
        }
        for &i in &sa_tmp {
            let r1 = rank[i] as usize;
            sa[counts[r1]] = i;
            counts[r1] += 1;
        }

        // Compute new ranks.
        tmp[sa[0]] = 0;
        for i in 1..n {
            let prev = sa[i - 1];
            let curr = sa[i];
            let same = rank[prev] == rank[curr] && {
                let rp2 = if prev + k < n { rank[prev + k] } else { -1 };
                let rc2 = if curr + k < n { rank[curr + k] } else { -1 };
                rp2 == rc2
            };
            tmp[curr] = tmp[prev] + if same { 0 } else { 1 };
        }

        // Early exit when all ranks are unique.
        let new_max_rank = tmp[sa[n - 1]];
        std::mem::swap(&mut rank, &mut tmp);

        if new_max_rank as usize == n - 1 {
            break;
        }

        k *= 2;
    }

    tracing::trace!(n, iterations, "suffix array constructed");
    sa
}

// ── Step 4: LCP array (Kasai's algorithm) ──────────────────

/// Build the LCP (Longest Common Prefix) array using Kasai's algorithm.
///
/// `lcp[i]` is the length of the longest common prefix between suffixes
/// `sa[i]` and `sa[i-1]`. `lcp[0]` is always 0.
///
/// The LCP computation stops at sentinel boundaries (negative values in
/// `text`) to prevent matches from crossing file boundaries.
fn build_lcp(text: &[i64], sa: &[usize]) -> Vec<usize> {
    let n = sa.len();
    if n == 0 {
        return vec![];
    }

    let mut rank = vec![0usize; n];
    for i in 0..n {
        rank[sa[i]] = i;
    }

    let mut lcp = vec![0usize; n];
    let mut k: usize = 0;

    for i in 0..n {
        if rank[i] == 0 {
            k = 0;
            continue;
        }
        let j = sa[rank[i] - 1];
        while i + k < n && j + k < n {
            // Stop at sentinels (negative values).
            if text[i + k] < 0 || text[j + k] < 0 {
                break;
            }
            if text[i + k] != text[j + k] {
                break;
            }
            k += 1;
        }
        lcp[rank[i]] = k;
        k = k.saturating_sub(1);
    }

    lcp
}

// ── Step 5: Clone group extraction ─────────────────────────

/// Extract clone groups from the suffix array and LCP array.
///
/// Uses a stack-based approach to find all maximal LCP intervals where the
/// minimum LCP value is >= `min_tokens`, and the interval contains suffixes
/// from at least two different positions (cross-file or non-overlapping
/// same-file).
fn extract_clone_groups(
    sa: &[usize],
    lcp: &[usize],
    file_of: &[usize],
    file_offsets: &[usize],
    min_tokens: usize,
    files: &[FileData],
) -> Vec<RawGroup> {
    let n = sa.len();
    if n < 2 {
        return vec![];
    }

    // Stack-based LCP interval extraction.
    // Each stack entry: (lcp_value, start_index_in_sa).
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let mut groups: Vec<RawGroup> = Vec::new();

    #[allow(clippy::needless_range_loop)] // `i` is used as a value, not just as an index
    for i in 1..=n {
        let cur_lcp = if i < n { lcp[i] } else { 0 };
        let mut start = i;

        while let Some(&(top_lcp, top_start)) = stack.last() {
            if top_lcp <= cur_lcp {
                break;
            }
            stack.pop();
            start = top_start;

            if top_lcp >= min_tokens {
                // The interval [start-1 .. i-1] shares a common prefix of
                // length `top_lcp`.
                let interval_begin = start - 1;
                let interval_end = i; // exclusive

                if let Some(group) = build_raw_group(
                    sa,
                    file_of,
                    file_offsets,
                    files,
                    interval_begin,
                    interval_end,
                    top_lcp,
                ) {
                    groups.push(group);
                }
            }
        }

        if i < n {
            stack.push((cur_lcp, start));
        }
    }

    groups
}

/// Build a `RawGroup` from an LCP interval, filtering to non-overlapping
/// instances.
fn build_raw_group(
    sa: &[usize],
    file_of: &[usize],
    file_offsets: &[usize],
    files: &[FileData],
    begin: usize,
    end: usize,
    length: usize,
) -> Option<RawGroup> {
    let mut instances: Vec<(usize, usize)> = Vec::new();

    for &pos in &sa[begin..end] {
        let fid = file_of[pos];
        if fid == usize::MAX {
            continue; // sentinel position
        }
        let offset_in_file = pos - file_offsets[fid];

        // Verify the clone doesn't extend beyond the file boundary.
        if offset_in_file + length > files[fid].hashed_tokens.len() {
            continue;
        }

        instances.push((fid, offset_in_file));
    }

    if instances.len() < 2 {
        return None;
    }

    // Remove overlapping instances within the same file.
    // Sort by (file_id, offset) and remove overlaps.
    instances.sort_unstable();
    let mut deduped: Vec<(usize, usize)> = Vec::with_capacity(instances.len());
    for &(fid, offset) in &instances {
        if let Some(&(last_fid, last_offset)) = deduped.last()
            && fid == last_fid
            && offset < last_offset + length
        {
            continue; // overlapping within the same file
        }
        deduped.push((fid, offset));
    }

    if deduped.len() < 2 {
        return None;
    }

    Some(RawGroup {
        instances: deduped,
        length,
    })
}

// ── Utility functions ──────────────────────────────────────

/// Check if all instances of `smaller` overlap with instances of `larger`.
/// Build a `CloneInstance` using a pre-computed line offset table for fast lookup.
fn build_clone_instance_fast(
    file: &FileData,
    token_offset: usize,
    token_length: usize,
    line_table: &[usize],
) -> Option<CloneInstance> {
    let tokens = &file.hashed_tokens;
    let source_tokens = &file.file_tokens.tokens;

    if token_offset + token_length > tokens.len() {
        return None;
    }

    // Map from hashed token indices back to source token spans.
    let first_hashed = &tokens[token_offset];
    let last_hashed = &tokens[token_offset + token_length - 1];

    let first_source = &source_tokens[first_hashed.original_index];
    let last_source = &source_tokens[last_hashed.original_index];

    let start_byte = first_source.span.start as usize;
    let end_byte = last_source.span.end as usize;

    // Guard against inverted spans that can occur when normalization reorders
    // token original_index values for very small windows.
    if start_byte > end_byte {
        return None;
    }

    let source = &file.file_tokens.source;
    let (start_line, start_col) = byte_offset_to_line_col_fast(source, start_byte, line_table);
    let (end_line, end_col) = byte_offset_to_line_col_fast(source, end_byte, line_table);

    // Extract the fragment.
    let fragment = if end_byte <= source.len() {
        source[start_byte..end_byte].to_string()
    } else {
        String::new()
    };

    Some(CloneInstance {
        file: file.path.clone(),
        start_line,
        end_line,
        start_col,
        end_col,
        fragment,
    })
}

/// Convert a byte offset into a 1-based line number and 0-based character column
/// using a pre-computed table of newline positions for O(log L) lookup.
fn byte_offset_to_line_col_fast(
    source: &str,
    byte_offset: usize,
    line_table: &[usize],
) -> (usize, usize) {
    let offset = byte_offset.min(source.len());
    // Binary search: find the number of newlines before this offset.
    let line_idx = line_table.partition_point(|&nl_pos| nl_pos < offset);
    let line = line_idx + 1; // 1-based
    let line_start = if line_idx == 0 {
        0
    } else {
        line_table[line_idx - 1] + 1
    };
    let col = source[line_start..offset].chars().count();
    (line, col)
}

/// Convert a byte offset into a 1-based line number and 0-based character column.
#[cfg(test)]
fn byte_offset_to_line_col(source: &str, byte_offset: usize) -> (usize, usize) {
    let offset = byte_offset.min(source.len());
    let before = &source[..offset];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |pos| pos + 1);
    let col = before[line_start..].chars().count();
    (line, col)
}

/// Compute aggregate duplication statistics.
fn compute_stats(
    clone_groups: &[CloneGroup],
    total_files: usize,
    total_lines: usize,
    total_tokens: usize,
) -> DuplicationStats {
    use std::collections::HashSet;
    use std::path::Path;

    let mut files_with_clones: HashSet<&Path> = HashSet::new();
    // Group duplicated lines by file to avoid cloning PathBuf per line.
    let mut file_dup_lines: HashMap<&Path, HashSet<usize>> = HashMap::new();
    let mut duplicated_tokens = 0usize;
    let mut clone_instances = 0usize;

    for group in clone_groups {
        for instance in &group.instances {
            files_with_clones.insert(&instance.file);
            clone_instances += 1;
            let lines = file_dup_lines.entry(&instance.file).or_default();
            for line in instance.start_line..=instance.end_line {
                lines.insert(line);
            }
        }
        // Each instance contributes token_count duplicated tokens,
        // but only count duplicates (all instances beyond the first).
        if group.instances.len() > 1 {
            duplicated_tokens += group.token_count * (group.instances.len() - 1);
        }
    }

    let dup_line_count: usize = file_dup_lines.values().map(|s| s.len()).sum();
    let duplication_percentage = if total_lines > 0 {
        (dup_line_count as f64 / total_lines as f64) * 100.0
    } else {
        0.0
    };

    // Cap duplicated_tokens to total_tokens to avoid impossible values
    // when overlapping clone groups double-count the same token positions.
    let duplicated_tokens = duplicated_tokens.min(total_tokens);

    DuplicationStats {
        total_files,
        files_with_clones: files_with_clones.len(),
        total_lines,
        duplicated_lines: dup_line_count,
        total_tokens,
        duplicated_tokens,
        clone_groups: clone_groups.len(),
        clone_instances,
        duplication_percentage,
    }
}

/// Create an empty report when there are no files to analyze.
fn empty_report(total_files: usize) -> DuplicationReport {
    DuplicationReport {
        clone_groups: vec![],
        stats: DuplicationStats {
            total_files,
            files_with_clones: 0,
            total_lines: 0,
            duplicated_lines: 0,
            total_tokens: 0,
            duplicated_tokens: 0,
            clone_groups: 0,
            clone_instances: 0,
            duplication_percentage: 0.0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duplicates::normalize::HashedToken;
    use crate::duplicates::tokenize::{FileTokens, SourceToken, TokenKind};
    use oxc_span::Span;

    fn make_hashed_tokens(hashes: &[u64]) -> Vec<HashedToken> {
        hashes
            .iter()
            .enumerate()
            .map(|(i, &hash)| HashedToken {
                hash,
                original_index: i,
            })
            .collect()
    }

    fn make_source_tokens(count: usize) -> Vec<SourceToken> {
        (0..count)
            .map(|i| SourceToken {
                kind: TokenKind::Identifier(format!("t{i}")),
                span: Span::new((i * 3) as u32, (i * 3 + 2) as u32),
            })
            .collect()
    }

    fn make_file_tokens(source: &str, count: usize) -> FileTokens {
        FileTokens {
            tokens: make_source_tokens(count),
            source: source.to_string(),
            line_count: source.lines().count().max(1),
        }
    }

    // ── Existing tests (adapted for CloneDetector) ─────────

    #[test]
    fn empty_input_produces_empty_report() {
        let detector = CloneDetector::new(5, 1, false);
        let report = detector.detect(vec![]);
        assert!(report.clone_groups.is_empty());
        assert_eq!(report.stats.total_files, 0);
    }

    #[test]
    fn single_file_no_clones() {
        let detector = CloneDetector::new(3, 1, false);
        let hashed = make_hashed_tokens(&[1, 2, 3, 4, 5]);
        let ft = make_file_tokens("a b c d e", 5);
        let report = detector.detect(vec![(PathBuf::from("a.ts"), hashed, ft)]);
        assert!(report.clone_groups.is_empty());
    }

    #[test]
    fn detects_exact_duplicate_across_files() {
        let detector = CloneDetector::new(3, 1, false);

        // Same token sequence in two files.
        let hashes = vec![10, 20, 30, 40, 50];
        let source_a = "a\nb\nc\nd\ne";
        let source_b = "a\nb\nc\nd\ne";

        let hashed_a = make_hashed_tokens(&hashes);
        let hashed_b = make_hashed_tokens(&hashes);
        let ft_a = make_file_tokens(source_a, 5);
        let ft_b = make_file_tokens(source_b, 5);

        let report = detector.detect(vec![
            (PathBuf::from("a.ts"), hashed_a, ft_a),
            (PathBuf::from("b.ts"), hashed_b, ft_b),
        ]);

        assert!(
            !report.clone_groups.is_empty(),
            "Should detect at least one clone group"
        );
    }

    #[test]
    fn no_detection_below_min_tokens() {
        let detector = CloneDetector::new(10, 1, false);

        let hashes = vec![10, 20, 30]; // Only 3 tokens, min is 10
        let hashed_a = make_hashed_tokens(&hashes);
        let hashed_b = make_hashed_tokens(&hashes);
        let ft_a = make_file_tokens("abc", 3);
        let ft_b = make_file_tokens("abc", 3);

        let report = detector.detect(vec![
            (PathBuf::from("a.ts"), hashed_a, ft_a),
            (PathBuf::from("b.ts"), hashed_b, ft_b),
        ]);

        assert!(report.clone_groups.is_empty());
    }

    #[test]
    fn byte_offset_to_line_col_basic() {
        let source = "abc\ndef\nghi";
        assert_eq!(byte_offset_to_line_col(source, 0), (1, 0));
        assert_eq!(byte_offset_to_line_col(source, 4), (2, 0));
        assert_eq!(byte_offset_to_line_col(source, 5), (2, 1));
        assert_eq!(byte_offset_to_line_col(source, 8), (3, 0));
    }

    #[test]
    fn byte_offset_beyond_source() {
        let source = "abc";
        // Should clamp to end of source.
        let (line, col) = byte_offset_to_line_col(source, 100);
        assert_eq!(line, 1);
        assert_eq!(col, 3);
    }

    #[test]
    fn skip_local_filters_same_directory() {
        let detector = CloneDetector::new(3, 1, true);

        let hashes = vec![10, 20, 30, 40, 50];
        let source = "a\nb\nc\nd\ne";

        let hashed_a = make_hashed_tokens(&hashes);
        let hashed_b = make_hashed_tokens(&hashes);
        let ft_a = make_file_tokens(source, 5);
        let ft_b = make_file_tokens(source, 5);

        // Same directory -> should be filtered with skip_local.
        let report = detector.detect(vec![
            (PathBuf::from("src/a.ts"), hashed_a, ft_a),
            (PathBuf::from("src/b.ts"), hashed_b, ft_b),
        ]);

        assert!(
            report.clone_groups.is_empty(),
            "Same-directory clones should be filtered with skip_local"
        );
    }

    #[test]
    fn skip_local_keeps_cross_directory() {
        let detector = CloneDetector::new(3, 1, true);

        let hashes = vec![10, 20, 30, 40, 50];
        let source = "a\nb\nc\nd\ne";

        let hashed_a = make_hashed_tokens(&hashes);
        let hashed_b = make_hashed_tokens(&hashes);
        let ft_a = make_file_tokens(source, 5);
        let ft_b = make_file_tokens(source, 5);

        // Different directories -> should be kept.
        let report = detector.detect(vec![
            (PathBuf::from("src/components/a.ts"), hashed_a, ft_a),
            (PathBuf::from("src/utils/b.ts"), hashed_b, ft_b),
        ]);

        assert!(
            !report.clone_groups.is_empty(),
            "Cross-directory clones should be kept with skip_local"
        );
    }

    #[test]
    fn stats_computation() {
        let groups = vec![CloneGroup {
            instances: vec![
                CloneInstance {
                    file: PathBuf::from("a.ts"),
                    start_line: 1,
                    end_line: 5,
                    start_col: 0,
                    end_col: 10,
                    fragment: "...".to_string(),
                },
                CloneInstance {
                    file: PathBuf::from("b.ts"),
                    start_line: 10,
                    end_line: 14,
                    start_col: 0,
                    end_col: 10,
                    fragment: "...".to_string(),
                },
            ],
            token_count: 50,
            line_count: 5,
        }];

        let stats = compute_stats(&groups, 10, 200, 1000);
        assert_eq!(stats.total_files, 10);
        assert_eq!(stats.files_with_clones, 2);
        assert_eq!(stats.clone_groups, 1);
        assert_eq!(stats.clone_instances, 2);
        assert_eq!(stats.duplicated_lines, 10); // 5 lines in each of 2 instances
        assert!(stats.duplication_percentage > 0.0);
    }

    // ── New suffix array / LCP tests ───────────────────────

    #[test]
    fn sa_construction_basic() {
        // "banana" encoded as integers: b=1, a=0, n=2
        let text: Vec<i64> = vec![1, 0, 2, 0, 2, 0];
        let sa = build_suffix_array(&text);

        // Suffixes sorted lexicographically:
        // SA[0] = 5: "a"           (0)
        // SA[1] = 3: "ana"         (0,2,0)
        // SA[2] = 1: "anana"       (0,2,0,2,0)
        // SA[3] = 0: "banana"      (1,0,2,0,2,0)
        // SA[4] = 4: "na"          (2,0)
        // SA[5] = 2: "nana"        (2,0,2,0)
        assert_eq!(sa, vec![5, 3, 1, 0, 4, 2]);
    }

    #[test]
    fn lcp_construction_basic() {
        let text: Vec<i64> = vec![1, 0, 2, 0, 2, 0];
        let sa = build_suffix_array(&text);
        let lcp = build_lcp(&text, &sa);

        // LCP values for "banana":
        // lcp[0] = 0 (by definition)
        // lcp[1] = 1 (LCP of "a" and "ana" = "a" = 1)
        // lcp[2] = 3 (LCP of "ana" and "anana" = "ana" = 3)
        // lcp[3] = 0 (LCP of "anana" and "banana" = "" = 0)
        // lcp[4] = 0 (LCP of "banana" and "na" = "" = 0)
        // lcp[5] = 2 (LCP of "na" and "nana" = "na" = 2)
        assert_eq!(lcp, vec![0, 1, 3, 0, 0, 2]);
    }

    #[test]
    fn lcp_stops_at_sentinels() {
        // Two "files": [0, 1, 2] sentinel [-1] [0, 1, 2]
        let text: Vec<i64> = vec![0, 1, 2, -1, 0, 1, 2];
        let sa = build_suffix_array(&text);
        let lcp = build_lcp(&text, &sa);

        // Find the SA positions corresponding to text positions 0 and 4
        // (both start "0 1 2 ..."). LCP should be exactly 3.
        let rank_0 = sa.iter().position(|&s| s == 0).expect("pos 0 in SA");
        let rank_4 = sa.iter().position(|&s| s == 4).expect("pos 4 in SA");
        let (lo, hi) = if rank_0 < rank_4 {
            (rank_0, rank_4)
        } else {
            (rank_4, rank_0)
        };

        // The minimum LCP in the range (lo, hi] gives the LCP between them.
        let min_lcp = lcp[(lo + 1)..=hi].iter().copied().min().unwrap_or(0);
        assert_eq!(
            min_lcp, 3,
            "LCP between identical sequences across sentinel should be 3"
        );
    }

    #[test]
    fn rank_reduction_maps_correctly() {
        let files = vec![
            FileData {
                path: PathBuf::from("a.ts"),
                hashed_tokens: make_hashed_tokens(&[100, 200, 300]),
                file_tokens: make_file_tokens("a b c", 3),
            },
            FileData {
                path: PathBuf::from("b.ts"),
                hashed_tokens: make_hashed_tokens(&[200, 300, 400]),
                file_tokens: make_file_tokens("d e f", 3),
            },
        ];

        let ranked = rank_reduce(&files);

        // Unique hashes: 100, 200, 300, 400 -> ranks 0, 1, 2, 3
        assert_eq!(ranked[0], vec![0, 1, 2]);
        assert_eq!(ranked[1], vec![1, 2, 3]);
    }

    #[test]
    fn three_file_grouping() {
        let detector = CloneDetector::new(3, 1, false);

        let hashes = vec![10, 20, 30, 40, 50];
        let source = "a\nb\nc\nd\ne";

        let data: Vec<(PathBuf, Vec<HashedToken>, FileTokens)> = (0..3)
            .map(|i| {
                (
                    PathBuf::from(format!("file{i}.ts")),
                    make_hashed_tokens(&hashes),
                    make_file_tokens(source, 5),
                )
            })
            .collect();

        let report = detector.detect(data);

        assert!(
            !report.clone_groups.is_empty(),
            "Should detect clones across 3 identical files"
        );

        // The largest group should contain 3 instances.
        let max_instances = report
            .clone_groups
            .iter()
            .map(|g| g.instances.len())
            .max()
            .unwrap_or(0);
        assert_eq!(
            max_instances, 3,
            "3 identical files should produce a group with 3 instances"
        );
    }

    #[test]
    fn overlapping_clones_largest_wins() {
        let detector = CloneDetector::new(3, 1, false);

        // File A and B: identical 10-token sequences.
        let hashes: Vec<u64> = (1..=10).collect();
        let source = "a\nb\nc\nd\ne\nf\ng\nh\ni\nj";

        let hashed_a = make_hashed_tokens(&hashes);
        let hashed_b = make_hashed_tokens(&hashes);
        let ft_a = make_file_tokens(source, 10);
        let ft_b = make_file_tokens(source, 10);

        let report = detector.detect(vec![
            (PathBuf::from("a.ts"), hashed_a, ft_a),
            (PathBuf::from("b.ts"), hashed_b, ft_b),
        ]);

        assert!(!report.clone_groups.is_empty());
        // The first group (sorted by token_count desc) should cover all 10.
        assert_eq!(
            report.clone_groups[0].token_count, 10,
            "Maximal clone should cover all 10 tokens"
        );
    }

    #[test]
    fn no_self_overlap() {
        let detector = CloneDetector::new(3, 1, false);

        // File with repeated pattern: [1,2,3,1,2,3]
        // The pattern [1,2,3] appears at offset 0 and offset 3.
        let hashes = vec![1, 2, 3, 1, 2, 3];
        // Source must be long enough for synthetic spans: token i has span (i*3, i*3+2).
        // Last token (5) has span (15, 17), so source must be >= 17 bytes.
        // Use a source with enough content spread across distinct lines.
        let source = "aa\nbb\ncc\ndd\nee\nff\ngg";

        let hashed = make_hashed_tokens(&hashes);
        let ft = make_file_tokens(source, 6);

        let report = detector.detect(vec![(PathBuf::from("a.ts"), hashed, ft)]);

        // Verify that no clone instance overlaps with another in the same file.
        for group in &report.clone_groups {
            let mut file_instances: HashMap<&PathBuf, Vec<(usize, usize)>> = HashMap::new();
            for inst in &group.instances {
                file_instances
                    .entry(&inst.file)
                    .or_default()
                    .push((inst.start_line, inst.end_line));
            }
            for (_file, mut ranges) in file_instances {
                ranges.sort();
                for w in ranges.windows(2) {
                    assert!(
                        w[1].0 > w[0].1,
                        "Clone instances in the same file should not overlap: {:?} and {:?}",
                        w[0],
                        w[1]
                    );
                }
            }
        }
    }

    #[test]
    fn empty_input_edge_case() {
        let detector = CloneDetector::new(0, 0, false);
        let report = detector.detect(vec![]);
        assert!(report.clone_groups.is_empty());
        assert_eq!(report.stats.total_files, 0);
    }

    #[test]
    fn single_file_internal_duplication() {
        let detector = CloneDetector::new(3, 1, false);

        // File with a repeated block separated by a different token.
        // [10, 20, 30, 99, 10, 20, 30]
        let hashes = vec![10, 20, 30, 99, 10, 20, 30];
        let source = "a\nb\nc\nx\na\nb\nc";

        let hashed = make_hashed_tokens(&hashes);
        let ft = make_file_tokens(source, 7);

        let report = detector.detect(vec![(PathBuf::from("a.ts"), hashed, ft)]);

        // Should detect the [10, 20, 30] clone at offsets 0 and 4.
        assert!(
            !report.clone_groups.is_empty(),
            "Should detect internal duplication within a single file"
        );
    }
}
