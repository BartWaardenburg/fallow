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
        if let Some(root_uri) = params.root_uri {
            if let Ok(path) = root_uri.to_file_path() {
                *self.root.write().await = Some(path);
            }
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
        // Debounced re-analysis could go here
    }
}

impl FallowLspServer {
    async fn run_analysis(&self) {
        let root = self.root.read().await.clone();
        let Some(root) = root else { return };

        let results = tokio::task::spawn_blocking(move || {
            fallow_core::analyze_project(&root)
        })
        .await;

        match results {
            Ok(results) => {
                self.publish_diagnostics(&results, &self.root.read().await.clone().unwrap())
                    .await;
                *self.results.write().await = Some(results);
            }
            Err(e) => {
                self.client
                    .log_message(MessageType::ERROR, format!("Analysis failed: {e}"))
                    .await;
            }
        }
    }

    async fn publish_diagnostics(&self, results: &AnalysisResults, root: &PathBuf) {
        // Collect diagnostics per file
        let mut diagnostics_by_file: std::collections::HashMap<Url, Vec<Diagnostic>> =
            std::collections::HashMap::new();

        for export in &results.unused_exports {
            if let Ok(uri) = Url::from_file_path(&export.path) {
                let diag = Diagnostic {
                    range: Range {
                        start: Position {
                            line: export.line.saturating_sub(1),
                            character: export.col,
                        },
                        end: Position {
                            line: export.line.saturating_sub(1),
                            character: export.col + export.export_name.len() as u32,
                        },
                    },
                    severity: Some(DiagnosticSeverity::HINT),
                    source: Some("fallow".to_string()),
                    message: format!("Export '{}' is unused", export.export_name),
                    tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                    ..Default::default()
                };
                diagnostics_by_file.entry(uri).or_default().push(diag);
            }
        }

        for export in &results.unused_types {
            if let Ok(uri) = Url::from_file_path(&export.path) {
                let diag = Diagnostic {
                    range: Range {
                        start: Position {
                            line: export.line.saturating_sub(1),
                            character: 0,
                        },
                        end: Position {
                            line: export.line.saturating_sub(1),
                            character: 0,
                        },
                    },
                    severity: Some(DiagnosticSeverity::HINT),
                    source: Some("fallow".to_string()),
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
                    message: "File is not reachable from any entry point".to_string(),
                    tags: Some(vec![DiagnosticTag::UNNECESSARY]),
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

        let _ = root; // suppress warning
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
