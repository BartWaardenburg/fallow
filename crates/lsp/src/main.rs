use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{Mutex, RwLock};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

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
    previous_diagnostic_uris: Arc<RwLock<HashSet<Url>>>,
    last_analysis: Arc<Mutex<Instant>>,
    analysis_guard: Arc<tokio::sync::Mutex<()>>,
    documents: Arc<RwLock<HashMap<Url, String>>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for FallowLspServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(root_uri) = params.root_uri
            && let Ok(path) = root_uri.to_file_path()
        {
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
                        code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
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
            let mut last = self.last_analysis.lock().await;
            if now.duration_since(*last) < std::time::Duration::from_millis(500) {
                return;
            }
            *last = now;
        }

        // Re-run analysis on save
        self.run_analysis().await;
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

                CodeLens {
                    range: Range {
                        start: Position {
                            line,
                            character: usage.col,
                        },
                        end: Position {
                            line,
                            character: usage.col,
                        },
                    },
                    command: Some(Command {
                        title,
                        command: "fallow.noop".to_string(),
                        arguments: None,
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
        let join_result =
            tokio::task::spawn_blocking(move || fallow_core::analyze_project(&root_clone)).await;

        match join_result {
            Ok(Ok(results)) => {
                self.publish_diagnostics(&results, &root).await;
                *self.results.write().await = Some(results);

                // Notify the client to re-request Code Lenses with the fresh data
                let _ = self.client.code_lens_refresh().await;

                self.client
                    .log_message(MessageType::INFO, "Analysis complete")
                    .await;
            }
            Ok(Err(e)) => {
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

    async fn publish_diagnostics(&self, results: &AnalysisResults, root: &Path) {
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
        previous_diagnostic_uris: Arc::new(RwLock::new(HashSet::new())),
        last_analysis: Arc::new(Mutex::new(
            Instant::now() - std::time::Duration::from_secs(10),
        )),
        analysis_guard: Arc::new(tokio::sync::Mutex::new(())),
        documents: Arc::new(RwLock::new(HashMap::new())),
    });

    Server::new(stdin, stdout, socket).serve(service).await;
}
