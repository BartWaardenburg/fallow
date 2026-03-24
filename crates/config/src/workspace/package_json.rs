use serde::{Deserialize, Serialize};

/// Type alias for standard `HashMap` used in serde-deserialized structs.
/// `rustc-hash` v2 does not have a `serde` feature, so fields deserialized
/// from JSON must use `std::collections::HashMap`.
#[expect(clippy::disallowed_types)]
type StdHashMap<K, V> = std::collections::HashMap<K, V>;

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
    pub source: Option<String>,
    #[serde(default)]
    pub browser: Option<serde_json::Value>,
    #[serde(default)]
    pub bin: Option<serde_json::Value>,
    #[serde(default)]
    pub exports: Option<serde_json::Value>,
    #[serde(default)]
    pub dependencies: Option<StdHashMap<String, String>>,
    #[serde(default, rename = "devDependencies")]
    pub dev_dependencies: Option<StdHashMap<String, String>>,
    #[serde(default, rename = "peerDependencies")]
    pub peer_dependencies: Option<StdHashMap<String, String>>,
    #[serde(default, rename = "optionalDependencies")]
    pub optional_dependencies: Option<StdHashMap<String, String>>,
    #[serde(default)]
    pub scripts: Option<StdHashMap<String, String>>,
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

    /// Get all dependency names (production + dev + peer + optional).
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
        if let Some(d) = &self.optional_dependencies {
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

    /// Get optional dependency names only.
    pub fn optional_dependency_names(&self) -> Vec<String> {
        self.optional_dependencies
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
        if let Some(source) = &self.source {
            entries.push(source.clone());
        }

        // Handle browser field (string or object with path values)
        if let Some(browser) = &self.browser {
            match browser {
                serde_json::Value::String(s) => entries.push(s.clone()),
                serde_json::Value::Object(map) => {
                    for v in map.values() {
                        if let serde_json::Value::String(s) = v
                            && (s.starts_with("./") || s.starts_with("../"))
                        {
                            entries.push(s.clone());
                        }
                    }
                }
                _ => {}
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_json_workspace_patterns_array() {
        let pkg: PackageJson =
            serde_json::from_str(r#"{"workspaces": ["packages/*", "apps/*"]}"#).unwrap();
        let patterns = pkg.workspace_patterns();
        assert_eq!(patterns, vec!["packages/*", "apps/*"]);
    }

    #[test]
    fn package_json_workspace_patterns_object() {
        let pkg: PackageJson =
            serde_json::from_str(r#"{"workspaces": {"packages": ["packages/*"]}}"#).unwrap();
        let patterns = pkg.workspace_patterns();
        assert_eq!(patterns, vec!["packages/*"]);
    }

    #[test]
    fn package_json_workspace_patterns_none() {
        let pkg: PackageJson = serde_json::from_str(r#"{"name": "test"}"#).unwrap();
        let patterns = pkg.workspace_patterns();
        assert!(patterns.is_empty());
    }

    #[test]
    fn package_json_workspace_patterns_empty_array() {
        let pkg: PackageJson = serde_json::from_str(r#"{"workspaces": []}"#).unwrap();
        let patterns = pkg.workspace_patterns();
        assert!(patterns.is_empty());
    }

    #[test]
    fn package_json_load_valid() {
        let temp_dir = std::env::temp_dir().join("fallow-test-pkg-json");
        let _ = std::fs::create_dir_all(&temp_dir);
        let pkg_path = temp_dir.join("package.json");
        std::fs::write(&pkg_path, r#"{"name": "test", "main": "index.js"}"#).unwrap();

        let pkg = PackageJson::load(&pkg_path).unwrap();
        assert_eq!(pkg.name, Some("test".to_string()));
        assert_eq!(pkg.main, Some("index.js".to_string()));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn package_json_load_missing_file() {
        let result = PackageJson::load(std::path::Path::new("/nonexistent/package.json"));
        assert!(result.is_err());
    }

    #[test]
    fn package_json_entry_points_combined() {
        let pkg: PackageJson = serde_json::from_str(
            r#"{
            "main": "dist/index.js",
            "module": "dist/index.mjs",
            "types": "dist/index.d.ts",
            "typings": "dist/types.d.ts"
        }"#,
        )
        .unwrap();
        let entries = pkg.entry_points();
        assert_eq!(entries.len(), 4);
        assert!(entries.contains(&"dist/index.js".to_string()));
        assert!(entries.contains(&"dist/index.mjs".to_string()));
        assert!(entries.contains(&"dist/index.d.ts".to_string()));
        assert!(entries.contains(&"dist/types.d.ts".to_string()));
    }

    #[test]
    fn package_json_exports_nested() {
        let pkg: PackageJson = serde_json::from_str(
            r#"{
            "exports": {
                ".": {
                    "import": "./dist/index.mjs",
                    "require": "./dist/index.cjs"
                },
                "./utils": {
                    "import": "./dist/utils.mjs"
                }
            }
        }"#,
        )
        .unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"./dist/index.mjs".to_string()));
        assert!(entries.contains(&"./dist/index.cjs".to_string()));
        assert!(entries.contains(&"./dist/utils.mjs".to_string()));
    }

    #[test]
    fn package_json_exports_array() {
        let pkg: PackageJson = serde_json::from_str(
            r#"{
            "exports": {
                ".": ["./dist/index.mjs", "./dist/index.cjs"]
            }
        }"#,
        )
        .unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"./dist/index.mjs".to_string()));
        assert!(entries.contains(&"./dist/index.cjs".to_string()));
    }

    #[test]
    fn extract_exports_ignores_non_relative() {
        let pkg: PackageJson = serde_json::from_str(
            r#"{
            "exports": {
                ".": "not-a-relative-path"
            }
        }"#,
        )
        .unwrap();
        let entries = pkg.entry_points();
        // "not-a-relative-path" doesn't start with "./" so should be excluded
        assert!(entries.is_empty());
    }

    #[test]
    fn package_json_source_field() {
        let pkg: PackageJson = serde_json::from_str(
            r#"{
            "main": "dist/index.js",
            "source": "src/index.ts"
        }"#,
        )
        .unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"src/index.ts".to_string()));
        assert!(entries.contains(&"dist/index.js".to_string()));
    }

    #[test]
    fn package_json_browser_field_string() {
        let pkg: PackageJson = serde_json::from_str(
            r#"{
            "browser": "./dist/browser.js"
        }"#,
        )
        .unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"./dist/browser.js".to_string()));
    }

    #[test]
    fn package_json_browser_field_object() {
        let pkg: PackageJson = serde_json::from_str(
            r#"{
            "browser": {
                "./server.js": "./browser.js",
                "module-name": false
            }
        }"#,
        )
        .unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"./browser.js".to_string()));
        // non-relative paths and false values should be excluded
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn package_json_exports_string() {
        let pkg: PackageJson =
            serde_json::from_str(r#"{"exports": "./dist/index.js"}"#).unwrap();
        let entries = pkg.entry_points();
        assert_eq!(entries, vec!["./dist/index.js"]);
    }

    #[test]
    fn package_json_workspace_patterns_object_with_nohoist() {
        let pkg: PackageJson = serde_json::from_str(
            r#"{
            "workspaces": {
                "packages": ["packages/*", "apps/*"],
                "nohoist": ["**/react-native"]
            }
        }"#,
        )
        .unwrap();
        let patterns = pkg.workspace_patterns();
        assert_eq!(patterns, vec!["packages/*", "apps/*"]);
    }

    #[test]
    fn package_json_missing_optional_fields() {
        let pkg: PackageJson = serde_json::from_str(r#"{}"#).unwrap();
        assert!(pkg.name.is_none());
        assert!(pkg.main.is_none());
        assert!(pkg.module.is_none());
        assert!(pkg.types.is_none());
        assert!(pkg.typings.is_none());
        assert!(pkg.source.is_none());
        assert!(pkg.browser.is_none());
        assert!(pkg.bin.is_none());
        assert!(pkg.exports.is_none());
        assert!(pkg.dependencies.is_none());
        assert!(pkg.dev_dependencies.is_none());
        assert!(pkg.peer_dependencies.is_none());
        assert!(pkg.optional_dependencies.is_none());
        assert!(pkg.scripts.is_none());
        assert!(pkg.workspaces.is_none());
        assert!(pkg.entry_points().is_empty());
        assert!(pkg.workspace_patterns().is_empty());
        assert!(pkg.all_dependency_names().is_empty());
    }

    #[test]
    fn package_json_all_dependency_names() {
        let pkg: PackageJson = serde_json::from_str(
            r#"{
            "dependencies": {"react": "^18", "react-dom": "^18"},
            "devDependencies": {"typescript": "^5"},
            "peerDependencies": {"node": ">=18"},
            "optionalDependencies": {"fsevents": "^2"}
        }"#,
        )
        .unwrap();
        let deps = pkg.all_dependency_names();
        assert_eq!(deps.len(), 5);
        assert!(deps.contains(&"react".to_string()));
        assert!(deps.contains(&"react-dom".to_string()));
        assert!(deps.contains(&"typescript".to_string()));
        assert!(deps.contains(&"node".to_string()));
        assert!(deps.contains(&"fsevents".to_string()));
    }

    #[test]
    fn package_json_production_dependency_names() {
        let pkg: PackageJson = serde_json::from_str(
            r#"{
            "dependencies": {"react": "^18"},
            "devDependencies": {"typescript": "^5"}
        }"#,
        )
        .unwrap();
        let prod = pkg.production_dependency_names();
        assert_eq!(prod, vec!["react"]);
        let dev = pkg.dev_dependency_names();
        assert_eq!(dev, vec!["typescript"]);
    }

    #[test]
    fn package_json_bin_field_string() {
        let pkg: PackageJson =
            serde_json::from_str(r#"{"bin": "./cli.js"}"#).unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"./cli.js".to_string()));
    }

    #[test]
    fn package_json_bin_field_object() {
        let pkg: PackageJson = serde_json::from_str(
            r#"{"bin": {"my-cli": "./bin/cli.js", "my-tool": "./bin/tool.js"}}"#,
        )
        .unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"./bin/cli.js".to_string()));
        assert!(entries.contains(&"./bin/tool.js".to_string()));
    }

    #[test]
    fn package_json_exports_deeply_nested() {
        let pkg: PackageJson = serde_json::from_str(
            r#"{
            "exports": {
                ".": {
                    "node": {
                        "import": "./dist/node.mjs",
                        "require": "./dist/node.cjs"
                    },
                    "browser": {
                        "import": "./dist/browser.mjs"
                    }
                }
            }
        }"#,
        )
        .unwrap();
        let entries = pkg.entry_points();
        assert_eq!(entries.len(), 3);
        assert!(entries.contains(&"./dist/node.mjs".to_string()));
        assert!(entries.contains(&"./dist/node.cjs".to_string()));
        assert!(entries.contains(&"./dist/browser.mjs".to_string()));
    }
}
