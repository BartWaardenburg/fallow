use std::path::Path;
use std::time::Duration;

use colored::Colorize;
use fallow_core::duplicates::DuplicationReport;

use super::{MAX_FLAT_ITEMS, format_path, relative_path, split_dir_filename, thousands};

/// Docs base URL for duplication explanations.
const DOCS_DUPLICATION: &str = "https://docs.fallow.tools/explanations/duplication";

/// Maximum clone groups shown in duplication output.
const MAX_CLONE_GROUPS: usize = 10;

pub(in crate::report) fn print_duplication_human(
    report: &DuplicationReport,
    root: &Path,
    elapsed: Duration,
    quiet: bool,
) {
    if !quiet {
        eprintln!();
    }

    if report.clone_groups.is_empty() {
        if !quiet {
            eprintln!(
                "{}",
                format!(
                    "\u{2713} No code duplication found ({:.2}s)",
                    elapsed.as_secs_f64()
                )
                .green()
                .bold()
            );
        }
        return;
    }

    for line in build_duplication_human_lines(report, root) {
        println!("{line}");
    }

    let stats = &report.stats;
    if !quiet {
        eprintln!(
            "{}",
            format!(
                "\u{2717} {} lines ({:.1}%) duplicated across {} file{} ({:.2}s)",
                thousands(stats.duplicated_lines),
                stats.duplication_percentage,
                stats.files_with_clones,
                if stats.files_with_clones == 1 {
                    ""
                } else {
                    "s"
                },
                elapsed.as_secs_f64(),
            )
            .red()
            .bold()
        );
    }
}

/// Build human-readable output lines for duplication report.
pub(in crate::report) fn build_duplication_human_lines(
    report: &DuplicationReport,
    root: &Path,
) -> Vec<String> {
    let mut lines = Vec::new();

    if report.clone_groups.is_empty() && report.clone_families.is_empty() {
        return lines;
    }

    // Sort clone groups by line count descending for most impactful first
    let mut sorted_groups: Vec<&fallow_core::duplicates::CloneGroup> =
        report.clone_groups.iter().collect();
    sorted_groups.sort_by(|a, b| b.line_count.cmp(&a.line_count));

    let total_groups = sorted_groups.len();
    let shown = total_groups.min(MAX_CLONE_GROUPS);

    lines.push(format!(
        "{} {}",
        "\u{25cf}".cyan(),
        format!("Duplicates ({total_groups} clone groups)")
            .cyan()
            .bold()
    ));
    lines.push(String::new());

    for group in &sorted_groups[..shown] {
        let instance_count = group.instances.len();

        // Line count: right-aligned, color-coded
        let lc = group.line_count;
        let lc_str = format!("{:>5}", thousands(lc));
        let lc_colored = if lc > 1000 {
            lc_str.red().bold().to_string()
        } else if lc > 100 {
            lc_str.yellow().to_string()
        } else {
            lc_str.dimmed().to_string()
        };

        lines.push(format!(
            "  {} lines  {} instance{}",
            lc_colored,
            instance_count,
            if instance_count == 1 { "" } else { "s" }
        ));

        for instance in &group.instances {
            let relative = relative_path(&instance.file, root);
            let path_str = relative.display().to_string();
            let (dir, filename) = split_dir_filename(&path_str);
            lines.push(format!(
                "    {}{}:{}-{}",
                dir.dimmed(),
                filename,
                instance.start_line,
                instance.end_line
            ));
        }
        lines.push(String::new());
    }

    if total_groups > MAX_CLONE_GROUPS {
        lines.push(format!(
            "  {}",
            format!(
                "... and {} more clone groups",
                total_groups - MAX_CLONE_GROUPS
            )
            .dimmed()
        ));
    }
    lines.push(format!(
        "  {}",
        format!("Identical code blocks detected via suffix-array analysis \u{2014} {DOCS_DUPLICATION}#clone-groups").dimmed()
    ));
    lines.push(String::new());

    // Detect mirrored directory patterns across families.
    // Families with exactly 2 files that share a common filename after stripping
    // directory prefixes are grouped under a "Mirrored directories" header.
    let (mirrored, non_mirrored) = detect_mirrored_families(&report.clone_families, root);

    if !mirrored.is_empty() {
        let shown_mirrors = mirrored.len().min(MAX_FLAT_ITEMS);
        for mirror in &mirrored[..shown_mirrors] {
            lines.push(format!(
                "{} {}",
                "\u{25cf}".yellow(),
                format!(
                    "Mirrored: {} \u{2194} {} ({} files, {} lines)",
                    mirror.dir_a,
                    mirror.dir_b,
                    mirror.file_count,
                    thousands(mirror.total_lines),
                )
                .yellow()
                .bold()
            ));

            let shown = mirror.files.len().min(MAX_FLAT_ITEMS);
            for filename in &mirror.files[..shown] {
                lines.push(format!("  {}", filename.dimmed()));
            }
            if mirror.files.len() > MAX_FLAT_ITEMS {
                lines.push(format!(
                    "  {}",
                    format!("... and {} more", mirror.files.len() - MAX_FLAT_ITEMS).dimmed()
                ));
            }
            lines.push(String::new());
        }
        if mirrored.len() > MAX_FLAT_ITEMS {
            lines.push(format!(
                "  {}",
                format!(
                    "... and {} more mirrored pairs",
                    mirrored.len() - MAX_FLAT_ITEMS
                )
                .dimmed()
            ));
            lines.push(String::new());
        }
        lines.push(format!(
            "  {}",
            format!("Directories containing identical file copies \u{2014} {DOCS_DUPLICATION}#clone-families").dimmed()
        ));
        lines.push(String::new());
    }

    // Print remaining clone families with refactoring suggestions
    // Suppress single-group families -- not actionable
    let multi_group_families: Vec<_> = non_mirrored.iter().filter(|f| f.groups.len() > 1).collect();

    if !multi_group_families.is_empty() {
        lines.push(format!(
            "{} {}",
            "\u{25cf}".yellow(),
            format!(
                "Clone families ({} with multiple groups)",
                multi_group_families.len()
            )
            .yellow()
            .bold()
        ));
        lines.push(String::new());

        let shown_families = multi_group_families.len().min(MAX_FLAT_ITEMS);
        for family in &multi_group_families[..shown_families] {
            let file_names: Vec<_> = family
                .files
                .iter()
                .map(|f| {
                    let path_str = relative_path(f, root).display().to_string();
                    format_path(&path_str)
                })
                .collect();

            lines.push(format!(
                "  {} groups, {} lines across {}",
                family.groups.len().to_string().bold(),
                thousands(family.total_duplicated_lines).bold(),
                file_names.join(", "),
            ));

            for suggestion in &family.suggestions {
                // Drop "lines saved" -- misleading
                lines.push(format!(
                    "    {} {}",
                    "\u{2192}".yellow(),
                    suggestion.description.dimmed(),
                ));
            }
            lines.push(String::new());
        }
        if multi_group_families.len() > MAX_FLAT_ITEMS {
            lines.push(format!(
                "  {}",
                format!(
                    "... and {} more families",
                    multi_group_families.len() - MAX_FLAT_ITEMS
                )
                .dimmed()
            ));
            lines.push(String::new());
        }
        lines.push(format!(
            "  {}",
            format!("Groups of related clones across the same files \u{2014} {DOCS_DUPLICATION}#clone-families").dimmed()
        ));
        lines.push(String::new());
    }

    lines
}

/// A detected mirrored directory pattern: two directory prefixes that contain
/// identical files (e.g., `src/` and `deno/lib/`).
pub(super) struct MirroredDirs {
    pub(super) dir_a: String,
    pub(super) dir_b: String,
    pub(super) files: Vec<String>,
    pub(super) file_count: usize,
    pub(super) total_lines: usize,
}

/// Detect mirrored directory patterns in clone families.
///
/// Scans families with exactly 2 files. If multiple families share the same
/// directory prefix pair (after stripping to the common filename), they're
/// grouped into a `MirroredDirs`. Families that don't match any mirror pattern
/// are returned as non-mirrored.
///
/// Minimum 3 families must share a pattern to qualify as "mirrored".
pub(super) fn detect_mirrored_families<'a>(
    families: &'a [fallow_core::duplicates::CloneFamily],
    root: &Path,
) -> (
    Vec<MirroredDirs>,
    Vec<&'a fallow_core::duplicates::CloneFamily>,
) {
    const MIN_MIRROR_FAMILIES: usize = 3;

    // For each 2-file family, extract the directory pair + relative filename
    // Entry: (family_index, filename, duplicated_lines)
    type MirrorEntry = (usize, String, usize);
    let mut pair_map: rustc_hash::FxHashMap<(String, String), Vec<MirrorEntry>> =
        rustc_hash::FxHashMap::default();

    for (idx, family) in families.iter().enumerate() {
        if family.files.len() != 2 {
            continue;
        }
        let path_a = relative_path(&family.files[0], root).display().to_string();
        let path_b = relative_path(&family.files[1], root).display().to_string();

        let (dir_a, file_a) = split_dir_filename(&path_a);
        let (dir_b, file_b) = split_dir_filename(&path_b);

        // Only match if the filenames are the same
        if file_a != file_b {
            continue;
        }

        // Normalize: always use the lexically smaller dir first
        let (da, db) = if dir_a <= dir_b {
            (dir_a.to_string(), dir_b.to_string())
        } else {
            (dir_b.to_string(), dir_a.to_string())
        };

        pair_map.entry((da, db)).or_default().push((
            idx,
            file_a.to_string(),
            family.total_duplicated_lines,
        ));
    }

    let mut mirrored_indices: rustc_hash::FxHashSet<usize> = rustc_hash::FxHashSet::default();
    let mut mirrors: Vec<MirroredDirs> = Vec::new();

    for ((dir_a, dir_b), entries) in &pair_map {
        if entries.len() < MIN_MIRROR_FAMILIES {
            continue;
        }
        for &(idx, _, _) in entries {
            mirrored_indices.insert(idx);
        }
        let total_lines: usize = entries.iter().map(|&(_, _, lines)| lines).sum();
        let mut files: Vec<String> = entries.iter().map(|(_, f, _)| f.clone()).collect();
        files.sort();
        let file_count = files.len();
        mirrors.push(MirroredDirs {
            dir_a: dir_a.clone(),
            dir_b: dir_b.clone(),
            files,
            file_count,
            total_lines,
        });
    }

    // Sort mirrors by total lines descending
    mirrors.sort_by(|a, b| b.total_lines.cmp(&a.total_lines));

    let non_mirrored: Vec<&fallow_core::duplicates::CloneFamily> = families
        .iter()
        .enumerate()
        .filter(|(idx, _)| !mirrored_indices.contains(idx))
        .map(|(_, f)| f)
        .collect();

    (mirrors, non_mirrored)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fallow_core::duplicates::{
        CloneFamily, CloneGroup, CloneInstance, DuplicationStats, RefactoringKind,
        RefactoringSuggestion,
    };
    use std::path::PathBuf;

    /// Strip ANSI escape sequences from a string, leaving only the printable text.
    fn strip_ansi(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                for inner in chars.by_ref() {
                    if inner == 'm' {
                        break;
                    }
                }
            } else {
                result.push(c);
            }
        }
        result
    }

    fn plain(lines: &[String]) -> String {
        lines
            .iter()
            .map(|l| strip_ansi(l))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn duplication_empty_report_produces_no_output() {
        let root = PathBuf::from("/project");
        let report = DuplicationReport::default();
        let lines = build_duplication_human_lines(&report, &root);
        assert!(lines.is_empty(), "Empty report should produce no lines");
    }

    #[test]
    fn duplication_groups_show_instances_with_line_count() {
        let root = PathBuf::from("/project");
        let report = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![
                    CloneInstance {
                        file: root.join("src/a.ts"),
                        start_line: 1,
                        end_line: 10,
                        start_col: 0,
                        end_col: 0,
                        fragment: String::new(),
                    },
                    CloneInstance {
                        file: root.join("src/b.ts"),
                        start_line: 5,
                        end_line: 14,
                        start_col: 0,
                        end_col: 0,
                        fragment: String::new(),
                    },
                ],
                token_count: 50,
                line_count: 10,
            }],
            clone_families: vec![],
            stats: DuplicationStats {
                clone_groups: 1,
                clone_instances: 2,
                ..Default::default()
            },
        };
        let lines = build_duplication_human_lines(&report, &root);
        let text = plain(&lines);
        // Line-count-led format, no "Clone group N" label
        assert!(text.contains("10"));
        assert!(text.contains("lines"));
        assert!(text.contains("2 instances"));
        assert!(text.contains("a.ts:1-10"));
        assert!(text.contains("b.ts:5-14"));
        // No tree connectors
        assert!(!text.contains("\u{251c}\u{2500}"));
        assert!(!text.contains("\u{2514}\u{2500}"));
    }

    #[test]
    fn duplication_single_instance_no_plural() {
        let root = PathBuf::from("/project");
        let report = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![CloneInstance {
                    file: root.join("src/a.ts"),
                    start_line: 1,
                    end_line: 10,
                    start_col: 0,
                    end_col: 0,
                    fragment: String::new(),
                }],
                token_count: 50,
                line_count: 10,
            }],
            clone_families: vec![],
            stats: DuplicationStats::default(),
        };
        let lines = build_duplication_human_lines(&report, &root);
        let text = plain(&lines);
        assert!(text.contains("1 instance"));
        assert!(!text.contains("1 instances"));
    }

    #[test]
    fn duplication_families_show_suggestions() {
        let root = PathBuf::from("/project");
        let dummy_group = CloneGroup {
            instances: vec![],
            token_count: 30,
            line_count: 5,
        };
        let report = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![CloneInstance {
                    file: root.join("src/a.ts"),
                    start_line: 1,
                    end_line: 5,
                    start_col: 0,
                    end_col: 0,
                    fragment: String::new(),
                }],
                token_count: 30,
                line_count: 5,
            }],
            clone_families: vec![CloneFamily {
                files: vec![root.join("src/a.ts"), root.join("src/b.ts")],
                groups: vec![dummy_group.clone(), dummy_group],
                total_duplicated_lines: 20,
                total_duplicated_tokens: 100,
                suggestions: vec![RefactoringSuggestion {
                    kind: RefactoringKind::ExtractFunction,
                    description: "Extract shared utility function".to_string(),
                    estimated_savings: 15,
                }],
            }],
            stats: DuplicationStats::default(),
        };
        let lines = build_duplication_human_lines(&report, &root);
        let text = plain(&lines);
        assert!(text.contains("Clone families"));
        assert!(text.contains("Extract shared utility function"));
        // "lines saved" is no longer shown
        assert!(!text.contains("lines saved"));
    }

    #[test]
    fn duplication_suggestion_with_zero_savings_omits_savings_text() {
        let root = PathBuf::from("/project");
        let dummy_group = CloneGroup {
            instances: vec![],
            token_count: 30,
            line_count: 5,
        };
        let report = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![CloneInstance {
                    file: root.join("src/a.ts"),
                    start_line: 1,
                    end_line: 5,
                    start_col: 0,
                    end_col: 0,
                    fragment: String::new(),
                }],
                token_count: 30,
                line_count: 5,
            }],
            clone_families: vec![CloneFamily {
                files: vec![root.join("src/a.ts")],
                groups: vec![dummy_group.clone(), dummy_group],
                total_duplicated_lines: 10,
                total_duplicated_tokens: 50,
                suggestions: vec![RefactoringSuggestion {
                    kind: RefactoringKind::ExtractModule,
                    description: "Extract to shared module".to_string(),
                    estimated_savings: 0,
                }],
            }],
            stats: DuplicationStats::default(),
        };
        let lines = build_duplication_human_lines(&report, &root);
        let text = plain(&lines);
        assert!(text.contains("Extract to shared module"));
        assert!(!text.contains("lines saved"));
    }

    #[test]
    fn duplication_single_group_family_is_suppressed() {
        let root = PathBuf::from("/project");
        let report = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![CloneInstance {
                    file: root.join("src/a.ts"),
                    start_line: 1,
                    end_line: 5,
                    start_col: 0,
                    end_col: 0,
                    fragment: String::new(),
                }],
                token_count: 30,
                line_count: 5,
            }],
            clone_families: vec![CloneFamily {
                files: vec![root.join("src/a.ts")],
                groups: vec![CloneGroup {
                    instances: vec![],
                    token_count: 30,
                    line_count: 5,
                }],
                total_duplicated_lines: 5,
                total_duplicated_tokens: 30,
                suggestions: vec![],
            }],
            stats: DuplicationStats::default(),
        };
        let lines = build_duplication_human_lines(&report, &root);
        let text = plain(&lines);
        // Single-group families are suppressed from output
        assert!(!text.contains("Clone families"));
    }

    #[test]
    fn duplication_multiple_groups_plural() {
        let root = PathBuf::from("/project");
        let dummy_group = CloneGroup {
            instances: vec![],
            token_count: 30,
            line_count: 5,
        };
        let report = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![CloneInstance {
                    file: root.join("src/a.ts"),
                    start_line: 1,
                    end_line: 5,
                    start_col: 0,
                    end_col: 0,
                    fragment: String::new(),
                }],
                token_count: 30,
                line_count: 5,
            }],
            clone_families: vec![CloneFamily {
                files: vec![root.join("src/a.ts")],
                groups: vec![dummy_group.clone(), dummy_group],
                total_duplicated_lines: 10,
                total_duplicated_tokens: 60,
                suggestions: vec![],
            }],
            stats: DuplicationStats::default(),
        };
        let lines = build_duplication_human_lines(&report, &root);
        let text = plain(&lines);
        assert!(text.contains("2 groups,"));
    }

    #[test]
    fn single_instance_clone_group_no_connectors() {
        let root = PathBuf::from("/project");
        let report = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![CloneInstance {
                    file: root.join("src/a.ts"),
                    start_line: 1,
                    end_line: 10,
                    start_col: 0,
                    end_col: 0,
                    fragment: String::new(),
                }],
                token_count: 50,
                line_count: 10,
            }],
            clone_families: vec![],
            stats: DuplicationStats::default(),
        };
        let lines = build_duplication_human_lines(&report, &root);
        let text = plain(&lines);
        // No tree connectors -- simple indentation
        assert!(!text.contains("\u{2514}\u{2500}"));
        assert!(!text.contains("\u{251c}\u{2500}"));
        assert!(text.contains("a.ts:1-10"));
    }

    #[test]
    fn mirrored_dirs_detected() {
        let root = PathBuf::from("/project");
        let mut families = Vec::new();
        // 4 families with same dir pattern (above MIN_MIRROR_FAMILIES threshold of 3)
        for name in &["a.ts", "b.ts", "c.ts", "d.ts"] {
            families.push(CloneFamily {
                files: vec![
                    root.join(format!("src/{name}")),
                    root.join(format!("deno/lib/{name}")),
                ],
                groups: vec![CloneGroup {
                    instances: vec![],
                    token_count: 100,
                    line_count: 50,
                }],
                total_duplicated_lines: 50,
                total_duplicated_tokens: 100,
                suggestions: vec![],
            });
        }
        let (mirrored, non_mirrored) = detect_mirrored_families(&families, &root);
        assert_eq!(mirrored.len(), 1);
        assert_eq!(mirrored[0].file_count, 4);
        assert!(non_mirrored.is_empty());
    }

    #[test]
    fn mirrored_dirs_below_threshold_not_detected() {
        let root = PathBuf::from("/project");
        let families = vec![
            CloneFamily {
                files: vec![root.join("src/a.ts"), root.join("deno/a.ts")],
                groups: vec![],
                total_duplicated_lines: 10,
                total_duplicated_tokens: 50,
                suggestions: vec![],
            },
            CloneFamily {
                files: vec![root.join("src/b.ts"), root.join("deno/b.ts")],
                groups: vec![],
                total_duplicated_lines: 10,
                total_duplicated_tokens: 50,
                suggestions: vec![],
            },
        ];
        let (mirrored, _) = detect_mirrored_families(&families, &root);
        // Only 2 families -- below threshold of 3
        assert!(mirrored.is_empty());
    }
}
