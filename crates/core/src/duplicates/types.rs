use std::path::PathBuf;

use serde::Serialize;

use fallow_types::serde_path;

// Re-export the canonical types from fallow-config.
pub use fallow_config::{DetectionMode, DuplicatesConfig};

/// A single instance of duplicated code at a specific location.
#[derive(Debug, Clone, Serialize)]
pub struct CloneInstance {
    /// Path to the file containing this clone instance.
    #[serde(serialize_with = "serde_path::serialize")]
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

/// The kind of refactoring suggested for a clone family.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum RefactoringKind {
    /// Extract a shared function/utility.
    ExtractFunction,
    /// Extract a shared module.
    ExtractModule,
}

/// A refactoring suggestion for a clone family.
#[derive(Debug, Clone, Serialize)]
pub struct RefactoringSuggestion {
    /// What kind of refactoring is suggested.
    pub kind: RefactoringKind,
    /// Human-readable description of the suggestion.
    pub description: String,
    /// Estimated lines that could be eliminated.
    pub estimated_savings: usize,
}

/// A clone family: a set of clone groups that share the same file set.
///
/// When multiple clone groups are all duplicated between the same set of files,
/// they form a family — indicating a deeper structural relationship that should
/// be refactored together rather than group-by-group.
#[derive(Debug, Clone, Serialize)]
pub struct CloneFamily {
    /// The files involved in this family (sorted for stable output).
    #[serde(serialize_with = "serde_path::serialize_vec")]
    pub files: Vec<PathBuf>,
    /// Clone groups belonging to this family.
    pub groups: Vec<CloneGroup>,
    /// Total number of duplicated lines across all groups.
    pub total_duplicated_lines: usize,
    /// Total number of duplicated tokens across all groups.
    pub total_duplicated_tokens: usize,
    /// Refactoring suggestions for this family.
    pub suggestions: Vec<RefactoringSuggestion>,
}

/// Overall duplication analysis report.
#[derive(Debug, Clone, Default, Serialize)]
pub struct DuplicationReport {
    /// All detected clone groups.
    pub clone_groups: Vec<CloneGroup>,
    /// Clone families: groups of clone groups sharing the same file set.
    pub clone_families: Vec<CloneFamily>,
    /// Aggregate statistics.
    pub stats: DuplicationStats,
}

/// Aggregate duplication statistics.
#[derive(Debug, Clone, Default, Serialize)]
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
        assert!((config.threshold - 0.0).abs() < f64::EPSILON);
        assert!(config.ignore.is_empty());
        assert!(!config.skip_local);
        assert!(!config.cross_language);
        assert!(config.normalization.ignore_identifiers.is_none());
        assert!(config.normalization.ignore_string_values.is_none());
        assert!(config.normalization.ignore_numeric_values.is_none());
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
minTokens = 30
minLines = 3
threshold = 5.0
skipLocal = true
ignore = ["**/*.generated.ts"]
"#;
        let config: DuplicatesConfig = toml::from_str(toml_str).unwrap();
        assert!(config.enabled);
        assert_eq!(config.mode, DetectionMode::Semantic);
        assert_eq!(config.min_tokens, 30);
        assert_eq!(config.min_lines, 3);
        assert!((config.threshold - 5.0).abs() < f64::EPSILON);
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

    #[test]
    fn config_deserialize_cross_language() {
        let toml_str = r"crossLanguage = true";
        let config: DuplicatesConfig = toml::from_str(toml_str).unwrap();
        assert!(config.cross_language);
    }

    #[test]
    fn config_deserialize_normalization_overrides() {
        let toml_str = r"
[normalization]
ignoreIdentifiers = true
ignoreStringValues = false
";
        let config: DuplicatesConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.normalization.ignore_identifiers, Some(true));
        assert_eq!(config.normalization.ignore_string_values, Some(false));
        assert!(config.normalization.ignore_numeric_values.is_none());
    }

    #[test]
    fn config_deserialize_json_normalization() {
        let json_str = r#"{
            "crossLanguage": true,
            "normalization": {
                "ignoreIdentifiers": true,
                "ignoreNumericValues": true
            }
        }"#;
        let config: DuplicatesConfig = serde_json::from_str(json_str).unwrap();
        assert!(config.cross_language);
        assert_eq!(config.normalization.ignore_identifiers, Some(true));
        assert_eq!(config.normalization.ignore_numeric_values, Some(true));
        assert!(config.normalization.ignore_string_values.is_none());
    }
}
