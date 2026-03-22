//! Drizzle ORM plugin.
//!
//! Detects Drizzle projects and marks migration/schema files as entry points.
//! Parses drizzle.config to extract the `schema` field (making schema file
//! exports framework-consumed entry points) and the `out` field (custom
//! migration output directory). Also extracts referenced dependencies from
//! import statements.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct DrizzlePlugin;

const ENABLERS: &[&str] = &["drizzle-orm"];

const ENTRY_PATTERNS: &[&str] = &["drizzle/**/*.{ts,js}"];

const CONFIG_PATTERNS: &[&str] = &["drizzle.config.{ts,js,mjs}"];

const ALWAYS_USED: &[&str] = &["drizzle.config.{ts,js,mjs}"];

const TOOLING_DEPENDENCIES: &[&str] = &["drizzle-orm", "drizzle-kit"];

impl Plugin for DrizzlePlugin {
    fn name(&self) -> &'static str {
        "drizzle"
    }

    fn enablers(&self) -> &'static [&'static str] {
        ENABLERS
    }

    fn entry_patterns(&self) -> &'static [&'static str] {
        ENTRY_PATTERNS
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

    fn resolve_config(&self, config_path: &Path, source: &str, _root: &Path) -> PluginResult {
        let mut result = PluginResult::default();

        // Extract import sources as referenced dependencies
        let imports = config_parser::extract_imports(source, config_path);
        for imp in &imports {
            let dep = crate::resolve::extract_package_name(imp);
            result.referenced_dependencies.push(dep);
        }

        // Extract `schema` field → entry patterns for schema files.
        // Drizzle schema files export tables, relations, and enums that are
        // consumed by the Drizzle runtime (via `drizzle()` init) and by
        // drizzle-kit (for migrations). These exports are never directly
        // imported in user code, so without this they appear as false positives.
        //
        // The `schema` field accepts:
        //   - A single path:  `schema: "./src/db/schema.ts"`
        //   - A glob pattern: `schema: "./src/db/*.ts"`
        //   - An array:       `schema: ["./src/db/users.ts", "./src/db/posts.ts"]`
        //   - A directory:    `schema: "./src/db/schema"` (Drizzle scans recursively)
        let schema_paths =
            config_parser::extract_config_string_or_array(source, config_path, &["schema"]);
        for path in &schema_paths {
            result
                .entry_patterns
                .extend(schema_path_to_entry_patterns(path));
        }

        // Extract `out` field → custom migration output directory.
        // Default is `drizzle/` (covered by static ENTRY_PATTERNS), but users
        // can configure a different directory.
        if let Some(out_dir) = config_parser::extract_config_string(source, config_path, &["out"]) {
            let out = out_dir.trim_start_matches("./").trim_end_matches('/');
            if out != "drizzle" {
                result.entry_patterns.push(format!("{out}/**/*.{{ts,js}}"));
            }
        }

        result
    }
}

/// Convert a schema path from drizzle.config into entry patterns.
///
/// Returns one or more glob patterns:
/// - Glob patterns (`src/db/*.ts`) → used as-is
/// - Directory paths (`src/db/schema`) → `dir/**/*.{ts,...}`
/// - Index/barrel files (`src/db/schema/index.ts`) → the file itself PLUS
///   `dir/**/*.{ts,...}` for siblings, because Drizzle follows imports from
///   the barrel to discover all schema files
/// - Other files (`src/db/schema.ts`) → just the file
fn schema_path_to_entry_patterns(path: &str) -> Vec<String> {
    let path = path.trim_start_matches("./");

    // If it contains glob characters, use as-is
    if path.contains('*') || path.contains('?') || path.contains('{') || path.contains('[') {
        return vec![path.to_string()];
    }

    // If it has a recognized JS/TS extension, it's a file
    if let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str())
        && matches!(
            ext,
            "ts" | "tsx" | "js" | "jsx" | "mts" | "mjs" | "cts" | "cjs"
        )
    {
        let mut patterns = vec![path.to_string()];

        // If this is an index/barrel file, also add the parent directory.
        // Drizzle follows imports from the barrel to discover all schema files,
        // so siblings (relations.ts, users.ts, etc.) should also be entry points.
        if let Some(stem) = Path::new(path).file_stem().and_then(|s| s.to_str())
            && stem == "index"
            && let Some(parent) = Path::new(path).parent()
            && parent != Path::new("")
        {
            let dir = parent.to_string_lossy();
            patterns.push(format!("{dir}/**/*.{{ts,tsx,js,jsx,mts,mjs,cts,cjs}}"));
        }

        return patterns;
    }

    // Otherwise, treat as a directory — Drizzle scans recursively
    vec![format!("{path}/**/*.{{ts,tsx,js,jsx,mts,mjs,cts,cjs}}")]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_path_file() {
        assert_eq!(
            schema_path_to_entry_patterns("./src/db/schema.ts"),
            vec!["src/db/schema.ts"]
        );
    }

    #[test]
    fn schema_path_index_file_adds_directory() {
        assert_eq!(
            schema_path_to_entry_patterns("./src/db/schema/index.ts"),
            vec![
                "src/db/schema/index.ts".to_string(),
                "src/db/schema/**/*.{ts,tsx,js,jsx,mts,mjs,cts,cjs}".to_string(),
            ]
        );
    }

    #[test]
    fn schema_path_directory() {
        assert_eq!(
            schema_path_to_entry_patterns("./src/db/schema"),
            vec!["src/db/schema/**/*.{ts,tsx,js,jsx,mts,mjs,cts,cjs}"]
        );
    }

    #[test]
    fn schema_path_glob() {
        assert_eq!(
            schema_path_to_entry_patterns("./src/db/*.ts"),
            vec!["src/db/*.ts"]
        );
    }

    #[test]
    fn schema_path_no_prefix() {
        assert_eq!(
            schema_path_to_entry_patterns("src/db/schema.ts"),
            vec!["src/db/schema.ts"]
        );
    }

    #[test]
    fn resolve_config_extracts_schema_string() {
        let source = r#"
            import { defineConfig } from "drizzle-kit";
            export default defineConfig({
                schema: "./src/db/schema.ts",
                out: "./drizzle",
                dialect: "postgresql",
            });
        "#;
        let plugin = DrizzlePlugin;
        let result = plugin.resolve_config(
            Path::new("drizzle.config.ts"),
            source,
            Path::new("/project"),
        );
        assert!(
            result
                .entry_patterns
                .contains(&"src/db/schema.ts".to_string())
        );
        assert!(
            result
                .referenced_dependencies
                .contains(&"drizzle-kit".to_string())
        );
    }

    #[test]
    fn resolve_config_extracts_schema_array() {
        let source = r#"
            export default {
                schema: ["./src/db/users.ts", "./src/db/posts.ts"],
                out: "./drizzle",
                dialect: "postgresql",
            };
        "#;
        let plugin = DrizzlePlugin;
        let result = plugin.resolve_config(
            Path::new("drizzle.config.ts"),
            source,
            Path::new("/project"),
        );
        assert!(
            result
                .entry_patterns
                .contains(&"src/db/users.ts".to_string())
        );
        assert!(
            result
                .entry_patterns
                .contains(&"src/db/posts.ts".to_string())
        );
    }

    #[test]
    fn resolve_config_extracts_schema_directory() {
        let source = r#"
            export default {
                schema: "./src/db",
                dialect: "postgresql",
            };
        "#;
        let plugin = DrizzlePlugin;
        let result = plugin.resolve_config(
            Path::new("drizzle.config.ts"),
            source,
            Path::new("/project"),
        );
        assert!(
            result
                .entry_patterns
                .contains(&"src/db/**/*.{ts,tsx,js,jsx,mts,mjs,cts,cjs}".to_string())
        );
    }

    #[test]
    fn resolve_config_extracts_schema_glob() {
        let source = r#"
            export default {
                schema: "./src/db/*.ts",
                dialect: "postgresql",
            };
        "#;
        let plugin = DrizzlePlugin;
        let result = plugin.resolve_config(
            Path::new("drizzle.config.ts"),
            source,
            Path::new("/project"),
        );
        assert!(result.entry_patterns.contains(&"src/db/*.ts".to_string()));
    }

    #[test]
    fn resolve_config_custom_out_dir() {
        let source = r#"
            export default {
                schema: "./src/db/schema.ts",
                out: "./migrations",
                dialect: "postgresql",
            };
        "#;
        let plugin = DrizzlePlugin;
        let result = plugin.resolve_config(
            Path::new("drizzle.config.ts"),
            source,
            Path::new("/project"),
        );
        assert!(
            result
                .entry_patterns
                .contains(&"src/db/schema.ts".to_string())
        );
        assert!(
            result
                .entry_patterns
                .contains(&"migrations/**/*.{ts,js}".to_string())
        );
    }

    #[test]
    fn resolve_config_default_out_dir_not_duplicated() {
        let source = r#"
            export default {
                schema: "./src/db/schema.ts",
                out: "./drizzle",
                dialect: "postgresql",
            };
        "#;
        let plugin = DrizzlePlugin;
        let result = plugin.resolve_config(
            Path::new("drizzle.config.ts"),
            source,
            Path::new("/project"),
        );
        // The default "drizzle/" out dir is already covered by static ENTRY_PATTERNS,
        // so resolve_config should NOT add a duplicate.
        assert!(
            !result
                .entry_patterns
                .iter()
                .any(|p| p.starts_with("drizzle/"))
        );
    }

    #[test]
    fn resolve_config_module_exports() {
        let source = r#"
            module.exports = {
                schema: "./src/db/schema",
                out: "./migrations",
                dialect: "mysql",
            };
        "#;
        let plugin = DrizzlePlugin;
        let result = plugin.resolve_config(
            Path::new("drizzle.config.js"),
            source,
            Path::new("/project"),
        );
        assert!(
            result
                .entry_patterns
                .contains(&"src/db/schema/**/*.{ts,tsx,js,jsx,mts,mjs,cts,cjs}".to_string())
        );
        assert!(
            result
                .entry_patterns
                .contains(&"migrations/**/*.{ts,js}".to_string())
        );
    }
}
