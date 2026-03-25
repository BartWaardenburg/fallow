use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
pub struct AnalyzeParams {
    /// Root directory of the project to analyze. Defaults to current working directory.
    pub root: Option<String>,

    /// Path to fallow config file (.fallowrc.json or fallow.toml).
    pub config: Option<String>,

    /// Only analyze production code (excludes tests, stories, dev files).
    pub production: Option<bool>,

    /// Scope analysis to a specific workspace package name.
    pub workspace: Option<String>,

    /// Issue types to include. When set, only these types are reported.
    /// Valid values: unused-files, unused-exports, unused-types, unused-deps,
    /// unused-enum-members, unused-class-members, unresolved-imports,
    /// unlisted-deps, duplicate-exports, circular-deps.
    pub issue_types: Option<Vec<String>>,
}

#[derive(Deserialize, JsonSchema)]
pub struct CheckChangedParams {
    /// Root directory of the project to analyze. Defaults to current working directory.
    pub root: Option<String>,

    /// Git ref to compare against (e.g., "main", "HEAD~5", a commit SHA).
    /// Only files changed since this ref are reported.
    pub since: String,

    /// Path to fallow config file.
    pub config: Option<String>,

    /// Only analyze production code.
    pub production: Option<bool>,

    /// Scope analysis to a specific workspace package name.
    pub workspace: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct FindDupesParams {
    /// Root directory of the project to analyze. Defaults to current working directory.
    pub root: Option<String>,

    /// Detection mode: "strict" (exact tokens), "mild" (normalized identifiers),
    /// "weak" (structural only), or "semantic" (type-aware). Defaults to "mild".
    pub mode: Option<String>,

    /// Minimum token count for a clone to be reported. Default: 50.
    pub min_tokens: Option<u32>,

    /// Minimum line count for a clone to be reported. Default: 5.
    pub min_lines: Option<u32>,

    /// Fail if duplication percentage exceeds this value. 0 = no limit.
    pub threshold: Option<f64>,

    /// Skip file-local duplicates, only report cross-file clones.
    pub skip_local: Option<bool>,

    /// Enable cross-language detection (strip TS type annotations for TS<->JS matching).
    pub cross_language: Option<bool>,

    /// Show only the N largest clone groups.
    pub top: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
pub struct FixParams {
    /// Root directory of the project. Defaults to current working directory.
    pub root: Option<String>,

    /// Path to fallow config file.
    pub config: Option<String>,

    /// Only analyze production code (excludes tests, stories, dev files).
    pub production: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct ProjectInfoParams {
    /// Root directory of the project. Defaults to current working directory.
    pub root: Option<String>,

    /// Path to fallow config file.
    pub config: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct HealthParams {
    /// Root directory of the project to analyze. Defaults to current working directory.
    pub root: Option<String>,

    /// Maximum cyclomatic complexity threshold. Functions exceeding this are reported.
    pub max_cyclomatic: Option<u16>,

    /// Maximum cognitive complexity threshold. Functions exceeding this are reported.
    pub max_cognitive: Option<u16>,

    /// Number of top results to return, sorted by complexity.
    pub top: Option<usize>,

    /// Sort order for results (e.g., "cyclomatic", "cognitive").
    pub sort: Option<String>,

    /// Git ref to compare against. Only files changed since this ref are analyzed.
    pub changed_since: Option<String>,

    /// Show only complexity findings. By default all sections are shown; use this to select only complexity.
    pub complexity: Option<bool>,

    /// Show only per-file health scores (fan-in, fan-out, dead code ratio, maintainability index).
    pub file_scores: Option<bool>,

    /// Show only hotspots: files that are both complex and frequently changing.
    pub hotspots: Option<bool>,

    /// Show only refactoring targets: ranked recommendations based on complexity, coupling, churn, and dead code.
    pub targets: Option<bool>,

    /// Git history window for hotspot analysis. Accepts durations (6m, 90d, 1y) or ISO dates.
    pub since: Option<String>,

    /// Minimum commits for a file to appear in hotspot ranking.
    pub min_commits: Option<u32>,

    /// Scope output to a single workspace package.
    pub workspace: Option<String>,

    /// Only analyze production code (excludes tests, stories, dev files).
    pub production: Option<bool>,
}
