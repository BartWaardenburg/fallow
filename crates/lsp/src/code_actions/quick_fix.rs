#[expect(clippy::disallowed_types)]
use std::collections::HashMap;
use std::path::Path;

use tower_lsp::lsp_types::*;

use fallow_core::results::AnalysisResults;

use crate::diagnostics::FIRST_LINE_RANGE;

/// Build quick-fix code actions for unused exports (remove the `export` keyword).
#[expect(clippy::disallowed_types)]
pub fn build_remove_export_actions(
    results: &AnalysisResults,
    file_path: &Path,
    uri: &Url,
    cursor_range: &Range,
    file_lines: &[&str],
) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();

    for (exports, msg_prefix) in [
        (&results.unused_exports, "Export"),
        (&results.unused_types, "Type export"),
    ] {
        for export in exports {
            if export.path != file_path {
                continue;
            }

            // export.line is a 1-based line number; convert to 0-based for LSP
            let export_line = export.line.saturating_sub(1);

            // Check if this diagnostic is in the requested range
            if export_line < cursor_range.start.line || export_line > cursor_range.end.line {
                continue;
            }

            // Determine the export prefix to remove by inspecting the line content
            let line_content = file_lines.get(export_line as usize).copied().unwrap_or("");
            let trimmed = line_content.trim_start();
            let indent_len = line_content.len() - trimmed.len();

            let prefix_to_remove = if trimmed.starts_with("export default ") {
                Some("export default ")
            } else if trimmed.starts_with("export ") {
                // Handles: export const, export function, export class, export type,
                // export interface, export enum, export abstract, export async,
                // export let, export var, etc.
                Some("export ")
            } else {
                None
            };

            let Some(prefix) = prefix_to_remove else {
                continue;
            };

            let title = format!("Remove unused export `{}`", export.export_name);
            let mut changes = HashMap::new();

            // Create a text edit that removes the export keyword prefix
            let edit = TextEdit {
                range: Range {
                    start: Position {
                        line: export_line,
                        character: indent_len as u32,
                    },
                    end: Position {
                        line: export_line,
                        character: (indent_len + prefix.len()) as u32,
                    },
                },
                new_text: String::new(),
            };

            changes.insert(uri.clone(), vec![edit]);

            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title,
                kind: Some(CodeActionKind::QUICKFIX),
                edit: Some(WorkspaceEdit {
                    changes: Some(changes),
                    ..Default::default()
                }),
                diagnostics: Some(vec![Diagnostic {
                    range: Range {
                        start: Position {
                            line: export_line,
                            character: export.col,
                        },
                        end: Position {
                            line: export_line,
                            character: export.col + export.export_name.len() as u32,
                        },
                    },
                    severity: Some(DiagnosticSeverity::HINT),
                    source: Some("fallow".to_string()),
                    message: format!("{msg_prefix} '{}' is unused", export.export_name),
                    tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                    ..Default::default()
                }]),
                ..Default::default()
            }));
        }
    }

    actions
}

/// Build quick-fix code actions for unused files (delete the file).
pub fn build_delete_file_actions(
    results: &AnalysisResults,
    file_path: &Path,
    uri: &Url,
    cursor_range: &Range,
) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();

    for file in &results.unused_files {
        if file.path != file_path {
            continue;
        }

        // The diagnostic is at line 0, col 0 — check if the request range overlaps
        if cursor_range.start.line > 0 {
            continue;
        }

        let title = "Delete this unused file".to_string();

        let delete_file_op = DocumentChangeOperation::Op(ResourceOp::Delete(DeleteFile {
            uri: uri.clone(),
            options: Some(DeleteFileOptions {
                recursive: Some(false),
                ignore_if_not_exists: Some(true),
                annotation_id: None,
            }),
        }));

        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title,
            kind: Some(CodeActionKind::QUICKFIX),
            edit: Some(WorkspaceEdit {
                document_changes: Some(DocumentChanges::Operations(vec![delete_file_op])),
                ..Default::default()
            }),
            diagnostics: Some(vec![Diagnostic {
                range: FIRST_LINE_RANGE,
                severity: Some(DiagnosticSeverity::WARNING),
                source: Some("fallow".to_string()),
                code: Some(NumberOrString::String("unused-file".to_string())),
                message: "File is not reachable from any entry point".to_string(),
                tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                ..Default::default()
            }]),
            ..Default::default()
        }));
    }

    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use fallow_core::results::{UnusedExport, UnusedFile};

    fn test_root() -> PathBuf {
        if cfg!(windows) {
            PathBuf::from("C:\\project")
        } else {
            PathBuf::from("/project")
        }
    }

    fn make_range(start_line: u32, end_line: u32) -> Range {
        Range {
            start: Position {
                line: start_line,
                character: 0,
            },
            end: Position {
                line: end_line,
                character: 0,
            },
        }
    }

    fn make_unused_export(path: &Path, name: &str, line: u32, col: u32) -> UnusedExport {
        UnusedExport {
            path: path.to_path_buf(),
            export_name: name.to_string(),
            is_type_only: false,
            line,
            col,
            span_start: 0,
            is_re_export: false,
        }
    }

    fn unwrap_code_action(action: &CodeActionOrCommand) -> &CodeAction {
        match action {
            CodeActionOrCommand::CodeAction(ca) => ca,
            CodeActionOrCommand::Command(_) => panic!("expected CodeAction, got Command"),
        }
    }

    // -----------------------------------------------------------------------
    // build_remove_export_actions
    // -----------------------------------------------------------------------

    #[test]
    fn no_export_actions_when_results_empty() {
        let root = test_root();
        let file = root.join("utils.ts");
        let uri = Url::from_file_path(&file).unwrap();
        let results = AnalysisResults::default();
        let lines = vec!["export const foo = 1;"];

        let actions = build_remove_export_actions(&results, &file, &uri, &make_range(0, 0), &lines);
        assert!(actions.is_empty());
    }

    #[test]
    fn no_export_actions_for_different_file() {
        let root = test_root();
        let file_a = root.join("a.ts");
        let file_b = root.join("b.ts");
        let uri_b = Url::from_file_path(&file_b).unwrap();

        let mut results = AnalysisResults::default();
        results
            .unused_exports
            .push(make_unused_export(&file_a, "foo", 1, 7));

        let lines = vec!["export const foo = 1;"];
        let actions =
            build_remove_export_actions(&results, &file_b, &uri_b, &make_range(0, 10), &lines);
        assert!(actions.is_empty());
    }

    #[test]
    fn no_export_actions_when_cursor_outside_export_line() {
        let root = test_root();
        let file = root.join("utils.ts");
        let uri = Url::from_file_path(&file).unwrap();

        let mut results = AnalysisResults::default();
        // Export on 1-based line 5 => 0-based line 4
        results
            .unused_exports
            .push(make_unused_export(&file, "bar", 5, 7));

        let lines = vec!["line0", "line1", "line2", "line3", "export const bar = 2;"];
        // Cursor on lines 0-2, export is on line 4
        let actions = build_remove_export_actions(&results, &file, &uri, &make_range(0, 2), &lines);
        assert!(actions.is_empty());
    }

    #[test]
    fn generates_action_for_unused_export_const() {
        let root = test_root();
        let file = root.join("utils.ts");
        let uri = Url::from_file_path(&file).unwrap();

        let mut results = AnalysisResults::default();
        results
            .unused_exports
            .push(make_unused_export(&file, "foo", 1, 13));

        let lines = vec!["export const foo = 42;"];
        let actions = build_remove_export_actions(&results, &file, &uri, &make_range(0, 0), &lines);

        assert_eq!(actions.len(), 1);
        let ca = unwrap_code_action(&actions[0]);

        assert_eq!(ca.title, "Remove unused export `foo`");
        assert_eq!(ca.kind, Some(CodeActionKind::QUICKFIX));

        // The edit should remove "export " (7 chars starting at column 0)
        let changes = ca.edit.as_ref().unwrap().changes.as_ref().unwrap();
        let edits = changes.get(&uri).unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].range.start.character, 0);
        assert_eq!(edits[0].range.end.character, 7); // "export " = 7 chars
        assert_eq!(edits[0].new_text, "");
    }

    #[test]
    fn generates_action_for_export_default() {
        let root = test_root();
        let file = root.join("component.tsx");
        let uri = Url::from_file_path(&file).unwrap();

        let mut results = AnalysisResults::default();
        results
            .unused_exports
            .push(make_unused_export(&file, "default", 1, 0));

        let lines = vec!["export default function App() {}"];
        let actions = build_remove_export_actions(&results, &file, &uri, &make_range(0, 0), &lines);

        assert_eq!(actions.len(), 1);
        let ca = unwrap_code_action(&actions[0]);

        let changes = ca.edit.as_ref().unwrap().changes.as_ref().unwrap();
        let edits = changes.get(&uri).unwrap();
        assert_eq!(edits.len(), 1);
        // "export default " = 15 chars
        assert_eq!(edits[0].range.start.character, 0);
        assert_eq!(edits[0].range.end.character, 15);
        assert_eq!(edits[0].new_text, "");
    }

    #[test]
    fn preserves_indentation_in_edit_range() {
        let root = test_root();
        let file = root.join("nested.ts");
        let uri = Url::from_file_path(&file).unwrap();

        let mut results = AnalysisResults::default();
        // Export on 1-based line 2 => 0-based line 1
        results
            .unused_exports
            .push(make_unused_export(&file, "helper", 2, 11));

        let lines = vec![
            "namespace Ns {",
            "    export function helper() {}", // 4 spaces indent
            "}",
        ];
        let actions = build_remove_export_actions(&results, &file, &uri, &make_range(1, 1), &lines);

        assert_eq!(actions.len(), 1);
        let ca = unwrap_code_action(&actions[0]);

        let changes = ca.edit.as_ref().unwrap().changes.as_ref().unwrap();
        let edits = changes.get(&uri).unwrap();
        assert_eq!(edits.len(), 1);
        // Edit should start at column 4 (after indent) and remove "export " (7 chars)
        assert_eq!(edits[0].range.start.line, 1);
        assert_eq!(edits[0].range.start.character, 4);
        assert_eq!(edits[0].range.end.character, 11); // 4 + 7
    }

    #[test]
    fn handles_type_exports() {
        let root = test_root();
        let file = root.join("types.ts");
        let uri = Url::from_file_path(&file).unwrap();

        let mut results = AnalysisResults::default();
        results.unused_types.push(UnusedExport {
            path: file.clone(),
            export_name: "MyType".to_string(),
            is_type_only: true,
            line: 1,
            col: 12,
            span_start: 0,
            is_re_export: false,
        });

        let lines = vec!["export type MyType = string;"];
        let actions = build_remove_export_actions(&results, &file, &uri, &make_range(0, 0), &lines);

        assert_eq!(actions.len(), 1);
        let ca = unwrap_code_action(&actions[0]);

        // Check the diagnostic message uses "Type export" prefix
        let diags = ca.diagnostics.as_ref().unwrap();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].message, "Type export 'MyType' is unused");
        assert_eq!(diags[0].severity, Some(DiagnosticSeverity::HINT));
        assert_eq!(diags[0].source, Some("fallow".to_string()));
        assert_eq!(diags[0].tags, Some(vec![DiagnosticTag::UNNECESSARY]));
    }

    #[test]
    fn combines_unused_exports_and_unused_types() {
        let root = test_root();
        let file = root.join("mixed.ts");
        let uri = Url::from_file_path(&file).unwrap();

        let mut results = AnalysisResults::default();
        results
            .unused_exports
            .push(make_unused_export(&file, "foo", 1, 13));
        results.unused_types.push(UnusedExport {
            path: file.clone(),
            export_name: "Bar".to_string(),
            is_type_only: true,
            line: 2,
            col: 12,
            span_start: 0,
            is_re_export: false,
        });

        let lines = vec!["export const foo = 1;", "export type Bar = string;"];
        let actions = build_remove_export_actions(&results, &file, &uri, &make_range(0, 1), &lines);

        assert_eq!(actions.len(), 2);

        let ca0 = unwrap_code_action(&actions[0]);
        let ca1 = unwrap_code_action(&actions[1]);

        assert_eq!(ca0.title, "Remove unused export `foo`");
        assert_eq!(ca1.title, "Remove unused export `Bar`");

        // Verify message prefixes differ
        let diag0 = &ca0.diagnostics.as_ref().unwrap()[0];
        let diag1 = &ca1.diagnostics.as_ref().unwrap()[0];
        assert!(diag0.message.starts_with("Export "));
        assert!(diag1.message.starts_with("Type export "));
    }

    #[test]
    fn skips_line_without_export_prefix() {
        let root = test_root();
        let file = root.join("odd.ts");
        let uri = Url::from_file_path(&file).unwrap();

        let mut results = AnalysisResults::default();
        // The result says line 1 has an unused export, but the actual line content
        // doesn't start with "export" (e.g., re-export or corrupted data)
        results
            .unused_exports
            .push(make_unused_export(&file, "foo", 1, 0));

        let lines = vec!["const foo = 1;"];
        let actions = build_remove_export_actions(&results, &file, &uri, &make_range(0, 0), &lines);
        assert!(
            actions.is_empty(),
            "Should skip exports where line doesn't start with 'export'"
        );
    }

    #[test]
    fn handles_export_on_line_0_saturating_sub() {
        let root = test_root();
        let file = root.join("edge.ts");
        let uri = Url::from_file_path(&file).unwrap();

        let mut results = AnalysisResults::default();
        // line=0 is unusual (lines are 1-based), but saturating_sub(1) handles it
        // gracefully by producing 0-based line 0 (same as line=1 would)
        results
            .unused_exports
            .push(make_unused_export(&file, "x", 0, 7));

        let lines = vec!["export const x = 1;"];
        let actions = build_remove_export_actions(&results, &file, &uri, &make_range(0, 0), &lines);

        // saturating_sub(0, 1) = 0, so it maps to line 0 which is in range
        assert_eq!(actions.len(), 1);
    }

    #[test]
    fn multiple_exports_same_file_all_in_range() {
        let root = test_root();
        let file = root.join("multi.ts");
        let uri = Url::from_file_path(&file).unwrap();

        let mut results = AnalysisResults::default();
        results
            .unused_exports
            .push(make_unused_export(&file, "a", 1, 7));
        results
            .unused_exports
            .push(make_unused_export(&file, "b", 2, 7));
        results
            .unused_exports
            .push(make_unused_export(&file, "c", 3, 7));

        let lines = vec![
            "export function a() {}",
            "export function b() {}",
            "export function c() {}",
        ];
        let actions = build_remove_export_actions(&results, &file, &uri, &make_range(0, 2), &lines);

        assert_eq!(actions.len(), 3);
        for action in &actions {
            let ca = unwrap_code_action(action);
            assert_eq!(ca.kind, Some(CodeActionKind::QUICKFIX));
        }
    }

    #[test]
    fn cursor_range_filters_subset_of_exports() {
        let root = test_root();
        let file = root.join("filter.ts");
        let uri = Url::from_file_path(&file).unwrap();

        let mut results = AnalysisResults::default();
        results
            .unused_exports
            .push(make_unused_export(&file, "a", 1, 7));
        results
            .unused_exports
            .push(make_unused_export(&file, "b", 3, 7));
        results
            .unused_exports
            .push(make_unused_export(&file, "c", 5, 7));

        let lines = vec![
            "export const a = 1;",
            "const used = true;",
            "export const b = 2;",
            "const also_used = false;",
            "export const c = 3;",
        ];
        // Cursor covers only line 2 (0-based), which is 1-based line 3 => export "b"
        let actions = build_remove_export_actions(&results, &file, &uri, &make_range(2, 2), &lines);

        assert_eq!(actions.len(), 1);
        let ca = unwrap_code_action(&actions[0]);
        assert_eq!(ca.title, "Remove unused export `b`");
    }

    #[test]
    fn diagnostic_range_matches_export_name_span() {
        let root = test_root();
        let file = root.join("span.ts");
        let uri = Url::from_file_path(&file).unwrap();

        let mut results = AnalysisResults::default();
        // col=13 means "export const " (13 chars), name "myLongExport" is 12 chars
        results
            .unused_exports
            .push(make_unused_export(&file, "myLongExport", 1, 13));

        let lines = vec!["export const myLongExport = 42;"];
        let actions = build_remove_export_actions(&results, &file, &uri, &make_range(0, 0), &lines);

        assert_eq!(actions.len(), 1);
        let ca = unwrap_code_action(&actions[0]);
        let diag = &ca.diagnostics.as_ref().unwrap()[0];

        assert_eq!(diag.range.start.line, 0);
        assert_eq!(diag.range.start.character, 13);
        assert_eq!(diag.range.end.line, 0);
        assert_eq!(diag.range.end.character, 25); // 13 + 12
    }

    #[test]
    fn handles_empty_file_lines() {
        let root = test_root();
        let file = root.join("empty.ts");
        let uri = Url::from_file_path(&file).unwrap();

        let mut results = AnalysisResults::default();
        results
            .unused_exports
            .push(make_unused_export(&file, "x", 1, 0));

        // No lines at all — the get() call returns None, unwrap_or("")
        let lines: Vec<&str> = vec![];
        let actions = build_remove_export_actions(&results, &file, &uri, &make_range(0, 0), &lines);
        // Empty string doesn't start with "export", so no action
        assert!(actions.is_empty());
    }

    #[test]
    fn handles_tab_indentation() {
        let root = test_root();
        let file = root.join("tabs.ts");
        let uri = Url::from_file_path(&file).unwrap();

        let mut results = AnalysisResults::default();
        results
            .unused_exports
            .push(make_unused_export(&file, "val", 1, 0));

        let lines = vec!["\t\texport const val = 1;"]; // 2 tabs of indent
        let actions = build_remove_export_actions(&results, &file, &uri, &make_range(0, 0), &lines);

        assert_eq!(actions.len(), 1);
        let ca = unwrap_code_action(&actions[0]);
        let changes = ca.edit.as_ref().unwrap().changes.as_ref().unwrap();
        let edits = changes.get(&uri).unwrap();
        // 2 bytes of tab indent + "export " (7 chars) = columns 2..9
        assert_eq!(edits[0].range.start.character, 2);
        assert_eq!(edits[0].range.end.character, 9);
    }

    // -----------------------------------------------------------------------
    // build_delete_file_actions
    // -----------------------------------------------------------------------

    #[test]
    fn no_delete_actions_when_no_unused_files() {
        let root = test_root();
        let file = root.join("used.ts");
        let uri = Url::from_file_path(&file).unwrap();
        let results = AnalysisResults::default();

        let actions = build_delete_file_actions(&results, &file, &uri, &make_range(0, 10));
        assert!(actions.is_empty());
    }

    #[test]
    fn no_delete_action_for_different_file() {
        let root = test_root();
        let file_a = root.join("a.ts");
        let file_b = root.join("b.ts");
        let uri_b = Url::from_file_path(&file_b).unwrap();

        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile { path: file_a });

        let actions = build_delete_file_actions(&results, &file_b, &uri_b, &make_range(0, 10));
        assert!(actions.is_empty());
    }

    #[test]
    fn no_delete_action_when_cursor_not_at_line_0() {
        let root = test_root();
        let file = root.join("unused.ts");
        let uri = Url::from_file_path(&file).unwrap();

        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile { path: file.clone() });

        // Cursor starts at line 1, but diagnostic is at line 0
        let actions = build_delete_file_actions(&results, &file, &uri, &make_range(1, 5));
        assert!(actions.is_empty());
    }

    #[test]
    fn generates_delete_action_when_cursor_at_line_0() {
        let root = test_root();
        let file = root.join("unused.ts");
        let uri = Url::from_file_path(&file).unwrap();

        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile { path: file.clone() });

        let actions = build_delete_file_actions(&results, &file, &uri, &make_range(0, 0));

        assert_eq!(actions.len(), 1);
        let ca = unwrap_code_action(&actions[0]);

        assert_eq!(ca.title, "Delete this unused file");
        assert_eq!(ca.kind, Some(CodeActionKind::QUICKFIX));
    }

    #[test]
    fn delete_action_uses_document_changes_with_delete_op() {
        let root = test_root();
        let file = root.join("unused.ts");
        let uri = Url::from_file_path(&file).unwrap();

        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile { path: file.clone() });

        let actions = build_delete_file_actions(&results, &file, &uri, &make_range(0, 0));
        let ca = unwrap_code_action(&actions[0]);

        let doc_changes = ca.edit.as_ref().unwrap().document_changes.as_ref().unwrap();

        match doc_changes {
            DocumentChanges::Operations(ops) => {
                assert_eq!(ops.len(), 1);
                match &ops[0] {
                    DocumentChangeOperation::Op(ResourceOp::Delete(del)) => {
                        assert_eq!(del.uri, uri);
                        let opts = del.options.as_ref().unwrap();
                        assert_eq!(opts.recursive, Some(false));
                        assert_eq!(opts.ignore_if_not_exists, Some(true));
                    }
                    other => panic!("expected Delete op, got: {other:?}"),
                }
            }
            other @ DocumentChanges::Edits(_) => panic!("expected Operations, got: {other:?}"),
        }
    }

    #[test]
    fn delete_action_diagnostic_has_correct_properties() {
        let root = test_root();
        let file = root.join("unused.ts");
        let uri = Url::from_file_path(&file).unwrap();

        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile { path: file.clone() });

        let actions = build_delete_file_actions(&results, &file, &uri, &make_range(0, 0));
        let ca = unwrap_code_action(&actions[0]);

        let diags = ca.diagnostics.as_ref().unwrap();
        assert_eq!(diags.len(), 1);
        let diag = &diags[0];

        assert_eq!(diag.range, FIRST_LINE_RANGE);
        assert_eq!(diag.severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(diag.source, Some("fallow".to_string()));
        assert_eq!(
            diag.code,
            Some(NumberOrString::String("unused-file".to_string()))
        );
        assert_eq!(diag.message, "File is not reachable from any entry point");
        assert_eq!(diag.tags, Some(vec![DiagnosticTag::UNNECESSARY]));
    }

    #[test]
    fn delete_action_with_cursor_spanning_line_0() {
        let root = test_root();
        let file = root.join("unused.ts");
        let uri = Url::from_file_path(&file).unwrap();

        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile { path: file.clone() });

        // Cursor from line 0 to line 50 — should still trigger because start.line == 0
        let actions = build_delete_file_actions(&results, &file, &uri, &make_range(0, 50));
        assert_eq!(actions.len(), 1);
    }

    #[test]
    fn multiple_unused_files_same_path_produces_multiple_actions() {
        let root = test_root();
        let file = root.join("unused.ts");
        let uri = Url::from_file_path(&file).unwrap();

        let mut results = AnalysisResults::default();
        // Unlikely in practice, but tests that the loop iterates all entries
        results.unused_files.push(UnusedFile { path: file.clone() });
        results.unused_files.push(UnusedFile { path: file.clone() });

        let actions = build_delete_file_actions(&results, &file, &uri, &make_range(0, 0));
        assert_eq!(actions.len(), 2);
    }

    // -----------------------------------------------------------------------
    // Integration: both functions together on same file
    // -----------------------------------------------------------------------

    #[test]
    fn unused_file_and_unused_export_in_same_file() {
        let root = test_root();
        let file = root.join("orphan.ts");
        let uri = Url::from_file_path(&file).unwrap();

        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile { path: file.clone() });
        results
            .unused_exports
            .push(make_unused_export(&file, "helper", 1, 16));

        let lines = vec!["export function helper() {}"];
        let cursor = make_range(0, 0);

        let export_actions = build_remove_export_actions(&results, &file, &uri, &cursor, &lines);
        let delete_actions = build_delete_file_actions(&results, &file, &uri, &cursor);

        // Both produce independent actions
        assert_eq!(export_actions.len(), 1);
        assert_eq!(delete_actions.len(), 1);

        // They are different action types
        let export_ca = unwrap_code_action(&export_actions[0]);
        let delete_ca = unwrap_code_action(&delete_actions[0]);
        assert!(export_ca.title.contains("Remove unused export"));
        assert!(delete_ca.title.contains("Delete"));
    }
}
