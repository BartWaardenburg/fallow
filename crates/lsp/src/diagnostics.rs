use rustc_hash::FxHashMap;
use std::path::Path;

use tower_lsp::lsp_types::*;

use fallow_core::duplicates::DuplicationReport;
use fallow_core::results::AnalysisResults;

/// Base URL for diagnostic documentation links.
const DOCS_BASE: &str = "https://docs.fallow.tools/explanations/dead-code#";

/// Build a `CodeDescription` with a documentation URL for the given anchor.
fn doc_link(anchor: &str) -> Option<CodeDescription> {
    let url = format!("{DOCS_BASE}{anchor}");
    Url::parse(&url).ok().map(|href| CodeDescription { href })
}

/// LSP range covering the entire first line — used for file-level and package.json diagnostics.
pub const FIRST_LINE_RANGE: Range = Range {
    start: Position {
        line: 0,
        character: 0,
    },
    end: Position {
        line: 0,
        character: u32::MAX,
    },
};

/// Build all LSP diagnostics from analysis results and duplication report, keyed by file URI.
pub fn build_diagnostics(
    results: &AnalysisResults,
    duplication: &DuplicationReport,
    root: &Path,
) -> FxHashMap<Url, Vec<Diagnostic>> {
    let mut diagnostics_by_file: FxHashMap<Url, Vec<Diagnostic>> = FxHashMap::default();

    // Helper: get the package.json URI for dependency-related diagnostics
    let package_json_path = root.join("package.json");
    let package_json_uri = Url::from_file_path(&package_json_path).ok();

    // Export-like issues: unused exports and unused types
    for (exports, code, anchor, msg_prefix) in [
        (
            &results.unused_exports,
            "unused-export",
            "unused-exports",
            "Export" as &str,
        ),
        (
            &results.unused_types,
            "unused-type",
            "unused-types",
            "Type export",
        ),
    ] {
        for export in exports {
            if let Ok(uri) = Url::from_file_path(&export.path) {
                let line = export.line.saturating_sub(1);
                diagnostics_by_file
                    .entry(uri)
                    .or_default()
                    .push(Diagnostic {
                        range: Range {
                            start: Position {
                                line,
                                character: export.col,
                            },
                            end: Position {
                                line,
                                character: export.col + export.export_name.len() as u32,
                            },
                        },
                        severity: Some(DiagnosticSeverity::HINT),
                        source: Some("fallow".to_string()),
                        code: Some(NumberOrString::String(code.to_string())),
                        code_description: doc_link(anchor),
                        message: format!("{msg_prefix} '{}' is unused", export.export_name),
                        tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                        ..Default::default()
                    });
            }
        }
    }

    // Unused files: file-level diagnostic on first line
    for file in &results.unused_files {
        if let Ok(uri) = Url::from_file_path(&file.path) {
            diagnostics_by_file
                .entry(uri)
                .or_default()
                .push(Diagnostic {
                    range: FIRST_LINE_RANGE,
                    severity: Some(DiagnosticSeverity::WARNING),
                    source: Some("fallow".to_string()),
                    code: Some(NumberOrString::String("unused-file".to_string())),
                    code_description: doc_link("unused-files"),
                    message: "File is not reachable from any entry point".to_string(),
                    tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                    ..Default::default()
                });
        }
    }

    // Unresolved imports
    for import in &results.unresolved_imports {
        if let Ok(uri) = Url::from_file_path(&import.path) {
            let line = import.line.saturating_sub(1);
            diagnostics_by_file
                .entry(uri)
                .or_default()
                .push(Diagnostic {
                    range: Range {
                        start: Position {
                            line,
                            character: import.specifier_col,
                        },
                        end: Position {
                            line,
                            // +2 accounts for the surrounding quotes on the string literal
                            character: import.specifier_col + import.specifier.len() as u32 + 2,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("fallow".to_string()),
                    code: Some(NumberOrString::String("unresolved-import".to_string())),
                    code_description: doc_link("unresolved-imports"),
                    message: format!("Cannot find module '{}'", import.specifier),
                    ..Default::default()
                });
        }
    }

    // Dependency issues: unused deps, unused dev deps (routed to their respective package.json)
    // Place diagnostic on the line where the dependency is declared.
    for dep in &results.unused_dependencies {
        if let Ok(dep_uri) = Url::from_file_path(&dep.path) {
            let line = dep.line.saturating_sub(1);
            let entry = diagnostics_by_file.entry(dep_uri).or_default();
            entry.push(Diagnostic {
                range: Range {
                    start: Position { line, character: 0 },
                    end: Position {
                        line,
                        character: u32::MAX,
                    },
                },
                severity: Some(DiagnosticSeverity::WARNING),
                source: Some("fallow".to_string()),
                code: Some(NumberOrString::String("unused-dependency".to_string())),
                code_description: doc_link("unused-dependencies"),
                message: format!("Unused dependency: {}", dep.package_name),
                ..Default::default()
            });
        }
    }
    for dep in &results.unused_dev_dependencies {
        if let Ok(dep_uri) = Url::from_file_path(&dep.path) {
            let line = dep.line.saturating_sub(1);
            let entry = diagnostics_by_file.entry(dep_uri).or_default();
            entry.push(Diagnostic {
                range: Range {
                    start: Position { line, character: 0 },
                    end: Position {
                        line,
                        character: u32::MAX,
                    },
                },
                severity: Some(DiagnosticSeverity::WARNING),
                source: Some("fallow".to_string()),
                code: Some(NumberOrString::String("unused-dev-dependency".to_string())),
                code_description: doc_link("unused-devdependencies"),
                message: format!("Unused devDependency: {}", dep.package_name),
                ..Default::default()
            });
        }
    }
    for dep in &results.unused_optional_dependencies {
        if let Ok(dep_uri) = Url::from_file_path(&dep.path) {
            let line = dep.line.saturating_sub(1);
            let entry = diagnostics_by_file.entry(dep_uri).or_default();
            entry.push(Diagnostic {
                range: Range {
                    start: Position { line, character: 0 },
                    end: Position {
                        line,
                        character: u32::MAX,
                    },
                },
                severity: Some(DiagnosticSeverity::WARNING),
                source: Some("fallow".to_string()),
                code: Some(NumberOrString::String(
                    "unused-optional-dependency".to_string(),
                )),
                code_description: doc_link("unused-optionaldependencies"),
                message: format!("Unused optionalDependency: {}", dep.package_name),
                ..Default::default()
            });
        }
    }
    // Unlisted deps still use root package.json
    if let Some(ref uri) = package_json_uri {
        for dep in &results.unlisted_dependencies {
            let entry = diagnostics_by_file.entry(uri.clone()).or_default();
            entry.push(Diagnostic {
                range: FIRST_LINE_RANGE,
                severity: Some(DiagnosticSeverity::WARNING),
                source: Some("fallow".to_string()),
                code: Some(NumberOrString::String("unlisted-dependency".to_string())),
                code_description: doc_link("unlisted-dependencies"),
                message: format!(
                    "Unlisted dependency: {} (used but not in package.json)",
                    dep.package_name
                ),
                ..Default::default()
            });
        }
    }

    // Member issues: unused enum members and unused class members
    for (members, code, anchor, kind_label) in [
        (
            &results.unused_enum_members,
            "unused-enum-member",
            "unused-enum-members",
            "Enum member" as &str,
        ),
        (
            &results.unused_class_members,
            "unused-class-member",
            "unused-class-members",
            "Class member",
        ),
    ] {
        for member in members {
            if let Ok(uri) = Url::from_file_path(&member.path) {
                let line = member.line.saturating_sub(1);
                diagnostics_by_file
                    .entry(uri)
                    .or_default()
                    .push(Diagnostic {
                        range: Range {
                            start: Position {
                                line,
                                character: member.col,
                            },
                            end: Position {
                                line,
                                character: member.col + member.member_name.len() as u32,
                            },
                        },
                        severity: Some(DiagnosticSeverity::HINT),
                        source: Some("fallow".to_string()),
                        code: Some(NumberOrString::String(code.to_string())),
                        code_description: doc_link(anchor),
                        message: format!(
                            "{kind_label} '{}.{}' is unused",
                            member.parent_name, member.member_name
                        ),
                        tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                        ..Default::default()
                    });
            }
        }
    }

    // Duplicate exports: WARNING on each file that has the duplicate
    for dup in &results.duplicate_exports {
        // Build related information linking all duplicate locations together
        for loc in &dup.locations {
            if let Ok(uri) = Url::from_file_path(&loc.path) {
                let related_info: Vec<DiagnosticRelatedInformation> = dup
                    .locations
                    .iter()
                    .filter(|l| l.path != loc.path)
                    .filter_map(|l| {
                        let other_uri = Url::from_file_path(&l.path).ok()?;
                        Some(DiagnosticRelatedInformation {
                            location: Location {
                                uri: other_uri,
                                range: Range {
                                    start: Position {
                                        line: l.line.saturating_sub(1),
                                        character: l.col,
                                    },
                                    end: Position {
                                        line: l.line.saturating_sub(1),
                                        character: l.col + dup.export_name.len() as u32,
                                    },
                                },
                            },
                            message: "Also exported here".to_string(),
                        })
                    })
                    .collect();
                let line = loc.line.saturating_sub(1);
                diagnostics_by_file
                    .entry(uri)
                    .or_default()
                    .push(Diagnostic {
                        range: Range {
                            start: Position {
                                line,
                                character: loc.col,
                            },
                            end: Position {
                                line,
                                character: loc.col + dup.export_name.len() as u32,
                            },
                        },
                        severity: Some(DiagnosticSeverity::WARNING),
                        source: Some("fallow".to_string()),
                        code: Some(NumberOrString::String("duplicate-export".to_string())),
                        code_description: doc_link("duplicate-exports"),
                        message: format!("Duplicate export '{}'", dup.export_name,),
                        related_information: if related_info.is_empty() {
                            None
                        } else {
                            Some(related_info)
                        },
                        ..Default::default()
                    });
            }
        }
    }

    // Code duplication diagnostics
    for group in &duplication.clone_groups {
        for instance in &group.instances {
            let Ok(inst_uri) = Url::from_file_path(&instance.file) else {
                continue;
            };

            let start_line = (instance.start_line as u32).saturating_sub(1);
            let end_line = (instance.end_line as u32).saturating_sub(1);

            // Build related information pointing to other instances in the group
            let related_info: Vec<DiagnosticRelatedInformation> = group
                .instances
                .iter()
                .filter(|other| {
                    !(other.file == instance.file && other.start_line == instance.start_line)
                })
                .filter_map(|other| {
                    let other_uri = Url::from_file_path(&other.file).ok()?;
                    Some(DiagnosticRelatedInformation {
                        location: Location {
                            uri: other_uri,
                            range: Range {
                                start: Position {
                                    line: (other.start_line as u32).saturating_sub(1),
                                    character: other.start_col as u32,
                                },
                                end: Position {
                                    line: (other.end_line as u32).saturating_sub(1),
                                    character: u32::MAX,
                                },
                            },
                        },
                        message: "Also duplicated here".to_string(),
                    })
                })
                .collect();

            diagnostics_by_file
                .entry(inst_uri)
                .or_default()
                .push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: start_line,
                            character: instance.start_col as u32,
                        },
                        end: Position {
                            line: end_line,
                            // Extend to end of last line to ensure full block is underlined
                            character: u32::MAX,
                        },
                    },
                    severity: Some(DiagnosticSeverity::INFORMATION),
                    source: Some("fallow".to_string()),
                    code: Some(NumberOrString::String("code-duplication".to_string())),
                    code_description: Url::parse(
                        "https://docs.fallow.tools/explanations/duplication",
                    )
                    .ok()
                    .map(|href| CodeDescription { href }),
                    message: format!(
                        "Duplicated code block ({} lines, {} instances)",
                        group.line_count,
                        group.instances.len()
                    ),
                    related_information: if related_info.is_empty() {
                        None
                    } else {
                        Some(related_info)
                    },
                    ..Default::default()
                });
        }
    }

    // Circular dependency diagnostics: one diagnostic per cycle on the first file
    for cycle in &results.circular_dependencies {
        if let Some(first_file) = cycle.files.first()
            && let Ok(uri) = Url::from_file_path(first_file)
        {
            let chain: Vec<String> = cycle
                .files
                .iter()
                .map(|f| {
                    f.file_name().map_or_else(
                        || f.display().to_string(),
                        |n| n.to_string_lossy().into_owned(),
                    )
                })
                .collect();
            let message = format!("Circular dependency: {}", chain.join(" \u{2192} "));
            let line = cycle.line.saturating_sub(1);

            // Related info: link to each file in the cycle chain
            let related_info: Vec<DiagnosticRelatedInformation> = cycle
                .files
                .iter()
                .skip(1) // skip the first file (it's the diagnostic location)
                .enumerate()
                .filter_map(|(i, f)| {
                    let file_uri = Url::from_file_path(f).ok()?;
                    let name = f.file_name().map_or_else(
                        || f.display().to_string(),
                        |n| n.to_string_lossy().into_owned(),
                    );
                    Some(DiagnosticRelatedInformation {
                        location: Location {
                            uri: file_uri,
                            range: FIRST_LINE_RANGE,
                        },
                        message: format!("Step {} in cycle: {name}", i + 2),
                    })
                })
                .collect();

            diagnostics_by_file
                .entry(uri)
                .or_default()
                .push(Diagnostic {
                    range: Range {
                        start: Position {
                            line,
                            character: cycle.col,
                        },
                        end: Position {
                            line,
                            character: u32::MAX,
                        },
                    },
                    severity: Some(DiagnosticSeverity::WARNING),
                    source: Some("fallow".to_string()),
                    code: Some(NumberOrString::String("circular-dependency".to_string())),
                    code_description: doc_link("circular-dependencies"),
                    message,
                    related_information: if related_info.is_empty() {
                        None
                    } else {
                        Some(related_info)
                    },
                    ..Default::default()
                });
        }
    }

    // Type-only dependencies: could be moved to devDependencies
    for dep in &results.type_only_dependencies {
        if let Ok(dep_uri) = Url::from_file_path(&dep.path) {
            let line = dep.line.saturating_sub(1);
            let entry = diagnostics_by_file.entry(dep_uri).or_default();
            entry.push(Diagnostic {
                range: Range {
                    start: Position { line, character: 0 },
                    end: Position {
                        line,
                        character: u32::MAX,
                    },
                },
                severity: Some(DiagnosticSeverity::INFORMATION),
                source: Some("fallow".to_string()),
                code: Some(NumberOrString::String("type-only-dependency".to_string())),
                code_description: doc_link("type-only-dependencies"),
                message: format!(
                    "Type-only dependency: {} (only used via type imports, could be a devDependency)",
                    dep.package_name
                ),
                ..Default::default()
            });
        }
    }

    diagnostics_by_file
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use fallow_core::duplicates::{CloneGroup, CloneInstance, DuplicationStats};
    use fallow_core::extract::MemberKind;
    use fallow_core::results::{
        CircularDependency, DependencyLocation, DuplicateExport, DuplicateLocation, ImportSite,
        TypeOnlyDependency, UnlistedDependency, UnresolvedImport, UnusedDependency, UnusedExport,
        UnusedFile, UnusedMember,
    };

    fn test_root() -> PathBuf {
        if cfg!(windows) {
            PathBuf::from("C:\\project")
        } else {
            PathBuf::from("/project")
        }
    }

    fn empty_duplication() -> DuplicationReport {
        DuplicationReport {
            clone_groups: vec![],
            clone_families: vec![],
            stats: DuplicationStats {
                total_files: 0,
                files_with_clones: 0,
                total_lines: 0,
                duplicated_lines: 0,
                total_tokens: 0,
                duplicated_tokens: 0,
                clone_groups: 0,
                clone_instances: 0,
                duplication_percentage: 0.0,
            },
        }
    }

    #[test]
    fn empty_results_produce_no_diagnostics() {
        let results = AnalysisResults::default();
        let duplication = empty_duplication();
        let root = test_root();

        let diags = build_diagnostics(&results, &duplication, &root);
        assert!(diags.is_empty());
    }

    #[test]
    fn unused_export_produces_hint_diagnostic() {
        let root = test_root();
        let mut results = AnalysisResults::default();
        results.unused_exports.push(UnusedExport {
            path: root.join("src/utils.ts"),
            export_name: "helper".to_string(),
            is_type_only: false,
            line: 5,
            col: 7,
            span_start: 40,
            is_re_export: false,
        });

        let duplication = empty_duplication();
        let diags = build_diagnostics(&results, &duplication, &root);

        let uri = Url::from_file_path(root.join("src/utils.ts")).unwrap();
        let file_diags = diags.get(&uri).expect("should have diagnostics for file");
        assert_eq!(file_diags.len(), 1);

        let d = &file_diags[0];
        assert_eq!(d.severity, Some(DiagnosticSeverity::HINT));
        assert_eq!(d.message, "Export 'helper' is unused");
        assert_eq!(
            d.code,
            Some(NumberOrString::String("unused-export".to_string()))
        );
        assert_eq!(d.source, Some("fallow".to_string()));
        // Line is 1-based in results, 0-based in LSP
        assert_eq!(d.range.start.line, 4);
        assert_eq!(d.range.start.character, 7);
        // End character = col + export_name.len()
        assert_eq!(d.range.end.character, 7 + "helper".len() as u32);
        assert_eq!(d.tags, Some(vec![DiagnosticTag::UNNECESSARY]));
    }

    #[test]
    fn unused_type_produces_hint_diagnostic() {
        let root = test_root();
        let mut results = AnalysisResults::default();
        results.unused_types.push(UnusedExport {
            path: root.join("src/types.ts"),
            export_name: "MyType".to_string(),
            is_type_only: true,
            line: 10,
            col: 0,
            span_start: 100,
            is_re_export: false,
        });

        let duplication = empty_duplication();
        let diags = build_diagnostics(&results, &duplication, &root);

        let uri = Url::from_file_path(root.join("src/types.ts")).unwrap();
        let file_diags = &diags[&uri];
        assert_eq!(file_diags.len(), 1);

        let d = &file_diags[0];
        assert_eq!(d.severity, Some(DiagnosticSeverity::HINT));
        assert_eq!(d.message, "Type export 'MyType' is unused");
        assert_eq!(
            d.code,
            Some(NumberOrString::String("unused-type".to_string()))
        );
    }

    #[test]
    fn unused_file_produces_warning_at_zero_range() {
        let root = test_root();
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/dead.ts"),
        });

        let duplication = empty_duplication();
        let diags = build_diagnostics(&results, &duplication, &root);

        let uri = Url::from_file_path(root.join("src/dead.ts")).unwrap();
        let file_diags = &diags[&uri];
        assert_eq!(file_diags.len(), 1);

        let d = &file_diags[0];
        assert_eq!(d.severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(d.range, FIRST_LINE_RANGE);
        assert_eq!(d.message, "File is not reachable from any entry point");
        assert_eq!(
            d.code,
            Some(NumberOrString::String("unused-file".to_string()))
        );
    }

    #[test]
    fn unresolved_import_produces_error_diagnostic() {
        let root = test_root();
        let mut results = AnalysisResults::default();
        // import { foo } from './missing-module'
        //                     ^--- specifier_col = 20 (quote position)
        results.unresolved_imports.push(UnresolvedImport {
            path: root.join("src/app.ts"),
            specifier: "./missing-module".to_string(),
            line: 3,
            col: 0,
            specifier_col: 20,
        });

        let duplication = empty_duplication();
        let diags = build_diagnostics(&results, &duplication, &root);

        let uri = Url::from_file_path(root.join("src/app.ts")).unwrap();
        let file_diags = &diags[&uri];
        assert_eq!(file_diags.len(), 1);

        let d = &file_diags[0];
        assert_eq!(d.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(d.message, "Cannot find module './missing-module'");
        assert_eq!(d.range.start.line, 2); // 1-based -> 0-based
        // Range covers the specifier string literal including quotes
        assert_eq!(d.range.start.character, 20);
        assert_eq!(
            d.range.end.character,
            20 + "./missing-module".len() as u32 + 2
        );
    }

    #[test]
    fn unused_dependency_produces_warning_at_package_json() {
        let root = test_root();
        let mut results = AnalysisResults::default();
        results.unused_dependencies.push(UnusedDependency {
            package_name: "lodash".to_string(),
            location: DependencyLocation::Dependencies,
            path: root.join("package.json"),
            line: 5,
        });

        let duplication = empty_duplication();
        let diags = build_diagnostics(&results, &duplication, &root);

        let uri = Url::from_file_path(root.join("package.json")).unwrap();
        let file_diags = &diags[&uri];
        assert_eq!(file_diags.len(), 1);

        let d = &file_diags[0];
        assert_eq!(d.severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(d.message, "Unused dependency: lodash");
        assert_eq!(d.range.start.line, 4); // 1-based line 5 → 0-based line 4
    }

    #[test]
    fn unused_dev_dependency_produces_warning() {
        let root = test_root();
        let mut results = AnalysisResults::default();
        results.unused_dev_dependencies.push(UnusedDependency {
            package_name: "prettier".to_string(),
            location: DependencyLocation::DevDependencies,
            path: root.join("package.json"),
            line: 5,
        });

        let duplication = empty_duplication();
        let diags = build_diagnostics(&results, &duplication, &root);

        let uri = Url::from_file_path(root.join("package.json")).unwrap();
        let file_diags = &diags[&uri];
        assert_eq!(file_diags.len(), 1);

        let d = &file_diags[0];
        assert_eq!(d.severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(d.message, "Unused devDependency: prettier");
    }

    #[test]
    fn unlisted_dependency_uses_root_package_json() {
        let root = test_root();
        let mut results = AnalysisResults::default();
        results.unlisted_dependencies.push(UnlistedDependency {
            package_name: "chalk".to_string(),
            imported_from: vec![ImportSite {
                path: root.join("src/cli.ts"),
                line: 2,
                col: 0,
            }],
        });

        let duplication = empty_duplication();
        let diags = build_diagnostics(&results, &duplication, &root);

        let uri = Url::from_file_path(root.join("package.json")).unwrap();
        let file_diags = &diags[&uri];
        assert_eq!(file_diags.len(), 1);

        let d = &file_diags[0];
        assert_eq!(d.severity, Some(DiagnosticSeverity::WARNING));
        assert!(d.message.contains("chalk"));
        assert!(d.message.contains("Unlisted dependency"));
    }

    #[test]
    fn unused_enum_member_produces_hint() {
        let root = test_root();
        let mut results = AnalysisResults::default();
        results.unused_enum_members.push(UnusedMember {
            path: root.join("src/enums.ts"),
            parent_name: "Color".to_string(),
            member_name: "Blue".to_string(),
            kind: MemberKind::EnumMember,
            line: 4,
            col: 2,
        });

        let duplication = empty_duplication();
        let diags = build_diagnostics(&results, &duplication, &root);

        let uri = Url::from_file_path(root.join("src/enums.ts")).unwrap();
        let file_diags = &diags[&uri];
        assert_eq!(file_diags.len(), 1);

        let d = &file_diags[0];
        assert_eq!(d.severity, Some(DiagnosticSeverity::HINT));
        assert_eq!(d.message, "Enum member 'Color.Blue' is unused");
        assert_eq!(
            d.code,
            Some(NumberOrString::String("unused-enum-member".to_string()))
        );
        assert_eq!(d.range.start.line, 3);
        assert_eq!(d.range.start.character, 2);
        assert_eq!(d.range.end.character, 2 + "Blue".len() as u32);
    }

    #[test]
    fn unused_class_member_produces_hint() {
        let root = test_root();
        let mut results = AnalysisResults::default();
        results.unused_class_members.push(UnusedMember {
            path: root.join("src/service.ts"),
            parent_name: "UserService".to_string(),
            member_name: "reset".to_string(),
            kind: MemberKind::ClassMethod,
            line: 20,
            col: 4,
        });

        let duplication = empty_duplication();
        let diags = build_diagnostics(&results, &duplication, &root);

        let uri = Url::from_file_path(root.join("src/service.ts")).unwrap();
        let file_diags = &diags[&uri];
        assert_eq!(file_diags.len(), 1);

        let d = &file_diags[0];
        assert_eq!(d.severity, Some(DiagnosticSeverity::HINT));
        assert_eq!(d.message, "Class member 'UserService.reset' is unused");
        assert_eq!(
            d.code,
            Some(NumberOrString::String("unused-class-member".to_string()))
        );
    }

    #[test]
    fn duplicate_export_produces_warning_with_related_files() {
        let root = test_root();
        let utils_path = root.join("src/utils.ts");
        let helpers_path = root.join("src/helpers.ts");

        let mut results = AnalysisResults::default();
        results.duplicate_exports.push(DuplicateExport {
            export_name: "formatDate".to_string(),
            locations: vec![
                DuplicateLocation {
                    path: utils_path.clone(),
                    line: 15,
                    col: 0,
                },
                DuplicateLocation {
                    path: helpers_path.clone(),
                    line: 30,
                    col: 0,
                },
            ],
        });

        let duplication = empty_duplication();
        let diags = build_diagnostics(&results, &duplication, &root);

        // Both files should have a diagnostic
        let uri_utils = Url::from_file_path(&utils_path).unwrap();
        let uri_helpers = Url::from_file_path(&helpers_path).unwrap();

        let utils_diags = &diags[&uri_utils];
        assert_eq!(utils_diags.len(), 1);
        let d = &utils_diags[0];
        assert_eq!(d.severity, Some(DiagnosticSeverity::WARNING));
        assert!(d.message.contains("formatDate"));
        // line 15 (1-based) → 14 (0-based)
        assert_eq!(d.range.start.line, 14);
        assert_eq!(d.range.start.character, 0);
        // Range spans the export name
        assert_eq!(d.range.end.character, "formatDate".len() as u32);
        // Related info points to the other file
        let related = d.related_information.as_ref().unwrap();
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].location.uri, uri_helpers);
        assert_eq!(related[0].message, "Also exported here");

        let helpers_diags = &diags[&uri_helpers];
        assert_eq!(helpers_diags.len(), 1);
        let dh = &helpers_diags[0];
        let related_h = dh.related_information.as_ref().unwrap();
        assert_eq!(related_h[0].location.uri, uri_utils);
    }

    #[test]
    fn duplication_diagnostic_has_related_information() {
        let root = test_root();
        let results = AnalysisResults::default();
        let duplication = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![
                    CloneInstance {
                        file: root.join("src/a.ts"),
                        start_line: 10,
                        end_line: 15,
                        start_col: 0,
                        end_col: 20,
                        fragment: "duplicated code".to_string(),
                    },
                    CloneInstance {
                        file: root.join("src/b.ts"),
                        start_line: 20,
                        end_line: 25,
                        start_col: 4,
                        end_col: 24,
                        fragment: "duplicated code".to_string(),
                    },
                ],
                token_count: 50,
                line_count: 6,
            }],
            clone_families: vec![],
            stats: DuplicationStats {
                total_files: 2,
                files_with_clones: 2,
                total_lines: 100,
                duplicated_lines: 12,
                total_tokens: 500,
                duplicated_tokens: 100,
                clone_groups: 1,
                clone_instances: 2,
                duplication_percentage: 12.0,
            },
        };

        let diags = build_diagnostics(&results, &duplication, &root);

        // File a.ts should have a diagnostic with related info pointing to b.ts
        let uri_a = Url::from_file_path(root.join("src/a.ts")).unwrap();
        let diags_a = &diags[&uri_a];
        assert_eq!(diags_a.len(), 1);

        let d = &diags_a[0];
        assert_eq!(d.severity, Some(DiagnosticSeverity::INFORMATION));
        assert_eq!(
            d.code,
            Some(NumberOrString::String("code-duplication".to_string()))
        );
        assert!(d.message.contains("6 lines"));
        assert!(d.message.contains("2 instances"));

        // Check related info
        let related = d.related_information.as_ref().unwrap();
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].message, "Also duplicated here");
        let related_uri = Url::from_file_path(root.join("src/b.ts")).unwrap();
        assert_eq!(related[0].location.uri, related_uri);
        // b.ts start_line = 20 (1-based) → 19 (0-based)
        assert_eq!(related[0].location.range.start.line, 19);
        assert_eq!(related[0].location.range.start.character, 4);

        // File b.ts should have related info pointing to a.ts
        let uri_b = Url::from_file_path(root.join("src/b.ts")).unwrap();
        let diags_b = &diags[&uri_b];
        assert_eq!(diags_b.len(), 1);
        let related_b = diags_b[0].related_information.as_ref().unwrap();
        assert_eq!(related_b.len(), 1);
        assert_eq!(related_b[0].location.uri, uri_a);
    }

    #[test]
    fn multiple_issues_same_file_aggregate() {
        let root = test_root();
        let mut results = AnalysisResults::default();
        let path = root.join("src/mod.ts");
        results.unused_exports.push(UnusedExport {
            path: path.clone(),
            export_name: "foo".to_string(),
            is_type_only: false,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        results.unused_exports.push(UnusedExport {
            path: path.clone(),
            export_name: "bar".to_string(),
            is_type_only: false,
            line: 5,
            col: 0,
            span_start: 50,
            is_re_export: false,
        });
        results.unresolved_imports.push(UnresolvedImport {
            path: path.clone(),
            specifier: "./gone".to_string(),
            line: 10,
            col: 0,
            specifier_col: 0,
        });

        let duplication = empty_duplication();
        let diags = build_diagnostics(&results, &duplication, &root);

        let uri = Url::from_file_path(&path).unwrap();
        let file_diags = &diags[&uri];
        assert_eq!(file_diags.len(), 3);
    }

    #[test]
    fn all_diagnostics_have_fallow_source() {
        let root = test_root();
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/a.ts"),
        });
        results.unused_exports.push(UnusedExport {
            path: root.join("src/b.ts"),
            export_name: "x".to_string(),
            is_type_only: false,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        results.unresolved_imports.push(UnresolvedImport {
            path: root.join("src/c.ts"),
            specifier: "./nope".to_string(),
            line: 1,
            col: 0,
            specifier_col: 0,
        });

        let duplication = empty_duplication();
        let diags = build_diagnostics(&results, &duplication, &root);

        for file_diags in diags.values() {
            for d in file_diags {
                assert_eq!(d.source, Some("fallow".to_string()));
            }
        }
    }

    #[test]
    fn line_conversion_saturates_at_zero() {
        let root = test_root();
        // Line 0 in results (unusual) should become 0 in LSP, not underflow
        let mut results = AnalysisResults::default();
        results.unused_exports.push(UnusedExport {
            path: root.join("src/edge.ts"),
            export_name: "x".to_string(),
            is_type_only: false,
            line: 0,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });

        let duplication = empty_duplication();
        let diags = build_diagnostics(&results, &duplication, &root);

        let uri = Url::from_file_path(root.join("src/edge.ts")).unwrap();
        let d = &diags[&uri][0];
        assert_eq!(d.range.start.line, 0);
    }

    #[test]
    fn unused_optional_dependency_produces_warning() {
        let root = test_root();
        let mut results = AnalysisResults::default();
        results.unused_optional_dependencies.push(UnusedDependency {
            package_name: "fsevents".to_string(),
            location: DependencyLocation::OptionalDependencies,
            path: root.join("package.json"),
            line: 12,
        });

        let duplication = empty_duplication();
        let diags = build_diagnostics(&results, &duplication, &root);

        let uri = Url::from_file_path(root.join("package.json")).unwrap();
        let file_diags = &diags[&uri];
        assert_eq!(file_diags.len(), 1);

        let d = &file_diags[0];
        assert_eq!(d.severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(d.message, "Unused optionalDependency: fsevents");
        assert_eq!(
            d.code,
            Some(NumberOrString::String(
                "unused-optional-dependency".to_string()
            ))
        );
        assert_eq!(d.range.start.line, 11); // 1-based 12 -> 0-based 11
        assert_eq!(d.range.start.character, 0);
        assert_eq!(d.range.end.character, u32::MAX);
    }

    #[test]
    fn circular_dependency_produces_warning_with_chain_message() {
        let root = test_root();
        let file_a = root.join("src/a.ts");
        let file_b = root.join("src/b.ts");
        let file_c = root.join("src/c.ts");

        let mut results = AnalysisResults::default();
        results
            .circular_dependencies
            .push(CircularDependency {
                files: vec![file_a.clone(), file_b.clone(), file_c.clone()],
                length: 3,
                line: 2,
                col: 20,
            });

        let duplication = empty_duplication();
        let diags = build_diagnostics(&results, &duplication, &root);

        // Diagnostic should be on the first file in the cycle
        let uri_a = Url::from_file_path(&file_a).unwrap();
        let file_diags = &diags[&uri_a];
        assert_eq!(file_diags.len(), 1);

        let d = &file_diags[0];
        assert_eq!(d.severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(
            d.code,
            Some(NumberOrString::String("circular-dependency".to_string()))
        );
        assert!(d.message.contains("Circular dependency"));
        assert!(d.message.contains("a.ts"));
        assert!(d.message.contains("b.ts"));
        assert!(d.message.contains("c.ts"));
        assert!(d.message.contains("\u{2192}")); // arrow separator

        // Line should be 0-based
        assert_eq!(d.range.start.line, 1); // 1-based 2 -> 0-based 1
        assert_eq!(d.range.start.character, 20);
        assert_eq!(d.range.end.character, u32::MAX);

        // Related information should point to other files in the cycle
        let related = d.related_information.as_ref().unwrap();
        assert_eq!(related.len(), 2); // file_b and file_c (skips first file)
        assert_eq!(related[0].message, "Step 2 in cycle: b.ts");
        assert_eq!(related[1].message, "Step 3 in cycle: c.ts");

        let uri_b = Url::from_file_path(&file_b).unwrap();
        let uri_c = Url::from_file_path(&file_c).unwrap();
        assert_eq!(related[0].location.uri, uri_b);
        assert_eq!(related[1].location.uri, uri_c);
    }

    #[test]
    fn circular_dependency_with_single_file_has_no_related_info() {
        let root = test_root();
        let file_a = root.join("src/self.ts");

        let mut results = AnalysisResults::default();
        results
            .circular_dependencies
            .push(CircularDependency {
                files: vec![file_a.clone()],
                length: 1,
                line: 1,
                col: 0,
            });

        let duplication = empty_duplication();
        let diags = build_diagnostics(&results, &duplication, &root);

        let uri = Url::from_file_path(&file_a).unwrap();
        let d = &diags[&uri][0];
        // With a single file, skip(1) yields nothing, so related_information is None
        assert!(d.related_information.is_none());
    }

    #[test]
    fn circular_dependency_with_empty_files_produces_no_diagnostic() {
        let root = test_root();
        let mut results = AnalysisResults::default();
        results
            .circular_dependencies
            .push(CircularDependency {
                files: vec![],
                length: 0,
                line: 0,
                col: 0,
            });

        let duplication = empty_duplication();
        let diags = build_diagnostics(&results, &duplication, &root);
        assert!(diags.is_empty());
    }

    #[test]
    fn type_only_dependency_produces_information_diagnostic() {
        let root = test_root();
        let mut results = AnalysisResults::default();
        results.type_only_dependencies.push(TypeOnlyDependency {
            package_name: "@types/react".to_string(),
            path: root.join("package.json"),
            line: 8,
        });

        let duplication = empty_duplication();
        let diags = build_diagnostics(&results, &duplication, &root);

        let uri = Url::from_file_path(root.join("package.json")).unwrap();
        let file_diags = &diags[&uri];
        assert_eq!(file_diags.len(), 1);

        let d = &file_diags[0];
        assert_eq!(d.severity, Some(DiagnosticSeverity::INFORMATION));
        assert_eq!(
            d.code,
            Some(NumberOrString::String("type-only-dependency".to_string()))
        );
        assert!(d.message.contains("@types/react"));
        assert!(d.message.contains("Type-only dependency"));
        assert!(d.message.contains("devDependency"));
        assert_eq!(d.range.start.line, 7); // 1-based 8 -> 0-based 7
        assert_eq!(d.range.start.character, 0);
        assert_eq!(d.range.end.character, u32::MAX);
    }

    #[test]
    fn doc_link_produces_valid_url() {
        let link = doc_link("unused-exports");
        assert!(link.is_some());
        let desc = link.unwrap();
        assert_eq!(
            desc.href.as_str(),
            "https://docs.fallow.tools/explanations/dead-code#unused-exports"
        );
    }

    #[test]
    fn first_line_range_values() {
        assert_eq!(FIRST_LINE_RANGE.start.line, 0);
        assert_eq!(FIRST_LINE_RANGE.start.character, 0);
        assert_eq!(FIRST_LINE_RANGE.end.line, 0);
        assert_eq!(FIRST_LINE_RANGE.end.character, u32::MAX);
    }

    #[test]
    fn all_diagnostic_codes_have_doc_links() {
        let root = test_root();
        let path = root.join("src/file.ts");
        let mut results = AnalysisResults::default();

        // Add one of each issue type to verify all produce code_description
        results.unused_exports.push(UnusedExport {
            path: path.clone(),
            export_name: "e".to_string(),
            is_type_only: false,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        results.unused_types.push(UnusedExport {
            path: path.clone(),
            export_name: "T".to_string(),
            is_type_only: true,
            line: 2,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        results.unused_files.push(UnusedFile {
            path: path.clone(),
        });
        results.unused_enum_members.push(UnusedMember {
            path: path.clone(),
            parent_name: "E".to_string(),
            member_name: "A".to_string(),
            kind: MemberKind::EnumMember,
            line: 3,
            col: 0,
        });

        let duplication = empty_duplication();
        let diags = build_diagnostics(&results, &duplication, &root);

        let uri = Url::from_file_path(&path).unwrap();
        let file_diags = &diags[&uri];

        for d in file_diags {
            assert!(
                d.code_description.is_some(),
                "Diagnostic code {:?} should have a code_description doc link",
                d.code
            );
            let href = &d.code_description.as_ref().unwrap().href;
            assert!(
                href.as_str().starts_with("https://docs.fallow.tools/"),
                "Doc link should point to fallow docs: {href}"
            );
        }
    }

    #[test]
    fn duplication_with_single_instance_has_no_related_info() {
        let root = test_root();
        let results = AnalysisResults::default();
        let duplication = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![CloneInstance {
                    file: root.join("src/only.ts"),
                    start_line: 1,
                    end_line: 5,
                    start_col: 0,
                    end_col: 10,
                    fragment: "code".to_string(),
                }],
                token_count: 20,
                line_count: 5,
            }],
            clone_families: vec![],
            stats: DuplicationStats {
                total_files: 1,
                files_with_clones: 1,
                total_lines: 20,
                duplicated_lines: 5,
                total_tokens: 100,
                duplicated_tokens: 20,
                clone_groups: 1,
                clone_instances: 1,
                duplication_percentage: 25.0,
            },
        };

        let diags = build_diagnostics(&results, &duplication, &root);
        let uri = Url::from_file_path(root.join("src/only.ts")).unwrap();
        let d = &diags[&uri][0];

        // Single instance => no "other" instances => no related info
        assert!(d.related_information.is_none());
    }

    #[test]
    fn duplicate_export_with_single_location_has_no_related_info() {
        let root = test_root();
        let path = root.join("src/solo.ts");

        let mut results = AnalysisResults::default();
        results.duplicate_exports.push(DuplicateExport {
            export_name: "helper".to_string(),
            locations: vec![DuplicateLocation {
                path: path.clone(),
                line: 5,
                col: 0,
            }],
        });

        let duplication = empty_duplication();
        let diags = build_diagnostics(&results, &duplication, &root);

        let uri = Url::from_file_path(&path).unwrap();
        let d = &diags[&uri][0];
        // No other locations to relate to
        assert!(d.related_information.is_none());
    }
}
