use std::fmt::Write;
use std::path::Path;

use tower_lsp::lsp_types::*;

use fallow_core::duplicates::DuplicationReport;
use fallow_core::results::AnalysisResults;

/// Build hover information for a position in a file.
///
/// Returns a hover with markdown content describing:
/// - Unused export/type status with explanation
/// - Used export reference counts with file locations
/// - Unused file status
/// - Unused member status
/// - Unresolved import details
/// - Code duplication instance details with other locations
pub fn build_hover(
    results: &AnalysisResults,
    duplication: &DuplicationReport,
    file_path: &Path,
    position: Position,
) -> Option<Hover> {
    // Check unused files (file-level hover at any position)
    if let Some(hover) = check_unused_file(results, file_path) {
        return Some(hover);
    }

    // Check unused exports at this line
    if let Some(hover) = check_unused_export(results, file_path, position) {
        return Some(hover);
    }

    // Check used exports at this line (show reference info)
    if let Some(hover) = check_used_export(results, file_path, position) {
        return Some(hover);
    }

    // Check unused members at this line
    if let Some(hover) = check_unused_member(results, file_path, position) {
        return Some(hover);
    }

    // Check unresolved imports at this line
    if let Some(hover) = check_unresolved_import(results, file_path, position) {
        return Some(hover);
    }

    // Check code duplication at this position
    if let Some(hover) = check_duplication(duplication, file_path, position) {
        return Some(hover);
    }

    None
}

/// Check if the file is in the unused files list.
fn check_unused_file(results: &AnalysisResults, file_path: &Path) -> Option<Hover> {
    let is_unused = results.unused_files.iter().any(|f| f.path == file_path);
    if !is_unused {
        return None;
    }

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: "**fallow**: This file is not imported by any other file and is not reachable \
                    from any entry point."
                .to_string(),
        }),
        range: None,
    })
}

/// Check if the position is on an unused export or type.
fn check_unused_export(
    results: &AnalysisResults,
    file_path: &Path,
    position: Position,
) -> Option<Hover> {
    for (exports, kind_label) in [
        (&results.unused_exports, "Export"),
        (&results.unused_types, "Type export"),
    ] {
        for export in exports {
            if export.path != file_path {
                continue;
            }
            let export_line = export.line.saturating_sub(1);
            if export_line != position.line {
                continue;
            }

            let value = format!(
                "**fallow**: {kind_label} `{}` is not imported by any other file.",
                export.export_name
            );

            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: Some(Range {
                    start: Position {
                        line: export_line,
                        character: export.col,
                    },
                    end: Position {
                        line: export_line,
                        character: export.col + export.export_name.len() as u32,
                    },
                }),
            });
        }
    }

    None
}

/// Check if the position is on a used export and show reference information.
fn check_used_export(
    results: &AnalysisResults,
    file_path: &Path,
    position: Position,
) -> Option<Hover> {
    for usage in &results.export_usages {
        if usage.path != file_path {
            continue;
        }
        let usage_line = usage.line.saturating_sub(1);
        if usage_line != position.line {
            continue;
        }

        // Skip exports with 0 references (they will be caught by unused export check)
        if usage.reference_count == 0 {
            continue;
        }

        let ref_word = if usage.reference_count == 1 {
            "file"
        } else {
            "files"
        };

        let mut value = format!(
            "**fallow**: Export `{}` is used by {} {ref_word}",
            usage.export_name, usage.reference_count,
        );

        // List up to 10 reference locations
        if !usage.reference_locations.is_empty() {
            value.push_str(":\n");
            for (i, loc) in usage.reference_locations.iter().take(10).enumerate() {
                let display_path = loc.path.file_name().map_or_else(
                    || loc.path.display().to_string(),
                    |name| name.to_string_lossy().into_owned(),
                );
                let _ = write!(value, "- `{display_path}` line {}", loc.line);
                if i < usage.reference_locations.len().min(10) - 1 {
                    value.push('\n');
                }
            }
            if usage.reference_locations.len() > 10 {
                let _ = write!(
                    value,
                    "\n- ... and {} more",
                    usage.reference_locations.len() - 10
                );
            }
        } else {
            value.push('.');
        }

        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: Some(Range {
                start: Position {
                    line: usage_line,
                    character: usage.col,
                },
                end: Position {
                    line: usage_line,
                    character: usage.col + usage.export_name.len() as u32,
                },
            }),
        });
    }

    None
}

/// Check if the position is on an unused enum or class member.
fn check_unused_member(
    results: &AnalysisResults,
    file_path: &Path,
    position: Position,
) -> Option<Hover> {
    for (members, kind_label) in [
        (&results.unused_enum_members, "Enum member"),
        (&results.unused_class_members, "Class member"),
    ] {
        for member in members {
            if member.path != file_path {
                continue;
            }
            let member_line = member.line.saturating_sub(1);
            if member_line != position.line {
                continue;
            }

            let value = format!(
                "**fallow**: {kind_label} `{}.{}` is never used outside its declaration.",
                member.parent_name, member.member_name
            );

            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: Some(Range {
                    start: Position {
                        line: member_line,
                        character: member.col,
                    },
                    end: Position {
                        line: member_line,
                        character: member.col + member.member_name.len() as u32,
                    },
                }),
            });
        }
    }

    None
}

/// Check if the position is on an unresolved import.
fn check_unresolved_import(
    results: &AnalysisResults,
    file_path: &Path,
    position: Position,
) -> Option<Hover> {
    for import in &results.unresolved_imports {
        if import.path != file_path {
            continue;
        }
        let import_line = import.line.saturating_sub(1);
        if import_line != position.line {
            continue;
        }

        let value = format!(
            "**fallow**: Cannot resolve import `{}`. The module may be missing, misspelled, \
             or not installed.",
            import.specifier
        );

        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: Some(Range {
                start: Position {
                    line: import_line,
                    character: import.col,
                },
                end: Position {
                    line: import_line,
                    character: import.col,
                },
            }),
        });
    }

    None
}

/// Check if the position overlaps with a code duplication instance.
fn check_duplication(
    duplication: &DuplicationReport,
    file_path: &Path,
    position: Position,
) -> Option<Hover> {
    for group in &duplication.clone_groups {
        for instance in &group.instances {
            if instance.file != file_path {
                continue;
            }

            let start_line = (instance.start_line as u32).saturating_sub(1);
            let end_line = (instance.end_line as u32).saturating_sub(1);

            // Check if the cursor is within this duplication range
            if position.line < start_line || position.line > end_line {
                continue;
            }

            let other_count = group.instances.len() - 1;
            let instance_word = if other_count == 1 {
                "instance"
            } else {
                "instances"
            };

            let mut value = format!(
                "**fallow**: Duplicated code block ({} lines, {} tokens). \
                 {other_count} other {instance_word}",
                group.line_count, group.token_count,
            );

            // List other instances
            let others: Vec<_> = group
                .instances
                .iter()
                .filter(|other| {
                    !(other.file == instance.file && other.start_line == instance.start_line)
                })
                .collect();

            if !others.is_empty() {
                value.push_str(":\n");
                for (i, other) in others.iter().take(10).enumerate() {
                    let display_path = other.file.file_name().map_or_else(
                        || other.file.display().to_string(),
                        |name| name.to_string_lossy().into_owned(),
                    );
                    let _ = write!(
                        value,
                        "- `{display_path}` lines {}-{}",
                        other.start_line, other.end_line
                    );
                    if i < others.len().min(10) - 1 {
                        value.push('\n');
                    }
                }
                if others.len() > 10 {
                    let _ = write!(value, "\n- ... and {} more", others.len() - 10);
                }
            } else {
                value.push('.');
            }

            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: Some(Range {
                    start: Position {
                        line: start_line,
                        character: instance.start_col as u32,
                    },
                    end: Position {
                        line: end_line,
                        character: instance.end_col as u32,
                    },
                }),
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use fallow_core::duplicates::{CloneGroup, CloneInstance, DuplicationStats};
    use fallow_core::extract::MemberKind;
    use fallow_core::results::{
        ExportUsage, ReferenceLocation, UnresolvedImport, UnusedExport, UnusedFile, UnusedMember,
    };

    fn test_root() -> PathBuf {
        if cfg!(windows) {
            PathBuf::from("C:\\project")
        } else {
            PathBuf::from("/project")
        }
    }

    fn empty_duplication() -> DuplicationReport {
        DuplicationReport {
            clone_groups: vec![],
            clone_families: vec![],
            stats: DuplicationStats {
                total_files: 0,
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

    #[test]
    fn no_hover_for_clean_file() {
        let root = test_root();
        let path = root.join("src/clean.ts");
        let results = AnalysisResults::default();
        let duplication = empty_duplication();
        let pos = Position {
            line: 5,
            character: 0,
        };

        let hover = build_hover(&results, &duplication, &path, pos);
        assert!(hover.is_none());
    }

    #[test]
    fn hover_on_unused_file() {
        let root = test_root();
        let path = root.join("src/dead.ts");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile { path: path.clone() });
        let duplication = empty_duplication();
        let pos = Position {
            line: 10,
            character: 0,
        };

        let hover = build_hover(&results, &duplication, &path, pos).unwrap();
        match &hover.contents {
            HoverContents::Markup(m) => {
                assert!(m.value.contains("not imported"));
                assert!(m.value.contains("entry point"));
            }
            _ => panic!("Expected markup contents"),
        }
    }

    #[test]
    fn hover_on_unused_export() {
        let root = test_root();
        let path = root.join("src/utils.ts");
        let mut results = AnalysisResults::default();
        results.unused_exports.push(UnusedExport {
            path: path.clone(),
            export_name: "helper".to_string(),
            is_type_only: false,
            line: 5,
            col: 7,
            span_start: 40,
            is_re_export: false,
        });
        let duplication = empty_duplication();
        let pos = Position {
            line: 4, // 0-based
            character: 10,
        };

        let hover = build_hover(&results, &duplication, &path, pos).unwrap();
        match &hover.contents {
            HoverContents::Markup(m) => {
                assert!(m.value.contains("helper"));
                assert!(m.value.contains("not imported"));
            }
            _ => panic!("Expected markup contents"),
        }
        // Should have a range covering the export name
        let range = hover.range.unwrap();
        assert_eq!(range.start.line, 4);
        assert_eq!(range.start.character, 7);
        assert_eq!(range.end.character, 7 + "helper".len() as u32);
    }

    #[test]
    fn hover_on_unused_type() {
        let root = test_root();
        let path = root.join("src/types.ts");
        let mut results = AnalysisResults::default();
        results.unused_types.push(UnusedExport {
            path: path.clone(),
            export_name: "MyType".to_string(),
            is_type_only: true,
            line: 3,
            col: 0,
            span_start: 20,
            is_re_export: false,
        });
        let duplication = empty_duplication();
        let pos = Position {
            line: 2, // 0-based
            character: 3,
        };

        let hover = build_hover(&results, &duplication, &path, pos).unwrap();
        match &hover.contents {
            HoverContents::Markup(m) => {
                assert!(m.value.contains("Type export"));
                assert!(m.value.contains("MyType"));
            }
            _ => panic!("Expected markup contents"),
        }
    }

    #[test]
    fn hover_on_used_export_with_references() {
        let root = test_root();
        let path = root.join("src/utils.ts");
        let mut results = AnalysisResults::default();
        results.export_usages.push(ExportUsage {
            path: path.clone(),
            export_name: "format".to_string(),
            line: 10,
            col: 7,
            reference_count: 2,
            reference_locations: vec![
                ReferenceLocation {
                    path: root.join("src/app.ts"),
                    line: 3,
                    col: 10,
                },
                ReferenceLocation {
                    path: root.join("src/main.ts"),
                    line: 8,
                    col: 0,
                },
            ],
        });
        let duplication = empty_duplication();
        let pos = Position {
            line: 9, // 0-based
            character: 10,
        };

        let hover = build_hover(&results, &duplication, &path, pos).unwrap();
        match &hover.contents {
            HoverContents::Markup(m) => {
                assert!(m.value.contains("format"));
                assert!(m.value.contains("2 files"));
                assert!(m.value.contains("app.ts"));
                assert!(m.value.contains("main.ts"));
            }
            _ => panic!("Expected markup contents"),
        }
    }

    #[test]
    fn hover_on_used_export_single_reference() {
        let root = test_root();
        let path = root.join("src/utils.ts");
        let mut results = AnalysisResults::default();
        results.export_usages.push(ExportUsage {
            path: path.clone(),
            export_name: "helper".to_string(),
            line: 5,
            col: 0,
            reference_count: 1,
            reference_locations: vec![ReferenceLocation {
                path: root.join("src/app.ts"),
                line: 1,
                col: 0,
            }],
        });
        let duplication = empty_duplication();
        let pos = Position {
            line: 4,
            character: 0,
        };

        let hover = build_hover(&results, &duplication, &path, pos).unwrap();
        match &hover.contents {
            HoverContents::Markup(m) => {
                assert!(m.value.contains("1 file"));
                // Should not contain "files" (plural)
                assert!(!m.value.contains("1 files"));
            }
            _ => panic!("Expected markup contents"),
        }
    }

    #[test]
    fn hover_on_used_export_zero_refs_skipped() {
        let root = test_root();
        let path = root.join("src/utils.ts");
        let mut results = AnalysisResults::default();
        results.export_usages.push(ExportUsage {
            path: path.clone(),
            export_name: "unused".to_string(),
            line: 5,
            col: 0,
            reference_count: 0,
            reference_locations: vec![],
        });
        let duplication = empty_duplication();
        let pos = Position {
            line: 4,
            character: 0,
        };

        // Should not produce hover from export_usages for 0-ref export
        // (unused export check would handle it if present)
        let hover = build_hover(&results, &duplication, &path, pos);
        assert!(hover.is_none());
    }

    #[test]
    fn hover_on_unused_enum_member() {
        let root = test_root();
        let path = root.join("src/enums.ts");
        let mut results = AnalysisResults::default();
        results.unused_enum_members.push(UnusedMember {
            path: path.clone(),
            parent_name: "Color".to_string(),
            member_name: "Blue".to_string(),
            kind: MemberKind::EnumMember,
            line: 4,
            col: 2,
        });
        let duplication = empty_duplication();
        let pos = Position {
            line: 3,
            character: 5,
        };

        let hover = build_hover(&results, &duplication, &path, pos).unwrap();
        match &hover.contents {
            HoverContents::Markup(m) => {
                assert!(m.value.contains("Color.Blue"));
                assert!(m.value.contains("never used"));
            }
            _ => panic!("Expected markup contents"),
        }
    }

    #[test]
    fn hover_on_unused_class_member() {
        let root = test_root();
        let path = root.join("src/service.ts");
        let mut results = AnalysisResults::default();
        results.unused_class_members.push(UnusedMember {
            path: path.clone(),
            parent_name: "UserService".to_string(),
            member_name: "reset".to_string(),
            kind: MemberKind::ClassMethod,
            line: 20,
            col: 4,
        });
        let duplication = empty_duplication();
        let pos = Position {
            line: 19,
            character: 6,
        };

        let hover = build_hover(&results, &duplication, &path, pos).unwrap();
        match &hover.contents {
            HoverContents::Markup(m) => {
                assert!(m.value.contains("UserService.reset"));
                assert!(m.value.contains("Class member"));
            }
            _ => panic!("Expected markup contents"),
        }
    }

    #[test]
    fn hover_on_unresolved_import() {
        let root = test_root();
        let path = root.join("src/app.ts");
        let mut results = AnalysisResults::default();
        results.unresolved_imports.push(UnresolvedImport {
            path: path.clone(),
            specifier: "./missing-module".to_string(),
            line: 3,
            col: 20,
        });
        let duplication = empty_duplication();
        let pos = Position {
            line: 2,
            character: 25,
        };

        let hover = build_hover(&results, &duplication, &path, pos).unwrap();
        match &hover.contents {
            HoverContents::Markup(m) => {
                assert!(m.value.contains("./missing-module"));
                assert!(m.value.contains("Cannot resolve"));
            }
            _ => panic!("Expected markup contents"),
        }
    }

    #[test]
    fn hover_on_duplication() {
        let root = test_root();
        let path_a = root.join("src/a.ts");
        let path_b = root.join("src/b.ts");
        let results = AnalysisResults::default();
        let duplication = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![
                    CloneInstance {
                        file: path_a.clone(),
                        start_line: 10,
                        end_line: 15,
                        start_col: 0,
                        end_col: 20,
                        fragment: "duplicated code".to_string(),
                    },
                    CloneInstance {
                        file: path_b.clone(),
                        start_line: 20,
                        end_line: 25,
                        start_col: 4,
                        end_col: 24,
                        fragment: "duplicated code".to_string(),
                    },
                ],
                token_count: 50,
                line_count: 6,
            }],
            clone_families: vec![],
            stats: DuplicationStats {
                total_files: 2,
                files_with_clones: 2,
                total_lines: 100,
                duplicated_lines: 12,
                total_tokens: 500,
                duplicated_tokens: 100,
                clone_groups: 1,
                clone_instances: 2,
                duplication_percentage: 12.0,
            },
        };

        // Hover inside the duplication range in file a
        let pos = Position {
            line: 11, // Between lines 9 (0-based 10-1) and 14 (15-1)
            character: 5,
        };

        let hover = build_hover(&results, &duplication, &path_a, pos).unwrap();
        match &hover.contents {
            HoverContents::Markup(m) => {
                assert!(m.value.contains("6 lines"));
                assert!(m.value.contains("50 tokens"));
                assert!(m.value.contains("1 other instance"));
                assert!(m.value.contains("b.ts"));
            }
            _ => panic!("Expected markup contents"),
        }

        // Range should cover the duplication span
        let range = hover.range.unwrap();
        assert_eq!(range.start.line, 9); // 10 - 1
        assert_eq!(range.end.line, 14); // 15 - 1
    }

    #[test]
    fn hover_outside_duplication_range_returns_none() {
        let root = test_root();
        let path = root.join("src/a.ts");
        let results = AnalysisResults::default();
        let duplication = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![CloneInstance {
                    file: path.clone(),
                    start_line: 10,
                    end_line: 15,
                    start_col: 0,
                    end_col: 20,
                    fragment: "code".to_string(),
                }],
                token_count: 30,
                line_count: 6,
            }],
            clone_families: vec![],
            stats: DuplicationStats {
                total_files: 1,
                files_with_clones: 1,
                total_lines: 50,
                duplicated_lines: 6,
                total_tokens: 200,
                duplicated_tokens: 30,
                clone_groups: 1,
                clone_instances: 1,
                duplication_percentage: 12.0,
            },
        };

        // Position before the duplication
        let pos = Position {
            line: 5,
            character: 0,
        };
        let hover = build_hover(&results, &duplication, &path, pos);
        assert!(hover.is_none());

        // Position after the duplication
        let pos = Position {
            line: 20,
            character: 0,
        };
        let hover = build_hover(&results, &duplication, &path, pos);
        assert!(hover.is_none());
    }

    #[test]
    fn unused_file_takes_priority_over_export_info() {
        let root = test_root();
        let path = root.join("src/dead.ts");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile { path: path.clone() });
        results.export_usages.push(ExportUsage {
            path: path.clone(),
            export_name: "foo".to_string(),
            line: 5,
            col: 0,
            reference_count: 3,
            reference_locations: vec![],
        });
        let duplication = empty_duplication();
        let pos = Position {
            line: 4,
            character: 0,
        };

        // Should show unused file hover, not export usage
        let hover = build_hover(&results, &duplication, &path, pos).unwrap();
        match &hover.contents {
            HoverContents::Markup(m) => {
                assert!(m.value.contains("not imported"));
                assert!(m.value.contains("entry point"));
            }
            _ => panic!("Expected markup contents"),
        }
    }

    #[test]
    fn hover_on_wrong_line_returns_none() {
        let root = test_root();
        let path = root.join("src/utils.ts");
        let mut results = AnalysisResults::default();
        results.unused_exports.push(UnusedExport {
            path: path.clone(),
            export_name: "helper".to_string(),
            is_type_only: false,
            line: 5,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        let duplication = empty_duplication();

        // Line 10, but export is on line 5 (0-based: 4)
        let pos = Position {
            line: 10,
            character: 0,
        };
        let hover = build_hover(&results, &duplication, &path, pos);
        assert!(hover.is_none());
    }

    #[test]
    fn hover_duplication_multiple_instances() {
        let root = test_root();
        let path_a = root.join("src/a.ts");
        let path_b = root.join("src/b.ts");
        let path_c = root.join("src/c.ts");
        let results = AnalysisResults::default();
        let duplication = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![
                    CloneInstance {
                        file: path_a.clone(),
                        start_line: 1,
                        end_line: 5,
                        start_col: 0,
                        end_col: 10,
                        fragment: "code".to_string(),
                    },
                    CloneInstance {
                        file: path_b.clone(),
                        start_line: 10,
                        end_line: 14,
                        start_col: 0,
                        end_col: 10,
                        fragment: "code".to_string(),
                    },
                    CloneInstance {
                        file: path_c.clone(),
                        start_line: 20,
                        end_line: 24,
                        start_col: 0,
                        end_col: 10,
                        fragment: "code".to_string(),
                    },
                ],
                token_count: 30,
                line_count: 5,
            }],
            clone_families: vec![],
            stats: DuplicationStats {
                total_files: 3,
                files_with_clones: 3,
                total_lines: 100,
                duplicated_lines: 15,
                total_tokens: 500,
                duplicated_tokens: 90,
                clone_groups: 1,
                clone_instances: 3,
                duplication_percentage: 15.0,
            },
        };

        let pos = Position {
            line: 2,
            character: 0,
        };
        let hover = build_hover(&results, &duplication, &path_a, pos).unwrap();
        match &hover.contents {
            HoverContents::Markup(m) => {
                assert!(m.value.contains("2 other instances"));
                assert!(m.value.contains("b.ts"));
                assert!(m.value.contains("c.ts"));
            }
            _ => panic!("Expected markup contents"),
        }
    }
}
