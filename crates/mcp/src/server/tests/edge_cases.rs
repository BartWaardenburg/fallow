use crate::params::*;
use crate::tools::{
    build_analyze_args, build_check_changed_args, build_find_dupes_args, build_fix_apply_args,
    build_fix_preview_args, build_health_args, build_project_info_args,
};

// ── Edge cases: special characters in arguments ───────────────────

#[test]
fn analyze_args_with_spaces_in_paths() {
    let params = AnalyzeParams {
        root: Some("/path/with spaces/project".to_string()),
        config: Some("my config.json".to_string()),
        production: None,
        workspace: Some("my package".to_string()),
        issue_types: None,
    };
    let args = build_analyze_args(&params).unwrap();
    assert!(args.contains(&"/path/with spaces/project".to_string()));
    assert!(args.contains(&"my config.json".to_string()));
    assert!(args.contains(&"my package".to_string()));
}

#[test]
fn check_changed_args_with_special_ref() {
    let params = CheckChangedParams {
        root: None,
        since: "origin/feature/my-branch".to_string(),
        config: None,
        production: None,
        workspace: None,
    };
    let args = build_check_changed_args(params);
    assert!(args.contains(&"origin/feature/my-branch".to_string()));
}

#[test]
fn health_args_boundary_values() {
    let params = HealthParams {
        root: None,
        max_cyclomatic: Some(0),
        max_cognitive: Some(u16::MAX),
        top: Some(0),
        sort: None,
        changed_since: None,
        complexity: None,
        file_scores: None,
        hotspots: None,
        targets: None,
        since: None,
        min_commits: None,
        workspace: None,
        production: None,
        save_snapshot: None,
    };
    let args = build_health_args(&params);
    assert!(args.contains(&"0".to_string()));
    assert!(args.contains(&"65535".to_string()));
}

#[test]
fn health_args_file_scores_flag() {
    let params = HealthParams {
        root: None,
        max_cyclomatic: None,
        max_cognitive: None,
        top: None,
        sort: None,
        changed_since: None,
        complexity: None,
        file_scores: Some(true),
        hotspots: None,
        targets: None,
        since: None,
        min_commits: None,
        production: None,
        workspace: None,
        save_snapshot: None,
    };
    let args = build_health_args(&params);
    assert!(args.contains(&"--file-scores".to_string()));
}

// ── Additional arg builder coverage: boolean false omission ───────

#[test]
fn check_changed_args_production_false_is_omitted() {
    let params = CheckChangedParams {
        root: None,
        since: "main".to_string(),
        config: None,
        production: Some(false),
        workspace: None,
    };
    let args = build_check_changed_args(params);
    assert!(!args.contains(&"--production".to_string()));
}

#[test]
fn find_dupes_args_cross_language_false_is_omitted() {
    let params = FindDupesParams {
        root: None,
        mode: None,
        min_tokens: None,
        min_lines: None,
        threshold: None,
        skip_local: None,
        cross_language: Some(false),
        top: None,
    };
    let args = build_find_dupes_args(&params).unwrap();
    assert!(!args.contains(&"--cross-language".to_string()));
}

#[test]
fn fix_preview_args_production_false_is_omitted() {
    let params = FixParams {
        root: None,
        config: None,
        production: Some(false),
    };
    let args = build_fix_preview_args(&params);
    assert!(!args.contains(&"--production".to_string()));
}

#[test]
fn fix_apply_args_production_false_is_omitted() {
    let params = FixParams {
        root: None,
        config: None,
        production: Some(false),
    };
    let args = build_fix_apply_args(&params);
    assert!(!args.contains(&"--production".to_string()));
}

#[test]
fn health_args_boolean_flags_false_are_omitted() {
    let params = HealthParams {
        root: None,
        max_cyclomatic: None,
        max_cognitive: None,
        top: None,
        sort: None,
        changed_since: None,
        complexity: Some(false),
        file_scores: Some(false),
        hotspots: Some(false),
        targets: None,
        since: None,
        min_commits: None,
        workspace: None,
        production: Some(false),
        save_snapshot: None,
    };
    let args = build_health_args(&params);
    assert!(!args.contains(&"--complexity".to_string()));
    assert!(!args.contains(&"--file-scores".to_string()));
    assert!(!args.contains(&"--hotspots".to_string()));
    assert!(!args.contains(&"--production".to_string()));
}

// ── Additional arg builder coverage: isolated optional params ─────

#[test]
fn health_args_complexity_flag_only() {
    let params = HealthParams {
        root: None,
        max_cyclomatic: None,
        max_cognitive: None,
        top: None,
        sort: None,
        changed_since: None,
        complexity: Some(true),
        file_scores: None,
        hotspots: None,
        targets: None,
        since: None,
        min_commits: None,
        workspace: None,
        production: None,
        save_snapshot: None,
    };
    let args = build_health_args(&params);
    assert!(args.contains(&"--complexity".to_string()));
    assert!(!args.contains(&"--file-scores".to_string()));
    assert!(!args.contains(&"--hotspots".to_string()));
}

#[test]
fn health_args_hotspots_flag_only() {
    let params = HealthParams {
        root: None,
        max_cyclomatic: None,
        max_cognitive: None,
        top: None,
        sort: None,
        changed_since: None,
        complexity: None,
        file_scores: None,
        hotspots: Some(true),
        targets: None,
        since: None,
        min_commits: None,
        workspace: None,
        production: None,
        save_snapshot: None,
    };
    let args = build_health_args(&params);
    assert!(args.contains(&"--hotspots".to_string()));
    assert!(!args.contains(&"--complexity".to_string()));
    assert!(!args.contains(&"--file-scores".to_string()));
}

#[test]
fn health_args_since_and_min_commits() {
    let params = HealthParams {
        root: None,
        max_cyclomatic: None,
        max_cognitive: None,
        top: None,
        sort: None,
        changed_since: None,
        complexity: None,
        file_scores: None,
        hotspots: None,
        targets: None,
        since: Some("90d".to_string()),
        min_commits: Some(10),
        workspace: None,
        production: None,
        save_snapshot: None,
    };
    let args = build_health_args(&params);
    assert!(args.contains(&"--since".to_string()));
    assert!(args.contains(&"90d".to_string()));
    assert!(args.contains(&"--min-commits".to_string()));
    assert!(args.contains(&"10".to_string()));
}

#[test]
fn health_args_workspace_and_production() {
    let params = HealthParams {
        root: None,
        max_cyclomatic: None,
        max_cognitive: None,
        top: None,
        sort: None,
        changed_since: None,
        complexity: None,
        file_scores: None,
        hotspots: None,
        targets: None,
        since: None,
        min_commits: None,
        workspace: Some("@scope/pkg".to_string()),
        production: Some(true),
        save_snapshot: None,
    };
    let args = build_health_args(&params);
    assert!(args.contains(&"--workspace".to_string()));
    assert!(args.contains(&"@scope/pkg".to_string()));
    assert!(args.contains(&"--production".to_string()));
}

#[test]
fn find_dupes_args_individual_numeric_params() {
    // Test min_tokens alone
    let params = FindDupesParams {
        root: None,
        mode: None,
        min_tokens: Some(75),
        min_lines: None,
        threshold: None,
        skip_local: None,
        cross_language: None,
        top: None,
    };
    let args = build_find_dupes_args(&params).unwrap();
    assert!(args.contains(&"--min-tokens".to_string()));
    assert!(args.contains(&"75".to_string()));
    assert!(!args.contains(&"--min-lines".to_string()));
    assert!(!args.contains(&"--threshold".to_string()));
    assert!(!args.contains(&"--top".to_string()));
}

#[test]
fn find_dupes_args_top_only() {
    let params = FindDupesParams {
        root: None,
        mode: None,
        min_tokens: None,
        min_lines: None,
        threshold: None,
        skip_local: None,
        cross_language: None,
        top: Some(3),
    };
    let args = build_find_dupes_args(&params).unwrap();
    assert!(args.contains(&"--top".to_string()));
    assert!(args.contains(&"3".to_string()));
}

#[test]
fn check_changed_args_only_root() {
    let params = CheckChangedParams {
        root: Some("/workspace".to_string()),
        since: "HEAD~1".to_string(),
        config: None,
        production: None,
        workspace: None,
    };
    let args = build_check_changed_args(params);
    assert!(args.contains(&"--root".to_string()));
    assert!(args.contains(&"/workspace".to_string()));
    assert!(!args.contains(&"--config".to_string()));
    assert!(!args.contains(&"--production".to_string()));
    assert!(!args.contains(&"--workspace".to_string()));
}

#[test]
fn project_info_args_only_root() {
    let params = ProjectInfoParams {
        root: Some("/app".to_string()),
        config: None,
    };
    let args = build_project_info_args(&params);
    assert!(args.contains(&"--root".to_string()));
    assert!(args.contains(&"/app".to_string()));
    assert!(!args.contains(&"--config".to_string()));
}

#[test]
fn project_info_args_only_config() {
    let params = ProjectInfoParams {
        root: None,
        config: Some(".fallowrc.json".to_string()),
    };
    let args = build_project_info_args(&params);
    assert!(args.contains(&"--config".to_string()));
    assert!(args.contains(&".fallowrc.json".to_string()));
    assert!(!args.contains(&"--root".to_string()));
}
