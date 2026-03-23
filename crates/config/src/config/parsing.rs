use std::io::Read as _;
use std::path::{Path, PathBuf};

use rustc_hash::FxHashSet;

use super::FallowConfig;

/// Supported config file names in priority order.
///
/// `find_and_load` checks these names in order within each directory,
/// returning the first match found.
pub(super) const CONFIG_NAMES: &[&str] = &[".fallowrc.json", "fallow.toml", ".fallow.toml"];

pub(super) const MAX_EXTENDS_DEPTH: usize = 10;

/// Detect config format from file extension.
pub(super) enum ConfigFormat {
    Toml,
    Json,
}

impl ConfigFormat {
    pub(super) fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|e| e.to_str()) {
            Some("json") => Self::Json,
            _ => Self::Toml,
        }
    }
}

/// Deep-merge two JSON values. `base` is lower-priority, `overlay` is higher.
/// Objects: merge field by field. Arrays/scalars: overlay replaces base.
pub(super) fn deep_merge_json(base: &mut serde_json::Value, overlay: serde_json::Value) {
    match (base, overlay) {
        (serde_json::Value::Object(base_map), serde_json::Value::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                if let Some(base_value) = base_map.get_mut(&key) {
                    deep_merge_json(base_value, value);
                } else {
                    base_map.insert(key, value);
                }
            }
        }
        (base, overlay) => {
            *base = overlay;
        }
    }
}

pub(super) fn parse_config_to_value(path: &Path) -> Result<serde_json::Value, miette::Report> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| miette::miette!("Failed to read config file {}: {}", path.display(), e))?;

    match ConfigFormat::from_path(path) {
        ConfigFormat::Toml => {
            let toml_value: toml::Value = toml::from_str(&content).map_err(|e| {
                miette::miette!("Failed to parse config file {}: {}", path.display(), e)
            })?;
            serde_json::to_value(toml_value).map_err(|e| {
                miette::miette!(
                    "Failed to convert TOML to JSON for {}: {}",
                    path.display(),
                    e
                )
            })
        }
        ConfigFormat::Json => {
            let mut stripped = String::new();
            json_comments::StripComments::new(content.as_bytes())
                .read_to_string(&mut stripped)
                .map_err(|e| {
                    miette::miette!("Failed to strip comments from {}: {}", path.display(), e)
                })?;
            serde_json::from_str(&stripped).map_err(|e| {
                miette::miette!("Failed to parse config file {}: {}", path.display(), e)
            })
        }
    }
}

pub(super) fn resolve_extends(
    path: &Path,
    visited: &mut FxHashSet<PathBuf>,
    depth: usize,
) -> Result<serde_json::Value, miette::Report> {
    if depth >= MAX_EXTENDS_DEPTH {
        return Err(miette::miette!(
            "Config extends chain too deep (>={MAX_EXTENDS_DEPTH} levels) at {}",
            path.display()
        ));
    }

    let canonical = path.canonicalize().map_err(|e| {
        miette::miette!(
            "Config file not found or unresolvable: {}: {}",
            path.display(),
            e
        )
    })?;

    if !visited.insert(canonical) {
        return Err(miette::miette!(
            "Circular extends detected: {} was already visited in the extends chain",
            path.display()
        ));
    }

    let mut value = parse_config_to_value(path)?;

    let extends = value
        .as_object_mut()
        .and_then(|obj| obj.remove("extends"))
        .and_then(|v| match v {
            serde_json::Value::Array(arr) => Some(
                arr.into_iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>(),
            ),
            serde_json::Value::String(s) => Some(vec![s]),
            _ => None,
        })
        .unwrap_or_default();

    if extends.is_empty() {
        return Ok(value);
    }

    let config_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut merged = serde_json::Value::Object(serde_json::Map::new());

    for extend_path_str in &extends {
        if Path::new(extend_path_str).is_absolute() {
            return Err(miette::miette!(
                "extends paths must be relative, got absolute path: {} (in {})",
                extend_path_str,
                path.display()
            ));
        }
        let extend_path = config_dir.join(extend_path_str);
        if !extend_path.exists() {
            return Err(miette::miette!(
                "Extended config file not found: {} (referenced from {})",
                extend_path.display(),
                path.display()
            ));
        }
        let base = resolve_extends(&extend_path, visited, depth + 1)?;
        deep_merge_json(&mut merged, base);
    }

    deep_merge_json(&mut merged, value);
    Ok(merged)
}

impl FallowConfig {
    /// Load config from a fallow config file (TOML or JSON/JSONC).
    ///
    /// The format is detected from the file extension:
    /// - `.toml` → TOML
    /// - `.json` → JSON (with JSONC comment stripping)
    ///
    /// Supports `extends` for config inheritance. Extended configs are loaded
    /// and deep-merged before this config's values are applied.
    pub fn load(path: &Path) -> Result<Self, miette::Report> {
        let mut visited = FxHashSet::default();
        let merged = resolve_extends(path, &mut visited, 0)?;

        serde_json::from_value(merged).map_err(|e| {
            miette::miette!(
                "Failed to deserialize config from {}: {}",
                path.display(),
                e
            )
        })
    }

    /// Find and load config from the current directory or ancestors.
    ///
    /// Checks for config files in priority order:
    /// `.fallowrc.json` > `fallow.toml` > `.fallow.toml`
    ///
    /// Stops searching at the first directory containing `.git` or `package.json`,
    /// to avoid picking up unrelated config files above the project root.
    ///
    /// Returns `Ok(Some(...))` if a config was found and parsed, `Ok(None)` if
    /// no config file exists, and `Err(...)` if a config file exists but fails to parse.
    pub fn find_and_load(start: &Path) -> Result<Option<(Self, PathBuf)>, String> {
        let mut dir = start;
        loop {
            for name in CONFIG_NAMES {
                let candidate = dir.join(name);
                if candidate.exists() {
                    match Self::load(&candidate) {
                        Ok(config) => return Ok(Some((config, candidate))),
                        Err(e) => {
                            return Err(format!("Failed to parse {}: {e}", candidate.display()));
                        }
                    }
                }
            }
            // Stop at project root indicators
            if dir.join(".git").exists() || dir.join("package.json").exists() {
                break;
            }
            dir = match dir.parent() {
                Some(parent) => parent,
                None => break,
            };
        }
        Ok(None)
    }

    /// Generate JSON Schema for the configuration format.
    pub fn json_schema() -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(FallowConfig)).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read as _;

    use super::*;
    use crate::PackageJson;
    use crate::config::duplicates_config::DuplicatesConfig;
    use crate::config::format::OutputFormat;
    use crate::config::rules::{RulesConfig, Severity};

    /// Create a unique temp directory for this test to avoid parallel test races.
    fn test_dir(name: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("fallow-{name}-{id}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn fallow_config_deserialize_minimal() {
        let toml_str = r#"
entry = ["src/main.ts"]
"#;
        let config: FallowConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.entry, vec!["src/main.ts"]);
        assert!(config.ignore_patterns.is_empty());
    }

    #[test]
    fn fallow_config_deserialize_ignore_exports() {
        let toml_str = r#"
[[ignoreExports]]
file = "src/types/*.ts"
exports = ["*"]

[[ignoreExports]]
file = "src/constants.ts"
exports = ["FOO", "BAR"]
"#;
        let config: FallowConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.ignore_exports.len(), 2);
        assert_eq!(config.ignore_exports[0].file, "src/types/*.ts");
        assert_eq!(config.ignore_exports[0].exports, vec!["*"]);
        assert_eq!(config.ignore_exports[1].exports, vec!["FOO", "BAR"]);
    }

    #[test]
    fn fallow_config_deserialize_ignore_dependencies() {
        let toml_str = r#"
ignoreDependencies = ["autoprefixer", "postcss"]
"#;
        let config: FallowConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.ignore_dependencies, vec!["autoprefixer", "postcss"]);
    }

    #[test]
    fn fallow_config_resolve_default_ignores() {
        let config = FallowConfig {
            schema: None,
            extends: vec![],
            entry: vec![],
            ignore_patterns: vec![],
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            duplicates: DuplicatesConfig::default(),
            rules: RulesConfig::default(),
            production: false,
            plugins: vec![],
            overrides: vec![],
        };
        let resolved = config.resolve(
            PathBuf::from("/tmp/test"),
            OutputFormat::Human,
            4,
            true,
            true,
        );

        // Default ignores should be compiled
        assert!(resolved.ignore_patterns.is_match("node_modules/foo/bar.ts"));
        assert!(resolved.ignore_patterns.is_match("dist/bundle.js"));
        assert!(resolved.ignore_patterns.is_match("build/output.js"));
        assert!(resolved.ignore_patterns.is_match(".git/config"));
        assert!(resolved.ignore_patterns.is_match("coverage/report.js"));
        assert!(resolved.ignore_patterns.is_match("foo.min.js"));
        assert!(resolved.ignore_patterns.is_match("bar.min.mjs"));
    }

    #[test]
    fn fallow_config_resolve_custom_ignores() {
        let config = FallowConfig {
            schema: None,
            extends: vec![],
            entry: vec!["src/**/*.ts".to_string()],
            ignore_patterns: vec!["**/*.generated.ts".to_string()],
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            duplicates: DuplicatesConfig::default(),
            rules: RulesConfig::default(),
            production: false,
            plugins: vec![],
            overrides: vec![],
        };
        let resolved = config.resolve(
            PathBuf::from("/tmp/test"),
            OutputFormat::Json,
            4,
            false,
            true,
        );

        assert!(resolved.ignore_patterns.is_match("src/foo.generated.ts"));
        assert_eq!(resolved.entry_patterns, vec!["src/**/*.ts"]);
        assert!(matches!(resolved.output, OutputFormat::Json));
        assert!(!resolved.no_cache);
    }

    #[test]
    fn fallow_config_resolve_cache_dir() {
        let config = FallowConfig {
            schema: None,
            extends: vec![],
            entry: vec![],
            ignore_patterns: vec![],
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            duplicates: DuplicatesConfig::default(),
            rules: RulesConfig::default(),
            production: false,
            plugins: vec![],
            overrides: vec![],
        };
        let resolved = config.resolve(
            PathBuf::from("/tmp/project"),
            OutputFormat::Human,
            4,
            true,
            true,
        );
        assert_eq!(resolved.cache_dir, PathBuf::from("/tmp/project/.fallow"));
        assert!(resolved.no_cache);
    }

    #[test]
    fn package_json_entry_points_main() {
        let pkg: PackageJson = serde_json::from_str(r#"{"main": "dist/index.js"}"#).unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"dist/index.js".to_string()));
    }

    #[test]
    fn package_json_entry_points_module() {
        let pkg: PackageJson = serde_json::from_str(r#"{"module": "dist/index.mjs"}"#).unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"dist/index.mjs".to_string()));
    }

    #[test]
    fn package_json_entry_points_types() {
        let pkg: PackageJson = serde_json::from_str(r#"{"types": "dist/index.d.ts"}"#).unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"dist/index.d.ts".to_string()));
    }

    #[test]
    fn package_json_entry_points_bin_string() {
        let pkg: PackageJson = serde_json::from_str(r#"{"bin": "bin/cli.js"}"#).unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"bin/cli.js".to_string()));
    }

    #[test]
    fn package_json_entry_points_bin_object() {
        let pkg: PackageJson =
            serde_json::from_str(r#"{"bin": {"cli": "bin/cli.js", "serve": "bin/serve.js"}}"#)
                .unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"bin/cli.js".to_string()));
        assert!(entries.contains(&"bin/serve.js".to_string()));
    }

    #[test]
    fn package_json_entry_points_exports_string() {
        let pkg: PackageJson = serde_json::from_str(r#"{"exports": "./dist/index.js"}"#).unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"./dist/index.js".to_string()));
    }

    #[test]
    fn package_json_entry_points_exports_object() {
        let pkg: PackageJson = serde_json::from_str(
            r#"{"exports": {".": {"import": "./dist/index.mjs", "require": "./dist/index.cjs"}}}"#,
        )
        .unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"./dist/index.mjs".to_string()));
        assert!(entries.contains(&"./dist/index.cjs".to_string()));
    }

    #[test]
    fn package_json_dependency_names() {
        let pkg: PackageJson = serde_json::from_str(
            r#"{
            "dependencies": {"react": "^18", "lodash": "^4"},
            "devDependencies": {"typescript": "^5"},
            "peerDependencies": {"react-dom": "^18"}
        }"#,
        )
        .unwrap();

        let all = pkg.all_dependency_names();
        assert!(all.contains(&"react".to_string()));
        assert!(all.contains(&"lodash".to_string()));
        assert!(all.contains(&"typescript".to_string()));
        assert!(all.contains(&"react-dom".to_string()));

        let prod = pkg.production_dependency_names();
        assert!(prod.contains(&"react".to_string()));
        assert!(!prod.contains(&"typescript".to_string()));

        let dev = pkg.dev_dependency_names();
        assert!(dev.contains(&"typescript".to_string()));
        assert!(!dev.contains(&"react".to_string()));
    }

    #[test]
    fn package_json_no_dependencies() {
        let pkg: PackageJson = serde_json::from_str(r#"{"name": "test"}"#).unwrap();
        assert!(pkg.all_dependency_names().is_empty());
        assert!(pkg.production_dependency_names().is_empty());
        assert!(pkg.dev_dependency_names().is_empty());
        assert!(pkg.entry_points().is_empty());
    }

    #[test]
    fn rules_deserialize_toml_kebab_case() {
        let toml_str = r#"
[rules]
unused-files = "error"
unused-exports = "warn"
unused-types = "off"
"#;
        let config: FallowConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.rules.unused_files, Severity::Error);
        assert_eq!(config.rules.unused_exports, Severity::Warn);
        assert_eq!(config.rules.unused_types, Severity::Off);
        // Unset fields default to error
        assert_eq!(config.rules.unresolved_imports, Severity::Error);
    }

    #[test]
    fn config_without_rules_defaults_to_error() {
        let toml_str = r#"
entry = ["src/main.ts"]
"#;
        let config: FallowConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.rules.unused_files, Severity::Error);
        assert_eq!(config.rules.unused_exports, Severity::Error);
    }

    #[test]
    fn fallow_config_denies_unknown_fields() {
        let toml_str = r#"
unknown_field = true
"#;
        let result: Result<FallowConfig, _> = toml::from_str(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn fallow_config_deserialize_json() {
        let json_str = r#"{"entry": ["src/main.ts"]}"#;
        let config: FallowConfig = serde_json::from_str(json_str).unwrap();
        assert_eq!(config.entry, vec!["src/main.ts"]);
    }

    #[test]
    fn fallow_config_deserialize_jsonc() {
        let jsonc_str = r#"{
            // This is a comment
            "entry": ["src/main.ts"],
            "rules": {
                "unused-files": "warn"
            }
        }"#;
        let mut stripped = String::new();
        json_comments::StripComments::new(jsonc_str.as_bytes())
            .read_to_string(&mut stripped)
            .unwrap();
        let config: FallowConfig = serde_json::from_str(&stripped).unwrap();
        assert_eq!(config.entry, vec!["src/main.ts"]);
        assert_eq!(config.rules.unused_files, Severity::Warn);
    }

    #[test]
    fn fallow_config_json_with_schema_field() {
        let json_str = r#"{"$schema": "https://fallow.dev/schema.json", "entry": ["src/main.ts"]}"#;
        let config: FallowConfig = serde_json::from_str(json_str).unwrap();
        assert_eq!(config.entry, vec!["src/main.ts"]);
    }

    #[test]
    fn fallow_config_json_schema_generation() {
        let schema = FallowConfig::json_schema();
        assert!(schema.is_object());
        let obj = schema.as_object().unwrap();
        assert!(obj.contains_key("properties"));
    }

    #[test]
    fn config_format_detection() {
        assert!(matches!(
            ConfigFormat::from_path(Path::new("fallow.toml")),
            ConfigFormat::Toml
        ));
        assert!(matches!(
            ConfigFormat::from_path(Path::new(".fallowrc.json")),
            ConfigFormat::Json
        ));
        assert!(matches!(
            ConfigFormat::from_path(Path::new(".fallow.toml")),
            ConfigFormat::Toml
        ));
    }

    #[test]
    fn config_names_priority_order() {
        assert_eq!(CONFIG_NAMES[0], ".fallowrc.json");
        assert_eq!(CONFIG_NAMES[1], "fallow.toml");
        assert_eq!(CONFIG_NAMES[2], ".fallow.toml");
    }

    #[test]
    fn load_json_config_file() {
        let dir = test_dir("json-config");
        let config_path = dir.join(".fallowrc.json");
        std::fs::write(
            &config_path,
            r#"{"entry": ["src/index.ts"], "rules": {"unused-exports": "warn"}}"#,
        )
        .unwrap();

        let config = FallowConfig::load(&config_path).unwrap();
        assert_eq!(config.entry, vec!["src/index.ts"]);
        assert_eq!(config.rules.unused_exports, Severity::Warn);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_jsonc_config_file() {
        let dir = test_dir("jsonc-config");
        let config_path = dir.join(".fallowrc.json");
        std::fs::write(
            &config_path,
            r#"{
                // Entry points for analysis
                "entry": ["src/index.ts"],
                /* Block comment */
                "rules": {
                    "unused-exports": "warn"
                }
            }"#,
        )
        .unwrap();

        let config = FallowConfig::load(&config_path).unwrap();
        assert_eq!(config.entry, vec!["src/index.ts"]);
        assert_eq!(config.rules.unused_exports, Severity::Warn);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn json_config_ignore_dependencies_camel_case() {
        let json_str = r#"{"ignoreDependencies": ["autoprefixer", "postcss"]}"#;
        let config: FallowConfig = serde_json::from_str(json_str).unwrap();
        assert_eq!(config.ignore_dependencies, vec!["autoprefixer", "postcss"]);
    }

    #[test]
    fn json_config_all_fields() {
        let json_str = r#"{
            "ignoreDependencies": ["lodash"],
            "ignoreExports": [{"file": "src/*.ts", "exports": ["*"]}],
            "rules": {
                "unused-files": "off",
                "unused-exports": "warn",
                "unused-dependencies": "error",
                "unused-dev-dependencies": "off",
                "unused-types": "warn",
                "unused-enum-members": "error",
                "unused-class-members": "off",
                "unresolved-imports": "warn",
                "unlisted-dependencies": "error",
                "duplicate-exports": "off"
            },
            "duplicates": {
                "minTokens": 100,
                "minLines": 10,
                "skipLocal": true
            }
        }"#;
        let config: FallowConfig = serde_json::from_str(json_str).unwrap();
        assert_eq!(config.ignore_dependencies, vec!["lodash"]);
        assert_eq!(config.rules.unused_files, Severity::Off);
        assert_eq!(config.rules.unused_exports, Severity::Warn);
        assert_eq!(config.rules.unused_dependencies, Severity::Error);
        assert_eq!(config.duplicates.min_tokens, 100);
        assert_eq!(config.duplicates.min_lines, 10);
        assert!(config.duplicates.skip_local);
    }

    // ── extends tests ──────────────────────────────────────────────

    #[test]
    fn extends_single_base() {
        let dir = test_dir("extends-single");

        std::fs::write(
            dir.join("base.json"),
            r#"{"rules": {"unused-files": "warn"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join(".fallowrc.json"),
            r#"{"extends": ["base.json"], "entry": ["src/index.ts"]}"#,
        )
        .unwrap();

        let config = FallowConfig::load(&dir.join(".fallowrc.json")).unwrap();
        assert_eq!(config.rules.unused_files, Severity::Warn);
        assert_eq!(config.entry, vec!["src/index.ts"]);
        // Unset fields from base still default
        assert_eq!(config.rules.unused_exports, Severity::Error);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extends_overlay_overrides_base() {
        let dir = test_dir("extends-overlay");

        std::fs::write(
            dir.join("base.json"),
            r#"{"rules": {"unused-files": "warn", "unused-exports": "off"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join(".fallowrc.json"),
            r#"{"extends": ["base.json"], "rules": {"unused-files": "error"}}"#,
        )
        .unwrap();

        let config = FallowConfig::load(&dir.join(".fallowrc.json")).unwrap();
        // Overlay overrides base
        assert_eq!(config.rules.unused_files, Severity::Error);
        // Base value preserved when not overridden
        assert_eq!(config.rules.unused_exports, Severity::Off);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extends_chained() {
        let dir = test_dir("extends-chained");

        std::fs::write(
            dir.join("grandparent.json"),
            r#"{"rules": {"unused-files": "off", "unused-exports": "warn"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("parent.json"),
            r#"{"extends": ["grandparent.json"], "rules": {"unused-files": "warn"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join(".fallowrc.json"),
            r#"{"extends": ["parent.json"]}"#,
        )
        .unwrap();

        let config = FallowConfig::load(&dir.join(".fallowrc.json")).unwrap();
        // grandparent: off -> parent: warn -> child: inherits warn
        assert_eq!(config.rules.unused_files, Severity::Warn);
        // grandparent: warn, not overridden
        assert_eq!(config.rules.unused_exports, Severity::Warn);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extends_circular_detected() {
        let dir = test_dir("extends-circular");

        std::fs::write(dir.join("a.json"), r#"{"extends": ["b.json"]}"#).unwrap();
        std::fs::write(dir.join("b.json"), r#"{"extends": ["a.json"]}"#).unwrap();

        let result = FallowConfig::load(&dir.join("a.json"));
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("Circular extends"),
            "Expected circular error, got: {err_msg}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extends_missing_file_errors() {
        let dir = test_dir("extends-missing");

        std::fs::write(
            dir.join(".fallowrc.json"),
            r#"{"extends": ["nonexistent.json"]}"#,
        )
        .unwrap();

        let result = FallowConfig::load(&dir.join(".fallowrc.json"));
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("not found"),
            "Expected not found error, got: {err_msg}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extends_string_sugar() {
        let dir = test_dir("extends-string");

        std::fs::write(dir.join("base.json"), r#"{"ignorePatterns": ["gen/**"]}"#).unwrap();
        // String form instead of array
        std::fs::write(dir.join(".fallowrc.json"), r#"{"extends": "base.json"}"#).unwrap();

        let config = FallowConfig::load(&dir.join(".fallowrc.json")).unwrap();
        assert_eq!(config.ignore_patterns, vec!["gen/**"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extends_deep_merge_preserves_arrays() {
        let dir = test_dir("extends-array");

        std::fs::write(dir.join("base.json"), r#"{"entry": ["src/a.ts"]}"#).unwrap();
        std::fs::write(
            dir.join(".fallowrc.json"),
            r#"{"extends": ["base.json"], "entry": ["src/b.ts"]}"#,
        )
        .unwrap();

        let config = FallowConfig::load(&dir.join(".fallowrc.json")).unwrap();
        // Arrays are replaced, not merged (overlay replaces base)
        assert_eq!(config.entry, vec!["src/b.ts"]);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
