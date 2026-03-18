use std::collections::HashMap;
use std::path::PathBuf;

use xxhash_rust::xxh3::xxh3_64;

use super::normalize::HashedToken;
use super::tokenize::FileTokens;
use super::types::{CloneGroup, CloneInstance, DuplicationReport, DuplicationStats};

/// Location of a frame (sliding window position) within a file.
#[derive(Debug, Clone)]
struct FrameLocation {
    /// Index into the `file_data` vector.
    file_id: usize,
    /// Offset into the file's hashed token sequence.
    token_offset: usize,
}

/// Data for a single file being analyzed.
struct FileData {
    path: PathBuf,
    hashed_tokens: Vec<HashedToken>,
    file_tokens: FileTokens,
}

/// Sliding-window hash-based clone detection engine.
///
/// Uses a sliding window of `min_tokens` size, hashing each window with xxh3.
/// Matching windows across files indicate duplicate code blocks, which are then
/// extended to their maximal length and grouped via union-find.
pub struct RabinKarpDetector {
    /// Minimum clone size in tokens.
    min_tokens: usize,
    /// Minimum clone size in lines.
    min_lines: usize,
    /// Only report cross-directory duplicates.
    skip_local: bool,
}

impl RabinKarpDetector {
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
    /// `file_tokens` is a list of (path, hashed_tokens, file_tokens) tuples,
    /// one per analyzed file.
    pub fn detect(
        &self,
        file_data: Vec<(PathBuf, Vec<HashedToken>, FileTokens)>,
    ) -> DuplicationReport {
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

        // Compute total stats
        let total_files = files.len();
        let total_lines: usize = files.iter().map(|f| f.file_tokens.line_count).sum();
        let total_tokens: usize = files.iter().map(|f| f.hashed_tokens.len()).sum();

        // Step 1: Build frame hash index using sliding window
        let frame_index = self.build_frame_index(&files);

        // Step 2: Find matching frame pairs and extend into maximal clones
        let raw_clones = self.find_clones(&frame_index, &files);

        // Step 3: Group clones and deduplicate
        let clone_groups = self.group_clones(raw_clones, &files);

        // Step 4: Compute stats
        let stats = compute_stats(&clone_groups, total_files, total_lines, total_tokens);

        DuplicationReport {
            clone_groups,
            stats,
        }
    }

    /// Build the frame hash index: for each sliding window position, store its hash
    /// and location.
    fn build_frame_index(&self, files: &[FileData]) -> HashMap<u64, Vec<FrameLocation>> {
        let mut index: HashMap<u64, Vec<FrameLocation>> = HashMap::new();

        for (file_id, file) in files.iter().enumerate() {
            let tokens = &file.hashed_tokens;
            if tokens.len() < self.min_tokens {
                continue;
            }

            for offset in 0..=(tokens.len() - self.min_tokens) {
                let frame_hash = compute_frame_hash(tokens, offset, self.min_tokens);
                index.entry(frame_hash).or_default().push(FrameLocation {
                    file_id,
                    token_offset: offset,
                });
            }
        }

        index
    }

    /// Find clone pairs from matching frames and extend them into maximal clones.
    fn find_clones(
        &self,
        frame_index: &HashMap<u64, Vec<FrameLocation>>,
        files: &[FileData],
    ) -> Vec<RawClone> {
        let mut clones: Vec<RawClone> = Vec::new();
        let mut visited: HashMap<(usize, usize, usize, usize), bool> = HashMap::new();

        // Iterate over all frame hash buckets with multiple entries
        for locations in frame_index.values() {
            if locations.len() < 2 {
                continue;
            }

            // Compare all pairs within this bucket
            for i in 0..locations.len() {
                for j in (i + 1)..locations.len() {
                    let loc_a = &locations[i];
                    let loc_b = &locations[j];

                    // Skip self-overlapping matches within the same file
                    if loc_a.file_id == loc_b.file_id {
                        let (lo, hi) = if loc_a.token_offset <= loc_b.token_offset {
                            (loc_a.token_offset, loc_b.token_offset)
                        } else {
                            (loc_b.token_offset, loc_a.token_offset)
                        };
                        if hi < lo + self.min_tokens {
                            continue;
                        }
                    }

                    // Skip if we already processed this pair's starting position
                    let pair_key = (
                        loc_a.file_id,
                        loc_a.token_offset,
                        loc_b.file_id,
                        loc_b.token_offset,
                    );
                    if visited.contains_key(&pair_key) {
                        continue;
                    }

                    // Extend the match to find the maximal clone length
                    let tokens_a = &files[loc_a.file_id].hashed_tokens;
                    let tokens_b = &files[loc_b.file_id].hashed_tokens;
                    let max_extend = (tokens_a.len() - loc_a.token_offset)
                        .min(tokens_b.len() - loc_b.token_offset);

                    let mut match_len = self.min_tokens;
                    while match_len < max_extend
                        && tokens_a[loc_a.token_offset + match_len].hash
                            == tokens_b[loc_b.token_offset + match_len].hash
                    {
                        match_len += 1;
                    }

                    // Mark all sub-positions as visited to avoid reporting smaller subsets
                    for offset in 0..match_len.saturating_sub(self.min_tokens - 1) {
                        visited.insert(
                            (
                                loc_a.file_id,
                                loc_a.token_offset + offset,
                                loc_b.file_id,
                                loc_b.token_offset + offset,
                            ),
                            true,
                        );
                    }

                    clones.push(RawClone {
                        file_a: loc_a.file_id,
                        offset_a: loc_a.token_offset,
                        file_b: loc_b.file_id,
                        offset_b: loc_b.token_offset,
                        length: match_len,
                    });
                }
            }
        }

        clones
    }

    /// Group raw clone pairs into clone groups using union-find.
    fn group_clones(&self, raw_clones: Vec<RawClone>, files: &[FileData]) -> Vec<CloneGroup> {
        if raw_clones.is_empty() {
            return vec![];
        }

        // Build a map from (file_id, offset, length) -> group_id using union-find
        let mut parent: Vec<usize> = (0..raw_clones.len() * 2).collect();

        // Each raw clone produces two "instance" indices
        // Instance 2*i -> (file_a, offset_a, length)
        // Instance 2*i+1 -> (file_b, offset_b, length)
        // We group instances that share the same content hash and length

        // Content hash for each clone's instance to merge identical blocks
        let mut content_keys: HashMap<(u64, usize), Vec<usize>> = HashMap::new();

        for (i, clone) in raw_clones.iter().enumerate() {
            let hash_a = compute_frame_hash(
                &files[clone.file_a].hashed_tokens,
                clone.offset_a,
                clone.length,
            );
            content_keys
                .entry((hash_a, clone.length))
                .or_default()
                .push(i);
        }

        // Union all clones with the same content hash
        for instances in content_keys.values() {
            if instances.len() > 1 {
                let first = instances[0];
                for &other in &instances[1..] {
                    union(&mut parent, first, other);
                }
            }
        }

        // Build groups: group_root -> Vec<(file_id, offset, length)>
        let mut groups: HashMap<usize, Vec<(usize, usize, usize)>> = HashMap::new();

        for (i, clone) in raw_clones.iter().enumerate() {
            let root = find(&mut parent, i);
            let entry = groups.entry(root).or_default();

            // Add both instances, dedup later
            entry.push((clone.file_a, clone.offset_a, clone.length));
            entry.push((clone.file_b, clone.offset_b, clone.length));
        }

        // Convert to CloneGroups
        let mut clone_groups: Vec<CloneGroup> = Vec::new();

        for instances in groups.values() {
            // Use the maximum length in the group
            let max_len = instances.iter().map(|&(_, _, l)| l).max().unwrap_or(0);

            // Deduplicate instances (same file + same offset)
            let mut seen: HashMap<(usize, usize), bool> = HashMap::new();
            let mut group_instances: Vec<CloneInstance> = Vec::new();

            for &(file_id, offset, _) in instances {
                if seen.contains_key(&(file_id, offset)) {
                    continue;
                }
                seen.insert((file_id, offset), true);

                let file = &files[file_id];
                let instance = build_clone_instance(file, offset, max_len);

                if let Some(inst) = instance {
                    group_instances.push(inst);
                }
            }

            // Apply skip_local: only keep cross-directory clones
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

            // Calculate line count from the instances
            let line_count = group_instances
                .iter()
                .map(|inst| inst.end_line.saturating_sub(inst.start_line) + 1)
                .max()
                .unwrap_or(0);

            // Apply minimum line filter
            if line_count < self.min_lines {
                continue;
            }

            // Sort instances by file path then start line for stable output
            group_instances
                .sort_by(|a, b| a.file.cmp(&b.file).then(a.start_line.cmp(&b.start_line)));

            clone_groups.push(CloneGroup {
                instances: group_instances,
                token_count: max_len,
                line_count,
            });
        }

        // Sort groups by token count (largest first) for better display
        clone_groups.sort_by(|a, b| b.token_count.cmp(&a.token_count));

        // Remove groups that are subsets of larger groups.
        // A group is a subset if ALL of its instances overlap with instances in a larger group.
        let mut keep = vec![true; clone_groups.len()];
        for i in 0..clone_groups.len() {
            if !keep[i] {
                continue;
            }
            for j in (i + 1)..clone_groups.len() {
                if !keep[j] {
                    continue;
                }
                // Check if group j is a subset of group i (group i is larger or equal in tokens)
                if is_subset_group(&clone_groups[j], &clone_groups[i]) {
                    keep[j] = false;
                }
            }
        }

        clone_groups
            .into_iter()
            .enumerate()
            .filter(|(i, _)| keep[*i])
            .map(|(_, g)| g)
            .collect()
    }
}

/// Check if all instances of `smaller` overlap with instances of `larger`.
fn is_subset_group(smaller: &CloneGroup, larger: &CloneGroup) -> bool {
    smaller.instances.iter().all(|s_inst| {
        larger.instances.iter().any(|l_inst| {
            s_inst.file == l_inst.file
                && s_inst.start_line >= l_inst.start_line
                && s_inst.end_line <= l_inst.end_line
        })
    })
}

/// A raw clone pair before grouping.
#[derive(Debug)]
struct RawClone {
    file_a: usize,
    offset_a: usize,
    file_b: usize,
    offset_b: usize,
    length: usize,
}

/// Compute the hash of a frame (sliding window) of tokens.
fn compute_frame_hash(tokens: &[HashedToken], offset: usize, length: usize) -> u64 {
    let mut buf = Vec::with_capacity(length * 8);
    for token in &tokens[offset..offset + length] {
        buf.extend_from_slice(&token.hash.to_le_bytes());
    }
    xxh3_64(&buf)
}

/// Build a `CloneInstance` from file data and token offset/length.
fn build_clone_instance(
    file: &FileData,
    token_offset: usize,
    token_length: usize,
) -> Option<CloneInstance> {
    let tokens = &file.hashed_tokens;
    let source_tokens = &file.file_tokens.tokens;

    if token_offset + token_length > tokens.len() {
        return None;
    }

    // Map from hashed token indices back to source token spans
    let first_hashed = &tokens[token_offset];
    let last_hashed = &tokens[token_offset + token_length - 1];

    let first_source = &source_tokens[first_hashed.original_index];
    let last_source = &source_tokens[last_hashed.original_index];

    let start_byte = first_source.span.start as usize;
    let end_byte = last_source.span.end as usize;

    let source = &file.file_tokens.source;
    let (start_line, start_col) = byte_offset_to_line_col(source, start_byte);
    let (end_line, end_col) = byte_offset_to_line_col(source, end_byte);

    // Extract the fragment
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

/// Convert a byte offset into a 1-based line number and 0-based character column.
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
    let mut files_with_clones: std::collections::HashSet<&PathBuf> =
        std::collections::HashSet::new();
    let mut duplicated_lines: std::collections::HashSet<(PathBuf, usize)> =
        std::collections::HashSet::new();
    let mut duplicated_tokens = 0usize;
    let mut clone_instances = 0usize;

    for group in clone_groups {
        for instance in &group.instances {
            files_with_clones.insert(&instance.file);
            clone_instances += 1;
            for line in instance.start_line..=instance.end_line {
                duplicated_lines.insert((instance.file.clone(), line));
            }
        }
        // Each instance contributes token_count duplicated tokens,
        // but only count duplicates (all instances beyond the first)
        if group.instances.len() > 1 {
            duplicated_tokens += group.token_count * (group.instances.len() - 1);
        }
    }

    let dup_line_count = duplicated_lines.len();
    let duplication_percentage = if total_lines > 0 {
        (dup_line_count as f64 / total_lines as f64) * 100.0
    } else {
        0.0
    };

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

// ── Union-Find ──────────────────────────────────────────────

/// Find the root of a node with path compression.
fn find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]]; // path halving
        x = parent[x];
    }
    x
}

/// Union two nodes.
fn union(parent: &mut [usize], a: usize, b: usize) {
    let ra = find(parent, a);
    let rb = find(parent, b);
    if ra != rb {
        parent[rb] = ra;
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

    #[test]
    fn empty_input_produces_empty_report() {
        let detector = RabinKarpDetector::new(5, 1, false);
        let report = detector.detect(vec![]);
        assert!(report.clone_groups.is_empty());
        assert_eq!(report.stats.total_files, 0);
    }

    #[test]
    fn single_file_no_clones() {
        let detector = RabinKarpDetector::new(3, 1, false);
        let hashed = make_hashed_tokens(&[1, 2, 3, 4, 5]);
        let ft = make_file_tokens("a b c d e", 5);
        let report = detector.detect(vec![(PathBuf::from("a.ts"), hashed, ft)]);
        assert!(report.clone_groups.is_empty());
    }

    #[test]
    fn detects_exact_duplicate_across_files() {
        let detector = RabinKarpDetector::new(3, 1, false);

        // Same token sequence in two files
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
        let detector = RabinKarpDetector::new(10, 1, false);

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
        // Should clamp to end of source
        let (line, col) = byte_offset_to_line_col(source, 100);
        assert_eq!(line, 1);
        assert_eq!(col, 3);
    }

    #[test]
    fn compute_frame_hash_deterministic() {
        let tokens = make_hashed_tokens(&[1, 2, 3, 4, 5]);
        let h1 = compute_frame_hash(&tokens, 0, 3);
        let h2 = compute_frame_hash(&tokens, 0, 3);
        assert_eq!(h1, h2);
    }

    #[test]
    fn compute_frame_hash_different_offsets() {
        let tokens = make_hashed_tokens(&[1, 2, 3, 4, 5]);
        let h1 = compute_frame_hash(&tokens, 0, 3);
        let h2 = compute_frame_hash(&tokens, 1, 3);
        assert_ne!(h1, h2);
    }

    #[test]
    fn union_find_works() {
        let mut parent: Vec<usize> = (0..5).collect();
        union(&mut parent, 0, 1);
        union(&mut parent, 2, 3);
        union(&mut parent, 0, 2);
        assert_eq!(find(&mut parent, 3), find(&mut parent, 1));
    }

    #[test]
    fn skip_local_filters_same_directory() {
        let detector = RabinKarpDetector::new(3, 1, true);

        let hashes = vec![10, 20, 30, 40, 50];
        let source = "a\nb\nc\nd\ne";

        let hashed_a = make_hashed_tokens(&hashes);
        let hashed_b = make_hashed_tokens(&hashes);
        let ft_a = make_file_tokens(source, 5);
        let ft_b = make_file_tokens(source, 5);

        // Same directory -> should be filtered with skip_local
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
        let detector = RabinKarpDetector::new(3, 1, true);

        let hashes = vec![10, 20, 30, 40, 50];
        let source = "a\nb\nc\nd\ne";

        let hashed_a = make_hashed_tokens(&hashes);
        let hashed_b = make_hashed_tokens(&hashes);
        let ft_a = make_file_tokens(source, 5);
        let ft_b = make_file_tokens(source, 5);

        // Different directories -> should be kept
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
}
