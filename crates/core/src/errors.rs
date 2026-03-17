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
