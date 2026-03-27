use std::path::Path;

use fallow_core::extract::MemberKind;
use fallow_core::results::*;

/// Build an `AnalysisResults` populated with one issue of every type.
///
/// Shared across all report format tests for consistency.
pub(super) fn sample_results(root: &Path) -> AnalysisResults {
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
        line: 5,
    });
    r.unused_dev_dependencies.push(UnusedDependency {
        package_name: "jest".to_string(),
        location: DependencyLocation::DevDependencies,
        path: root.join("package.json"),
        line: 5,
    });
    r.unused_optional_dependencies.push(UnusedDependency {
        package_name: "fsevents".to_string(),
        location: DependencyLocation::OptionalDependencies,
        path: root.join("package.json"),
        line: 15,
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
        specifier_col: 0,
    });
    r.unlisted_dependencies.push(UnlistedDependency {
        package_name: "chalk".to_string(),
        imported_from: vec![ImportSite {
            path: root.join("src/cli.ts"),
            line: 2,
            col: 0,
        }],
    });
    r.duplicate_exports.push(DuplicateExport {
        export_name: "Config".to_string(),
        locations: vec![
            DuplicateLocation {
                path: root.join("src/config.ts"),
                line: 15,
                col: 0,
            },
            DuplicateLocation {
                path: root.join("src/types.ts"),
                line: 30,
                col: 0,
            },
        ],
    });
    r.type_only_dependencies.push(TypeOnlyDependency {
        package_name: "zod".to_string(),
        path: root.join("package.json"),
        line: 8,
    });
    r.test_only_dependencies.push(TestOnlyDependency {
        package_name: "msw".to_string(),
        path: root.join("package.json"),
        line: 12,
    });
    r.circular_dependencies.push(CircularDependency {
        files: vec![root.join("src/a.ts"), root.join("src/b.ts")],
        length: 2,
        line: 3,
        col: 0,
    });

    r
}
