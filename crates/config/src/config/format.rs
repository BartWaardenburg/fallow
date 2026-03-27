/// Output format for results.
///
/// This is CLI-only (via `--format` flag), not stored in config files.
#[derive(Debug, Default, Clone)]
pub enum OutputFormat {
    /// Human-readable terminal output with source context.
    #[default]
    Human,
    /// Machine-readable JSON.
    Json,
    /// SARIF format for GitHub Code Scanning.
    Sarif,
    /// One issue per line (grep-friendly).
    Compact,
    /// Markdown for PR comments.
    Markdown,
    /// `CodeClimate` JSON for GitLab Code Quality.
    CodeClimate,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_format_default_is_human() {
        let format = OutputFormat::default();
        assert!(matches!(format, OutputFormat::Human));
    }
}
