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
        workspace: Some("my package".to_string()),
        ..Default::default()
    };
    let args = build_analyze_args(&params).unwrap();
    assert!(args.contains(&"/path/with spaces/project".to_string()));
    assert!(args.contains(&"my config.json".to_string()));
    assert!(args.contains(&"my package".to_string()));
}

#[test]
fn check_changed_args_with_special_ref() {
    let params = CheckChangedParams {
        since: "origin/feature/my-branch".to_string(),
        root: None,
        config: None,
        production: None,
        workspace: None,
        baseline: None,
        save_baseline: None,
        no_cache: None,
        threads: None,
    };
    let args = build_check_changed_args(params);
    assert!(args.contains(&"origin/feature/my-branch".to_string()));
}

#[test]
fn health_args_boundary_values() {
    let params = HealthParams {
        max_cyclomatic: Some(0),
        max_cognitive: Some(u16::MAX),
        top: Some(0),
        ..Default::default()
    };
    let args = build_health_args(&params);
    assert!(args.contains(&"0".to_string()));
    assert!(args.contains(&"65535".to_string()));
}

#[test]
fn health_args_file_scores_flag() {
    let params = HealthParams {
        file_scores: Some(true),
        ..Default::default()
    };
    let args = build_health_args(&params);
    assert!(args.contains(&"--file-scores".to_string()));
}

// ── Additional arg builder coverage: boolean false omission ───────

#[test]
fn check_changed_args_production_false_is_omitted() {
    let params = CheckChangedParams {
        since: "main".to_string(),
        production: Some(false),
        root: None,
        config: None,
        workspace: None,
        baseline: None,
        save_baseline: None,
        no_cache: None,
        threads: None,
    };
    let args = build_check_changed_args(params);
    assert!(!args.contains(&"--production".to_string()));
}

#[test]
fn find_dupes_args_cross_language_false_is_omitted() {
    let params = FindDupesParams {
        cross_language: Some(false),
        ..Default::default()
    };
    let args = build_find_dupes_args(&params).unwrap();
    assert!(!args.contains(&"--cross-language".to_string()));
}

#[test]
fn fix_preview_args_production_false_is_omitted() {
    let params = FixParams {
        production: Some(false),
        ..Default::default()
    };
    let args = build_fix_preview_args(&params);
    assert!(!args.contains(&"--production".to_string()));
}

#[test]
fn fix_apply_args_production_false_is_omitted() {
    let params = FixParams {
        production: Some(false),
        ..Default::default()
    };
    let args = build_fix_apply_args(&params);
    assert!(!args.contains(&"--production".to_string()));
}

#[test]
fn health_args_boolean_flags_false_are_omitted() {
    let params = HealthParams {
        complexity: Some(false),
        file_scores: Some(false),
        hotspots: Some(false),
        production: Some(false),
        no_cache: Some(false),
        ..Default::default()
    };
    let args = build_health_args(&params);
    assert!(!args.contains(&"--complexity".to_string()));
    assert!(!args.contains(&"--file-scores".to_string()));
    assert!(!args.contains(&"--hotspots".to_string()));
    assert!(!args.contains(&"--production".to_string()));
    assert!(!args.contains(&"--no-cache".to_string()));
}

// ── Additional arg builder coverage: isolated optional params ─────

#[test]
fn health_args_complexity_flag_only() {
    let params = HealthParams {
        complexity: Some(true),
        ..Default::default()
    };
    let args = build_health_args(&params);
    assert!(args.contains(&"--complexity".to_string()));
    assert!(!args.contains(&"--file-scores".to_string()));
    assert!(!args.contains(&"--hotspots".to_string()));
}

#[test]
fn health_args_hotspots_flag_only() {
    let params = HealthParams {
        hotspots: Some(true),
        ..Default::default()
    };
    let args = build_health_args(&params);
    assert!(args.contains(&"--hotspots".to_string()));
    assert!(!args.contains(&"--complexity".to_string()));
    assert!(!args.contains(&"--file-scores".to_string()));
}

#[test]
fn health_args_since_and_min_commits() {
    let params = HealthParams {
        since: Some("90d".to_string()),
        min_commits: Some(10),
        ..Default::default()
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
        workspace: Some("@scope/pkg".to_string()),
        production: Some(true),
        ..Default::default()
    };
    let args = build_health_args(&params);
    assert!(args.contains(&"--workspace".to_string()));
    assert!(args.contains(&"@scope/pkg".to_string()));
    assert!(args.contains(&"--production".to_string()));
}

#[test]
fn find_dupes_args_individual_numeric_params() {
    let params = FindDupesParams {
        min_tokens: Some(75),
        ..Default::default()
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
        top: Some(3),
        ..Default::default()
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
        baseline: None,
        save_baseline: None,
        no_cache: None,
        threads: None,
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
        ..Default::default()
    };
    let args = build_project_info_args(&params);
    assert!(args.contains(&"--root".to_string()));
    assert!(args.contains(&"/app".to_string()));
    assert!(!args.contains(&"--config".to_string()));
}

#[test]
fn project_info_args_only_config() {
    let params = ProjectInfoParams {
        config: Some(".fallowrc.json".to_string()),
        ..Default::default()
    };
    let args = build_project_info_args(&params);
    assert!(args.contains(&"--config".to_string()));
    assert!(args.contains(&".fallowrc.json".to_string()));
    assert!(!args.contains(&"--root".to_string()));
}

// ── Global flags: baseline and threads in isolation ───────────────

#[test]
fn analyze_args_baseline_only() {
    let params = AnalyzeParams {
        baseline: Some("baseline.json".to_string()),
        ..Default::default()
    };
    let args = build_analyze_args(&params).unwrap();
    assert!(args.contains(&"--baseline".to_string()));
    assert!(args.contains(&"baseline.json".to_string()));
    assert!(!args.contains(&"--save-baseline".to_string()));
}

#[test]
fn analyze_args_threads_only() {
    let params = AnalyzeParams {
        threads: Some(16),
        ..Default::default()
    };
    let args = build_analyze_args(&params).unwrap();
    assert!(args.contains(&"--threads".to_string()));
    assert!(args.contains(&"16".to_string()));
}

#[test]
fn find_dupes_args_config_and_workspace() {
    let params = FindDupesParams {
        config: Some("custom.toml".to_string()),
        workspace: Some("libs/core".to_string()),
        ..Default::default()
    };
    let args = build_find_dupes_args(&params).unwrap();
    assert!(args.contains(&"--config".to_string()));
    assert!(args.contains(&"custom.toml".to_string()));
    assert!(args.contains(&"--workspace".to_string()));
    assert!(args.contains(&"libs/core".to_string()));
}

#[test]
fn fix_args_workspace_only() {
    let params = FixParams {
        workspace: Some("@my/pkg".to_string()),
        ..Default::default()
    };
    let preview = build_fix_preview_args(&params);
    assert!(preview.contains(&"--workspace".to_string()));
    assert!(preview.contains(&"@my/pkg".to_string()));

    let apply = build_fix_apply_args(&params);
    assert!(apply.contains(&"--workspace".to_string()));
    assert!(apply.contains(&"@my/pkg".to_string()));
}

#[test]
fn health_args_config_only() {
    let params = HealthParams {
        config: Some("health.toml".to_string()),
        ..Default::default()
    };
    let args = build_health_args(&params);
    assert!(args.contains(&"--config".to_string()));
    assert!(args.contains(&"health.toml".to_string()));
}

#[test]
fn health_args_baseline_and_save_baseline() {
    let params = HealthParams {
        baseline: Some("old.json".to_string()),
        save_baseline: Some("new.json".to_string()),
        ..Default::default()
    };
    let args = build_health_args(&params);
    assert!(args.contains(&"--baseline".to_string()));
    assert!(args.contains(&"old.json".to_string()));
    assert!(args.contains(&"--save-baseline".to_string()));
    assert!(args.contains(&"new.json".to_string()));
}
