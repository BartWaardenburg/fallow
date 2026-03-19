use std::path::PathBuf;

pub(crate) fn validate_git_ref(s: &str) -> Result<&str, String> {
    if s.is_empty() {
        return Err("git ref cannot be empty".to_string());
    }
    // Reject refs starting with '-' to prevent argument injection
    if s.starts_with('-') {
        return Err("git ref cannot start with '-'".to_string());
    }
    // Allowlist: only permit safe characters in git refs.
    // Covers branches, tags, HEAD~N, HEAD^N, commit SHAs.
    // Inside braces (@{...}), colons and spaces are allowed for reflog
    // timestamps like HEAD@{2025-01-01} and HEAD@{1 week ago}.
    let mut in_braces = false;
    for c in s.chars() {
        match c {
            '{' => in_braces = true,
            '}' => in_braces = false,
            ':' | ' ' if in_braces => {} // allowed inside @{...}
            c if c.is_ascii_alphanumeric()
                || matches!(c, '.' | '_' | '-' | '/' | '~' | '^' | '@' | '{' | '}') => {}
            _ => return Err(format!("git ref contains disallowed character: '{c}'")),
        }
    }
    if in_braces {
        return Err("git ref has unclosed '{'".to_string());
    }
    Ok(s)
}

pub(crate) fn validate_root(root: &std::path::Path) -> Result<PathBuf, String> {
    let canonical = root
        .canonicalize()
        .map_err(|e| format!("invalid root path '{}': {e}", root.display()))?;
    if !canonical.is_dir() {
        return Err(format!("root path '{}' is not a directory", root.display()));
    }
    Ok(canonical)
}

/// Reject strings containing control characters (bytes < 0x20) except
/// newline (0x0A) and tab (0x09). This prevents agents from accidentally
/// passing invisible characters in CLI arguments.
pub(crate) fn validate_no_control_chars(s: &str, arg_name: &str) -> Result<(), String> {
    for (i, byte) in s.bytes().enumerate() {
        if byte < 0x20 && byte != b'\n' && byte != b'\t' {
            return Err(format!(
                "{arg_name} contains control character (byte 0x{byte:02x}) at position {i}"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── validate_no_control_chars ────────────────────────────────────

    #[test]
    fn control_chars_rejects_null_byte() {
        let result = validate_no_control_chars("main\x00branch", "--changed-since");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("0x00"));
        assert!(err.contains("--changed-since"));
    }

    #[test]
    fn control_chars_rejects_bell() {
        assert!(validate_no_control_chars("test\x07ref", "--workspace").is_err());
    }

    #[test]
    fn control_chars_rejects_escape() {
        assert!(validate_no_control_chars("\x1b[31mred", "--config").is_err());
    }

    #[test]
    fn control_chars_rejects_carriage_return() {
        assert!(validate_no_control_chars("main\rinjected", "--changed-since").is_err());
    }

    #[test]
    fn control_chars_allows_normal_text() {
        assert!(validate_no_control_chars("main", "--changed-since").is_ok());
    }

    #[test]
    fn control_chars_allows_newline() {
        assert!(validate_no_control_chars("line1\nline2", "--config").is_ok());
    }

    #[test]
    fn control_chars_allows_tab() {
        assert!(validate_no_control_chars("col1\tcol2", "--config").is_ok());
    }

    #[test]
    fn control_chars_allows_empty_string() {
        assert!(validate_no_control_chars("", "--workspace").is_ok());
    }

    #[test]
    fn control_chars_allows_unicode() {
        assert!(validate_no_control_chars("my-package-日本語", "--workspace").is_ok());
    }

    #[test]
    fn control_chars_allows_paths_with_dots_and_slashes() {
        assert!(validate_no_control_chars("./path/to/config.toml", "--config").is_ok());
    }

    // ── validate_git_ref ────────────────────────────────────────────

    #[test]
    fn git_ref_allows_reflog_timestamp() {
        assert_eq!(
            validate_git_ref("HEAD@{2025-01-01}").unwrap(),
            "HEAD@{2025-01-01}"
        );
    }

    #[test]
    fn git_ref_allows_reflog_relative_date() {
        assert_eq!(
            validate_git_ref("HEAD@{1 week ago}").unwrap(),
            "HEAD@{1 week ago}"
        );
    }

    #[test]
    fn git_ref_rejects_unclosed_brace() {
        let result = validate_git_ref("HEAD@{");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("unclosed"),
            "Error should mention unclosed brace, got: {err}"
        );
    }

    #[test]
    fn git_ref_rejects_colon_outside_braces() {
        let result = validate_git_ref("HEAD:file.txt");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("disallowed character"),
            "Error should mention disallowed character, got: {err}"
        );
        assert!(err.contains(':'), "Error should mention the colon, got: {err}");
    }

    #[test]
    fn git_ref_rejects_space_outside_braces() {
        let result = validate_git_ref("some ref");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("disallowed character"),
            "Error should mention disallowed character, got: {err}"
        );
    }

    #[test]
    fn git_ref_allows_reflog_index() {
        assert_eq!(
            validate_git_ref("origin/main@{0}").unwrap(),
            "origin/main@{0}"
        );
    }

    #[test]
    fn git_ref_allows_simple_branch_names() {
        assert_eq!(validate_git_ref("main").unwrap(), "main");
        assert_eq!(
            validate_git_ref("feature/my-branch").unwrap(),
            "feature/my-branch"
        );
    }

    #[test]
    fn git_ref_allows_head_tilde_caret() {
        assert_eq!(validate_git_ref("HEAD~3").unwrap(), "HEAD~3");
        assert_eq!(validate_git_ref("HEAD^2").unwrap(), "HEAD^2");
    }

    #[test]
    fn git_ref_allows_commit_sha() {
        assert_eq!(
            validate_git_ref("abc123def456").unwrap(),
            "abc123def456"
        );
    }

    #[test]
    fn git_ref_rejects_empty() {
        let result = validate_git_ref("");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn git_ref_rejects_leading_dash() {
        let result = validate_git_ref("--evil-flag");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("start with '-'"));
    }

    #[test]
    fn git_ref_allows_multiple_braces_segments() {
        // e.g. HEAD@{0}~3 is valid
        assert!(validate_git_ref("HEAD@{0}~3").is_ok());
    }

    #[test]
    fn git_ref_allows_space_in_complex_reflog() {
        // HEAD@{3 days ago} — spaces allowed inside braces
        assert_eq!(
            validate_git_ref("HEAD@{3 days ago}").unwrap(),
            "HEAD@{3 days ago}"
        );
    }

    #[test]
    fn git_ref_rejects_semicolon() {
        let result = validate_git_ref("main;rm -rf /");
        assert!(result.is_err());
    }

    #[test]
    fn git_ref_rejects_backtick() {
        let result = validate_git_ref("main`whoami`");
        assert!(result.is_err());
    }

    #[test]
    fn git_ref_rejects_dollar_sign() {
        let result = validate_git_ref("main$HOME");
        assert!(result.is_err());
    }
}
