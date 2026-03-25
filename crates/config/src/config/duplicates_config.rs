use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const fn default_true() -> bool {
    true
}

const fn default_min_tokens() -> usize {
    50
}

const fn default_min_lines() -> usize {
    5
}

/// Configuration for code duplication detection.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DuplicatesConfig {
    /// Whether duplication detection is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Detection mode: strict, mild, weak, or semantic.
    #[serde(default)]
    pub mode: DetectionMode,

    /// Minimum number of tokens for a clone.
    #[serde(default = "default_min_tokens")]
    pub min_tokens: usize,

    /// Minimum number of lines for a clone.
    #[serde(default = "default_min_lines")]
    pub min_lines: usize,

    /// Maximum allowed duplication percentage (0 = no limit).
    #[serde(default)]
    pub threshold: f64,

    /// Additional ignore patterns for duplication analysis.
    #[serde(default)]
    pub ignore: Vec<String>,

    /// Only report cross-directory duplicates.
    #[serde(default)]
    pub skip_local: bool,

    /// Enable cross-language clone detection by stripping type annotations.
    ///
    /// When enabled, TypeScript type annotations (parameter types, return types,
    /// generics, interfaces, type aliases) are stripped from the token stream,
    /// allowing detection of clones between `.ts` and `.js` files.
    #[serde(default)]
    pub cross_language: bool,

    /// Fine-grained normalization overrides on top of the detection mode.
    #[serde(default)]
    pub normalization: NormalizationConfig,
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
            cross_language: false,
            normalization: NormalizationConfig::default(),
        }
    }
}

/// Fine-grained normalization overrides.
///
/// Each option, when set to `Some(true)`, forces that normalization regardless of
/// the detection mode. When set to `Some(false)`, it forces preservation. When
/// `None`, the detection mode's default behavior applies.
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NormalizationConfig {
    /// Blind all identifiers (variable names, function names, etc.) to the same hash.
    /// Default in `semantic` mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_identifiers: Option<bool>,

    /// Blind string literal values to the same hash.
    /// Default in `weak` and `semantic` modes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_string_values: Option<bool>,

    /// Blind numeric literal values to the same hash.
    /// Default in `semantic` mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_numeric_values: Option<bool>,
}

/// Resolved normalization flags: mode defaults merged with user overrides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedNormalization {
    pub ignore_identifiers: bool,
    pub ignore_string_values: bool,
    pub ignore_numeric_values: bool,
}

impl ResolvedNormalization {
    /// Resolve normalization from a detection mode and optional overrides.
    pub fn resolve(mode: DetectionMode, overrides: &NormalizationConfig) -> Self {
        let (default_ids, default_strings, default_numbers) = match mode {
            DetectionMode::Strict | DetectionMode::Mild => (false, false, false),
            DetectionMode::Weak => (false, true, false),
            DetectionMode::Semantic => (true, true, true),
        };

        Self {
            ignore_identifiers: overrides.ignore_identifiers.unwrap_or(default_ids),
            ignore_string_values: overrides.ignore_string_values.unwrap_or(default_strings),
            ignore_numeric_values: overrides.ignore_numeric_values.unwrap_or(default_numbers),
        }
    }
}

/// Detection mode controlling how aggressively tokens are normalized.
///
/// Since fallow uses AST-based tokenization (not lexer-based), whitespace and
/// comments are inherently absent from the token stream. The `Strict` and `Mild`
/// modes are currently equivalent. `Weak` mode additionally blinds string
/// literals. `Semantic` mode blinds all identifiers and literal values for
/// Type-2 (renamed variable) clone detection.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DetectionMode {
    /// All tokens preserved including identifier names and literal values (Type-1 only).
    Strict,
    /// Default mode -- equivalent to strict for AST-based tokenization.
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── DuplicatesConfig defaults ────────────────────────────────────

    #[test]
    fn duplicates_config_defaults() {
        let config = DuplicatesConfig::default();
        assert!(config.enabled);
        assert_eq!(config.mode, DetectionMode::Mild);
        assert_eq!(config.min_tokens, 50);
        assert_eq!(config.min_lines, 5);
        assert_eq!(config.threshold, 0.0);
        assert!(config.ignore.is_empty());
        assert!(!config.skip_local);
        assert!(!config.cross_language);
    }

    // ── DetectionMode FromStr ────────────────────────────────────────

    #[test]
    fn detection_mode_from_str_all_variants() {
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
    }

    #[test]
    fn detection_mode_from_str_case_insensitive() {
        assert_eq!(
            "STRICT".parse::<DetectionMode>().unwrap(),
            DetectionMode::Strict
        );
        assert_eq!(
            "Weak".parse::<DetectionMode>().unwrap(),
            DetectionMode::Weak
        );
        assert_eq!(
            "SEMANTIC".parse::<DetectionMode>().unwrap(),
            DetectionMode::Semantic
        );
    }

    #[test]
    fn detection_mode_from_str_unknown() {
        let err = "foobar".parse::<DetectionMode>().unwrap_err();
        assert!(err.contains("unknown detection mode"));
        assert!(err.contains("foobar"));
    }

    // ── DetectionMode Display ────────────────────────────────────────

    #[test]
    fn detection_mode_display() {
        assert_eq!(DetectionMode::Strict.to_string(), "strict");
        assert_eq!(DetectionMode::Mild.to_string(), "mild");
        assert_eq!(DetectionMode::Weak.to_string(), "weak");
        assert_eq!(DetectionMode::Semantic.to_string(), "semantic");
    }

    // ── ResolvedNormalization::resolve ────────────────────────────────

    #[test]
    fn resolve_strict_mode_all_false() {
        let resolved =
            ResolvedNormalization::resolve(DetectionMode::Strict, &NormalizationConfig::default());
        assert!(!resolved.ignore_identifiers);
        assert!(!resolved.ignore_string_values);
        assert!(!resolved.ignore_numeric_values);
    }

    #[test]
    fn resolve_mild_mode_all_false() {
        let resolved =
            ResolvedNormalization::resolve(DetectionMode::Mild, &NormalizationConfig::default());
        assert!(!resolved.ignore_identifiers);
        assert!(!resolved.ignore_string_values);
        assert!(!resolved.ignore_numeric_values);
    }

    #[test]
    fn resolve_weak_mode_only_strings_true() {
        let resolved =
            ResolvedNormalization::resolve(DetectionMode::Weak, &NormalizationConfig::default());
        assert!(!resolved.ignore_identifiers);
        assert!(resolved.ignore_string_values);
        assert!(!resolved.ignore_numeric_values);
    }

    #[test]
    fn resolve_semantic_mode_all_true() {
        let resolved = ResolvedNormalization::resolve(
            DetectionMode::Semantic,
            &NormalizationConfig::default(),
        );
        assert!(resolved.ignore_identifiers);
        assert!(resolved.ignore_string_values);
        assert!(resolved.ignore_numeric_values);
    }

    #[test]
    fn resolve_override_forces_true() {
        // Strict mode defaults to all false, but override forces ignore_identifiers to true
        let overrides = NormalizationConfig {
            ignore_identifiers: Some(true),
            ignore_string_values: None,
            ignore_numeric_values: None,
        };
        let resolved = ResolvedNormalization::resolve(DetectionMode::Strict, &overrides);
        assert!(resolved.ignore_identifiers);
        assert!(!resolved.ignore_string_values);
        assert!(!resolved.ignore_numeric_values);
    }

    #[test]
    fn resolve_override_forces_false() {
        // Semantic mode defaults to all true, but override forces ignore_identifiers to false
        let overrides = NormalizationConfig {
            ignore_identifiers: Some(false),
            ignore_string_values: Some(false),
            ignore_numeric_values: None,
        };
        let resolved = ResolvedNormalization::resolve(DetectionMode::Semantic, &overrides);
        assert!(!resolved.ignore_identifiers);
        assert!(!resolved.ignore_string_values);
        assert!(resolved.ignore_numeric_values); // not overridden
    }

    #[test]
    fn resolve_all_overrides_on_weak() {
        let overrides = NormalizationConfig {
            ignore_identifiers: Some(true),
            ignore_string_values: Some(false), // override weak default (true -> false)
            ignore_numeric_values: Some(true),
        };
        let resolved = ResolvedNormalization::resolve(DetectionMode::Weak, &overrides);
        assert!(resolved.ignore_identifiers);
        assert!(!resolved.ignore_string_values); // overridden from true to false
        assert!(resolved.ignore_numeric_values);
    }

    // ── DuplicatesConfig deserialization ──────────────────────────────

    #[test]
    fn duplicates_config_json_all_fields() {
        let json = r#"{
            "enabled": false,
            "mode": "semantic",
            "minTokens": 100,
            "minLines": 10,
            "threshold": 5.0,
            "ignore": ["**/vendor/**"],
            "skipLocal": true,
            "crossLanguage": true
        }"#;
        let config: DuplicatesConfig = serde_json::from_str(json).unwrap();
        assert!(!config.enabled);
        assert_eq!(config.mode, DetectionMode::Semantic);
        assert_eq!(config.min_tokens, 100);
        assert_eq!(config.min_lines, 10);
        assert_eq!(config.threshold, 5.0);
        assert_eq!(config.ignore, vec!["**/vendor/**"]);
        assert!(config.skip_local);
        assert!(config.cross_language);
    }

    #[test]
    fn duplicates_config_json_partial_uses_defaults() {
        let json = r#"{"mode": "weak"}"#;
        let config: DuplicatesConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled); // default
        assert_eq!(config.mode, DetectionMode::Weak);
        assert_eq!(config.min_tokens, 50); // default
        assert_eq!(config.min_lines, 5); // default
    }

    #[test]
    fn normalization_config_json_overrides() {
        let json = r#"{
            "ignoreIdentifiers": true,
            "ignoreStringValues": false
        }"#;
        let config: NormalizationConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.ignore_identifiers, Some(true));
        assert_eq!(config.ignore_string_values, Some(false));
        assert_eq!(config.ignore_numeric_values, None);
    }
}
