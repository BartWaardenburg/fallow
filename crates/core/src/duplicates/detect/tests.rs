use std::path::PathBuf;

use oxc_span::Span;
use rustc_hash::FxHashMap;

use super::*;
use crate::duplicates::normalize::HashedToken;
use crate::duplicates::tokenize::{FileTokens, SourceToken, TokenKind};

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
    assert_eq!(utils::byte_offset_to_line_col(source, 0), (1, 0));
    assert_eq!(utils::byte_offset_to_line_col(source, 4), (2, 0));
    assert_eq!(utils::byte_offset_to_line_col(source, 5), (2, 1));
    assert_eq!(utils::byte_offset_to_line_col(source, 8), (3, 0));
}

#[test]
fn byte_offset_beyond_source() {
    let source = "abc";
    // Should clamp to end of source.
    let (line, col) = utils::byte_offset_to_line_col(source, 100);
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
    use crate::duplicates::types::{CloneGroup, CloneInstance};

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

    let stats = statistics::compute_stats(&groups, 10, 200, 1000);
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
    let sa = suffix_array::build_suffix_array(&text);

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
    let sa = suffix_array::build_suffix_array(&text);
    let lcp_arr = lcp::build_lcp(&text, &sa);

    // LCP values for "banana":
    // lcp[0] = 0 (by definition)
    // lcp[1] = 1 (LCP of "a" and "ana" = "a" = 1)
    // lcp[2] = 3 (LCP of "ana" and "anana" = "ana" = 3)
    // lcp[3] = 0 (LCP of "anana" and "banana" = "" = 0)
    // lcp[4] = 0 (LCP of "banana" and "na" = "" = 0)
    // lcp[5] = 2 (LCP of "na" and "nana" = "na" = 2)
    assert_eq!(lcp_arr, vec![0, 1, 3, 0, 0, 2]);
}

#[test]
fn lcp_stops_at_sentinels() {
    // Two "files": [0, 1, 2] sentinel [-1] [0, 1, 2]
    let text: Vec<i64> = vec![0, 1, 2, -1, 0, 1, 2];
    let sa = suffix_array::build_suffix_array(&text);
    let lcp_arr = lcp::build_lcp(&text, &sa);

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
    let min_lcp = lcp_arr[(lo + 1)..=hi].iter().copied().min().unwrap_or(0);
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

    let ranked = ranking::rank_reduce(&files);

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
        let mut file_instances: FxHashMap<&PathBuf, Vec<(usize, usize)>> = FxHashMap::default();
        for inst in &group.instances {
            file_instances
                .entry(&inst.file)
                .or_default()
                .push((inst.start_line, inst.end_line));
        }
        for (_file, mut ranges) in file_instances {
            ranges.sort_unstable();
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
