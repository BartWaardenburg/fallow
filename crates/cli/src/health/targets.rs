use crate::health_types::{
    ContributingFactor, EffortEstimate, EvidenceFunction, FileHealthScore, HotspotEntry,
    RecommendationCategory, RefactoringTarget, TargetEvidence,
};

/// Auxiliary data used by `compute_refactoring_targets` to generate evidence and apply rules.
pub(super) struct TargetAuxData<'a> {
    pub circular_files: &'a rustc_hash::FxHashSet<std::path::PathBuf>,
    pub top_complex_fns: &'a rustc_hash::FxHashMap<std::path::PathBuf, Vec<(String, u32, u16)>>,
    pub entry_points: &'a rustc_hash::FxHashSet<std::path::PathBuf>,
    pub value_export_counts: &'a rustc_hash::FxHashMap<std::path::PathBuf, usize>,
    pub unused_export_names: &'a rustc_hash::FxHashMap<std::path::PathBuf, Vec<String>>,
    pub cycle_members: &'a rustc_hash::FxHashMap<std::path::PathBuf, Vec<std::path::PathBuf>>,
}

/// Compute the refactoring priority score for a file.
///
/// Formula (avoids double-counting with MI):
/// ```text
/// priority = min(density, 1) * 30 + hotspot_boost * 25 + dead_code * 20 + fan_in_norm * 15 + fan_out_norm * 10
/// ```
/// All inputs are clamped to \[0, 1\] so each weight is a true percentage share.
pub(super) fn compute_target_priority(score: &FileHealthScore, hotspot_score: Option<f64>) -> f64 {
    // Normalize all inputs to [0, 1] so each weight is a true percentage share.
    let density_norm = score.complexity_density.min(1.0);
    let fan_in_norm = (score.fan_in as f64 / 20.0).min(1.0);
    let fan_out_norm = (score.fan_out as f64 / 30.0).min(1.0);
    let hotspot_boost = hotspot_score.map_or(0.0, |s| s / 100.0);

    // Keep the formula readable — it matches the documented specification.
    #[expect(clippy::suboptimal_flops)]
    let priority = density_norm * 30.0
        + hotspot_boost * 25.0
        + score.dead_code_ratio * 20.0
        + fan_in_norm * 15.0
        + fan_out_norm * 10.0;

    (priority.clamp(0.0, 100.0) * 10.0).round() / 10.0
}

/// Compute refactoring targets by applying rules to file scores and auxiliary data.
///
/// Rules are evaluated in priority order; first match determines the category and
/// recommendation. All contributing factors are collected regardless of which rule wins.
/// Files matching no rule are skipped.
pub(super) fn compute_refactoring_targets(
    file_scores: &[FileHealthScore],
    aux: &TargetAuxData,
    hotspots: &[HotspotEntry],
) -> Vec<RefactoringTarget> {
    // Build hotspot lookup by path for O(1) access
    let hotspot_map: rustc_hash::FxHashMap<&std::path::Path, &HotspotEntry> =
        hotspots.iter().map(|h| (h.path.as_path(), h)).collect();

    let mut targets = Vec::new();

    for score in file_scores {
        let hotspot = hotspot_map.get(score.path.as_path());
        let hotspot_score = hotspot.map(|h| h.score);
        let is_circular = aux.circular_files.contains(&score.path);
        let is_entry = aux.entry_points.contains(&score.path);
        let top_fns = aux.top_complex_fns.get(&score.path);
        let value_exports = aux
            .value_export_counts
            .get(&score.path)
            .copied()
            .unwrap_or(0);

        // Collect all contributing factors
        let mut factors = Vec::new();

        if score.complexity_density > 0.3 {
            factors.push(ContributingFactor {
                metric: "complexity_density",
                value: score.complexity_density,
                threshold: 0.3,
                detail: format!("density {:.2} exceeds 0.3", score.complexity_density),
            });
        }
        if score.fan_in >= 10 {
            factors.push(ContributingFactor {
                metric: "fan_in",
                value: score.fan_in as f64,
                threshold: 10.0,
                detail: format!("{} files depend on this", score.fan_in),
            });
        }
        if score.dead_code_ratio >= 0.5 && value_exports >= 3 {
            let unused_count = (score.dead_code_ratio * value_exports as f64)
                .round()
                .min(value_exports as f64) as usize;
            factors.push(ContributingFactor {
                metric: "dead_code_ratio",
                value: score.dead_code_ratio,
                threshold: 0.5,
                detail: format!(
                    "{} unused of {} value exports ({:.0}%)",
                    unused_count,
                    value_exports,
                    score.dead_code_ratio * 100.0
                ),
            });
        }
        if score.fan_out >= 15 {
            factors.push(ContributingFactor {
                metric: "fan_out",
                value: score.fan_out as f64,
                threshold: 15.0,
                detail: format!("imports {} modules", score.fan_out),
            });
        }
        if is_circular {
            factors.push(ContributingFactor {
                metric: "circular_dependency",
                value: 1.0,
                threshold: 1.0,
                detail: "participates in an import cycle".into(),
            });
        }
        if let Some(h) = hotspot
            && h.score >= 30.0
        {
            factors.push(ContributingFactor {
                metric: "hotspot_score",
                value: h.score,
                threshold: 30.0,
                detail: format!(
                    "hotspot score {:.0} ({} commits, {} trend)",
                    h.score,
                    h.commits,
                    match h.trend {
                        fallow_core::churn::ChurnTrend::Accelerating => "accelerating",
                        fallow_core::churn::ChurnTrend::Cooling => "cooling",
                        fallow_core::churn::ChurnTrend::Stable => "stable",
                    }
                ),
            });
        }
        if let Some(fns) = top_fns
            && let Some((name, _, cog)) = fns.first()
            && *cog >= 30
        {
            factors.push(ContributingFactor {
                metric: "cognitive_complexity",
                value: f64::from(*cog),
                threshold: 30.0,
                detail: format!("{name} has cognitive complexity {cog}"),
            });
        }

        // Skip if no factors triggered
        if factors.is_empty() {
            continue;
        }

        // Evaluate rules in priority order — first match determines category + recommendation
        let matched = try_match_rules(
            score,
            hotspot.copied(),
            is_circular,
            is_entry,
            top_fns,
            value_exports,
        );

        let Some((category, recommendation)) = matched else {
            continue;
        };

        let priority = compute_target_priority(score, hotspot_score);
        let effort = compute_effort_estimate(score);
        let evidence = build_evidence(
            &category,
            &score.path,
            aux.unused_export_names,
            top_fns,
            aux.cycle_members,
        );

        targets.push(RefactoringTarget {
            path: score.path.clone(),
            priority,
            recommendation,
            category,
            effort,
            factors,
            evidence,
        });
    }

    // Sort by priority descending, break ties by path for determinism
    targets.sort_by(|a, b| {
        b.priority
            .partial_cmp(&a.priority)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.path.cmp(&b.path))
    });

    targets
}

/// Try to match a file against refactoring rules in priority order.
///
/// Returns the first matching `(category, recommendation)`, or `None` if no rule matches.
fn try_match_rules(
    score: &FileHealthScore,
    hotspot: Option<&HotspotEntry>,
    is_circular: bool,
    is_entry: bool,
    top_fns: Option<&Vec<(String, u32, u16)>>,
    value_exports: usize,
) -> Option<(RecommendationCategory, String)> {
    // Rule 1: Urgent churn + complexity
    if let Some(h) = hotspot
        && h.score >= 50.0
        && matches!(h.trend, fallow_core::churn::ChurnTrend::Accelerating)
        && score.complexity_density > 0.5
    {
        return Some((
            RecommendationCategory::UrgentChurnComplexity,
            "Actively-changing file with growing complexity \u{2014} stabilize before adding features".into(),
        ));
    }

    // Rule 2: Circular dependency with high fan-in
    if is_circular && score.fan_in >= 5 {
        return Some((
            RecommendationCategory::BreakCircularDependency,
            format!(
                "Break import cycle \u{2014} {} files depend on this, changes cascade through the cycle",
                score.fan_in
            ),
        ));
    }

    // Rule 3: Split high-impact file
    if score.complexity_density > 0.3
        && (score.fan_in >= 20 || (score.fan_in >= 10 && score.function_count >= 5))
    {
        return Some((
            RecommendationCategory::SplitHighImpact,
            format!(
                "Split high-impact file \u{2014} {} dependents amplify every change",
                score.fan_in
            ),
        ));
    }

    // Rule 4: Remove dead code (gate: >=3 value exports)
    if score.dead_code_ratio >= 0.5 && value_exports >= 3 {
        let unused_count = (score.dead_code_ratio * value_exports as f64).round() as usize;
        return Some((
            RecommendationCategory::RemoveDeadCode,
            format!(
                "Remove {} unused exports to reduce surface area ({:.0}% dead)",
                unused_count,
                score.dead_code_ratio * 100.0
            ),
        ));
    }

    // Rule 5: Extract complex functions (cognitive >= 30)
    if let Some(fns) = top_fns {
        let high: Vec<&(String, u32, u16)> = fns.iter().filter(|(_, _, cog)| *cog >= 30).collect();
        if !high.is_empty() {
            let desc = match high.len() {
                1 => format!(
                    "Extract {} (cognitive: {}) into smaller functions",
                    high[0].0, high[0].2
                ),
                _ => format!(
                    "Extract {} (cognitive: {}) and {} (cognitive: {}) into smaller functions",
                    high[0].0, high[0].2, high[1].0, high[1].2
                ),
            };
            return Some((RecommendationCategory::ExtractComplexFunctions, desc));
        }
    }

    // Rule 6: Extract dependencies (not for entry points)
    if !is_entry && score.fan_out >= 15 && score.maintainability_index < 60.0 {
        return Some((
            RecommendationCategory::ExtractDependencies,
            format!(
                "Reduce coupling \u{2014} this file imports {} modules, limiting testability",
                score.fan_out
            ),
        ));
    }

    // Rule 7: Circular dependency (low fan-in fallback)
    if is_circular {
        return Some((
            RecommendationCategory::BreakCircularDependency,
            "Break import cycle to reduce change cascade risk".into(),
        ));
    }

    None
}

/// Compute effort estimate based on file size, function count, and fan-in.
///
/// - **Low**: `lines < 100 AND function_count <= 3 AND fan_in < 5`
/// - **High**: `lines >= 500 OR fan_in >= 20 OR (function_count >= 15 AND complexity_density > 0.5)`
/// - **Medium**: everything else
fn compute_effort_estimate(score: &FileHealthScore) -> EffortEstimate {
    if score.lines >= 500
        || score.fan_in >= 20
        || (score.function_count >= 15 && score.complexity_density > 0.5)
    {
        EffortEstimate::High
    } else if score.lines < 100 && score.function_count <= 3 && score.fan_in < 5 {
        EffortEstimate::Low
    } else {
        EffortEstimate::Medium
    }
}

/// Build structured evidence for a refactoring target based on its category.
fn build_evidence(
    category: &RecommendationCategory,
    path: &std::path::Path,
    unused_export_names: &rustc_hash::FxHashMap<std::path::PathBuf, Vec<String>>,
    top_fns: Option<&Vec<(String, u32, u16)>>,
    cycle_members: &rustc_hash::FxHashMap<std::path::PathBuf, Vec<std::path::PathBuf>>,
) -> Option<TargetEvidence> {
    match category {
        RecommendationCategory::RemoveDeadCode => {
            let exports = unused_export_names.get(path).cloned().unwrap_or_default();
            if exports.is_empty() {
                None
            } else {
                Some(TargetEvidence {
                    unused_exports: exports,
                    complex_functions: vec![],
                    cycle_path: vec![],
                })
            }
        }
        RecommendationCategory::ExtractComplexFunctions => {
            let functions = top_fns
                .map(|fns| {
                    fns.iter()
                        .filter(|(_, _, cog)| *cog >= 30)
                        .map(|(name, line, cog)| EvidenceFunction {
                            name: name.clone(),
                            line: *line,
                            cognitive: *cog,
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if functions.is_empty() {
                None
            } else {
                Some(TargetEvidence {
                    unused_exports: vec![],
                    complex_functions: functions,
                    cycle_path: vec![],
                })
            }
        }
        RecommendationCategory::BreakCircularDependency => {
            let members = cycle_members
                .get(path)
                .map(|files| {
                    files
                        .iter()
                        .map(|f| f.to_string_lossy().into_owned())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if members.is_empty() {
                None
            } else {
                Some(TargetEvidence {
                    unused_exports: vec![],
                    complex_functions: vec![],
                    cycle_path: members,
                })
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- compute_target_priority ---

    fn make_score(overrides: impl FnOnce(&mut FileHealthScore)) -> FileHealthScore {
        let mut s = FileHealthScore {
            path: std::path::PathBuf::from("/src/foo.ts"),
            fan_in: 0,
            fan_out: 0,
            dead_code_ratio: 0.0,
            complexity_density: 0.0,
            maintainability_index: 100.0,
            total_cyclomatic: 0,
            total_cognitive: 0,
            function_count: 1,
            lines: 100,
        };
        overrides(&mut s);
        s
    }

    #[test]
    fn target_priority_all_zero() {
        let score = make_score(|_| {});
        let priority = compute_target_priority(&score, None);
        assert!((priority).abs() < f64::EPSILON);
    }

    #[test]
    fn target_priority_max_all_inputs() {
        let score = make_score(|s| {
            s.complexity_density = 2.0; // clamped to 1.0
            s.fan_in = 40; // clamped to 1.0
            s.fan_out = 60; // clamped to 1.0
            s.dead_code_ratio = 1.0;
        });
        let priority = compute_target_priority(&score, Some(100.0));
        assert!((priority - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn target_priority_complexity_density_weight() {
        // density=1.0, all else zero -> 30 points
        let score = make_score(|s| s.complexity_density = 1.0);
        let priority = compute_target_priority(&score, None);
        assert!((priority - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn target_priority_hotspot_weight() {
        // hotspot_score=100 -> boost=1.0 -> 25 points
        let score = make_score(|_| {});
        let priority = compute_target_priority(&score, Some(100.0));
        assert!((priority - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn target_priority_dead_code_weight() {
        // dead_code_ratio=1.0 -> 20 points
        let score = make_score(|s| s.dead_code_ratio = 1.0);
        let priority = compute_target_priority(&score, None);
        assert!((priority - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn target_priority_fan_in_weight() {
        // fan_in=20 -> norm=1.0 -> 15 points
        let score = make_score(|s| s.fan_in = 20);
        let priority = compute_target_priority(&score, None);
        assert!((priority - 15.0).abs() < f64::EPSILON);
    }

    #[test]
    fn target_priority_fan_out_weight() {
        // fan_out=30 -> norm=1.0 -> 10 points
        let score = make_score(|s| s.fan_out = 30);
        let priority = compute_target_priority(&score, None);
        assert!((priority - 10.0).abs() < f64::EPSILON);
    }

    // --- try_match_rules ---

    #[test]
    fn rule_no_match_clean_file() {
        let score = make_score(|_| {});
        let result = try_match_rules(&score, None, false, false, None, 0);
        assert!(result.is_none());
    }

    #[test]
    fn rule_circular_dep_high_fan_in() {
        let score = make_score(|s| s.fan_in = 5);
        let result = try_match_rules(&score, None, true, false, None, 0);
        assert!(result.is_some());
        let (cat, _) = result.unwrap();
        assert!(matches!(
            cat,
            RecommendationCategory::BreakCircularDependency
        ));
    }

    #[test]
    fn rule_circular_dep_low_fan_in_fallback() {
        let score = make_score(|s| s.fan_in = 1);
        let result = try_match_rules(&score, None, true, false, None, 0);
        assert!(result.is_some());
        let (cat, _) = result.unwrap();
        assert!(matches!(
            cat,
            RecommendationCategory::BreakCircularDependency
        ));
    }

    #[test]
    fn rule_split_high_impact() {
        let score = make_score(|s| {
            s.complexity_density = 0.5;
            s.fan_in = 20;
        });
        let result = try_match_rules(&score, None, false, false, None, 0);
        assert!(result.is_some());
        let (cat, _) = result.unwrap();
        assert!(matches!(cat, RecommendationCategory::SplitHighImpact));
    }

    #[test]
    fn rule_remove_dead_code() {
        let score = make_score(|s| s.dead_code_ratio = 0.6);
        let result = try_match_rules(&score, None, false, false, None, 5);
        assert!(result.is_some());
        let (cat, _) = result.unwrap();
        assert!(matches!(cat, RecommendationCategory::RemoveDeadCode));
    }

    #[test]
    fn rule_dead_code_gate_too_few_exports() {
        // dead_code_ratio high but only 2 value exports — below gate of 3
        let score = make_score(|s| s.dead_code_ratio = 0.8);
        let result = try_match_rules(&score, None, false, false, None, 2);
        // Should NOT match dead code rule
        assert!(result.is_none());
    }

    #[test]
    fn rule_extract_complex_functions() {
        let score = make_score(|_| {});
        let fns = vec![("handleSubmit".to_string(), 10u32, 35u16)];
        let result = try_match_rules(&score, None, false, false, Some(&fns), 0);
        assert!(result.is_some());
        let (cat, rec) = result.unwrap();
        assert!(matches!(
            cat,
            RecommendationCategory::ExtractComplexFunctions
        ));
        assert!(rec.contains("handleSubmit"));
    }

    #[test]
    fn rule_extract_dependencies_not_entry() {
        let score = make_score(|s| {
            s.fan_out = 20;
            s.maintainability_index = 50.0;
        });
        let result = try_match_rules(&score, None, false, false, None, 0);
        assert!(result.is_some());
        let (cat, _) = result.unwrap();
        assert!(matches!(cat, RecommendationCategory::ExtractDependencies));
    }

    #[test]
    fn rule_extract_dependencies_skipped_for_entry() {
        let score = make_score(|s| {
            s.fan_out = 20;
            s.maintainability_index = 50.0;
        });
        // is_entry=true -> rule 6 should not match
        let result = try_match_rules(&score, None, false, true, None, 0);
        assert!(result.is_none());
    }

    #[test]
    fn rule_urgent_churn_complexity() {
        let score = make_score(|s| s.complexity_density = 0.8);
        let hotspot = HotspotEntry {
            path: std::path::PathBuf::from("/src/foo.ts"),
            score: 60.0,
            commits: 20,
            weighted_commits: 15.0,
            lines_added: 500,
            lines_deleted: 100,
            complexity_density: 0.8,
            fan_in: 5,
            trend: fallow_core::churn::ChurnTrend::Accelerating,
        };
        let result = try_match_rules(&score, Some(&hotspot), false, false, None, 0);
        assert!(result.is_some());
        let (cat, _) = result.unwrap();
        assert!(matches!(cat, RecommendationCategory::UrgentChurnComplexity));
    }
}
