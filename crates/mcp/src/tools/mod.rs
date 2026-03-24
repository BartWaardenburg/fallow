mod analyze;
mod check_changed;
mod dupes;
mod fix;
mod health;
mod project_info;

pub use analyze::build_analyze_args;
pub use check_changed::build_check_changed_args;
pub use dupes::build_find_dupes_args;
pub use fix::{build_fix_apply_args, build_fix_preview_args};
pub use health::build_health_args;
pub use project_info::build_project_info_args;

use std::process::Stdio;

use rmcp::ErrorData as McpError;
use rmcp::model::*;
use tokio::process::Command;

/// Issue type flag names mapped to their CLI flags.
pub const ISSUE_TYPE_FLAGS: &[(&str, &str)] = &[
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
pub const VALID_DUPES_MODES: &[&str] = &["strict", "mild", "weak", "semantic"];

/// Execute the fallow CLI binary with the given arguments and return the result.
pub async fn run_fallow(binary: &str, args: &[String]) -> Result<CallToolResult, McpError> {
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
