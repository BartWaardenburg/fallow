use std::process::Stdio;

use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::{ErrorData as McpError, ServerHandler, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::process::Command;

// ── Parameter types ────────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct AnalyzeParams {
    /// Root directory of the project to analyze. Defaults to current working directory.
    pub root: Option<String>,

    /// Path to fallow config file (.fallowrc.json or fallow.toml).
    pub config: Option<String>,

    /// Only analyze production code (excludes tests, stories, dev files).
    pub production: Option<bool>,

    /// Scope analysis to a specific workspace package name.
    pub workspace: Option<String>,

    /// Issue types to include. When set, only these types are reported.
    /// Valid values: unused-files, unused-exports, unused-types, unused-deps,
    /// unused-enum-members, unused-class-members, unresolved-imports,
    /// unlisted-deps, duplicate-exports, circular-deps.
    pub issue_types: Option<Vec<String>>,
}

#[derive(Deserialize, JsonSchema)]
pub struct CheckChangedParams {
    /// Root directory of the project to analyze. Defaults to current working directory.
    pub root: Option<String>,

    /// Git ref to compare against (e.g., "main", "HEAD~5", a commit SHA).
    /// Only files changed since this ref are reported.
    pub since: String,

    /// Path to fallow config file.
    pub config: Option<String>,

    /// Only analyze production code.
    pub production: Option<bool>,

    /// Scope analysis to a specific workspace package name.
    pub workspace: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct FindDupesParams {
    /// Root directory of the project to analyze. Defaults to current working directory.
    pub root: Option<String>,

    /// Detection mode: "strict" (exact tokens), "mild" (normalized identifiers),
    /// "weak" (structural only), or "semantic" (type-aware). Defaults to "mild".
    pub mode: Option<String>,

    /// Minimum token count for a clone to be reported. Default: 50.
    pub min_tokens: Option<u32>,

    /// Minimum line count for a clone to be reported. Default: 5.
    pub min_lines: Option<u32>,

    /// Fail if duplication percentage exceeds this value. 0 = no limit.
    pub threshold: Option<f64>,

    /// Skip file-local duplicates, only report cross-file clones.
    pub skip_local: Option<bool>,

    /// Enable cross-language detection (strip TS type annotations for TS↔JS matching).
    pub cross_language: Option<bool>,

    /// Show only the N largest clone groups.
    pub top: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
pub struct FixParams {
    /// Root directory of the project. Defaults to current working directory.
    pub root: Option<String>,

    /// Path to fallow config file.
    pub config: Option<String>,

    /// Only analyze production code (excludes tests, stories, dev files).
    pub production: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct ProjectInfoParams {
    /// Root directory of the project. Defaults to current working directory.
    pub root: Option<String>,

    /// Path to fallow config file.
    pub config: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct HealthParams {
    /// Root directory of the project to analyze. Defaults to current working directory.
    pub root: Option<String>,

    /// Maximum cyclomatic complexity threshold. Functions exceeding this are reported.
    pub max_cyclomatic: Option<u16>,

    /// Maximum cognitive complexity threshold. Functions exceeding this are reported.
    pub max_cognitive: Option<u16>,

    /// Number of top results to return, sorted by complexity.
    pub top: Option<usize>,

    /// Sort order for results (e.g., "cyclomatic", "cognitive").
    pub sort: Option<String>,

    /// Git ref to compare against. Only files changed since this ref are analyzed.
    pub changed_since: Option<String>,

    /// Show only complexity findings. By default all sections are shown; use this to select only complexity.
    pub complexity: Option<bool>,

    /// Show only per-file health scores (fan-in, fan-out, dead code ratio, maintainability index).
    pub file_scores: Option<bool>,

    /// Show only hotspots: files that are both complex and frequently changing.
    pub hotspots: Option<bool>,

    /// Git history window for hotspot analysis. Accepts durations (6m, 90d, 1y) or ISO dates.
    pub since: Option<String>,

    /// Minimum commits for a file to appear in hotspot ranking.
    pub min_commits: Option<u32>,

    /// Scope output to a single workspace package.
    pub workspace: Option<String>,

    /// Only analyze production code (excludes tests, stories, dev files).
    pub production: Option<bool>,
}

// ── Server ─────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct FallowMcp {
    binary: String,
    tool_router: ToolRouter<Self>,
}

impl FallowMcp {
    pub fn new() -> Self {
        let binary = resolve_binary();
        Self {
            binary,
            tool_router: Self::tool_router(),
        }
    }
}

/// Resolve the fallow binary path.
/// Priority: `FALLOW_BIN` env var > sibling binary next to fallow-mcp > PATH lookup.
fn resolve_binary() -> String {
    if let Ok(bin) = std::env::var("FALLOW_BIN") {
        return bin;
    }

    // Check for sibling binary next to the current executable (npm install scenario)
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.with_file_name("fallow");
        if sibling.is_file()
            && let Some(path) = sibling.to_str()
        {
            return path.to_string();
        }
    }

    "fallow".to_string()
}

// ── Tool implementations ───────────────────────────────────────────

/// Issue type flag names mapped to their CLI flags.
const ISSUE_TYPE_FLAGS: &[(&str, &str)] = &[
    ("unused-files", "--unused-files"),
    ("unused-exports", "--unused-exports"),
    ("unused-types", "--unused-types"),
    ("unused-deps", "--unused-deps"),
    ("unused-enum-members", "--unused-enum-members"),
    ("unused-class-members", "--unused-class-members"),
    ("unresolved-imports", "--unresolved-imports"),
    ("unlisted-deps", "--unlisted-deps"),
    ("duplicate-exports", "--duplicate-exports"),
    ("circular-deps", "--circular-deps"),
];

/// Valid detection modes for the `find_dupes` tool.
const VALID_DUPES_MODES: &[&str] = &["strict", "mild", "weak", "semantic"];

// ── Argument builders (pure functions, testable without async) ─────

/// Build CLI arguments for the `analyze` tool.
/// Returns `Err(message)` if an invalid issue type is provided.
fn build_analyze_args(params: &AnalyzeParams) -> Result<Vec<String>, String> {
    let mut args = vec![
        "check".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--quiet".to_string(),
    ];

    if let Some(ref root) = params.root {
        args.extend(["--root".to_string(), root.clone()]);
    }
    if let Some(ref config) = params.config {
        args.extend(["--config".to_string(), config.clone()]);
    }
    if params.production == Some(true) {
        args.push("--production".to_string());
    }
    if let Some(ref workspace) = params.workspace {
        args.extend(["--workspace".to_string(), workspace.clone()]);
    }
    if let Some(ref types) = params.issue_types {
        for t in types {
            match ISSUE_TYPE_FLAGS.iter().find(|&&(name, _)| name == t) {
                Some(&(_, flag)) => args.push(flag.to_string()),
                None => {
                    let valid = ISSUE_TYPE_FLAGS
                        .iter()
                        .map(|&(n, _)| n)
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(format!("Unknown issue type '{t}'. Valid values: {valid}"));
                }
            }
        }
    }

    Ok(args)
}

/// Build CLI arguments for the `check_changed` tool.
fn build_check_changed_args(params: CheckChangedParams) -> Vec<String> {
    let mut args = vec![
        "check".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--quiet".to_string(),
        "--changed-since".to_string(),
        params.since,
    ];

    if let Some(ref root) = params.root {
        args.extend(["--root".to_string(), root.clone()]);
    }
    if let Some(ref config) = params.config {
        args.extend(["--config".to_string(), config.clone()]);
    }
    if params.production == Some(true) {
        args.push("--production".to_string());
    }
    if let Some(ref workspace) = params.workspace {
        args.extend(["--workspace".to_string(), workspace.clone()]);
    }

    args
}

/// Build CLI arguments for the `find_dupes` tool.
/// Returns `Err(message)` if an invalid mode is provided.
fn build_find_dupes_args(params: &FindDupesParams) -> Result<Vec<String>, String> {
    let mut args = vec![
        "dupes".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--quiet".to_string(),
    ];

    if let Some(ref root) = params.root {
        args.extend(["--root".to_string(), root.clone()]);
    }
    if let Some(ref mode) = params.mode {
        if !VALID_DUPES_MODES.contains(&mode.as_str()) {
            return Err(format!(
                "Invalid mode '{mode}'. Valid values: strict, mild, weak, semantic"
            ));
        }
        args.extend(["--mode".to_string(), mode.clone()]);
    }
    if let Some(min_tokens) = params.min_tokens {
        args.extend(["--min-tokens".to_string(), min_tokens.to_string()]);
    }
    if let Some(min_lines) = params.min_lines {
        args.extend(["--min-lines".to_string(), min_lines.to_string()]);
    }
    if let Some(threshold) = params.threshold {
        args.extend(["--threshold".to_string(), threshold.to_string()]);
    }
    if params.skip_local == Some(true) {
        args.push("--skip-local".to_string());
    }
    if params.cross_language == Some(true) {
        args.push("--cross-language".to_string());
    }
    if let Some(top) = params.top {
        args.extend(["--top".to_string(), top.to_string()]);
    }

    Ok(args)
}

/// Build CLI arguments for the `fix_preview` tool.
fn build_fix_preview_args(params: &FixParams) -> Vec<String> {
    let mut args = vec![
        "fix".to_string(),
        "--dry-run".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--quiet".to_string(),
    ];

    if let Some(ref root) = params.root {
        args.extend(["--root".to_string(), root.clone()]);
    }
    if let Some(ref config) = params.config {
        args.extend(["--config".to_string(), config.clone()]);
    }
    if params.production == Some(true) {
        args.push("--production".to_string());
    }

    args
}

/// Build CLI arguments for the `fix_apply` tool.
fn build_fix_apply_args(params: &FixParams) -> Vec<String> {
    let mut args = vec![
        "fix".to_string(),
        "--yes".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--quiet".to_string(),
    ];

    if let Some(ref root) = params.root {
        args.extend(["--root".to_string(), root.clone()]);
    }
    if let Some(ref config) = params.config {
        args.extend(["--config".to_string(), config.clone()]);
    }
    if params.production == Some(true) {
        args.push("--production".to_string());
    }

    args
}

/// Build CLI arguments for the `project_info` tool.
fn build_project_info_args(params: &ProjectInfoParams) -> Vec<String> {
    let mut args = vec![
        "list".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--quiet".to_string(),
    ];

    if let Some(ref root) = params.root {
        args.extend(["--root".to_string(), root.clone()]);
    }
    if let Some(ref config) = params.config {
        args.extend(["--config".to_string(), config.clone()]);
    }

    args
}

/// Build CLI arguments for the `check_health` tool.
fn build_health_args(params: &HealthParams) -> Vec<String> {
    let mut args = vec![
        "health".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--quiet".to_string(),
    ];

    if let Some(ref root) = params.root {
        args.extend(["--root".to_string(), root.clone()]);
    }
    if let Some(max_cyclomatic) = params.max_cyclomatic {
        args.extend(["--max-cyclomatic".to_string(), max_cyclomatic.to_string()]);
    }
    if let Some(max_cognitive) = params.max_cognitive {
        args.extend(["--max-cognitive".to_string(), max_cognitive.to_string()]);
    }
    if let Some(top) = params.top {
        args.extend(["--top".to_string(), top.to_string()]);
    }
    if let Some(ref sort) = params.sort {
        args.extend(["--sort".to_string(), sort.clone()]);
    }
    if let Some(ref changed_since) = params.changed_since {
        args.extend(["--changed-since".to_string(), changed_since.clone()]);
    }
    if params.complexity == Some(true) {
        args.push("--complexity".to_string());
    }
    if params.file_scores == Some(true) {
        args.push("--file-scores".to_string());
    }
    if params.hotspots == Some(true) {
        args.push("--hotspots".to_string());
    }
    if let Some(ref since) = params.since {
        args.extend(["--since".to_string(), since.clone()]);
    }
    if let Some(min_commits) = params.min_commits {
        args.extend(["--min-commits".to_string(), min_commits.to_string()]);
    }
    if let Some(ref workspace) = params.workspace {
        args.extend(["--workspace".to_string(), workspace.clone()]);
    }
    if params.production == Some(true) {
        args.push("--production".to_string());
    }

    args
}

#[tool_router]
impl FallowMcp {
    #[tool(
        description = "Analyze a TypeScript/JavaScript project for unused code, circular dependencies, code duplication, complexity hotspots, and more. Detects unused files, exports, types, dependencies, enum/class members, unresolved imports, unlisted dependencies, duplicate exports, and circular dependencies. Returns structured JSON with all issues found, grouped by issue type.",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    async fn analyze(&self, params: Parameters<AnalyzeParams>) -> Result<CallToolResult, McpError> {
        let params = params.0;
        match build_analyze_args(&params) {
            Ok(args) => run_fallow(&self.binary, &args).await,
            Err(msg) => Ok(CallToolResult::error(vec![Content::text(msg)])),
        }
    }

    #[tool(
        description = "Analyze only files changed since a git ref. Useful for incremental CI checks on pull requests. Returns the same structured JSON as analyze, but filtered to only include issues in changed files.",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    async fn check_changed(
        &self,
        params: Parameters<CheckChangedParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = build_check_changed_args(params.0);
        run_fallow(&self.binary, &args).await
    }

    #[tool(
        description = "Find code duplication across the project. Detects clone groups (identical or similar code blocks) with configurable detection modes and thresholds. Returns clone families with refactoring suggestions. Set top=N to show only the N largest clone groups.",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    async fn find_dupes(
        &self,
        params: Parameters<FindDupesParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
        match build_find_dupes_args(&params) {
            Ok(args) => run_fallow(&self.binary, &args).await,
            Err(msg) => Ok(CallToolResult::error(vec![Content::text(msg)])),
        }
    }

    #[tool(
        description = "Preview auto-fixes without modifying any files. Shows what would be changed: which unused exports would be removed and which unused dependencies would be deleted from package.json. Returns a JSON list of planned fixes.",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    async fn fix_preview(&self, params: Parameters<FixParams>) -> Result<CallToolResult, McpError> {
        let args = build_fix_preview_args(&params.0);
        run_fallow(&self.binary, &args).await
    }

    #[tool(
        description = "Apply auto-fixes to the project. Removes unused export keywords from source files and deletes unused dependencies from package.json. This modifies files on disk. Use fix_preview first to review planned changes.",
        annotations(destructive_hint = true, read_only_hint = false)
    )]
    async fn fix_apply(&self, params: Parameters<FixParams>) -> Result<CallToolResult, McpError> {
        let args = build_fix_apply_args(&params.0);
        run_fallow(&self.binary, &args).await
    }

    #[tool(
        description = "Get project metadata: active framework plugins, discovered source files, and detected entry points. Useful for understanding how fallow sees the project before running analysis.",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    async fn project_info(
        &self,
        params: Parameters<ProjectInfoParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = build_project_info_args(&params.0);
        run_fallow(&self.binary, &args).await
    }

    #[tool(
        description = "Check code health metrics (cyclomatic and cognitive complexity) for functions in the project. Returns structured JSON with complexity scores per function, sorted by severity. Set file_scores=true for per-file maintainability index (fan-in, fan-out, dead code ratio, complexity density). Set hotspots=true to identify files that are both complex and frequently changing (combines git churn with complexity). Useful for identifying hard-to-maintain code and prioritizing refactoring.",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    async fn check_health(
        &self,
        params: Parameters<HealthParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = build_health_args(&params.0);
        run_fallow(&self.binary, &args).await
    }
}

// ── ServerHandler ──────────────────────────────────────────────────

#[rmcp::tool_handler]
impl ServerHandler for FallowMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                Implementation::new("fallow-mcp", env!("CARGO_PKG_VERSION"))
                    .with_description("Codebase analysis for TypeScript/JavaScript projects"),
            )
            .with_instructions(
                "Fallow MCP server — codebase analysis for TypeScript/JavaScript projects. \
                 Tools: analyze (full analysis), check_changed (incremental/PR analysis), \
                 find_dupes (code duplication), fix_preview/fix_apply (auto-fix), \
                 project_info (plugins, files, entry points), \
                 check_health (code complexity metrics).",
            )
    }
}

// ── Runner ─────────────────────────────────────────────────────────

async fn run_fallow(binary: &str, args: &[String]) -> Result<CallToolResult, McpError> {
    let output = Command::new(binary)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| {
            McpError::internal_error(
                format!(
                    "Failed to execute fallow binary '{binary}': {e}. \
                     Ensure fallow is installed and available in PATH, \
                     or set the FALLOW_BIN environment variable."
                ),
                None,
            )
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        let exit_code = output.status.code().unwrap_or(-1);

        // Exit code 1 = issues found (not an error for analysis tools)
        if exit_code == 1 {
            let text = if stdout.is_empty() {
                "{}".to_string()
            } else {
                stdout.to_string()
            };
            return Ok(CallToolResult::success(vec![Content::text(text)]));
        }

        // Exit code 2 = real error (invalid config, etc.)
        let error_msg = if stderr.is_empty() {
            format!("fallow exited with code {exit_code}")
        } else {
            format!("fallow exited with code {exit_code}: {}", stderr.trim())
        };

        return Ok(CallToolResult::error(vec![Content::text(error_msg)]));
    }

    if stdout.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "{}".to_string(),
        )]));
    }

    Ok(CallToolResult::success(vec![Content::text(
        stdout.to_string(),
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Extract the text content from a `CallToolResult`.
    fn extract_text(result: &CallToolResult) -> &str {
        match &result.content[0].raw {
            RawContent::Text(t) => &t.text,
            _ => panic!("expected text content"),
        }
    }

    // ── Server info & tool registration ───────────────────────────

    #[test]
    fn server_info_is_correct() {
        let server = FallowMcp::new();
        let info = ServerHandler::get_info(&server);
        assert_eq!(info.server_info.name, "fallow-mcp");
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
        assert!(info.capabilities.tools.is_some());
        assert!(info.instructions.is_some());
    }

    #[test]
    fn all_tools_registered() {
        let server = FallowMcp::new();
        let tools = server.tool_router.list_all();
        let names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();
        assert!(names.contains(&"analyze".to_string()));
        assert!(names.contains(&"check_changed".to_string()));
        assert!(names.contains(&"find_dupes".to_string()));
        assert!(names.contains(&"fix_preview".to_string()));
        assert!(names.contains(&"fix_apply".to_string()));
        assert!(names.contains(&"project_info".to_string()));
        assert!(names.contains(&"check_health".to_string()));
        assert_eq!(tools.len(), 7);
    }

    #[test]
    fn read_only_tools_have_annotations() {
        let server = FallowMcp::new();
        let tools = server.tool_router.list_all();
        let read_only = [
            "analyze",
            "check_changed",
            "find_dupes",
            "fix_preview",
            "project_info",
            "check_health",
        ];
        for tool in &tools {
            let name = tool.name.to_string();
            if read_only.contains(&name.as_str()) {
                let ann = tool.annotations.as_ref().expect("annotations");
                assert_eq!(ann.read_only_hint, Some(true), "{name} should be read-only");
            }
        }
    }

    #[test]
    fn fix_apply_is_destructive() {
        let server = FallowMcp::new();
        let tools = server.tool_router.list_all();
        let fix = tools.iter().find(|t| t.name == "fix_apply").unwrap();
        let ann = fix.annotations.as_ref().unwrap();
        assert_eq!(ann.destructive_hint, Some(true));
        assert_eq!(ann.read_only_hint, Some(false));
    }

    #[test]
    fn issue_type_flags_are_complete() {
        assert_eq!(ISSUE_TYPE_FLAGS.len(), 10);
        for &(name, flag) in ISSUE_TYPE_FLAGS {
            assert!(
                flag.starts_with("--"),
                "flag for {name} should start with --"
            );
        }
    }

    // ── Parameter deserialization ─────────────────────────────────

    #[test]
    fn analyze_params_deserialize() {
        let json = r#"{"root":"/tmp/project","production":true,"issue_types":["unused-files"]}"#;
        let params: AnalyzeParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.root.as_deref(), Some("/tmp/project"));
        assert_eq!(params.production, Some(true));
        assert_eq!(params.issue_types.unwrap(), vec!["unused-files"]);
    }

    #[test]
    fn analyze_params_minimal() {
        let json = "{}";
        let params: AnalyzeParams = serde_json::from_str(json).unwrap();
        assert!(params.root.is_none());
        assert!(params.production.is_none());
        assert!(params.issue_types.is_none());
    }

    #[test]
    fn check_changed_params_require_since() {
        let json = "{}";
        let result: Result<CheckChangedParams, _> = serde_json::from_str(json);
        assert!(result.is_err());

        let json = r#"{"since":"main"}"#;
        let params: CheckChangedParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.since, "main");
    }

    #[test]
    fn find_dupes_params_defaults() {
        let json = "{}";
        let params: FindDupesParams = serde_json::from_str(json).unwrap();
        assert!(params.mode.is_none());
        assert!(params.min_tokens.is_none());
        assert!(params.skip_local.is_none());
    }

    #[test]
    fn fix_params_with_production() {
        let json = r#"{"root":"/tmp","production":true}"#;
        let params: FixParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.production, Some(true));
    }

    #[test]
    fn health_params_all_fields_deserialize() {
        let json = r#"{
            "root": "/project",
            "max_cyclomatic": 25,
            "max_cognitive": 30,
            "top": 10,
            "sort": "cognitive",
            "changed_since": "HEAD~3"
        }"#;
        let params: HealthParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.root.as_deref(), Some("/project"));
        assert_eq!(params.max_cyclomatic, Some(25));
        assert_eq!(params.max_cognitive, Some(30));
        assert_eq!(params.top, Some(10));
        assert_eq!(params.sort.as_deref(), Some("cognitive"));
        assert_eq!(params.changed_since.as_deref(), Some("HEAD~3"));
    }

    #[test]
    fn health_params_minimal() {
        let params: HealthParams = serde_json::from_str("{}").unwrap();
        assert!(params.root.is_none());
        assert!(params.max_cyclomatic.is_none());
        assert!(params.max_cognitive.is_none());
        assert!(params.top.is_none());
        assert!(params.sort.is_none());
        assert!(params.changed_since.is_none());
    }

    #[test]
    fn project_info_params_deserialize() {
        let json = r#"{"root": "/app", "config": ".fallowrc.json"}"#;
        let params: ProjectInfoParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.root.as_deref(), Some("/app"));
        assert_eq!(params.config.as_deref(), Some(".fallowrc.json"));
    }

    #[test]
    fn find_dupes_params_all_fields_deserialize() {
        let json = r#"{
            "root": "/project",
            "mode": "strict",
            "min_tokens": 100,
            "min_lines": 10,
            "threshold": 5.5,
            "skip_local": true,
            "top": 5
        }"#;
        let params: FindDupesParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.root.as_deref(), Some("/project"));
        assert_eq!(params.mode.as_deref(), Some("strict"));
        assert_eq!(params.min_tokens, Some(100));
        assert_eq!(params.min_lines, Some(10));
        assert_eq!(params.threshold, Some(5.5));
        assert_eq!(params.skip_local, Some(true));
        assert_eq!(params.top, Some(5));
    }

    // ── Argument building: analyze ────────────────────────────────

    #[test]
    fn analyze_args_minimal_produces_base_args() {
        let params = AnalyzeParams {
            root: None,
            config: None,
            production: None,
            workspace: None,
            issue_types: None,
        };
        let args = build_analyze_args(&params).unwrap();
        assert_eq!(args, ["check", "--format", "json", "--quiet"]);
    }

    #[test]
    fn analyze_args_with_all_options() {
        let params = AnalyzeParams {
            root: Some("/my/project".to_string()),
            config: Some("fallow.toml".to_string()),
            production: Some(true),
            workspace: Some("@my/pkg".to_string()),
            issue_types: Some(vec![
                "unused-files".to_string(),
                "unused-exports".to_string(),
            ]),
        };
        let args = build_analyze_args(&params).unwrap();
        assert_eq!(
            args,
            [
                "check",
                "--format",
                "json",
                "--quiet",
                "--root",
                "/my/project",
                "--config",
                "fallow.toml",
                "--production",
                "--workspace",
                "@my/pkg",
                "--unused-files",
                "--unused-exports",
            ]
        );
    }

    #[test]
    fn analyze_args_production_false_is_omitted() {
        let params = AnalyzeParams {
            root: None,
            config: None,
            production: Some(false),
            workspace: None,
            issue_types: None,
        };
        let args = build_analyze_args(&params).unwrap();
        assert!(!args.contains(&"--production".to_string()));
    }

    #[test]
    fn analyze_args_invalid_issue_type_returns_error() {
        let params = AnalyzeParams {
            root: None,
            config: None,
            production: None,
            workspace: None,
            issue_types: Some(vec!["nonexistent-type".to_string()]),
        };
        let err = build_analyze_args(&params).unwrap_err();
        assert!(err.contains("Unknown issue type 'nonexistent-type'"));
        assert!(err.contains("unused-files"));
    }

    #[test]
    fn analyze_args_all_issue_types_accepted() {
        let all_types: Vec<String> = ISSUE_TYPE_FLAGS
            .iter()
            .map(|&(name, _)| name.to_string())
            .collect();
        let params = AnalyzeParams {
            root: None,
            config: None,
            production: None,
            workspace: None,
            issue_types: Some(all_types),
        };
        let args = build_analyze_args(&params).unwrap();
        for &(_, flag) in ISSUE_TYPE_FLAGS {
            assert!(
                args.contains(&flag.to_string()),
                "missing flag {flag} in args"
            );
        }
    }

    #[test]
    fn analyze_args_mixed_valid_and_invalid_issue_types_fails_on_first_invalid() {
        let params = AnalyzeParams {
            root: None,
            config: None,
            production: None,
            workspace: None,
            issue_types: Some(vec![
                "unused-files".to_string(),
                "bogus".to_string(),
                "unused-deps".to_string(),
            ]),
        };
        let err = build_analyze_args(&params).unwrap_err();
        assert!(err.contains("'bogus'"));
    }

    #[test]
    fn analyze_args_empty_issue_types_vec_produces_no_flags() {
        let params = AnalyzeParams {
            root: None,
            config: None,
            production: None,
            workspace: None,
            issue_types: Some(vec![]),
        };
        let args = build_analyze_args(&params).unwrap();
        assert_eq!(args, ["check", "--format", "json", "--quiet"]);
    }

    // ── Argument building: check_changed ──────────────────────────

    #[test]
    fn check_changed_args_includes_since_ref() {
        let params = CheckChangedParams {
            root: None,
            since: "main".to_string(),
            config: None,
            production: None,
            workspace: None,
        };
        let args = build_check_changed_args(params);
        assert_eq!(
            args,
            [
                "check",
                "--format",
                "json",
                "--quiet",
                "--changed-since",
                "main"
            ]
        );
    }

    #[test]
    fn check_changed_args_with_all_options() {
        let params = CheckChangedParams {
            root: Some("/app".to_string()),
            since: "HEAD~5".to_string(),
            config: Some("custom.json".to_string()),
            production: Some(true),
            workspace: Some("frontend".to_string()),
        };
        let args = build_check_changed_args(params);
        assert_eq!(
            args,
            [
                "check",
                "--format",
                "json",
                "--quiet",
                "--changed-since",
                "HEAD~5",
                "--root",
                "/app",
                "--config",
                "custom.json",
                "--production",
                "--workspace",
                "frontend",
            ]
        );
    }

    #[test]
    fn check_changed_args_with_commit_sha() {
        let params = CheckChangedParams {
            root: None,
            since: "abc123def456".to_string(),
            config: None,
            production: None,
            workspace: None,
        };
        let args = build_check_changed_args(params);
        assert!(args.contains(&"abc123def456".to_string()));
    }

    // ── Argument building: find_dupes ─────────────────────────────

    #[test]
    fn find_dupes_args_minimal() {
        let params = FindDupesParams {
            root: None,
            mode: None,
            min_tokens: None,
            min_lines: None,
            threshold: None,
            skip_local: None,
            cross_language: None,
            top: None,
        };
        let args = build_find_dupes_args(&params).unwrap();
        assert_eq!(args, ["dupes", "--format", "json", "--quiet"]);
    }

    #[test]
    fn find_dupes_args_with_all_options() {
        let params = FindDupesParams {
            root: Some("/repo".to_string()),
            mode: Some("semantic".to_string()),
            min_tokens: Some(100),
            min_lines: Some(10),
            threshold: Some(5.5),
            skip_local: Some(true),
            cross_language: Some(true),
            top: Some(5),
        };
        let args = build_find_dupes_args(&params).unwrap();
        assert_eq!(
            args,
            [
                "dupes",
                "--format",
                "json",
                "--quiet",
                "--root",
                "/repo",
                "--mode",
                "semantic",
                "--min-tokens",
                "100",
                "--min-lines",
                "10",
                "--threshold",
                "5.5",
                "--skip-local",
                "--cross-language",
                "--top",
                "5",
            ]
        );
    }

    #[test]
    fn find_dupes_args_all_valid_modes_accepted() {
        for mode in VALID_DUPES_MODES {
            let params = FindDupesParams {
                root: None,
                mode: Some(mode.to_string()),
                min_tokens: None,
                min_lines: None,
                threshold: None,
                skip_local: None,
                cross_language: None,
                top: None,
            };
            let args = build_find_dupes_args(&params).unwrap();
            assert!(
                args.contains(&mode.to_string()),
                "mode '{mode}' should be in args"
            );
        }
    }

    #[test]
    fn find_dupes_args_invalid_mode_returns_error() {
        let params = FindDupesParams {
            root: None,
            mode: Some("aggressive".to_string()),
            min_tokens: None,
            min_lines: None,
            threshold: None,
            skip_local: None,
            cross_language: None,
            top: None,
        };
        let err = build_find_dupes_args(&params).unwrap_err();
        assert!(err.contains("Invalid mode 'aggressive'"));
        assert!(err.contains("strict"));
        assert!(err.contains("mild"));
        assert!(err.contains("weak"));
        assert!(err.contains("semantic"));
    }

    #[test]
    fn find_dupes_args_skip_local_false_is_omitted() {
        let params = FindDupesParams {
            root: None,
            mode: None,
            min_tokens: None,
            min_lines: None,
            threshold: None,
            skip_local: Some(false),
            cross_language: None,
            top: None,
        };
        let args = build_find_dupes_args(&params).unwrap();
        assert!(!args.contains(&"--skip-local".to_string()));
    }

    #[test]
    fn find_dupes_args_threshold_zero() {
        let params = FindDupesParams {
            root: None,
            mode: None,
            min_tokens: None,
            min_lines: None,
            threshold: Some(0.0),
            skip_local: None,
            cross_language: None,
            top: None,
        };
        let args = build_find_dupes_args(&params).unwrap();
        assert!(args.contains(&"--threshold".to_string()));
        assert!(args.contains(&"0".to_string()));
    }

    // ── Argument building: fix_preview vs fix_apply ───────────────

    #[test]
    fn fix_preview_args_include_dry_run() {
        let params = FixParams {
            root: None,
            config: None,
            production: None,
        };
        let args = build_fix_preview_args(&params);
        assert!(args.contains(&"--dry-run".to_string()));
        assert!(!args.contains(&"--yes".to_string()));
        assert_eq!(args[0], "fix");
    }

    #[test]
    fn fix_apply_args_include_yes_flag() {
        let params = FixParams {
            root: None,
            config: None,
            production: None,
        };
        let args = build_fix_apply_args(&params);
        assert!(args.contains(&"--yes".to_string()));
        assert!(!args.contains(&"--dry-run".to_string()));
        assert_eq!(args[0], "fix");
    }

    #[test]
    fn fix_preview_args_with_all_options() {
        let params = FixParams {
            root: Some("/app".to_string()),
            config: Some("config.json".to_string()),
            production: Some(true),
        };
        let args = build_fix_preview_args(&params);
        assert_eq!(
            args,
            [
                "fix",
                "--dry-run",
                "--format",
                "json",
                "--quiet",
                "--root",
                "/app",
                "--config",
                "config.json",
                "--production",
            ]
        );
    }

    #[test]
    fn fix_apply_args_with_all_options() {
        let params = FixParams {
            root: Some("/app".to_string()),
            config: Some("config.json".to_string()),
            production: Some(true),
        };
        let args = build_fix_apply_args(&params);
        assert_eq!(
            args,
            [
                "fix",
                "--yes",
                "--format",
                "json",
                "--quiet",
                "--root",
                "/app",
                "--config",
                "config.json",
                "--production",
            ]
        );
    }

    // ── Argument building: project_info ───────────────────────────

    #[test]
    fn project_info_args_minimal() {
        let params = ProjectInfoParams {
            root: None,
            config: None,
        };
        let args = build_project_info_args(&params);
        assert_eq!(args, ["list", "--format", "json", "--quiet"]);
    }

    #[test]
    fn project_info_args_with_root_and_config() {
        let params = ProjectInfoParams {
            root: Some("/workspace".to_string()),
            config: Some("fallow.toml".to_string()),
        };
        let args = build_project_info_args(&params);
        assert_eq!(
            args,
            [
                "list",
                "--format",
                "json",
                "--quiet",
                "--root",
                "/workspace",
                "--config",
                "fallow.toml",
            ]
        );
    }

    // ── Argument building: health ─────────────────────────────────

    #[test]
    fn health_args_minimal() {
        let params = HealthParams {
            root: None,
            max_cyclomatic: None,
            max_cognitive: None,
            top: None,
            sort: None,
            changed_since: None,
            complexity: None,
            file_scores: None,
            hotspots: None,
            since: None,
            min_commits: None,
            production: None,
            workspace: None,
        };
        let args = build_health_args(&params);
        assert_eq!(args, ["health", "--format", "json", "--quiet"]);
    }

    #[test]
    fn health_args_with_all_options() {
        let params = HealthParams {
            root: Some("/src".to_string()),
            max_cyclomatic: Some(25),
            max_cognitive: Some(15),
            top: Some(20),
            sort: Some("cognitive".to_string()),
            changed_since: Some("develop".to_string()),
            complexity: Some(true),
            file_scores: Some(true),
            hotspots: Some(true),
            since: Some("6m".to_string()),
            min_commits: Some(5),
            workspace: Some("packages/ui".to_string()),
            production: Some(true),
        };
        let args = build_health_args(&params);
        assert_eq!(
            args,
            [
                "health",
                "--format",
                "json",
                "--quiet",
                "--root",
                "/src",
                "--max-cyclomatic",
                "25",
                "--max-cognitive",
                "15",
                "--top",
                "20",
                "--sort",
                "cognitive",
                "--changed-since",
                "develop",
                "--complexity",
                "--file-scores",
                "--hotspots",
                "--since",
                "6m",
                "--min-commits",
                "5",
                "--workspace",
                "packages/ui",
                "--production",
            ]
        );
    }

    #[test]
    fn health_args_partial_options() {
        let params = HealthParams {
            root: None,
            max_cyclomatic: Some(10),
            max_cognitive: None,
            top: None,
            sort: Some("cyclomatic".to_string()),
            changed_since: None,
            complexity: None,
            file_scores: None,
            hotspots: None,
            since: None,
            min_commits: None,
            workspace: None,
            production: None,
        };
        let args = build_health_args(&params);
        assert_eq!(
            args,
            [
                "health",
                "--format",
                "json",
                "--quiet",
                "--max-cyclomatic",
                "10",
                "--sort",
                "cyclomatic",
            ]
        );
    }

    // ── All tools produce --format json --quiet ───────────────────

    #[test]
    fn all_arg_builders_include_format_json_and_quiet() {
        let analyze = build_analyze_args(&AnalyzeParams {
            root: None,
            config: None,
            production: None,
            workspace: None,
            issue_types: None,
        })
        .unwrap();

        let check_changed = build_check_changed_args(CheckChangedParams {
            root: None,
            since: "main".to_string(),
            config: None,
            production: None,
            workspace: None,
        });

        let dupes = build_find_dupes_args(&FindDupesParams {
            root: None,
            mode: None,
            min_tokens: None,
            min_lines: None,
            threshold: None,
            skip_local: None,
            cross_language: None,
            top: None,
        })
        .unwrap();

        let fix_preview = build_fix_preview_args(&FixParams {
            root: None,
            config: None,
            production: None,
        });

        let fix_apply = build_fix_apply_args(&FixParams {
            root: None,
            config: None,
            production: None,
        });

        let project_info = build_project_info_args(&ProjectInfoParams {
            root: None,
            config: None,
        });

        let health = build_health_args(&HealthParams {
            root: None,
            max_cyclomatic: None,
            max_cognitive: None,
            top: None,
            sort: None,
            changed_since: None,
            complexity: None,
            file_scores: None,
            hotspots: None,
            since: None,
            min_commits: None,
            workspace: None,
            production: None,
        });

        for (name, args) in [
            ("analyze", &analyze),
            ("check_changed", &check_changed),
            ("find_dupes", &dupes),
            ("fix_preview", &fix_preview),
            ("fix_apply", &fix_apply),
            ("project_info", &project_info),
            ("health", &health),
        ] {
            assert!(
                args.contains(&"--format".to_string()),
                "{name} missing --format"
            );
            assert!(args.contains(&"json".to_string()), "{name} missing json");
            assert!(
                args.contains(&"--quiet".to_string()),
                "{name} missing --quiet"
            );
        }
    }

    // ── Correct subcommand for each tool ──────────────────────────

    #[test]
    fn each_tool_uses_correct_subcommand() {
        let analyze = build_analyze_args(&AnalyzeParams {
            root: None,
            config: None,
            production: None,
            workspace: None,
            issue_types: None,
        })
        .unwrap();
        assert_eq!(analyze[0], "check");

        let changed = build_check_changed_args(CheckChangedParams {
            root: None,
            since: "x".to_string(),
            config: None,
            production: None,
            workspace: None,
        });
        assert_eq!(changed[0], "check");

        let dupes = build_find_dupes_args(&FindDupesParams {
            root: None,
            mode: None,
            min_tokens: None,
            min_lines: None,
            threshold: None,
            skip_local: None,
            cross_language: None,
            top: None,
        })
        .unwrap();
        assert_eq!(dupes[0], "dupes");

        let preview = build_fix_preview_args(&FixParams {
            root: None,
            config: None,
            production: None,
        });
        assert_eq!(preview[0], "fix");

        let apply = build_fix_apply_args(&FixParams {
            root: None,
            config: None,
            production: None,
        });
        assert_eq!(apply[0], "fix");

        let info = build_project_info_args(&ProjectInfoParams {
            root: None,
            config: None,
        });
        assert_eq!(info[0], "list");

        let health = build_health_args(&HealthParams {
            root: None,
            max_cyclomatic: None,
            max_cognitive: None,
            top: None,
            sort: None,
            changed_since: None,
            complexity: None,
            file_scores: None,
            hotspots: None,
            since: None,
            min_commits: None,
            workspace: None,
            production: None,
        });
        assert_eq!(health[0], "health");
    }

    // ── run_fallow: binary execution and exit code handling ───────

    #[tokio::test]
    async fn run_fallow_missing_binary() {
        let result = run_fallow("nonexistent-binary-12345", &["check".to_string()]).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("nonexistent-binary-12345"));
        assert!(err.message.contains("FALLOW_BIN"));
    }

    // The following tests shell out to `/bin/sh` which is Unix-only.
    // On Windows, these are skipped.

    #[cfg(unix)]
    #[tokio::test]
    async fn run_fallow_exit_code_0_with_stdout() {
        // `echo '{"ok":true}'` exits 0 and writes to stdout
        let result = run_fallow(
            "/bin/sh",
            &["-c".to_string(), "echo '{\"ok\":true}'".to_string()],
        )
        .await
        .unwrap();
        assert_eq!(result.is_error, Some(false));
        let text = extract_text(&result);
        assert!(text.contains(r#"{"ok":true}"#));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_fallow_exit_code_0_empty_stdout_returns_empty_json() {
        // A command that succeeds with no output
        let result = run_fallow("/bin/sh", &["-c".to_string(), "true".to_string()])
            .await
            .unwrap();
        assert_eq!(result.is_error, Some(false));
        assert_eq!(extract_text(&result), "{}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_fallow_exit_code_1_treated_as_success_with_issues() {
        // Exit code 1 with JSON stdout = issues found (not an error)
        let result = run_fallow(
            "/bin/sh",
            &[
                "-c".to_string(),
                "echo '{\"issues\":[]}'; exit 1".to_string(),
            ],
        )
        .await
        .unwrap();
        assert_eq!(result.is_error, Some(false));
        let text = extract_text(&result);
        assert!(text.contains("issues"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_fallow_exit_code_1_empty_stdout_returns_empty_json() {
        let result = run_fallow("/bin/sh", &["-c".to_string(), "exit 1".to_string()])
            .await
            .unwrap();
        assert_eq!(result.is_error, Some(false));
        assert_eq!(extract_text(&result), "{}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_fallow_exit_code_2_with_stderr_returns_error() {
        let result = run_fallow(
            "/bin/sh",
            &[
                "-c".to_string(),
                "echo 'invalid config' >&2; exit 2".to_string(),
            ],
        )
        .await
        .unwrap();
        assert_eq!(result.is_error, Some(true));
        let text = extract_text(&result);
        assert!(text.contains("exited with code 2"));
        assert!(text.contains("invalid config"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_fallow_exit_code_2_empty_stderr_returns_generic_error() {
        let result = run_fallow("/bin/sh", &["-c".to_string(), "exit 2".to_string()])
            .await
            .unwrap();
        assert_eq!(result.is_error, Some(true));
        let text = extract_text(&result);
        assert_eq!(text, "fallow exited with code 2");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_fallow_high_exit_code_returns_error() {
        let result = run_fallow("/bin/sh", &["-c".to_string(), "exit 127".to_string()])
            .await
            .unwrap();
        assert_eq!(result.is_error, Some(true));
        let text = extract_text(&result);
        assert!(text.contains("127"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_fallow_stderr_is_trimmed_in_error_message() {
        let result = run_fallow(
            "/bin/sh",
            &[
                "-c".to_string(),
                "echo '  whitespace around  ' >&2; exit 3".to_string(),
            ],
        )
        .await
        .unwrap();
        let text = extract_text(&result);
        // Verify stderr is trimmed (no trailing whitespace/newline)
        assert!(text.ends_with("whitespace around"));
    }

    // ── resolve_binary ────────────────────────────────────────────

    #[test]
    #[expect(unsafe_code)]
    fn resolve_binary_defaults_to_fallow() {
        // SAFETY: test-only, no concurrent env access in this test binary
        unsafe { std::env::remove_var("FALLOW_BIN") };
        let bin = resolve_binary();
        // Either "fallow" (PATH) or a sibling path — both are valid
        assert!(bin.contains("fallow"));
    }

    #[test]
    #[expect(unsafe_code)]
    fn resolve_binary_respects_env_var() {
        // SAFETY: test-only, no concurrent env access in this test binary.
        // Both set_var and remove_var are unsafe in Rust 2024 edition due to
        // potential data races, but cargo test runs each test function serially
        // within the same thread by default.
        unsafe { std::env::set_var("FALLOW_BIN", "/custom/path/fallow") };
        let bin = resolve_binary();
        assert_eq!(bin, "/custom/path/fallow");
        // SAFETY: cleanup after test, same reasoning as above
        unsafe { std::env::remove_var("FALLOW_BIN") };
    }

    // ── Edge cases: special characters in arguments ───────────────

    #[test]
    fn analyze_args_with_spaces_in_paths() {
        let params = AnalyzeParams {
            root: Some("/path/with spaces/project".to_string()),
            config: Some("my config.json".to_string()),
            production: None,
            workspace: Some("my package".to_string()),
            issue_types: None,
        };
        let args = build_analyze_args(&params).unwrap();
        assert!(args.contains(&"/path/with spaces/project".to_string()));
        assert!(args.contains(&"my config.json".to_string()));
        assert!(args.contains(&"my package".to_string()));
    }

    #[test]
    fn check_changed_args_with_special_ref() {
        let params = CheckChangedParams {
            root: None,
            since: "origin/feature/my-branch".to_string(),
            config: None,
            production: None,
            workspace: None,
        };
        let args = build_check_changed_args(params);
        assert!(args.contains(&"origin/feature/my-branch".to_string()));
    }

    #[test]
    fn health_args_boundary_values() {
        let params = HealthParams {
            root: None,
            max_cyclomatic: Some(0),
            max_cognitive: Some(u16::MAX),
            top: Some(0),
            sort: None,
            changed_since: None,
            complexity: None,
            file_scores: None,
            hotspots: None,
            since: None,
            min_commits: None,
            workspace: None,
            production: None,
        };
        let args = build_health_args(&params);
        assert!(args.contains(&"0".to_string()));
        assert!(args.contains(&"65535".to_string()));
    }

    #[test]
    fn health_args_file_scores_flag() {
        let params = HealthParams {
            root: None,
            max_cyclomatic: None,
            max_cognitive: None,
            top: None,
            sort: None,
            changed_since: None,
            complexity: None,
            file_scores: Some(true),
            hotspots: None,
            since: None,
            min_commits: None,
            production: None,
            workspace: None,
        };
        let args = build_health_args(&params);
        assert!(args.contains(&"--file-scores".to_string()));
    }
}
