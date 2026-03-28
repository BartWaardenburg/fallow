//! `ESLint` plugin.
//!
//! Detects `ESLint` projects and marks config files as always used.
//! Parses `ESLint` config to extract plugin/config imports as referenced dependencies.
//! Also covers Prettier and lint-staged config files.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct EslintPlugin;

const ENABLERS: &[&str] = &["eslint", "@eslint/js"];

const CONFIG_PATTERNS: &[&str] = &[
    "eslint.config.{js,mjs,cjs,ts,mts,cts}",
    ".eslintrc.{js,cjs,mjs,json,yaml,yml}",
];

const ALWAYS_USED: &[&str] = &[
    "eslint.config.{js,mjs,cjs,ts,mts,cts}",
    ".eslintrc.{js,cjs,mjs,json,yaml,yml}",
    ".prettierrc.{js,cjs,mjs,json,yaml,yml}",
    "prettier.config.{js,mjs,cjs,ts}",
    ".lintstagedrc.{js,cjs,mjs,json}",
    "lint-staged.config.{js,mjs,cjs}",
];

const TOOLING_DEPENDENCIES: &[&str] = &["eslint"];

const ESLINT_CONFIG_EXPORTS: &[&str] = &["default"];

/// ESLint config filenames to check for file-based activation.
/// In monorepos, `eslint` is typically only in the root package.json, but
/// workspace packages still have their own ESLint config files.
const ESLINT_CONFIG_FILES: &[&str] = &[
    "eslint.config.js",
    "eslint.config.mjs",
    "eslint.config.cjs",
    "eslint.config.ts",
    "eslint.config.mts",
    "eslint.config.cts",
    ".eslintrc.js",
    ".eslintrc.cjs",
    ".eslintrc.json",
    ".eslintrc.yml",
    ".eslintrc.yaml",
];

impl Plugin for EslintPlugin {
    fn name(&self) -> &'static str {
        "eslint"
    }

    fn enablers(&self) -> &'static [&'static str] {
        ENABLERS
    }

    /// Activate when `eslint` is in deps OR when an ESLint config file exists.
    /// In monorepos, `eslint` is usually only in the root package.json, but
    /// workspace packages have their own config files and ESLint-related devDeps.
    fn is_enabled_with_deps(&self, deps: &[String], root: &Path) -> bool {
        // Standard enabler check
        let enablers = self.enablers();
        if enablers.iter().any(|e| deps.iter().any(|d| d == e)) {
            return true;
        }
        // File-based activation: check for ESLint config files in the workspace root
        ESLINT_CONFIG_FILES.iter().any(|f| root.join(f).exists())
    }

    fn config_patterns(&self) -> &'static [&'static str] {
        CONFIG_PATTERNS
    }

    fn always_used(&self) -> &'static [&'static str] {
        ALWAYS_USED
    }

    fn tooling_dependencies(&self) -> &'static [&'static str] {
        TOOLING_DEPENDENCIES
    }

    fn package_json_config_key(&self) -> Option<&'static str> {
        Some("eslintConfig")
    }

    fn used_exports(&self) -> Vec<(&'static str, &'static [&'static str])> {
        vec![(
            "eslint.config.{js,mjs,cjs,ts,mts,cts}",
            ESLINT_CONFIG_EXPORTS,
        )]
    }

    fn resolve_config(&self, config_path: &Path, source: &str, root: &Path) -> PluginResult {
        let mut result = PluginResult::default();

        // For JSON configs, wrap in parens so Oxc can parse them
        let is_json = config_path.extension().is_some_and(|ext| ext == "json");
        let (parse_source, parse_path_buf) = if is_json {
            (format!("({source})"), config_path.with_extension("js"))
        } else {
            (source.to_string(), config_path.to_path_buf())
        };
        let parse_path: &Path = &parse_path_buf;

        // Extract import sources as referenced dependencies (eslint plugins, configs)
        let imports = config_parser::extract_imports(&parse_source, parse_path);
        for imp in &imports {
            let dep = crate::resolve::extract_package_name(imp);
            result.referenced_dependencies.push(dep);
        }

        // Follow shared config imports one level deep to discover peer deps.
        // e.g. eslint.config.js imports @sveltejs/eslint-config, which internally
        // imports typescript-eslint, eslint-plugin-svelte, @eslint/js — all peer deps
        // that the host project must install.
        for imp in &imports {
            let pkg_name = crate::resolve::extract_package_name(imp);
            if let Some((entry_source, entry_path)) = read_package_entry(root, &pkg_name) {
                let nested = config_parser::extract_imports(&entry_source, &entry_path);
                for nested_imp in &nested {
                    result
                        .referenced_dependencies
                        .push(crate::resolve::extract_package_name(nested_imp));
                }
            }
        }

        // Legacy .eslintrc: extract plugins by short name
        // e.g. plugins: ["react"] → eslint-plugin-react
        let plugins =
            config_parser::extract_config_shallow_strings(&parse_source, parse_path, "plugins");
        for plugin in &plugins {
            result
                .referenced_dependencies
                .push(resolve_eslint_plugin_name(plugin));
        }

        // Legacy .eslintrc: extract extends
        // e.g. extends: ["airbnb", "plugin:react/recommended"]
        let extends =
            config_parser::extract_config_shallow_strings(&parse_source, parse_path, "extends");
        for ext in &extends {
            if let Some(dep) = resolve_eslint_extends_name(ext) {
                result.referenced_dependencies.push(dep);
            }
        }

        // Legacy .eslintrc: extract parser
        // e.g. parser: "@typescript-eslint/parser"
        if let Some(parser) =
            config_parser::extract_config_string(&parse_source, parse_path, &["parser"])
        {
            let dep = crate::resolve::extract_package_name(&parser);
            result.referenced_dependencies.push(dep);
        }

        // Flat config: extract plugin names from plugins object keys
        // e.g. plugins: { react: reactPlugin, "@typescript-eslint": tseslint }
        let plugin_keys =
            config_parser::extract_config_object_keys(&parse_source, parse_path, &["plugins"]);
        for key in &plugin_keys {
            result
                .referenced_dependencies
                .push(resolve_eslint_plugin_name(key));
        }

        // settings["import/resolver"] → resolver package dependencies
        // Handles three formats:
        //   Object: { typescript: { project: "..." } } → eslint-import-resolver-typescript
        //   String: "typescript" → eslint-import-resolver-typescript
        //   Array:  ["typescript", "node"] → eslint-import-resolver-typescript
        let resolver_path = &["settings", "import/resolver"];
        let resolver_keys =
            config_parser::extract_config_object_keys(&parse_source, parse_path, resolver_path);
        for key in &resolver_keys {
            if let Some(dep) = resolve_eslint_resolver_name(key) {
                result.referenced_dependencies.push(dep);
            }
        }
        if let Some(resolver) =
            config_parser::extract_config_string(&parse_source, parse_path, resolver_path)
            && let Some(dep) = resolve_eslint_resolver_name(&resolver)
        {
            result.referenced_dependencies.push(dep);
        }
        let resolver_strings =
            config_parser::extract_config_string_array(&parse_source, parse_path, resolver_path);
        for resolver in &resolver_strings {
            if let Some(dep) = resolve_eslint_resolver_name(resolver) {
                result.referenced_dependencies.push(dep);
            }
        }

        result
    }
}

/// Read a package's entry point source from node_modules.
///
/// Resolves the package's `module` or `main` field from its `package.json`,
/// reads the entry file, and returns its source and path. Returns `None` if
/// the package is not found or the entry file is unreadable.
fn read_package_entry(root: &Path, pkg_name: &str) -> Option<(String, std::path::PathBuf)> {
    let pkg_dir = root.join("node_modules").join(pkg_name);
    let pkg_json_str = std::fs::read_to_string(pkg_dir.join("package.json")).ok()?;
    let pkg_json: serde_json::Value = serde_json::from_str(&pkg_json_str).ok()?;

    // Resolve entry point: "exports"."." → "module" → "main" → "index.js"
    let entry_rel = pkg_json
        .get("exports")
        .and_then(|e| {
            // "exports": "./index.js" (string shorthand)
            e.as_str().or_else(|| {
                // "exports": { ".": "./index.js" } or { ".": { "import": "./index.mjs" } }
                e.get(".").and_then(|dot| {
                    dot.as_str()
                        .or_else(|| dot.get("import").and_then(|v| v.as_str()))
                        .or_else(|| dot.get("default").and_then(|v| v.as_str()))
                })
            })
        })
        .or_else(|| pkg_json.get("module").and_then(|v| v.as_str()))
        .or_else(|| pkg_json.get("main").and_then(|v| v.as_str()))
        .unwrap_or("index.js");

    let entry_path = pkg_dir.join(entry_rel);
    let source = std::fs::read_to_string(&entry_path).ok()?;
    Some((source, entry_path))
}

/// Resolve `ESLint` plugin short name to full package name.
///
/// - `"react"` → `"eslint-plugin-react"`
/// - `"@typescript-eslint"` → `"@typescript-eslint/eslint-plugin"`
/// - `"eslint-plugin-react"` → `"eslint-plugin-react"` (already full)
fn resolve_eslint_plugin_name(name: &str) -> String {
    if name.starts_with("eslint-plugin-") || name.contains("/eslint-plugin") {
        name.to_string()
    } else if let Some(scope) = name.strip_prefix('@') {
        if scope.contains('/') {
            // Already scoped with subpath, push as-is
            name.to_string()
        } else {
            // "@typescript-eslint" → "@typescript-eslint/eslint-plugin"
            format!("{name}/eslint-plugin")
        }
    } else {
        format!("eslint-plugin-{name}")
    }
}

/// Resolve `ESLint` extends name to a package dependency.
///
/// - `"airbnb"` → `"eslint-config-airbnb"`
/// - `"plugin:react/recommended"` → `"eslint-plugin-react"`
/// - `"eslint:recommended"` → `None` (built-in)
fn resolve_eslint_extends_name(name: &str) -> Option<String> {
    if name.starts_with("eslint:") {
        // Built-in ESLint config
        None
    } else if let Some(rest) = name.strip_prefix("plugin:") {
        // "plugin:react/recommended" → extract plugin name
        let plugin_name = rest.split('/').next()?;
        Some(resolve_eslint_plugin_name(plugin_name))
    } else if name.starts_with("eslint-config-") || name.contains("/eslint-config") {
        Some(name.to_string())
    } else if name.starts_with('@') {
        // Scoped package, push as-is
        Some(name.to_string())
    } else {
        Some(format!("eslint-config-{name}"))
    }
}

/// Resolve ESLint import resolver name to a package dependency.
///
/// - `"typescript"` → `"eslint-import-resolver-typescript"`
/// - `"node"` → `None` (built-in to eslint-plugin-import)
/// - `"eslint-import-resolver-typescript"` → `"eslint-import-resolver-typescript"` (already full)
fn resolve_eslint_resolver_name(name: &str) -> Option<String> {
    if matches!(name, "node" | "webpack") {
        // Built-in resolvers provided by eslint-plugin-import
        None
    } else if name.starts_with("eslint-import-resolver-") {
        Some(name.to_string())
    } else {
        Some(format!("eslint-import-resolver-{name}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ESLint plugin name resolution ───────────────────────────────

    #[test]
    fn plugin_short_name() {
        assert_eq!(resolve_eslint_plugin_name("react"), "eslint-plugin-react");
    }

    #[test]
    fn plugin_scoped_short_name() {
        assert_eq!(
            resolve_eslint_plugin_name("@typescript-eslint"),
            "@typescript-eslint/eslint-plugin"
        );
    }

    #[test]
    fn plugin_already_full_name() {
        assert_eq!(
            resolve_eslint_plugin_name("eslint-plugin-react"),
            "eslint-plugin-react"
        );
    }

    #[test]
    fn plugin_scoped_with_subpath() {
        assert_eq!(
            resolve_eslint_plugin_name("@scope/some-plugin"),
            "@scope/some-plugin"
        );
    }

    // ── ESLint extends name resolution ──────────────────────────────

    #[test]
    fn extends_short_name() {
        assert_eq!(
            resolve_eslint_extends_name("airbnb"),
            Some("eslint-config-airbnb".to_string())
        );
    }

    #[test]
    fn extends_plugin_rule() {
        assert_eq!(
            resolve_eslint_extends_name("plugin:react/recommended"),
            Some("eslint-plugin-react".to_string())
        );
    }

    #[test]
    fn extends_plugin_scoped() {
        assert_eq!(
            resolve_eslint_extends_name("plugin:@typescript-eslint/recommended"),
            Some("@typescript-eslint/eslint-plugin".to_string())
        );
    }

    #[test]
    fn extends_builtin() {
        assert_eq!(resolve_eslint_extends_name("eslint:recommended"), None);
    }

    #[test]
    fn extends_already_full_config_name() {
        assert_eq!(
            resolve_eslint_extends_name("eslint-config-prettier"),
            Some("eslint-config-prettier".to_string())
        );
    }

    #[test]
    fn extends_scoped_package() {
        assert_eq!(
            resolve_eslint_extends_name("@vue/eslint-config-typescript"),
            Some("@vue/eslint-config-typescript".to_string())
        );
    }

    // ── ESLint resolve_config integration ───────────────────────────

    #[test]
    fn resolve_config_legacy_eslintrc() {
        let source = r#"
            module.exports = {
                parser: "@typescript-eslint/parser",
                plugins: ["react", "@typescript-eslint"],
                extends: ["airbnb", "plugin:react/recommended", "eslint:recommended"]
            };
        "#;
        let plugin = EslintPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new(".eslintrc.js"),
            source,
            std::path::Path::new("/project"),
        );

        let deps = &result.referenced_dependencies;
        assert!(deps.contains(&"@typescript-eslint/parser".to_string()));
        assert!(deps.contains(&"eslint-plugin-react".to_string()));
        assert!(deps.contains(&"@typescript-eslint/eslint-plugin".to_string()));
        assert!(deps.contains(&"eslint-config-airbnb".to_string()));
        // eslint:recommended should NOT be in deps
        assert!(!deps.iter().any(|d| d.contains("eslint:recommended")));
    }

    #[test]
    fn resolve_config_json_eslintrc() {
        let source = r#"{"plugins": ["react"], "extends": ["airbnb"]}"#;
        let plugin = EslintPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new(".eslintrc.json"),
            source,
            std::path::Path::new("/project"),
        );

        let deps = &result.referenced_dependencies;
        assert!(deps.contains(&"eslint-plugin-react".to_string()));
        assert!(deps.contains(&"eslint-config-airbnb".to_string()));
    }

    #[test]
    fn resolve_config_flat_config_imports() {
        let source = r"
            import react from 'eslint-plugin-react';
            import tseslint from 'typescript-eslint';
            export default [{}];
        ";
        let plugin = EslintPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("eslint.config.js"),
            source,
            std::path::Path::new("/project"),
        );

        let deps = &result.referenced_dependencies;
        assert!(deps.contains(&"eslint-plugin-react".to_string()));
        assert!(deps.contains(&"typescript-eslint".to_string()));
    }

    // ── Shared config following ─────────────────────────────────────

    #[test]
    fn shared_config_following_discovers_peer_deps() {
        // Create a temp dir with a mock shared config in node_modules
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Create node_modules/@mock/eslint-config with a package.json and index.js
        let pkg_dir = root.join("node_modules/@mock/eslint-config");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(
            pkg_dir.join("package.json"),
            r#"{"name": "@mock/eslint-config", "main": "index.js"}"#,
        )
        .unwrap();
        std::fs::write(
            pkg_dir.join("index.js"),
            r"
                import js from '@eslint/js';
                import ts from 'typescript-eslint';
                import svelte from 'eslint-plugin-svelte';
                export default [js.configs.recommended, ...ts.configs.recommended];
            ",
        )
        .unwrap();

        let source = r"
            import config from '@mock/eslint-config';
            export default [...config];
        ";
        let plugin = EslintPlugin;
        let result = plugin.resolve_config(std::path::Path::new("eslint.config.js"), source, root);

        let deps = &result.referenced_dependencies;
        // Direct import
        assert!(
            deps.contains(&"@mock/eslint-config".to_string()),
            "should find direct import"
        );
        // Peer deps from shared config's entry point
        assert!(
            deps.contains(&"@eslint/js".to_string()),
            "should find @eslint/js from shared config"
        );
        assert!(
            deps.contains(&"typescript-eslint".to_string()),
            "should find typescript-eslint from shared config"
        );
        assert!(
            deps.contains(&"eslint-plugin-svelte".to_string()),
            "should find eslint-plugin-svelte from shared config"
        );
    }

    #[test]
    fn shared_config_missing_node_modules_graceful() {
        // When node_modules doesn't exist, should not panic
        let source = r"
            import config from 'some-nonexistent-config';
            export default [...config];
        ";
        let plugin = EslintPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("eslint.config.js"),
            source,
            std::path::Path::new("/nonexistent"),
        );

        // Should still find the direct import
        assert!(
            result
                .referenced_dependencies
                .contains(&"some-nonexistent-config".to_string())
        );
    }

    #[test]
    fn read_package_entry_exports_field() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        let pkg_dir = root.join("node_modules/modern-pkg");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(
            pkg_dir.join("package.json"),
            r#"{"name": "modern-pkg", "exports": { ".": { "import": "./dist/index.mjs" } }}"#,
        )
        .unwrap();
        std::fs::create_dir_all(pkg_dir.join("dist")).unwrap();
        std::fs::write(
            pkg_dir.join("dist/index.mjs"),
            "import foo from 'some-dep'; export default foo;",
        )
        .unwrap();

        let (source, path) = super::read_package_entry(root, "modern-pkg").unwrap();
        assert!(source.contains("some-dep"));
        assert!(path.ends_with("dist/index.mjs"));
    }

    // ── ESLint resolver name resolution ────────────────────────────

    #[test]
    fn resolver_short_name() {
        assert_eq!(
            resolve_eslint_resolver_name("typescript"),
            Some("eslint-import-resolver-typescript".to_string())
        );
    }

    #[test]
    fn resolver_node_builtin() {
        assert_eq!(resolve_eslint_resolver_name("node"), None);
    }

    #[test]
    fn resolver_webpack_builtin() {
        assert_eq!(resolve_eslint_resolver_name("webpack"), None);
    }

    #[test]
    fn resolver_already_full_name() {
        assert_eq!(
            resolve_eslint_resolver_name("eslint-import-resolver-typescript"),
            Some("eslint-import-resolver-typescript".to_string())
        );
    }

    // ── ESLint resolver config integration ─────────────────────────

    #[test]
    fn resolve_config_resolver_object_keys() {
        let source = r#"
            module.exports = {
                settings: {
                    "import/resolver": {
                        typescript: { project: "./tsconfig.json" },
                        node: { extensions: [".js", ".ts"] }
                    }
                }
            };
        "#;
        let plugin = EslintPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new(".eslintrc.js"),
            source,
            std::path::Path::new("/project"),
        );

        let deps = &result.referenced_dependencies;
        assert!(
            deps.contains(&"eslint-import-resolver-typescript".to_string()),
            "should resolve typescript resolver"
        );
        // "node" is built-in, should NOT be added
        assert!(
            !deps.iter().any(|d| d == "eslint-import-resolver-node"),
            "node resolver is built-in"
        );
    }

    #[test]
    fn resolve_config_resolver_string() {
        let source = r#"
            module.exports = {
                settings: {
                    "import/resolver": "typescript"
                }
            };
        "#;
        let plugin = EslintPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new(".eslintrc.js"),
            source,
            std::path::Path::new("/project"),
        );

        assert!(
            result
                .referenced_dependencies
                .contains(&"eslint-import-resolver-typescript".to_string())
        );
    }

    #[test]
    fn resolve_config_resolver_json() {
        let source =
            r#"{"settings": {"import/resolver": {"typescript": {"project": "./tsconfig.json"}}}}"#;
        let plugin = EslintPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new(".eslintrc.json"),
            source,
            std::path::Path::new("/project"),
        );

        assert!(
            result
                .referenced_dependencies
                .contains(&"eslint-import-resolver-typescript".to_string())
        );
    }

    #[test]
    fn resolve_config_resolver_flat_config() {
        // Flat configs that use defineConfig() can have settings extracted
        let source = r#"
            import { defineConfig } from 'eslint/config';
            export default defineConfig({
                settings: {
                    "import/resolver": {
                        typescript: { alwaysTryTypes: true }
                    }
                }
            });
        "#;
        let plugin = EslintPlugin;
        let result = plugin.resolve_config(
            std::path::Path::new("eslint.config.mjs"),
            source,
            std::path::Path::new("/project"),
        );

        assert!(
            result
                .referenced_dependencies
                .contains(&"eslint-import-resolver-typescript".to_string())
        );
    }
}
