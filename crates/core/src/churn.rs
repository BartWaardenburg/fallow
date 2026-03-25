//! Git churn analysis for hotspot detection.
//!
//! Shells out to `git log` to collect per-file change history, then computes
//! recency-weighted churn scores and trend indicators.

use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

/// Recency weight half-life in days. A commit from 90 days ago counts half
/// as much as today's commit; 180 days ago counts 25%.
const HALF_LIFE_DAYS: f64 = 90.0;

/// Parsed duration for the `--since` flag.
#[derive(Debug, Clone)]
pub struct SinceDuration {
    /// Value to pass to `git log --after` (e.g., `"6 months ago"` or `"2025-06-01"`).
    pub git_after: String,
    /// Human-readable display string (e.g., `"6 months"`).
    pub display: String,
}

/// Churn trend indicator based on comparing recent vs older halves of the analysis period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChurnTrend {
    /// Recent half has >1.5× the commits of the older half.
    Accelerating,
    /// Churn is roughly stable between halves.
    Stable,
    /// Recent half has <0.67× the commits of the older half.
    Cooling,
}

impl std::fmt::Display for ChurnTrend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Accelerating => write!(f, "accelerating"),
            Self::Stable => write!(f, "stable"),
            Self::Cooling => write!(f, "cooling"),
        }
    }
}

/// Per-file churn data collected from git history.
#[derive(Debug, Clone)]
pub struct FileChurn {
    /// Absolute file path.
    pub path: PathBuf,
    /// Total number of commits touching this file in the analysis window.
    pub commits: u32,
    /// Recency-weighted commit count (exponential decay, half-life 90 days).
    pub weighted_commits: f64,
    /// Total lines added across all commits.
    pub lines_added: u32,
    /// Total lines deleted across all commits.
    pub lines_deleted: u32,
    /// Churn trend: accelerating, stable, or cooling.
    pub trend: ChurnTrend,
}

/// Result of churn analysis.
pub struct ChurnResult {
    /// Per-file churn data, keyed by absolute path.
    pub files: FxHashMap<PathBuf, FileChurn>,
    /// Whether the repository is a shallow clone.
    pub shallow_clone: bool,
}

/// Parse a `--since` value into a git-compatible duration.
///
/// Accepts:
/// - Durations: `6m`, `6months`, `90d`, `90days`, `1y`, `1year`, `2w`, `2weeks`
/// - ISO dates: `2025-06-01`
///
/// Returns an error for unrecognized formats.
pub fn parse_since(input: &str) -> Result<SinceDuration, String> {
    // Try ISO date first (YYYY-MM-DD)
    if is_iso_date(input) {
        return Ok(SinceDuration {
            git_after: input.to_string(),
            display: input.to_string(),
        });
    }

    // Parse duration: number + unit
    let (num_str, unit) = split_number_unit(input)?;
    let num: u64 = num_str
        .parse()
        .map_err(|_| format!("invalid number in --since: {input}"))?;

    if num == 0 {
        return Err("--since duration must be greater than 0".to_string());
    }

    match unit {
        "d" | "day" | "days" => {
            let s = if num == 1 { "" } else { "s" };
            Ok(SinceDuration {
                git_after: format!("{num} day{s} ago"),
                display: format!("{num} day{s}"),
            })
        }
        "w" | "week" | "weeks" => {
            let s = if num == 1 { "" } else { "s" };
            Ok(SinceDuration {
                git_after: format!("{num} week{s} ago"),
                display: format!("{num} week{s}"),
            })
        }
        "m" | "month" | "months" => {
            let s = if num == 1 { "" } else { "s" };
            Ok(SinceDuration {
                git_after: format!("{num} month{s} ago"),
                display: format!("{num} month{s}"),
            })
        }
        "y" | "year" | "years" => {
            let s = if num == 1 { "" } else { "s" };
            Ok(SinceDuration {
                git_after: format!("{num} year{s} ago"),
                display: format!("{num} year{s}"),
            })
        }
        _ => Err(format!(
            "unknown duration unit '{unit}' in --since. Use d/w/m/y (e.g., 6m, 90d, 1y)"
        )),
    }
}

/// Analyze git churn for files in the given root directory.
///
/// Returns `None` if git is not available or the directory is not a git repository.
pub fn analyze_churn(root: &Path, since: &SinceDuration) -> Option<ChurnResult> {
    let shallow = is_shallow_clone(root);

    let output = Command::new("git")
        .args([
            "log",
            "--numstat",
            "--no-merges",
            "--no-renames",
            "--format=format:%at",
            &format!("--after={}", since.git_after),
        ])
        .current_dir(root)
        .output();

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!("hotspot analysis skipped: failed to run git: {e}");
            return None;
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("hotspot analysis skipped: git log failed: {stderr}");
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let files = parse_git_log(&stdout, root);

    Some(ChurnResult {
        files,
        shallow_clone: shallow,
    })
}

/// Check if the repository is a shallow clone.
pub fn is_shallow_clone(root: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-shallow-repository"])
        .current_dir(root)
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .eq_ignore_ascii_case("true")
        })
        .unwrap_or(false)
}

/// Check if the directory is inside a git repository.
pub fn is_git_repo(root: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(root)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// ── Internal ──────────────────────────────────────────────────────

/// Intermediate per-file accumulator during git log parsing.
struct FileAccum {
    /// Commit timestamps (epoch seconds) for trend computation.
    commit_timestamps: Vec<u64>,
    /// Recency-weighted commit sum.
    weighted_commits: f64,
    lines_added: u32,
    lines_deleted: u32,
}

/// Parse `git log --numstat --format=format:%at` output.
fn parse_git_log(stdout: &str, root: &Path) -> FxHashMap<PathBuf, FileChurn> {
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut accum: FxHashMap<PathBuf, FileAccum> = FxHashMap::default();
    let mut current_timestamp: Option<u64> = None;

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Try to parse as epoch timestamp (from %at format)
        if let Ok(ts) = line.parse::<u64>() {
            current_timestamp = Some(ts);
            continue;
        }

        // Try to parse as numstat line: "10\t5\tpath/to/file"
        if let Some((added, deleted, path)) = parse_numstat_line(line) {
            let abs_path = root.join(path);
            let ts = current_timestamp.unwrap_or(now_secs);
            let age_days = (now_secs.saturating_sub(ts)) as f64 / 86400.0;
            let weight = 0.5_f64.powf(age_days / HALF_LIFE_DAYS);

            let entry = accum.entry(abs_path).or_insert_with(|| FileAccum {
                commit_timestamps: Vec::new(),
                weighted_commits: 0.0,
                lines_added: 0,
                lines_deleted: 0,
            });
            entry.commit_timestamps.push(ts);
            entry.weighted_commits += weight;
            entry.lines_added += added;
            entry.lines_deleted += deleted;
        }
    }

    // Convert accumulators to FileChurn with trend computation
    accum
        .into_iter()
        .map(|(path, acc)| {
            let commits = acc.commit_timestamps.len() as u32;
            let trend = compute_trend(&acc.commit_timestamps);
            let churn = FileChurn {
                path: path.clone(),
                commits,
                weighted_commits: (acc.weighted_commits * 100.0).round() / 100.0,
                lines_added: acc.lines_added,
                lines_deleted: acc.lines_deleted,
                trend,
            };
            (path, churn)
        })
        .collect()
}

/// Parse a single numstat line: `"10\t5\tpath/to/file.ts"`.
/// Binary files show as `"-\t-\tpath"` — skip those.
fn parse_numstat_line(line: &str) -> Option<(u32, u32, &str)> {
    let mut parts = line.splitn(3, '\t');
    let added_str = parts.next()?;
    let deleted_str = parts.next()?;
    let path = parts.next()?;

    // Binary files show "-" for added/deleted — skip them
    let added: u32 = added_str.parse().ok()?;
    let deleted: u32 = deleted_str.parse().ok()?;

    Some((added, deleted, path))
}

/// Compute churn trend by splitting commits into two temporal halves.
///
/// Finds the midpoint between the oldest and newest commit timestamps,
/// then compares commit counts in each half:
/// - Recent > 1.5× older → Accelerating
/// - Recent < 0.67× older → Cooling
/// - Otherwise → Stable
fn compute_trend(timestamps: &[u64]) -> ChurnTrend {
    if timestamps.len() < 2 {
        return ChurnTrend::Stable;
    }

    let min_ts = timestamps.iter().copied().min().unwrap_or(0);
    let max_ts = timestamps.iter().copied().max().unwrap_or(0);

    if max_ts == min_ts {
        return ChurnTrend::Stable;
    }

    let midpoint = min_ts + (max_ts - min_ts) / 2;
    let recent = timestamps.iter().filter(|&&ts| ts > midpoint).count() as f64;
    let older = timestamps.iter().filter(|&&ts| ts <= midpoint).count() as f64;

    if older < 1.0 {
        return ChurnTrend::Stable;
    }

    let ratio = recent / older;
    if ratio > 1.5 {
        ChurnTrend::Accelerating
    } else if ratio < 0.67 {
        ChurnTrend::Cooling
    } else {
        ChurnTrend::Stable
    }
}

fn is_iso_date(input: &str) -> bool {
    input.len() == 10
        && input.as_bytes().get(4) == Some(&b'-')
        && input.as_bytes().get(7) == Some(&b'-')
        && input[..4].bytes().all(|b| b.is_ascii_digit())
        && input[5..7].bytes().all(|b| b.is_ascii_digit())
        && input[8..10].bytes().all(|b| b.is_ascii_digit())
}

fn split_number_unit(input: &str) -> Result<(&str, &str), String> {
    let pos = input.find(|c: char| !c.is_ascii_digit()).ok_or_else(|| {
        format!("--since requires a unit suffix (e.g., 6m, 90d, 1y), got: {input}")
    })?;
    if pos == 0 {
        return Err(format!(
            "--since must start with a number (e.g., 6m, 90d, 1y), got: {input}"
        ));
    }
    Ok((&input[..pos], &input[pos..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_since ──────────────────────────────────────────────

    #[test]
    fn parse_since_months_short() {
        let d = parse_since("6m").unwrap();
        assert_eq!(d.git_after, "6 months ago");
        assert_eq!(d.display, "6 months");
    }

    #[test]
    fn parse_since_months_long() {
        let d = parse_since("6months").unwrap();
        assert_eq!(d.git_after, "6 months ago");
        assert_eq!(d.display, "6 months");
    }

    #[test]
    fn parse_since_days() {
        let d = parse_since("90d").unwrap();
        assert_eq!(d.git_after, "90 days ago");
        assert_eq!(d.display, "90 days");
    }

    #[test]
    fn parse_since_year_singular() {
        let d = parse_since("1y").unwrap();
        assert_eq!(d.git_after, "1 year ago");
        assert_eq!(d.display, "1 year");
    }

    #[test]
    fn parse_since_years_plural() {
        let d = parse_since("2years").unwrap();
        assert_eq!(d.git_after, "2 years ago");
        assert_eq!(d.display, "2 years");
    }

    #[test]
    fn parse_since_weeks() {
        let d = parse_since("2w").unwrap();
        assert_eq!(d.git_after, "2 weeks ago");
        assert_eq!(d.display, "2 weeks");
    }

    #[test]
    fn parse_since_iso_date() {
        let d = parse_since("2025-06-01").unwrap();
        assert_eq!(d.git_after, "2025-06-01");
        assert_eq!(d.display, "2025-06-01");
    }

    #[test]
    fn parse_since_month_singular() {
        let d = parse_since("1month").unwrap();
        assert_eq!(d.display, "1 month");
    }

    #[test]
    fn parse_since_day_singular() {
        let d = parse_since("1day").unwrap();
        assert_eq!(d.display, "1 day");
    }

    #[test]
    fn parse_since_zero_rejected() {
        assert!(parse_since("0m").is_err());
    }

    #[test]
    fn parse_since_no_unit_rejected() {
        assert!(parse_since("90").is_err());
    }

    #[test]
    fn parse_since_unknown_unit_rejected() {
        assert!(parse_since("6x").is_err());
    }

    #[test]
    fn parse_since_no_number_rejected() {
        assert!(parse_since("months").is_err());
    }

    // ── parse_numstat_line ───────────────────────────────────────

    #[test]
    fn numstat_normal() {
        let (a, d, p) = parse_numstat_line("10\t5\tsrc/file.ts").unwrap();
        assert_eq!(a, 10);
        assert_eq!(d, 5);
        assert_eq!(p, "src/file.ts");
    }

    #[test]
    fn numstat_binary_skipped() {
        assert!(parse_numstat_line("-\t-\tsrc/image.png").is_none());
    }

    #[test]
    fn numstat_zero_lines() {
        let (a, d, p) = parse_numstat_line("0\t0\tsrc/empty.ts").unwrap();
        assert_eq!(a, 0);
        assert_eq!(d, 0);
        assert_eq!(p, "src/empty.ts");
    }

    // ── compute_trend ────────────────────────────────────────────

    #[test]
    fn trend_empty_is_stable() {
        assert_eq!(compute_trend(&[]), ChurnTrend::Stable);
    }

    #[test]
    fn trend_single_commit_is_stable() {
        assert_eq!(compute_trend(&[100]), ChurnTrend::Stable);
    }

    #[test]
    fn trend_accelerating() {
        // 2 old commits, 5 recent commits
        let timestamps = vec![100, 200, 800, 850, 900, 950, 1000];
        assert_eq!(compute_trend(&timestamps), ChurnTrend::Accelerating);
    }

    #[test]
    fn trend_cooling() {
        // 5 old commits, 2 recent commits
        let timestamps = vec![100, 150, 200, 250, 300, 900, 1000];
        assert_eq!(compute_trend(&timestamps), ChurnTrend::Cooling);
    }

    #[test]
    fn trend_stable_even_distribution() {
        // 3 old commits, 3 recent commits → ratio = 1.0 → stable
        let timestamps = vec![100, 200, 300, 700, 800, 900];
        assert_eq!(compute_trend(&timestamps), ChurnTrend::Stable);
    }

    #[test]
    fn trend_same_timestamp_is_stable() {
        let timestamps = vec![500, 500, 500];
        assert_eq!(compute_trend(&timestamps), ChurnTrend::Stable);
    }

    // ── is_iso_date ──────────────────────────────────────────────

    #[test]
    fn iso_date_valid() {
        assert!(is_iso_date("2025-06-01"));
        assert!(is_iso_date("2025-12-31"));
    }

    #[test]
    fn iso_date_with_time_rejected() {
        // Only exact YYYY-MM-DD (10 chars) is accepted
        assert!(!is_iso_date("2025-06-01T00:00:00"));
    }

    #[test]
    fn iso_date_invalid() {
        assert!(!is_iso_date("6months"));
        assert!(!is_iso_date("2025"));
        assert!(!is_iso_date("not-a-date"));
        assert!(!is_iso_date("abcd-ef-gh"));
    }

    // ── Display ──────────────────────────────────────────────────

    #[test]
    fn trend_display() {
        assert_eq!(ChurnTrend::Accelerating.to_string(), "accelerating");
        assert_eq!(ChurnTrend::Stable.to_string(), "stable");
        assert_eq!(ChurnTrend::Cooling.to_string(), "cooling");
    }

    // ── parse_git_log ───────────────────────────────────────────

    #[test]
    fn parse_git_log_single_commit() {
        let root = Path::new("/project");
        let output = "1700000000\n10\t5\tsrc/index.ts\n";
        let result = parse_git_log(output, root);
        assert_eq!(result.len(), 1);
        let churn = result.get(&PathBuf::from("/project/src/index.ts")).unwrap();
        assert_eq!(churn.commits, 1);
        assert_eq!(churn.lines_added, 10);
        assert_eq!(churn.lines_deleted, 5);
    }

    #[test]
    fn parse_git_log_multiple_commits_same_file() {
        let root = Path::new("/project");
        let output = "1700000000\n10\t5\tsrc/index.ts\n\n1700100000\n3\t2\tsrc/index.ts\n";
        let result = parse_git_log(output, root);
        assert_eq!(result.len(), 1);
        let churn = result.get(&PathBuf::from("/project/src/index.ts")).unwrap();
        assert_eq!(churn.commits, 2);
        assert_eq!(churn.lines_added, 13);
        assert_eq!(churn.lines_deleted, 7);
    }

    #[test]
    fn parse_git_log_multiple_files() {
        let root = Path::new("/project");
        let output = "1700000000\n10\t5\tsrc/a.ts\n3\t1\tsrc/b.ts\n";
        let result = parse_git_log(output, root);
        assert_eq!(result.len(), 2);
        assert!(result.contains_key(&PathBuf::from("/project/src/a.ts")));
        assert!(result.contains_key(&PathBuf::from("/project/src/b.ts")));
    }

    #[test]
    fn parse_git_log_empty_output() {
        let root = Path::new("/project");
        let result = parse_git_log("", root);
        assert!(result.is_empty());
    }

    #[test]
    fn parse_git_log_skips_binary_files() {
        let root = Path::new("/project");
        let output = "1700000000\n-\t-\timage.png\n10\t5\tsrc/a.ts\n";
        let result = parse_git_log(output, root);
        assert_eq!(result.len(), 1);
        assert!(!result.contains_key(&PathBuf::from("/project/image.png")));
    }

    #[test]
    fn parse_git_log_weighted_commits_are_positive() {
        let root = Path::new("/project");
        // Use a timestamp near "now" to ensure weight doesn't decay to zero
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let output = format!("{now_secs}\n10\t5\tsrc/a.ts\n");
        let result = parse_git_log(&output, root);
        let churn = result.get(&PathBuf::from("/project/src/a.ts")).unwrap();
        assert!(
            churn.weighted_commits > 0.0,
            "weighted_commits should be positive for recent commits"
        );
    }

    // ── compute_trend edge cases ─────────────────────────────────

    #[test]
    fn trend_boundary_1_5x_ratio() {
        // Exactly 1.5x ratio (3 recent : 2 old) → boundary between stable and accelerating
        // midpoint = 100 + (1000-100)/2 = 550
        // old: 100, 200 (2 timestamps <= 550)
        // recent: 600, 800, 1000 (3 timestamps > 550)
        // ratio = 3/2 = 1.5 — NOT > 1.5, so stable
        let timestamps = vec![100, 200, 600, 800, 1000];
        assert_eq!(compute_trend(&timestamps), ChurnTrend::Stable);
    }

    #[test]
    fn trend_just_above_1_5x() {
        // midpoint = 100 + (1000-100)/2 = 550
        // old: 100 (1 timestamp <= 550)
        // recent: 600, 800, 1000 (3 timestamps > 550)
        // ratio = 3/1 = 3.0 → accelerating
        let timestamps = vec![100, 600, 800, 1000];
        assert_eq!(compute_trend(&timestamps), ChurnTrend::Accelerating);
    }
}
