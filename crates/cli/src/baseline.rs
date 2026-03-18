use std::collections::HashSet;

/// Baseline data for comparison.
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct BaselineData {
    pub unused_files: Vec<String>,
    pub unused_exports: Vec<String>,
    pub unused_types: Vec<String>,
    pub unused_dependencies: Vec<String>,
    pub unused_dev_dependencies: Vec<String>,
}

impl BaselineData {
    pub(crate) fn from_results(results: &fallow_core::results::AnalysisResults) -> Self {
        Self {
            unused_files: results
                .unused_files
                .iter()
                .map(|f| f.path.to_string_lossy().replace('\\', "/"))
                .collect(),
            unused_exports: results
                .unused_exports
                .iter()
                .map(|e| {
                    format!(
                        "{}:{}",
                        e.path.to_string_lossy().replace('\\', "/"),
                        e.export_name
                    )
                })
                .collect(),
            unused_types: results
                .unused_types
                .iter()
                .map(|e| {
                    format!(
                        "{}:{}",
                        e.path.to_string_lossy().replace('\\', "/"),
                        e.export_name
                    )
                })
                .collect(),
            unused_dependencies: results
                .unused_dependencies
                .iter()
                .map(|d| d.package_name.clone())
                .collect(),
            unused_dev_dependencies: results
                .unused_dev_dependencies
                .iter()
                .map(|d| d.package_name.clone())
                .collect(),
        }
    }
}

/// Filter results to only include issues not present in the baseline.
pub(crate) fn filter_new_issues(
    mut results: fallow_core::results::AnalysisResults,
    baseline: &BaselineData,
) -> fallow_core::results::AnalysisResults {
    let baseline_files: HashSet<&str> = baseline.unused_files.iter().map(|s| s.as_str()).collect();
    let baseline_exports: HashSet<&str> =
        baseline.unused_exports.iter().map(|s| s.as_str()).collect();
    let baseline_types: HashSet<&str> = baseline.unused_types.iter().map(|s| s.as_str()).collect();
    let baseline_deps: HashSet<&str> = baseline
        .unused_dependencies
        .iter()
        .map(|s| s.as_str())
        .collect();
    let baseline_dev_deps: HashSet<&str> = baseline
        .unused_dev_dependencies
        .iter()
        .map(|s| s.as_str())
        .collect();

    results
        .unused_files
        .retain(|f| !baseline_files.contains(f.path.to_string_lossy().replace('\\', "/").as_str()));
    results.unused_exports.retain(|e| {
        let key = format!(
            "{}:{}",
            e.path.to_string_lossy().replace('\\', "/"),
            e.export_name
        );
        !baseline_exports.contains(key.as_str())
    });
    results.unused_types.retain(|e| {
        let key = format!(
            "{}:{}",
            e.path.to_string_lossy().replace('\\', "/"),
            e.export_name
        );
        !baseline_types.contains(key.as_str())
    });
    results
        .unused_dependencies
        .retain(|d| !baseline_deps.contains(d.package_name.as_str()));
    results
        .unused_dev_dependencies
        .retain(|d| !baseline_dev_deps.contains(d.package_name.as_str()));
    results
}
