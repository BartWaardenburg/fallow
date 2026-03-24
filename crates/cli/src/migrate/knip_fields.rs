use serde_json::{Map, Value};

use super::knip_tables::{
    KNIP_PLUGIN_KEYS, KNIP_RULE_MAP, KNIP_UNMAPPABLE_FIELDS, KNIP_UNMAPPABLE_ISSUE_TYPES,
};
use super::{MigrationWarning, string_or_array};

type JsonMap = Map<String, Value>;

/// Migrate a string-or-array field from knip to a fallow config field.
pub(super) fn migrate_simple_field(
    obj: &JsonMap,
    src_key: &str,
    dst_key: &str,
    config: &mut JsonMap,
) {
    if let Some(val) = obj.get(src_key) {
        let entries = string_or_array(val);
        if !entries.is_empty() {
            config.insert(
                dst_key.to_string(),
                Value::Array(entries.into_iter().map(Value::String).collect()),
            );
        }
    }
}

/// Migrate knip `rules` to fallow `rules`, warning about unmappable rule names.
pub(super) fn migrate_rules(
    rules_val: &Value,
    config: &mut JsonMap,
    warnings: &mut Vec<MigrationWarning>,
) {
    let Some(rules_obj) = rules_val.as_object() else {
        return;
    };

    let mut fallow_rules = Map::new();
    for (knip_name, fallow_name) in KNIP_RULE_MAP {
        if let Some(severity_val) = rules_obj.get(*knip_name)
            && let Some(severity_str) = severity_val.as_str()
        {
            fallow_rules.insert(
                (*fallow_name).to_string(),
                Value::String(severity_str.to_string()),
            );
        }
    }

    // Warn about unmappable rule names
    for (key, _) in rules_obj {
        let is_mapped = KNIP_RULE_MAP.iter().any(|(k, _)| k == key);
        let is_unmappable = KNIP_UNMAPPABLE_ISSUE_TYPES.contains(&key.as_str());
        if !is_mapped && is_unmappable {
            warnings.push(MigrationWarning {
                source: "knip",
                field: format!("rules.{key}"),
                message: format!("issue type `{key}` has no fallow equivalent"),
                suggestion: None,
            });
        }
    }

    if !fallow_rules.is_empty() {
        config.insert("rules".to_string(), Value::Object(fallow_rules));
    }
}

/// Migrate knip `exclude` — set excluded issue types to `"off"` in fallow rules.
pub(super) fn migrate_exclude(
    excluded: &[String],
    config: &mut JsonMap,
    warnings: &mut Vec<MigrationWarning>,
) {
    let rules = config
        .entry("rules".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(rules_obj) = rules.as_object_mut() else {
        return;
    };

    for knip_name in excluded {
        if let Some((_, fallow_name)) = KNIP_RULE_MAP.iter().find(|(k, _)| k == knip_name) {
            rules_obj.insert((*fallow_name).to_string(), Value::String("off".to_string()));
        } else if KNIP_UNMAPPABLE_ISSUE_TYPES.contains(&knip_name.as_str()) {
            warnings.push(MigrationWarning {
                source: "knip",
                field: format!("exclude.{knip_name}"),
                message: format!("issue type `{knip_name}` has no fallow equivalent"),
                suggestion: None,
            });
        }
    }
}

/// Migrate knip `include` — set non-included issue types to `"off"` in fallow rules.
pub(super) fn migrate_include(
    included: &[String],
    config: &mut JsonMap,
    warnings: &mut Vec<MigrationWarning>,
) {
    let rules = config
        .entry("rules".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(rules_obj) = rules.as_object_mut() else {
        return;
    };

    for (knip_name, fallow_name) in KNIP_RULE_MAP {
        if !included.iter().any(|i| i == knip_name) {
            // Not included -- set to off (unless already set by rules)
            rules_obj
                .entry((*fallow_name).to_string())
                .or_insert_with(|| Value::String("off".to_string()));
        }
    }
    // Warn about unmappable included types
    for name in included {
        let is_mapped = KNIP_RULE_MAP.iter().any(|(k, _)| k == name);
        if !is_mapped && KNIP_UNMAPPABLE_ISSUE_TYPES.contains(&name.as_str()) {
            warnings.push(MigrationWarning {
                source: "knip",
                field: format!("include.{name}"),
                message: format!("issue type `{name}` has no fallow equivalent"),
                suggestion: None,
            });
        }
    }
}

/// Migrate knip `ignoreDependencies` — filter out regex patterns with warnings.
pub(super) fn migrate_ignore_deps(
    ignore_deps_val: &Value,
    config: &mut JsonMap,
    warnings: &mut Vec<MigrationWarning>,
) {
    let deps = string_or_array(ignore_deps_val);
    let non_regex: Vec<String> = deps
        .into_iter()
        .filter(|d| {
            // Skip values that look like regex patterns
            if d.starts_with('/') && d.ends_with('/') {
                warnings.push(MigrationWarning {
                    source: "knip",
                    field: "ignoreDependencies".to_string(),
                    message: format!("regex pattern `{d}` skipped (fallow uses exact strings)"),
                    suggestion: Some("add each dependency name explicitly".to_string()),
                });
                false
            } else {
                true
            }
        })
        .collect();
    if !non_regex.is_empty() {
        config.insert(
            "ignoreDependencies".to_string(),
            Value::Array(non_regex.into_iter().map(Value::String).collect()),
        );
    }
}

/// Warn about knip fields that have no fallow equivalent.
pub(super) fn warn_unmappable_fields(obj: &JsonMap, warnings: &mut Vec<MigrationWarning>) {
    for (field, message, suggestion) in KNIP_UNMAPPABLE_FIELDS {
        if obj.contains_key(*field) {
            warnings.push(MigrationWarning {
                source: "knip",
                field: (*field).to_string(),
                message: (*message).to_string(),
                suggestion: suggestion.map(|s| s.to_string()),
            });
        }
    }
}

/// Warn about knip plugin-specific config keys that are auto-detected in fallow.
pub(super) fn warn_plugin_keys(obj: &JsonMap, warnings: &mut Vec<MigrationWarning>) {
    for key in obj.keys() {
        if KNIP_PLUGIN_KEYS.contains(&key.as_str()) {
            warnings.push(MigrationWarning {
                source: "knip",
                field: key.clone(),
                message: format!(
                    "plugin config `{key}` is auto-detected by fallow's built-in plugins"
                ),
                suggestion: Some(
                    "remove this section; fallow detects framework config automatically"
                        .to_string(),
                ),
            });
        }
    }
}
