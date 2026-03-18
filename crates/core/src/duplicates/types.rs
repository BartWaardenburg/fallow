use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A single instance of duplicated code at a specific location.
#[derive(Debug, Clone, Serialize)]
pub struct CloneInstance {
    /// Path to the file containing this clone instance.
    pub file: PathBuf,
    /// 1-based start line of the clone.
    pub start_line: usize,
    /// 1-based end line of the clone.
    pub end_line: usize,
    /// 0-based start column.
    pub start_col: usize,
    /// 0-based end column.
    pub end_col: usize,
    /// The actual source code fragment.
    pub fragment: String,
}

/// A group of code clones -- the same (or normalized-equivalent) code appearing in multiple places.
#[derive(Debug, Clone, Serialize)]
pub struct CloneGroup {
    /// All instances where this duplicated code appears.
    pub instances: Vec<CloneInstance>,
    /// Number of tokens in the duplicated block.
    pub token_count: usize,
    /// Number of lines in the duplicated block.
    pub line_count: usize,
}

/// Overall duplication analysis report.
#[derive(Debug, Clone, Serialize)]
pub struct DuplicationReport {
    /// All detected clone groups.
    pub clone_groups: Vec<CloneGroup>,
    /// Aggregate statistics.
    pub stats: DuplicationStats,
}

/// Aggregate duplication statistics.
#[derive(Debug, Clone, Serialize)]
pub struct DuplicationStats {
    /// Total files analyzed.
    pub total_files: usize,
    /// Files containing at least one clone instance.
    pub files_with_clones: usize,
    /// Total lines across all analyzed files.
    pub total_lines: usize,
    /// Lines that are part of at least one clone.
    pub duplicated_lines: usize,
    /// Total tokens across all analyzed files.
    pub total_tokens: usize,
    /// Tokens that are part of at least one clone.
    pub duplicated_tokens: usize,
    /// Number of clone groups found.
    pub clone_groups: usize,
    /// Total clone instances across all groups.
    pub clone_instances: usize,
    /// Percentage of duplicated lines (0.0 - 100.0).
    pub duplication_percentage: f64,
}

/// Detection mode controlling how aggressively tokens are normalized.
///
/// Since fallow uses AST-based tokenization (not lexer-based), whitespace and
/// comments are inherently absent from the token stream. The `Strict` and `Mild`
/// modes are currently equivalent. `Weak` mode additionally blinds string
/// literals. `Semantic` mode blinds all identifiers and literal values for
/// Type-2 (renamed variable) clone detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DetectionMode {
    /// All tokens preserved including identifier names and literal values (Type-1 only).
    Strict,
    /// Default mode — equivalent to strict for AST-based tokenization.
    #[default]
    Mild,
    /// Blind string literal values (structure-preserving).
    Weak,
    /// Blind all identifiers and literal values for structural (Type-2) detection.
    Semantic,
}

impl std::fmt::Display for DetectionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Strict => write!(f, "strict"),
            Self::Mild => write!(f, "mild"),
            Self::Weak => write!(f, "weak"),
            Self::Semantic => write!(f, "semantic"),
        }
    }
}

impl std::str::FromStr for DetectionMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "strict" => Ok(Self::Strict),
            "mild" => Ok(Self::Mild),
            "weak" => Ok(Self::Weak),
            "semantic" => Ok(Self::Semantic),
            other => Err(format!("unknown detection mode: '{other}'")),
        }
    }
}

/// Configuration for duplication detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicatesConfig {
    /// Whether duplication detection is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Detection mode controlling normalization aggressiveness.
    #[serde(default)]
    pub mode: DetectionMode,
    /// Minimum number of tokens for a block to be considered a clone.
    #[serde(default = "default_min_tokens")]
    pub min_tokens: usize,
    /// Minimum number of lines for a block to be considered a clone.
    #[serde(default = "default_min_lines")]
    pub min_lines: usize,
    /// Maximum allowed duplication percentage (0 = no limit).
    #[serde(default)]
    pub threshold: f64,
    /// Additional ignore patterns specific to duplication analysis.
    #[serde(default)]
    pub ignore: Vec<String>,
    /// Only report cross-directory duplicates.
    #[serde(default)]
    pub skip_local: bool,
}

impl Default for DuplicatesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: DetectionMode::default(),
            min_tokens: default_min_tokens(),
            min_lines: default_min_lines(),
            threshold: 0.0,
            ignore: vec![],
            skip_local: false,
        }
    }
}

const fn default_true() -> bool {
    true
}

const fn default_min_tokens() -> usize {
    50
}

const fn default_min_lines() -> usize {
    5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let config = DuplicatesConfig::default();
        assert!(config.enabled);
        assert_eq!(config.mode, DetectionMode::Mild);
        assert_eq!(config.min_tokens, 50);
        assert_eq!(config.min_lines, 5);
        assert_eq!(config.threshold, 0.0);
        assert!(config.ignore.is_empty());
        assert!(!config.skip_local);
    }

    #[test]
    fn detection_mode_display() {
        assert_eq!(DetectionMode::Strict.to_string(), "strict");
        assert_eq!(DetectionMode::Mild.to_string(), "mild");
        assert_eq!(DetectionMode::Weak.to_string(), "weak");
        assert_eq!(DetectionMode::Semantic.to_string(), "semantic");
    }

    #[test]
    fn detection_mode_from_str() {
        assert_eq!(
            "strict".parse::<DetectionMode>().unwrap(),
            DetectionMode::Strict
        );
        assert_eq!(
            "mild".parse::<DetectionMode>().unwrap(),
            DetectionMode::Mild
        );
        assert_eq!(
            "weak".parse::<DetectionMode>().unwrap(),
            DetectionMode::Weak
        );
        assert_eq!(
            "semantic".parse::<DetectionMode>().unwrap(),
            DetectionMode::Semantic
        );
        assert!("unknown".parse::<DetectionMode>().is_err());
    }

    #[test]
    fn detection_mode_default_is_mild() {
        assert_eq!(DetectionMode::default(), DetectionMode::Mild);
    }

    #[test]
    fn config_deserialize_toml() {
        let toml_str = r#"
enabled = true
mode = "semantic"
min_tokens = 30
min_lines = 3
threshold = 5.0
skip_local = true
ignore = ["**/*.generated.ts"]
"#;
        let config: DuplicatesConfig = toml::from_str(toml_str).unwrap();
        assert!(config.enabled);
        assert_eq!(config.mode, DetectionMode::Semantic);
        assert_eq!(config.min_tokens, 30);
        assert_eq!(config.min_lines, 3);
        assert_eq!(config.threshold, 5.0);
        assert!(config.skip_local);
        assert_eq!(config.ignore, vec!["**/*.generated.ts"]);
    }

    #[test]
    fn config_deserialize_defaults() {
        let toml_str = "";
        let config: DuplicatesConfig = toml::from_str(toml_str).unwrap();
        assert!(config.enabled);
        assert_eq!(config.mode, DetectionMode::Mild);
        assert_eq!(config.min_tokens, 50);
        assert_eq!(config.min_lines, 5);
    }
}
