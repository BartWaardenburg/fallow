//! Analysis result types for all issue categories.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::extract::MemberKind;
use crate::serde_path;

/// Complete analysis results.
#[derive(Debug, Default, Clone, Serialize)]
pub struct AnalysisResults {
    /// Files not reachable from any entry point.
    pub unused_files: Vec<UnusedFile>,
    /// Exports never imported by other modules.
    pub unused_exports: Vec<UnusedExport>,
    /// Type exports never imported by other modules.
    pub unused_types: Vec<UnusedExport>,
    /// Dependencies listed in package.json but never imported.
    pub unused_dependencies: Vec<UnusedDependency>,
    /// Dev dependencies listed in package.json but never imported.
    pub unused_dev_dependencies: Vec<UnusedDependency>,
    /// Optional dependencies listed in package.json but never imported.
    pub unused_optional_dependencies: Vec<UnusedDependency>,
    /// Enum members never accessed.
    pub unused_enum_members: Vec<UnusedMember>,
    /// Class members never accessed.
    pub unused_class_members: Vec<UnusedMember>,
    /// Import specifiers that could not be resolved.
    pub unresolved_imports: Vec<UnresolvedImport>,
    /// Dependencies used in code but not listed in package.json.
    pub unlisted_dependencies: Vec<UnlistedDependency>,
    /// Exports with the same name across multiple modules.
    pub duplicate_exports: Vec<DuplicateExport>,
    /// Production dependencies only used via type-only imports (could be devDependencies).
    /// Only populated in production mode.
    pub type_only_dependencies: Vec<TypeOnlyDependency>,
    /// Circular dependency chains detected in the module graph.
    pub circular_dependencies: Vec<CircularDependency>,
    /// Usage counts for all exports across the project. Used by the LSP for Code Lens.
    /// Not included in issue counts -- this is metadata, not an issue type.
    /// Skipped during serialization: this is internal LSP data, not part of the JSON output schema.
    #[serde(skip)]
    pub export_usages: Vec<ExportUsage>,
}

impl AnalysisResults {
    /// Total number of issues found.
    pub const fn total_issues(&self) -> usize {
        self.unused_files.len()
            + self.unused_exports.len()
            + self.unused_types.len()
            + self.unused_dependencies.len()
            + self.unused_dev_dependencies.len()
            + self.unused_optional_dependencies.len()
            + self.unused_enum_members.len()
            + self.unused_class_members.len()
            + self.unresolved_imports.len()
            + self.unlisted_dependencies.len()
            + self.duplicate_exports.len()
            + self.type_only_dependencies.len()
            + self.circular_dependencies.len()
    }

    /// Whether any issues were found.
    pub const fn has_issues(&self) -> bool {
        self.total_issues() > 0
    }
}

/// A file that is not reachable from any entry point.
#[derive(Debug, Clone, Serialize)]
pub struct UnusedFile {
    /// Absolute path to the unused file.
    #[serde(serialize_with = "serde_path::serialize")]
    pub path: PathBuf,
}

/// An export that is never imported by other modules.
#[derive(Debug, Clone, Serialize)]
pub struct UnusedExport {
    /// File containing the unused export.
    #[serde(serialize_with = "serde_path::serialize")]
    pub path: PathBuf,
    /// Name of the unused export.
    pub export_name: String,
    /// Whether this is a type-only export.
    pub is_type_only: bool,
    /// 1-based line number of the export.
    pub line: u32,
    /// 0-based byte column offset.
    pub col: u32,
    /// Byte offset into the source file (used by the fix command).
    pub span_start: u32,
    /// Whether this finding comes from a barrel/index re-export rather than the source definition.
    pub is_re_export: bool,
}

/// A dependency that is listed in package.json but never imported.
#[derive(Debug, Clone, Serialize)]
pub struct UnusedDependency {
    /// npm package name.
    pub package_name: String,
    /// Whether this is in `dependencies`, `devDependencies`, or `optionalDependencies`.
    pub location: DependencyLocation,
    /// Path to the package.json where this dependency is listed.
    /// For root deps this is `<root>/package.json`, for workspace deps it is `<ws>/package.json`.
    #[serde(serialize_with = "serde_path::serialize")]
    pub path: PathBuf,
}

/// Where in package.json a dependency is listed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DependencyLocation {
    /// Listed in `dependencies`.
    Dependencies,
    /// Listed in `devDependencies`.
    DevDependencies,
    /// Listed in `optionalDependencies`.
    OptionalDependencies,
}

/// An unused enum or class member.
#[derive(Debug, Clone, Serialize)]
pub struct UnusedMember {
    /// File containing the unused member.
    #[serde(serialize_with = "serde_path::serialize")]
    pub path: PathBuf,
    /// Name of the parent enum or class.
    pub parent_name: String,
    /// Name of the unused member.
    pub member_name: String,
    /// Whether this is an enum member, class method, or class property.
    pub kind: MemberKind,
    /// 1-based line number.
    pub line: u32,
    /// 0-based byte column offset.
    pub col: u32,
}

/// An import that could not be resolved.
#[derive(Debug, Clone, Serialize)]
pub struct UnresolvedImport {
    /// File containing the unresolved import.
    #[serde(serialize_with = "serde_path::serialize")]
    pub path: PathBuf,
    /// The import specifier that could not be resolved.
    pub specifier: String,
    /// 1-based line number.
    pub line: u32,
    /// 0-based byte column offset.
    pub col: u32,
}

/// A dependency used in code but not listed in package.json.
#[derive(Debug, Clone, Serialize)]
pub struct UnlistedDependency {
    /// npm package name.
    pub package_name: String,
    /// Files that import this unlisted dependency.
    #[serde(serialize_with = "serde_path::serialize_vec")]
    pub imported_from: Vec<PathBuf>,
}

/// An export that appears multiple times across the project.
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateExport {
    /// The duplicated export name.
    pub export_name: String,
    /// Locations where this export name appears.
    pub locations: Vec<DuplicateLocation>,
}

/// A location where a duplicate export appears.
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateLocation {
    /// File containing the duplicate export.
    #[serde(serialize_with = "serde_path::serialize")]
    pub path: PathBuf,
    /// 1-based line number.
    pub line: u32,
    /// 0-based byte column offset.
    pub col: u32,
}

/// A production dependency that is only used via type-only imports.
/// In production builds, type imports are erased, so this dependency
/// is not needed at runtime and could be moved to devDependencies.
#[derive(Debug, Clone, Serialize)]
pub struct TypeOnlyDependency {
    /// npm package name.
    pub package_name: String,
    /// Path to the package.json where the dependency is listed.
    #[serde(serialize_with = "serde_path::serialize")]
    pub path: PathBuf,
}

/// A circular dependency chain detected in the module graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircularDependency {
    /// Files forming the cycle, in import order.
    #[serde(serialize_with = "serde_path::serialize_vec")]
    pub files: Vec<PathBuf>,
    /// Number of files in the cycle.
    pub length: usize,
}

/// Usage count for an export symbol. Used by the LSP Code Lens to show
/// reference counts above each export declaration.
#[derive(Debug, Clone, Serialize)]
pub struct ExportUsage {
    /// File containing the export.
    #[serde(serialize_with = "serde_path::serialize")]
    pub path: PathBuf,
    /// Name of the exported symbol.
    pub export_name: String,
    /// 1-based line number.
    pub line: u32,
    /// 0-based byte column offset.
    pub col: u32,
    /// Number of files that reference this export.
    pub reference_count: usize,
    /// Locations where this export is referenced. Used by the LSP Code Lens
    /// to enable click-to-navigate via `editor.action.showReferences`.
    pub reference_locations: Vec<ReferenceLocation>,
}

/// A location where an export is referenced (import site in another file).
#[derive(Debug, Clone, Serialize)]
pub struct ReferenceLocation {
    /// File containing the import that references the export.
    #[serde(serialize_with = "serde_path::serialize")]
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
            locations: vec![
                DuplicateLocation {
                    path: PathBuf::from("h.ts"),
                    line: 15,
                    col: 0,
                },
                DuplicateLocation {
                    path: PathBuf::from("i.ts"),
                    line: 30,
                    col: 0,
                },
            ],
        });
        results.circular_dependencies.push(CircularDependency {
            files: vec![PathBuf::from("a.ts"), PathBuf::from("b.ts")],
            length: 2,
        });

        assert_eq!(results.total_issues(), 11);
        assert!(results.has_issues());
    }
}
