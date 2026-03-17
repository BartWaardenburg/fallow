use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Workspace configuration for monorepo support.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkspaceConfig {
    /// Additional workspace patterns (beyond what's in root package.json).
    #[serde(default)]
    pub patterns: Vec<String>,
}

/// Discovered workspace info from package.json or pnpm-workspace.yaml.
#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    /// Workspace root path.
    pub root: PathBuf,
    /// Package name from package.json.
    pub name: String,
    /// Whether this workspace is depended on by other workspaces.
    pub is_internal_dependency: bool,
}

/// Parsed package.json with fields relevant to fallow.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PackageJson {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub main: Option<String>,
    #[serde(default)]
    pub module: Option<String>,
    #[serde(default)]
    pub types: Option<String>,
    #[serde(default)]
    pub typings: Option<String>,
    #[serde(default)]
    pub bin: Option<serde_json::Value>,
    #[serde(default)]
    pub exports: Option<serde_json::Value>,
    #[serde(default)]
    pub dependencies: Option<std::collections::HashMap<String, String>>,
    #[serde(default, rename = "devDependencies")]
    pub dev_dependencies: Option<std::collections::HashMap<String, String>>,
    #[serde(default, rename = "peerDependencies")]
    pub peer_dependencies: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub scripts: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub workspaces: Option<serde_json::Value>,
}

impl PackageJson {
    /// Load from a package.json file.
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
    }

    /// Get all dependency names (production + dev + peer).
    pub fn all_dependency_names(&self) -> Vec<String> {
        let mut deps = Vec::new();
        if let Some(d) = &self.dependencies {
            deps.extend(d.keys().cloned());
        }
        if let Some(d) = &self.dev_dependencies {
            deps.extend(d.keys().cloned());
        }
        if let Some(d) = &self.peer_dependencies {
            deps.extend(d.keys().cloned());
        }
        deps
    }

    /// Get production dependency names only.
    pub fn production_dependency_names(&self) -> Vec<String> {
        self.dependencies
            .as_ref()
            .map(|d| d.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Get dev dependency names only.
    pub fn dev_dependency_names(&self) -> Vec<String> {
        self.dev_dependencies
            .as_ref()
            .map(|d| d.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Extract entry points from package.json fields.
    pub fn entry_points(&self) -> Vec<String> {
        let mut entries = Vec::new();

        if let Some(main) = &self.main {
            entries.push(main.clone());
        }
        if let Some(module) = &self.module {
            entries.push(module.clone());
        }
        if let Some(types) = &self.types {
            entries.push(types.clone());
        }
        if let Some(typings) = &self.typings {
            entries.push(typings.clone());
        }

        // Handle bin field (string or object)
        if let Some(bin) = &self.bin {
            match bin {
                serde_json::Value::String(s) => entries.push(s.clone()),
                serde_json::Value::Object(map) => {
                    for v in map.values() {
                        if let serde_json::Value::String(s) = v {
                            entries.push(s.clone());
                        }
                    }
                }
                _ => {}
            }
        }

        // Handle exports field (recursive)
        if let Some(exports) = &self.exports {
            extract_exports_entries(exports, &mut entries);
        }

        entries
    }

    /// Extract workspace patterns from package.json.
    pub fn workspace_patterns(&self) -> Vec<String> {
        match &self.workspaces {
            Some(serde_json::Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
            Some(serde_json::Value::Object(obj)) => obj
                .get("packages")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }
}

/// Recursively extract file paths from package.json exports field.
fn extract_exports_entries(value: &serde_json::Value, entries: &mut Vec<String>) {
    match value {
        serde_json::Value::String(s) => {
            if s.starts_with("./") || s.starts_with("../") {
                entries.push(s.clone());
            }
        }
        serde_json::Value::Object(map) => {
            for v in map.values() {
                extract_exports_entries(v, entries);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                extract_exports_entries(v, entries);
            }
        }
        _ => {}
    }
}
