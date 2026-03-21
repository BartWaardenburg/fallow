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

#[tool_router]
impl FallowMcp {
    #[tool(
        description = "Analyze a JavaScript/TypeScript project for dead code. Detects unused files, exports, types, dependencies, enum/class members, unresolved imports, unlisted dependencies, and duplicate exports. Returns structured JSON with all issues found, grouped by issue type.",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    async fn analyze(&self, params: Parameters<AnalyzeParams>) -> Result<CallToolResult, McpError> {
        let params = params.0;
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
                        return Ok(CallToolResult::error(vec![Content::text(format!(
                            "Unknown issue type '{t}'. Valid values: {valid}"
                        ))]));
                    }
                }
            }
        }

        run_fallow(&self.binary, &args).await
    }

    #[tool(
        description = "Analyze only files changed since a git ref. Useful for incremental CI checks on pull requests. Returns the same structured JSON as analyze, but filtered to only include issues in changed files.",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    async fn check_changed(
        &self,
        params: Parameters<CheckChangedParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
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

        run_fallow(&self.binary, &args).await
    }

    #[tool(
        description = "Find code duplication across the project. Detects clone groups (identical or similar code blocks) with configurable detection modes and thresholds. Returns clone families with refactoring suggestions.",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    async fn find_dupes(
        &self,
        params: Parameters<FindDupesParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;
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
            const VALID_MODES: &[&str] = &["strict", "mild", "weak", "semantic"];
            if !VALID_MODES.contains(&mode.as_str()) {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Invalid mode '{mode}'. Valid values: strict, mild, weak, semantic"
                ))]));
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

        run_fallow(&self.binary, &args).await
    }

    #[tool(
        description = "Preview auto-fixes without modifying any files. Shows what would be changed: which unused exports would be removed and which unused dependencies would be deleted from package.json. Returns a JSON list of planned fixes.",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    async fn fix_preview(&self, params: Parameters<FixParams>) -> Result<CallToolResult, McpError> {
        let params = params.0;
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

        run_fallow(&self.binary, &args).await
    }

    #[tool(
        description = "Apply auto-fixes to the project. Removes unused export keywords from source files and deletes unused dependencies from package.json. This modifies files on disk. Use fix_preview first to review planned changes.",
        annotations(destructive_hint = true, read_only_hint = false)
    )]
    async fn fix_apply(&self, params: Parameters<FixParams>) -> Result<CallToolResult, McpError> {
        let params = params.0;
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
        let params = params.0;
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
                    .with_description("Dead code analysis for JavaScript/TypeScript projects"),
            )
            .with_instructions(
                "Fallow MCP server — dead code analysis for JavaScript/TypeScript projects. \
                 Tools: analyze (full analysis), check_changed (incremental/PR analysis), \
                 find_dupes (code duplication), fix_preview/fix_apply (auto-fix), \
                 project_info (plugins, files, entry points).",
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
        assert_eq!(tools.len(), 6);
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

    #[tokio::test]
    async fn run_fallow_missing_binary() {
        let result = run_fallow("nonexistent-binary-12345", &["check".to_string()]).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("nonexistent-binary-12345"));
    }

    #[test]
    fn resolve_binary_defaults_to_fallow() {
        // SAFETY: test-only, no concurrent env access
        unsafe { std::env::remove_var("FALLOW_BIN") };
        let bin = resolve_binary();
        // Either "fallow" (PATH) or a sibling path — both are valid
        assert!(bin.contains("fallow"));
    }
}
