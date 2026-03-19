use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Workspace configuration for monorepo support.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
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

/// Discover all workspace packages in a monorepo.
pub fn discover_workspaces(root: &Path) -> Vec<WorkspaceInfo> {
    let mut patterns = Vec::new();

    // 1. Check root package.json for workspace patterns
    let pkg_path = root.join("package.json");
    if let Ok(pkg) = PackageJson::load(&pkg_path) {
        patterns.extend(pkg.workspace_patterns());
    }

    // 2. Check pnpm-workspace.yaml
    let pnpm_workspace = root.join("pnpm-workspace.yaml");
    if pnpm_workspace.exists()
        && let Ok(content) = std::fs::read_to_string(&pnpm_workspace)
    {
        patterns.extend(parse_pnpm_workspace_yaml(&content));
    }

    if patterns.is_empty() {
        return Vec::new();
    }

    // 3. Separate positive and negated patterns.
    // Negated patterns (e.g., `!**/test/**`) are used as exclusion filters —
    // the `glob` crate does not support `!` prefixed patterns natively.
    let (positive, negative): (Vec<&String>, Vec<&String>) =
        patterns.iter().partition(|p| !p.starts_with('!'));
    let negation_matchers: Vec<globset::GlobMatcher> = negative
        .iter()
        .filter_map(|p| {
            let stripped = p.strip_prefix('!').unwrap_or(p);
            globset::Glob::new(stripped)
                .ok()
                .map(|g| g.compile_matcher())
        })
        .collect();

    // Expand patterns to find workspace directories
    // Pre-compute canonical root once for security checks in expand_workspace_glob
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut workspaces = Vec::new();
    for pattern in &positive {
        // Normalize the pattern for directory matching:
        // - `packages/*` → glob for `packages/*` (find all subdirs)
        // - `packages/` → glob for `packages/*` (trailing slash means "contents of")
        // - `apps`       → glob for `apps` (exact directory)
        let glob_pattern = if pattern.ends_with('/') {
            format!("{}*", pattern)
        } else if !pattern.contains('*') && !pattern.contains('?') && !pattern.contains('{') {
            // Bare directory name — treat as exact match
            (*pattern).clone()
        } else {
            (*pattern).clone()
        };

        // Walk directories matching the glob
        let matched_dirs = expand_workspace_glob(root, &glob_pattern, &canonical_root);
        for dir in matched_dirs {
            // Skip workspace entries that point to the project root itself
            // (e.g. pnpm-workspace.yaml listing `.` as a workspace)
            let canonical_dir = dir.canonicalize().unwrap_or_else(|_| dir.clone());
            if canonical_dir == canonical_root {
                continue;
            }

            // Check against negation patterns — skip directories that match any negated pattern
            let relative = dir.strip_prefix(root).unwrap_or(&dir);
            let relative_str = relative.to_string_lossy();
            if negation_matchers
                .iter()
                .any(|m| m.is_match(relative_str.as_ref()))
            {
                continue;
            }

            let ws_pkg_path = dir.join("package.json");
            if ws_pkg_path.exists()
                && let Ok(pkg) = PackageJson::load(&ws_pkg_path)
            {
                // Collect dependency names during initial load to avoid
                // re-reading package.json in step 5.
                let dep_names = pkg.all_dependency_names();
                let name = pkg.name.unwrap_or_else(|| {
                    dir.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                });
                workspaces.push((
                    WorkspaceInfo {
                        root: dir,
                        name,
                        is_internal_dependency: false,
                    },
                    dep_names,
                ));
            }
        }
    }

    // 4. Deduplicate workspaces by canonical path.
    // Overlapping patterns (e.g., `examples/*` and `examples/minimal/*`) can match the
    // same directory, causing duplicate workspace entries and double-reported issues.
    {
        let mut seen = std::collections::HashSet::new();
        workspaces.retain(|(ws, _)| {
            let canonical = ws.root.canonicalize().unwrap_or_else(|_| ws.root.clone());
            seen.insert(canonical)
        });
    }

    // 5. Mark workspaces that are depended on by other workspaces.
    // Uses dep names collected during initial package.json load (step 3)
    // to avoid re-reading all workspace package.json files.
    let all_dep_names: std::collections::HashSet<String> = workspaces
        .iter()
        .flat_map(|(_, deps)| deps.iter().cloned())
        .collect();
    for (ws, _) in &mut workspaces {
        ws.is_internal_dependency = all_dep_names.contains(&ws.name);
    }

    workspaces.into_iter().map(|(ws, _)| ws).collect()
}

/// Expand a workspace glob pattern to matching directories.
///
/// Uses the `glob` crate for full glob support including `**` (deep matching).
/// `canonical_root` is pre-computed to avoid repeated canonicalize() syscalls.
fn expand_workspace_glob(root: &Path, pattern: &str, canonical_root: &Path) -> Vec<PathBuf> {
    let full_pattern = root.join(pattern).to_string_lossy().to_string();
    match glob::glob(&full_pattern) {
        Ok(paths) => paths
            .filter_map(Result::ok)
            .filter(|p| p.is_dir())
            .filter(|p| {
                // Security: ensure workspace directory is within project root
                p.canonicalize()
                    .ok()
                    .is_some_and(|cp| cp.starts_with(canonical_root))
            })
            .collect(),
        Err(e) => {
            eprintln!("Warning: Invalid workspace glob pattern '{pattern}': {e}");
            Vec::new()
        }
    }
}

/// Parse pnpm-workspace.yaml to extract package patterns.
fn parse_pnpm_workspace_yaml(content: &str) -> Vec<String> {
    // Simple YAML parsing for the common format:
    // packages:
    //   - 'packages/*'
    //   - 'apps/*'
    let mut patterns = Vec::new();
    let mut in_packages = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "packages:" {
            in_packages = true;
            continue;
        }
        if in_packages {
            if trimmed.starts_with("- ") {
                let value = trimmed
                    .strip_prefix("- ")
                    .unwrap_or(trimmed)
                    .trim_matches('\'')
                    .trim_matches('"');
                patterns.push(value.to_string());
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                break; // New top-level key
            }
        }
    }

    patterns
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
    pub source: Option<String>,
    #[serde(default)]
    pub browser: Option<serde_json::Value>,
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
    fn parse_pnpm_workspace_basic() {
        let yaml = "packages:\n  - 'packages/*'\n  - 'apps/*'\n";
        let patterns = parse_pnpm_workspace_yaml(yaml);
        assert_eq!(patterns, vec!["packages/*", "apps/*"]);
    }

    #[test]
    fn parse_pnpm_workspace_double_quotes() {
        let yaml = "packages:\n  - \"packages/*\"\n  - \"apps/*\"\n";
        let patterns = parse_pnpm_workspace_yaml(yaml);
        assert_eq!(patterns, vec!["packages/*", "apps/*"]);
    }

    #[test]
    fn parse_pnpm_workspace_no_quotes() {
        let yaml = "packages:\n  - packages/*\n  - apps/*\n";
        let patterns = parse_pnpm_workspace_yaml(yaml);
        assert_eq!(patterns, vec!["packages/*", "apps/*"]);
    }

    #[test]
    fn parse_pnpm_workspace_empty() {
        let yaml = "";
        let patterns = parse_pnpm_workspace_yaml(yaml);
        assert!(patterns.is_empty());
    }

    #[test]
    fn parse_pnpm_workspace_no_packages_key() {
        let yaml = "other:\n  - something\n";
        let patterns = parse_pnpm_workspace_yaml(yaml);
        assert!(patterns.is_empty());
    }

    #[test]
    fn parse_pnpm_workspace_with_comments() {
        let yaml = "packages:\n  # Comment\n  - 'packages/*'\n";
        let patterns = parse_pnpm_workspace_yaml(yaml);
        assert_eq!(patterns, vec!["packages/*"]);
    }

    #[test]
    fn parse_pnpm_workspace_stops_at_next_key() {
        let yaml = "packages:\n  - 'packages/*'\ncatalog:\n  react: ^18\n";
        let patterns = parse_pnpm_workspace_yaml(yaml);
        assert_eq!(patterns, vec!["packages/*"]);
    }

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
}
