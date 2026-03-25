mod code_actions;
mod code_lens;
mod diagnostics;
mod hover;

use rustc_hash::{FxHashMap, FxHashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{Mutex, RwLock};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use serde::{Deserialize, Serialize};

use fallow_core::duplicates::DuplicationReport;
use fallow_core::results::AnalysisResults;

// ── Custom LSP notification: fallow/analysisComplete ──────────────────────

/// Custom notification sent to the client after every analysis completes.
/// Carries summary stats so the extension can update the status bar, context
/// keys, and other UI without running a separate CLI process.
enum AnalysisComplete {}

impl notification::Notification for AnalysisComplete {
    type Params = AnalysisCompleteParams;
    const METHOD: &'static str = "fallow/analysisComplete";
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisCompleteParams {
    total_issues: usize,
    unused_files: usize,
    unused_exports: usize,
    unused_types: usize,
    unused_dependencies: usize,
    unused_dev_dependencies: usize,
    unused_enum_members: usize,
    unused_class_members: usize,
    unresolved_imports: usize,
    unlisted_dependencies: usize,
    duplicate_exports: usize,
    type_only_dependencies: usize,
    circular_dependencies: usize,
    duplication_percentage: f64,
    clone_groups: usize,
}

/// Diagnostic codes that the LSP client can disable via initializationOptions.
/// Maps config key (e.g. "unused-files") to diagnostic code (e.g. "unused-file").
const ISSUE_TYPE_TO_DIAGNOSTIC_CODE: &[(&str, &str)] = &[
    ("unused-files", "unused-file"),
    ("unused-exports", "unused-export"),
    ("unused-types", "unused-type"),
    ("unused-dependencies", "unused-dependency"),
    ("unused-dev-dependencies", "unused-dev-dependency"),
    ("unused-enum-members", "unused-enum-member"),
    ("unused-class-members", "unused-class-member"),
    ("unresolved-imports", "unresolved-import"),
    ("unlisted-dependencies", "unlisted-dependency"),
    ("duplicate-exports", "duplicate-export"),
    ("type-only-dependencies", "type-only-dependency"),
    ("circular-dependencies", "circular-dependency"),
];

struct FallowLspServer {
    client: Client,
    root: Arc<RwLock<Option<PathBuf>>>,
    results: Arc<RwLock<Option<AnalysisResults>>>,
    duplication: Arc<RwLock<Option<DuplicationReport>>>,
    previous_diagnostic_uris: Arc<RwLock<FxHashSet<Url>>>,
    last_analysis: Arc<Mutex<Instant>>,
    analysis_guard: Arc<tokio::sync::Mutex<()>>,
    documents: Arc<RwLock<FxHashMap<Url, String>>>,
    /// Diagnostic codes to suppress (parsed from initializationOptions.issueTypes)
    disabled_diagnostic_codes: Arc<RwLock<FxHashSet<String>>>,
    /// Cached diagnostics for pull-model support (textDocument/diagnostic)
    cached_diagnostics: Arc<RwLock<FxHashMap<Url, Vec<Diagnostic>>>>,
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

        // Parse initializationOptions for issue type toggles
        if let Some(opts) = &params.initialization_options
            && let Some(issue_types) = opts.get("issueTypes").and_then(|v| v.as_object())
        {
            let mut disabled = FxHashSet::default();
            for &(config_key, diag_code) in ISSUE_TYPE_TO_DIAGNOSTIC_CODE {
                if let Some(enabled) = issue_types.get(config_key).and_then(|v| v.as_bool())
                    && !enabled
                {
                    disabled.insert(diag_code.to_string());
                }
            }
            // "code-duplication" is controlled by the duplication.* settings,
            // not issueTypes — always enabled at the LSP level
            *self.disabled_diagnostic_codes.write().await = disabled;
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
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
                hover_provider: Some(HoverProviderCapability::Simple(true)),
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
        {
            let now = Instant::now();
            let mut last = self.last_analysis.lock().await;
            if now.duration_since(*last) < std::time::Duration::from_millis(500) {
                return;
            }
            // Update timestamp under the lock to prevent TOCTOU races
            // where multiple saves pass the debounce check simultaneously
            *last = now;
        }

        // Re-run analysis on save
        self.run_analysis().await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.documents
            .write()
            .await
            .insert(params.text_document.uri, params.text_document.text);
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

    #[expect(clippy::significant_drop_tightening)]
    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let results = self.results.read().await;
        let Some(results) = results.as_ref() else {
            return Ok(None);
        };

        let uri = &params.text_document.uri;
        let Ok(file_path) = uri.to_file_path() else {
            return Ok(None);
        };

        let mut actions = Vec::new();

        // Read file content once for computing line positions and edit ranges.
        // Prefer in-memory document text (from did_open/did_change), fall back to disk.
        let documents = self.documents.read().await;
        let file_content = documents
            .get(uri)
            .cloned()
            .unwrap_or_else(|| std::fs::read_to_string(&file_path).unwrap_or_default());
        drop(documents);
        let file_lines: Vec<&str> = file_content.lines().collect();

        // Generate "Remove export" code actions for unused exports
        actions.extend(code_actions::build_remove_export_actions(
            results,
            &file_path,
            uri,
            &params.range,
            &file_lines,
        ));

        // Generate "Delete this file" code actions for unused files
        actions.extend(code_actions::build_delete_file_actions(
            results,
            &file_path,
            uri,
            &params.range,
        ));

        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
    }

    #[expect(clippy::significant_drop_tightening)]
    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let results = self.results.read().await;
        let Some(results) = results.as_ref() else {
            return Ok(None);
        };

        let Ok(file_path) = params.text_document.uri.to_file_path() else {
            return Ok(None);
        };

        let lenses = code_lens::build_code_lenses(results, &file_path, &params.text_document.uri);

        if lenses.is_empty() {
            Ok(None)
        } else {
            Ok(Some(lenses))
        }
    }

    #[expect(clippy::significant_drop_tightening)]
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let results = self.results.read().await;
        let Some(results) = results.as_ref() else {
            return Ok(None);
        };

        let uri = &params.text_document_position_params.text_document.uri;
        let Ok(file_path) = uri.to_file_path() else {
            return Ok(None);
        };

        let position = params.text_document_position_params.position;

        let duplication = self.duplication.read().await;
        let empty_report = fallow_core::duplicates::DuplicationReport::default();
        let duplication_ref = duplication.as_ref().unwrap_or(&empty_report);

        Ok(hover::build_hover(
            results,
            duplication_ref,
            &file_path,
            position,
        ))
    }
}

impl FallowLspServer {
    /// Pull-model diagnostic handler (textDocument/diagnostic, LSP 3.17).
    /// Returns cached diagnostics for the requested document.
    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> Result<DocumentDiagnosticReportResult> {
        let uri = params.text_document.uri;
        let items = self
            .cached_diagnostics
            .read()
            .await
            .get(&uri)
            .cloned()
            .unwrap_or_default();
        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items,
                },
            }),
        ))
    }
    async fn run_analysis(&self) {
        let root = self.root.read().await.clone();
        let Some(root) = root else { return };

        let Ok(_guard) = self.analysis_guard.try_lock() else {
            return; // analysis already running
        };

        self.client
            .log_message(MessageType::INFO, "Running fallow analysis...")
            .await;

        // Discover all project roots: the workspace root itself, plus any
        // subdirectories with their own package.json (sub-projects, fixtures, etc.)
        let project_roots = find_project_roots(&root);

        self.client
            .log_message(
                MessageType::INFO,
                format!("Found {} project root(s)", project_roots.len()),
            )
            .await;

        let join_result = tokio::task::spawn_blocking(move || {
            let mut merged_results = AnalysisResults::default();
            let mut merged_duplication = DuplicationReport::default();
            let mut analysis_roots: Vec<std::path::PathBuf> = Vec::new();

            for project_root in &project_roots {
                match fallow_core::analyze_project(project_root) {
                    Ok(results) => {
                        merge_results(&mut merged_results, results);
                        analysis_roots.push(project_root.clone());
                    }
                    Err(_) => {
                        // Skip projects that fail to analyze (e.g., no source files)
                    }
                }

                let dupes_config = fallow_config::FallowConfig::find_and_load(project_root)
                    .ok()
                    .flatten()
                    .map(|(c, _)| c.duplicates)
                    .unwrap_or_default();

                let duplication = fallow_core::duplicates::find_duplicates_in_project(
                    project_root,
                    &dupes_config,
                );
                merge_duplication(&mut merged_duplication, duplication);
            }

            (merged_results, merged_duplication, analysis_roots)
        })
        .await;

        match join_result {
            Ok((results, duplication, roots)) => {
                // Publish diagnostics using all analysis roots
                for analysis_root in &roots {
                    self.publish_diagnostics(&results, &duplication, analysis_root)
                        .await;
                }

                // Send summary stats to the client before storing results
                self.client
                    .send_notification::<AnalysisComplete>(AnalysisCompleteParams {
                        total_issues: results.total_issues(),
                        unused_files: results.unused_files.len(),
                        unused_exports: results.unused_exports.len(),
                        unused_types: results.unused_types.len(),
                        unused_dependencies: results.unused_dependencies.len(),
                        unused_dev_dependencies: results.unused_dev_dependencies.len(),
                        unused_enum_members: results.unused_enum_members.len(),
                        unused_class_members: results.unused_class_members.len(),
                        unresolved_imports: results.unresolved_imports.len(),
                        unlisted_dependencies: results.unlisted_dependencies.len(),
                        duplicate_exports: results.duplicate_exports.len(),
                        type_only_dependencies: results.type_only_dependencies.len(),
                        circular_dependencies: results.circular_dependencies.len(),
                        duplication_percentage: duplication.stats.duplication_percentage,
                        clone_groups: duplication.stats.clone_groups,
                    })
                    .await;

                *self.results.write().await = Some(results);
                *self.duplication.write().await = Some(duplication);

                let _ = self.client.code_lens_refresh().await;

                self.client
                    .log_message(MessageType::INFO, "Analysis complete")
                    .await;
            }
            Err(e) => {
                self.client
                    .log_message(MessageType::ERROR, format!("Analysis failed: {e}"))
                    .await;
            }
        }
    }

    #[expect(clippy::significant_drop_tightening)]
    async fn publish_diagnostics(
        &self,
        results: &AnalysisResults,
        duplication: &DuplicationReport,
        root: &std::path::Path,
    ) {
        let diagnostics_by_file = diagnostics::build_diagnostics(results, duplication, root);
        let disabled = self.disabled_diagnostic_codes.read().await;

        // Collect the set of URIs we are publishing to
        let mut new_uris: FxHashSet<Url> = FxHashSet::default();

        // Publish diagnostics for current results, filtering out disabled issue types
        for (uri, diags) in &diagnostics_by_file {
            let filtered: Vec<Diagnostic> = if disabled.is_empty() {
                diags.clone()
            } else {
                diags
                    .iter()
                    .filter(|d| {
                        d.code.as_ref().is_none_or(|code| match code {
                            NumberOrString::String(s) => !disabled.contains(s.as_str()),
                            NumberOrString::Number(_) => true,
                        })
                    })
                    .cloned()
                    .collect()
            };

            // Track all URIs we publish to (even empty), so stale-clearing
            // only fires for URIs that truly disappeared from results
            new_uris.insert(uri.clone());
            self.client
                .publish_diagnostics(uri.clone(), filtered.clone(), None)
                .await;

            // Cache for pull-model requests (textDocument/diagnostic)
            self.cached_diagnostics
                .write()
                .await
                .insert(uri.clone(), filtered);
        }

        // Clear stale diagnostics: send empty arrays for URIs that had diagnostics
        // in the previous run but not in this one
        {
            let previous_uris = self.previous_diagnostic_uris.read().await;
            let mut cache = self.cached_diagnostics.write().await;
            for old_uri in previous_uris.iter() {
                if !new_uris.contains(old_uri) {
                    self.client
                        .publish_diagnostics(old_uri.clone(), vec![], None)
                        .await;
                    cache.remove(old_uri);
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

    let (service, socket) = LspService::build(|client| FallowLspServer {
        client,
        root: Arc::new(RwLock::new(None)),
        results: Arc::new(RwLock::new(None)),
        duplication: Arc::new(RwLock::new(None)),
        previous_diagnostic_uris: Arc::new(RwLock::new(FxHashSet::default())),
        last_analysis: Arc::new(Mutex::new(
            Instant::now()
                .checked_sub(std::time::Duration::from_secs(10))
                .unwrap_or_else(Instant::now),
        )),
        analysis_guard: Arc::new(tokio::sync::Mutex::new(())),
        documents: Arc::new(RwLock::new(FxHashMap::default())),
        disabled_diagnostic_codes: Arc::new(RwLock::new(FxHashSet::default())),
        cached_diagnostics: Arc::new(RwLock::new(FxHashMap::default())),
    })
    .custom_method("textDocument/diagnostic", FallowLspServer::diagnostic)
    .finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}

/// Find all project roots under a workspace directory.
/// Find all project roots under a workspace directory.
///
/// Uses the workspace root plus any configured monorepo workspaces
/// (package.json `workspaces`, pnpm-workspace.yaml, tsconfig references).
fn find_project_roots(workspace_root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut roots = vec![workspace_root.to_path_buf()];

    let workspaces = fallow_config::discover_workspaces(workspace_root);
    for ws in &workspaces {
        roots.push(ws.root.clone());
    }

    roots.sort();
    roots.dedup();
    roots
}

/// Merge analysis results from a sub-project into the accumulated results.
fn merge_results(target: &mut AnalysisResults, source: AnalysisResults) {
    target.unused_files.extend(source.unused_files);
    target.unused_exports.extend(source.unused_exports);
    target.unused_types.extend(source.unused_types);
    target
        .unused_dependencies
        .extend(source.unused_dependencies);
    target
        .unused_dev_dependencies
        .extend(source.unused_dev_dependencies);
    target
        .unused_optional_dependencies
        .extend(source.unused_optional_dependencies);
    target
        .unused_enum_members
        .extend(source.unused_enum_members);
    target
        .unused_class_members
        .extend(source.unused_class_members);
    target.unresolved_imports.extend(source.unresolved_imports);
    target
        .unlisted_dependencies
        .extend(source.unlisted_dependencies);
    target.duplicate_exports.extend(source.duplicate_exports);
    target
        .type_only_dependencies
        .extend(source.type_only_dependencies);
    target
        .circular_dependencies
        .extend(source.circular_dependencies);
}

/// Merge duplication reports from a sub-project into the accumulated report.
fn merge_duplication(target: &mut DuplicationReport, source: DuplicationReport) {
    target.clone_groups.extend(source.clone_groups);
    target.clone_families.extend(source.clone_families);
    target.stats.clone_groups += source.stats.clone_groups;
    target.stats.clone_instances += source.stats.clone_instances;
    target.stats.total_files += source.stats.total_files;
    target.stats.files_with_clones += source.stats.files_with_clones;
    target.stats.total_lines += source.stats.total_lines;
    target.stats.duplicated_lines += source.stats.duplicated_lines;
    target.stats.total_tokens += source.stats.total_tokens;
    target.stats.duplicated_tokens += source.stats.duplicated_tokens;
    // Recompute percentage from merged totals (don't sum sub-project percentages)
    target.stats.duplication_percentage = if target.stats.total_lines > 0 {
        (target.stats.duplicated_lines as f64 / target.stats.total_lines as f64) * 100.0
    } else {
        0.0
    };
}
