use super::super::PluginResult;
use super::*;
use fallow_config::{ExternalPluginDef, ExternalUsedExport, PluginDetection};
use helpers::{check_plugin_detection, discover_json_config_files, process_config_result};
use std::collections::HashMap;

/// Helper: build a PackageJson with given dependency names.
fn make_pkg(deps: &[&str]) -> PackageJson {
    let map: HashMap<String, String> = deps.iter().map(|d| (d.to_string(), "*".into())).collect();
    PackageJson {
        dependencies: Some(map),
        ..Default::default()
    }
}

/// Helper: build a PackageJson with dev dependencies.
fn make_pkg_dev(deps: &[&str]) -> PackageJson {
    let map: HashMap<String, String> = deps.iter().map(|d| (d.to_string(), "*".into())).collect();
    PackageJson {
        dev_dependencies: Some(map),
        ..Default::default()
    }
}

// ── Plugin detection via enablers ────────────────────────────

#[test]
fn nextjs_detected_when_next_in_deps() {
    let registry = PluginRegistry::default();
    let pkg = make_pkg(&["next", "react"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(
        result.active_plugins.contains(&"nextjs".to_string()),
        "nextjs plugin should be active when 'next' is in deps"
    );
}

#[test]
fn nextjs_not_detected_without_next() {
    let registry = PluginRegistry::default();
    let pkg = make_pkg(&["react", "react-dom"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(
        !result.active_plugins.contains(&"nextjs".to_string()),
        "nextjs plugin should not be active without 'next' in deps"
    );
}

#[test]
fn prefix_enabler_matches_scoped_packages() {
    // Storybook uses "@storybook/" prefix matcher
    let registry = PluginRegistry::default();
    let pkg = make_pkg(&["@storybook/react"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(
        result.active_plugins.contains(&"storybook".to_string()),
        "storybook should activate via prefix match on @storybook/react"
    );
}

#[test]
fn prefix_enabler_does_not_match_without_slash() {
    // "storybook" (exact) should match, but "@storybook" (without /) should not match via prefix
    let registry = PluginRegistry::default();
    // This only has a package called "@storybookish" — it should NOT match
    let mut map = HashMap::new();
    map.insert("@storybookish".to_string(), "*".to_string());
    let pkg = PackageJson {
        dependencies: Some(map),
        ..Default::default()
    };
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(
        !result.active_plugins.contains(&"storybook".to_string()),
        "storybook should not activate for '@storybookish' (no slash prefix match)"
    );
}

#[test]
fn multiple_plugins_detected_simultaneously() {
    let registry = PluginRegistry::default();
    let pkg = make_pkg(&["next", "vitest", "typescript"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(result.active_plugins.contains(&"nextjs".to_string()));
    assert!(result.active_plugins.contains(&"vitest".to_string()));
    assert!(result.active_plugins.contains(&"typescript".to_string()));
}

#[test]
fn no_plugins_for_empty_deps() {
    let registry = PluginRegistry::default();
    let pkg = PackageJson::default();
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(
        result.active_plugins.is_empty(),
        "no plugins should activate with empty package.json"
    );
}

// ── Aggregation: entry patterns, tooling deps ────────────────

#[test]
fn active_plugin_contributes_entry_patterns() {
    let registry = PluginRegistry::default();
    let pkg = make_pkg(&["next"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    // Next.js should contribute App Router entry patterns
    assert!(
        result
            .entry_patterns
            .iter()
            .any(|(p, _)| p.contains("app/**/page")),
        "nextjs plugin should add app/**/page entry pattern"
    );
}

#[test]
fn inactive_plugin_does_not_contribute_entry_patterns() {
    let registry = PluginRegistry::default();
    let pkg = make_pkg(&["react"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    // Next.js patterns should not be present
    assert!(
        !result
            .entry_patterns
            .iter()
            .any(|(p, _)| p.contains("app/**/page")),
        "nextjs patterns should not appear when plugin is inactive"
    );
}

#[test]
fn active_plugin_contributes_tooling_deps() {
    let registry = PluginRegistry::default();
    let pkg = make_pkg(&["next"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(
        result.tooling_dependencies.contains(&"next".to_string()),
        "nextjs plugin should list 'next' as a tooling dependency"
    );
}

#[test]
fn dev_deps_also_trigger_plugins() {
    let registry = PluginRegistry::default();
    let pkg = make_pkg_dev(&["vitest"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(
        result.active_plugins.contains(&"vitest".to_string()),
        "vitest should activate from devDependencies"
    );
}

// ── External plugins ─────────────────────────────────────────

#[test]
fn external_plugin_detected_by_enablers() {
    let ext = ExternalPluginDef {
        schema: None,
        name: "my-framework".to_string(),
        detection: None,
        enablers: vec!["my-framework".to_string()],
        entry_points: vec!["src/routes/**/*.ts".to_string()],
        config_patterns: vec![],
        always_used: vec!["my.config.ts".to_string()],
        tooling_dependencies: vec!["my-framework-cli".to_string()],
        used_exports: vec![],
    };
    let registry = PluginRegistry::new(vec![ext]);
    let pkg = make_pkg(&["my-framework"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(result.active_plugins.contains(&"my-framework".to_string()));
    assert!(
        result
            .entry_patterns
            .iter()
            .any(|(p, _)| p == "src/routes/**/*.ts")
    );
    assert!(
        result
            .tooling_dependencies
            .contains(&"my-framework-cli".to_string())
    );
}

#[test]
fn external_plugin_not_detected_when_dep_missing() {
    let ext = ExternalPluginDef {
        schema: None,
        name: "my-framework".to_string(),
        detection: None,
        enablers: vec!["my-framework".to_string()],
        entry_points: vec!["src/routes/**/*.ts".to_string()],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let registry = PluginRegistry::new(vec![ext]);
    let pkg = make_pkg(&["react"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(!result.active_plugins.contains(&"my-framework".to_string()));
    assert!(
        !result
            .entry_patterns
            .iter()
            .any(|(p, _)| p == "src/routes/**/*.ts")
    );
}

#[test]
fn external_plugin_prefix_enabler() {
    let ext = ExternalPluginDef {
        schema: None,
        name: "custom-plugin".to_string(),
        detection: None,
        enablers: vec!["@custom/".to_string()],
        entry_points: vec!["custom/**/*.ts".to_string()],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let registry = PluginRegistry::new(vec![ext]);
    let pkg = make_pkg(&["@custom/core"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(result.active_plugins.contains(&"custom-plugin".to_string()));
}

#[test]
fn external_plugin_detection_dependency() {
    let ext = ExternalPluginDef {
        schema: None,
        name: "detected-plugin".to_string(),
        detection: Some(PluginDetection::Dependency {
            package: "special-dep".to_string(),
        }),
        enablers: vec![],
        entry_points: vec!["special/**/*.ts".to_string()],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let registry = PluginRegistry::new(vec![ext]);
    let pkg = make_pkg(&["special-dep"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(
        result
            .active_plugins
            .contains(&"detected-plugin".to_string())
    );
}

#[test]
fn external_plugin_detection_any_combinator() {
    let ext = ExternalPluginDef {
        schema: None,
        name: "any-plugin".to_string(),
        detection: Some(PluginDetection::Any {
            conditions: vec![
                PluginDetection::Dependency {
                    package: "pkg-a".to_string(),
                },
                PluginDetection::Dependency {
                    package: "pkg-b".to_string(),
                },
            ],
        }),
        enablers: vec![],
        entry_points: vec!["any/**/*.ts".to_string()],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let registry = PluginRegistry::new(vec![ext]);
    // Only pkg-b present — should still match via Any
    let pkg = make_pkg(&["pkg-b"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(result.active_plugins.contains(&"any-plugin".to_string()));
}

#[test]
fn external_plugin_detection_all_combinator_fails_partial() {
    let ext = ExternalPluginDef {
        schema: None,
        name: "all-plugin".to_string(),
        detection: Some(PluginDetection::All {
            conditions: vec![
                PluginDetection::Dependency {
                    package: "pkg-a".to_string(),
                },
                PluginDetection::Dependency {
                    package: "pkg-b".to_string(),
                },
            ],
        }),
        enablers: vec![],
        entry_points: vec![],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let registry = PluginRegistry::new(vec![ext]);
    // Only pkg-a present — All requires both
    let pkg = make_pkg(&["pkg-a"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(!result.active_plugins.contains(&"all-plugin".to_string()));
}

#[test]
fn external_plugin_used_exports_aggregated() {
    let ext = ExternalPluginDef {
        schema: None,
        name: "ue-plugin".to_string(),
        detection: None,
        enablers: vec!["ue-dep".to_string()],
        entry_points: vec![],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![ExternalUsedExport {
            pattern: "pages/**/*.tsx".to_string(),
            exports: vec!["default".to_string(), "getServerSideProps".to_string()],
        }],
    };
    let registry = PluginRegistry::new(vec![ext]);
    let pkg = make_pkg(&["ue-dep"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(result.used_exports.iter().any(|(pat, exports)| {
        pat == "pages/**/*.tsx" && exports.contains(&"default".to_string())
    }));
}

#[test]
fn external_plugin_without_enablers_or_detection_stays_inactive() {
    let ext = ExternalPluginDef {
        schema: None,
        name: "orphan-plugin".to_string(),
        detection: None,
        enablers: vec![],
        entry_points: vec!["orphan/**/*.ts".to_string()],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let registry = PluginRegistry::new(vec![ext]);
    let pkg = make_pkg(&["anything"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(!result.active_plugins.contains(&"orphan-plugin".to_string()));
}

// ── Virtual module prefixes ──────────────────────────────────

#[test]
fn nuxt_contributes_virtual_module_prefixes() {
    let registry = PluginRegistry::default();
    let pkg = make_pkg(&["nuxt"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(
        result.virtual_module_prefixes.contains(&"#".to_string()),
        "nuxt should contribute '#' virtual module prefix"
    );
}

// ── process_static_patterns: always_used aggregation ─────────

#[test]
fn active_plugin_contributes_always_used_files() {
    let registry = PluginRegistry::default();
    let pkg = make_pkg(&["next"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    // Next.js marks next.config.{ts,js,mjs,cjs} as always used
    assert!(
        result
            .always_used
            .iter()
            .any(|(p, name)| p.contains("next.config") && name == "nextjs"),
        "nextjs plugin should add next.config to always_used"
    );
}

#[test]
fn active_plugin_contributes_config_patterns() {
    let registry = PluginRegistry::default();
    let pkg = make_pkg(&["next"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(
        result
            .config_patterns
            .iter()
            .any(|p| p.contains("next.config")),
        "nextjs plugin should add next.config to config_patterns"
    );
}

#[test]
fn active_plugin_contributes_used_exports() {
    let registry = PluginRegistry::default();
    let pkg = make_pkg(&["next"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    // Next.js has used_exports for page patterns (default, getServerSideProps, etc.)
    assert!(
        !result.used_exports.is_empty(),
        "nextjs plugin should contribute used_exports"
    );
    assert!(
        result
            .used_exports
            .iter()
            .any(|(_, exports)| exports.contains(&"default".to_string())),
        "nextjs used_exports should include 'default'"
    );
}

#[test]
fn sveltekit_contributes_path_aliases() {
    let registry = PluginRegistry::default();
    let pkg = make_pkg(&["@sveltejs/kit"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(
        result
            .path_aliases
            .iter()
            .any(|(prefix, _)| prefix == "$lib/"),
        "sveltekit plugin should contribute $lib/ path alias"
    );
}

#[test]
fn docusaurus_contributes_virtual_module_prefixes() {
    let registry = PluginRegistry::default();
    let pkg = make_pkg(&["@docusaurus/core"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(
        result
            .virtual_module_prefixes
            .iter()
            .any(|p| p == "@theme/"),
        "docusaurus should contribute @theme/ virtual module prefix"
    );
}

// ── External plugin: detection takes priority over enablers ──

#[test]
fn external_plugin_detection_overrides_enablers() {
    // When detection is set AND enablers is set, detection should be used.
    // Detection says "requires pkg-x", enablers says "pkg-y".
    // With only pkg-y in deps, plugin should NOT activate because detection takes priority.
    let ext = ExternalPluginDef {
        schema: None,
        name: "priority-test".to_string(),
        detection: Some(PluginDetection::Dependency {
            package: "pkg-x".to_string(),
        }),
        enablers: vec!["pkg-y".to_string()],
        entry_points: vec!["src/**/*.ts".to_string()],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let registry = PluginRegistry::new(vec![ext]);
    let pkg = make_pkg(&["pkg-y"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(
        !result.active_plugins.contains(&"priority-test".to_string()),
        "detection should take priority over enablers — pkg-x not present"
    );
}

#[test]
fn external_plugin_detection_overrides_enablers_positive() {
    // Same as above but with pkg-x present — should activate via detection
    let ext = ExternalPluginDef {
        schema: None,
        name: "priority-test".to_string(),
        detection: Some(PluginDetection::Dependency {
            package: "pkg-x".to_string(),
        }),
        enablers: vec!["pkg-y".to_string()],
        entry_points: vec![],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let registry = PluginRegistry::new(vec![ext]);
    let pkg = make_pkg(&["pkg-x"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(
        result.active_plugins.contains(&"priority-test".to_string()),
        "detection should activate when pkg-x is present"
    );
}

// ── External plugin: config_patterns are added to always_used ─

#[test]
fn external_plugin_config_patterns_added_to_always_used() {
    let ext = ExternalPluginDef {
        schema: None,
        name: "cfg-plugin".to_string(),
        detection: None,
        enablers: vec!["cfg-dep".to_string()],
        entry_points: vec![],
        config_patterns: vec!["my-tool.config.ts".to_string()],
        always_used: vec!["setup.ts".to_string()],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let registry = PluginRegistry::new(vec![ext]);
    let pkg = make_pkg(&["cfg-dep"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    // Both config_patterns AND always_used should be in the always_used result
    assert!(
        result
            .always_used
            .iter()
            .any(|(p, _)| p == "my-tool.config.ts"),
        "external plugin config_patterns should be in always_used"
    );
    assert!(
        result.always_used.iter().any(|(p, _)| p == "setup.ts"),
        "external plugin always_used should be in always_used"
    );
}

// ── External plugin: All combinator succeeds when all present ─

#[test]
fn external_plugin_detection_all_combinator_succeeds() {
    let ext = ExternalPluginDef {
        schema: None,
        name: "all-pass".to_string(),
        detection: Some(PluginDetection::All {
            conditions: vec![
                PluginDetection::Dependency {
                    package: "pkg-a".to_string(),
                },
                PluginDetection::Dependency {
                    package: "pkg-b".to_string(),
                },
            ],
        }),
        enablers: vec![],
        entry_points: vec!["all/**/*.ts".to_string()],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let registry = PluginRegistry::new(vec![ext]);
    let pkg = make_pkg(&["pkg-a", "pkg-b"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(
        result.active_plugins.contains(&"all-pass".to_string()),
        "All combinator should pass when all dependencies present"
    );
}

// ── External plugin: nested Any inside All ───────────────────

#[test]
fn external_plugin_nested_any_inside_all() {
    let ext = ExternalPluginDef {
        schema: None,
        name: "nested-plugin".to_string(),
        detection: Some(PluginDetection::All {
            conditions: vec![
                PluginDetection::Dependency {
                    package: "required-dep".to_string(),
                },
                PluginDetection::Any {
                    conditions: vec![
                        PluginDetection::Dependency {
                            package: "optional-a".to_string(),
                        },
                        PluginDetection::Dependency {
                            package: "optional-b".to_string(),
                        },
                    ],
                },
            ],
        }),
        enablers: vec![],
        entry_points: vec![],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let registry = PluginRegistry::new(vec![ext.clone()]);
    // Has required-dep + optional-b → should pass
    let pkg = make_pkg(&["required-dep", "optional-b"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(
        result.active_plugins.contains(&"nested-plugin".to_string()),
        "nested Any inside All: should pass with required-dep + optional-b"
    );

    // Has only required-dep (missing any optional) → should fail
    let registry2 = PluginRegistry::new(vec![ext]);
    let pkg2 = make_pkg(&["required-dep"]);
    let result2 = registry2.run(&pkg2, Path::new("/project"), &[]);
    assert!(
        !result2
            .active_plugins
            .contains(&"nested-plugin".to_string()),
        "nested Any inside All: should fail with only required-dep (no optional)"
    );
}

// ── External plugin: FileExists detection ────────────────────

#[test]
fn external_plugin_detection_file_exists_against_discovered() {
    // FileExists checks discovered_files first
    let ext = ExternalPluginDef {
        schema: None,
        name: "file-check".to_string(),
        detection: Some(PluginDetection::FileExists {
            pattern: "src/special.ts".to_string(),
        }),
        enablers: vec![],
        entry_points: vec!["special/**/*.ts".to_string()],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let registry = PluginRegistry::new(vec![ext]);
    let pkg = PackageJson::default();
    let discovered = vec![PathBuf::from("/project/src/special.ts")];
    let result = registry.run(&pkg, Path::new("/project"), &discovered);
    assert!(
        result.active_plugins.contains(&"file-check".to_string()),
        "FileExists detection should match against discovered files"
    );
}

#[test]
fn external_plugin_detection_file_exists_no_match() {
    let ext = ExternalPluginDef {
        schema: None,
        name: "file-miss".to_string(),
        detection: Some(PluginDetection::FileExists {
            pattern: "src/nonexistent.ts".to_string(),
        }),
        enablers: vec![],
        entry_points: vec![],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let registry = PluginRegistry::new(vec![ext]);
    let pkg = PackageJson::default();
    let result = registry.run(&pkg, Path::new("/nonexistent-project-root-xyz"), &[]);
    assert!(
        !result.active_plugins.contains(&"file-miss".to_string()),
        "FileExists detection should not match when file doesn't exist"
    );
}

// ── check_plugin_detection unit tests ────────────────────────

#[test]
fn check_plugin_detection_dependency_matches() {
    let detection = PluginDetection::Dependency {
        package: "react".to_string(),
    };
    let deps = vec!["react", "react-dom"];
    assert!(check_plugin_detection(
        &detection,
        &deps,
        Path::new("/project"),
        &[]
    ));
}

#[test]
fn check_plugin_detection_dependency_no_match() {
    let detection = PluginDetection::Dependency {
        package: "vue".to_string(),
    };
    let deps = vec!["react"];
    assert!(!check_plugin_detection(
        &detection,
        &deps,
        Path::new("/project"),
        &[]
    ));
}

#[test]
fn check_plugin_detection_file_exists_discovered_files() {
    let detection = PluginDetection::FileExists {
        pattern: "src/index.ts".to_string(),
    };
    let discovered = vec![PathBuf::from("/root/src/index.ts")];
    assert!(check_plugin_detection(
        &detection,
        &[],
        Path::new("/root"),
        &discovered
    ));
}

#[test]
fn check_plugin_detection_file_exists_glob_pattern_in_discovered() {
    let detection = PluginDetection::FileExists {
        pattern: "src/**/*.config.ts".to_string(),
    };
    let discovered = vec![
        PathBuf::from("/root/src/app.config.ts"),
        PathBuf::from("/root/src/utils/helper.ts"),
    ];
    assert!(check_plugin_detection(
        &detection,
        &[],
        Path::new("/root"),
        &discovered
    ));
}

#[test]
fn check_plugin_detection_file_exists_no_discovered_match() {
    let detection = PluginDetection::FileExists {
        pattern: "src/specific.ts".to_string(),
    };
    let discovered = vec![PathBuf::from("/root/src/other.ts")];
    // No discovered match, and disk glob won't find anything in nonexistent path
    assert!(!check_plugin_detection(
        &detection,
        &[],
        Path::new("/nonexistent-root-xyz"),
        &discovered
    ));
}

#[test]
fn check_plugin_detection_all_empty_conditions() {
    // All with empty conditions → vacuously true
    let detection = PluginDetection::All { conditions: vec![] };
    assert!(check_plugin_detection(
        &detection,
        &[],
        Path::new("/project"),
        &[]
    ));
}

#[test]
fn check_plugin_detection_any_empty_conditions() {
    // Any with empty conditions → vacuously false
    let detection = PluginDetection::Any { conditions: vec![] };
    assert!(!check_plugin_detection(
        &detection,
        &[],
        Path::new("/project"),
        &[]
    ));
}

// ── process_config_result ────────────────────────────────────

#[test]
fn process_config_result_merges_all_fields() {
    let mut aggregated = AggregatedPluginResult::default();
    let config_result = PluginResult {
        entry_patterns: vec!["src/routes/**/*.ts".to_string()],
        referenced_dependencies: vec!["lodash".to_string(), "axios".to_string()],
        always_used_files: vec!["setup.ts".to_string()],
        setup_files: vec![PathBuf::from("/project/test/setup.ts")],
    };
    process_config_result("test-plugin", config_result, &mut aggregated);

    assert_eq!(aggregated.entry_patterns.len(), 1);
    assert_eq!(aggregated.entry_patterns[0].0, "src/routes/**/*.ts");
    assert_eq!(aggregated.entry_patterns[0].1, "test-plugin");

    assert_eq!(aggregated.referenced_dependencies.len(), 2);
    assert!(
        aggregated
            .referenced_dependencies
            .contains(&"lodash".to_string())
    );
    assert!(
        aggregated
            .referenced_dependencies
            .contains(&"axios".to_string())
    );

    assert_eq!(aggregated.discovered_always_used.len(), 1);
    assert_eq!(aggregated.discovered_always_used[0].0, "setup.ts");
    assert_eq!(aggregated.discovered_always_used[0].1, "test-plugin");

    assert_eq!(aggregated.setup_files.len(), 1);
    assert_eq!(
        aggregated.setup_files[0].0,
        PathBuf::from("/project/test/setup.ts")
    );
    assert_eq!(aggregated.setup_files[0].1, "test-plugin");
}

#[test]
fn process_config_result_accumulates_across_multiple_calls() {
    let mut aggregated = AggregatedPluginResult::default();

    let result1 = PluginResult {
        entry_patterns: vec!["a.ts".to_string()],
        referenced_dependencies: vec!["dep-a".to_string()],
        always_used_files: vec![],
        setup_files: vec![PathBuf::from("/project/setup-a.ts")],
    };
    let result2 = PluginResult {
        entry_patterns: vec!["b.ts".to_string()],
        referenced_dependencies: vec!["dep-b".to_string()],
        always_used_files: vec!["c.ts".to_string()],
        setup_files: vec![],
    };

    process_config_result("plugin-a", result1, &mut aggregated);
    process_config_result("plugin-b", result2, &mut aggregated);

    // Verify entry patterns are tagged with the correct plugin name
    assert_eq!(aggregated.entry_patterns.len(), 2);
    assert_eq!(aggregated.entry_patterns[0].0, "a.ts");
    assert_eq!(aggregated.entry_patterns[0].1, "plugin-a");
    assert_eq!(aggregated.entry_patterns[1].0, "b.ts");
    assert_eq!(aggregated.entry_patterns[1].1, "plugin-b");

    // Verify referenced dependencies from both calls
    assert_eq!(aggregated.referenced_dependencies.len(), 2);
    assert!(
        aggregated
            .referenced_dependencies
            .contains(&"dep-a".to_string())
    );
    assert!(
        aggregated
            .referenced_dependencies
            .contains(&"dep-b".to_string())
    );

    // Verify always_used_files tagged with plugin-b
    assert_eq!(aggregated.discovered_always_used.len(), 1);
    assert_eq!(aggregated.discovered_always_used[0].0, "c.ts");
    assert_eq!(aggregated.discovered_always_used[0].1, "plugin-b");

    // Verify setup_files tagged with plugin-a
    assert_eq!(aggregated.setup_files.len(), 1);
    assert_eq!(
        aggregated.setup_files[0].0,
        PathBuf::from("/project/setup-a.ts")
    );
    assert_eq!(aggregated.setup_files[0].1, "plugin-a");
}

// ── PluginResult::is_empty ───────────────────────────────────

#[test]
fn plugin_result_is_empty_for_default() {
    assert!(
        PluginResult::default().is_empty(),
        "default PluginResult should be empty"
    );
}

#[test]
fn plugin_result_not_empty_when_any_field_set() {
    let fields: Vec<PluginResult> = vec![
        PluginResult {
            entry_patterns: vec!["src/**/*.ts".to_string()],
            ..Default::default()
        },
        PluginResult {
            referenced_dependencies: vec!["lodash".to_string()],
            ..Default::default()
        },
        PluginResult {
            always_used_files: vec!["setup.ts".to_string()],
            ..Default::default()
        },
        PluginResult {
            setup_files: vec![PathBuf::from("/project/setup.ts")],
            ..Default::default()
        },
    ];
    for (i, result) in fields.iter().enumerate() {
        assert!(
            !result.is_empty(),
            "PluginResult with field index {i} set should not be empty"
        );
    }
}

// ── check_has_config_file ────────────────────────────────────

#[test]
fn check_has_config_file_returns_true_when_file_matches() {
    let registry = PluginRegistry::default();
    let matchers = registry.precompile_config_matchers();

    // Find the nextjs plugin entry in matchers
    let has_next = matchers.iter().any(|(p, _)| p.name() == "nextjs");
    assert!(has_next, "nextjs should be in precompiled matchers");

    let next_plugin: &dyn Plugin = &super::super::nextjs::NextJsPlugin;
    // A file matching next.config.ts should be detected
    let abs = PathBuf::from("/project/next.config.ts");
    let relative_files: Vec<(&PathBuf, String)> = vec![(&abs, "next.config.ts".to_string())];

    assert!(
        check_has_config_file(next_plugin, &matchers, &relative_files),
        "check_has_config_file should return true when config file matches"
    );
}

#[test]
fn check_has_config_file_returns_false_when_no_match() {
    let registry = PluginRegistry::default();
    let matchers = registry.precompile_config_matchers();

    let next_plugin: &dyn Plugin = &super::super::nextjs::NextJsPlugin;
    let abs = PathBuf::from("/project/src/index.ts");
    let relative_files: Vec<(&PathBuf, String)> = vec![(&abs, "src/index.ts".to_string())];

    assert!(
        !check_has_config_file(next_plugin, &matchers, &relative_files),
        "check_has_config_file should return false when no config file matches"
    );
}

#[test]
fn check_has_config_file_returns_false_for_plugin_without_config_patterns() {
    let registry = PluginRegistry::default();
    let matchers = registry.precompile_config_matchers();

    // MSW plugin has no config_patterns
    let msw_plugin: &dyn Plugin = &super::super::msw::MswPlugin;
    let abs = PathBuf::from("/project/something.ts");
    let relative_files: Vec<(&PathBuf, String)> = vec![(&abs, "something.ts".to_string())];

    assert!(
        !check_has_config_file(msw_plugin, &matchers, &relative_files),
        "plugin with no config_patterns should return false"
    );
}

// ── discover_json_config_files ───────────────────────────────

#[test]
fn discover_json_config_files_skips_resolved_plugins() {
    let registry = PluginRegistry::default();
    let matchers = registry.precompile_config_matchers();

    let mut resolved: FxHashSet<&str> = FxHashSet::default();
    // Mark all plugins as resolved — should return empty
    for (plugin, _) in &matchers {
        resolved.insert(plugin.name());
    }

    let json_configs = discover_json_config_files(&matchers, &resolved, &[], Path::new("/project"));
    assert!(
        json_configs.is_empty(),
        "discover_json_config_files should skip all resolved plugins"
    );
}

#[test]
fn discover_json_config_files_returns_empty_for_nonexistent_root() {
    let registry = PluginRegistry::default();
    let matchers = registry.precompile_config_matchers();
    let resolved: FxHashSet<&str> = FxHashSet::default();

    let json_configs = discover_json_config_files(
        &matchers,
        &resolved,
        &[],
        Path::new("/nonexistent-root-xyz-abc"),
    );
    assert!(
        json_configs.is_empty(),
        "discover_json_config_files should return empty for nonexistent root"
    );
}

// ── process_static_patterns: comprehensive ───────────────────

#[test]
fn process_static_patterns_populates_all_fields() {
    let mut result = AggregatedPluginResult::default();
    let plugin: &dyn Plugin = &super::super::nextjs::NextJsPlugin;
    helpers::process_static_patterns(plugin, Path::new("/project"), &mut result);

    assert!(result.active_plugins.contains(&"nextjs".to_string()));
    assert!(!result.entry_patterns.is_empty());
    assert!(!result.config_patterns.is_empty());
    assert!(!result.always_used.is_empty());
    assert!(!result.tooling_dependencies.is_empty());
    // Next.js has used_exports for page patterns
    assert!(!result.used_exports.is_empty());
}

#[test]
fn process_static_patterns_entry_patterns_tagged_with_plugin_name() {
    let mut result = AggregatedPluginResult::default();
    let plugin: &dyn Plugin = &super::super::nextjs::NextJsPlugin;
    helpers::process_static_patterns(plugin, Path::new("/project"), &mut result);

    for (_, name) in &result.entry_patterns {
        assert_eq!(
            name, "nextjs",
            "all entry patterns should be tagged with 'nextjs'"
        );
    }
}

#[test]
fn process_static_patterns_always_used_tagged_with_plugin_name() {
    let mut result = AggregatedPluginResult::default();
    let plugin: &dyn Plugin = &super::super::nextjs::NextJsPlugin;
    helpers::process_static_patterns(plugin, Path::new("/project"), &mut result);

    for (_, name) in &result.always_used {
        assert_eq!(
            name, "nextjs",
            "all always_used should be tagged with 'nextjs'"
        );
    }
}

// ── Multiple external plugins ────────────────────────────────

#[test]
fn multiple_external_plugins_independently_activated() {
    let ext_a = ExternalPluginDef {
        schema: None,
        name: "ext-a".to_string(),
        detection: None,
        enablers: vec!["dep-a".to_string()],
        entry_points: vec!["a/**/*.ts".to_string()],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let ext_b = ExternalPluginDef {
        schema: None,
        name: "ext-b".to_string(),
        detection: None,
        enablers: vec!["dep-b".to_string()],
        entry_points: vec!["b/**/*.ts".to_string()],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let registry = PluginRegistry::new(vec![ext_a, ext_b]);
    // Only dep-a present
    let pkg = make_pkg(&["dep-a"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(result.active_plugins.contains(&"ext-a".to_string()));
    assert!(!result.active_plugins.contains(&"ext-b".to_string()));
    assert!(result.entry_patterns.iter().any(|(p, _)| p == "a/**/*.ts"));
    assert!(!result.entry_patterns.iter().any(|(p, _)| p == "b/**/*.ts"));
}

// ── External plugin: multiple used_exports ───────────────────

#[test]
fn external_plugin_multiple_used_exports() {
    let ext = ExternalPluginDef {
        schema: None,
        name: "multi-ue".to_string(),
        detection: None,
        enablers: vec!["multi-dep".to_string()],
        entry_points: vec![],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![
            ExternalUsedExport {
                pattern: "routes/**/*.ts".to_string(),
                exports: vec!["loader".to_string(), "action".to_string()],
            },
            ExternalUsedExport {
                pattern: "api/**/*.ts".to_string(),
                exports: vec!["GET".to_string(), "POST".to_string()],
            },
        ],
    };
    let registry = PluginRegistry::new(vec![ext]);
    let pkg = make_pkg(&["multi-dep"]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert_eq!(
        result.used_exports.len(),
        2,
        "should have two used_export entries"
    );
    assert!(result.used_exports.iter().any(|(pat, exports)| {
        pat == "routes/**/*.ts" && exports.contains(&"loader".to_string())
    }));
    assert!(
        result
            .used_exports
            .iter()
            .any(|(pat, exports)| { pat == "api/**/*.ts" && exports.contains(&"GET".to_string()) })
    );
}

// ── Registry creation / default ──────────────────────────────

#[test]
fn default_registry_has_all_builtin_plugins() {
    let registry = PluginRegistry::default();
    // Verify we have the expected number of built-in plugins (84 as per docs)
    // We test a representative sample to avoid brittle exact count checks.
    let pkg = make_pkg(&[
        "next",
        "vitest",
        "eslint",
        "typescript",
        "tailwindcss",
        "prisma",
    ]);
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(result.active_plugins.contains(&"nextjs".to_string()));
    assert!(result.active_plugins.contains(&"vitest".to_string()));
    assert!(result.active_plugins.contains(&"eslint".to_string()));
    assert!(result.active_plugins.contains(&"typescript".to_string()));
    assert!(result.active_plugins.contains(&"tailwind".to_string()));
    assert!(result.active_plugins.contains(&"prisma".to_string()));
}

// ── run_workspace_fast: early exit with no active plugins ────

#[test]
fn run_workspace_fast_returns_empty_for_no_active_plugins() {
    let registry = PluginRegistry::default();
    let matchers = registry.precompile_config_matchers();
    let pkg = PackageJson::default();
    let relative_files: Vec<(&PathBuf, String)> = vec![];
    let result = registry.run_workspace_fast(
        &pkg,
        Path::new("/workspace/pkg"),
        Path::new("/workspace"),
        &matchers,
        &relative_files,
    );
    assert!(result.active_plugins.is_empty());
    assert!(result.entry_patterns.is_empty());
    assert!(result.config_patterns.is_empty());
    assert!(result.always_used.is_empty());
}

#[test]
fn run_workspace_fast_detects_active_plugins() {
    let registry = PluginRegistry::default();
    let matchers = registry.precompile_config_matchers();
    let pkg = make_pkg(&["next"]);
    let relative_files: Vec<(&PathBuf, String)> = vec![];
    let result = registry.run_workspace_fast(
        &pkg,
        Path::new("/workspace/pkg"),
        Path::new("/workspace"),
        &matchers,
        &relative_files,
    );
    assert!(result.active_plugins.contains(&"nextjs".to_string()));
    assert!(!result.entry_patterns.is_empty());
}

#[test]
fn run_workspace_fast_filters_matchers_to_active_plugins() {
    let registry = PluginRegistry::default();
    let matchers = registry.precompile_config_matchers();

    // With only 'next' in deps, config matchers for other plugins (jest, vite, etc.)
    // should be excluded from the workspace run.
    let pkg = make_pkg(&["next"]);
    let relative_files: Vec<(&PathBuf, String)> = vec![];
    let result = registry.run_workspace_fast(
        &pkg,
        Path::new("/workspace/pkg"),
        Path::new("/workspace"),
        &matchers,
        &relative_files,
    );
    // Only nextjs should be active
    assert!(result.active_plugins.contains(&"nextjs".to_string()));
    assert!(
        !result.active_plugins.contains(&"jest".to_string()),
        "jest should not be active without jest dep"
    );
}

// ── process_external_plugins edge cases ──────────────────────

#[test]
fn process_external_plugins_empty_list() {
    let mut result = AggregatedPluginResult::default();
    helpers::process_external_plugins(&[], &[], Path::new("/project"), &[], &mut result);
    assert!(result.active_plugins.is_empty());
}

#[test]
fn process_external_plugins_prefix_enabler_requires_slash() {
    // Prefix enabler "@org/" should NOT match "@organism" (no trailing slash)
    let ext = ExternalPluginDef {
        schema: None,
        name: "prefix-strict".to_string(),
        detection: None,
        enablers: vec!["@org/".to_string()],
        entry_points: vec![],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let mut result = AggregatedPluginResult::default();
    let deps = vec!["@organism".to_string()];
    helpers::process_external_plugins(&[ext], &deps, Path::new("/project"), &[], &mut result);
    assert!(
        !result.active_plugins.contains(&"prefix-strict".to_string()),
        "@org/ prefix should not match @organism"
    );
}

#[test]
fn process_external_plugins_prefix_enabler_matches_scoped() {
    let ext = ExternalPluginDef {
        schema: None,
        name: "prefix-match".to_string(),
        detection: None,
        enablers: vec!["@org/".to_string()],
        entry_points: vec![],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let mut result = AggregatedPluginResult::default();
    let deps = vec!["@org/core".to_string()];
    helpers::process_external_plugins(&[ext], &deps, Path::new("/project"), &[], &mut result);
    assert!(
        result.active_plugins.contains(&"prefix-match".to_string()),
        "@org/ prefix should match @org/core"
    );
}

// ── Config file matching with filesystem ─────────────────────

#[test]
fn run_with_config_file_in_discovered_files() {
    // When a config file is in the discovered files list, config resolution
    // should be attempted. We can test this with a temp dir.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // Create a vitest config file
    std::fs::write(
        root.join("vitest.config.ts"),
        r"
import { defineConfig } from 'vitest/config';
export default defineConfig({
test: {
    include: ['tests/**/*.test.ts'],
    setupFiles: ['./test/setup.ts'],
}
});
",
    )
    .unwrap();

    let registry = PluginRegistry::default();
    let pkg = make_pkg(&["vitest"]);
    let config_path = root.join("vitest.config.ts");
    let discovered = vec![config_path];
    let result = registry.run(&pkg, root, &discovered);

    assert!(result.active_plugins.contains(&"vitest".to_string()));
    // Config parsing should have discovered additional entry patterns
    assert!(
        result
            .entry_patterns
            .iter()
            .any(|(p, _)| p == "tests/**/*.test.ts"),
        "config parsing should extract test.include patterns"
    );
    // Config parsing should have discovered setup files
    assert!(
        !result.setup_files.is_empty(),
        "config parsing should extract setupFiles"
    );
    // vitest/config should be a referenced dependency (from the import)
    assert!(
        result.referenced_dependencies.iter().any(|d| d == "vitest"),
        "config parsing should extract imports as referenced dependencies"
    );
}

#[test]
fn run_discovers_json_config_on_disk_fallback() {
    // JSON config files like angular.json are not in the discovered source file set.
    // They should be found via the filesystem fallback (Phase 3b).
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // Create a minimal angular.json
    std::fs::write(
        root.join("angular.json"),
        r#"{
            "version": 1,
            "projects": {
                "app": {
                    "root": "",
                    "architect": {
                        "build": {
                            "options": {
                                "main": "src/main.ts"
                            }
                        }
                    }
                }
            }
        }"#,
    )
    .unwrap();

    let registry = PluginRegistry::default();
    let pkg = make_pkg(&["@angular/core"]);
    // No source files discovered — angular.json should be found via disk fallback
    let result = registry.run(&pkg, root, &[]);

    assert!(result.active_plugins.contains(&"angular".to_string()));
    // Angular config parsing should extract main entry point
    assert!(
        result
            .entry_patterns
            .iter()
            .any(|(p, _)| p.contains("src/main.ts")),
        "angular.json parsing should extract main entry point"
    );
}

// ── Peer and optional dependencies trigger plugins ────────────

#[test]
fn peer_deps_trigger_plugins() {
    let mut map = HashMap::new();
    map.insert("next".to_string(), "^14.0.0".to_string());
    let pkg = PackageJson {
        peer_dependencies: Some(map),
        ..Default::default()
    };
    let registry = PluginRegistry::default();
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(
        result.active_plugins.contains(&"nextjs".to_string()),
        "peerDependencies should trigger plugin detection"
    );
}

#[test]
fn optional_deps_trigger_plugins() {
    let mut map = HashMap::new();
    map.insert("next".to_string(), "^14.0.0".to_string());
    let pkg = PackageJson {
        optional_dependencies: Some(map),
        ..Default::default()
    };
    let registry = PluginRegistry::default();
    let result = registry.run(&pkg, Path::new("/project"), &[]);
    assert!(
        result.active_plugins.contains(&"nextjs".to_string()),
        "optionalDependencies should trigger plugin detection"
    );
}

// ── FileExists detection with glob in discovered files ───────

#[test]
fn check_plugin_detection_file_exists_wildcard_in_discovered() {
    let detection = PluginDetection::FileExists {
        pattern: "**/*.svelte".to_string(),
    };
    let discovered = vec![
        PathBuf::from("/root/src/App.svelte"),
        PathBuf::from("/root/src/utils.ts"),
    ];
    assert!(
        check_plugin_detection(&detection, &[], Path::new("/root"), &discovered),
        "FileExists with glob should match discovered .svelte file"
    );
}

// ── External plugin: FileExists with All combinator ──────────

#[test]
fn external_plugin_detection_all_with_file_and_dep() {
    let ext = ExternalPluginDef {
        schema: None,
        name: "combo-check".to_string(),
        detection: Some(PluginDetection::All {
            conditions: vec![
                PluginDetection::Dependency {
                    package: "my-lib".to_string(),
                },
                PluginDetection::FileExists {
                    pattern: "src/setup.ts".to_string(),
                },
            ],
        }),
        enablers: vec![],
        entry_points: vec!["src/**/*.ts".to_string()],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let registry = PluginRegistry::new(vec![ext]);
    let pkg = make_pkg(&["my-lib"]);
    let discovered = vec![PathBuf::from("/project/src/setup.ts")];
    let result = registry.run(&pkg, Path::new("/project"), &discovered);
    assert!(
        result.active_plugins.contains(&"combo-check".to_string()),
        "All(dep + fileExists) should pass when both conditions met"
    );
}

#[test]
fn external_plugin_detection_all_dep_and_file_missing_file() {
    let ext = ExternalPluginDef {
        schema: None,
        name: "combo-fail".to_string(),
        detection: Some(PluginDetection::All {
            conditions: vec![
                PluginDetection::Dependency {
                    package: "my-lib".to_string(),
                },
                PluginDetection::FileExists {
                    pattern: "src/nonexistent-xyz.ts".to_string(),
                },
            ],
        }),
        enablers: vec![],
        entry_points: vec![],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let registry = PluginRegistry::new(vec![ext]);
    let pkg = make_pkg(&["my-lib"]);
    let result = registry.run(&pkg, Path::new("/nonexistent-root-xyz"), &[]);
    assert!(
        !result.active_plugins.contains(&"combo-fail".to_string()),
        "All(dep + fileExists) should fail when file is missing"
    );
}

// ── Vitest file-based activation ─────────────────────────────

#[test]
fn vitest_activates_by_config_file_existence() {
    // Vitest has a custom is_enabled_with_deps that also checks for config files
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(root.join("vitest.config.ts"), "").unwrap();

    let registry = PluginRegistry::default();
    // No vitest in deps, but config file exists
    let pkg = PackageJson::default();
    let result = registry.run(&pkg, root, &[]);
    assert!(
        result.active_plugins.contains(&"vitest".to_string()),
        "vitest should activate when vitest.config.ts exists on disk"
    );
}

#[test]
fn eslint_activates_by_config_file_existence() {
    // ESLint also has file-based activation
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(root.join("eslint.config.js"), "").unwrap();

    let registry = PluginRegistry::default();
    let pkg = PackageJson::default();
    let result = registry.run(&pkg, root, &[]);
    assert!(
        result.active_plugins.contains(&"eslint".to_string()),
        "eslint should activate when eslint.config.js exists on disk"
    );
}

// ── discover_json_config_files: glob pattern in subdirectories

#[test]
fn discover_json_config_files_finds_in_subdirectory() {
    // Nx plugin has "**/project.json" config pattern — glob-based discovery
    // should check directories where discovered source files live.
    // The function checks the parent directory of each discovered source file.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let subdir = root.join("packages").join("app");
    std::fs::create_dir_all(&subdir).unwrap();
    std::fs::write(subdir.join("project.json"), r#"{"name": "app"}"#).unwrap();

    let registry = PluginRegistry::default();
    let matchers = registry.precompile_config_matchers();
    let resolved: FxHashSet<&str> = FxHashSet::default();

    // The source file's parent must be packages/app/ so that project.json
    // is found via dir.join("project.json")
    let src_file = subdir.join("index.ts");
    let relative_files: Vec<(&PathBuf, String)> =
        vec![(&src_file, "packages/app/index.ts".to_string())];

    let json_configs = discover_json_config_files(&matchers, &resolved, &relative_files, root);
    // Check if any nx project.json was discovered
    let found_project_json = json_configs
        .iter()
        .any(|(path, _)| path.ends_with("project.json"));
    assert!(
        found_project_json,
        "discover_json_config_files should find project.json in parent dir of discovered source file"
    );
}

// ── builtin::create_builtin_plugins ─────────────────────────

#[test]
fn create_builtin_plugins_returns_non_empty() {
    let plugins = builtin::create_builtin_plugins();
    assert!(
        !plugins.is_empty(),
        "create_builtin_plugins should return a non-empty list"
    );
}

#[test]
fn create_builtin_plugins_all_have_unique_names() {
    let plugins = builtin::create_builtin_plugins();
    let mut seen = FxHashSet::default();
    for plugin in &plugins {
        let name = plugin.name();
        assert!(seen.insert(name), "duplicate plugin name found: {name}");
    }
}

#[test]
fn create_builtin_plugins_contains_critical_plugins() {
    let plugins = builtin::create_builtin_plugins();
    let names: Vec<&str> = plugins.iter().map(|p| p.name()).collect();

    let critical = [
        "typescript",
        "eslint",
        "jest",
        "vitest",
        "webpack",
        "nextjs",
        "vite",
        "prettier",
        "tailwind",
        "storybook",
        "prisma",
        "babel",
    ];
    for expected in &critical {
        assert!(
            names.contains(expected),
            "critical plugin '{expected}' missing from builtin plugins"
        );
    }
}

#[test]
fn create_builtin_plugins_all_have_non_empty_names() {
    let plugins = builtin::create_builtin_plugins();
    for plugin in &plugins {
        assert!(
            !plugin.name().is_empty(),
            "all builtin plugins must have a non-empty name"
        );
    }
}

// ── process_static_patterns: minimal plugin ─────────────────

#[test]
fn process_static_patterns_with_minimal_plugin() {
    // MSW has entry_patterns, always_used, tooling_dependencies but no config_patterns
    let mut result = AggregatedPluginResult::default();
    let plugin: &dyn Plugin = &super::super::msw::MswPlugin;
    helpers::process_static_patterns(plugin, Path::new("/project"), &mut result);

    assert!(result.active_plugins.contains(&"msw".to_string()));
    assert!(!result.entry_patterns.is_empty());
    assert!(result.config_patterns.is_empty());
    assert!(!result.always_used.is_empty());
    assert!(!result.tooling_dependencies.is_empty());
}

#[test]
fn process_static_patterns_accumulates_across_plugins() {
    let mut result = AggregatedPluginResult::default();
    let next_plugin: &dyn Plugin = &super::super::nextjs::NextJsPlugin;
    let msw_plugin: &dyn Plugin = &super::super::msw::MswPlugin;

    helpers::process_static_patterns(next_plugin, Path::new("/project"), &mut result);
    let count_after_first = result.entry_patterns.len();

    helpers::process_static_patterns(msw_plugin, Path::new("/project"), &mut result);
    assert!(
        result.entry_patterns.len() > count_after_first,
        "second plugin should add more entry patterns"
    );
    assert_eq!(result.active_plugins.len(), 2);
    assert!(result.active_plugins.contains(&"nextjs".to_string()));
    assert!(result.active_plugins.contains(&"msw".to_string()));
}

// ── process_config_result: empty result ─────────────────────

#[test]
fn process_config_result_empty_result_is_noop() {
    let mut aggregated = AggregatedPluginResult::default();
    let empty = PluginResult::default();
    process_config_result("empty-plugin", empty, &mut aggregated);

    assert!(aggregated.entry_patterns.is_empty());
    assert!(aggregated.referenced_dependencies.is_empty());
    assert!(aggregated.discovered_always_used.is_empty());
    assert!(aggregated.setup_files.is_empty());
}

// ── check_plugin_detection: direct unit tests ───────────────

#[test]
fn check_plugin_detection_any_with_single_match() {
    let detection = PluginDetection::Any {
        conditions: vec![
            PluginDetection::Dependency {
                package: "missing-pkg".to_string(),
            },
            PluginDetection::Dependency {
                package: "present-pkg".to_string(),
            },
        ],
    };
    let deps = vec!["present-pkg"];
    assert!(
        check_plugin_detection(&detection, &deps, Path::new("/project"), &[]),
        "Any should succeed when at least one condition matches"
    );
}

#[test]
fn check_plugin_detection_all_with_all_matching() {
    let detection = PluginDetection::All {
        conditions: vec![
            PluginDetection::Dependency {
                package: "pkg-a".to_string(),
            },
            PluginDetection::Dependency {
                package: "pkg-b".to_string(),
            },
        ],
    };
    let deps = vec!["pkg-a", "pkg-b"];
    assert!(
        check_plugin_detection(&detection, &deps, Path::new("/project"), &[]),
        "All should succeed when every condition matches"
    );
}

#[test]
fn check_plugin_detection_all_with_partial_match() {
    let detection = PluginDetection::All {
        conditions: vec![
            PluginDetection::Dependency {
                package: "pkg-a".to_string(),
            },
            PluginDetection::Dependency {
                package: "pkg-b".to_string(),
            },
        ],
    };
    let deps = vec!["pkg-a"];
    assert!(
        !check_plugin_detection(&detection, &deps, Path::new("/project"), &[]),
        "All should fail when only some conditions match"
    );
}

#[test]
fn check_plugin_detection_any_with_no_matches() {
    let detection = PluginDetection::Any {
        conditions: vec![
            PluginDetection::Dependency {
                package: "missing-a".to_string(),
            },
            PluginDetection::Dependency {
                package: "missing-b".to_string(),
            },
        ],
    };
    let deps: Vec<&str> = vec!["unrelated"];
    assert!(
        !check_plugin_detection(&detection, &deps, Path::new("/project"), &[]),
        "Any should fail when no conditions match"
    );
}

#[test]
fn check_plugin_detection_nested_all_inside_any() {
    let detection = PluginDetection::Any {
        conditions: vec![
            PluginDetection::All {
                conditions: vec![
                    PluginDetection::Dependency {
                        package: "pkg-a".to_string(),
                    },
                    PluginDetection::Dependency {
                        package: "pkg-b".to_string(),
                    },
                ],
            },
            PluginDetection::Dependency {
                package: "pkg-c".to_string(),
            },
        ],
    };
    // Only pkg-c — the Any should succeed via the second branch
    let deps = vec!["pkg-c"];
    assert!(
        check_plugin_detection(&detection, &deps, Path::new("/project"), &[]),
        "nested All inside Any: should pass via the Any fallback branch"
    );
}

// ── process_external_plugins: detection via check_plugin_detection ──

#[test]
fn process_external_plugins_detection_dependency() {
    let ext = ExternalPluginDef {
        schema: None,
        name: "detect-dep".to_string(),
        detection: Some(PluginDetection::Dependency {
            package: "my-dep".to_string(),
        }),
        enablers: vec![],
        entry_points: vec!["src/**/*.ts".to_string()],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let mut result = AggregatedPluginResult::default();
    let deps = vec!["my-dep".to_string()];
    helpers::process_external_plugins(&[ext], &deps, Path::new("/project"), &[], &mut result);
    assert!(result.active_plugins.contains(&"detect-dep".to_string()));
    assert!(
        result
            .entry_patterns
            .iter()
            .any(|(p, _)| p == "src/**/*.ts")
    );
}

#[test]
fn process_external_plugins_detection_not_matched() {
    let ext = ExternalPluginDef {
        schema: None,
        name: "detect-miss".to_string(),
        detection: Some(PluginDetection::Dependency {
            package: "missing-dep".to_string(),
        }),
        enablers: vec![],
        entry_points: vec!["src/**/*.ts".to_string()],
        config_patterns: vec![],
        always_used: vec![],
        tooling_dependencies: vec![],
        used_exports: vec![],
    };
    let mut result = AggregatedPluginResult::default();
    let deps = vec!["other-dep".to_string()];
    helpers::process_external_plugins(&[ext], &deps, Path::new("/project"), &[], &mut result);
    assert!(!result.active_plugins.contains(&"detect-miss".to_string()));
    assert!(result.entry_patterns.is_empty());
}
