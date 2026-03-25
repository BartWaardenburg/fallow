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
}

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
    /// Human-readable label for terminal and compact output.
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
/// priority = min(complexity_density, 1) × 30
///          + hotspot_boost × 25            (hotspot_score / 100 if in hotspots, else 0)
///          + dead_code_ratio × 20
///          + fan_in_norm × 15              (min(fan_in / 20, 1.0))
///          + fan_out_norm × 10             (min(fan_out / 30, 1.0))
/// ```
///
/// All inputs clamped to \[0, 1\] so each weight is a true percentage share.
/// Clamped to \[0, 100\]. Higher is more urgent. Does not use the maintainability
/// index to avoid double-counting (MI already incorporates density and dead code).
#[derive(Debug, Clone, serde::Serialize)]
pub struct RefactoringTarget {
    /// Absolute file path (stripped to relative in output).
    pub path: std::path::PathBuf,
    /// Priority score (0–100, higher = more urgent).
    pub priority: f64,
    /// One-line actionable recommendation.
    pub recommendation: String,
    /// Recommendation category for tooling/filtering.
    pub category: RecommendationCategory,
    /// Which metric values contributed to this recommendation.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub factors: Vec<ContributingFactor>,
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
        let labels: Vec<&str> = categories.iter().map(|c| c.label()).collect();
        let unique: std::collections::HashSet<&&str> = labels.iter().collect();
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
            file_scores: vec![],
            hotspots: vec![],
            hotspot_summary: None,
            targets: vec![],
        };
        let json = serde_json::to_string(&report).unwrap();
        // Empty vecs should be omitted due to skip_serializing_if
        assert!(!json.contains("file_scores"));
        assert!(!json.contains("hotspots"));
        assert!(!json.contains("hotspot_summary"));
        assert!(!json.contains("targets"));
    }

    #[test]
    fn refactoring_target_skips_empty_factors() {
        let target = RefactoringTarget {
            path: std::path::PathBuf::from("/src/foo.ts"),
            priority: 75.0,
            recommendation: "Test recommendation".into(),
            category: RecommendationCategory::RemoveDeadCode,
            factors: vec![],
        };
        let json = serde_json::to_string(&target).unwrap();
        assert!(!json.contains("factors"));
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
