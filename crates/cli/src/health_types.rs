//! Health / complexity analysis report types.
//!
//! Separated from the `health` command module so that report formatters
//! (which are compiled as part of both the lib and bin targets) can
//! reference these types without pulling in binary-only dependencies.

/// Result of complexity analysis for reporting.
#[derive(Debug, serde::Serialize)]
pub struct HealthReport {
    /// Functions exceeding thresholds.
    pub findings: Vec<HealthFinding>,
    /// Summary statistics.
    pub summary: HealthSummary,
    /// Project-wide vital signs (always computed from available data).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vital_signs: Option<VitalSigns>,
    /// Per-file health scores (only populated with `--file-scores` or `--hotspots`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub file_scores: Vec<FileHealthScore>,
    /// Hotspot entries (only populated with `--hotspots`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hotspots: Vec<HotspotEntry>,
    /// Hotspot analysis summary (only set with `--hotspots`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hotspot_summary: Option<HotspotSummary>,
    /// Ranked refactoring recommendations (only populated with `--targets`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<RefactoringTarget>,
    /// Adaptive thresholds used for target scoring (only set with `--targets`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_thresholds: Option<TargetThresholds>,
}

/// Project-wide vital signs — a fixed set of metrics for trend tracking.
///
/// Metrics are `Option` when the data source was not available in the current run
/// (e.g., `duplication_pct` is `None` unless the duplication pipeline was run,
/// `hotspot_count` is `None` without git history).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VitalSigns {
    /// Percentage of files not reachable from any entry point.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dead_file_pct: Option<f64>,
    /// Percentage of exports never imported by other modules.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dead_export_pct: Option<f64>,
    /// Average cyclomatic complexity across all functions.
    pub avg_cyclomatic: f64,
    /// 90th percentile cyclomatic complexity.
    pub p90_cyclomatic: u32,
    /// Code duplication percentage (None if duplication pipeline was not run).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplication_pct: Option<f64>,
    /// Number of hotspot files (score >= 50). None if git history unavailable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hotspot_count: Option<u32>,
    /// Average maintainability index across all scored files (0–100).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maintainability_avg: Option<f64>,
    /// Number of unused dependencies (dependencies + devDependencies + optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unused_dep_count: Option<u32>,
    /// Number of circular dependency chains.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circular_dep_count: Option<u32>,
}

/// Raw counts backing the vital signs percentages.
///
/// Stored alongside `VitalSigns` in snapshots so that Phase 2b trend reporting
/// can decompose percentage changes into numerator vs denominator shifts.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VitalSignsCounts {
    pub total_files: usize,
    pub total_exports: usize,
    pub dead_files: usize,
    pub dead_exports: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicated_lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_scored: Option<usize>,
    pub total_deps: usize,
}

/// A point-in-time snapshot of project vital signs, persisted to disk.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VitalSignsSnapshot {
    /// Schema version for snapshot format (independent of report schema_version).
    pub snapshot_schema_version: u32,
    /// Fallow version that produced this snapshot.
    pub version: String,
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// Git commit SHA at time of snapshot (None if not in a git repo).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_sha: Option<String>,
    /// Git branch name (None if not in a git repo or detached HEAD).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
    /// Whether the repository is a shallow clone.
    #[serde(default)]
    pub shallow_clone: bool,
    /// The vital signs metrics.
    pub vital_signs: VitalSigns,
    /// Raw counts for trend decomposition.
    pub counts: VitalSignsCounts,
}

/// Current snapshot schema version. Independent of the report's SCHEMA_VERSION.
pub const SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// Hotspot score threshold for counting a file as a hotspot in vital signs.
pub const HOTSPOT_SCORE_THRESHOLD: f64 = 50.0;

/// A single function that exceeds a complexity threshold.
#[derive(Debug, serde::Serialize)]
pub struct HealthFinding {
    /// Absolute file path.
    pub path: std::path::PathBuf,
    /// Function name.
    pub name: String,
    /// 1-based line number.
    pub line: u32,
    /// 0-based column.
    pub col: u32,
    /// Cyclomatic complexity.
    pub cyclomatic: u16,
    /// Cognitive complexity.
    pub cognitive: u16,
    /// Number of lines in the function.
    pub line_count: u32,
    /// Which threshold was exceeded.
    pub exceeded: ExceededThreshold,
}

/// Which complexity threshold was exceeded.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExceededThreshold {
    /// Only cyclomatic exceeded.
    Cyclomatic,
    /// Only cognitive exceeded.
    Cognitive,
    /// Both thresholds exceeded.
    Both,
}

/// Summary statistics for the health report.
#[derive(Debug, serde::Serialize)]
pub struct HealthSummary {
    /// Number of files analyzed.
    pub files_analyzed: usize,
    /// Total number of functions found.
    pub functions_analyzed: usize,
    /// Number of functions above threshold.
    pub functions_above_threshold: usize,
    /// Configured cyclomatic threshold.
    pub max_cyclomatic_threshold: u16,
    /// Configured cognitive threshold.
    pub max_cognitive_threshold: u16,
    /// Number of files scored (only set with `--file-scores`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_scored: Option<usize>,
    /// Average maintainability index across all scored files (only set with `--file-scores`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_maintainability: Option<f64>,
}

/// Per-file health score combining complexity, coupling, and dead code metrics.
///
/// Files with zero functions (barrel files, re-export files) are excluded by default.
///
/// ## Maintainability Index Formula
///
/// ```text
/// fan_out_penalty = min(ln(fan_out + 1) × 4, 15)
/// maintainability = 100
///     - (complexity_density × 30)
///     - (dead_code_ratio × 20)
///     - fan_out_penalty
/// ```
///
/// Clamped to \[0, 100\]. Higher is better.
///
/// - **complexity_density**: total cyclomatic complexity / lines of code
/// - **dead_code_ratio**: fraction of value exports (excluding type-only exports) with zero references (0.0–1.0)
/// - **fan_out_penalty**: logarithmic scaling with cap at 15 points; reflects diminishing marginal risk of additional imports
#[derive(Debug, Clone, serde::Serialize)]
pub struct FileHealthScore {
    /// File path (absolute; stripped to relative in output).
    pub path: std::path::PathBuf,
    /// Number of files that import this file.
    pub fan_in: usize,
    /// Number of files this file imports.
    pub fan_out: usize,
    /// Fraction of value exports with zero references (0.0–1.0). Files with no value exports get 0.0.
    /// Type-only exports (interfaces, type aliases) are excluded from both numerator and denominator
    /// to avoid inflating the ratio for well-typed codebases that export props types alongside components.
    pub dead_code_ratio: f64,
    /// Total cyclomatic complexity / lines of code.
    pub complexity_density: f64,
    /// Weighted composite score (0–100, higher is better).
    pub maintainability_index: f64,
    /// Sum of cyclomatic complexity across all functions.
    pub total_cyclomatic: u32,
    /// Sum of cognitive complexity across all functions.
    pub total_cognitive: u32,
    /// Number of functions in this file.
    pub function_count: usize,
    /// Total lines of code (from line_offsets).
    pub lines: u32,
}

/// A hotspot: a file that is both complex and frequently changing.
///
/// ## Score Formula
///
/// ```text
/// normalized_churn = weighted_commits / max_weighted_commits   (0..1)
/// normalized_complexity = complexity_density / max_density      (0..1)
/// score = normalized_churn × normalized_complexity × 100       (0..100)
/// ```
///
/// Score uses within-project max normalization. Higher score = higher risk.
/// Fan-in is shown separately as "blast radius" — not baked into the score.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HotspotEntry {
    /// File path (absolute; stripped to relative in output).
    pub path: std::path::PathBuf,
    /// Hotspot score (0–100). Higher means more risk.
    pub score: f64,
    /// Number of commits in the analysis window.
    pub commits: u32,
    /// Recency-weighted commit count (exponential decay, half-life 90 days).
    pub weighted_commits: f64,
    /// Total lines added across all commits.
    pub lines_added: u32,
    /// Total lines deleted across all commits.
    pub lines_deleted: u32,
    /// Cyclomatic complexity / lines of code.
    pub complexity_density: f64,
    /// Number of files that import this file (blast radius).
    pub fan_in: usize,
    /// Churn trend: accelerating, stable, or cooling.
    pub trend: fallow_core::churn::ChurnTrend,
}

/// Summary statistics for hotspot analysis.
#[derive(Debug, serde::Serialize)]
pub struct HotspotSummary {
    /// Analysis window display string (e.g., "6 months").
    pub since: String,
    /// Minimum commits threshold.
    pub min_commits: u32,
    /// Number of files with churn data meeting the threshold.
    pub files_analyzed: usize,
    /// Number of files excluded (below min_commits).
    pub files_excluded: usize,
    /// Whether the repository is a shallow clone.
    pub shallow_clone: bool,
}

/// Adaptive thresholds used for refactoring target scoring.
///
/// Derived from the project's metric distribution (percentile-based with floors).
/// Exposed in JSON output so consumers can interpret scores in context.
#[derive(Debug, Clone, serde::Serialize)]
#[allow(clippy::struct_field_names)] // triggered in bin but not lib — #[expect] would fail in lib
pub struct TargetThresholds {
    /// Fan-in saturation point for priority formula (p95, floor 5).
    pub fan_in_p95: f64,
    /// Fan-in moderate threshold for contributing factors (p75, floor 3).
    pub fan_in_p75: f64,
    /// Fan-out saturation point for priority formula (p95, floor 8).
    pub fan_out_p95: f64,
    /// Fan-out high threshold for rules and contributing factors (p90, floor 5).
    pub fan_out_p90: usize,
}

/// Category of refactoring recommendation.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationCategory {
    /// Actively-changing file with growing complexity — highest urgency.
    UrgentChurnComplexity,
    /// File participates in an import cycle with significant blast radius.
    BreakCircularDependency,
    /// High fan-in + high complexity — changes here ripple widely.
    SplitHighImpact,
    /// Majority of exports are unused — reduce surface area.
    RemoveDeadCode,
    /// Contains functions with very high cognitive complexity.
    ExtractComplexFunctions,
    /// Excessive imports reduce testability and increase coupling.
    ExtractDependencies,
}

impl RecommendationCategory {
    /// Human-readable label for terminal output.
    pub fn label(&self) -> &'static str {
        match self {
            Self::UrgentChurnComplexity => "churn+complexity",
            Self::BreakCircularDependency => "circular dep",
            Self::SplitHighImpact => "high impact",
            Self::RemoveDeadCode => "dead code",
            Self::ExtractComplexFunctions => "complexity",
            Self::ExtractDependencies => "coupling",
        }
    }

    /// Machine-parseable label for compact output (no spaces).
    pub fn compact_label(&self) -> &'static str {
        match self {
            Self::UrgentChurnComplexity => "churn_complexity",
            Self::BreakCircularDependency => "circular_dep",
            Self::SplitHighImpact => "high_impact",
            Self::RemoveDeadCode => "dead_code",
            Self::ExtractComplexFunctions => "complexity",
            Self::ExtractDependencies => "coupling",
        }
    }
}

/// A contributing factor that triggered or strengthened a recommendation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContributingFactor {
    /// Metric name (matches JSON field names: `"fan_in"`, `"dead_code_ratio"`, etc.).
    pub metric: &'static str,
    /// Raw metric value for programmatic use.
    pub value: f64,
    /// Threshold that was exceeded.
    pub threshold: f64,
    /// Human-readable explanation.
    pub detail: String,
}

/// A ranked refactoring recommendation for a file.
///
/// ## Priority Formula
///
/// ```text
/// priority = min(density, 1) × 30 + hotspot_boost × 25 + dead_code × 20 + fan_in_norm × 15 + fan_out_norm × 10
/// ```
///
/// Fan-in and fan-out normalization uses adaptive percentile-based thresholds
/// (p95 of the project distribution, with floors) instead of fixed constants.
///
/// ## Efficiency (default sort)
///
/// ```text
/// efficiency = priority / effort_numeric   (Low=1, Medium=2, High=3)
/// ```
///
/// Surfaces quick wins: high-priority, low-effort targets rank first.
/// Effort estimate for a refactoring target.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffortEstimate {
    /// Small file, few functions, low fan-in — quick to address.
    Low,
    /// Moderate size or coupling — needs planning.
    Medium,
    /// Large file, many functions, or high fan-in — significant effort.
    High,
}

impl EffortEstimate {
    /// Human-readable label for terminal output.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    /// Numeric value for arithmetic (efficiency = priority / effort).
    pub fn numeric(&self) -> f64 {
        match self {
            Self::Low => 1.0,
            Self::Medium => 2.0,
            Self::High => 3.0,
        }
    }
}

/// Confidence level for a refactoring recommendation.
///
/// Based on the data source reliability:
/// - **High**: deterministic graph/AST analysis (dead code, circular deps, complexity)
/// - **Medium**: heuristic thresholds (fan-in/fan-out coupling)
/// - **Low**: depends on git history quality (churn-based recommendations)
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// Recommendation based on deterministic analysis (graph, AST).
    High,
    /// Recommendation based on heuristic thresholds.
    Medium,
    /// Recommendation depends on external data quality (git history).
    Low,
}

impl Confidence {
    /// Human-readable label for terminal output.
    pub fn label(&self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

/// Evidence linking a target back to specific analysis data.
///
/// Provides enough detail for an AI agent to act on a recommendation
/// without a second tool call.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TargetEvidence {
    /// Names of unused exports (populated for `RemoveDeadCode` targets).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unused_exports: Vec<String>,
    /// Complex functions with line numbers and cognitive scores (populated for `ExtractComplexFunctions`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub complex_functions: Vec<EvidenceFunction>,
    /// Files forming the import cycle (populated for `BreakCircularDependency` targets).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cycle_path: Vec<String>,
}

/// A function referenced in target evidence.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EvidenceFunction {
    /// Function name.
    pub name: String,
    /// 1-based line number.
    pub line: u32,
    /// Cognitive complexity score.
    pub cognitive: u16,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RefactoringTarget {
    /// Absolute file path (stripped to relative in output).
    pub path: std::path::PathBuf,
    /// Priority score (0–100, higher = more urgent).
    pub priority: f64,
    /// Efficiency score (priority / effort). Higher = better quick-win value.
    /// Surfaces low-effort, high-priority targets first.
    pub efficiency: f64,
    /// One-line actionable recommendation.
    pub recommendation: String,
    /// Recommendation category for tooling/filtering.
    pub category: RecommendationCategory,
    /// Estimated effort to address this target.
    pub effort: EffortEstimate,
    /// Confidence in this recommendation based on data source reliability.
    pub confidence: Confidence,
    /// Which metric values contributed to this recommendation.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub factors: Vec<ContributingFactor>,
    /// Structured evidence linking to specific analysis data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<TargetEvidence>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- RecommendationCategory ---

    #[test]
    fn category_labels_are_non_empty() {
        let categories = [
            RecommendationCategory::UrgentChurnComplexity,
            RecommendationCategory::BreakCircularDependency,
            RecommendationCategory::SplitHighImpact,
            RecommendationCategory::RemoveDeadCode,
            RecommendationCategory::ExtractComplexFunctions,
            RecommendationCategory::ExtractDependencies,
        ];
        for cat in &categories {
            assert!(!cat.label().is_empty(), "{cat:?} should have a label");
        }
    }

    #[test]
    fn category_labels_are_unique() {
        let categories = [
            RecommendationCategory::UrgentChurnComplexity,
            RecommendationCategory::BreakCircularDependency,
            RecommendationCategory::SplitHighImpact,
            RecommendationCategory::RemoveDeadCode,
            RecommendationCategory::ExtractComplexFunctions,
            RecommendationCategory::ExtractDependencies,
        ];
        let labels: Vec<&str> = categories
            .iter()
            .map(super::RecommendationCategory::label)
            .collect();
        let unique: rustc_hash::FxHashSet<&&str> = labels.iter().collect();
        assert_eq!(labels.len(), unique.len(), "category labels must be unique");
    }

    // --- Serde serialization ---

    #[test]
    fn category_serializes_as_snake_case() {
        let json = serde_json::to_string(&RecommendationCategory::UrgentChurnComplexity).unwrap();
        assert_eq!(json, r#""urgent_churn_complexity""#);

        let json = serde_json::to_string(&RecommendationCategory::BreakCircularDependency).unwrap();
        assert_eq!(json, r#""break_circular_dependency""#);
    }

    #[test]
    fn exceeded_threshold_serializes_as_snake_case() {
        let json = serde_json::to_string(&ExceededThreshold::Both).unwrap();
        assert_eq!(json, r#""both""#);

        let json = serde_json::to_string(&ExceededThreshold::Cyclomatic).unwrap();
        assert_eq!(json, r#""cyclomatic""#);
    }

    #[test]
    fn health_report_skips_empty_collections() {
        let report = HealthReport {
            findings: vec![],
            summary: HealthSummary {
                files_analyzed: 0,
                functions_analyzed: 0,
                functions_above_threshold: 0,
                max_cyclomatic_threshold: 20,
                max_cognitive_threshold: 15,
                files_scored: None,
                average_maintainability: None,
            },
            vital_signs: None,
            file_scores: vec![],
            hotspots: vec![],
            hotspot_summary: None,
            targets: vec![],
            target_thresholds: None,
        };
        let json = serde_json::to_string(&report).unwrap();
        // Empty vecs should be omitted due to skip_serializing_if
        assert!(!json.contains("file_scores"));
        assert!(!json.contains("hotspots"));
        assert!(!json.contains("hotspot_summary"));
        assert!(!json.contains("targets"));
        assert!(!json.contains("vital_signs"));
    }

    #[test]
    fn vital_signs_serialization_roundtrip() {
        let vs = VitalSigns {
            dead_file_pct: Some(3.2),
            dead_export_pct: Some(8.1),
            avg_cyclomatic: 4.7,
            p90_cyclomatic: 12,
            duplication_pct: None,
            hotspot_count: Some(5),
            maintainability_avg: Some(72.4),
            unused_dep_count: Some(4),
            circular_dep_count: Some(2),
        };
        let json = serde_json::to_string(&vs).unwrap();
        let deserialized: VitalSigns = serde_json::from_str(&json).unwrap();
        assert!((deserialized.avg_cyclomatic - 4.7).abs() < f64::EPSILON);
        assert_eq!(deserialized.p90_cyclomatic, 12);
        assert_eq!(deserialized.hotspot_count, Some(5));
        // duplication_pct should be absent in JSON and None after deser
        assert!(!json.contains("duplication_pct"));
        assert!(deserialized.duplication_pct.is_none());
    }

    #[test]
    fn vital_signs_snapshot_roundtrip() {
        let snapshot = VitalSignsSnapshot {
            snapshot_schema_version: SNAPSHOT_SCHEMA_VERSION,
            version: "1.8.1".into(),
            timestamp: "2026-03-25T14:30:00Z".into(),
            git_sha: Some("abc1234".into()),
            git_branch: Some("main".into()),
            shallow_clone: false,
            vital_signs: VitalSigns {
                dead_file_pct: Some(3.2),
                dead_export_pct: Some(8.1),
                avg_cyclomatic: 4.7,
                p90_cyclomatic: 12,
                duplication_pct: None,
                hotspot_count: None,
                maintainability_avg: Some(72.4),
                unused_dep_count: Some(4),
                circular_dep_count: Some(2),
            },
            counts: VitalSignsCounts {
                total_files: 1200,
                total_exports: 5400,
                dead_files: 38,
                dead_exports: 437,
                duplicated_lines: None,
                total_lines: None,
                files_scored: Some(1150),
                total_deps: 42,
            },
        };
        let json = serde_json::to_string_pretty(&snapshot).unwrap();
        let rt: VitalSignsSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(rt.snapshot_schema_version, SNAPSHOT_SCHEMA_VERSION);
        assert_eq!(rt.git_sha.as_deref(), Some("abc1234"));
        assert_eq!(rt.counts.total_files, 1200);
        assert_eq!(rt.counts.dead_exports, 437);
    }

    #[test]
    fn refactoring_target_skips_empty_factors() {
        let target = RefactoringTarget {
            path: std::path::PathBuf::from("/src/foo.ts"),
            priority: 75.0,
            efficiency: 75.0,
            recommendation: "Test recommendation".into(),
            category: RecommendationCategory::RemoveDeadCode,
            effort: EffortEstimate::Low,
            confidence: Confidence::High,
            factors: vec![],
            evidence: None,
        };
        let json = serde_json::to_string(&target).unwrap();
        assert!(!json.contains("factors"));
        assert!(!json.contains("evidence"));
    }

    #[test]
    fn effort_numeric_values() {
        assert!((EffortEstimate::Low.numeric() - 1.0).abs() < f64::EPSILON);
        assert!((EffortEstimate::Medium.numeric() - 2.0).abs() < f64::EPSILON);
        assert!((EffortEstimate::High.numeric() - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn confidence_labels_are_non_empty() {
        let levels = [Confidence::High, Confidence::Medium, Confidence::Low];
        for level in &levels {
            assert!(!level.label().is_empty(), "{level:?} should have a label");
        }
    }

    #[test]
    fn confidence_serializes_as_snake_case() {
        let json = serde_json::to_string(&Confidence::High).unwrap();
        assert_eq!(json, r#""high""#);
        let json = serde_json::to_string(&Confidence::Medium).unwrap();
        assert_eq!(json, r#""medium""#);
        let json = serde_json::to_string(&Confidence::Low).unwrap();
        assert_eq!(json, r#""low""#);
    }

    #[test]
    fn contributing_factor_serializes_correctly() {
        let factor = ContributingFactor {
            metric: "fan_in",
            value: 15.0,
            threshold: 10.0,
            detail: "15 files depend on this".into(),
        };
        let json = serde_json::to_string(&factor).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["metric"], "fan_in");
        assert_eq!(parsed["value"], 15.0);
        assert_eq!(parsed["threshold"], 10.0);
    }
}
