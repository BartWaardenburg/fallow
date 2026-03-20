use std::path::PathBuf;

/// The underlying error kind, describing what went wrong.
#[derive(Debug)]
pub enum FallowErrorKind {
    /// Failed to read a source file.
    FileReadError {
        path: PathBuf,
        source: std::io::Error,
    },
    /// Failed to parse a source file (syntax errors).
    ParseError { path: PathBuf, errors: Vec<String> },
    /// Failed to resolve an import.
    ResolveError {
        from_file: PathBuf,
        specifier: String,
    },
    /// Configuration error.
    ConfigError { message: String },
}

/// Errors that can occur during analysis.
///
/// Wraps a [`FallowErrorKind`] with optional diagnostic metadata:
/// an error code, actionable help text, and additional context.
#[derive(Debug)]
pub struct FallowError {
    /// The underlying error kind (boxed to keep `Result<T, FallowError>` small).
    kind: Box<FallowErrorKind>,
    /// Optional error code (e.g. `"E001"`).
    code: Option<String>,
    /// Actionable suggestion for the user.
    help: Option<String>,
    /// Additional context about the error.
    context: Option<String>,
}

impl FallowError {
    /// Create a new `FallowError` from a kind.
    #[must_use]
    fn new(kind: FallowErrorKind) -> Self {
        Self {
            kind: Box::new(kind),
            code: None,
            help: None,
            context: None,
        }
    }

    /// Create a file-read error with default help text.
    pub fn file_read(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::new(FallowErrorKind::FileReadError {
            path: path.into(),
            source,
        })
        .with_code("E001")
        .with_help("Check file permissions and encoding")
    }

    /// Create a parse error with default help text.
    pub fn parse(path: impl Into<PathBuf>, errors: Vec<String>) -> Self {
        Self::new(FallowErrorKind::ParseError {
            path: path.into(),
            errors,
        })
        .with_code("E002")
        .with_help(
            "This may indicate unsupported syntax. Consider adding the file to the ignore list.",
        )
    }

    /// Create a resolve error with default help text.
    pub fn resolve(from_file: impl Into<PathBuf>, specifier: impl Into<String>) -> Self {
        Self::new(FallowErrorKind::ResolveError {
            from_file: from_file.into(),
            specifier: specifier.into(),
        })
        .with_code("E003")
        .with_help("Check that the module is installed and the import path is correct")
    }

    /// Create a config error with default error code.
    pub fn config(message: impl Into<String>) -> Self {
        Self::new(FallowErrorKind::ConfigError {
            message: message.into(),
        })
        .with_code("E004")
    }

    /// Attach an error code (e.g. `"E001"`).
    #[must_use]
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Attach actionable help text.
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Attach additional context about the error.
    #[must_use]
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Returns the error kind.
    #[must_use]
    pub fn kind(&self) -> &FallowErrorKind {
        &self.kind
    }

    /// Returns the error code, if set.
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// Returns the help text, if set.
    #[must_use]
    pub fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }

    /// Returns the context string, if set.
    #[must_use]
    pub fn context(&self) -> Option<&str> {
        self.context.as_deref()
    }
}

impl std::fmt::Display for FallowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Error code prefix: "error[E001]: ..." or "error: ..."
        if let Some(ref code) = self.code {
            write!(f, "error[{code}]: ")?;
        } else {
            write!(f, "error: ")?;
        }

        // Main message from the kind
        match &*self.kind {
            FallowErrorKind::FileReadError { path, source } => {
                write!(f, "Failed to read {}: {source}", path.display())?;
            }
            FallowErrorKind::ParseError { path, errors } => {
                write!(
                    f,
                    "Parse errors in {} ({} errors)",
                    path.display(),
                    errors.len()
                )?;
            }
            FallowErrorKind::ResolveError {
                from_file,
                specifier,
            } => {
                write!(
                    f,
                    "Cannot resolve '{}' from {}",
                    specifier,
                    from_file.display()
                )?;
            }
            FallowErrorKind::ConfigError { message } => {
                write!(f, "Configuration error: {message}")?;
            }
        }

        // Context line
        if let Some(ref context) = self.context {
            write!(f, "\n  context: {context}")?;
        }

        // Help line
        if let Some(ref help) = self.help {
            write!(f, "\n  help: {help}")?;
        }

        Ok(())
    }
}

impl std::error::Error for FallowError {}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Display tests (struct variants via constructors) ──────────

    #[test]
    fn fallow_error_display_file_read() {
        let err = FallowError::file_read(
            PathBuf::from("test.ts"),
            std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        );
        let msg = format!("{err}");
        assert!(msg.contains("test.ts"));
        assert!(msg.contains("not found"));
        assert!(msg.contains("E001"));
        assert!(msg.contains("help:"));
    }

    #[test]
    fn fallow_error_display_parse() {
        let err = FallowError::parse(
            PathBuf::from("bad.ts"),
            vec![
                "unexpected token".to_string(),
                "missing semicolon".to_string(),
            ],
        );
        let msg = format!("{err}");
        assert!(msg.contains("bad.ts"));
        assert!(msg.contains("2 errors"));
        assert!(msg.contains("E002"));
        assert!(msg.contains("help:"));
    }

    #[test]
    fn fallow_error_display_resolve() {
        let err = FallowError::resolve(PathBuf::from("src/index.ts"), "./missing");
        let msg = format!("{err}");
        assert!(msg.contains("./missing"));
        assert!(msg.contains("src/index.ts"));
        assert!(msg.contains("E003"));
    }

    #[test]
    fn fallow_error_display_config() {
        let err = FallowError::config("invalid TOML");
        let msg = format!("{err}");
        assert!(msg.contains("invalid TOML"));
        assert!(msg.contains("E004"));
    }

    // ── Builder method tests ─────────────────────────────────────

    #[test]
    fn with_help_appends_help_line() {
        let err =
            FallowError::config("bad config").with_help("Check the configuration file syntax");
        let msg = format!("{err}");
        assert!(msg.contains("help: Check the configuration file syntax"));
    }

    #[test]
    fn with_context_appends_context_line() {
        let err = FallowError::config("bad config").with_context("while loading fallow.toml");
        let msg = format!("{err}");
        assert!(msg.contains("context: while loading fallow.toml"));
    }

    #[test]
    fn with_code_overrides_default_code() {
        let err = FallowError::config("bad config").with_code("E999");
        let msg = format!("{err}");
        assert!(msg.contains("error[E999]:"));
        assert!(!msg.contains("E004"));
    }

    #[test]
    fn builder_methods_chain() {
        let err = FallowError::config("parse failure")
            .with_code("E100")
            .with_help("Try running `fallow init`")
            .with_context("in fallow.jsonc at line 5");
        let msg = format!("{err}");
        assert!(msg.contains("error[E100]:"));
        assert!(msg.contains("parse failure"));
        assert!(msg.contains("context: in fallow.jsonc at line 5"));
        assert!(msg.contains("help: Try running `fallow init`"));
    }

    #[test]
    fn error_without_code_shows_plain_prefix() {
        let err = FallowError::new(FallowErrorKind::ConfigError {
            message: "test".into(),
        });
        let msg = format!("{err}");
        assert!(msg.starts_with("error: "));
        assert!(!msg.contains('['));
    }

    // ── Accessor tests ───────────────────────────────────────────

    #[test]
    fn accessors_return_expected_values() {
        let err = FallowError::file_read(
            "a.ts",
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        )
        .with_context("ctx");

        assert_eq!(err.code(), Some("E001"));
        assert!(err.help().is_some());
        assert_eq!(err.context(), Some("ctx"));
        assert!(matches!(err.kind(), FallowErrorKind::FileReadError { .. }));
    }

    #[test]
    fn accessors_none_when_unset() {
        let err = FallowError::new(FallowErrorKind::ConfigError {
            message: "x".into(),
        });
        assert!(err.code().is_none());
        assert!(err.help().is_none());
        assert!(err.context().is_none());
    }

    // ── Display format tests ─────────────────────────────────────

    #[test]
    fn context_appears_before_help() {
        let err = FallowError::config("oops")
            .with_context("loading config")
            .with_help("fix it");
        let msg = format!("{err}");
        let ctx_pos = msg.find("context:").expect("context present");
        let help_pos = msg.find("help:").expect("help present");
        assert!(ctx_pos < help_pos, "context should appear before help");
    }

    #[test]
    fn file_read_default_help_mentions_permissions() {
        let err = FallowError::file_read(
            "x.ts",
            std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
        );
        assert!(err.help().unwrap().contains("permissions"));
    }

    #[test]
    fn parse_default_help_mentions_ignore() {
        let err = FallowError::parse("x.ts", vec!["err".into()]);
        assert!(err.help().unwrap().contains("ignore"));
    }

    #[test]
    fn resolve_default_help_mentions_installed() {
        let err = FallowError::resolve("a.ts", "./b");
        assert!(err.help().unwrap().contains("installed"));
    }
}
