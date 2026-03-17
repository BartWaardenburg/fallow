use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use fallow_core::results::AnalysisResults;

struct FallowLspServer {
    client: Client,
    root: Arc<RwLock<Option<PathBuf>>>,
    results: Arc<RwLock<Option<AnalysisResults>>>,
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
        // Re-run analysis on save
        self.run_analysis().await;
    }

    async fn did_change(&self, _params: DidChangeTextDocumentParams) {
        // Re-analysis is triggered on save, not on every change
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

        // Read file content once for computing line positions and edit ranges
        let file_content = std::fs::read_to_string(&file_path).unwrap_or_default();
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
            let mut changes = std::collections::HashMap::new();

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

        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
    }
}

impl FallowLspServer {
    async fn run_analysis(&self) {
        let root = self.root.read().await.clone();
        let Some(root) = root else { return };

        self.client
            .log_message(MessageType::INFO, "Running fallow analysis...")
            .await;

        let join_result =
            tokio::task::spawn_blocking(move || fallow_core::analyze_project(&root)).await;

        match join_result {
            Ok(Ok(results)) => {
                let root_path = self.root.read().await.clone().unwrap();
                self.publish_diagnostics(&results, &root_path).await;
                *self.results.write().await = Some(results);

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

    async fn publish_diagnostics(&self, results: &AnalysisResults, _root: &PathBuf) {
        // Collect diagnostics per file
        let mut diagnostics_by_file: std::collections::HashMap<Url, Vec<Diagnostic>> =
            std::collections::HashMap::new();

        for export in &results.unused_exports {
            if let Ok(uri) = Url::from_file_path(&export.path) {
                // export.line is 1-based; LSP uses 0-based
                let line = export.line.saturating_sub(1);
                let diag = Diagnostic {
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
                    code: Some(NumberOrString::String("unused-export".to_string())),
                    message: format!("Export '{}' is unused", export.export_name),
                    tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                    ..Default::default()
                };
                diagnostics_by_file.entry(uri).or_default().push(diag);
            }
        }

        for export in &results.unused_types {
            if let Ok(uri) = Url::from_file_path(&export.path) {
                // export.line is 1-based; LSP uses 0-based
                let line = export.line.saturating_sub(1);
                let diag = Diagnostic {
                    range: Range {
                        start: Position {
                            line,
                            character: export.col,
                        },
                        end: Position {
                            line,
                            character: export.col,
                        },
                    },
                    severity: Some(DiagnosticSeverity::HINT),
                    source: Some("fallow".to_string()),
                    code: Some(NumberOrString::String("unused-type".to_string())),
                    message: format!("Type export '{}' is unused", export.export_name),
                    tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                    ..Default::default()
                };
                diagnostics_by_file.entry(uri).or_default().push(diag);
            }
        }

        for file in &results.unused_files {
            if let Ok(uri) = Url::from_file_path(&file.path) {
                let diag = Diagnostic {
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: 0,
                            character: 0,
                        },
                    },
                    severity: Some(DiagnosticSeverity::WARNING),
                    source: Some("fallow".to_string()),
                    code: Some(NumberOrString::String("unused-file".to_string())),
                    message: "File is not reachable from any entry point".to_string(),
                    tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                    ..Default::default()
                };
                diagnostics_by_file.entry(uri).or_default().push(diag);
            }
        }

        for import in &results.unresolved_imports {
            if let Ok(uri) = Url::from_file_path(&import.path) {
                // import.line is 1-based; LSP uses 0-based
                let line = import.line.saturating_sub(1);
                let diag = Diagnostic {
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
                };
                diagnostics_by_file.entry(uri).or_default().push(diag);
            }
        }

        // Publish
        for (uri, diagnostics) in diagnostics_by_file {
            self.client
                .publish_diagnostics(uri, diagnostics, None)
                .await;
        }
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
    });

    Server::new(stdin, stdout, socket).serve(service).await;
}
