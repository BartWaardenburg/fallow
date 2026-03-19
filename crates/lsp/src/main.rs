use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{Mutex, RwLock};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use fallow_core::duplicates::{CloneGroup, DuplicationReport};
use fallow_core::results::AnalysisResults;

/// LSP range at position (0,0) used for file-level and package.json diagnostics.
const ZERO_RANGE: Range = Range {
    start: Position {
        line: 0,
        character: 0,
    },
    end: Position {
        line: 0,
        character: 0,
    },
};

struct FallowLspServer {
    client: Client,
    root: Arc<RwLock<Option<PathBuf>>>,
    results: Arc<RwLock<Option<AnalysisResults>>>,
    duplication: Arc<RwLock<Option<DuplicationReport>>>,
    previous_diagnostic_uris: Arc<RwLock<HashSet<Url>>>,
    last_analysis: Arc<Mutex<Instant>>,
    analysis_guard: Arc<tokio::sync::Mutex<()>>,
    documents: Arc<RwLock<HashMap<Url, String>>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for FallowLspServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let root = params
            .root_uri
            .and_then(|u| u.to_file_path().ok())
            .or_else(|| {
                params
                    .workspace_folders
                    .as_deref()
                    .and_then(|fs| fs.first())
                    .and_then(|f| f.uri.to_file_path().ok())
            });
        if let Some(path) = root {
            *self.root.write().await = Some(path);
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("fallow".to_string()),
                        inter_file_dependencies: true,
                        workspace_diagnostics: true,
                        ..Default::default()
                    },
                )),
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![
                            CodeActionKind::QUICKFIX,
                            CodeActionKind::REFACTOR_EXTRACT,
                        ]),
                        ..Default::default()
                    },
                )),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false),
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "fallow LSP server initialized")
            .await;

        // Run initial analysis
        self.run_analysis().await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_save(&self, _params: DidSaveTextDocumentParams) {
        // Debounce: skip if last analysis was less than 500ms ago
        let now = Instant::now();
        {
            let last = self.last_analysis.lock().await;
            if now.duration_since(*last) < std::time::Duration::from_millis(500) {
                return;
            }
        }

        // Re-run analysis on save
        self.run_analysis().await;

        // Update timestamp AFTER analysis completes so long-running analyses
        // don't cause subsequent save events to be silently skipped
        *self.last_analysis.lock().await = Instant::now();
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // Store latest document text for code actions
        if let Some(change) = params.content_changes.into_iter().last() {
            self.documents
                .write()
                .await
                .insert(params.text_document.uri, change.text);
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents
            .write()
            .await
            .remove(&params.text_document.uri);
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let results = self.results.read().await;
        let Some(results) = results.as_ref() else {
            return Ok(None);
        };

        let uri = &params.text_document.uri;
        let file_path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };

        let mut actions = Vec::new();

        // Read file content once for computing line positions and edit ranges.
        // Prefer in-memory document text (from did_change), fall back to disk.
        let documents = self.documents.read().await;
        let file_content = match documents.get(uri) {
            Some(text) => text.clone(),
            None => std::fs::read_to_string(&file_path).unwrap_or_default(),
        };
        drop(documents);
        let file_lines: Vec<&str> = file_content.lines().collect();

        // Generate "Remove export" code actions for unused exports
        for export in results
            .unused_exports
            .iter()
            .chain(results.unused_types.iter())
        {
            if export.path != file_path {
                continue;
            }

            // export.line is a 1-based line number; convert to 0-based for LSP
            let export_line = export.line.saturating_sub(1);

            // Check if this diagnostic is in the requested range
            if export_line < params.range.start.line || export_line > params.range.end.line {
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
                    message: format!("Export '{}' is unused", export.export_name),
                    tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                    ..Default::default()
                }]),
                ..Default::default()
            }));
        }

        // Generate "Delete this file" code actions for unused files
        for file in &results.unused_files {
            if file.path != file_path {
                continue;
            }

            // The diagnostic is at line 0, col 0 — check if the request range overlaps
            if params.range.start.line > 0 {
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
                    range: ZERO_RANGE,
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

        // Generate "Extract duplicate" code actions for duplication diagnostics
        {
            let duplication = self.duplication.read().await;
            if let Some(ref report) = *duplication {
                let extract_actions = build_extract_duplicate_actions(
                    &file_path,
                    uri,
                    &params.range,
                    &report.clone_groups,
                    &file_lines,
                );
                actions.extend(extract_actions);
            }
        }

        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let results = self.results.read().await;
        let Some(results) = results.as_ref() else {
            return Ok(None);
        };

        let file_path = match params.text_document.uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };

        let lenses: Vec<CodeLens> = results
            .export_usages
            .iter()
            .filter(|usage| usage.path == file_path)
            .map(|usage| {
                // usage.line is 1-based; LSP positions are 0-based
                let line = usage.line.saturating_sub(1);
                let title = if usage.reference_count == 1 {
                    "1 reference".to_string()
                } else {
                    format!("{} references", usage.reference_count)
                };

                let export_position = Position {
                    line,
                    character: usage.col,
                };

                // Build reference Location objects for editor.action.showReferences
                let ref_locations: Vec<serde_json::Value> = usage
                    .reference_locations
                    .iter()
                    .filter_map(|loc| {
                        let uri = Url::from_file_path(&loc.path).ok()?;
                        let ref_line = loc.line.saturating_sub(1);
                        Some(serde_json::json!({
                            "uri": uri.as_str(),
                            "range": {
                                "start": { "line": ref_line, "character": loc.col },
                                "end": { "line": ref_line, "character": loc.col }
                            }
                        }))
                    })
                    .collect();

                // Use editor.action.showReferences when we have reference locations,
                // fall back to display-only noop otherwise
                let (command_name, arguments) = if ref_locations.is_empty() {
                    ("fallow.noop".to_string(), None)
                } else {
                    (
                        "editor.action.showReferences".to_string(),
                        Some(vec![
                            serde_json::json!(params.text_document.uri.as_str()),
                            serde_json::json!({
                                "line": export_position.line,
                                "character": export_position.character,
                            }),
                            serde_json::json!(ref_locations),
                        ]),
                    )
                };

                CodeLens {
                    range: Range {
                        start: export_position,
                        end: export_position,
                    },
                    command: Some(Command {
                        title,
                        command: command_name,
                        arguments,
                    }),
                    data: None,
                }
            })
            .collect();

        if lenses.is_empty() {
            Ok(None)
        } else {
            Ok(Some(lenses))
        }
    }
}

/// Build "Extract duplicate into function" code actions for clone groups overlapping the cursor.
fn build_extract_duplicate_actions(
    file_path: &Path,
    uri: &Url,
    cursor_range: &Range,
    clone_groups: &[CloneGroup],
    file_lines: &[&str],
) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();
    let mut extract_count: u32 = 0;
    let total_lines = file_lines.len() as u32;

    for group in clone_groups {
        // Find instances in this file that overlap the cursor range
        let instances_in_file: Vec<_> = group
            .instances
            .iter()
            .filter(|inst| inst.file == file_path)
            .collect();

        if instances_in_file.is_empty() {
            continue;
        }

        // Check if any instance overlaps the cursor range (1-based to 0-based)
        let overlapping = instances_in_file.iter().any(|inst| {
            let inst_start_line = (inst.start_line as u32).saturating_sub(1);
            let inst_end_line = (inst.end_line as u32).saturating_sub(1);
            inst_start_line <= cursor_range.end.line && inst_end_line >= cursor_range.start.line
        });

        if !overlapping {
            continue;
        }

        extract_count += 1;
        let func_name = if extract_count == 1 {
            "extractedDuplicate".to_string()
        } else {
            format!("extractedDuplicate{extract_count}")
        };
        let instance_count_in_file = instances_in_file.len();
        let has_cross_file_instances = group.instances.iter().any(|inst| inst.file != file_path);

        let title = if instance_count_in_file > 1 && has_cross_file_instances {
            format!(
                "Extract duplicate into function ({instance_count_in_file} instances in this file, others remain)"
            )
        } else if instance_count_in_file > 1 {
            format!(
                "Extract duplicate into function ({instance_count_in_file} instances in this file)"
            )
        } else if has_cross_file_instances {
            "Extract duplicate into function (other files unchanged)".to_string()
        } else {
            "Extract duplicate into function".to_string()
        };

        // Build the function body from the fragment of the first instance.
        // Strip common leading whitespace (dedent), then re-indent with 2 spaces.
        let fragment = &instances_in_file[0].fragment;
        let common_indent = fragment
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.len() - line.trim_start().len())
            .min()
            .unwrap_or(0);
        let indented_fragment: String = fragment
            .lines()
            .map(|line| {
                let stripped = if line.len() > common_indent {
                    &line[common_indent..]
                } else {
                    line.trim_start()
                };
                if stripped.is_empty() {
                    String::new()
                } else {
                    format!("  {stripped}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        let function_text = format!(
            "function {func_name}() {{\n\
             {indented_fragment}\n\
             }}\n\n"
        );

        let first_instance = instances_in_file[0];
        let first_start_0based = (first_instance.start_line as u32).saturating_sub(1);

        // Find a suitable insert position at module scope (no indentation) above
        // the first instance. Walk backwards to avoid inserting inside a function body.
        let insert_line = {
            let mut line = first_start_0based;
            while line > 0 {
                line -= 1;
                let content = file_lines.get(line as usize).copied().unwrap_or("");
                // An empty line or a line starting at column 0 (module scope) is a good insert point
                if content.is_empty() || (!content.starts_with(' ') && !content.starts_with('\t')) {
                    break;
                }
            }
            line
        };
        let can_insert_separately = insert_line < first_start_0based;

        let mut edits: Vec<TextEdit> = Vec::new();

        if can_insert_separately {
            // Insert the extracted function before the first instance
            edits.push(TextEdit {
                range: Range {
                    start: Position {
                        line: insert_line,
                        character: 0,
                    },
                    end: Position {
                        line: insert_line,
                        character: 0,
                    },
                },
                new_text: function_text.clone(),
            });
        }

        // Replace each instance in this file with a function call.
        for (i, inst) in instances_in_file.iter().enumerate() {
            let inst_start_line = (inst.start_line as u32).saturating_sub(1);
            let inst_end_line = (inst.end_line as u32)
                .saturating_sub(1)
                .min(total_lines.saturating_sub(1));

            // Derive indentation from the original first line
            let indent = file_lines
                .get(inst_start_line as usize)
                .map(|line| {
                    let trimmed = line.trim_start();
                    &line[..line.len() - trimmed.len()]
                })
                .unwrap_or("");

            let call_text = format!("{indent}{func_name}();\n");

            // For the first instance when we can't insert separately (clone starts at
            // line 0), prepend the function definition to the replacement text.
            let replacement = if i == 0 && !can_insert_separately {
                format!("{function_text}{call_text}")
            } else {
                call_text
            };

            // Clamp end line to document bounds
            let end_line = (inst_end_line + 1).min(total_lines);

            edits.push(TextEdit {
                range: Range {
                    start: Position {
                        line: inst_start_line,
                        character: 0,
                    },
                    end: Position {
                        line: end_line,
                        character: 0,
                    },
                },
                new_text: replacement,
            });
        }

        // Sort edits in reverse document order for LSP spec compliance
        edits.sort_by(|a, b| {
            b.range
                .start
                .line
                .cmp(&a.range.start.line)
                .then(b.range.start.character.cmp(&a.range.start.character))
        });

        let mut changes = HashMap::new();
        changes.insert(uri.clone(), edits);

        // Build the diagnostic that this action is associated with
        let diag_instance = instances_in_file[0];
        let diag_start_line = (diag_instance.start_line as u32).saturating_sub(1);
        let diag_end_line = (diag_instance.end_line as u32).saturating_sub(1);

        // Build related information for other instances
        let related_info: Vec<DiagnosticRelatedInformation> = group
            .instances
            .iter()
            .filter(|inst| {
                // Exclude the current diagnostic instance
                !(inst.file == file_path && inst.start_line == diag_instance.start_line)
            })
            .filter_map(|inst| {
                let inst_uri = Url::from_file_path(&inst.file).ok()?;
                Some(DiagnosticRelatedInformation {
                    location: Location {
                        uri: inst_uri,
                        range: Range {
                            start: Position {
                                line: (inst.start_line as u32).saturating_sub(1),
                                character: inst.start_col as u32,
                            },
                            end: Position {
                                line: (inst.end_line as u32).saturating_sub(1),
                                character: inst.end_col as u32,
                            },
                        },
                    },
                    message: "Also duplicated here".to_string(),
                })
            })
            .collect();

        let diagnostic = Diagnostic {
            range: Range {
                start: Position {
                    line: diag_start_line,
                    character: diag_instance.start_col as u32,
                },
                end: Position {
                    line: diag_end_line,
                    character: diag_instance.end_col as u32,
                },
            },
            severity: Some(DiagnosticSeverity::HINT),
            source: Some("fallow".to_string()),
            code: Some(NumberOrString::String("code-duplication".to_string())),
            message: format!(
                "Duplicated code block ({} lines, {} instances)",
                group.line_count,
                group.instances.len()
            ),
            related_information: if related_info.is_empty() {
                None
            } else {
                Some(related_info)
            },
            ..Default::default()
        };

        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title,
            kind: Some(CodeActionKind::REFACTOR_EXTRACT),
            edit: Some(WorkspaceEdit {
                changes: Some(changes),
                ..Default::default()
            }),
            diagnostics: Some(vec![diagnostic]),
            ..Default::default()
        }));
    }

    actions
}

impl FallowLspServer {
    async fn run_analysis(&self) {
        let root = self.root.read().await.clone();
        let Some(root) = root else { return };

        let _guard = match self.analysis_guard.try_lock() {
            Ok(guard) => guard,
            Err(_) => return, // analysis already running
        };

        self.client
            .log_message(MessageType::INFO, "Running fallow analysis...")
            .await;

        let root_clone = root.clone();
        let join_result = tokio::task::spawn_blocking(move || {
            let analysis = fallow_core::analyze_project(&root_clone);

            // Load user's duplication config, falling back to defaults
            let dupes_config = fallow_config::FallowConfig::find_and_load(&root_clone)
                .ok()
                .flatten()
                .map(|(c, _)| c.duplicates)
                .unwrap_or_default();

            let duplication =
                fallow_core::duplicates::find_duplicates_in_project(&root_clone, &dupes_config);

            (analysis, duplication)
        })
        .await;

        match join_result {
            Ok((Ok(results), duplication)) => {
                self.publish_diagnostics(&results, &duplication, &root)
                    .await;
                *self.results.write().await = Some(results);
                *self.duplication.write().await = Some(duplication);

                // Notify the client to re-request Code Lenses with the fresh data
                let _ = self.client.code_lens_refresh().await;

                self.client
                    .log_message(MessageType::INFO, "Analysis complete")
                    .await;
            }
            Ok((Err(e), _)) => {
                self.client
                    .log_message(MessageType::ERROR, format!("Analysis error: {e}"))
                    .await;
            }
            Err(e) => {
                self.client
                    .log_message(MessageType::ERROR, format!("Analysis failed: {e}"))
                    .await;
            }
        }
    }

    async fn publish_diagnostics(
        &self,
        results: &AnalysisResults,
        duplication: &DuplicationReport,
        root: &Path,
    ) {
        // Collect diagnostics per file
        let mut diagnostics_by_file: HashMap<Url, Vec<Diagnostic>> = HashMap::new();

        // Helper: get the package.json URI for dependency-related diagnostics
        let package_json_path = root.join("package.json");
        let package_json_uri = Url::from_file_path(&package_json_path).ok();

        // Export-like issues: unused exports and unused types
        for (exports, code, msg_prefix) in [
            (&results.unused_exports, "unused-export", "Export" as &str),
            (&results.unused_types, "unused-type", "Type export"),
        ] {
            for export in exports {
                if let Ok(uri) = Url::from_file_path(&export.path) {
                    let line = export.line.saturating_sub(1);
                    diagnostics_by_file
                        .entry(uri)
                        .or_default()
                        .push(Diagnostic {
                            range: Range {
                                start: Position {
                                    line,
                                    character: export.col,
                                },
                                end: Position {
                                    line,
                                    character: export.col + export.export_name.len() as u32,
                                },
                            },
                            severity: Some(DiagnosticSeverity::HINT),
                            source: Some("fallow".to_string()),
                            code: Some(NumberOrString::String(code.to_string())),
                            message: format!("{msg_prefix} '{}' is unused", export.export_name),
                            tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                            ..Default::default()
                        });
                }
            }
        }

        // Unused files: path-only diagnostic at (0,0)
        for file in &results.unused_files {
            if let Ok(uri) = Url::from_file_path(&file.path) {
                diagnostics_by_file
                    .entry(uri)
                    .or_default()
                    .push(Diagnostic {
                        range: ZERO_RANGE,
                        severity: Some(DiagnosticSeverity::WARNING),
                        source: Some("fallow".to_string()),
                        code: Some(NumberOrString::String("unused-file".to_string())),
                        message: "File is not reachable from any entry point".to_string(),
                        tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                        ..Default::default()
                    });
            }
        }

        // Unresolved imports
        for import in &results.unresolved_imports {
            if let Ok(uri) = Url::from_file_path(&import.path) {
                let line = import.line.saturating_sub(1);
                diagnostics_by_file
                    .entry(uri)
                    .or_default()
                    .push(Diagnostic {
                        range: Range {
                            start: Position {
                                line,
                                character: import.col,
                            },
                            end: Position {
                                line,
                                character: import.col,
                            },
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        source: Some("fallow".to_string()),
                        code: Some(NumberOrString::String("unresolved-import".to_string())),
                        message: format!("Cannot resolve import '{}'", import.specifier),
                        ..Default::default()
                    });
            }
        }

        // Dependency issues: unused deps, unused dev deps (routed to their respective package.json)
        for dep in &results.unused_dependencies {
            if let Ok(dep_uri) = Url::from_file_path(&dep.path) {
                let entry = diagnostics_by_file.entry(dep_uri).or_default();
                entry.push(Diagnostic {
                    range: ZERO_RANGE,
                    severity: Some(DiagnosticSeverity::WARNING),
                    source: Some("fallow".to_string()),
                    code: Some(NumberOrString::String("unused-dependency".to_string())),
                    message: format!("Unused dependency: {}", dep.package_name),
                    ..Default::default()
                });
            }
        }
        for dep in &results.unused_dev_dependencies {
            if let Ok(dep_uri) = Url::from_file_path(&dep.path) {
                let entry = diagnostics_by_file.entry(dep_uri).or_default();
                entry.push(Diagnostic {
                    range: ZERO_RANGE,
                    severity: Some(DiagnosticSeverity::WARNING),
                    source: Some("fallow".to_string()),
                    code: Some(NumberOrString::String("unused-dev-dependency".to_string())),
                    message: format!("Unused devDependency: {}", dep.package_name),
                    ..Default::default()
                });
            }
        }
        // Unlisted deps still use root package.json
        if let Some(ref uri) = package_json_uri {
            for dep in &results.unlisted_dependencies {
                let entry = diagnostics_by_file.entry(uri.clone()).or_default();
                entry.push(Diagnostic {
                    range: ZERO_RANGE,
                    severity: Some(DiagnosticSeverity::WARNING),
                    source: Some("fallow".to_string()),
                    code: Some(NumberOrString::String("unlisted-dependency".to_string())),
                    message: format!(
                        "Unlisted dependency: {} (used but not in package.json)",
                        dep.package_name
                    ),
                    ..Default::default()
                });
            }
        }

        // Member issues: unused enum members and unused class members
        for (members, code, kind_label) in [
            (
                &results.unused_enum_members,
                "unused-enum-member",
                "Enum member" as &str,
            ),
            (
                &results.unused_class_members,
                "unused-class-member",
                "Class member",
            ),
        ] {
            for member in members {
                if let Ok(uri) = Url::from_file_path(&member.path) {
                    let line = member.line.saturating_sub(1);
                    diagnostics_by_file
                        .entry(uri)
                        .or_default()
                        .push(Diagnostic {
                            range: Range {
                                start: Position {
                                    line,
                                    character: member.col,
                                },
                                end: Position {
                                    line,
                                    character: member.col + member.member_name.len() as u32,
                                },
                            },
                            severity: Some(DiagnosticSeverity::HINT),
                            source: Some("fallow".to_string()),
                            code: Some(NumberOrString::String(code.to_string())),
                            message: format!(
                                "{kind_label} '{}.{}' is unused",
                                member.parent_name, member.member_name
                            ),
                            tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                            ..Default::default()
                        });
                }
            }
        }

        // Duplicate exports: WARNING on each file that has the duplicate
        for dup in &results.duplicate_exports {
            for location in &dup.locations {
                if let Ok(uri) = Url::from_file_path(location) {
                    let other_files: Vec<String> = dup
                        .locations
                        .iter()
                        .filter(|l| *l != location)
                        .map(|l| l.display().to_string())
                        .collect();
                    diagnostics_by_file
                        .entry(uri)
                        .or_default()
                        .push(Diagnostic {
                            range: ZERO_RANGE,
                            severity: Some(DiagnosticSeverity::WARNING),
                            source: Some("fallow".to_string()),
                            code: Some(NumberOrString::String("duplicate-export".to_string())),
                            message: format!(
                                "Duplicate export '{}' (also in: {})",
                                dup.export_name,
                                other_files.join(", ")
                            ),
                            ..Default::default()
                        });
                }
            }
        }

        // Code duplication diagnostics
        for group in &duplication.clone_groups {
            for instance in &group.instances {
                let Ok(inst_uri) = Url::from_file_path(&instance.file) else {
                    continue;
                };

                let start_line = (instance.start_line as u32).saturating_sub(1);
                let end_line = (instance.end_line as u32).saturating_sub(1);

                // Build related information pointing to other instances in the group
                let related_info: Vec<DiagnosticRelatedInformation> = group
                    .instances
                    .iter()
                    .filter(|other| {
                        !(other.file == instance.file && other.start_line == instance.start_line)
                    })
                    .filter_map(|other| {
                        let other_uri = Url::from_file_path(&other.file).ok()?;
                        Some(DiagnosticRelatedInformation {
                            location: Location {
                                uri: other_uri,
                                range: Range {
                                    start: Position {
                                        line: (other.start_line as u32).saturating_sub(1),
                                        character: other.start_col as u32,
                                    },
                                    end: Position {
                                        line: (other.end_line as u32).saturating_sub(1),
                                        character: other.end_col as u32,
                                    },
                                },
                            },
                            message: "Also duplicated here".to_string(),
                        })
                    })
                    .collect();

                diagnostics_by_file
                    .entry(inst_uri)
                    .or_default()
                    .push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: start_line,
                                character: instance.start_col as u32,
                            },
                            end: Position {
                                line: end_line,
                                character: instance.end_col as u32,
                            },
                        },
                        severity: Some(DiagnosticSeverity::HINT),
                        source: Some("fallow".to_string()),
                        code: Some(NumberOrString::String("code-duplication".to_string())),
                        message: format!(
                            "Duplicated code block ({} lines, {} instances)",
                            group.line_count,
                            group.instances.len()
                        ),
                        related_information: if related_info.is_empty() {
                            None
                        } else {
                            Some(related_info)
                        },
                        ..Default::default()
                    });
            }
        }

        // Collect the set of URIs we are publishing to
        let new_uris: HashSet<Url> = diagnostics_by_file.keys().cloned().collect();

        // Publish diagnostics for current results
        for (uri, diagnostics) in &diagnostics_by_file {
            self.client
                .publish_diagnostics(uri.clone(), diagnostics.clone(), None)
                .await;
        }

        // Clear stale diagnostics: send empty arrays for URIs that had diagnostics
        // in the previous run but not in this one
        {
            let previous_uris = self.previous_diagnostic_uris.read().await;
            for old_uri in previous_uris.iter() {
                if !new_uris.contains(old_uri) {
                    self.client
                        .publish_diagnostics(old_uri.clone(), vec![], None)
                        .await;
                }
            }
        }

        // Update the tracked URIs for next run
        *self.previous_diagnostic_uris.write().await = new_uris;
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("fallow=info")
        .with_writer(std::io::stderr)
        .init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| FallowLspServer {
        client,
        root: Arc::new(RwLock::new(None)),
        results: Arc::new(RwLock::new(None)),
        duplication: Arc::new(RwLock::new(None)),
        previous_diagnostic_uris: Arc::new(RwLock::new(HashSet::new())),
        last_analysis: Arc::new(Mutex::new(
            Instant::now() - std::time::Duration::from_secs(10),
        )),
        analysis_guard: Arc::new(tokio::sync::Mutex::new(())),
        documents: Arc::new(RwLock::new(HashMap::new())),
    });

    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use fallow_core::duplicates::CloneInstance;

    fn instance(file: &str, start: usize, end: usize, fragment: &str) -> CloneInstance {
        CloneInstance {
            file: PathBuf::from(file),
            start_line: start,
            end_line: end,
            start_col: 0,
            end_col: 0,
            fragment: fragment.to_string(),
        }
    }

    fn group(instances: Vec<CloneInstance>, line_count: usize) -> CloneGroup {
        CloneGroup {
            instances,
            token_count: line_count * 10,
            line_count,
        }
    }

    /// Parse the edits from a code action result for a specific URI.
    fn extract_edits(action: &CodeActionOrCommand, uri: &Url) -> Vec<(u32, u32, String)> {
        match action {
            CodeActionOrCommand::CodeAction(ca) => {
                let ws = ca.edit.as_ref().unwrap();
                let edits = ws.changes.as_ref().unwrap().get(uri).unwrap();
                edits
                    .iter()
                    .map(|e| (e.range.start.line, e.range.end.line, e.new_text.clone()))
                    .collect()
            }
            _ => panic!("expected CodeAction"),
        }
    }

    #[test]
    fn no_actions_when_no_clone_groups() {
        let uri = Url::from_file_path("/tmp/test.ts").unwrap();
        let actions = build_extract_duplicate_actions(
            Path::new("/tmp/test.ts"),
            &uri,
            &Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 10,
                    character: 0,
                },
            },
            &[],
            &[],
        );
        assert!(actions.is_empty());
    }

    #[test]
    fn no_actions_when_cursor_outside_clone() {
        let uri = Url::from_file_path("/tmp/test.ts").unwrap();
        let groups = vec![group(
            vec![
                instance("/tmp/test.ts", 10, 20, "const x = 1;"),
                instance("/tmp/other.ts", 10, 20, "const x = 1;"),
            ],
            11,
        )];
        let file_lines: Vec<&str> = (0..30).map(|_| "some code").collect();

        // Cursor at line 0-5 (0-based), clone is at lines 9-19 (0-based)
        let actions = build_extract_duplicate_actions(
            Path::new("/tmp/test.ts"),
            &uri,
            &Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 5,
                    character: 0,
                },
            },
            &groups,
            &file_lines,
        );
        assert!(actions.is_empty());
    }

    #[test]
    fn generates_action_when_cursor_overlaps_clone() {
        let uri = Url::from_file_path("/tmp/test.ts").unwrap();
        let fragment = "const x = 1;\nconst y = 2;\nreturn x + y;";
        let groups = vec![group(
            vec![
                instance("/tmp/test.ts", 10, 12, fragment),
                instance("/tmp/other.ts", 5, 7, fragment),
            ],
            3,
        )];
        let file_lines: Vec<&str> = (0..30).map(|_| "  some code").collect();

        // Cursor at line 10 (0-based = 1-based line 11, inside clone at 1-based 10-12)
        let actions = build_extract_duplicate_actions(
            Path::new("/tmp/test.ts"),
            &uri,
            &Range {
                start: Position {
                    line: 10,
                    character: 0,
                },
                end: Position {
                    line: 10,
                    character: 0,
                },
            },
            &groups,
            &file_lines,
        );
        assert_eq!(actions.len(), 1);

        // Check title mentions cross-file
        match &actions[0] {
            CodeActionOrCommand::CodeAction(ca) => {
                assert_eq!(
                    ca.title,
                    "Extract duplicate into function (other files unchanged)"
                );
                assert_eq!(ca.kind, Some(CodeActionKind::REFACTOR_EXTRACT));
            }
            _ => panic!("expected CodeAction"),
        }
    }

    #[test]
    fn action_edits_are_correct_structure() {
        let uri = Url::from_file_path("/tmp/test.ts").unwrap();
        let fragment = "const x = 1;\nconst y = 2;";
        let groups = vec![group(
            vec![
                instance("/tmp/test.ts", 10, 11, fragment),
                instance("/tmp/other.ts", 5, 6, fragment),
            ],
            2,
        )];
        let file_lines: Vec<&str> = (0..30).map(|_| "code here").collect();

        let actions = build_extract_duplicate_actions(
            Path::new("/tmp/test.ts"),
            &uri,
            &Range {
                start: Position {
                    line: 9,
                    character: 0,
                },
                end: Position {
                    line: 11,
                    character: 0,
                },
            },
            &groups,
            &file_lines,
        );
        assert_eq!(actions.len(), 1);

        let edits = extract_edits(&actions[0], &uri);
        // Should have 2 edits: replace instance + insert function (sorted reverse)
        assert_eq!(edits.len(), 2);

        // Edits sorted in reverse order: replacement first (line 9), then insert (line 8)
        let (replace_start, replace_end, replace_text) = &edits[0];
        assert_eq!(*replace_start, 9); // 1-based 10 -> 0-based 9
        assert_eq!(*replace_end, 11); // end_line 11 (1-based) -> 10 (0-based) + 1 = 11
        assert!(replace_text.contains("extractedDuplicate();"));

        let (insert_start, insert_end, insert_text) = &edits[1];
        assert_eq!(*insert_start, 8); // 1 line before first instance (0-based 9 - 1 = 8)
        assert_eq!(*insert_end, 8); // Point insert (same line)
        assert!(insert_text.contains("function extractedDuplicate()"));
        assert!(insert_text.contains("const x = 1;"));
        assert!(insert_text.contains("const y = 2;"));
    }

    #[test]
    fn multiple_instances_same_file_get_replaced() {
        let uri = Url::from_file_path("/tmp/test.ts").unwrap();
        let fragment = "doStuff();";
        let groups = vec![group(
            vec![
                instance("/tmp/test.ts", 5, 5, fragment),
                instance("/tmp/test.ts", 15, 15, fragment),
            ],
            1,
        )];
        let file_lines: Vec<&str> = (0..30).map(|_| "line content").collect();

        let actions = build_extract_duplicate_actions(
            Path::new("/tmp/test.ts"),
            &uri,
            &Range {
                start: Position {
                    line: 4,
                    character: 0,
                },
                end: Position {
                    line: 4,
                    character: 0,
                },
            },
            &groups,
            &file_lines,
        );
        assert_eq!(actions.len(), 1);

        match &actions[0] {
            CodeActionOrCommand::CodeAction(ca) => {
                assert_eq!(
                    ca.title,
                    "Extract duplicate into function (2 instances in this file)"
                );
            }
            _ => panic!("expected CodeAction"),
        }

        let edits = extract_edits(&actions[0], &uri);
        // 3 edits: insert + 2 replacements (sorted reverse)
        assert_eq!(edits.len(), 3);

        // Verify reverse order: highest line first
        assert!(edits[0].0 >= edits[1].0);
        assert!(edits[1].0 >= edits[2].0);
    }

    #[test]
    fn clone_at_line_1_combines_insert_with_replacement() {
        let uri = Url::from_file_path("/tmp/test.ts").unwrap();
        let fragment = "const a = 1;";
        let groups = vec![group(
            vec![
                instance("/tmp/test.ts", 1, 1, fragment),
                instance("/tmp/other.ts", 1, 1, fragment),
            ],
            1,
        )];
        let file_lines = vec!["const a = 1;", "const b = 2;"];

        let actions = build_extract_duplicate_actions(
            Path::new("/tmp/test.ts"),
            &uri,
            &Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            },
            &groups,
            &file_lines,
        );
        assert_eq!(actions.len(), 1);

        let edits = extract_edits(&actions[0], &uri);
        // Only 1 edit (combined insert + replacement, since can't insert separately)
        assert_eq!(edits.len(), 1);

        let (start, _end, text) = &edits[0];
        assert_eq!(*start, 0);
        // The text should contain both the function definition and the call
        assert!(
            text.contains("function extractedDuplicate()"),
            "Should contain function def, got: {text}"
        );
        assert!(
            text.contains("extractedDuplicate();"),
            "Should contain function call, got: {text}"
        );
    }

    #[test]
    fn multiple_overlapping_groups_get_numbered_names() {
        let uri = Url::from_file_path("/tmp/test.ts").unwrap();
        let groups = vec![
            group(
                vec![
                    instance("/tmp/test.ts", 5, 8, "block1();"),
                    instance("/tmp/other.ts", 5, 8, "block1();"),
                ],
                4,
            ),
            group(
                vec![
                    instance("/tmp/test.ts", 6, 7, "block2();"),
                    instance("/tmp/other.ts", 10, 11, "block2();"),
                ],
                2,
            ),
        ];
        let file_lines: Vec<&str> = (0..30).map(|_| "code").collect();

        // Cursor overlaps both groups
        let actions = build_extract_duplicate_actions(
            Path::new("/tmp/test.ts"),
            &uri,
            &Range {
                start: Position {
                    line: 5,
                    character: 0,
                },
                end: Position {
                    line: 7,
                    character: 0,
                },
            },
            &groups,
            &file_lines,
        );
        assert_eq!(actions.len(), 2);

        // First action uses "extractedDuplicate", second uses "extractedDuplicate2"
        let edits1 = extract_edits(&actions[0], &uri);
        let edits2 = extract_edits(&actions[1], &uri);

        let has_first = edits1
            .iter()
            .any(|(_, _, t)| t.contains("function extractedDuplicate()"));
        let has_second = edits2
            .iter()
            .any(|(_, _, t)| t.contains("function extractedDuplicate2()"));

        assert!(has_first, "First action should use extractedDuplicate");
        assert!(has_second, "Second action should use extractedDuplicate2");
    }

    #[test]
    fn indentation_is_preserved_in_replacement() {
        let uri = Url::from_file_path("/tmp/test.ts").unwrap();
        let fragment = "return 42;";
        let groups = vec![group(
            vec![
                instance("/tmp/test.ts", 5, 5, fragment),
                instance("/tmp/other.ts", 5, 5, fragment),
            ],
            1,
        )];
        let file_lines = vec![
            "function a() {",
            "  if (true) {",
            "    return 1;",
            "  }",
            "    return 42;", // line 4 (0-based) = line 5 (1-based), indented with 4 spaces
            "}",
        ];

        let actions = build_extract_duplicate_actions(
            Path::new("/tmp/test.ts"),
            &uri,
            &Range {
                start: Position {
                    line: 4,
                    character: 0,
                },
                end: Position {
                    line: 4,
                    character: 0,
                },
            },
            &groups,
            &file_lines,
        );
        assert_eq!(actions.len(), 1);

        let edits = extract_edits(&actions[0], &uri);
        // Find the replacement edit (not the insert)
        let replacement = edits
            .iter()
            .find(|(s, e, _)| *s == 4 && *e > *s)
            .expect("should have replacement edit");
        assert_eq!(
            replacement.2, "    extractedDuplicate();\n",
            "Should preserve 4-space indent"
        );
    }

    #[test]
    fn end_to_end_duplication_detection_on_fixture() {
        use fallow_core::discover::{DiscoveredFile, FileId};
        use fallow_core::duplicates::{DuplicatesConfig, find_duplicates};

        let fixture_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/duplicate-code");

        if !fixture_dir.exists() {
            // Skip test if fixture doesn't exist
            return;
        }

        let src_dir = fixture_dir.join("src");
        let original = src_dir.join("original.ts");
        let copy1 = src_dir.join("copy1.ts");

        if !original.exists() || !copy1.exists() {
            return;
        }

        let original_content = std::fs::read_to_string(&original).unwrap();
        let copy1_content = std::fs::read_to_string(&copy1).unwrap();

        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: original.clone(),
                size_bytes: original_content.len() as u64,
            },
            DiscoveredFile {
                id: FileId(1),
                path: copy1.clone(),
                size_bytes: copy1_content.len() as u64,
            },
        ];

        let config = DuplicatesConfig {
            min_tokens: 10,
            min_lines: 2,
            ..DuplicatesConfig::default()
        };

        let report = find_duplicates(fixture_dir.as_path(), &files, &config);

        // Verify we get clone groups
        assert!(
            !report.clone_groups.is_empty(),
            "Should detect clones in duplicate-code fixture"
        );

        // Now test the code action builder with real data
        let file_lines: Vec<&str> = original_content.lines().collect();
        let uri = Url::from_file_path(&original).unwrap();

        // Use a range covering the whole file
        let actions = build_extract_duplicate_actions(
            &original,
            &uri,
            &Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: file_lines.len() as u32,
                    character: 0,
                },
            },
            &report.clone_groups,
            &file_lines,
        );

        // Should generate at least one code action
        assert!(
            !actions.is_empty(),
            "Should generate extract actions for duplicate-code fixture"
        );

        // Verify each action has proper structure
        for action in &actions {
            match action {
                CodeActionOrCommand::CodeAction(ca) => {
                    assert!(ca.title.starts_with("Extract duplicate into function"));
                    assert_eq!(ca.kind, Some(CodeActionKind::REFACTOR_EXTRACT));

                    // Has edits
                    let ws = ca.edit.as_ref().expect("should have workspace edit");
                    let changes = ws.changes.as_ref().expect("should have changes");
                    let file_edits = changes.get(&uri).expect("should have edits for file");
                    assert!(!file_edits.is_empty());

                    // Has associated diagnostic
                    let diags = ca.diagnostics.as_ref().expect("should have diagnostics");
                    assert_eq!(diags.len(), 1);
                    assert_eq!(
                        diags[0].code,
                        Some(NumberOrString::String("code-duplication".to_string()))
                    );

                    // Verify edits don't have overlapping ranges
                    for (i, edit_a) in file_edits.iter().enumerate() {
                        for (j, edit_b) in file_edits.iter().enumerate() {
                            if i == j {
                                continue;
                            }
                            let a_start = edit_a.range.start.line;
                            let a_end = edit_a.range.end.line;
                            let b_start = edit_b.range.start.line;
                            let b_end = edit_b.range.end.line;

                            // Point inserts at same position are not counted as overlap
                            if a_start == a_end && b_start == b_end && a_start == b_start {
                                continue;
                            }

                            let overlaps = a_start < b_end && b_start < a_end;
                            assert!(
                                !overlaps,
                                "Edits should not overlap: [{a_start}-{a_end}] vs [{b_start}-{b_end}]"
                            );
                        }
                    }

                    // Verify edits are sorted in reverse order
                    for window in file_edits.windows(2) {
                        assert!(
                            window[0].range.start.line >= window[1].range.start.line,
                            "Edits should be in reverse order: {} should >= {}",
                            window[0].range.start.line,
                            window[1].range.start.line
                        );
                    }
                }
                _ => panic!("expected CodeAction"),
            }
        }

        // Simulate applying the first action: verify the resulting text makes sense
        if let CodeActionOrCommand::CodeAction(ca) = &actions[0] {
            let edits = ca
                .edit
                .as_ref()
                .unwrap()
                .changes
                .as_ref()
                .unwrap()
                .get(&uri)
                .unwrap();

            // At least one edit should contain the function definition
            let has_function_def = edits
                .iter()
                .any(|e| e.new_text.contains("function extractedDuplicate()"));
            assert!(
                has_function_def,
                "One edit should contain the extracted function definition"
            );

            // At least one edit should contain the function call
            let has_call = edits
                .iter()
                .any(|e| e.new_text.contains("extractedDuplicate();"));
            assert!(has_call, "One edit should contain the function call");
        }
    }

    /// Apply LSP text edits (in reverse order) to source text and return the result.
    fn apply_edits(source: &str, edits: &[TextEdit]) -> String {
        let lines: Vec<&str> = source.lines().collect();

        // Build a list of (start_line, end_line, new_text) sorted in reverse
        let mut sorted_edits: Vec<_> = edits.iter().collect();
        sorted_edits.sort_by(|a, b| {
            b.range
                .start
                .line
                .cmp(&a.range.start.line)
                .then(b.range.start.character.cmp(&a.range.start.character))
        });

        let mut result_lines: Vec<String> = lines.iter().map(|l| format!("{l}\n")).collect();

        for edit in sorted_edits {
            let start = edit.range.start.line as usize;
            let end = edit.range.end.line as usize;

            // Replace lines [start, end) with new_text
            let end_clamped = end.min(result_lines.len());
            let start_clamped = start.min(result_lines.len());

            let new_lines: Vec<String> = if edit.new_text.is_empty() {
                vec![]
            } else {
                vec![edit.new_text.clone()]
            };

            result_lines.splice(start_clamped..end_clamped, new_lines);
        }

        result_lines.join("")
    }

    #[test]
    fn apply_extract_action_produces_valid_output() {
        let uri = Url::from_file_path("/tmp/test.ts").unwrap();
        let source = "\
function a() {
    const x = 1;
    const y = 2;
    return x + y;
}

function b() {
    const x = 1;
    const y = 2;
    return x + y;
}
";
        let fragment = "    const x = 1;\n    const y = 2;\n    return x + y;";
        let groups = vec![group(
            vec![
                instance("/tmp/test.ts", 2, 4, fragment),
                instance("/tmp/test.ts", 8, 10, fragment),
            ],
            3,
        )];
        let file_lines: Vec<&str> = source.lines().collect();

        let actions = build_extract_duplicate_actions(
            Path::new("/tmp/test.ts"),
            &uri,
            &Range {
                start: Position {
                    line: 1,
                    character: 0,
                },
                end: Position {
                    line: 4,
                    character: 0,
                },
            },
            &groups,
            &file_lines,
        );
        assert_eq!(actions.len(), 1);

        // Apply the edits to the source
        if let CodeActionOrCommand::CodeAction(ca) = &actions[0] {
            let edits = ca
                .edit
                .as_ref()
                .unwrap()
                .changes
                .as_ref()
                .unwrap()
                .get(&uri)
                .unwrap();

            let result = apply_edits(source, edits);

            // The result should contain the extracted function
            assert!(
                result.contains("function extractedDuplicate()"),
                "Result should contain function def:\n{result}"
            );

            // Both original instances should be replaced with calls
            let call_count = result.matches("extractedDuplicate();").count();
            assert_eq!(
                call_count, 2,
                "Should have 2 function calls, got {call_count}:\n{result}"
            );

            // The original duplicate code should no longer appear (except inside the function)
            let x_count = result.matches("const x = 1;").count();
            assert_eq!(
                x_count, 1,
                "Should have exactly 1 copy of the code (inside the function), got {x_count}:\n{result}"
            );

            // The function wrappers should still exist
            assert!(
                result.contains("function a()"),
                "Original function a should remain:\n{result}"
            );
            assert!(
                result.contains("function b()"),
                "Original function b should remain:\n{result}"
            );

            // Print result for visual inspection
            eprintln!("=== Applied edit result ===\n{result}=== End ===");
        }
    }

    #[test]
    fn apply_extract_action_on_real_fixture() {
        use fallow_core::discover::{DiscoveredFile, FileId};
        use fallow_core::duplicates::{DuplicatesConfig, find_duplicates};

        let fixture_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/duplicate-code");

        if !fixture_dir.exists() {
            return;
        }

        let original = fixture_dir.join("src/original.ts");
        let copy1 = fixture_dir.join("src/copy1.ts");

        if !original.exists() || !copy1.exists() {
            return;
        }

        let original_content = std::fs::read_to_string(&original).unwrap();
        let copy1_content = std::fs::read_to_string(&copy1).unwrap();

        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: original.clone(),
                size_bytes: original_content.len() as u64,
            },
            DiscoveredFile {
                id: FileId(1),
                path: copy1.clone(),
                size_bytes: copy1_content.len() as u64,
            },
        ];

        let config = DuplicatesConfig {
            min_tokens: 10,
            min_lines: 2,
            ..DuplicatesConfig::default()
        };

        let report = find_duplicates(fixture_dir.as_path(), &files, &config);
        assert!(!report.clone_groups.is_empty());

        let file_lines: Vec<&str> = original_content.lines().collect();
        let uri = Url::from_file_path(&original).unwrap();

        let actions = build_extract_duplicate_actions(
            &original,
            &uri,
            &Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: file_lines.len() as u32,
                    character: 0,
                },
            },
            &report.clone_groups,
            &file_lines,
        );

        assert!(!actions.is_empty(), "Should have at least one action");

        // Apply the first action and verify the result
        if let CodeActionOrCommand::CodeAction(ca) = &actions[0] {
            let edits = ca
                .edit
                .as_ref()
                .unwrap()
                .changes
                .as_ref()
                .unwrap()
                .get(&uri)
                .unwrap();

            let result = apply_edits(&original_content, edits);

            // Should contain the extracted function
            assert!(
                result.contains("function extractedDuplicate()"),
                "Should contain extracted function:\n{result}"
            );

            // Should contain a call to it
            assert!(
                result.contains("extractedDuplicate();"),
                "Should contain function call:\n{result}"
            );

            // Print for visual inspection
            eprintln!(
                "=== Real fixture: applied edit ===\n{result}=== End ({} chars) ===",
                result.len()
            );
        }
    }

    #[test]
    fn realistic_partial_duplicate_scenario() {
        let uri = Url::from_file_path("/tmp/utils.ts").unwrap();
        let source = "\
import { db } from './db';

export function fetchUsers() {
  const users = db.query('SELECT * FROM users');
  const filtered = users.filter(u => u.active);
  return filtered.map(u => ({ id: u.id, name: u.name }));
}

export function fetchOrders() {
  const orders = db.query('SELECT * FROM orders');
  return orders;
}

export function fetchProducts() {
  const products = db.query('SELECT * FROM products');
  const filtered = products.filter(p => p.active);
  return filtered.map(p => ({ id: p.id, name: p.name }));
}
";
        // The duplication detector found lines 5-6 and 16-17 as duplicates
        // (the filter+map pattern with different variable names, in semantic mode)
        let fragment_a = "  const filtered = users.filter(u => u.active);\n  return filtered.map(u => ({ id: u.id, name: u.name }));";
        let fragment_b = "  const filtered = products.filter(p => p.active);\n  return filtered.map(p => ({ id: p.id, name: p.name }));";

        let groups = vec![group(
            vec![
                instance("/tmp/utils.ts", 5, 6, fragment_a),
                instance("/tmp/utils.ts", 16, 17, fragment_b),
            ],
            2,
        )];

        let file_lines: Vec<&str> = source.lines().collect();

        // Cursor on line 5 (0-based = 1-based line 6, inside first duplicate)
        let actions = build_extract_duplicate_actions(
            Path::new("/tmp/utils.ts"),
            &uri,
            &Range {
                start: Position {
                    line: 4,
                    character: 0,
                },
                end: Position {
                    line: 5,
                    character: 0,
                },
            },
            &groups,
            &file_lines,
        );
        assert_eq!(actions.len(), 1);

        match &actions[0] {
            CodeActionOrCommand::CodeAction(ca) => {
                assert_eq!(
                    ca.title,
                    "Extract duplicate into function (2 instances in this file)"
                );
            }
            _ => panic!("expected CodeAction"),
        }

        if let CodeActionOrCommand::CodeAction(ca) = &actions[0] {
            let edits = ca
                .edit
                .as_ref()
                .unwrap()
                .changes
                .as_ref()
                .unwrap()
                .get(&uri)
                .unwrap();

            let result = apply_edits(source, edits);
            eprintln!("=== Realistic scenario ===\n{result}=== End ===");

            // The import and fetchOrders should be untouched
            assert!(
                result.contains("import { db } from './db';"),
                "Import should be preserved:\n{result}"
            );
            assert!(
                result.contains("export function fetchOrders()"),
                "fetchOrders should be preserved:\n{result}"
            );

            // The extracted function should exist
            assert!(
                result.contains("function extractedDuplicate() {"),
                "Extracted function should exist:\n{result}"
            );

            // Both instances should be replaced
            let call_count = result.matches("extractedDuplicate();").count();
            assert_eq!(
                call_count, 2,
                "Should have 2 calls to extractedDuplicate:\n{result}"
            );

            // fetchUsers and fetchProducts should still have their opening lines
            assert!(
                result.contains("export function fetchUsers()"),
                "fetchUsers should still exist:\n{result}"
            );
            assert!(
                result.contains("export function fetchProducts()"),
                "fetchProducts should still exist:\n{result}"
            );

            // The body of the extracted function should be dedented
            assert!(
                result.contains("  const filtered ="),
                "Function body should have 2-space indent:\n{result}"
            );
            assert!(
                !result.contains("    const filtered ="),
                "Function body should NOT have 4-space indent:\n{result}"
            );
        }
    }
}
