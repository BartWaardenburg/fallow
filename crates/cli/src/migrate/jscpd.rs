use super::{MigrationWarning, string_or_array};

/// jscpd fields that cannot be mapped and generate warnings.
const JSCPD_UNMAPPABLE_FIELDS: &[(&str, &str, Option<&str>)] = &[
    ("maxLines", "No maximum line count limit in fallow", None),
    ("maxSize", "No maximum file size limit in fallow", None),
    (
        "ignorePattern",
        "Content-based ignore patterns are not supported",
        Some("use inline suppression: // fallow-ignore-next-line code-duplication"),
    ),
    (
        "reporters",
        "Reporters are not configurable in fallow",
        Some("use --format flag instead (human/json/sarif/compact)"),
    ),
    (
        "output",
        "fallow writes to stdout",
        Some("redirect output with shell: fallow dupes > report.json"),
    ),
    (
        "blame",
        "Git blame integration is not supported in fallow",
        None,
    ),
    ("absolute", "fallow always shows relative paths", None),
    (
        "noSymlinks",
        "Symlink handling is not configurable in fallow",
        None,
    ),
    (
        "ignoreCase",
        "Case-insensitive matching is not supported in fallow",
        None,
    ),
    ("format", "fallow auto-detects JS/TS files", None),
    (
        "formatsExts",
        "Custom file extensions are not configurable in fallow",
        None,
    ),
    ("store", "Store backend is not configurable in fallow", None),
    (
        "tokensToSkip",
        "Token skipping is not configurable in fallow",
        None,
    ),
    (
        "exitCode",
        "Exit codes are not configurable in fallow",
        Some("use the rules system to control which issues cause CI failure"),
    ),
    (
        "pattern",
        "Pattern filtering is not supported in fallow",
        None,
    ),
    (
        "path",
        "Source path configuration is not supported",
        Some("run fallow from the project root directory"),
    ),
];

pub(super) fn migrate_jscpd(
    jscpd: &serde_json::Value,
    config: &mut serde_json::Map<String, serde_json::Value>,
    warnings: &mut Vec<MigrationWarning>,
) {
    let obj = match jscpd.as_object() {
        Some(o) => o,
        None => {
            warnings.push(MigrationWarning {
                source: "jscpd",
                field: "(root)".to_string(),
                message: "expected an object, got something else".to_string(),
                suggestion: None,
            });
            return;
        }
    };

    let mut dupes = serde_json::Map::new();

    // minTokens -> duplicates.minTokens
    if let Some(min_tokens) = obj.get("minTokens").and_then(|v| v.as_u64()) {
        dupes.insert(
            "minTokens".to_string(),
            serde_json::Value::Number(min_tokens.into()),
        );
    }

    // minLines -> duplicates.minLines
    if let Some(min_lines) = obj.get("minLines").and_then(|v| v.as_u64()) {
        dupes.insert(
            "minLines".to_string(),
            serde_json::Value::Number(min_lines.into()),
        );
    }

    // threshold -> duplicates.threshold
    if let Some(threshold) = obj.get("threshold").and_then(|v| v.as_f64())
        && let Some(n) = serde_json::Number::from_f64(threshold)
    {
        dupes.insert("threshold".to_string(), serde_json::Value::Number(n));
    }

    // mode -> duplicates.mode
    if let Some(mode_str) = obj.get("mode").and_then(|v| v.as_str()) {
        let fallow_mode = match mode_str {
            "strict" => Some("strict"),
            "mild" => Some("mild"),
            "weak" => {
                warnings.push(MigrationWarning {
                    source: "jscpd",
                    field: "mode".to_string(),
                    message: "jscpd's \"weak\" mode may differ semantically from fallow's \"weak\" \
                              mode. jscpd uses lexer-based tokens while fallow uses AST-based tokens."
                        .to_string(),
                    suggestion: Some(
                        "test with both \"weak\" and \"mild\" to find the best match".to_string(),
                    ),
                });
                Some("weak")
            }
            other => {
                warnings.push(MigrationWarning {
                    source: "jscpd",
                    field: "mode".to_string(),
                    message: format!("unknown mode `{other}`, defaulting to \"mild\""),
                    suggestion: None,
                });
                None
            }
        };
        if let Some(mode) = fallow_mode {
            dupes.insert(
                "mode".to_string(),
                serde_json::Value::String(mode.to_string()),
            );
        }
    }

    // skipLocal -> duplicates.skipLocal
    if let Some(skip_local) = obj.get("skipLocal").and_then(|v| v.as_bool()) {
        dupes.insert("skipLocal".to_string(), serde_json::Value::Bool(skip_local));
    }

    // ignore -> duplicates.ignore (glob patterns)
    if let Some(ignore_val) = obj.get("ignore") {
        let ignores = string_or_array(ignore_val);
        if !ignores.is_empty() {
            dupes.insert(
                "ignore".to_string(),
                serde_json::Value::Array(
                    ignores.into_iter().map(serde_json::Value::String).collect(),
                ),
            );
        }
    }

    if !dupes.is_empty() {
        config.insert("duplicates".to_string(), serde_json::Value::Object(dupes));
    }

    // Warn about unmappable fields
    for (field, message, suggestion) in JSCPD_UNMAPPABLE_FIELDS {
        if obj.contains_key(*field) {
            warnings.push(MigrationWarning {
                source: "jscpd",
                field: (*field).to_string(),
                message: (*message).to_string(),
                suggestion: suggestion.map(|s| s.to_string()),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_config() -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::new()
    }

    #[test]
    fn migrate_jscpd_basic() {
        let jscpd: serde_json::Value =
            serde_json::from_str(r#"{"minTokens": 100, "minLines": 10, "threshold": 5.0}"#)
                .unwrap();
        let mut config = empty_config();
        let mut warnings = Vec::new();
        migrate_jscpd(&jscpd, &mut config, &mut warnings);

        let dupes = config.get("duplicates").unwrap().as_object().unwrap();
        assert_eq!(dupes.get("minTokens").unwrap(), 100);
        assert_eq!(dupes.get("minLines").unwrap(), 10);
        assert_eq!(dupes.get("threshold").unwrap(), 5.0);
        assert!(warnings.is_empty());
    }

    #[test]
    fn migrate_jscpd_mode_weak_warns() {
        let jscpd: serde_json::Value = serde_json::from_str(r#"{"mode": "weak"}"#).unwrap();
        let mut config = empty_config();
        let mut warnings = Vec::new();
        migrate_jscpd(&jscpd, &mut config, &mut warnings);

        let dupes = config.get("duplicates").unwrap().as_object().unwrap();
        assert_eq!(dupes.get("mode").unwrap(), "weak");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("differ semantically"));
    }

    #[test]
    fn migrate_jscpd_skip_local() {
        let jscpd: serde_json::Value = serde_json::from_str(r#"{"skipLocal": true}"#).unwrap();
        let mut config = empty_config();
        let mut warnings = Vec::new();
        migrate_jscpd(&jscpd, &mut config, &mut warnings);

        let dupes = config.get("duplicates").unwrap().as_object().unwrap();
        assert_eq!(dupes.get("skipLocal").unwrap(), true);
    }

    #[test]
    fn migrate_jscpd_ignore_patterns() {
        let jscpd: serde_json::Value =
            serde_json::from_str(r#"{"ignore": ["**/*.test.ts", "dist/**"]}"#).unwrap();
        let mut config = empty_config();
        let mut warnings = Vec::new();
        migrate_jscpd(&jscpd, &mut config, &mut warnings);

        let dupes = config.get("duplicates").unwrap().as_object().unwrap();
        assert_eq!(
            dupes.get("ignore").unwrap(),
            &serde_json::json!(["**/*.test.ts", "dist/**"])
        );
    }

    #[test]
    fn migrate_jscpd_unmappable_fields_generate_warnings() {
        let jscpd: serde_json::Value = serde_json::from_str(
            r#"{"minTokens": 50, "maxLines": 1000, "reporters": ["console"], "blame": true}"#,
        )
        .unwrap();
        let mut config = empty_config();
        let mut warnings = Vec::new();
        migrate_jscpd(&jscpd, &mut config, &mut warnings);

        assert_eq!(warnings.len(), 3);
        let fields: Vec<&str> = warnings.iter().map(|w| w.field.as_str()).collect();
        assert!(fields.contains(&"maxLines"));
        assert!(fields.contains(&"reporters"));
        assert!(fields.contains(&"blame"));
    }
}
