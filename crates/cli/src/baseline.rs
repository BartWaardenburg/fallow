use rustc_hash::{FxHashMap, FxHashSet};
use std::path::Path;

use fallow_core::duplicates::DuplicationReport;

/// Baseline data for comparison.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct BaselineData {
    pub unused_files: Vec<String>,
    pub unused_exports: Vec<String>,
    pub unused_types: Vec<String>,
    pub unused_dependencies: Vec<String>,
    pub unused_dev_dependencies: Vec<String>,
    /// Circular dependency chains, keyed by sorted file paths joined with `->`.
    #[serde(default)]
    pub circular_dependencies: Vec<String>,
}

impl BaselineData {
    pub fn from_results(results: &fallow_core::results::AnalysisResults) -> Self {
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
            circular_dependencies: results
                .circular_dependencies
                .iter()
                .map(circular_dep_key)
                .collect(),
        }
    }
}

/// Generate a stable key for a circular dependency based on sorted file paths.
fn circular_dep_key(dep: &fallow_core::results::CircularDependency) -> String {
    let mut paths: Vec<String> = dep
        .files
        .iter()
        .map(|f| f.to_string_lossy().replace('\\', "/"))
        .collect();
    paths.sort();
    paths.join("->")
}

/// Filter results to only include issues not present in the baseline.
pub fn filter_new_issues(
    mut results: fallow_core::results::AnalysisResults,
    baseline: &BaselineData,
) -> fallow_core::results::AnalysisResults {
    let baseline_files: FxHashSet<&str> =
        baseline.unused_files.iter().map(|s| s.as_str()).collect();
    let baseline_exports: FxHashSet<&str> =
        baseline.unused_exports.iter().map(|s| s.as_str()).collect();
    let baseline_types: FxHashSet<&str> =
        baseline.unused_types.iter().map(|s| s.as_str()).collect();
    let baseline_deps: FxHashSet<&str> = baseline
        .unused_dependencies
        .iter()
        .map(|s| s.as_str())
        .collect();
    let baseline_dev_deps: FxHashSet<&str> = baseline
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

    let baseline_circular: FxHashSet<&str> = baseline
        .circular_dependencies
        .iter()
        .map(|s| s.as_str())
        .collect();
    results.circular_dependencies.retain(|c| {
        let key = circular_dep_key(c);
        !baseline_circular.contains(key.as_str())
    });

    results
}

/// Baseline data for duplication comparison.
///
/// Each clone group is keyed by a canonical string derived from its sorted
/// (`file:start_line-end_line`) instance locations. This allows stable comparison
/// across runs even if group ordering changes.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct DuplicationBaselineData {
    /// Clone group keys: sorted list of `file:start-end` per group.
    pub clone_groups: Vec<String>,
}

impl DuplicationBaselineData {
    /// Build a duplication baseline from the current report.
    pub fn from_report(report: &DuplicationReport, root: &Path) -> Self {
        Self {
            clone_groups: report
                .clone_groups
                .iter()
                .map(|g| clone_group_key(g, root))
                .collect(),
        }
    }
}

/// Generate a stable key for a clone group based on its instance locations.
fn clone_group_key(group: &fallow_core::duplicates::CloneGroup, root: &Path) -> String {
    let mut parts: Vec<String> = group
        .instances
        .iter()
        .map(|i| {
            let relative = i
                .file
                .strip_prefix(root)
                .unwrap_or(&i.file)
                .to_string_lossy()
                .replace('\\', "/");
            format!("{}:{}-{}", relative, i.start_line, i.end_line)
        })
        .collect();
    parts.sort();
    parts.join("|")
}

/// Filter a duplication report to only include clone groups not present in the baseline.
pub fn filter_new_clone_groups(
    mut report: DuplicationReport,
    baseline: &DuplicationBaselineData,
    root: &Path,
) -> DuplicationReport {
    let baseline_keys: FxHashSet<&str> = baseline.clone_groups.iter().map(|s| s.as_str()).collect();

    report.clone_groups.retain(|g| {
        let key = clone_group_key(g, root);
        !baseline_keys.contains(key.as_str())
    });

    // Re-generate families from the filtered groups
    report.clone_families =
        fallow_core::duplicates::families::group_into_families(&report.clone_groups);

    // Re-compute stats for the filtered groups
    report.stats = recompute_stats(&report);

    report
}

/// Recompute duplication statistics after filtering (baseline or `--changed-since`).
///
/// Uses per-file line deduplication (matching `compute_stats` in `detect.rs`)
/// so overlapping clone instances don't inflate the duplicated line count.
pub fn recompute_stats(report: &DuplicationReport) -> fallow_core::duplicates::DuplicationStats {
    let mut files_with_clones: FxHashSet<&Path> = FxHashSet::default();
    let mut file_dup_lines: FxHashMap<&Path, FxHashSet<usize>> = FxHashMap::default();
    let mut duplicated_tokens = 0usize;
    let mut clone_instances = 0usize;

    for group in &report.clone_groups {
        for instance in &group.instances {
            files_with_clones.insert(&instance.file);
            clone_instances += 1;
            let lines = file_dup_lines.entry(&instance.file).or_default();
            for line in instance.start_line..=instance.end_line {
                lines.insert(line);
            }
        }
        duplicated_tokens += group.token_count * group.instances.len();
    }

    let duplicated_lines: usize = file_dup_lines.values().map(FxHashSet::len).sum();

    fallow_core::duplicates::DuplicationStats {
        total_files: report.stats.total_files,
        files_with_clones: files_with_clones.len(),
        total_lines: report.stats.total_lines,
        duplicated_lines,
        total_tokens: report.stats.total_tokens,
        duplicated_tokens,
        clone_groups: report.clone_groups.len(),
        clone_instances,
        duplication_percentage: if report.stats.total_lines > 0 {
            (duplicated_lines as f64 / report.stats.total_lines as f64) * 100.0
        } else {
            0.0
        },
    }
}

// ── Health baseline ─────────────────────────────────────────────────

/// Baseline data for health (complexity) comparison.
///
/// Each finding is keyed by `relative_path:function_name:line` for stable comparison.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct HealthBaselineData {
    pub findings: Vec<String>,
}

impl HealthBaselineData {
    /// Build a health baseline from a list of findings.
    pub fn from_findings(findings: &[crate::health_types::HealthFinding], root: &Path) -> Self {
        Self {
            findings: findings
                .iter()
                .map(|f| health_finding_key(f, root))
                .collect(),
        }
    }
}

/// Generate a stable key for a health finding.
fn health_finding_key(finding: &crate::health_types::HealthFinding, root: &Path) -> String {
    let relative = finding
        .path
        .strip_prefix(root)
        .unwrap_or(&finding.path)
        .to_string_lossy()
        .replace('\\', "/");
    format!("{}:{}:{}", relative, finding.name, finding.line)
}

/// Filter health findings to only include those not present in the baseline.
pub fn filter_new_health_findings(
    mut findings: Vec<crate::health_types::HealthFinding>,
    baseline: &HealthBaselineData,
    root: &Path,
) -> Vec<crate::health_types::HealthFinding> {
    let baseline_keys: FxHashSet<&str> = baseline.findings.iter().map(|s| s.as_str()).collect();
    findings.retain(|f| {
        let key = health_finding_key(f, root);
        !baseline_keys.contains(key.as_str())
    });
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use fallow_core::duplicates::{CloneGroup, CloneInstance, DuplicationReport, DuplicationStats};
    use fallow_core::results::{
        AnalysisResults, DependencyLocation, UnusedDependency, UnusedExport, UnusedFile,
    };
    use std::path::PathBuf;

    fn make_results() -> AnalysisResults {
        AnalysisResults {
            unused_files: vec![
                UnusedFile {
                    path: PathBuf::from("src/old.ts"),
                },
                UnusedFile {
                    path: PathBuf::from("src/dead.ts"),
                },
            ],
            unused_exports: vec![UnusedExport {
                path: PathBuf::from("src/utils.ts"),
                export_name: "helperA".to_string(),
                is_type_only: false,
                line: 5,
                col: 0,
                span_start: 40,
                is_re_export: false,
            }],
            unused_types: vec![UnusedExport {
                path: PathBuf::from("src/types.ts"),
                export_name: "OldType".to_string(),
                is_type_only: true,
                line: 10,
                col: 0,
                span_start: 100,
                is_re_export: false,
            }],
            unused_dependencies: vec![UnusedDependency {
                package_name: "lodash".to_string(),
                location: DependencyLocation::Dependencies,
                path: PathBuf::from("package.json"),
                line: 5,
            }],
            unused_dev_dependencies: vec![UnusedDependency {
                package_name: "jest".to_string(),
                location: DependencyLocation::DevDependencies,
                path: PathBuf::from("package.json"),
                line: 5,
            }],
            ..Default::default()
        }
    }

    // ── BaselineData round-trip ──────────────────────────────────

    #[test]
    fn baseline_from_results_captures_all_fields() {
        let results = make_results();
        let baseline = BaselineData::from_results(&results);
        assert_eq!(baseline.unused_files.len(), 2);
        assert!(baseline.unused_files.contains(&"src/old.ts".to_string()));
        assert!(baseline.unused_files.contains(&"src/dead.ts".to_string()));
        assert_eq!(baseline.unused_exports, vec!["src/utils.ts:helperA"]);
        assert_eq!(baseline.unused_types, vec!["src/types.ts:OldType"]);
        assert_eq!(baseline.unused_dependencies, vec!["lodash"]);
        assert_eq!(baseline.unused_dev_dependencies, vec!["jest"]);
    }

    #[test]
    fn baseline_serialization_roundtrip() {
        let results = make_results();
        let baseline = BaselineData::from_results(&results);
        let json = serde_json::to_string(&baseline).unwrap();
        let deserialized: BaselineData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.unused_files, baseline.unused_files);
        assert_eq!(deserialized.unused_exports, baseline.unused_exports);
        assert_eq!(deserialized.unused_types, baseline.unused_types);
        assert_eq!(
            deserialized.unused_dependencies,
            baseline.unused_dependencies
        );
        assert_eq!(
            deserialized.unused_dev_dependencies,
            baseline.unused_dev_dependencies
        );
    }

    // ── filter_new_issues ────────────────────────────────────────

    #[test]
    fn filter_removes_baseline_issues() {
        let results = make_results();
        let baseline = BaselineData::from_results(&results);
        let filtered = filter_new_issues(results, &baseline);
        assert!(
            filtered.unused_files.is_empty(),
            "all files were in baseline"
        );
        assert!(
            filtered.unused_exports.is_empty(),
            "all exports were in baseline"
        );
        assert!(
            filtered.unused_types.is_empty(),
            "all types were in baseline"
        );
        assert!(
            filtered.unused_dependencies.is_empty(),
            "all deps were in baseline"
        );
        assert!(
            filtered.unused_dev_dependencies.is_empty(),
            "all dev deps were in baseline"
        );
    }

    #[test]
    fn filter_keeps_new_issues_not_in_baseline() {
        let baseline = BaselineData {
            unused_files: vec!["src/old.ts".to_string()],
            unused_exports: vec![],
            unused_types: vec![],
            unused_dependencies: vec![],
            unused_dev_dependencies: vec![],
            circular_dependencies: vec![],
        };
        let results = AnalysisResults {
            unused_files: vec![
                UnusedFile {
                    path: PathBuf::from("src/old.ts"),
                },
                UnusedFile {
                    path: PathBuf::from("src/new-dead.ts"),
                },
            ],
            ..Default::default()
        };
        let filtered = filter_new_issues(results, &baseline);
        assert_eq!(filtered.unused_files.len(), 1);
        assert_eq!(
            filtered.unused_files[0].path,
            PathBuf::from("src/new-dead.ts")
        );
    }

    #[test]
    fn filter_with_empty_baseline_keeps_all() {
        let baseline = BaselineData {
            unused_files: vec![],
            unused_exports: vec![],
            unused_types: vec![],
            unused_dependencies: vec![],
            unused_dev_dependencies: vec![],
            circular_dependencies: vec![],
        };
        let results = make_results();
        let filtered = filter_new_issues(results, &baseline);
        assert_eq!(filtered.unused_files.len(), 2);
        assert_eq!(filtered.unused_exports.len(), 1);
    }

    #[test]
    fn filter_new_exports_by_file_and_name() {
        let baseline = BaselineData {
            unused_files: vec![],
            unused_exports: vec!["src/utils.ts:helperA".to_string()],
            unused_types: vec![],
            unused_dependencies: vec![],
            unused_dev_dependencies: vec![],
            circular_dependencies: vec![],
        };
        let results = AnalysisResults {
            unused_exports: vec![
                UnusedExport {
                    path: PathBuf::from("src/utils.ts"),
                    export_name: "helperA".to_string(),
                    is_type_only: false,
                    line: 5,
                    col: 0,
                    span_start: 40,
                    is_re_export: false,
                },
                UnusedExport {
                    path: PathBuf::from("src/utils.ts"),
                    export_name: "helperB".to_string(),
                    is_type_only: false,
                    line: 10,
                    col: 0,
                    span_start: 80,
                    is_re_export: false,
                },
            ],
            ..Default::default()
        };
        let filtered = filter_new_issues(results, &baseline);
        assert_eq!(filtered.unused_exports.len(), 1);
        assert_eq!(filtered.unused_exports[0].export_name, "helperB");
    }

    // ── DuplicationBaselineData ──────────────────────────────────

    fn make_clone_group(instances: Vec<(&str, usize, usize)>) -> CloneGroup {
        CloneGroup {
            instances: instances
                .into_iter()
                .map(|(file, start, end)| CloneInstance {
                    file: PathBuf::from(file),
                    start_line: start,
                    end_line: end,
                    start_col: 0,
                    end_col: 0,
                    fragment: String::new(),
                })
                .collect(),
            token_count: 50,
            line_count: 10,
        }
    }

    fn make_duplication_report(groups: Vec<CloneGroup>) -> DuplicationReport {
        DuplicationReport {
            clone_groups: groups,
            clone_families: vec![],
            stats: DuplicationStats {
                total_files: 10,
                files_with_clones: 2,
                total_lines: 1000,
                duplicated_lines: 100,
                total_tokens: 5000,
                duplicated_tokens: 500,
                clone_groups: 1,
                clone_instances: 2,
                duplication_percentage: 10.0,
            },
        }
    }

    #[test]
    fn clone_group_key_is_deterministic() {
        let root = Path::new("/project");
        let group = make_clone_group(vec![
            ("/project/src/a.ts", 1, 10),
            ("/project/src/b.ts", 5, 15),
        ]);
        let key1 = clone_group_key(&group, root);
        let key2 = clone_group_key(&group, root);
        assert_eq!(key1, key2);
    }

    #[test]
    fn clone_group_key_is_sorted() {
        let root = Path::new("/project");
        // Order of instances in group shouldn't matter for the key
        let group_ab = make_clone_group(vec![
            ("/project/src/a.ts", 1, 10),
            ("/project/src/b.ts", 5, 15),
        ]);
        let group_ba = make_clone_group(vec![
            ("/project/src/b.ts", 5, 15),
            ("/project/src/a.ts", 1, 10),
        ]);
        assert_eq!(
            clone_group_key(&group_ab, root),
            clone_group_key(&group_ba, root),
            "key should be stable regardless of instance order"
        );
    }

    #[test]
    fn duplication_baseline_roundtrip() {
        let root = Path::new("/project");
        let group = make_clone_group(vec![
            ("/project/src/a.ts", 1, 10),
            ("/project/src/b.ts", 5, 15),
        ]);
        let report = make_duplication_report(vec![group]);
        let baseline = DuplicationBaselineData::from_report(&report, root);
        let json = serde_json::to_string(&baseline).unwrap();
        let deserialized: DuplicationBaselineData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.clone_groups, baseline.clone_groups);
    }

    #[test]
    fn filter_new_clone_groups_removes_baseline() {
        let root = Path::new("/project");
        let group = make_clone_group(vec![
            ("/project/src/a.ts", 1, 10),
            ("/project/src/b.ts", 5, 15),
        ]);
        let report = make_duplication_report(vec![group.clone()]);
        let baseline = DuplicationBaselineData::from_report(&report, root);
        let filtered = filter_new_clone_groups(report, &baseline, root);
        assert!(
            filtered.clone_groups.is_empty(),
            "baseline group should be filtered out"
        );
    }

    #[test]
    fn filter_new_clone_groups_keeps_new_groups() {
        let root = Path::new("/project");
        let baseline_group = make_clone_group(vec![
            ("/project/src/a.ts", 1, 10),
            ("/project/src/b.ts", 5, 15),
        ]);
        let new_group = make_clone_group(vec![
            ("/project/src/c.ts", 20, 30),
            ("/project/src/d.ts", 25, 35),
        ]);
        let baseline_report = make_duplication_report(vec![baseline_group]);
        let baseline = DuplicationBaselineData::from_report(&baseline_report, root);

        let report = make_duplication_report(vec![
            make_clone_group(vec![
                ("/project/src/a.ts", 1, 10),
                ("/project/src/b.ts", 5, 15),
            ]),
            new_group,
        ]);
        let filtered = filter_new_clone_groups(report, &baseline, root);
        assert_eq!(
            filtered.clone_groups.len(),
            1,
            "only the new group should remain"
        );
    }

    #[test]
    fn recompute_stats_after_filtering() {
        let root = Path::new("/project");
        let group = make_clone_group(vec![
            ("/project/src/a.ts", 1, 10),
            ("/project/src/b.ts", 5, 15),
        ]);
        let report = make_duplication_report(vec![group]);
        let baseline = DuplicationBaselineData::from_report(&report, root);
        let filtered = filter_new_clone_groups(report, &baseline, root);
        assert_eq!(filtered.stats.clone_groups, 0);
        assert_eq!(filtered.stats.clone_instances, 0);
        assert_eq!(filtered.stats.duplicated_lines, 0);
    }

    #[test]
    fn recompute_stats_zero_total_lines() {
        let report = DuplicationReport {
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
        };
        let stats = super::recompute_stats(&report);
        assert_eq!(stats.duplication_percentage, 0.0);
    }

    // ── HealthBaselineData ──────────────────────────────────────────

    fn make_health_finding(
        root: &Path,
        name: &str,
        line: u32,
    ) -> crate::health_types::HealthFinding {
        crate::health_types::HealthFinding {
            path: root.join("src/utils.ts"),
            name: name.to_string(),
            line,
            col: 0,
            cyclomatic: 25,
            cognitive: 30,
            line_count: 80,
            exceeded: crate::health_types::ExceededThreshold::Both,
        }
    }

    #[test]
    fn health_baseline_roundtrip() {
        let root = PathBuf::from("/project");
        let findings = vec![make_health_finding(&root, "parseExpression", 42)];
        let baseline = HealthBaselineData::from_findings(&findings, &root);
        let json = serde_json::to_string(&baseline).unwrap();
        let deserialized: HealthBaselineData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.findings, baseline.findings);
        assert_eq!(baseline.findings, vec!["src/utils.ts:parseExpression:42"]);
    }

    #[test]
    fn health_baseline_filters_known_findings() {
        let root = PathBuf::from("/project");
        let findings = vec![
            make_health_finding(&root, "parseExpression", 42),
            make_health_finding(&root, "newFunction", 100),
        ];
        let baseline = HealthBaselineData::from_findings(&findings[..1], &root);
        let filtered = filter_new_health_findings(findings, &baseline, &root);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "newFunction");
    }

    #[test]
    fn health_baseline_empty_keeps_all() {
        let root = PathBuf::from("/project");
        let findings = vec![make_health_finding(&root, "parseExpression", 42)];
        let baseline = HealthBaselineData { findings: vec![] };
        let filtered = filter_new_health_findings(findings, &baseline, &root);
        assert_eq!(filtered.len(), 1);
    }

    // ── circular_dep_key sort stability ─────────────────────────

    #[test]
    fn circular_dep_key_is_order_independent() {
        use fallow_core::results::CircularDependency;

        let dep_ab = CircularDependency {
            files: vec![PathBuf::from("src/a.ts"), PathBuf::from("src/b.ts")],
            length: 2,
            line: 1,
            col: 0,
        };
        let dep_ba = CircularDependency {
            files: vec![PathBuf::from("src/b.ts"), PathBuf::from("src/a.ts")],
            length: 2,
            line: 1,
            col: 0,
        };
        assert_eq!(
            super::circular_dep_key(&dep_ab),
            super::circular_dep_key(&dep_ba),
            "same files in different order should produce identical keys"
        );
    }

    #[test]
    fn circular_dep_key_different_files_different_keys() {
        use fallow_core::results::CircularDependency;

        let dep1 = CircularDependency {
            files: vec![PathBuf::from("src/a.ts"), PathBuf::from("src/b.ts")],
            length: 2,
            line: 1,
            col: 0,
        };
        let dep2 = CircularDependency {
            files: vec![PathBuf::from("src/a.ts"), PathBuf::from("src/c.ts")],
            length: 2,
            line: 1,
            col: 0,
        };
        assert_ne!(
            super::circular_dep_key(&dep1),
            super::circular_dep_key(&dep2),
        );
    }

    #[test]
    fn circular_dep_key_three_files_order_independent() {
        use fallow_core::results::CircularDependency;

        let dep_abc = CircularDependency {
            files: vec![
                PathBuf::from("src/a.ts"),
                PathBuf::from("src/b.ts"),
                PathBuf::from("src/c.ts"),
            ],
            length: 3,
            line: 1,
            col: 0,
        };
        let dep_cab = CircularDependency {
            files: vec![
                PathBuf::from("src/c.ts"),
                PathBuf::from("src/a.ts"),
                PathBuf::from("src/b.ts"),
            ],
            length: 3,
            line: 1,
            col: 0,
        };
        assert_eq!(
            super::circular_dep_key(&dep_abc),
            super::circular_dep_key(&dep_cab),
        );
    }
}
