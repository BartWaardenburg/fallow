use std::path::PathBuf;

/// Errors that can occur during analysis.
#[derive(Debug)]
pub enum FallowError {
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

impl std::fmt::Display for FallowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileReadError { path, source } => {
                write!(f, "Failed to read {}: {source}", path.display())
            }
            Self::ParseError { path, errors } => {
                write!(
                    f,
                    "Parse errors in {} ({} errors)",
                    path.display(),
                    errors.len()
                )
            }
            Self::ResolveError {
                from_file,
                specifier,
            } => {
                write!(
                    f,
                    "Cannot resolve '{}' from {}",
                    specifier,
                    from_file.display()
                )
            }
            Self::ConfigError { message } => {
                write!(f, "Configuration error: {message}")
            }
        }
    }
}

impl std::error::Error for FallowError {}

/// Collection of non-fatal warnings during analysis.
#[derive(Debug, Default)]
pub struct AnalysisWarnings {
    pub parse_errors: Vec<FallowError>,
    pub resolve_errors: Vec<FallowError>,
    pub file_read_errors: Vec<FallowError>,
}

impl AnalysisWarnings {
    pub fn total(&self) -> usize {
        self.parse_errors.len() + self.resolve_errors.len() + self.file_read_errors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallow_error_display_file_read() {
        let err = FallowError::FileReadError {
            path: PathBuf::from("test.ts"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };
        let msg = format!("{err}");
        assert!(msg.contains("test.ts"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn fallow_error_display_parse() {
        let err = FallowError::ParseError {
            path: PathBuf::from("bad.ts"),
            errors: vec![
                "unexpected token".to_string(),
                "missing semicolon".to_string(),
            ],
        };
        let msg = format!("{err}");
        assert!(msg.contains("bad.ts"));
        assert!(msg.contains("2 errors"));
    }

    #[test]
    fn fallow_error_display_resolve() {
        let err = FallowError::ResolveError {
            from_file: PathBuf::from("src/index.ts"),
            specifier: "./missing".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("./missing"));
        assert!(msg.contains("src/index.ts"));
    }

    #[test]
    fn fallow_error_display_config() {
        let err = FallowError::ConfigError {
            message: "invalid TOML".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("invalid TOML"));
    }

    #[test]
    fn analysis_warnings_empty() {
        let warnings = AnalysisWarnings::default();
        assert!(warnings.is_empty());
        assert_eq!(warnings.total(), 0);
    }

    #[test]
    fn analysis_warnings_total() {
        let mut warnings = AnalysisWarnings::default();
        warnings.parse_errors.push(FallowError::ParseError {
            path: PathBuf::from("a.ts"),
            errors: vec![],
        });
        warnings.resolve_errors.push(FallowError::ResolveError {
            from_file: PathBuf::from("b.ts"),
            specifier: "./c".to_string(),
        });
        warnings.file_read_errors.push(FallowError::FileReadError {
            path: PathBuf::from("d.ts"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        });
        assert_eq!(warnings.total(), 3);
        assert!(!warnings.is_empty());
    }
}
