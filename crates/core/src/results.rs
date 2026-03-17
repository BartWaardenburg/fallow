use std::path::PathBuf;

use serde::Serialize;

/// Complete analysis results.
#[derive(Debug, Default, Serialize)]
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
    }

    /// Whether any issues were found.
    pub fn has_issues(&self) -> bool {
        self.total_issues() > 0
    }
}

/// A file that is not reachable from any entry point.
#[derive(Debug, Serialize)]
pub struct UnusedFile {
    pub path: PathBuf,
}

/// An export that is never imported by other modules.
#[derive(Debug, Serialize)]
pub struct UnusedExport {
    pub path: PathBuf,
    pub export_name: String,
    pub is_type_only: bool,
    pub line: u32,
    pub col: u32,
    /// Byte offset into the source file (used by the fix command).
    pub span_start: u32,
}

/// A dependency that is listed in package.json but never imported.
#[derive(Debug, Serialize)]
pub struct UnusedDependency {
    pub package_name: String,
    pub location: DependencyLocation,
}

/// Where in package.json a dependency is listed.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DependencyLocation {
    Dependencies,
    DevDependencies,
}

/// An unused enum or class member.
#[derive(Debug, Serialize)]
pub struct UnusedMember {
    pub path: PathBuf,
    pub parent_name: String,
    pub member_name: String,
    pub kind: String,
    pub line: u32,
    pub col: u32,
}

/// An import that could not be resolved.
#[derive(Debug, Serialize)]
pub struct UnresolvedImport {
    pub path: PathBuf,
    pub specifier: String,
    pub line: u32,
    pub col: u32,
}

/// A dependency used in code but not listed in package.json.
#[derive(Debug, Serialize)]
pub struct UnlistedDependency {
    pub package_name: String,
    pub imported_from: Vec<PathBuf>,
}

/// An export that appears multiple times across the project.
#[derive(Debug, Serialize)]
pub struct DuplicateExport {
    pub export_name: String,
    pub locations: Vec<PathBuf>,
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
        });
        results.unused_types.push(UnusedExport {
            path: PathBuf::from("c.ts"),
            export_name: "T".to_string(),
            is_type_only: true,
            line: 1,
            col: 0,
            span_start: 0,
        });
        results.unused_dependencies.push(UnusedDependency {
            package_name: "dep".to_string(),
            location: DependencyLocation::Dependencies,
        });
        results.unused_dev_dependencies.push(UnusedDependency {
            package_name: "dev".to_string(),
            location: DependencyLocation::DevDependencies,
        });
        results.unused_enum_members.push(UnusedMember {
            path: PathBuf::from("d.ts"),
            parent_name: "E".to_string(),
            member_name: "A".to_string(),
            kind: "enum_member".to_string(),
            line: 1,
            col: 0,
        });
        results.unused_class_members.push(UnusedMember {
            path: PathBuf::from("e.ts"),
            parent_name: "C".to_string(),
            member_name: "m".to_string(),
            kind: "class_method".to_string(),
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
