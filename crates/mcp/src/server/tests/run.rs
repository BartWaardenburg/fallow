#[cfg(unix)]
use rmcp::model::*;

use crate::tools::run_fallow;

use super::super::resolve_binary;

/// Extract the text content from a `CallToolResult`.
#[cfg(unix)]
fn extract_text(result: &CallToolResult) -> &str {
    match &result.content[0].raw {
        RawContent::Text(t) => &t.text,
        _ => panic!("expected text content"),
    }
}

// ── run_fallow: binary execution and exit code handling ───────────

#[tokio::test]
async fn run_fallow_missing_binary() {
    let result = run_fallow("nonexistent-binary-12345", &["dead-code".to_string()]).await;
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

// ── resolve_binary ────────────────────────────────────────────────

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
