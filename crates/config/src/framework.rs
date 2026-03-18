use serde::{Deserialize, Serialize};

/// Declarative framework detection and entry point configuration.
///
/// Users can define custom framework presets via `fallow.toml` to add
/// project-specific entry points, always-used files, and used export rules.
/// Built-in framework support is provided by the plugin system in fallow-core.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FrameworkPreset {
    /// Unique name for this framework.
    pub name: String,

    /// How to detect if this framework is in use.
    #[serde(default)]
    pub detection: Option<FrameworkDetection>,

    /// Glob patterns for files that are entry points.
    #[serde(default)]
    pub entry_points: Vec<FrameworkEntryPattern>,

    /// Files that are always considered "used".
    #[serde(default)]
    pub always_used: Vec<String>,

    /// Exports that are always considered used in matching files.
    #[serde(default)]
    pub used_exports: Vec<FrameworkUsedExport>,
}

/// How to detect if a framework is in use.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FrameworkDetection {
    /// Framework detected if this package is in dependencies.
    Dependency { package: String },
    /// Framework detected if this file pattern matches.
    FileExists { pattern: String },
    /// All conditions must be true.
    All { conditions: Vec<FrameworkDetection> },
    /// Any condition must be true.
    Any { conditions: Vec<FrameworkDetection> },
}

/// Entry point pattern from a framework.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FrameworkEntryPattern {
    /// Glob pattern for entry point files.
    pub pattern: String,
}

/// Exports considered used for files matching a pattern.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FrameworkUsedExport {
    /// Files matching this glob pattern.
    pub file_pattern: String,
    /// These exports are always considered used.
    pub exports: Vec<String>,
}

/// Resolved framework rule (after loading custom presets).
#[derive(Debug, Clone)]
pub struct FrameworkRule {
    pub name: String,
    pub detection: Option<FrameworkDetection>,
    pub entry_points: Vec<FrameworkEntryPattern>,
    pub always_used: Vec<String>,
    pub used_exports: Vec<FrameworkUsedExport>,
}

impl From<FrameworkPreset> for FrameworkRule {
    fn from(preset: FrameworkPreset) -> Self {
        Self {
            name: preset.name,
            detection: preset.detection,
            entry_points: preset.entry_points,
            always_used: preset.always_used,
            used_exports: preset.used_exports,
        }
    }
}

/// Load user-defined framework rules from fallow.toml.
///
/// Built-in framework support is handled by the plugin system in fallow-core.
/// This function only processes custom user-defined presets.
pub fn resolve_framework_rules(custom: &[FrameworkPreset]) -> Vec<FrameworkRule> {
    custom
        .iter()
        .map(|p| FrameworkRule::from(p.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_framework_rules_empty() {
        let rules = resolve_framework_rules(&[]);
        assert!(rules.is_empty());
    }

    #[test]
    fn resolve_framework_rules_with_custom() {
        let custom = vec![FrameworkPreset {
            name: "custom".to_string(),
            detection: None,
            entry_points: vec![FrameworkEntryPattern {
                pattern: "src/custom/**/*.ts".to_string(),
            }],
            always_used: vec![],
            used_exports: vec![],
        }];
        let rules = resolve_framework_rules(&custom);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name, "custom");
    }

    #[test]
    fn framework_preset_to_rule() {
        let preset = FrameworkPreset {
            name: "test".to_string(),
            detection: Some(FrameworkDetection::Dependency {
                package: "test-pkg".to_string(),
            }),
            entry_points: vec![FrameworkEntryPattern {
                pattern: "src/**/*.test.ts".to_string(),
            }],
            always_used: vec!["setup.ts".to_string()],
            used_exports: vec![FrameworkUsedExport {
                file_pattern: "src/**/*.test.ts".to_string(),
                exports: vec!["default".to_string()],
            }],
        };
        let rule: FrameworkRule = preset.into();
        assert_eq!(rule.name, "test");
        assert!(rule.detection.is_some());
        assert_eq!(rule.entry_points.len(), 1);
        assert_eq!(rule.always_used, vec!["setup.ts"]);
        assert_eq!(rule.used_exports.len(), 1);
    }

    #[test]
    fn framework_detection_deserialize_dependency() {
        let json = r#"{"type": "dependency", "package": "next"}"#;
        let detection: FrameworkDetection = serde_json::from_str(json).unwrap();
        assert!(
            matches!(detection, FrameworkDetection::Dependency { package } if package == "next")
        );
    }

    #[test]
    fn framework_detection_deserialize_file_exists() {
        let json = r#"{"type": "file_exists", "pattern": "tsconfig.json"}"#;
        let detection: FrameworkDetection = serde_json::from_str(json).unwrap();
        assert!(
            matches!(detection, FrameworkDetection::FileExists { pattern } if pattern == "tsconfig.json")
        );
    }

    #[test]
    fn framework_detection_deserialize_all() {
        let json = r#"{"type": "all", "conditions": [{"type": "dependency", "package": "a"}, {"type": "dependency", "package": "b"}]}"#;
        let detection: FrameworkDetection = serde_json::from_str(json).unwrap();
        assert!(
            matches!(detection, FrameworkDetection::All { conditions } if conditions.len() == 2)
        );
    }

    #[test]
    fn framework_detection_deserialize_any() {
        let json = r#"{"type": "any", "conditions": [{"type": "dependency", "package": "a"}]}"#;
        let detection: FrameworkDetection = serde_json::from_str(json).unwrap();
        assert!(
            matches!(detection, FrameworkDetection::Any { conditions } if conditions.len() == 1)
        );
    }
}
