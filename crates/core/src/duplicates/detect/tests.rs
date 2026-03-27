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

// ── suffix_array module tests ───────────────────────────

#[test]
fn sa_empty_input() {
    let sa = suffix_array::build_suffix_array(&[]);
    assert!(sa.is_empty());
}

#[test]
fn sa_single_element() {
    let sa = suffix_array::build_suffix_array(&[42]);
    assert_eq!(sa, vec![0]);
}

#[test]
fn sa_two_elements_sorted() {
    // [0, 1] -> suffixes: "0 1" at 0, "1" at 1
    // Lex order: "0 1" < "1" -> SA = [0, 1]
    let sa = suffix_array::build_suffix_array(&[0, 1]);
    assert_eq!(sa, vec![0, 1]);
}

#[test]
fn sa_two_elements_reversed() {
    // [1, 0] -> suffixes: "1 0" at 0, "0" at 1
    // Lex order: "0" < "1 0" -> SA = [1, 0]
    let sa = suffix_array::build_suffix_array(&[1, 0]);
    assert_eq!(sa, vec![1, 0]);
}

#[test]
fn sa_all_identical() {
    // [3, 3, 3, 3] -> all suffixes start with 3, shorter suffixes sort first
    // SA should be [3, 2, 1, 0] (shortest suffix first)
    let sa = suffix_array::build_suffix_array(&[3, 3, 3, 3]);
    assert_eq!(sa, vec![3, 2, 1, 0]);
}

#[test]
fn sa_already_sorted() {
    // [0, 1, 2, 3] -> suffixes in ascending order
    let sa = suffix_array::build_suffix_array(&[0, 1, 2, 3]);
    // "0 1 2 3" < "1 2 3" < "2 3" < "3"
    assert_eq!(sa, vec![0, 1, 2, 3]);
}

#[test]
fn sa_reverse_sorted() {
    // [3, 2, 1, 0]
    let sa = suffix_array::build_suffix_array(&[3, 2, 1, 0]);
    // "0" at 3, "1 0" at 2, "2 1 0" at 1, "3 2 1 0" at 0
    assert_eq!(sa, vec![3, 2, 1, 0]);
}

#[test]
fn sa_with_negative_sentinels() {
    // Sentinels are negative. They should sort before positive values.
    let text: Vec<i64> = vec![5, 10, -1, 5, 10];
    let sa = suffix_array::build_suffix_array(&text);

    // Verify the SA is a valid permutation.
    let mut sorted_sa = sa.clone();
    sorted_sa.sort_unstable();
    assert_eq!(sorted_sa, vec![0, 1, 2, 3, 4]);

    // The sentinel at position 2 should sort earliest (most negative).
    assert_eq!(sa[0], 2, "Sentinel position should be first in SA");
}

#[test]
fn sa_ordering_invariant() {
    // Verify that suffixes are lexicographically ordered for a longer input.
    let text: Vec<i64> = vec![3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5];
    let sa = suffix_array::build_suffix_array(&text);

    // Check that sa[i] < sa[i+1] in lexicographic suffix order.
    for i in 0..sa.len() - 1 {
        let s1 = &text[sa[i]..];
        let s2 = &text[sa[i + 1]..];
        assert!(
            s1 < s2,
            "SA ordering violated at position {i}: suffix at {} ({s1:?}) >= suffix at {} ({s2:?})",
            sa[i],
            sa[i + 1]
        );
    }
}

#[test]
fn sa_is_valid_permutation() {
    let text: Vec<i64> = vec![1, 0, 2, 0, 2, 0];
    let mut sa = suffix_array::build_suffix_array(&text);
    let n = text.len();
    sa.sort_unstable();
    let expected: Vec<usize> = (0..n).collect();
    assert_eq!(sa, expected, "SA must be a permutation of 0..n");
}

#[test]
fn sa_multiple_sentinels() {
    // Three files separated by unique sentinels: [1, 2, -1, 3, 4, -2, 5, 6]
    let text: Vec<i64> = vec![1, 2, -1, 3, 4, -2, 5, 6];
    let sa = suffix_array::build_suffix_array(&text);

    // Sentinel -2 (position 5) should sort before -1 (position 2) which sorts
    // before all positive values.
    assert_eq!(sa[0], 5, "Most negative sentinel should be first");
    assert_eq!(sa[1], 2, "Second sentinel should be second");
}

// ── lcp module tests ────────────────────────────────────

#[test]
fn lcp_empty_input() {
    let lcp_arr = lcp::build_lcp(&[], &[]);
    assert!(lcp_arr.is_empty());
}

#[test]
fn lcp_single_element() {
    let text: Vec<i64> = vec![42];
    let sa = suffix_array::build_suffix_array(&text);
    let lcp_arr = lcp::build_lcp(&text, &sa);
    assert_eq!(lcp_arr, vec![0]);
}

#[test]
fn lcp_no_common_prefixes() {
    // All distinct values -> no shared prefixes between adjacent suffixes.
    let text: Vec<i64> = vec![0, 1, 2, 3];
    let sa = suffix_array::build_suffix_array(&text);
    let lcp_arr = lcp::build_lcp(&text, &sa);

    // LCP[0] is always 0. Adjacent suffixes in SA share a prefix only when
    // values coincide at corresponding positions.
    assert_eq!(lcp_arr[0], 0);

    // For strictly increasing [0,1,2,3], sorted SA = [0,1,2,3].
    // LCP between "0 1 2 3" and "1 2 3" = 0
    // LCP between "1 2 3" and "2 3" = 0
    // LCP between "2 3" and "3" = 0
    for v in &lcp_arr {
        assert_eq!(*v, 0);
    }
}

#[test]
fn lcp_all_identical() {
    // [5, 5, 5, 5] -> SA = [3, 2, 1, 0]
    // Suffixes: [5], [5,5], [5,5,5], [5,5,5,5]
    // LCP[0]=0, LCP[1]=1 (5 vs 5,5), LCP[2]=2, LCP[3]=3
    let text: Vec<i64> = vec![5, 5, 5, 5];
    let sa = suffix_array::build_suffix_array(&text);
    let lcp_arr = lcp::build_lcp(&text, &sa);
    assert_eq!(lcp_arr, vec![0, 1, 2, 3]);
}

#[test]
fn lcp_sentinel_prevents_cross_file_extension() {
    // File 1: [1, 2, 3], sentinel [-1], File 2: [1, 2, 3, 4]
    // The LCP between suffixes starting at pos 0 and pos 4 should be exactly 3
    // (stops before sentinel in file 1), not 4.
    let text: Vec<i64> = vec![1, 2, 3, -1, 1, 2, 3, 4];
    let sa = suffix_array::build_suffix_array(&text);
    let lcp_arr = lcp::build_lcp(&text, &sa);

    let rank_0 = sa.iter().position(|&s| s == 0).unwrap();
    let rank_4 = sa.iter().position(|&s| s == 4).unwrap();
    let (lo, hi) = if rank_0 < rank_4 {
        (rank_0, rank_4)
    } else {
        (rank_4, rank_0)
    };
    let min_lcp = lcp_arr[(lo + 1)..=hi].iter().copied().min().unwrap();
    assert_eq!(min_lcp, 3, "LCP should stop at sentinel, not extend to 4");
}

#[test]
fn lcp_multiple_sentinels_between_files() {
    // Three files with sentinels: [10, 20, -1, 10, 20, -2, 10, 20]
    let text: Vec<i64> = vec![10, 20, -1, 10, 20, -2, 10, 20];
    let sa = suffix_array::build_suffix_array(&text);
    let lcp_arr = lcp::build_lcp(&text, &sa);

    // All three instances of [10, 20] should have LCP=2 pairwise.
    // Find SA positions for text positions 0, 3, and 6 (all start with "10 20").
    let positions = [0usize, 3, 6];
    let mut ranks: Vec<usize> = positions
        .iter()
        .map(|&p| sa.iter().position(|&s| s == p).unwrap())
        .collect();
    ranks.sort_unstable();

    // Between consecutive ranks, min LCP should be 2.
    for w in ranks.windows(2) {
        let min_lcp = lcp_arr[(w[0] + 1)..=w[1]].iter().copied().min().unwrap();
        assert_eq!(
            min_lcp, 2,
            "LCP between identical sequences across sentinels should be 2"
        );
    }
}

#[test]
fn lcp_sentinel_at_start() {
    // Sentinel followed by tokens: [-1, 5, 10]
    let text: Vec<i64> = vec![-1, 5, 10];
    let sa = suffix_array::build_suffix_array(&text);
    let lcp_arr = lcp::build_lcp(&text, &sa);

    // LCP should be 0 for the sentinel entry since sentinel breaks matching.
    assert_eq!(lcp_arr[0], 0);
}

// ── concatenation module tests ──────────────────────────

#[test]
fn concat_empty_files_list() {
    let (text, file_of, file_offsets) = concatenation::concatenate_with_sentinels(&[]);
    assert!(text.is_empty());
    assert!(file_of.is_empty());
    assert!(file_offsets.is_empty());
}

#[test]
fn concat_single_file_no_sentinel() {
    let files = vec![vec![1u32, 2, 3]];
    let (text, file_of, file_offsets) = concatenation::concatenate_with_sentinels(&files);

    // Single file: no sentinel appended.
    assert_eq!(text, vec![1i64, 2, 3]);
    assert_eq!(file_of, vec![0, 0, 0]);
    assert_eq!(file_offsets, vec![0]);
}

#[test]
fn concat_two_files_one_sentinel() {
    let files = vec![vec![1u32, 2], vec![3u32, 4]];
    let (text, file_of, file_offsets) = concatenation::concatenate_with_sentinels(&files);

    assert_eq!(text, vec![1i64, 2, -1, 3, 4]);
    assert_eq!(file_of, vec![0, 0, usize::MAX, 1, 1]);
    assert_eq!(file_offsets, vec![0, 3]);
}

#[test]
fn concat_three_files_unique_sentinels() {
    let files = vec![vec![10u32], vec![20u32], vec![30u32]];
    let (text, file_of, file_offsets) = concatenation::concatenate_with_sentinels(&files);

    // sentinels: -1 between file 0 and 1, -2 between file 1 and 2
    assert_eq!(text, vec![10i64, -1, 20, -2, 30]);
    assert_eq!(file_of, vec![0, usize::MAX, 1, usize::MAX, 2]);
    assert_eq!(file_offsets, vec![0, 2, 4]);
}

#[test]
fn concat_sentinels_are_unique() {
    let files = vec![vec![0u32; 3]; 5]; // 5 files of 3 tokens each
    let (text, _file_of, _file_offsets) = concatenation::concatenate_with_sentinels(&files);

    // Extract sentinel values (negative entries).
    let sentinels: Vec<i64> = text.iter().copied().filter(|&v| v < 0).collect();
    assert_eq!(sentinels.len(), 4, "4 sentinels between 5 files");

    // All sentinels must be unique.
    let unique: rustc_hash::FxHashSet<i64> = sentinels.iter().copied().collect();
    assert_eq!(
        unique.len(),
        sentinels.len(),
        "All sentinels must be unique"
    );
}

#[test]
fn concat_file_of_maps_correctly() {
    let files = vec![vec![1u32, 2, 3], vec![4u32, 5]];
    let (text, file_of, _) = concatenation::concatenate_with_sentinels(&files);

    for (pos, &fid) in file_of.iter().enumerate() {
        if text[pos] < 0 {
            assert_eq!(
                fid,
                usize::MAX,
                "Sentinel positions should map to usize::MAX"
            );
        } else if pos < 3 {
            assert_eq!(fid, 0, "Position {pos} should belong to file 0");
        } else {
            assert_eq!(fid, 1, "Position {pos} should belong to file 1");
        }
    }
}

#[test]
fn concat_file_offsets_are_correct() {
    let files = vec![vec![1u32, 2, 3], vec![4u32, 5, 6, 7], vec![8u32]];
    let (_text, _file_of, file_offsets) = concatenation::concatenate_with_sentinels(&files);

    assert_eq!(file_offsets[0], 0, "First file starts at 0");
    assert_eq!(
        file_offsets[1], 4,
        "Second file starts after 3 tokens + 1 sentinel"
    );
    assert_eq!(file_offsets[2], 9, "Third file starts after 3+1+4+1 = 9");
}

#[test]
fn concat_empty_file_in_middle() {
    let files = vec![vec![1u32, 2], vec![], vec![3u32, 4]];
    let (text, file_of, file_offsets) = concatenation::concatenate_with_sentinels(&files);

    // Empty file contributes 0 tokens but still gets a file offset.
    assert_eq!(file_offsets.len(), 3);
    // text: [1, 2, -1, -2, 3, 4]
    assert_eq!(text.len(), 6);
    // The empty file's offset points to the position right after its sentinel.
    assert_eq!(
        file_offsets[1], 3,
        "Empty file offset is after first sentinel"
    );

    // Verify sentinels.
    assert_eq!(text[2], -1);
    assert_eq!(text[3], -2);
    assert_eq!(file_of[2], usize::MAX);
    assert_eq!(file_of[3], usize::MAX);
}

// ── extraction module tests ─────────────────────────────

fn make_file_data(path: &str, source: &str, num_tokens: usize) -> FileData {
    FileData {
        path: PathBuf::from(path),
        hashed_tokens: make_hashed_tokens(&(0..num_tokens as u64).collect::<Vec<_>>()),
        file_tokens: make_file_tokens(source, num_tokens),
    }
}

#[test]
fn extraction_empty_sa() {
    let groups = extraction::extract_clone_groups(&[], &[], &[], &[], 3, &[]);
    assert!(groups.is_empty());
}

#[test]
fn extraction_single_suffix_no_groups() {
    // Only one suffix -> cannot form a clone group.
    let groups = extraction::extract_clone_groups(&[0], &[0], &[0], &[0], 1, &[]);
    assert!(groups.is_empty());
}

#[test]
fn extraction_below_min_tokens_no_groups() {
    // Two identical files but min_tokens is higher than file length.
    let files = vec![
        make_file_data("a.ts", "ab", 2),
        make_file_data("b.ts", "ab", 2),
    ];
    let ranked = ranking::rank_reduce(&files);
    let (text, file_of, file_offsets) = concatenation::concatenate_with_sentinels(&ranked);
    let sa = suffix_array::build_suffix_array(&text);
    let lcp_arr = lcp::build_lcp(&text, &sa);

    let groups = extraction::extract_clone_groups(
        &sa,
        &lcp_arr,
        &file_of,
        &file_offsets,
        10, // min_tokens > file length
        &files,
    );
    assert!(groups.is_empty());
}

#[test]
fn extraction_skips_sentinel_positions() {
    // Verify that sentinel positions (file_of = usize::MAX) are never included
    // in clone group instances.
    let files = vec![
        make_file_data("a.ts", "aa\nbb\ncc", 3),
        make_file_data("b.ts", "aa\nbb\ncc", 3),
    ];
    let ranked = ranking::rank_reduce(&files);
    let (text, file_of, file_offsets) = concatenation::concatenate_with_sentinels(&ranked);
    let sa = suffix_array::build_suffix_array(&text);
    let lcp_arr = lcp::build_lcp(&text, &sa);

    let groups =
        extraction::extract_clone_groups(&sa, &lcp_arr, &file_of, &file_offsets, 2, &files);

    for group in &groups {
        for &(fid, _offset) in &group.instances {
            assert_ne!(
                fid,
                usize::MAX,
                "Sentinel positions must not appear in instances"
            );
        }
    }
}

#[test]
fn extraction_produces_valid_offsets() {
    // All instance offsets must be within the file's token bounds.
    let files = vec![
        make_file_data("a.ts", "aa\nbb\ncc\ndd\nee", 5),
        make_file_data("b.ts", "aa\nbb\ncc\ndd\nee", 5),
    ];
    let ranked = ranking::rank_reduce(&files);
    let (text, file_of, file_offsets) = concatenation::concatenate_with_sentinels(&ranked);
    let sa = suffix_array::build_suffix_array(&text);
    let lcp_arr = lcp::build_lcp(&text, &sa);

    let groups =
        extraction::extract_clone_groups(&sa, &lcp_arr, &file_of, &file_offsets, 2, &files);

    for group in &groups {
        for &(fid, offset) in &group.instances {
            assert!(
                offset + group.length <= files[fid].hashed_tokens.len(),
                "Instance offset {offset} + length {} exceeds file {fid} token count {}",
                group.length,
                files[fid].hashed_tokens.len()
            );
        }
    }
}

#[test]
fn extraction_removes_overlapping_same_file() {
    // File with overlapping repeat: [1,2,1,2,1] has [1,2] at offsets 0, 2 and
    // also at offset 2 again via a different LCP interval. The extraction should
    // deduplicate overlapping instances in the same file.
    let hashed = make_hashed_tokens(&[1, 2, 1, 2, 1]);
    let file = FileData {
        path: PathBuf::from("a.ts"),
        hashed_tokens: hashed,
        file_tokens: make_file_tokens("aa\nbb\ncc\ndd\nee", 5),
    };
    let files = vec![file];
    let ranked = ranking::rank_reduce(&files);
    let (text, file_of, file_offsets) = concatenation::concatenate_with_sentinels(&ranked);
    let sa = suffix_array::build_suffix_array(&text);
    let lcp_arr = lcp::build_lcp(&text, &sa);

    let groups =
        extraction::extract_clone_groups(&sa, &lcp_arr, &file_of, &file_offsets, 2, &files);

    // Verify no two instances in the same file overlap.
    for group in &groups {
        let mut same_file: Vec<(usize, usize)> = group
            .instances
            .iter()
            .filter(|&&(fid, _)| fid == 0)
            .map(|&(_, offset)| (offset, offset + group.length))
            .collect();
        same_file.sort_unstable();
        for w in same_file.windows(2) {
            assert!(
                w[1].0 >= w[0].1,
                "Overlapping instances: [{}, {}) and [{}, {})",
                w[0].0,
                w[0].1,
                w[1].0,
                w[1].1
            );
        }
    }
}

#[test]
fn extraction_at_least_two_instances() {
    // Every returned group must have at least 2 instances.
    let files = vec![
        make_file_data("a.ts", "aa\nbb\ncc\ndd\nee", 5),
        make_file_data("b.ts", "aa\nbb\ncc\ndd\nee", 5),
    ];
    let ranked = ranking::rank_reduce(&files);
    let (text, file_of, file_offsets) = concatenation::concatenate_with_sentinels(&ranked);
    let sa = suffix_array::build_suffix_array(&text);
    let lcp_arr = lcp::build_lcp(&text, &sa);

    let groups =
        extraction::extract_clone_groups(&sa, &lcp_arr, &file_of, &file_offsets, 2, &files);

    for group in &groups {
        assert!(
            group.instances.len() >= 2,
            "Group with length {} has only {} instance(s)",
            group.length,
            group.instances.len()
        );
    }
}

// ── ranking module tests ────────────────────────────────

#[test]
fn rank_reduce_empty_files() {
    let ranked = ranking::rank_reduce(&[]);
    assert!(ranked.is_empty());
}

#[test]
fn rank_reduce_single_empty_file() {
    let files = vec![FileData {
        path: PathBuf::from("a.ts"),
        hashed_tokens: vec![],
        file_tokens: make_file_tokens("", 0),
    }];
    let ranked = ranking::rank_reduce(&files);
    assert_eq!(ranked.len(), 1);
    assert!(ranked[0].is_empty());
}

#[test]
fn rank_reduce_all_same_hash() {
    let files = vec![FileData {
        path: PathBuf::from("a.ts"),
        hashed_tokens: make_hashed_tokens(&[42, 42, 42]),
        file_tokens: make_file_tokens("a b c", 3),
    }];
    let ranked = ranking::rank_reduce(&files);
    // All same hash -> all same rank.
    assert_eq!(ranked[0][0], ranked[0][1]);
    assert_eq!(ranked[0][1], ranked[0][2]);
}

#[test]
fn rank_reduce_preserves_equality() {
    // Equal hashes in different files must produce equal ranks.
    let files = vec![
        FileData {
            path: PathBuf::from("a.ts"),
            hashed_tokens: make_hashed_tokens(&[10, 20, 30]),
            file_tokens: make_file_tokens("a b c", 3),
        },
        FileData {
            path: PathBuf::from("b.ts"),
            hashed_tokens: make_hashed_tokens(&[30, 20, 10]),
            file_tokens: make_file_tokens("d e f", 3),
        },
    ];
    let ranked = ranking::rank_reduce(&files);

    // Hash 10 in file A position 0 should equal hash 10 in file B position 2.
    assert_eq!(ranked[0][0], ranked[1][2], "Hash 10 must map to same rank");
    // Hash 20 equality.
    assert_eq!(ranked[0][1], ranked[1][1], "Hash 20 must map to same rank");
    // Hash 30 equality.
    assert_eq!(ranked[0][2], ranked[1][0], "Hash 30 must map to same rank");
}

#[test]
fn rank_reduce_distinct_hashes_get_distinct_ranks() {
    let files = vec![FileData {
        path: PathBuf::from("a.ts"),
        hashed_tokens: make_hashed_tokens(&[100, 200, 300, 400]),
        file_tokens: make_file_tokens("a b c d", 4),
    }];
    let ranked = ranking::rank_reduce(&files);

    let mut ranks = ranked[0].clone();
    ranks.sort_unstable();
    ranks.dedup();
    assert_eq!(
        ranks.len(),
        4,
        "4 distinct hashes should produce 4 distinct ranks"
    );
}

// ── statistics module tests ─────────────────────────────

#[test]
fn stats_empty_groups() {
    let stats = statistics::compute_stats(&[], 5, 100, 500);
    assert_eq!(stats.total_files, 5);
    assert_eq!(stats.files_with_clones, 0);
    assert_eq!(stats.clone_groups, 0);
    assert_eq!(stats.clone_instances, 0);
    assert_eq!(stats.duplicated_lines, 0);
    assert_eq!(stats.duplicated_tokens, 0);
    assert!((stats.duplication_percentage - 0.0).abs() < f64::EPSILON);
}

#[test]
fn stats_zero_total_lines() {
    use crate::duplicates::types::{CloneGroup, CloneInstance};

    let groups = vec![CloneGroup {
        instances: vec![
            CloneInstance {
                file: PathBuf::from("a.ts"),
                start_line: 1,
                end_line: 1,
                start_col: 0,
                end_col: 5,
                fragment: String::new(),
            },
            CloneInstance {
                file: PathBuf::from("b.ts"),
                start_line: 1,
                end_line: 1,
                start_col: 0,
                end_col: 5,
                fragment: String::new(),
            },
        ],
        token_count: 10,
        line_count: 1,
    }];

    // total_lines = 0 -> duplication_percentage should be 0.0 (no div by zero).
    let stats = statistics::compute_stats(&groups, 2, 0, 100);
    assert!((stats.duplication_percentage - 0.0).abs() < f64::EPSILON);
}

#[test]
fn stats_duplicated_tokens_capped() {
    use crate::duplicates::types::{CloneGroup, CloneInstance};

    // Create a group where duplicated_tokens would exceed total_tokens.
    let groups = vec![CloneGroup {
        instances: vec![
            CloneInstance {
                file: PathBuf::from("a.ts"),
                start_line: 1,
                end_line: 10,
                start_col: 0,
                end_col: 10,
                fragment: String::new(),
            },
            CloneInstance {
                file: PathBuf::from("b.ts"),
                start_line: 1,
                end_line: 10,
                start_col: 0,
                end_col: 10,
                fragment: String::new(),
            },
            CloneInstance {
                file: PathBuf::from("c.ts"),
                start_line: 1,
                end_line: 10,
                start_col: 0,
                end_col: 10,
                fragment: String::new(),
            },
        ],
        token_count: 100,
        line_count: 10,
    }];

    // 3 instances, token_count=100 -> duplicated = 100 * (3-1) = 200.
    // But total_tokens = 50 -> capped to 50.
    let stats = statistics::compute_stats(&groups, 3, 30, 50);
    assert_eq!(
        stats.duplicated_tokens, 50,
        "duplicated_tokens must be capped to total_tokens"
    );
}

#[test]
fn stats_multiple_groups_same_file() {
    use crate::duplicates::types::{CloneGroup, CloneInstance};

    // Two clone groups in the same files with overlapping lines.
    let groups = vec![
        CloneGroup {
            instances: vec![
                CloneInstance {
                    file: PathBuf::from("a.ts"),
                    start_line: 1,
                    end_line: 5,
                    start_col: 0,
                    end_col: 10,
                    fragment: String::new(),
                },
                CloneInstance {
                    file: PathBuf::from("b.ts"),
                    start_line: 1,
                    end_line: 5,
                    start_col: 0,
                    end_col: 10,
                    fragment: String::new(),
                },
            ],
            token_count: 20,
            line_count: 5,
        },
        CloneGroup {
            instances: vec![
                CloneInstance {
                    file: PathBuf::from("a.ts"),
                    start_line: 3,
                    end_line: 8,
                    start_col: 0,
                    end_col: 10,
                    fragment: String::new(),
                },
                CloneInstance {
                    file: PathBuf::from("b.ts"),
                    start_line: 3,
                    end_line: 8,
                    start_col: 0,
                    end_col: 10,
                    fragment: String::new(),
                },
            ],
            token_count: 30,
            line_count: 6,
        },
    ];

    let stats = statistics::compute_stats(&groups, 2, 100, 500);
    assert_eq!(stats.files_with_clones, 2);
    assert_eq!(stats.clone_groups, 2);
    assert_eq!(stats.clone_instances, 4);
    // Lines 1-5 and 3-8 in each file -> unique lines per file: {1,2,3,4,5,6,7,8} = 8.
    // Two files -> 8 + 8 = 16 duplicated lines.
    assert_eq!(stats.duplicated_lines, 16);
}

#[test]
fn stats_single_instance_no_duplicated_tokens() {
    use crate::duplicates::types::{CloneGroup, CloneInstance};

    // A group with only one instance (shouldn't normally happen after filtering,
    // but test the edge case). Only instances beyond the first count.
    let groups = vec![CloneGroup {
        instances: vec![CloneInstance {
            file: PathBuf::from("a.ts"),
            start_line: 1,
            end_line: 5,
            start_col: 0,
            end_col: 10,
            fragment: String::new(),
        }],
        token_count: 50,
        line_count: 5,
    }];

    let stats = statistics::compute_stats(&groups, 1, 100, 500);
    // Only 1 instance -> 0 duplicated tokens (instances.len() - 1 = 0).
    assert_eq!(stats.duplicated_tokens, 0);
}

// ── utils module tests ──────────────────────────────────

#[test]
fn byte_offset_to_line_col_fast_matches_simple() {
    let source = "abc\ndef\nghi";
    let line_table: Vec<usize> = source
        .bytes()
        .enumerate()
        .filter_map(|(i, b)| if b == b'\n' { Some(i) } else { None })
        .collect();

    // Compare fast version against the simple version.
    for offset in 0..source.len() {
        let fast = utils::byte_offset_to_line_col_fast(source, offset, &line_table);
        let simple = utils::byte_offset_to_line_col(source, offset);
        assert_eq!(fast, simple, "Mismatch at offset {offset}");
    }
}

#[test]
fn byte_offset_to_line_col_fast_beyond_source() {
    let source = "abc\ndef";
    let line_table: Vec<usize> = source
        .bytes()
        .enumerate()
        .filter_map(|(i, b)| if b == b'\n' { Some(i) } else { None })
        .collect();

    let (line, col) = utils::byte_offset_to_line_col_fast(source, 1000, &line_table);
    let (line_s, col_s) = utils::byte_offset_to_line_col(source, 1000);
    assert_eq!((line, col), (line_s, col_s));
}

#[test]
fn byte_offset_to_line_col_fast_empty_source() {
    let source = "";
    let line_table: Vec<usize> = vec![];
    let (line, col) = utils::byte_offset_to_line_col_fast(source, 0, &line_table);
    assert_eq!(line, 1);
    assert_eq!(col, 0);
}

#[test]
fn byte_offset_to_line_col_fast_at_newlines() {
    let source = "a\nb\nc";
    let line_table: Vec<usize> = vec![1, 3]; // newlines at byte 1 and 3

    // At the newline itself.
    let (line, col) = utils::byte_offset_to_line_col_fast(source, 1, &line_table);
    assert_eq!(line, 1, "Newline byte belongs to line 1");
    assert_eq!(col, 1, "Column should be 1 (after 'a')");

    // Right after the newline.
    let (line, col) = utils::byte_offset_to_line_col_fast(source, 2, &line_table);
    assert_eq!(line, 2, "Byte after first newline is line 2");
    assert_eq!(col, 0, "Column should be 0 at start of line");
}

#[test]
fn byte_offset_to_line_col_multibyte_chars() {
    // UTF-8 multibyte: each emoji is 4 bytes.
    let source = "\u{1F600}\n\u{1F601}"; // 4 bytes + \n + 4 bytes
    let (line, col) = utils::byte_offset_to_line_col(source, 0);
    assert_eq!(line, 1);
    assert_eq!(col, 0);

    let (line, col) = utils::byte_offset_to_line_col(source, 4);
    assert_eq!(line, 1);
    assert_eq!(col, 1); // one character before the newline

    let (line, col) = utils::byte_offset_to_line_col(source, 5);
    assert_eq!(line, 2);
    assert_eq!(col, 0);
}

#[test]
fn byte_offset_to_line_col_inside_multibyte() {
    // Offset landing inside a multibyte char should snap backward.
    let source = "\u{1F600}abc"; // 4-byte emoji + 3 ASCII
    // Offset 2 is inside the emoji -> should snap to byte 0 (start of emoji).
    let (line, col) = utils::byte_offset_to_line_col(source, 2);
    assert_eq!(line, 1);
    assert_eq!(col, 0, "Should snap to character boundary");
}

// ── End-to-end pipeline tests ───────────────────────────

#[test]
fn pipeline_rank_concat_sa_lcp_roundtrip() {
    // Two files with partially shared content.
    let files = vec![
        FileData {
            path: PathBuf::from("a.ts"),
            hashed_tokens: make_hashed_tokens(&[10, 20, 30, 40]),
            file_tokens: make_file_tokens("a\nb\nc\nd", 4),
        },
        FileData {
            path: PathBuf::from("b.ts"),
            hashed_tokens: make_hashed_tokens(&[10, 20, 30, 50]),
            file_tokens: make_file_tokens("e\nf\ng\nh", 4),
        },
    ];

    let ranked = ranking::rank_reduce(&files);
    let (text, file_of, file_offsets) = concatenation::concatenate_with_sentinels(&ranked);
    let sa = suffix_array::build_suffix_array(&text);
    let lcp_arr = lcp::build_lcp(&text, &sa);

    // SA must be a valid permutation.
    let mut sorted_sa = sa.clone();
    sorted_sa.sort_unstable();
    let expected: Vec<usize> = (0..text.len()).collect();
    assert_eq!(sorted_sa, expected);

    // LCP[0] must be 0.
    assert_eq!(lcp_arr[0], 0);

    // Extract groups with min_tokens=3. The shared [10,20,30] should appear.
    let groups =
        extraction::extract_clone_groups(&sa, &lcp_arr, &file_of, &file_offsets, 3, &files);
    assert!(
        !groups.is_empty(),
        "Should find clone group for shared [10,20,30]"
    );

    // At least one group should have length 3.
    let has_len_3 = groups.iter().any(|g| g.length == 3);
    assert!(has_len_3, "Should have a group of length 3");
}

#[test]
fn pipeline_no_false_positives_with_different_files() {
    // Two files with completely different token hashes.
    let files = vec![
        FileData {
            path: PathBuf::from("a.ts"),
            hashed_tokens: make_hashed_tokens(&[1, 2, 3, 4, 5]),
            file_tokens: make_file_tokens("a\nb\nc\nd\ne", 5),
        },
        FileData {
            path: PathBuf::from("b.ts"),
            hashed_tokens: make_hashed_tokens(&[6, 7, 8, 9, 10]),
            file_tokens: make_file_tokens("f\ng\nh\ni\nj", 5),
        },
    ];

    let ranked = ranking::rank_reduce(&files);
    let (text, file_of, file_offsets) = concatenation::concatenate_with_sentinels(&ranked);
    let sa = suffix_array::build_suffix_array(&text);
    let lcp_arr = lcp::build_lcp(&text, &sa);

    let groups =
        extraction::extract_clone_groups(&sa, &lcp_arr, &file_of, &file_offsets, 2, &files);
    assert!(
        groups.is_empty(),
        "Completely different files should produce no clone groups"
    );
}

#[test]
fn min_tokens_zero_returns_empty() {
    // min_tokens=0 is an early-exit edge case in CloneDetector::detect.
    let detector = CloneDetector::new(0, 1, false);
    let hashes = vec![10, 20, 30];
    let report = detector.detect(vec![(
        PathBuf::from("a.ts"),
        make_hashed_tokens(&hashes),
        make_file_tokens("abc", 3),
    )]);
    assert!(report.clone_groups.is_empty());
}

#[test]
fn detector_stats_are_consistent() {
    let detector = CloneDetector::new(3, 1, false);
    let hashes = vec![10, 20, 30, 40, 50];
    let source = "a\nb\nc\nd\ne";

    let report = detector.detect(vec![
        (
            PathBuf::from("a.ts"),
            make_hashed_tokens(&hashes),
            make_file_tokens(source, 5),
        ),
        (
            PathBuf::from("b.ts"),
            make_hashed_tokens(&hashes),
            make_file_tokens(source, 5),
        ),
    ]);

    let stats = &report.stats;
    assert_eq!(stats.total_files, 2);
    assert_eq!(stats.total_lines, 10); // 5 lines per file
    assert_eq!(stats.total_tokens, 10); // 5 tokens per file
    assert_eq!(stats.clone_groups, report.clone_groups.len());
    assert!(stats.duplication_percentage >= 0.0);
    assert!(stats.duplication_percentage <= 100.0);
    assert!(stats.duplicated_tokens <= stats.total_tokens);
    assert!(stats.duplicated_lines <= stats.total_lines);
    assert!(stats.files_with_clones <= stats.total_files);
}

#[test]
fn detector_groups_sorted_by_token_count_desc() {
    let detector = CloneDetector::new(3, 1, false);

    // File A and B have two distinct shared blocks of different lengths.
    // Block 1: [10,20,30] (3 tokens) at start.
    // Block 2: [40,50,60,70] (4 tokens) at end, separated by a unique token.
    let hashes_a: Vec<u64> = vec![10, 20, 30, 99, 40, 50, 60, 70];
    let hashes_b: Vec<u64> = vec![10, 20, 30, 88, 40, 50, 60, 70];
    let source = "a\nb\nc\nd\ne\nf\ng\nh";

    let report = detector.detect(vec![
        (
            PathBuf::from("dir_a/a.ts"),
            make_hashed_tokens(&hashes_a),
            make_file_tokens(source, 8),
        ),
        (
            PathBuf::from("dir_b/b.ts"),
            make_hashed_tokens(&hashes_b),
            make_file_tokens(source, 8),
        ),
    ]);

    // Verify groups are sorted by token_count descending.
    for w in report.clone_groups.windows(2) {
        assert!(
            w[0].token_count >= w[1].token_count,
            "Groups should be sorted by token_count desc: {} < {}",
            w[0].token_count,
            w[1].token_count
        );
    }
}

mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Suffix array is always a permutation of 0..n.
        #[test]
        fn suffix_array_is_permutation(values in prop::collection::vec(-100i64..100i64, 1..100)) {
            let sa = suffix_array::build_suffix_array(&values);
            let n = values.len();
            prop_assert_eq!(sa.len(), n, "SA length should equal input length");
            let mut sorted = sa;
            sorted.sort_unstable();
            let expected: Vec<usize> = (0..n).collect();
            prop_assert_eq!(sorted, expected, "SA should be a permutation of 0..n");
        }

        /// Suffix array of empty input is empty.
        #[test]
        fn suffix_array_empty_input(_unused in Just(())) {
            let sa = suffix_array::build_suffix_array(&[]);
            prop_assert!(sa.is_empty());
        }

        /// LCP values are always non-negative (they are usize, so >= 0).
        /// Also: LCP array has the same length as the suffix array.
        #[test]
        fn lcp_values_same_length_as_sa(values in prop::collection::vec(0i64..50i64, 1..80)) {
            let sa = suffix_array::build_suffix_array(&values);
            let lcp_arr = lcp::build_lcp(&values, &sa);
            prop_assert_eq!(lcp_arr.len(), sa.len(), "LCP array should have same length as SA");
            // LCP[0] is always 0
            if !lcp_arr.is_empty() {
                prop_assert_eq!(lcp_arr[0], 0, "LCP[0] should always be 0");
            }
        }

        /// LCP values should never exceed the remaining text length.
        #[test]
        fn lcp_values_bounded_by_text_length(values in prop::collection::vec(0i64..20i64, 1..60)) {
            let n = values.len();
            let sa = suffix_array::build_suffix_array(&values);
            let lcp_arr = lcp::build_lcp(&values, &sa);
            for (i, &lcp_val) in lcp_arr.iter().enumerate() {
                if i > 0 {
                    let remaining_curr = n - sa[i];
                    let remaining_prev = n - sa[i - 1];
                    let max_possible = remaining_curr.min(remaining_prev);
                    prop_assert!(
                        lcp_val <= max_possible,
                        "LCP[{}]={} exceeds max possible {} (suffixes at {} and {})",
                        i, lcp_val, max_possible, sa[i], sa[i - 1]
                    );
                }
            }
        }

        /// Detected clones always have >= min_tokens tokens.
        #[test]
        fn clones_respect_min_tokens(
            min_tokens in 3..15usize,
            hash_values in prop::collection::vec(1u64..20u64, 5..30),
        ) {
            let detector = CloneDetector::new(min_tokens, 1, false);
            let source_a = (0..hash_values.len()).fold(String::new(), |mut acc, i| {
                use std::fmt::Write;
                let _ = writeln!(acc, "t{i}");
                acc
            });
            let source_b = source_a.clone();

            let hashed_a = make_hashed_tokens(&hash_values);
            let hashed_b = make_hashed_tokens(&hash_values);
            let ft_a = make_file_tokens(&source_a, hash_values.len());
            let ft_b = make_file_tokens(&source_b, hash_values.len());

            let report = detector.detect(vec![
                (PathBuf::from("dir_a/a.ts"), hashed_a, ft_a),
                (PathBuf::from("dir_b/b.ts"), hashed_b, ft_b),
            ]);

            for group in &report.clone_groups {
                prop_assert!(
                    group.token_count >= min_tokens,
                    "Clone group has {} tokens, but min is {}",
                    group.token_count, min_tokens
                );
            }
        }

        /// Clone groups should always have at least 2 instances.
        #[test]
        fn clone_groups_have_at_least_two_instances(
            hash_values in prop::collection::vec(1u64..10u64, 5..20),
        ) {
            let detector = CloneDetector::new(3, 1, false);
            let source = (0..hash_values.len()).fold(String::new(), |mut acc, i| {
                use std::fmt::Write;
                let _ = writeln!(acc, "t{i}");
                acc
            });

            let hashed_a = make_hashed_tokens(&hash_values);
            let hashed_b = make_hashed_tokens(&hash_values);
            let ft_a = make_file_tokens(&source, hash_values.len());
            let ft_b = make_file_tokens(&source, hash_values.len());

            let report = detector.detect(vec![
                (PathBuf::from("dir_a/a.ts"), hashed_a, ft_a),
                (PathBuf::from("dir_b/b.ts"), hashed_b, ft_b),
            ]);

            for group in &report.clone_groups {
                prop_assert!(
                    group.instances.len() >= 2,
                    "Clone group should have at least 2 instances, got {}",
                    group.instances.len()
                );
            }
        }
    }
}
