use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Severity level for rules.
///
/// Controls whether an issue type causes CI failure (`error`), is reported
/// without failing (`warn`), or is suppressed entirely (`off`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Report and fail CI (non-zero exit code).
    #[default]
    Error,
    /// Report but don't fail CI.
    Warn,
    /// Don't detect or report.
    Off,
}

impl Severity {
    /// Default value for fields that should default to `Warn` instead of `Error`.
    const fn default_warn() -> Self {
        Self::Warn
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warn => write!(f, "warn"),
            Self::Off => write!(f, "off"),
        }
    }
}

impl std::str::FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "error" => Ok(Self::Error),
            "warn" | "warning" => Ok(Self::Warn),
            "off" | "none" => Ok(Self::Off),
            other => Err(format!(
                "unknown severity: '{other}' (expected error, warn, or off)"
            )),
        }
    }
}

/// Per-issue-type severity configuration.
///
/// Controls which issue types cause CI failure, are reported as warnings,
/// or are suppressed entirely. All fields default to `Severity::Error`.
///
/// Rule names use kebab-case in config files (e.g., `"unused-files": "error"`).
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct RulesConfig {
    #[serde(default)]
    pub unused_files: Severity,
    #[serde(default)]
    pub unused_exports: Severity,
    #[serde(default)]
    pub unused_types: Severity,
    #[serde(default)]
    pub unused_dependencies: Severity,
    #[serde(default)]
    pub unused_dev_dependencies: Severity,
    #[serde(default)]
    pub unused_optional_dependencies: Severity,
    #[serde(default)]
    pub unused_enum_members: Severity,
    #[serde(default)]
    pub unused_class_members: Severity,
    #[serde(default)]
    pub unresolved_imports: Severity,
    #[serde(default)]
    pub unlisted_dependencies: Severity,
    #[serde(default)]
    pub duplicate_exports: Severity,
    #[serde(default = "Severity::default_warn")]
    pub type_only_dependencies: Severity,
    #[serde(default)]
    pub circular_dependencies: Severity,
}

impl Default for RulesConfig {
    fn default() -> Self {
        Self {
            unused_files: Severity::Error,
            unused_exports: Severity::Error,
            unused_types: Severity::Error,
            unused_dependencies: Severity::Error,
            unused_dev_dependencies: Severity::Error,
            unused_optional_dependencies: Severity::Error,
            unused_enum_members: Severity::Error,
            unused_class_members: Severity::Error,
            unresolved_imports: Severity::Error,
            unlisted_dependencies: Severity::Error,
            duplicate_exports: Severity::Error,
            type_only_dependencies: Severity::Warn,
            circular_dependencies: Severity::Error,
        }
    }
}

impl RulesConfig {
    /// Apply a partial rules config on top. Only `Some` fields override.
    pub const fn apply_partial(&mut self, partial: &PartialRulesConfig) {
        if let Some(s) = partial.unused_files {
            self.unused_files = s;
        }
        if let Some(s) = partial.unused_exports {
            self.unused_exports = s;
        }
        if let Some(s) = partial.unused_types {
            self.unused_types = s;
        }
        if let Some(s) = partial.unused_dependencies {
            self.unused_dependencies = s;
        }
        if let Some(s) = partial.unused_dev_dependencies {
            self.unused_dev_dependencies = s;
        }
        if let Some(s) = partial.unused_optional_dependencies {
            self.unused_optional_dependencies = s;
        }
        if let Some(s) = partial.unused_enum_members {
            self.unused_enum_members = s;
        }
        if let Some(s) = partial.unused_class_members {
            self.unused_class_members = s;
        }
        if let Some(s) = partial.unresolved_imports {
            self.unresolved_imports = s;
        }
        if let Some(s) = partial.unlisted_dependencies {
            self.unlisted_dependencies = s;
        }
        if let Some(s) = partial.duplicate_exports {
            self.duplicate_exports = s;
        }
        if let Some(s) = partial.type_only_dependencies {
            self.type_only_dependencies = s;
        }
        if let Some(s) = partial.circular_dependencies {
            self.circular_dependencies = s;
        }
    }
}

/// Partial per-issue-type severity for overrides. All fields optional.
#[derive(Debug, Default, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct PartialRulesConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unused_files: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unused_exports: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unused_types: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unused_dependencies: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unused_dev_dependencies: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unused_optional_dependencies: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unused_enum_members: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unused_class_members: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unresolved_imports: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unlisted_dependencies: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duplicate_exports: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_only_dependencies: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub circular_dependencies: Option<Severity>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_default_all_error_except_type_only() {
        let rules = RulesConfig::default();
        assert_eq!(rules.unused_files, Severity::Error);
        assert_eq!(rules.unused_exports, Severity::Error);
        assert_eq!(rules.unused_types, Severity::Error);
        assert_eq!(rules.unused_dependencies, Severity::Error);
        assert_eq!(rules.unused_dev_dependencies, Severity::Error);
        assert_eq!(rules.unused_enum_members, Severity::Error);
        assert_eq!(rules.unused_class_members, Severity::Error);
        assert_eq!(rules.unresolved_imports, Severity::Error);
        assert_eq!(rules.unlisted_dependencies, Severity::Error);
        assert_eq!(rules.duplicate_exports, Severity::Error);
        assert_eq!(rules.type_only_dependencies, Severity::Warn);
        assert_eq!(rules.circular_dependencies, Severity::Error);
    }

    #[test]
    fn rules_deserialize_kebab_case() {
        let json_str = r#"{
            "unused-files": "error",
            "unused-exports": "warn",
            "unused-types": "off"
        }"#;
        let rules: RulesConfig = serde_json::from_str(json_str).unwrap();
        assert_eq!(rules.unused_files, Severity::Error);
        assert_eq!(rules.unused_exports, Severity::Warn);
        assert_eq!(rules.unused_types, Severity::Off);
        // Unset fields default to error
        assert_eq!(rules.unresolved_imports, Severity::Error);
    }

    #[test]
    fn severity_from_str() {
        assert_eq!("error".parse::<Severity>().unwrap(), Severity::Error);
        assert_eq!("warn".parse::<Severity>().unwrap(), Severity::Warn);
        assert_eq!("warning".parse::<Severity>().unwrap(), Severity::Warn);
        assert_eq!("off".parse::<Severity>().unwrap(), Severity::Off);
        assert_eq!("none".parse::<Severity>().unwrap(), Severity::Off);
        assert!("invalid".parse::<Severity>().is_err());
    }

    #[test]
    fn apply_partial_only_some_fields() {
        let mut rules = RulesConfig::default();
        let partial = PartialRulesConfig {
            unused_files: Some(Severity::Warn),
            unused_exports: Some(Severity::Off),
            ..Default::default()
        };
        rules.apply_partial(&partial);
        assert_eq!(rules.unused_files, Severity::Warn);
        assert_eq!(rules.unused_exports, Severity::Off);
        // Unset fields unchanged
        assert_eq!(rules.unused_types, Severity::Error);
        assert_eq!(rules.unresolved_imports, Severity::Error);
    }

    #[test]
    fn severity_display() {
        assert_eq!(Severity::Error.to_string(), "error");
        assert_eq!(Severity::Warn.to_string(), "warn");
        assert_eq!(Severity::Off.to_string(), "off");
    }

    #[test]
    fn apply_partial_all_none_changes_nothing() {
        let mut rules = RulesConfig::default();
        let original = rules.clone();
        let partial = PartialRulesConfig::default(); // all None
        rules.apply_partial(&partial);
        assert_eq!(rules.unused_files, original.unused_files);
        assert_eq!(rules.unused_exports, original.unused_exports);
        assert_eq!(
            rules.type_only_dependencies,
            original.type_only_dependencies
        );
    }

    #[test]
    fn apply_partial_all_fields_set() {
        let mut rules = RulesConfig::default();
        let partial = PartialRulesConfig {
            unused_files: Some(Severity::Off),
            unused_exports: Some(Severity::Off),
            unused_types: Some(Severity::Off),
            unused_dependencies: Some(Severity::Off),
            unused_dev_dependencies: Some(Severity::Off),
            unused_optional_dependencies: Some(Severity::Off),
            unused_enum_members: Some(Severity::Off),
            unused_class_members: Some(Severity::Off),
            unresolved_imports: Some(Severity::Off),
            unlisted_dependencies: Some(Severity::Off),
            duplicate_exports: Some(Severity::Off),
            type_only_dependencies: Some(Severity::Off),
            circular_dependencies: Some(Severity::Off),
        };
        rules.apply_partial(&partial);
        assert_eq!(rules.unused_files, Severity::Off);
        assert_eq!(rules.circular_dependencies, Severity::Off);
        assert_eq!(rules.type_only_dependencies, Severity::Off);
    }

    #[test]
    fn rules_config_defaults_include_optional_deps() {
        let rules = RulesConfig::default();
        assert_eq!(rules.unused_optional_dependencies, Severity::Error);
    }
}
