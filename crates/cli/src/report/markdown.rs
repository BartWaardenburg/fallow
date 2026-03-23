use std::fmt::Write;
use std::path::Path;

use fallow_core::duplicates::DuplicationReport;
use fallow_core::results::{AnalysisResults, UnusedExport, UnusedMember};

use super::{normalize_uri, relative_path};

/// Escape backticks in user-controlled strings to prevent breaking markdown code spans.
fn escape_backticks(s: &str) -> String {
    s.replace('`', "\\`")
}

pub(super) fn print_markdown(results: &AnalysisResults, root: &Path) {
    println!("{}", build_markdown(results, root));
}

/// Build markdown output for analysis results.
pub fn build_markdown(results: &AnalysisResults, root: &Path) -> String {
    let rel = |p: &Path| {
        escape_backticks(&normalize_uri(
            &relative_path(p, root).display().to_string(),
        ))
    };

    let total = results.total_issues();
    let mut out = String::new();

    if total == 0 {
        out.push_str("## Fallow: no issues found\n");
        return out;
    }

    let _ = write!(
        out,
        "## Fallow: {total} issue{} found\n\n",
        if total == 1 { "" } else { "s" }
    );

    // ── Unused files ──
    markdown_section(&mut out, &results.unused_files, "Unused files", |file| {
        vec![format!("- `{}`", rel(&file.path))]
    });

    // ── Unused exports ──
    markdown_grouped_section(
        &mut out,
        &results.unused_exports,
        "Unused exports",
        root,
        |e| e.path.as_path(),
        format_export,
    );

    // ── Unused types ──
    markdown_grouped_section(
        &mut out,
        &results.unused_types,
        "Unused type exports",
        root,
        |e| e.path.as_path(),
        format_export,
    );

    // ── Unused dependencies ──
    markdown_section(
        &mut out,
        &results.unused_dependencies,
        "Unused dependencies",
        |dep| format_dependency(&dep.package_name, &dep.path, root),
    );

    // ── Unused devDependencies ──
    markdown_section(
        &mut out,
        &results.unused_dev_dependencies,
        "Unused devDependencies",
        |dep| format_dependency(&dep.package_name, &dep.path, root),
    );

    // ── Unused optionalDependencies ──
    markdown_section(
        &mut out,
        &results.unused_optional_dependencies,
        "Unused optionalDependencies",
        |dep| format_dependency(&dep.package_name, &dep.path, root),
    );

    // ── Unused enum members ──
    markdown_grouped_section(
        &mut out,
        &results.unused_enum_members,
        "Unused enum members",
        root,
        |m| m.path.as_path(),
        format_member,
    );

    // ── Unused class members ──
    markdown_grouped_section(
        &mut out,
        &results.unused_class_members,
        "Unused class members",
        root,
        |m| m.path.as_path(),
        format_member,
    );

    // ── Unresolved imports ──
    markdown_grouped_section(
        &mut out,
        &results.unresolved_imports,
        "Unresolved imports",
        root,
        |i| i.path.as_path(),
        |i| format!(":{} `{}`", i.line, escape_backticks(&i.specifier)),
    );

    // ── Unlisted dependencies ──
    markdown_section(
        &mut out,
        &results.unlisted_dependencies,
        "Unlisted dependencies",
        |dep| vec![format!("- `{}`", escape_backticks(&dep.package_name))],
    );

    // ── Duplicate exports ──
    markdown_section(
        &mut out,
        &results.duplicate_exports,
        "Duplicate exports",
        |dup| {
            let locations: Vec<String> = dup
                .locations
                .iter()
                .map(|p| format!("`{}`", rel(p)))
                .collect();
            vec![format!(
                "- `{}` in {}",
                escape_backticks(&dup.export_name),
                locations.join(", ")
            )]
        },
    );

    // ── Type-only dependencies ──
    markdown_section(
        &mut out,
        &results.type_only_dependencies,
        "Type-only dependencies (consider moving to devDependencies)",
        |dep| format_dependency(&dep.package_name, &dep.path, root),
    );

    // ── Circular dependencies ──
    markdown_section(
        &mut out,
        &results.circular_dependencies,
        "Circular dependencies",
        |cycle| {
            let chain: Vec<String> = cycle.files.iter().map(|p| rel(p)).collect();
            let mut display_chain = chain.clone();
            if let Some(first) = chain.first() {
                display_chain.push(first.clone());
            }
            vec![format!(
                "- {}",
                display_chain
                    .iter()
                    .map(|s| format!("`{s}`"))
                    .collect::<Vec<_>>()
                    .join(" \u{2192} ")
            )]
        },
    );

    out
}

fn format_export(e: &UnusedExport) -> String {
    let re = if e.is_re_export { " (re-export)" } else { "" };
    format!(":{} `{}`{re}", e.line, escape_backticks(&e.export_name))
}

fn format_member(m: &UnusedMember) -> String {
    format!(
        ":{} `{}.{}`",
        m.line,
        escape_backticks(&m.parent_name),
        escape_backticks(&m.member_name)
    )
}

fn format_dependency(dep_name: &str, pkg_path: &Path, root: &Path) -> Vec<String> {
    let name = escape_backticks(dep_name);
    let pkg_label = relative_path(pkg_path, root).display().to_string();
    if pkg_label == "package.json" {
        vec![format!("- `{name}`")]
    } else {
        let label = escape_backticks(&pkg_label);
        vec![format!("- `{name}` ({label})")]
    }
}

/// Emit a markdown section with a header and per-item lines. Skipped if empty.
fn markdown_section<T>(
    out: &mut String,
    items: &[T],
    title: &str,
    format_lines: impl Fn(&T) -> Vec<String>,
) {
    if items.is_empty() {
        return;
    }
    let _ = write!(out, "### {title} ({})\n\n", items.len());
    for item in items {
        for line in format_lines(item) {
            out.push_str(&line);
            out.push('\n');
        }
    }
    out.push('\n');
}

/// Emit a markdown section whose items are grouped by file path.
fn markdown_grouped_section<'a, T>(
    out: &mut String,
    items: &'a [T],
    title: &str,
    root: &Path,
    get_path: impl Fn(&'a T) -> &'a Path,
    format_detail: impl Fn(&T) -> String,
) {
    if items.is_empty() {
        return;
    }
    let _ = write!(out, "### {title} ({})\n\n", items.len());

    let mut indices: Vec<usize> = (0..items.len()).collect();
    indices.sort_by(|&a, &b| get_path(&items[a]).cmp(get_path(&items[b])));

    let rel = |p: &Path| normalize_uri(&relative_path(p, root).display().to_string());
    let mut last_file = String::new();
    for &i in &indices {
        let item = &items[i];
        let file_str = rel(get_path(item));
        if file_str != last_file {
            let _ = writeln!(out, "- `{file_str}`");
            last_file = file_str;
        }
        let _ = writeln!(out, "  - {}", format_detail(item));
    }
    out.push('\n');
}

// ── Duplication markdown output ──────────────────────────────────

pub(super) fn print_duplication_markdown(report: &DuplicationReport, root: &Path) {
    println!("{}", build_duplication_markdown(report, root));
}

/// Build markdown output for duplication results.
pub fn build_duplication_markdown(report: &DuplicationReport, root: &Path) -> String {
    let rel = |p: &Path| normalize_uri(&relative_path(p, root).display().to_string());

    let mut out = String::new();

    if report.clone_groups.is_empty() {
        out.push_str("## Fallow: no code duplication found\n");
        return out;
    }

    let stats = &report.stats;
    let _ = write!(
        out,
        "## Fallow: {} clone group{} found ({:.1}% duplication)\n\n",
        stats.clone_groups,
        if stats.clone_groups == 1 { "" } else { "s" },
        stats.duplication_percentage,
    );

    out.push_str("### Duplicates\n\n");
    for (i, group) in report.clone_groups.iter().enumerate() {
        let instance_count = group.instances.len();
        let _ = write!(
            out,
            "**Clone group {}** ({} lines, {instance_count} instance{})\n\n",
            i + 1,
            group.line_count,
            if instance_count == 1 { "" } else { "s" }
        );
        for instance in &group.instances {
            let relative = rel(&instance.file);
            let _ = writeln!(
                out,
                "- `{relative}:{}-{}`",
                instance.start_line, instance.end_line
            );
        }
        out.push('\n');
    }

    // Clone families
    if !report.clone_families.is_empty() {
        out.push_str("### Clone Families\n\n");
        for (i, family) in report.clone_families.iter().enumerate() {
            let file_names: Vec<_> = family.files.iter().map(|f| rel(f)).collect();
            let _ = write!(
                out,
                "**Family {}** ({} group{}, {} lines across {})\n\n",
                i + 1,
                family.groups.len(),
                if family.groups.len() == 1 { "" } else { "s" },
                family.total_duplicated_lines,
                file_names
                    .iter()
                    .map(|s| format!("`{s}`"))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            for suggestion in &family.suggestions {
                let savings = if suggestion.estimated_savings > 0 {
                    format!(" (~{} lines saved)", suggestion.estimated_savings)
                } else {
                    String::new()
                };
                let _ = writeln!(out, "- {}{savings}", suggestion.description);
            }
            out.push('\n');
        }
    }

    // Summary line
    let _ = writeln!(
        out,
        "**Summary:** {} duplicated lines ({:.1}%) across {} file{}",
        stats.duplicated_lines,
        stats.duplication_percentage,
        stats.files_with_clones,
        if stats.files_with_clones == 1 {
            ""
        } else {
            "s"
        },
    );

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use fallow_core::duplicates::{
        CloneFamily, CloneGroup, CloneInstance, DuplicationReport, DuplicationStats,
        RefactoringKind, RefactoringSuggestion,
    };
    use fallow_core::extract::MemberKind;
    use fallow_core::results::*;
    use std::path::PathBuf;

    /// Helper: build an `AnalysisResults` populated with one issue of every type.
    fn sample_results(root: &Path) -> AnalysisResults {
        let mut r = AnalysisResults::default();

        r.unused_files.push(UnusedFile {
            path: root.join("src/dead.ts"),
        });
        r.unused_exports.push(UnusedExport {
            path: root.join("src/utils.ts"),
            export_name: "helperFn".to_string(),
            is_type_only: false,
            line: 10,
            col: 4,
            span_start: 120,
            is_re_export: false,
        });
        r.unused_types.push(UnusedExport {
            path: root.join("src/types.ts"),
            export_name: "OldType".to_string(),
            is_type_only: true,
            line: 5,
            col: 0,
            span_start: 60,
            is_re_export: false,
        });
        r.unused_dependencies.push(UnusedDependency {
            package_name: "lodash".to_string(),
            location: DependencyLocation::Dependencies,
            path: root.join("package.json"),
        });
        r.unused_dev_dependencies.push(UnusedDependency {
            package_name: "jest".to_string(),
            location: DependencyLocation::DevDependencies,
            path: root.join("package.json"),
        });
        r.unused_enum_members.push(UnusedMember {
            path: root.join("src/enums.ts"),
            parent_name: "Status".to_string(),
            member_name: "Deprecated".to_string(),
            kind: MemberKind::EnumMember,
            line: 8,
            col: 2,
        });
        r.unused_class_members.push(UnusedMember {
            path: root.join("src/service.ts"),
            parent_name: "UserService".to_string(),
            member_name: "legacyMethod".to_string(),
            kind: MemberKind::ClassMethod,
            line: 42,
            col: 4,
        });
        r.unresolved_imports.push(UnresolvedImport {
            path: root.join("src/app.ts"),
            specifier: "./missing-module".to_string(),
            line: 3,
            col: 0,
        });
        r.unlisted_dependencies.push(UnlistedDependency {
            package_name: "chalk".to_string(),
            imported_from: vec![root.join("src/cli.ts")],
        });
        r.duplicate_exports.push(DuplicateExport {
            export_name: "Config".to_string(),
            locations: vec![root.join("src/config.ts"), root.join("src/types.ts")],
        });
        r.type_only_dependencies.push(TypeOnlyDependency {
            package_name: "zod".to_string(),
            path: root.join("package.json"),
        });
        r.circular_dependencies.push(CircularDependency {
            files: vec![root.join("src/a.ts"), root.join("src/b.ts")],
            length: 2,
        });

        r
    }

    #[test]
    fn markdown_empty_results_no_issues() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let md = build_markdown(&results, &root);
        assert_eq!(md, "## Fallow: no issues found\n");
    }

    #[test]
    fn markdown_contains_header_with_count() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let md = build_markdown(&results, &root);
        assert!(md.starts_with(&format!(
            "## Fallow: {} issues found\n",
            results.total_issues()
        )));
    }

    #[test]
    fn markdown_contains_all_sections() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let md = build_markdown(&results, &root);

        assert!(md.contains("### Unused files (1)"));
        assert!(md.contains("### Unused exports (1)"));
        assert!(md.contains("### Unused type exports (1)"));
        assert!(md.contains("### Unused dependencies (1)"));
        assert!(md.contains("### Unused devDependencies (1)"));
        assert!(md.contains("### Unused enum members (1)"));
        assert!(md.contains("### Unused class members (1)"));
        assert!(md.contains("### Unresolved imports (1)"));
        assert!(md.contains("### Unlisted dependencies (1)"));
        assert!(md.contains("### Duplicate exports (1)"));
        assert!(md.contains("### Type-only dependencies"));
        assert!(md.contains("### Circular dependencies (1)"));
    }

    #[test]
    fn markdown_unused_file_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/dead.ts"),
        });
        let md = build_markdown(&results, &root);
        assert!(md.contains("- `src/dead.ts`"));
    }

    #[test]
    fn markdown_unused_export_grouped_by_file() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_exports.push(UnusedExport {
            path: root.join("src/utils.ts"),
            export_name: "helperFn".to_string(),
            is_type_only: false,
            line: 10,
            col: 4,
            span_start: 120,
            is_re_export: false,
        });
        let md = build_markdown(&results, &root);
        assert!(md.contains("- `src/utils.ts`"));
        assert!(md.contains(":10 `helperFn`"));
    }

    #[test]
    fn markdown_re_export_tagged() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_exports.push(UnusedExport {
            path: root.join("src/index.ts"),
            export_name: "reExported".to_string(),
            is_type_only: false,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: true,
        });
        let md = build_markdown(&results, &root);
        assert!(md.contains("(re-export)"));
    }

    #[test]
    fn markdown_unused_dep_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_dependencies.push(UnusedDependency {
            package_name: "lodash".to_string(),
            location: DependencyLocation::Dependencies,
            path: root.join("package.json"),
        });
        let md = build_markdown(&results, &root);
        assert!(md.contains("- `lodash`"));
    }

    #[test]
    fn markdown_circular_dep_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.circular_dependencies.push(CircularDependency {
            files: vec![root.join("src/a.ts"), root.join("src/b.ts")],
            length: 2,
        });
        let md = build_markdown(&results, &root);
        assert!(md.contains("`src/a.ts`"));
        assert!(md.contains("`src/b.ts`"));
        assert!(md.contains("\u{2192}"));
    }

    #[test]
    fn markdown_strips_root_prefix() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: PathBuf::from("/project/src/deep/nested/file.ts"),
        });
        let md = build_markdown(&results, &root);
        assert!(md.contains("`src/deep/nested/file.ts`"));
        assert!(!md.contains("/project/"));
    }

    #[test]
    fn markdown_single_issue_no_plural() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/dead.ts"),
        });
        let md = build_markdown(&results, &root);
        assert!(md.starts_with("## Fallow: 1 issue found\n"));
    }

    #[test]
    fn markdown_type_only_dep_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.type_only_dependencies.push(TypeOnlyDependency {
            package_name: "zod".to_string(),
            path: root.join("package.json"),
        });
        let md = build_markdown(&results, &root);
        assert!(md.contains("### Type-only dependencies"));
        assert!(md.contains("- `zod`"));
    }

    #[test]
    fn markdown_escapes_backticks_in_export_names() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_exports.push(UnusedExport {
            path: root.join("src/utils.ts"),
            export_name: "foo`bar".to_string(),
            is_type_only: false,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        let md = build_markdown(&results, &root);
        assert!(md.contains("foo\\`bar"));
        assert!(!md.contains("foo`bar`"));
    }

    #[test]
    fn markdown_escapes_backticks_in_package_names() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_dependencies.push(UnusedDependency {
            package_name: "pkg`name".to_string(),
            location: DependencyLocation::Dependencies,
            path: root.join("package.json"),
        });
        let md = build_markdown(&results, &root);
        assert!(md.contains("pkg\\`name"));
    }

    // ── Duplication markdown ──

    #[test]
    fn duplication_markdown_empty() {
        let report = DuplicationReport::default();
        let root = PathBuf::from("/project");
        let md = build_duplication_markdown(&report, &root);
        assert_eq!(md, "## Fallow: no code duplication found\n");
    }

    #[test]
    fn duplication_markdown_contains_groups() {
        let root = PathBuf::from("/project");
        let report = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![
                    CloneInstance {
                        file: root.join("src/a.ts"),
                        start_line: 1,
                        end_line: 10,
                        start_col: 0,
                        end_col: 0,
                        fragment: String::new(),
                    },
                    CloneInstance {
                        file: root.join("src/b.ts"),
                        start_line: 5,
                        end_line: 14,
                        start_col: 0,
                        end_col: 0,
                        fragment: String::new(),
                    },
                ],
                token_count: 50,
                line_count: 10,
            }],
            clone_families: vec![],
            stats: DuplicationStats {
                total_files: 10,
                files_with_clones: 2,
                total_lines: 500,
                duplicated_lines: 20,
                total_tokens: 2500,
                duplicated_tokens: 100,
                clone_groups: 1,
                clone_instances: 2,
                duplication_percentage: 4.0,
            },
        };
        let md = build_duplication_markdown(&report, &root);
        assert!(md.contains("**Clone group 1**"));
        assert!(md.contains("`src/a.ts:1-10`"));
        assert!(md.contains("`src/b.ts:5-14`"));
        assert!(md.contains("4.0% duplication"));
    }

    #[test]
    fn duplication_markdown_contains_families() {
        let root = PathBuf::from("/project");
        let report = DuplicationReport {
            clone_groups: vec![CloneGroup {
                instances: vec![CloneInstance {
                    file: root.join("src/a.ts"),
                    start_line: 1,
                    end_line: 5,
                    start_col: 0,
                    end_col: 0,
                    fragment: String::new(),
                }],
                token_count: 30,
                line_count: 5,
            }],
            clone_families: vec![CloneFamily {
                files: vec![root.join("src/a.ts"), root.join("src/b.ts")],
                groups: vec![],
                total_duplicated_lines: 20,
                total_duplicated_tokens: 100,
                suggestions: vec![RefactoringSuggestion {
                    kind: RefactoringKind::ExtractFunction,
                    description: "Extract shared utility function".to_string(),
                    estimated_savings: 15,
                }],
            }],
            stats: DuplicationStats {
                clone_groups: 1,
                clone_instances: 1,
                duplication_percentage: 2.0,
                ..Default::default()
            },
        };
        let md = build_duplication_markdown(&report, &root);
        assert!(md.contains("### Clone Families"));
        assert!(md.contains("**Family 1**"));
        assert!(md.contains("Extract shared utility function"));
        assert!(md.contains("~15 lines saved"));
    }
}
