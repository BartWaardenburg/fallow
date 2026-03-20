use std::path::Path;

use fallow_core::duplicates::DuplicationReport;
use fallow_core::results::{AnalysisResults, UnusedExport, UnusedMember};

use super::{relative_path, normalize_uri};

pub(super) fn print_compact(results: &AnalysisResults, root: &Path) {
    for line in build_compact_lines(results, root) {
        println!("{line}");
    }
}

/// Build compact output lines for analysis results.
/// Each issue is represented as a single `prefix:details` line.
pub fn build_compact_lines(results: &AnalysisResults, root: &Path) -> Vec<String> {
    let rel = |p: &Path| normalize_uri(&relative_path(p, root).display().to_string());

    let compact_export = |export: &UnusedExport, kind: &str, re_kind: &str| -> String {
        let tag = if export.is_re_export { re_kind } else { kind };
        format!(
            "{}:{}:{}:{}",
            tag,
            rel(&export.path),
            export.line,
            export.export_name
        )
    };

    let compact_member = |member: &UnusedMember, kind: &str| -> String {
        format!(
            "{}:{}:{}:{}.{}",
            kind,
            rel(&member.path),
            member.line,
            member.parent_name,
            member.member_name
        )
    };

    let mut lines = Vec::new();

    for file in &results.unused_files {
        lines.push(format!("unused-file:{}", rel(&file.path)));
    }
    for export in &results.unused_exports {
        lines.push(compact_export(export, "unused-export", "unused-re-export"));
    }
    for export in &results.unused_types {
        lines.push(compact_export(
            export,
            "unused-type",
            "unused-re-export-type",
        ));
    }
    for dep in &results.unused_dependencies {
        lines.push(format!("unused-dep:{}", dep.package_name));
    }
    for dep in &results.unused_dev_dependencies {
        lines.push(format!("unused-devdep:{}", dep.package_name));
    }
    for member in &results.unused_enum_members {
        lines.push(compact_member(member, "unused-enum-member"));
    }
    for member in &results.unused_class_members {
        lines.push(compact_member(member, "unused-class-member"));
    }
    for import in &results.unresolved_imports {
        lines.push(format!(
            "unresolved-import:{}:{}:{}",
            rel(&import.path),
            import.line,
            import.specifier
        ));
    }
    for dep in &results.unlisted_dependencies {
        lines.push(format!("unlisted-dep:{}", dep.package_name));
    }
    for dup in &results.duplicate_exports {
        lines.push(format!("duplicate-export:{}", dup.export_name));
    }

    lines
}

pub(super) fn print_duplication_compact(report: &DuplicationReport, root: &Path) {
    for (i, group) in report.clone_groups.iter().enumerate() {
        for instance in &group.instances {
            let relative = normalize_uri(&relative_path(&instance.file, root).display().to_string());
            println!(
                "clone-group-{}:{}:{}-{}:{}tokens",
                i + 1,
                relative,
                instance.start_line,
                instance.end_line,
                group.token_count
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

        r
    }

    #[test]
    fn compact_empty_results_no_lines() {
        let root = PathBuf::from("/project");
        let results = AnalysisResults::default();
        let lines = build_compact_lines(&results, &root);
        assert!(lines.is_empty());
    }

    #[test]
    fn compact_unused_file_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: root.join("src/dead.ts"),
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "unused-file:src/dead.ts");
    }

    #[test]
    fn compact_unused_export_format() {
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

        let lines = build_compact_lines(&results, &root);
        assert_eq!(lines[0], "unused-export:src/utils.ts:10:helperFn");
    }

    #[test]
    fn compact_unused_type_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_types.push(UnusedExport {
            path: root.join("src/types.ts"),
            export_name: "OldType".to_string(),
            is_type_only: true,
            line: 5,
            col: 0,
            span_start: 60,
            is_re_export: false,
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(lines[0], "unused-type:src/types.ts:5:OldType");
    }

    #[test]
    fn compact_unused_dep_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_dependencies.push(UnusedDependency {
            package_name: "lodash".to_string(),
            location: DependencyLocation::Dependencies,
            path: root.join("package.json"),
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(lines[0], "unused-dep:lodash");
    }

    #[test]
    fn compact_unused_devdep_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_dev_dependencies.push(UnusedDependency {
            package_name: "jest".to_string(),
            location: DependencyLocation::DevDependencies,
            path: root.join("package.json"),
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(lines[0], "unused-devdep:jest");
    }

    #[test]
    fn compact_unused_enum_member_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_enum_members.push(UnusedMember {
            path: root.join("src/enums.ts"),
            parent_name: "Status".to_string(),
            member_name: "Deprecated".to_string(),
            kind: MemberKind::EnumMember,
            line: 8,
            col: 2,
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(
            lines[0],
            "unused-enum-member:src/enums.ts:8:Status.Deprecated"
        );
    }

    #[test]
    fn compact_unused_class_member_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_class_members.push(UnusedMember {
            path: root.join("src/service.ts"),
            parent_name: "UserService".to_string(),
            member_name: "legacyMethod".to_string(),
            kind: MemberKind::ClassMethod,
            line: 42,
            col: 4,
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(
            lines[0],
            "unused-class-member:src/service.ts:42:UserService.legacyMethod"
        );
    }

    #[test]
    fn compact_unresolved_import_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unresolved_imports.push(UnresolvedImport {
            path: root.join("src/app.ts"),
            specifier: "./missing-module".to_string(),
            line: 3,
            col: 0,
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(lines[0], "unresolved-import:src/app.ts:3:./missing-module");
    }

    #[test]
    fn compact_unlisted_dep_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unlisted_dependencies.push(UnlistedDependency {
            package_name: "chalk".to_string(),
            imported_from: vec![],
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(lines[0], "unlisted-dep:chalk");
    }

    #[test]
    fn compact_duplicate_export_format() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.duplicate_exports.push(DuplicateExport {
            export_name: "Config".to_string(),
            locations: vec![root.join("src/a.ts"), root.join("src/b.ts")],
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(lines[0], "duplicate-export:Config");
    }

    #[test]
    fn compact_all_issue_types_produce_lines() {
        let root = PathBuf::from("/project");
        let results = sample_results(&root);
        let lines = build_compact_lines(&results, &root);

        // 10 issue types, one of each
        assert_eq!(lines.len(), 10);

        // Verify ordering: unused_files first, duplicate_exports last
        assert!(lines[0].starts_with("unused-file:"));
        assert!(lines[1].starts_with("unused-export:"));
        assert!(lines[2].starts_with("unused-type:"));
        assert!(lines[3].starts_with("unused-dep:"));
        assert!(lines[4].starts_with("unused-devdep:"));
        assert!(lines[5].starts_with("unused-enum-member:"));
        assert!(lines[6].starts_with("unused-class-member:"));
        assert!(lines[7].starts_with("unresolved-import:"));
        assert!(lines[8].starts_with("unlisted-dep:"));
        assert!(lines[9].starts_with("duplicate-export:"));
    }

    #[test]
    fn compact_strips_root_prefix_from_paths() {
        let root = PathBuf::from("/project");
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: PathBuf::from("/project/src/deep/nested/file.ts"),
        });

        let lines = build_compact_lines(&results, &root);
        assert_eq!(lines[0], "unused-file:src/deep/nested/file.ts");
    }
}
