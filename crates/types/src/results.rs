use std::path::PathBuf;

use serde::Serialize;

use crate::extract::MemberKind;

/// Complete analysis results.
#[derive(Debug, Default, Clone, Serialize)]
pub struct AnalysisResults {
    pub unused_files: Vec<UnusedFile>,
    pub unused_exports: Vec<UnusedExport>,
    pub unused_types: Vec<UnusedExport>,
    pub unused_dependencies: Vec<UnusedDependency>,
    pub unused_dev_dependencies: Vec<UnusedDependency>,
    pub unused_enum_members: Vec<UnusedMember>,
    pub unused_class_members: Vec<UnusedMember>,
    pub unresolved_imports: Vec<UnresolvedImport>,
    pub unlisted_dependencies: Vec<UnlistedDependency>,
    pub duplicate_exports: Vec<DuplicateExport>,
    /// Production dependencies only used via type-only imports (could be devDependencies).
    /// Only populated in production mode.
    pub type_only_dependencies: Vec<TypeOnlyDependency>,
    /// Usage counts for all exports across the project. Used by the LSP for Code Lens.
    /// Not included in issue counts -- this is metadata, not an issue type.
    /// Skipped during serialization: this is internal LSP data, not part of the JSON output schema.
    #[serde(skip)]
    pub export_usages: Vec<ExportUsage>,
}

impl AnalysisResults {
    /// Total number of issues found.
    pub fn total_issues(&self) -> usize {
        self.unused_files.len()
            + self.unused_exports.len()
            + self.unused_types.len()
            + self.unused_dependencies.len()
            + self.unused_dev_dependencies.len()
            + self.unused_enum_members.len()
            + self.unused_class_members.len()
            + self.unresolved_imports.len()
            + self.unlisted_dependencies.len()
            + self.duplicate_exports.len()
            + self.type_only_dependencies.len()
    }

    /// Whether any issues were found.
    pub fn has_issues(&self) -> bool {
        self.total_issues() > 0
    }
}

/// A file that is not reachable from any entry point.
#[derive(Debug, Clone, Serialize)]
pub struct UnusedFile {
    pub path: PathBuf,
}

/// An export that is never imported by other modules.
#[derive(Debug, Clone, Serialize)]
pub struct UnusedExport {
    pub path: PathBuf,
    pub export_name: String,
    pub is_type_only: bool,
    pub line: u32,
    pub col: u32,
    /// Byte offset into the source file (used by the fix command).
    pub span_start: u32,
    /// Whether this finding comes from a barrel/index re-export rather than the source definition.
    pub is_re_export: bool,
}

/// A dependency that is listed in package.json but never imported.
#[derive(Debug, Clone, Serialize)]
pub struct UnusedDependency {
    pub package_name: String,
    pub location: DependencyLocation,
    /// Path to the package.json where this dependency is listed.
    /// For root deps this is `<root>/package.json`, for workspace deps it is `<ws>/package.json`.
    pub path: PathBuf,
}

/// Where in package.json a dependency is listed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DependencyLocation {
    Dependencies,
    DevDependencies,
}

/// An unused enum or class member.
#[derive(Debug, Clone, Serialize)]
pub struct UnusedMember {
    pub path: PathBuf,
    pub parent_name: String,
    pub member_name: String,
    pub kind: MemberKind,
    pub line: u32,
    pub col: u32,
}

/// An import that could not be resolved.
#[derive(Debug, Clone, Serialize)]
pub struct UnresolvedImport {
    pub path: PathBuf,
    pub specifier: String,
    pub line: u32,
    pub col: u32,
}

/// A dependency used in code but not listed in package.json.
#[derive(Debug, Clone, Serialize)]
pub struct UnlistedDependency {
    pub package_name: String,
    pub imported_from: Vec<PathBuf>,
}

/// An export that appears multiple times across the project.
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateExport {
    pub export_name: String,
    pub locations: Vec<PathBuf>,
}

/// A production dependency that is only used via type-only imports.
/// In production builds, type imports are erased, so this dependency
/// is not needed at runtime and could be moved to devDependencies.
#[derive(Debug, Clone, Serialize)]
pub struct TypeOnlyDependency {
    pub package_name: String,
    /// Path to the package.json where the dependency is listed.
    pub path: PathBuf,
}

/// Usage count for an export symbol. Used by the LSP Code Lens to show
/// reference counts above each export declaration.
#[derive(Debug, Clone, Serialize)]
pub struct ExportUsage {
    pub path: PathBuf,
    pub export_name: String,
    /// 1-based line number.
    pub line: u32,
    /// 0-based byte column offset.
    pub col: u32,
    pub reference_count: usize,
    /// Locations where this export is referenced. Used by the LSP Code Lens
    /// to enable click-to-navigate via `editor.action.showReferences`.
    pub reference_locations: Vec<ReferenceLocation>,
}

/// A location where an export is referenced (import site in another file).
#[derive(Debug, Clone, Serialize)]
pub struct ReferenceLocation {
    pub path: PathBuf,
    /// 1-based line number.
    pub line: u32,
    /// 0-based byte column offset.
    pub col: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_results_no_issues() {
        let results = AnalysisResults::default();
        assert_eq!(results.total_issues(), 0);
        assert!(!results.has_issues());
    }

    #[test]
    fn results_with_unused_file() {
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: PathBuf::from("test.ts"),
        });
        assert_eq!(results.total_issues(), 1);
        assert!(results.has_issues());
    }

    #[test]
    fn results_with_unused_export() {
        let mut results = AnalysisResults::default();
        results.unused_exports.push(UnusedExport {
            path: PathBuf::from("test.ts"),
            export_name: "foo".to_string(),
            is_type_only: false,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        assert_eq!(results.total_issues(), 1);
        assert!(results.has_issues());
    }

    #[test]
    fn results_total_counts_all_types() {
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: PathBuf::from("a.ts"),
        });
        results.unused_exports.push(UnusedExport {
            path: PathBuf::from("b.ts"),
            export_name: "x".to_string(),
            is_type_only: false,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        results.unused_types.push(UnusedExport {
            path: PathBuf::from("c.ts"),
            export_name: "T".to_string(),
            is_type_only: true,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        results.unused_dependencies.push(UnusedDependency {
            package_name: "dep".to_string(),
            location: DependencyLocation::Dependencies,
            path: PathBuf::from("package.json"),
        });
        results.unused_dev_dependencies.push(UnusedDependency {
            package_name: "dev".to_string(),
            location: DependencyLocation::DevDependencies,
            path: PathBuf::from("package.json"),
        });
        results.unused_enum_members.push(UnusedMember {
            path: PathBuf::from("d.ts"),
            parent_name: "E".to_string(),
            member_name: "A".to_string(),
            kind: MemberKind::EnumMember,
            line: 1,
            col: 0,
        });
        results.unused_class_members.push(UnusedMember {
            path: PathBuf::from("e.ts"),
            parent_name: "C".to_string(),
            member_name: "m".to_string(),
            kind: MemberKind::ClassMethod,
            line: 1,
            col: 0,
        });
        results.unresolved_imports.push(UnresolvedImport {
            path: PathBuf::from("f.ts"),
            specifier: "./missing".to_string(),
            line: 1,
            col: 0,
        });
        results.unlisted_dependencies.push(UnlistedDependency {
            package_name: "unlisted".to_string(),
            imported_from: vec![PathBuf::from("g.ts")],
        });
        results.duplicate_exports.push(DuplicateExport {
            export_name: "dup".to_string(),
            locations: vec![PathBuf::from("h.ts"), PathBuf::from("i.ts")],
        });

        assert_eq!(results.total_issues(), 10);
        assert!(results.has_issues());
    }
}
