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

const TOOLING_DEPENDENCIES: &[&str] = &[
    "eslint",
    "@eslint/js",
    "@eslint/eslintrc",
    "eslint-config-next",
    "eslint-config-prettier",
    "eslint-plugin-react",
    "eslint-plugin-react-hooks",
    "eslint-plugin-jsx-a11y",
    "eslint-plugin-import",
    "@typescript-eslint/parser",
    "@typescript-eslint/eslint-plugin",
    "prettier",
    "eslint-plugin-prettier",
    "eslint-plugin-sonarjs",
    "eslint-plugin-storybook",
];

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

    fn used_exports(&self) -> Vec<(&'static str, &'static [&'static str])> {
        vec![(
            "eslint.config.{js,mjs,cjs,ts,mts,cts}",
            ESLINT_CONFIG_EXPORTS,
        )]
    }

    fn resolve_config(&self, config_path: &Path, source: &str, _root: &Path) -> PluginResult {
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

        result
    }
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
        let source = r#"
            import react from 'eslint-plugin-react';
            import tseslint from 'typescript-eslint';
            export default [{}];
        "#;
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
}
