use fallow_config::{ResolvedConfig, RulesConfig, Severity};

// ── Rules helpers ────────────────────────────────────────────────

/// Remove issues whose effective severity is `Off` from the results.
///
/// When overrides are configured, per-file rule resolution is used for
/// file-scoped issue types. Non-file-scoped issues (unused deps, unlisted deps,
/// duplicate exports) use the base rules only.
pub fn apply_rules(results: &mut fallow_core::results::AnalysisResults, config: &ResolvedConfig) {
    let rules = &config.rules;
    let has_overrides = !config.overrides.is_empty();

    // File-scoped issue types: filter per-file when overrides exist
    if has_overrides {
        results
            .unused_files
            .retain(|f| config.resolve_rules_for_path(&f.path).unused_files != Severity::Off);
        results
            .unused_exports
            .retain(|e| config.resolve_rules_for_path(&e.path).unused_exports != Severity::Off);
        results
            .unused_types
            .retain(|e| config.resolve_rules_for_path(&e.path).unused_types != Severity::Off);
        results.unused_enum_members.retain(|m| {
            config.resolve_rules_for_path(&m.path).unused_enum_members != Severity::Off
        });
        results.unused_class_members.retain(|m| {
            config.resolve_rules_for_path(&m.path).unused_class_members != Severity::Off
        });
        results
            .unresolved_imports
            .retain(|i| config.resolve_rules_for_path(&i.path).unresolved_imports != Severity::Off);
    } else {
        if rules.unused_files == Severity::Off {
            results.unused_files.clear();
        }
        if rules.unused_exports == Severity::Off {
            results.unused_exports.clear();
        }
        if rules.unused_types == Severity::Off {
            results.unused_types.clear();
        }
        if rules.unused_enum_members == Severity::Off {
            results.unused_enum_members.clear();
        }
        if rules.unused_class_members == Severity::Off {
            results.unused_class_members.clear();
        }
        if rules.unresolved_imports == Severity::Off {
            results.unresolved_imports.clear();
        }
    }

    // Non-file-scoped issue types: always use base rules
    if rules.unused_dependencies == Severity::Off {
        results.unused_dependencies.clear();
    }
    if rules.unused_dev_dependencies == Severity::Off {
        results.unused_dev_dependencies.clear();
    }
    if rules.unused_optional_dependencies == Severity::Off {
        results.unused_optional_dependencies.clear();
    }
    if rules.unlisted_dependencies == Severity::Off {
        results.unlisted_dependencies.clear();
    }
    if rules.duplicate_exports == Severity::Off {
        results.duplicate_exports.clear();
    }
    if rules.circular_dependencies == Severity::Off {
        results.circular_dependencies.clear();
    }
}

/// Check whether any issue type with `Severity::Error` has remaining issues.
///
/// When overrides are configured, per-file rule resolution is used for
/// file-scoped issue types to determine if any individual issue has Error severity.
pub fn has_error_severity_issues(
    results: &fallow_core::results::AnalysisResults,
    rules: &RulesConfig,
    config: Option<&ResolvedConfig>,
) -> bool {
    let has_overrides = config.is_some_and(|c| !c.overrides.is_empty());

    // File-scoped issue types: check per-file when overrides exist
    let file_scoped_errors = if has_overrides {
        let config = config.unwrap();
        results
            .unused_files
            .iter()
            .any(|f| config.resolve_rules_for_path(&f.path).unused_files == Severity::Error)
            || results
                .unused_exports
                .iter()
                .any(|e| config.resolve_rules_for_path(&e.path).unused_exports == Severity::Error)
            || results
                .unused_types
                .iter()
                .any(|e| config.resolve_rules_for_path(&e.path).unused_types == Severity::Error)
            || results.unused_enum_members.iter().any(|m| {
                config.resolve_rules_for_path(&m.path).unused_enum_members == Severity::Error
            })
            || results.unused_class_members.iter().any(|m| {
                config.resolve_rules_for_path(&m.path).unused_class_members == Severity::Error
            })
            || results.unresolved_imports.iter().any(|i| {
                config.resolve_rules_for_path(&i.path).unresolved_imports == Severity::Error
            })
    } else {
        (rules.unused_files == Severity::Error && !results.unused_files.is_empty())
            || (rules.unused_exports == Severity::Error && !results.unused_exports.is_empty())
            || (rules.unused_types == Severity::Error && !results.unused_types.is_empty())
            || (rules.unused_enum_members == Severity::Error
                && !results.unused_enum_members.is_empty())
            || (rules.unused_class_members == Severity::Error
                && !results.unused_class_members.is_empty())
            || (rules.unresolved_imports == Severity::Error
                && !results.unresolved_imports.is_empty())
    };

    // Non-file-scoped issue types: always use base rules
    file_scoped_errors
        || (rules.unused_dependencies == Severity::Error && !results.unused_dependencies.is_empty())
        || (rules.unused_dev_dependencies == Severity::Error
            && !results.unused_dev_dependencies.is_empty())
        || (rules.unused_optional_dependencies == Severity::Error
            && !results.unused_optional_dependencies.is_empty())
        || (rules.unlisted_dependencies == Severity::Error
            && !results.unlisted_dependencies.is_empty())
        || (rules.duplicate_exports == Severity::Error && !results.duplicate_exports.is_empty())
        || (rules.circular_dependencies == Severity::Error
            && !results.circular_dependencies.is_empty())
}

/// Promote all `Warn` severities to `Error` for a single run.
pub fn promote_warns_to_errors(rules: &mut RulesConfig) {
    if rules.unused_files == Severity::Warn {
        rules.unused_files = Severity::Error;
    }
    if rules.unused_exports == Severity::Warn {
        rules.unused_exports = Severity::Error;
    }
    if rules.unused_types == Severity::Warn {
        rules.unused_types = Severity::Error;
    }
    if rules.unused_dependencies == Severity::Warn {
        rules.unused_dependencies = Severity::Error;
    }
    if rules.unused_dev_dependencies == Severity::Warn {
        rules.unused_dev_dependencies = Severity::Error;
    }
    if rules.unused_optional_dependencies == Severity::Warn {
        rules.unused_optional_dependencies = Severity::Error;
    }
    if rules.unused_enum_members == Severity::Warn {
        rules.unused_enum_members = Severity::Error;
    }
    if rules.unused_class_members == Severity::Warn {
        rules.unused_class_members = Severity::Error;
    }
    if rules.unresolved_imports == Severity::Warn {
        rules.unresolved_imports = Severity::Error;
    }
    if rules.unlisted_dependencies == Severity::Warn {
        rules.unlisted_dependencies = Severity::Error;
    }
    if rules.duplicate_exports == Severity::Warn {
        rules.duplicate_exports = Severity::Error;
    }
    if rules.circular_dependencies == Severity::Warn {
        rules.circular_dependencies = Severity::Error;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fallow_core::extract::MemberKind;
    use fallow_core::results::*;
    use std::path::PathBuf;

    // ── Helper: build populated AnalysisResults ──────────────────

    fn make_results() -> AnalysisResults {
        let mut r = AnalysisResults::default();
        r.unused_files.push(UnusedFile {
            path: PathBuf::from("/project/src/a.ts"),
        });
        r.unused_exports.push(UnusedExport {
            path: PathBuf::from("/project/src/b.ts"),
            export_name: "foo".into(),
            is_type_only: false,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        r.unused_types.push(UnusedExport {
            path: PathBuf::from("/project/src/c.ts"),
            export_name: "MyType".into(),
            is_type_only: true,
            line: 5,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        r.unused_dependencies.push(UnusedDependency {
            package_name: "lodash".into(),
            location: DependencyLocation::Dependencies,
            path: PathBuf::from("/project/package.json"),
        });
        r.unused_dev_dependencies.push(UnusedDependency {
            package_name: "jest".into(),
            location: DependencyLocation::DevDependencies,
            path: PathBuf::from("/project/package.json"),
        });
        r.unused_enum_members.push(UnusedMember {
            path: PathBuf::from("/project/src/d.ts"),
            parent_name: "Status".into(),
            member_name: "Pending".into(),
            kind: MemberKind::EnumMember,
            line: 3,
            col: 0,
        });
        r.unused_class_members.push(UnusedMember {
            path: PathBuf::from("/project/src/e.ts"),
            parent_name: "Service".into(),
            member_name: "helper".into(),
            kind: MemberKind::ClassMethod,
            line: 10,
            col: 0,
        });
        r.unresolved_imports.push(UnresolvedImport {
            path: PathBuf::from("/project/src/f.ts"),
            specifier: "./missing".into(),
            line: 1,
            col: 0,
        });
        r.unlisted_dependencies.push(UnlistedDependency {
            package_name: "chalk".into(),
            imported_from: vec![PathBuf::from("/project/src/g.ts")],
        });
        r.duplicate_exports.push(DuplicateExport {
            export_name: "helper".into(),
            locations: vec![
                PathBuf::from("/project/src/h.ts"),
                PathBuf::from("/project/src/i.ts"),
            ],
        });
        r
    }

    /// Build a minimal ResolvedConfig from a RulesConfig for testing.
    fn config_with_rules(rules: RulesConfig) -> ResolvedConfig {
        fallow_config::FallowConfig {
            schema: None,
            extends: vec![],
            entry: vec![],
            ignore_patterns: vec![],
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            duplicates: fallow_config::DuplicatesConfig::default(),
            rules,
            production: false,
            plugins: vec![],
            overrides: vec![],
        }
        .resolve(
            PathBuf::from("/project"),
            fallow_config::OutputFormat::Human,
            1,
            true,
            true,
        )
    }

    // ── apply_rules ──────────────────────────────────────────────

    #[test]
    fn apply_rules_default_error_preserves_all() {
        let mut results = make_results();
        let config = config_with_rules(RulesConfig::default());
        let original_total = results.total_issues();
        apply_rules(&mut results, &config);
        assert_eq!(results.total_issues(), original_total);
    }

    #[test]
    fn apply_rules_off_clears_that_issue_type() {
        let mut results = make_results();
        let mut rules = RulesConfig::default();
        rules.unused_files = Severity::Off;
        let config = config_with_rules(rules);
        apply_rules(&mut results, &config);
        assert!(results.unused_files.is_empty());
        // Other types are preserved
        assert!(!results.unused_exports.is_empty());
    }

    #[test]
    fn apply_rules_warn_preserves_issues() {
        let mut results = make_results();
        let mut rules = RulesConfig::default();
        rules.unused_exports = Severity::Warn;
        let config = config_with_rules(rules);
        apply_rules(&mut results, &config);
        assert_eq!(results.unused_exports.len(), 1);
    }

    #[test]
    fn apply_rules_all_off_clears_everything() {
        let mut results = make_results();
        let rules = RulesConfig {
            unused_files: Severity::Off,
            unused_exports: Severity::Off,
            unused_types: Severity::Off,
            unused_dependencies: Severity::Off,
            unused_dev_dependencies: Severity::Off,
            unused_optional_dependencies: Severity::Off,
            unused_enum_members: Severity::Off,
            unused_class_members: Severity::Off,
            unresolved_imports: Severity::Off,
            unlisted_dependencies: Severity::Off,
            duplicate_exports: Severity::Off,
            circular_dependencies: Severity::Off,
        };
        let config = config_with_rules(rules);
        apply_rules(&mut results, &config);
        assert_eq!(results.total_issues(), 0);
    }

    #[test]
    fn apply_rules_off_each_type_individually() {
        // Verify every rule field maps to its corresponding results field
        let field_setters: Vec<(fn(&mut RulesConfig), fn(&AnalysisResults) -> bool)> = vec![
            (
                |r| r.unused_files = Severity::Off,
                |res| res.unused_files.is_empty(),
            ),
            (
                |r| r.unused_exports = Severity::Off,
                |res| res.unused_exports.is_empty(),
            ),
            (
                |r| r.unused_types = Severity::Off,
                |res| res.unused_types.is_empty(),
            ),
            (
                |r| r.unused_dependencies = Severity::Off,
                |res| res.unused_dependencies.is_empty(),
            ),
            (
                |r| r.unused_dev_dependencies = Severity::Off,
                |res| res.unused_dev_dependencies.is_empty(),
            ),
            (
                |r| r.unused_enum_members = Severity::Off,
                |res| res.unused_enum_members.is_empty(),
            ),
            (
                |r| r.unused_class_members = Severity::Off,
                |res| res.unused_class_members.is_empty(),
            ),
            (
                |r| r.unresolved_imports = Severity::Off,
                |res| res.unresolved_imports.is_empty(),
            ),
            (
                |r| r.unlisted_dependencies = Severity::Off,
                |res| res.unlisted_dependencies.is_empty(),
            ),
            (
                |r| r.duplicate_exports = Severity::Off,
                |res| res.duplicate_exports.is_empty(),
            ),
        ];

        for (set_off, check_empty) in field_setters {
            let mut results = make_results();
            let mut rules = RulesConfig::default();
            set_off(&mut rules);
            let config = config_with_rules(rules);
            apply_rules(&mut results, &config);
            assert!(
                check_empty(&results),
                "Setting a rule to Off should clear the corresponding results"
            );
        }
    }

    // ── has_error_severity_issues ────────────────────────────────

    #[test]
    fn empty_results_no_error_issues() {
        let results = AnalysisResults::default();
        let rules = RulesConfig::default();
        assert!(!has_error_severity_issues(&results, &rules, None));
    }

    #[test]
    fn error_severity_with_issues_returns_true() {
        let results = make_results();
        let rules = RulesConfig::default(); // all Error
        assert!(has_error_severity_issues(&results, &rules, None));
    }

    #[test]
    fn warn_severity_with_issues_returns_false() {
        let results = make_results();
        let rules = RulesConfig {
            unused_files: Severity::Warn,
            unused_exports: Severity::Warn,
            unused_types: Severity::Warn,
            unused_dependencies: Severity::Warn,
            unused_dev_dependencies: Severity::Warn,
            unused_optional_dependencies: Severity::Warn,
            unused_enum_members: Severity::Warn,
            unused_class_members: Severity::Warn,
            unresolved_imports: Severity::Warn,
            unlisted_dependencies: Severity::Warn,
            duplicate_exports: Severity::Warn,
            circular_dependencies: Severity::Warn,
        };
        assert!(!has_error_severity_issues(&results, &rules, None));
    }

    #[test]
    fn mixed_severity_returns_true_for_error_with_issues() {
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: PathBuf::from("/project/src/a.ts"),
        });
        let mut rules = RulesConfig {
            unused_files: Severity::Warn,
            unused_exports: Severity::Warn,
            unused_types: Severity::Warn,
            unused_dependencies: Severity::Warn,
            unused_dev_dependencies: Severity::Warn,
            unused_optional_dependencies: Severity::Warn,
            unused_enum_members: Severity::Warn,
            unused_class_members: Severity::Warn,
            unresolved_imports: Severity::Warn,
            unlisted_dependencies: Severity::Warn,
            duplicate_exports: Severity::Warn,
            circular_dependencies: Severity::Warn,
        };
        // Only unused_files present, but set to Warn — should not trigger
        assert!(!has_error_severity_issues(&results, &rules, None));

        // Promote unused_files to Error — should now trigger
        rules.unused_files = Severity::Error;
        assert!(has_error_severity_issues(&results, &rules, None));
    }

    #[test]
    fn off_severity_with_issues_returns_false() {
        let mut results = AnalysisResults::default();
        results.unresolved_imports.push(UnresolvedImport {
            path: PathBuf::from("/project/src/a.ts"),
            specifier: "./missing".into(),
            line: 1,
            col: 0,
        });
        let mut rules = RulesConfig::default();
        rules.unresolved_imports = Severity::Off;
        // Other fields are default (Error) but have no issues
        assert!(!has_error_severity_issues(&results, &rules, None));
    }
}
