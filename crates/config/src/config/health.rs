use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const fn default_max_cyclomatic() -> u16 {
    20
}

const fn default_max_cognitive() -> u16 {
    15
}

/// Configuration for complexity health metrics (`fallow health`).
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthConfig {
    /// Maximum allowed cyclomatic complexity per function (default: 20).
    /// Functions exceeding this threshold are reported.
    #[serde(default = "default_max_cyclomatic")]
    pub max_cyclomatic: u16,

    /// Maximum allowed cognitive complexity per function (default: 15).
    /// Functions exceeding this threshold are reported.
    #[serde(default = "default_max_cognitive")]
    pub max_cognitive: u16,

    /// Glob patterns to exclude from complexity analysis.
    #[serde(default)]
    pub ignore: Vec<String>,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            max_cyclomatic: default_max_cyclomatic(),
            max_cognitive: default_max_cognitive(),
            ignore: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_config_defaults() {
        let config = HealthConfig::default();
        assert_eq!(config.max_cyclomatic, 20);
        assert_eq!(config.max_cognitive, 15);
        assert!(config.ignore.is_empty());
    }

    #[test]
    fn health_config_json_all_fields() {
        let json = r#"{
            "maxCyclomatic": 30,
            "maxCognitive": 25,
            "ignore": ["**/generated/**", "vendor/**"]
        }"#;
        let config: HealthConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.max_cyclomatic, 30);
        assert_eq!(config.max_cognitive, 25);
        assert_eq!(config.ignore, vec!["**/generated/**", "vendor/**"]);
    }

    #[test]
    fn health_config_json_partial_uses_defaults() {
        let json = r#"{"maxCyclomatic": 10}"#;
        let config: HealthConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.max_cyclomatic, 10);
        assert_eq!(config.max_cognitive, 15); // default
        assert!(config.ignore.is_empty()); // default
    }

    #[test]
    fn health_config_json_empty_object_uses_all_defaults() {
        let config: HealthConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(config.max_cyclomatic, 20);
        assert_eq!(config.max_cognitive, 15);
        assert!(config.ignore.is_empty());
    }

    #[test]
    fn health_config_json_only_ignore() {
        let json = r#"{"ignore": ["test/**"]}"#;
        let config: HealthConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.max_cyclomatic, 20); // default
        assert_eq!(config.max_cognitive, 15); // default
        assert_eq!(config.ignore, vec!["test/**"]);
    }
}
