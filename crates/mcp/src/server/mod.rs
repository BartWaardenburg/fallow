use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::{ErrorData as McpError, ServerHandler, tool, tool_router};

use crate::params::*;
use crate::tools::{
    build_analyze_args, build_check_changed_args, build_find_dupes_args, build_fix_apply_args,
    build_fix_preview_args, build_health_args, build_project_info_args, run_fallow,
};

#[cfg(test)]
mod tests;

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

#[tool_router]
impl FallowMcp {
    #[tool(
        description = "Analyze a TypeScript/JavaScript project for unused code and circular dependencies. Detects unused files, exports, types, dependencies, enum/class members, unresolved imports, unlisted dependencies, duplicate exports, and circular dependencies. Returns structured JSON with all issues found, grouped by issue type. For code duplication use find_dupes, for complexity hotspots use check_health. Supports baseline comparisons (baseline/save_baseline) and performance tuning (no_cache, threads).",
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
        description = "Analyze only files changed since a git ref. Useful for incremental CI checks on pull requests. Returns the same structured JSON as analyze, but filtered to only include issues in changed files. Supports baseline comparisons (baseline/save_baseline) and performance tuning (no_cache, threads).",
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
        description = "Find code duplication across the project. Detects clone groups (identical or similar code blocks) with configurable detection modes and thresholds. Returns clone families with refactoring suggestions. Set top=N to show only the N largest clone groups. Supports config, workspace scoping, baseline comparisons, and performance tuning (no_cache, threads).",
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
        description = "Preview auto-fixes without modifying any files. Shows what would be changed: which unused exports would be removed and which unused dependencies would be deleted from package.json. Returns a JSON list of planned fixes. Supports workspace scoping and performance tuning (no_cache, threads).",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    async fn fix_preview(&self, params: Parameters<FixParams>) -> Result<CallToolResult, McpError> {
        let args = build_fix_preview_args(&params.0);
        run_fallow(&self.binary, &args).await
    }

    #[tool(
        description = "Apply auto-fixes to the project. Removes unused export keywords from source files and deletes unused dependencies from package.json. This modifies files on disk. Use fix_preview first to review planned changes. Supports workspace scoping and performance tuning (no_cache, threads).",
        annotations(destructive_hint = true, read_only_hint = false)
    )]
    async fn fix_apply(&self, params: Parameters<FixParams>) -> Result<CallToolResult, McpError> {
        let args = build_fix_apply_args(&params.0);
        run_fallow(&self.binary, &args).await
    }

    #[tool(
        description = "Get project metadata: active framework plugins, discovered source files, and detected entry points. Useful for understanding how fallow sees the project before running analysis. Supports performance tuning (no_cache, threads).",
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
        description = "Check code health metrics (cyclomatic and cognitive complexity) for functions in the project. Returns structured JSON with complexity scores per function, sorted by severity. Set file_scores=true for per-file maintainability index (fan-in, fan-out, dead code ratio, complexity density). Set hotspots=true to identify files that are both complex and frequently changing (combines git churn with complexity). Set targets=true for ranked refactoring recommendations based on complexity, coupling, churn, and dead code signals. Supports config, baseline comparisons, and performance tuning (no_cache, threads). Useful for identifying hard-to-maintain code and prioritizing refactoring.",
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
