use crate::params::*;
use crate::tools::ISSUE_TYPE_FLAGS;

#[test]
fn issue_type_flags_are_complete() {
    assert_eq!(ISSUE_TYPE_FLAGS.len(), 10);
    for &(name, flag) in ISSUE_TYPE_FLAGS {
        assert!(
            flag.starts_with("--"),
            "flag for {name} should start with --"
        );
    }
}

#[test]
fn analyze_params_deserialize() {
    let json = r#"{"root":"/tmp/project","production":true,"issue_types":["unused-files"]}"#;
    let params: AnalyzeParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.root.as_deref(), Some("/tmp/project"));
    assert_eq!(params.production, Some(true));
    assert_eq!(params.issue_types.unwrap(), vec!["unused-files"]);
}

#[test]
fn analyze_params_minimal() {
    let params: AnalyzeParams = serde_json::from_str("{}").unwrap();
    assert!(params.root.is_none());
    assert!(params.production.is_none());
    assert!(params.issue_types.is_none());
    assert!(params.baseline.is_none());
    assert!(params.save_baseline.is_none());
    assert!(params.no_cache.is_none());
    assert!(params.threads.is_none());
}

#[test]
fn analyze_params_with_global_flags() {
    let json = r#"{
        "baseline": "base.json",
        "save_baseline": "new.json",
        "no_cache": true,
        "threads": 4
    }"#;
    let params: AnalyzeParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.baseline.as_deref(), Some("base.json"));
    assert_eq!(params.save_baseline.as_deref(), Some("new.json"));
    assert_eq!(params.no_cache, Some(true));
    assert_eq!(params.threads, Some(4));
}

#[test]
fn check_changed_params_require_since() {
    let json = "{}";
    let result: Result<CheckChangedParams, _> = serde_json::from_str(json);
    assert!(result.is_err());

    let json = r#"{"since":"main"}"#;
    let params: CheckChangedParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.since, "main");
}

#[test]
fn find_dupes_params_defaults() {
    let params: FindDupesParams = serde_json::from_str("{}").unwrap();
    assert!(params.mode.is_none());
    assert!(params.min_tokens.is_none());
    assert!(params.skip_local.is_none());
    assert!(params.config.is_none());
    assert!(params.workspace.is_none());
    assert!(params.baseline.is_none());
    assert!(params.no_cache.is_none());
    assert!(params.threads.is_none());
}

#[test]
fn fix_params_with_production() {
    let json = r#"{"root":"/tmp","production":true}"#;
    let params: FixParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.production, Some(true));
}

#[test]
fn fix_params_with_global_flags() {
    let json = r#"{"workspace":"frontend","no_cache":true,"threads":2}"#;
    let params: FixParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.workspace.as_deref(), Some("frontend"));
    assert_eq!(params.no_cache, Some(true));
    assert_eq!(params.threads, Some(2));
}

#[test]
fn health_params_all_fields_deserialize() {
    let json = r#"{
        "root": "/project",
        "config": "fallow.toml",
        "max_cyclomatic": 25,
        "max_cognitive": 30,
        "top": 10,
        "sort": "cognitive",
        "changed_since": "HEAD~3",
        "baseline": "base.json",
        "save_baseline": "new.json",
        "no_cache": true,
        "threads": 8
    }"#;
    let params: HealthParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.root.as_deref(), Some("/project"));
    assert_eq!(params.config.as_deref(), Some("fallow.toml"));
    assert_eq!(params.max_cyclomatic, Some(25));
    assert_eq!(params.max_cognitive, Some(30));
    assert_eq!(params.top, Some(10));
    assert_eq!(params.sort.as_deref(), Some("cognitive"));
    assert_eq!(params.changed_since.as_deref(), Some("HEAD~3"));
    assert_eq!(params.baseline.as_deref(), Some("base.json"));
    assert_eq!(params.save_baseline.as_deref(), Some("new.json"));
    assert_eq!(params.no_cache, Some(true));
    assert_eq!(params.threads, Some(8));
}

#[test]
fn health_params_minimal() {
    let params: HealthParams = serde_json::from_str("{}").unwrap();
    assert!(params.root.is_none());
    assert!(params.config.is_none());
    assert!(params.max_cyclomatic.is_none());
    assert!(params.max_cognitive.is_none());
    assert!(params.top.is_none());
    assert!(params.sort.is_none());
    assert!(params.changed_since.is_none());
    assert!(params.baseline.is_none());
    assert!(params.no_cache.is_none());
    assert!(params.threads.is_none());
}

#[test]
fn project_info_params_deserialize() {
    let json = r#"{"root": "/app", "config": ".fallowrc.json"}"#;
    let params: ProjectInfoParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.root.as_deref(), Some("/app"));
    assert_eq!(params.config.as_deref(), Some(".fallowrc.json"));
}

#[test]
fn project_info_params_with_global_flags() {
    let json = r#"{"no_cache": true, "threads": 4}"#;
    let params: ProjectInfoParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.no_cache, Some(true));
    assert_eq!(params.threads, Some(4));
}

#[test]
fn find_dupes_params_all_fields_deserialize() {
    let json = r#"{
        "root": "/project",
        "config": "fallow.toml",
        "workspace": "@my/lib",
        "mode": "strict",
        "min_tokens": 100,
        "min_lines": 10,
        "threshold": 5.5,
        "skip_local": true,
        "top": 5,
        "baseline": "base.json",
        "save_baseline": "new.json",
        "no_cache": true,
        "threads": 4
    }"#;
    let params: FindDupesParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.root.as_deref(), Some("/project"));
    assert_eq!(params.config.as_deref(), Some("fallow.toml"));
    assert_eq!(params.workspace.as_deref(), Some("@my/lib"));
    assert_eq!(params.mode.as_deref(), Some("strict"));
    assert_eq!(params.min_tokens, Some(100));
    assert_eq!(params.min_lines, Some(10));
    assert_eq!(params.threshold, Some(5.5));
    assert_eq!(params.skip_local, Some(true));
    assert_eq!(params.top, Some(5));
    assert_eq!(params.baseline.as_deref(), Some("base.json"));
    assert_eq!(params.save_baseline.as_deref(), Some("new.json"));
    assert_eq!(params.no_cache, Some(true));
    assert_eq!(params.threads, Some(4));
}

#[test]
fn check_changed_params_all_fields_deserialize() {
    let json = r#"{
        "root": "/app",
        "since": "develop",
        "config": "custom.toml",
        "production": true,
        "workspace": "frontend",
        "baseline": "base.json",
        "save_baseline": "new.json",
        "no_cache": true,
        "threads": 2
    }"#;
    let params: CheckChangedParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.root.as_deref(), Some("/app"));
    assert_eq!(params.since, "develop");
    assert_eq!(params.config.as_deref(), Some("custom.toml"));
    assert_eq!(params.production, Some(true));
    assert_eq!(params.workspace.as_deref(), Some("frontend"));
    assert_eq!(params.baseline.as_deref(), Some("base.json"));
    assert_eq!(params.save_baseline.as_deref(), Some("new.json"));
    assert_eq!(params.no_cache, Some(true));
    assert_eq!(params.threads, Some(2));
}

#[test]
fn fix_params_minimal_deserialize() {
    let params: FixParams = serde_json::from_str("{}").unwrap();
    assert!(params.root.is_none());
    assert!(params.config.is_none());
    assert!(params.production.is_none());
    assert!(params.workspace.is_none());
    assert!(params.no_cache.is_none());
    assert!(params.threads.is_none());
}

#[test]
fn project_info_params_minimal_deserialize() {
    let params: ProjectInfoParams = serde_json::from_str("{}").unwrap();
    assert!(params.root.is_none());
    assert!(params.config.is_none());
    assert!(params.no_cache.is_none());
    assert!(params.threads.is_none());
}

#[test]
fn find_dupes_params_with_cross_language_deserialize() {
    let json = r#"{"cross_language": true}"#;
    let params: FindDupesParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.cross_language, Some(true));
}

#[test]
fn health_params_all_boolean_section_flags_deserialize() {
    let json = r#"{
        "complexity": true,
        "file_scores": true,
        "hotspots": true,
        "since": "6m",
        "min_commits": 3,
        "workspace": "ui",
        "production": true
    }"#;
    let params: HealthParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.complexity, Some(true));
    assert_eq!(params.file_scores, Some(true));
    assert_eq!(params.hotspots, Some(true));
    assert_eq!(params.since.as_deref(), Some("6m"));
    assert_eq!(params.min_commits, Some(3));
    assert_eq!(params.workspace.as_deref(), Some("ui"));
    assert_eq!(params.production, Some(true));
}
