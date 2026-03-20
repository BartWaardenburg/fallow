use super::{MigrationWarning, string_or_array};

/// Knip rule names mapped to fallow rule names.
const KNIP_RULE_MAP: &[(&str, &str)] = &[
    ("files", "unused-files"),
    ("dependencies", "unused-dependencies"),
    ("devDependencies", "unused-dev-dependencies"),
    ("exports", "unused-exports"),
    ("types", "unused-types"),
    ("enumMembers", "unused-enum-members"),
    ("classMembers", "unused-class-members"),
    ("unlisted", "unlisted-dependencies"),
    ("unresolved", "unresolved-imports"),
    ("duplicates", "duplicate-exports"),
];

/// Knip fields that cannot be mapped and generate warnings.
const KNIP_UNMAPPABLE_FIELDS: &[(&str, &str, Option<&str>)] = &[
    ("project", "Fallow auto-discovers project files", None),
    (
        "paths",
        "Fallow reads path mappings from tsconfig.json automatically",
        None,
    ),
    (
        "ignoreFiles",
        "No separate concept in fallow",
        Some("use `ignore` patterns instead"),
    ),
    (
        "ignoreBinaries",
        "Binary filtering is not configurable in fallow",
        None,
    ),
    (
        "ignoreMembers",
        "Member-level ignoring is not configurable in fallow",
        Some("use inline suppression comments: // fallow-ignore-next-line"),
    ),
    (
        "ignoreUnresolved",
        "Unresolved import filtering is not configurable in fallow",
        Some("use inline suppression comments: // fallow-ignore-next-line unresolved-import"),
    ),
    ("ignoreExportsUsedInFile", "No equivalent in fallow", None),
    (
        "ignoreWorkspaces",
        "Workspace filtering is not configurable per-workspace",
        Some("use --workspace flag to scope output to a single package"),
    ),
    (
        "ignoreIssues",
        "No global issue ignoring in fallow",
        Some("use inline suppression comments: // fallow-ignore-file [issue-type]"),
    ),
    (
        "includeEntryExports",
        "Entry export inclusion is not configurable in fallow",
        None,
    ),
    (
        "tags",
        "Tag-based filtering is not supported in fallow",
        None,
    ),
    (
        "compilers",
        "Custom compilers are not supported in fallow (uses Oxc parser)",
        None,
    ),
    ("treatConfigHintsAsErrors", "No equivalent in fallow", None),
];

/// Knip issue type names that have no fallow equivalent.
const KNIP_UNMAPPABLE_ISSUE_TYPES: &[&str] = &[
    "optionalPeerDependencies",
    "binaries",
    "nsExports",
    "nsTypes",
    "catalog",
];

/// Known knip plugin config keys (framework-specific). These are auto-detected by fallow plugins.
const KNIP_PLUGIN_KEYS: &[&str] = &[
    "angular",
    "astro",
    "ava",
    "babel",
    "biome",
    "capacitor",
    "changesets",
    "commitizen",
    "commitlint",
    "cspell",
    "cucumber",
    "cypress",
    "docusaurus",
    "drizzle",
    "eleventy",
    "eslint",
    "expo",
    "gatsby",
    "github-actions",
    "graphql-codegen",
    "husky",
    "jest",
    "knex",
    "lefthook",
    "lint-staged",
    "markdownlint",
    "mocha",
    "moonrepo",
    "msw",
    "nest",
    "next",
    "node-test-runner",
    "npm-package-json-lint",
    "nuxt",
    "nx",
    "nyc",
    "oclif",
    "playwright",
    "postcss",
    "prettier",
    "prisma",
    "react-cosmos",
    "react-router",
    "release-it",
    "remark",
    "remix",
    "rollup",
    "rspack",
    "semantic-release",
    "sentry",
    "simple-git-hooks",
    "size-limit",
    "storybook",
    "stryker",
    "stylelint",
    "svelte",
    "syncpack",
    "tailwind",
    "tsup",
    "tsx",
    "typedoc",
    "typescript",
    "unbuild",
    "unocss",
    "vercel-og",
    "vite",
    "vitest",
    "vue",
    "webpack",
    "wireit",
    "wrangler",
    "xo",
    "yorkie",
];

pub(super) fn migrate_knip(
    knip: &serde_json::Value,
    config: &mut serde_json::Map<String, serde_json::Value>,
    warnings: &mut Vec<MigrationWarning>,
) {
    let Some(obj) = knip.as_object() else {
        warnings.push(MigrationWarning {
            source: "knip",
            field: "(root)".to_string(),
            message: "expected an object, got something else".to_string(),
            suggestion: None,
        });
        return;
    };

    // entry -> entry
    if let Some(entry_val) = obj.get("entry") {
        let entries = string_or_array(entry_val);
        if !entries.is_empty() {
            config.insert(
                "entry".to_string(),
                serde_json::Value::Array(
                    entries.into_iter().map(serde_json::Value::String).collect(),
                ),
            );
        }
    }

    // ignore -> ignorePatterns
    if let Some(ignore_val) = obj.get("ignore") {
        let ignores = string_or_array(ignore_val);
        if !ignores.is_empty() {
            config.insert(
                "ignorePatterns".to_string(),
                serde_json::Value::Array(
                    ignores.into_iter().map(serde_json::Value::String).collect(),
                ),
            );
        }
    }

    // ignoreDependencies -> ignoreDependencies (skip regex values)
    if let Some(ignore_deps_val) = obj.get("ignoreDependencies") {
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
                serde_json::Value::Array(
                    non_regex
                        .into_iter()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }
    }

    // rules -> rules mapping
    if let Some(rules_val) = obj.get("rules")
        && let Some(rules_obj) = rules_val.as_object()
    {
        let mut fallow_rules = serde_json::Map::new();
        for (knip_name, fallow_name) in KNIP_RULE_MAP {
            if let Some(severity_val) = rules_obj.get(*knip_name)
                && let Some(severity_str) = severity_val.as_str()
            {
                fallow_rules.insert(
                    (*fallow_name).to_string(),
                    serde_json::Value::String(severity_str.to_string()),
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
            config.insert("rules".to_string(), serde_json::Value::Object(fallow_rules));
        }
    }

    // exclude -> set those issue types to "off" in rules
    if let Some(exclude_val) = obj.get("exclude") {
        let excluded = string_or_array(exclude_val);
        if !excluded.is_empty() {
            let rules = config
                .entry("rules".to_string())
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
            if let Some(rules_obj) = rules.as_object_mut() {
                for knip_name in &excluded {
                    if let Some((_, fallow_name)) =
                        KNIP_RULE_MAP.iter().find(|(k, _)| k == knip_name)
                    {
                        rules_obj.insert(
                            (*fallow_name).to_string(),
                            serde_json::Value::String("off".to_string()),
                        );
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
        }
    }

    // include -> set non-included issue types to "off" in rules
    if let Some(include_val) = obj.get("include") {
        let included = string_or_array(include_val);
        if !included.is_empty() {
            let rules = config
                .entry("rules".to_string())
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
            if let Some(rules_obj) = rules.as_object_mut() {
                for (knip_name, fallow_name) in KNIP_RULE_MAP {
                    if !included.iter().any(|i| i == knip_name) {
                        // Not included -- set to off (unless already set by rules)
                        rules_obj
                            .entry((*fallow_name).to_string())
                            .or_insert_with(|| serde_json::Value::String("off".to_string()));
                    }
                }
                // Warn about unmappable included types
                for name in &included {
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
        }
    }

    // Warn about unmappable fields
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

    // Warn about plugin-specific config keys
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

    // Warn about workspaces with per-workspace plugin overrides
    if let Some(workspaces_val) = obj.get("workspaces")
        && workspaces_val.is_object()
    {
        warnings.push(MigrationWarning {
            source: "knip",
            field: "workspaces".to_string(),
            message: "per-workspace plugin overrides have limited support in fallow".to_string(),
            suggestion: Some(
                "fallow auto-discovers workspace packages; use --workspace flag to scope output"
                    .to_string(),
            ),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_config() -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::new()
    }

    #[test]
    fn migrate_minimal_knip_json() {
        let knip: serde_json::Value =
            serde_json::from_str(r#"{"entry": ["src/index.ts"]}"#).unwrap();
        let mut config = empty_config();
        let mut warnings = Vec::new();
        migrate_knip(&knip, &mut config, &mut warnings);

        assert_eq!(
            config.get("entry").unwrap(),
            &serde_json::json!(["src/index.ts"])
        );
        assert!(warnings.is_empty());
    }

    #[test]
    fn migrate_knip_with_rules() {
        let knip: serde_json::Value = serde_json::from_str(
            r#"{"rules": {"files": "warn", "exports": "off", "dependencies": "error"}}"#,
        )
        .unwrap();
        let mut config = empty_config();
        let mut warnings = Vec::new();
        migrate_knip(&knip, &mut config, &mut warnings);

        let rules = config.get("rules").unwrap().as_object().unwrap();
        assert_eq!(rules.get("unused-files").unwrap(), "warn");
        assert_eq!(rules.get("unused-exports").unwrap(), "off");
        assert_eq!(rules.get("unused-dependencies").unwrap(), "error");
    }

    #[test]
    fn migrate_knip_with_exclude() {
        let knip: serde_json::Value =
            serde_json::from_str(r#"{"exclude": ["files", "types"]}"#).unwrap();
        let mut config = empty_config();
        let mut warnings = Vec::new();
        migrate_knip(&knip, &mut config, &mut warnings);

        let rules = config.get("rules").unwrap().as_object().unwrap();
        assert_eq!(rules.get("unused-files").unwrap(), "off");
        assert_eq!(rules.get("unused-types").unwrap(), "off");
    }

    #[test]
    fn migrate_knip_with_include() {
        let knip: serde_json::Value =
            serde_json::from_str(r#"{"include": ["files", "exports"]}"#).unwrap();
        let mut config = empty_config();
        let mut warnings = Vec::new();
        migrate_knip(&knip, &mut config, &mut warnings);

        let rules = config.get("rules").unwrap().as_object().unwrap();
        // Included types should NOT be set to "off"
        assert!(!rules.contains_key("unused-files") || rules.get("unused-files").unwrap() != "off");
        assert!(
            !rules.contains_key("unused-exports") || rules.get("unused-exports").unwrap() != "off"
        );
        // Non-included types should be "off"
        assert_eq!(rules.get("unused-dependencies").unwrap(), "off");
        assert_eq!(rules.get("unused-types").unwrap(), "off");
        assert_eq!(rules.get("unused-enum-members").unwrap(), "off");
        assert_eq!(rules.get("unused-class-members").unwrap(), "off");
    }

    #[test]
    fn migrate_knip_with_ignore_patterns() {
        let knip: serde_json::Value =
            serde_json::from_str(r#"{"ignore": ["src/generated/**", "**/*.test.ts"]}"#).unwrap();
        let mut config = empty_config();
        let mut warnings = Vec::new();
        migrate_knip(&knip, &mut config, &mut warnings);

        assert_eq!(
            config.get("ignorePatterns").unwrap(),
            &serde_json::json!(["src/generated/**", "**/*.test.ts"])
        );
    }

    #[test]
    fn migrate_knip_with_ignore_dependencies() {
        let knip: serde_json::Value =
            serde_json::from_str(r#"{"ignoreDependencies": ["@org/lib", "lodash"]}"#).unwrap();
        let mut config = empty_config();
        let mut warnings = Vec::new();
        migrate_knip(&knip, &mut config, &mut warnings);

        assert_eq!(
            config.get("ignoreDependencies").unwrap(),
            &serde_json::json!(["@org/lib", "lodash"])
        );
    }

    #[test]
    fn migrate_knip_regex_ignore_deps_skipped() {
        let knip: serde_json::Value =
            serde_json::from_str(r#"{"ignoreDependencies": ["/^@org/", "lodash"]}"#).unwrap();
        let mut config = empty_config();
        let mut warnings = Vec::new();
        migrate_knip(&knip, &mut config, &mut warnings);

        assert_eq!(
            config.get("ignoreDependencies").unwrap(),
            &serde_json::json!(["lodash"])
        );
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].field == "ignoreDependencies");
    }

    #[test]
    fn migrate_knip_unmappable_fields_generate_warnings() {
        let knip: serde_json::Value =
            serde_json::from_str(r#"{"project": ["src/**"], "paths": {"@/*": ["src/*"]}}"#)
                .unwrap();
        let mut config = empty_config();
        let mut warnings = Vec::new();
        migrate_knip(&knip, &mut config, &mut warnings);

        assert_eq!(warnings.len(), 2);
        let fields: Vec<&str> = warnings.iter().map(|w| w.field.as_str()).collect();
        assert!(fields.contains(&"project"));
        assert!(fields.contains(&"paths"));
    }

    #[test]
    fn migrate_knip_plugin_keys_generate_warnings() {
        let knip: serde_json::Value = serde_json::from_str(
            r#"{"entry": ["src/index.ts"], "eslint": {"entry": ["eslint.config.js"]}}"#,
        )
        .unwrap();
        let mut config = empty_config();
        let mut warnings = Vec::new();
        migrate_knip(&knip, &mut config, &mut warnings);

        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].field, "eslint");
        assert!(warnings[0].message.contains("auto-detected"));
    }

    #[test]
    fn migrate_knip_entry_string() {
        let knip: serde_json::Value = serde_json::from_str(r#"{"entry": "src/index.ts"}"#).unwrap();
        let mut config = empty_config();
        let mut warnings = Vec::new();
        migrate_knip(&knip, &mut config, &mut warnings);

        assert_eq!(
            config.get("entry").unwrap(),
            &serde_json::json!(["src/index.ts"])
        );
    }

    #[test]
    fn migrate_knip_exclude_unmappable_warns() {
        let knip: serde_json::Value =
            serde_json::from_str(r#"{"exclude": ["optionalPeerDependencies"]}"#).unwrap();
        let mut config = empty_config();
        let mut warnings = Vec::new();
        migrate_knip(&knip, &mut config, &mut warnings);

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].field.contains("optionalPeerDependencies"));
    }

    #[test]
    fn migrate_knip_rules_unmappable_warns() {
        let knip: serde_json::Value =
            serde_json::from_str(r#"{"rules": {"binaries": "warn", "files": "error"}}"#).unwrap();
        let mut config = empty_config();
        let mut warnings = Vec::new();
        migrate_knip(&knip, &mut config, &mut warnings);

        let rules = config.get("rules").unwrap().as_object().unwrap();
        assert_eq!(rules.get("unused-files").unwrap(), "error");
        assert!(!rules.contains_key("binaries"));

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].field.contains("binaries"));
    }
}
