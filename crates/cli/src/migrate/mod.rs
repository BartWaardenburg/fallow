mod jscpd;
mod knip;

use std::fmt::Write as _;
use std::io::Read as _;
use std::path::Path;
use std::process::ExitCode;

use jscpd::migrate_jscpd;
use knip::migrate_knip;

/// A warning about a config field that could not be migrated.
struct MigrationWarning {
    source: &'static str,
    field: String,
    message: String,
    suggestion: Option<String>,
}

impl std::fmt::Display for MigrationWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] `{}`: {}", self.source, self.field, self.message)?;
        if let Some(ref suggestion) = self.suggestion {
            write!(f, " (suggestion: {suggestion})")?;
        }
        Ok(())
    }
}

/// Result of migrating one or more source configs.
struct MigrationResult {
    config: serde_json::Value,
    warnings: Vec<MigrationWarning>,
    sources: Vec<String>,
}

/// Run the migrate command.
pub fn run_migrate(root: &Path, use_toml: bool, dry_run: bool, from: Option<&Path>) -> ExitCode {
    // Check if a fallow config already exists
    let existing_names = [".fallowrc.json", "fallow.toml", ".fallow.toml"];
    if !dry_run {
        for name in &existing_names {
            let path = root.join(name);
            if path.exists() {
                eprintln!(
                    "Error: {name} already exists. Remove it first or use --dry-run to preview."
                );
                return ExitCode::from(2);
            }
        }
    }

    let result = from.map_or_else(|| migrate_auto_detect(root), migrate_from_file);

    let result = match result {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::from(2);
        }
    };

    if result.sources.is_empty() {
        eprintln!("No knip or jscpd configuration found to migrate.");
        return ExitCode::from(2);
    }

    // Generate output
    let output_content = if use_toml {
        generate_toml(&result)
    } else {
        generate_jsonc(&result)
    };

    if dry_run {
        println!("{output_content}");
    } else {
        let filename = if use_toml {
            "fallow.toml"
        } else {
            ".fallowrc.json"
        };
        let output_path = root.join(filename);
        if let Err(e) = std::fs::write(&output_path, &output_content) {
            eprintln!("Error: failed to write {filename}: {e}");
            return ExitCode::from(2);
        }
        eprintln!("Created {filename}");
    }

    // Print source info
    for source in &result.sources {
        eprintln!("Migrated from: {source}");
    }

    // Print warnings
    if !result.warnings.is_empty() {
        eprintln!();
        eprintln!("Warnings ({} skipped fields):", result.warnings.len());
        for warning in &result.warnings {
            eprintln!("  {warning}");
        }
    }

    ExitCode::SUCCESS
}

/// Auto-detect and migrate from knip and/or jscpd configs in the given root.
#[allow(clippy::case_sensitive_file_extension_comparisons)] // JS/TS extensions are always lowercase
fn migrate_auto_detect(root: &Path) -> Result<MigrationResult, String> {
    let mut config = serde_json::Map::new();
    let mut warnings = Vec::new();
    let mut sources = Vec::new();

    // Try knip configs
    let knip_files = [
        "knip.json",
        "knip.jsonc",
        ".knip.json",
        ".knip.jsonc",
        "knip.ts",
        "knip.config.ts",
    ];

    for name in &knip_files {
        let path = root.join(name);
        if path.exists() {
            if name.ends_with(".ts") {
                warnings.push(MigrationWarning {
                    source: "knip",
                    field: name.to_string(),
                    message: format!(
                        "TypeScript config files ({name}) cannot be parsed. \
                         Convert to knip.json first, then re-run migrate."
                    ),
                    suggestion: None,
                });
                continue;
            }
            let knip_value = load_json_or_jsonc(&path)?;
            migrate_knip(&knip_value, &mut config, &mut warnings);
            sources.push(name.to_string());
            break; // Only use the first knip config found
        }
    }

    // Try jscpd standalone config
    let mut found_jscpd_file = false;
    let jscpd_path = root.join(".jscpd.json");
    if jscpd_path.exists() {
        let jscpd_value = load_json_or_jsonc(&jscpd_path)?;
        migrate_jscpd(&jscpd_value, &mut config, &mut warnings);
        sources.push(".jscpd.json".to_string());
        found_jscpd_file = true;
    }

    // Check package.json for embedded knip/jscpd config (single read)
    let need_pkg_knip = sources.is_empty();
    let need_pkg_jscpd = !found_jscpd_file;
    if need_pkg_knip || need_pkg_jscpd {
        let pkg_path = root.join("package.json");
        if pkg_path.exists() {
            let pkg_content = std::fs::read_to_string(&pkg_path)
                .map_err(|e| format!("failed to read package.json: {e}"))?;
            let pkg_value: serde_json::Value = serde_json::from_str(&pkg_content)
                .map_err(|e| format!("failed to parse package.json: {e}"))?;
            if need_pkg_knip && let Some(knip_config) = pkg_value.get("knip") {
                migrate_knip(knip_config, &mut config, &mut warnings);
                sources.push("package.json (knip key)".to_string());
            }
            if need_pkg_jscpd && let Some(jscpd_config) = pkg_value.get("jscpd") {
                migrate_jscpd(jscpd_config, &mut config, &mut warnings);
                sources.push("package.json (jscpd key)".to_string());
            }
        }
    }

    Ok(MigrationResult {
        config: serde_json::Value::Object(config),
        warnings,
        sources,
    })
}

/// Migrate from a specific config file.
#[allow(clippy::case_sensitive_file_extension_comparisons)] // JS/TS extensions are always lowercase
fn migrate_from_file(path: &Path) -> Result<MigrationResult, String> {
    if !path.exists() {
        return Err(format!("config file not found: {}", path.display()));
    }

    let mut config = serde_json::Map::new();
    let mut warnings = Vec::new();
    let mut sources = Vec::new();

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();

    if filename.contains("knip") {
        if filename.ends_with(".ts") {
            return Err(format!(
                "TypeScript config files ({filename}) cannot be parsed. \
                 Convert to knip.json first, then re-run migrate."
            ));
        }
        let knip_value = load_json_or_jsonc(path)?;
        migrate_knip(&knip_value, &mut config, &mut warnings);
        sources.push(path.display().to_string());
    } else if filename.contains("jscpd") {
        let jscpd_value = load_json_or_jsonc(path)?;
        migrate_jscpd(&jscpd_value, &mut config, &mut warnings);
        sources.push(path.display().to_string());
    } else if filename == "package.json" {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
        let pkg_value: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("failed to parse {}: {e}", path.display()))?;
        if let Some(knip_config) = pkg_value.get("knip") {
            migrate_knip(knip_config, &mut config, &mut warnings);
            sources.push(format!("{} (knip key)", path.display()));
        }
        if let Some(jscpd_config) = pkg_value.get("jscpd") {
            migrate_jscpd(jscpd_config, &mut config, &mut warnings);
            sources.push(format!("{} (jscpd key)", path.display()));
        }
        if sources.is_empty() {
            return Err(format!(
                "no knip or jscpd configuration found in {}",
                path.display()
            ));
        }
    } else {
        // Try to detect format from content
        let value = load_json_or_jsonc(path)?;
        // If it has knip-like fields, treat as knip
        if value.get("entry").is_some()
            || value.get("ignore").is_some()
            || value.get("rules").is_some()
            || value.get("project").is_some()
            || value.get("ignoreDependencies").is_some()
        {
            migrate_knip(&value, &mut config, &mut warnings);
            sources.push(path.display().to_string());
        }
        // If it has jscpd-like fields, treat as jscpd
        else if value.get("minTokens").is_some()
            || value.get("minLines").is_some()
            || value.get("threshold").is_some()
            || value.get("mode").is_some()
        {
            migrate_jscpd(&value, &mut config, &mut warnings);
            sources.push(path.display().to_string());
        } else {
            return Err(format!(
                "could not determine config format for {}",
                path.display()
            ));
        }
    }

    Ok(MigrationResult {
        config: serde_json::Value::Object(config),
        warnings,
        sources,
    })
}

/// Load a JSON or JSONC file, stripping comments if present.
fn load_json_or_jsonc(path: &Path) -> Result<serde_json::Value, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;

    // Try plain JSON first
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
        return Ok(value);
    }

    // Try stripping comments (JSONC)
    let mut stripped = String::new();
    json_comments::StripComments::new(content.as_bytes())
        .read_to_string(&mut stripped)
        .map_err(|e| format!("failed to strip comments from {}: {e}", path.display()))?;

    serde_json::from_str(&stripped).map_err(|e| format!("failed to parse {}: {e}", path.display()))
}

/// Extract a string-or-array field as a Vec<String>.
fn string_or_array(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(s) => vec![s.clone()],
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => vec![],
    }
}

// -- Output generation -------------------------------------------------------

fn generate_jsonc(result: &MigrationResult) -> String {
    let mut output = String::new();
    output.push_str("{\n");
    output.push_str(
        "  \"$schema\": \"https://raw.githubusercontent.com/fallow-rs/fallow/main/schema.json\",\n",
    );

    let obj = result
        .config
        .as_object()
        .expect("config is always an Object");
    let source_comment = result.sources.join(", ");
    let _ = writeln!(output, "  // Migrated from {source_comment}");

    let mut entries: Vec<(&String, &serde_json::Value)> = obj.iter().collect();
    // Sort keys for consistent output
    let key_order = [
        "entry",
        "ignorePatterns",
        "ignoreDependencies",
        "rules",
        "duplicates",
    ];
    entries.sort_by_key(|(k, _)| {
        key_order
            .iter()
            .position(|o| *o == k.as_str())
            .unwrap_or(usize::MAX)
    });

    let total = entries.len();
    for (i, (key, value)) in entries.iter().enumerate() {
        let is_last = i == total - 1;
        let serialized = serde_json::to_string_pretty(value).unwrap_or_default();
        // Indent the serialized value by 2 spaces (but the first line is on the key line)
        let indented = indent_json_value(&serialized, 2);
        if is_last {
            let _ = writeln!(output, "  \"{key}\": {indented}");
        } else {
            let _ = writeln!(output, "  \"{key}\": {indented},");
        }
    }

    output.push_str("}\n");
    output
}

/// Indent a pretty-printed JSON value's continuation lines.
fn indent_json_value(json: &str, spaces: usize) -> String {
    let indent = " ".repeat(spaces);
    let mut lines: Vec<&str> = json.lines().collect();
    if lines.len() <= 1 {
        return json.to_string();
    }
    // First line stays as-is, subsequent lines get indented
    let first = lines.remove(0);
    let rest: Vec<String> = lines.iter().map(|l| format!("{indent}{l}")).collect();
    let mut result = first.to_string();
    for line in rest {
        result.push('\n');
        result.push_str(&line);
    }
    result
}

fn generate_toml(result: &MigrationResult) -> String {
    let mut output = String::new();
    let source_comment = result.sources.join(", ");
    let _ = writeln!(output, "# Migrated from {source_comment}\n");

    let obj = result
        .config
        .as_object()
        .expect("config is always an Object");

    // Top-level simple fields first
    // Note: fallow config uses #[serde(rename_all = "camelCase")] so TOML keys must be camelCase
    for key in &["entry", "ignorePatterns", "ignoreDependencies"] {
        if let Some(value) = obj.get(*key)
            && let Some(arr) = value.as_array()
        {
            let items: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| format!("\"{s}\"")))
                .collect();
            let _ = writeln!(output, "{key} = [{}]", items.join(", "));
        }
    }

    // [rules] table
    if let Some(rules) = obj.get("rules")
        && let Some(rules_obj) = rules.as_object()
        && !rules_obj.is_empty()
    {
        output.push_str("\n[rules]\n");
        for (key, value) in rules_obj {
            if let Some(s) = value.as_str() {
                let _ = writeln!(output, "{key} = \"{s}\"");
            }
        }
    }

    // [duplicates] table
    if let Some(dupes) = obj.get("duplicates")
        && let Some(dupes_obj) = dupes.as_object()
        && !dupes_obj.is_empty()
    {
        output.push_str("\n[duplicates]\n");
        for (key, value) in dupes_obj {
            match value {
                serde_json::Value::Number(n) => {
                    let _ = writeln!(output, "{key} = {n}");
                }
                serde_json::Value::Bool(b) => {
                    let _ = writeln!(output, "{key} = {b}");
                }
                serde_json::Value::String(s) => {
                    let _ = writeln!(output, "{key} = \"{s}\"");
                }
                serde_json::Value::Array(arr) => {
                    let items: Vec<String> = arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| format!("\"{s}\"")))
                        .collect();
                    let _ = writeln!(output, "{key} = [{}]", items.join(", "));
                }
                _ => {}
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use std::io::Read as _;

    use super::*;

    fn empty_config() -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::new()
    }

    // -- Combined migration tests --------------------------------------------

    #[test]
    fn migrate_both_knip_and_jscpd() {
        let knip: serde_json::Value =
            serde_json::from_str(r#"{"entry": ["src/index.ts"], "ignore": ["dist/**"]}"#).unwrap();
        let jscpd: serde_json::Value =
            serde_json::from_str(r#"{"minTokens": 100, "skipLocal": true}"#).unwrap();
        let mut config_map = empty_config();
        let mut warnings = Vec::new();
        migrate_knip(&knip, &mut config_map, &mut warnings);
        migrate_jscpd(&jscpd, &mut config_map, &mut warnings);

        assert!(config_map.contains_key("entry"));
        assert!(config_map.contains_key("ignorePatterns"));
        assert!(config_map.contains_key("duplicates"));
    }

    // -- Output format tests -------------------------------------------------

    #[test]
    fn jsonc_output_has_schema() {
        let result = MigrationResult {
            config: serde_json::json!({"entry": ["src/index.ts"]}),
            warnings: vec![],
            sources: vec!["knip.json".to_string()],
        };
        let output = generate_jsonc(&result);
        assert!(output.contains("$schema"));
        assert!(output.contains("fallow-rs/fallow"));
    }

    #[test]
    fn jsonc_output_has_source_comment() {
        let result = MigrationResult {
            config: serde_json::json!({"entry": ["src/index.ts"]}),
            warnings: vec![],
            sources: vec!["knip.json".to_string()],
        };
        let output = generate_jsonc(&result);
        assert!(output.contains("// Migrated from knip.json"));
    }

    #[test]
    fn toml_output_has_source_comment() {
        let result = MigrationResult {
            config: serde_json::json!({"entry": ["src/index.ts"]}),
            warnings: vec![],
            sources: vec!["knip.json".to_string()],
        };
        let output = generate_toml(&result);
        assert!(output.contains("# Migrated from knip.json"));
    }

    #[test]
    fn toml_output_rules_section() {
        let result = MigrationResult {
            config: serde_json::json!({
                "rules": {
                    "unused-files": "error",
                    "unused-exports": "warn"
                }
            }),
            warnings: vec![],
            sources: vec!["knip.json".to_string()],
        };
        let output = generate_toml(&result);
        assert!(output.contains("[rules]"));
        assert!(output.contains("unused-files = \"error\""));
        assert!(output.contains("unused-exports = \"warn\""));
    }

    #[test]
    fn toml_output_duplicates_section() {
        let result = MigrationResult {
            config: serde_json::json!({
                "duplicates": {
                    "minTokens": 100,
                    "skipLocal": true
                }
            }),
            warnings: vec![],
            sources: vec![".jscpd.json".to_string()],
        };
        let output = generate_toml(&result);
        assert!(output.contains("[duplicates]"));
        assert!(output.contains("minTokens = 100"));
        assert!(output.contains("skipLocal = true"));
    }

    // -- Deserialization roundtrip tests --------------------------------------

    #[test]
    fn toml_output_deserializes_as_valid_config() {
        let result = MigrationResult {
            config: serde_json::json!({
                "entry": ["src/index.ts"],
                "ignorePatterns": ["dist/**"],
                "ignoreDependencies": ["lodash"],
                "rules": {
                    "unused-files": "error",
                    "unused-exports": "warn"
                },
                "duplicates": {
                    "minTokens": 100,
                    "skipLocal": true
                }
            }),
            warnings: vec![],
            sources: vec!["knip.json".to_string()],
        };
        let output = generate_toml(&result);
        let config: fallow_config::FallowConfig = toml::from_str(&output).unwrap();
        assert_eq!(config.entry, vec!["src/index.ts"]);
        assert_eq!(config.ignore_patterns, vec!["dist/**"]);
        assert_eq!(config.ignore_dependencies, vec!["lodash"]);
    }

    #[test]
    fn jsonc_output_deserializes_as_valid_config() {
        let result = MigrationResult {
            config: serde_json::json!({
                "entry": ["src/index.ts"],
                "ignoreDependencies": ["lodash"],
                "rules": {
                    "unused-files": "warn"
                }
            }),
            warnings: vec![],
            sources: vec!["knip.json".to_string()],
        };
        let output = generate_jsonc(&result);
        let mut stripped = String::new();
        json_comments::StripComments::new(output.as_bytes())
            .read_to_string(&mut stripped)
            .unwrap();
        let config: fallow_config::FallowConfig = serde_json::from_str(&stripped).unwrap();
        assert_eq!(config.entry, vec!["src/index.ts"]);
        assert_eq!(config.ignore_dependencies, vec!["lodash"]);
    }

    // -- JSONC comment stripping test ----------------------------------------

    #[test]
    fn jsonc_comments_stripped() {
        let tmpdir = std::env::temp_dir().join("fallow-test-migrate-jsonc");
        let _ = std::fs::create_dir_all(&tmpdir);
        let path = tmpdir.join("knip.jsonc");
        std::fs::write(
            &path,
            r#"{
                // Entry points
                "entry": ["src/index.ts"],
                /* Block comment */
                "ignore": ["dist/**"]
            }"#,
        )
        .unwrap();

        let value = load_json_or_jsonc(&path).unwrap();
        assert_eq!(value["entry"], serde_json::json!(["src/index.ts"]));
        assert_eq!(value["ignore"], serde_json::json!(["dist/**"]));

        let _ = std::fs::remove_dir_all(&tmpdir);
    }

    // -- Package.json embedded config detection ------------------------------

    #[test]
    fn auto_detect_package_json_knip() {
        let tmpdir = std::env::temp_dir().join("fallow-test-migrate-pkg-knip");
        let _ = std::fs::create_dir_all(&tmpdir);
        let pkg_path = tmpdir.join("package.json");
        std::fs::write(
            &pkg_path,
            r#"{"name": "test", "knip": {"entry": ["src/main.ts"]}}"#,
        )
        .unwrap();

        let result = migrate_auto_detect(&tmpdir).unwrap();
        assert!(!result.sources.is_empty());
        assert!(result.sources[0].contains("package.json"));

        let config_obj = result.config.as_object().unwrap();
        assert_eq!(
            config_obj.get("entry").unwrap(),
            &serde_json::json!(["src/main.ts"])
        );

        let _ = std::fs::remove_dir_all(&tmpdir);
    }

    #[test]
    fn auto_detect_package_json_jscpd() {
        let tmpdir = std::env::temp_dir().join("fallow-test-migrate-pkg-jscpd");
        let _ = std::fs::create_dir_all(&tmpdir);
        let pkg_path = tmpdir.join("package.json");
        std::fs::write(&pkg_path, r#"{"name": "test", "jscpd": {"minTokens": 75}}"#).unwrap();

        let result = migrate_auto_detect(&tmpdir).unwrap();
        assert!(!result.sources.is_empty());
        assert!(result.sources[0].contains("package.json"));

        let config_obj = result.config.as_object().unwrap();
        let dupes = config_obj.get("duplicates").unwrap().as_object().unwrap();
        assert_eq!(dupes.get("minTokens").unwrap(), 75);

        let _ = std::fs::remove_dir_all(&tmpdir);
    }
}
