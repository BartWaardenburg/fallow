use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A declarative plugin definition loaded from a standalone TOML file.
///
/// External plugins provide the same static pattern capabilities as built-in
/// plugins (entry points, always-used files, used exports, tooling dependencies),
/// but are defined in standalone TOML files rather than compiled Rust code.
///
/// They cannot do AST-based config parsing (`resolve_config()`), but cover the
/// vast majority of framework integration use cases.
///
/// # File format
///
/// ```toml
/// name = "my-framework"
/// enablers = ["my-framework", "@my-framework/core"]
/// entry_points = ["src/routes/**/*.{ts,tsx}"]
/// config_patterns = ["my-framework.config.{ts,js}"]
/// always_used = ["src/setup.ts"]
/// tooling_dependencies = ["my-framework-cli"]
///
/// [[used_exports]]
/// pattern = "src/routes/**/*.{ts,tsx}"
/// exports = ["default", "loader", "action"]
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct ExternalPluginDef {
    /// Unique name for this plugin.
    pub name: String,

    /// Package names that activate this plugin when found in package.json.
    /// Supports exact matches and prefix patterns (ending with `/`).
    #[serde(default)]
    pub enablers: Vec<String>,

    /// Glob patterns for entry point files.
    #[serde(default)]
    pub entry_points: Vec<String>,

    /// Glob patterns for config files (marked as always-used when active).
    #[serde(default)]
    pub config_patterns: Vec<String>,

    /// Files that are always considered "used" when this plugin is active.
    #[serde(default)]
    pub always_used: Vec<String>,

    /// Dependencies that are tooling (used via CLI/config, not source imports).
    /// These should not be flagged as unused devDependencies.
    #[serde(default)]
    pub tooling_dependencies: Vec<String>,

    /// Exports that are always considered used for matching file patterns.
    #[serde(default)]
    pub used_exports: Vec<ExternalUsedExport>,
}

/// Exports considered used for files matching a pattern.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct ExternalUsedExport {
    /// Glob pattern for files.
    pub pattern: String,
    /// Export names always considered used.
    pub exports: Vec<String>,
}

/// Discover and load external plugin definitions for a project.
///
/// Discovery order (first occurrence of a plugin name wins):
/// 1. Paths from the `plugins` config field (files or directories)
/// 2. `.fallow/plugins/` directory (auto-discover `*.toml` files)
/// 3. Project root `fallow-plugin-*.toml` files
pub fn discover_external_plugins(
    root: &Path,
    config_plugin_paths: &[String],
) -> Vec<ExternalPluginDef> {
    let mut plugins = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    // All paths are checked against the canonical root to prevent symlink escapes
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    // 1. Explicit paths from config
    for path_str in config_plugin_paths {
        let path = root.join(path_str);
        if !is_within_root(&path, &canonical_root) {
            eprintln!(
                "Warning: plugin path '{}' resolves outside project root, skipping",
                path_str
            );
            continue;
        }
        if path.is_dir() {
            load_plugins_from_dir(&path, &canonical_root, &mut plugins, &mut seen_names);
        } else if path.is_file() {
            load_plugin_file(&path, &canonical_root, &mut plugins, &mut seen_names);
        }
    }

    // 2. .fallow/plugins/ directory
    let plugins_dir = root.join(".fallow").join("plugins");
    if plugins_dir.is_dir() && is_within_root(&plugins_dir, &canonical_root) {
        load_plugins_from_dir(&plugins_dir, &canonical_root, &mut plugins, &mut seen_names);
    }

    // 3. Project root fallow-plugin-*.toml files
    if let Ok(entries) = std::fs::read_dir(root) {
        let mut plugin_files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.is_file()
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("fallow-plugin-") && n.ends_with(".toml"))
            })
            .collect();
        plugin_files.sort();
        for path in plugin_files {
            load_plugin_file(&path, &canonical_root, &mut plugins, &mut seen_names);
        }
    }

    plugins
}

/// Check if a path resolves within the canonical root (follows symlinks).
fn is_within_root(path: &Path, canonical_root: &Path) -> bool {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    canonical.starts_with(canonical_root)
}

fn load_plugins_from_dir(
    dir: &Path,
    canonical_root: &Path,
    plugins: &mut Vec<ExternalPluginDef>,
    seen: &mut std::collections::HashSet<String>,
) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut toml_files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("toml"))
            .collect();
        toml_files.sort();
        for path in toml_files {
            load_plugin_file(&path, canonical_root, plugins, seen);
        }
    }
}

fn load_plugin_file(
    path: &Path,
    canonical_root: &Path,
    plugins: &mut Vec<ExternalPluginDef>,
    seen: &mut std::collections::HashSet<String>,
) {
    // Verify symlinks don't escape the project root
    if !is_within_root(path, canonical_root) {
        eprintln!(
            "Warning: plugin file '{}' resolves outside project root (symlink?), skipping",
            path.display()
        );
        return;
    }
    match std::fs::read_to_string(path) {
        Ok(content) => match toml::from_str::<ExternalPluginDef>(&content) {
            Ok(plugin) => {
                if plugin.name.is_empty() {
                    eprintln!(
                        "Warning: external plugin in {} has an empty name, skipping",
                        path.display()
                    );
                    return;
                }
                if seen.insert(plugin.name.clone()) {
                    plugins.push(plugin);
                } else {
                    eprintln!(
                        "Warning: duplicate external plugin '{}' in {}, skipping",
                        plugin.name,
                        path.display()
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "Warning: failed to parse external plugin {}: {e}",
                    path.display()
                );
            }
        },
        Err(e) => {
            eprintln!(
                "Warning: failed to read external plugin file {}: {e}",
                path.display()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_minimal_plugin() {
        let toml_str = r#"
name = "my-plugin"
enablers = ["my-pkg"]
"#;
        let plugin: ExternalPluginDef = toml::from_str(toml_str).unwrap();
        assert_eq!(plugin.name, "my-plugin");
        assert_eq!(plugin.enablers, vec!["my-pkg"]);
        assert!(plugin.entry_points.is_empty());
        assert!(plugin.always_used.is_empty());
        assert!(plugin.config_patterns.is_empty());
        assert!(plugin.tooling_dependencies.is_empty());
        assert!(plugin.used_exports.is_empty());
    }

    #[test]
    fn deserialize_full_plugin() {
        let toml_str = r#"
name = "my-framework"
enablers = ["my-framework", "@my-framework/core"]
entry_points = ["src/routes/**/*.{ts,tsx}", "src/middleware.ts"]
config_patterns = ["my-framework.config.{ts,js,mjs}"]
always_used = ["src/setup.ts", "public/**/*"]
tooling_dependencies = ["my-framework-cli"]

[[used_exports]]
pattern = "src/routes/**/*.{ts,tsx}"
exports = ["default", "loader", "action"]

[[used_exports]]
pattern = "src/middleware.ts"
exports = ["default"]
"#;
        let plugin: ExternalPluginDef = toml::from_str(toml_str).unwrap();
        assert_eq!(plugin.name, "my-framework");
        assert_eq!(plugin.enablers.len(), 2);
        assert_eq!(plugin.entry_points.len(), 2);
        assert_eq!(
            plugin.config_patterns,
            vec!["my-framework.config.{ts,js,mjs}"]
        );
        assert_eq!(plugin.always_used.len(), 2);
        assert_eq!(plugin.tooling_dependencies, vec!["my-framework-cli"]);
        assert_eq!(plugin.used_exports.len(), 2);
        assert_eq!(plugin.used_exports[0].pattern, "src/routes/**/*.{ts,tsx}");
        assert_eq!(
            plugin.used_exports[0].exports,
            vec!["default", "loader", "action"]
        );
    }

    #[test]
    fn discover_plugins_from_fallow_plugins_dir() {
        let dir =
            std::env::temp_dir().join(format!("fallow-test-ext-plugins-{}", std::process::id()));
        let plugins_dir = dir.join(".fallow").join("plugins");
        let _ = std::fs::create_dir_all(&plugins_dir);

        std::fs::write(
            plugins_dir.join("my-plugin.toml"),
            r#"
name = "my-plugin"
enablers = ["my-pkg"]
entry_points = ["src/**/*.ts"]
"#,
        )
        .unwrap();

        let plugins = discover_external_plugins(&dir, &[]);
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "my-plugin");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn discover_fallow_plugin_files_in_root() {
        let dir =
            std::env::temp_dir().join(format!("fallow-test-root-plugins-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);

        std::fs::write(
            dir.join("fallow-plugin-custom.toml"),
            r#"
name = "custom"
enablers = ["custom-pkg"]
"#,
        )
        .unwrap();

        // Non-matching file should be ignored
        std::fs::write(dir.join("some-other-file.toml"), r#"name = "ignored""#).unwrap();

        let plugins = discover_external_plugins(&dir, &[]);
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "custom");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn deduplicates_by_name() {
        let dir =
            std::env::temp_dir().join(format!("fallow-test-dedup-plugins-{}", std::process::id()));
        let plugins_dir = dir.join(".fallow").join("plugins");
        let _ = std::fs::create_dir_all(&plugins_dir);

        // Same name in .fallow/plugins/ and root
        std::fs::write(
            plugins_dir.join("my-plugin.toml"),
            r#"
name = "my-plugin"
enablers = ["pkg-a"]
"#,
        )
        .unwrap();

        std::fs::write(
            dir.join("fallow-plugin-my-plugin.toml"),
            r#"
name = "my-plugin"
enablers = ["pkg-b"]
"#,
        )
        .unwrap();

        let plugins = discover_external_plugins(&dir, &[]);
        assert_eq!(plugins.len(), 1);
        // First one wins (.fallow/plugins/ before root)
        assert_eq!(plugins[0].enablers, vec!["pkg-a"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_plugin_paths_take_priority() {
        let dir =
            std::env::temp_dir().join(format!("fallow-test-config-paths-{}", std::process::id()));
        let custom_dir = dir.join("custom-plugins");
        let _ = std::fs::create_dir_all(&custom_dir);

        std::fs::write(
            custom_dir.join("explicit.toml"),
            r#"
name = "explicit"
enablers = ["explicit-pkg"]
"#,
        )
        .unwrap();

        let plugins = discover_external_plugins(&dir, &["custom-plugins".to_string()]);
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "explicit");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_plugin_path_to_single_file() {
        let dir =
            std::env::temp_dir().join(format!("fallow-test-single-file-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);

        std::fs::write(
            dir.join("my-plugin.toml"),
            r#"
name = "single-file"
enablers = ["single-pkg"]
"#,
        )
        .unwrap();

        let plugins = discover_external_plugins(&dir, &["my-plugin.toml".to_string()]);
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "single-file");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn skips_invalid_toml() {
        let dir =
            std::env::temp_dir().join(format!("fallow-test-invalid-plugin-{}", std::process::id()));
        let plugins_dir = dir.join(".fallow").join("plugins");
        let _ = std::fs::create_dir_all(&plugins_dir);

        // Invalid: missing required `name` field
        std::fs::write(plugins_dir.join("bad.toml"), r#"enablers = ["pkg"]"#).unwrap();

        // Valid
        std::fs::write(
            plugins_dir.join("good.toml"),
            r#"
name = "good"
enablers = ["good-pkg"]
"#,
        )
        .unwrap();

        let plugins = discover_external_plugins(&dir, &[]);
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "good");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn prefix_enablers() {
        let toml_str = r#"
name = "scoped"
enablers = ["@myorg/"]
"#;
        let plugin: ExternalPluginDef = toml::from_str(toml_str).unwrap();
        assert_eq!(plugin.enablers, vec!["@myorg/"]);
    }

    #[test]
    fn skips_empty_name() {
        let dir =
            std::env::temp_dir().join(format!("fallow-test-empty-name-{}", std::process::id()));
        let plugins_dir = dir.join(".fallow").join("plugins");
        let _ = std::fs::create_dir_all(&plugins_dir);

        std::fs::write(
            plugins_dir.join("empty.toml"),
            r#"
name = ""
enablers = ["pkg"]
"#,
        )
        .unwrap();

        let plugins = discover_external_plugins(&dir, &[]);
        assert!(plugins.is_empty(), "empty-name plugin should be skipped");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_paths_outside_root() {
        let dir =
            std::env::temp_dir().join(format!("fallow-test-path-escape-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);

        // Attempt to load a plugin from outside the project root
        let plugins = discover_external_plugins(&dir, &["../../../etc".to_string()]);
        assert!(plugins.is_empty(), "paths outside root should be rejected");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
