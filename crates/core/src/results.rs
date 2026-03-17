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
