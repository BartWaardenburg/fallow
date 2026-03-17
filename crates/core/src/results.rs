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
}

impl AnalysisResults {
    /// Total number of issues found.
    pub fn total_issues(&self) -> usize {
        self.unused_files.len()
            + self.unused_exports.len()
            + self.unused_types.len()
            + self.unused_dependencies.len()
            + self.unused_dev_dependencies.len()
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
