use crate::params::*;
use crate::tools::{
    ISSUE_TYPE_FLAGS, VALID_DUPES_MODES, build_analyze_args, build_check_changed_args,
    build_find_dupes_args, build_fix_apply_args, build_fix_preview_args, build_health_args,
    build_project_info_args,
};

// ── Argument building: analyze ────────────────────────────────────

#[test]
fn analyze_args_minimal_produces_base_args() {
    let params = AnalyzeParams {
        root: None,
        config: None,
        production: None,
        workspace: None,
        issue_types: None,
    };
    let args = build_analyze_args(&params).unwrap();
    assert_eq!(args, ["check", "--format", "json", "--quiet", "--explain"]);
}

#[test]
fn analyze_args_with_all_options() {
    let params = AnalyzeParams {
        root: Some("/my/project".to_string()),
        config: Some("fallow.toml".to_string()),
        production: Some(true),
        workspace: Some("@my/pkg".to_string()),
        issue_types: Some(vec![
            "unused-files".to_string(),
            "unused-exports".to_string(),
        ]),
    };
    let args = build_analyze_args(&params).unwrap();
    assert_eq!(
        args,
        [
            "check",
            "--format",
            "json",
            "--quiet",
            "--explain",
            "--root",
            "/my/project",
            "--config",
            "fallow.toml",
            "--production",
            "--workspace",
            "@my/pkg",
            "--unused-files",
            "--unused-exports",
        ]
    );
}

#[test]
fn analyze_args_production_false_is_omitted() {
    let params = AnalyzeParams {
        root: None,
        config: None,
        production: Some(false),
        workspace: None,
        issue_types: None,
    };
    let args = build_analyze_args(&params).unwrap();
    assert!(!args.contains(&"--production".to_string()));
}

#[test]
fn analyze_args_invalid_issue_type_returns_error() {
    let params = AnalyzeParams {
        root: None,
        config: None,
        production: None,
        workspace: None,
        issue_types: Some(vec!["nonexistent-type".to_string()]),
    };
    let err = build_analyze_args(&params).unwrap_err();
    assert!(err.contains("Unknown issue type 'nonexistent-type'"));
    assert!(err.contains("unused-files"));
}

#[test]
fn analyze_args_all_issue_types_accepted() {
    let all_types: Vec<String> = ISSUE_TYPE_FLAGS
        .iter()
        .map(|&(name, _)| name.to_string())
        .collect();
    let params = AnalyzeParams {
        root: None,
        config: None,
        production: None,
        workspace: None,
        issue_types: Some(all_types),
    };
    let args = build_analyze_args(&params).unwrap();
    for &(_, flag) in ISSUE_TYPE_FLAGS {
        assert!(
            args.contains(&flag.to_string()),
            "missing flag {flag} in args"
        );
    }
}

#[test]
fn analyze_args_mixed_valid_and_invalid_issue_types_fails_on_first_invalid() {
    let params = AnalyzeParams {
        root: None,
        config: None,
        production: None,
        workspace: None,
        issue_types: Some(vec![
            "unused-files".to_string(),
            "bogus".to_string(),
            "unused-deps".to_string(),
        ]),
    };
    let err = build_analyze_args(&params).unwrap_err();
    assert!(err.contains("'bogus'"));
}

#[test]
fn analyze_args_empty_issue_types_vec_produces_no_flags() {
    let params = AnalyzeParams {
        root: None,
        config: None,
        production: None,
        workspace: None,
        issue_types: Some(vec![]),
    };
    let args = build_analyze_args(&params).unwrap();
    assert_eq!(args, ["check", "--format", "json", "--quiet", "--explain"]);
}

// ── Argument building: check_changed ──────────────────────────────

#[test]
fn check_changed_args_includes_since_ref() {
    let params = CheckChangedParams {
        root: None,
        since: "main".to_string(),
        config: None,
        production: None,
        workspace: None,
    };
    let args = build_check_changed_args(params);
    assert_eq!(
        args,
        [
            "check",
            "--format",
            "json",
            "--quiet",
            "--explain",
            "--changed-since",
            "main"
        ]
    );
}

#[test]
fn check_changed_args_with_all_options() {
    let params = CheckChangedParams {
        root: Some("/app".to_string()),
        since: "HEAD~5".to_string(),
        config: Some("custom.json".to_string()),
        production: Some(true),
        workspace: Some("frontend".to_string()),
    };
    let args = build_check_changed_args(params);
    assert_eq!(
        args,
        [
            "check",
            "--format",
            "json",
            "--quiet",
            "--explain",
            "--changed-since",
            "HEAD~5",
            "--root",
            "/app",
            "--config",
            "custom.json",
            "--production",
            "--workspace",
            "frontend",
        ]
    );
}

#[test]
fn check_changed_args_with_commit_sha() {
    let params = CheckChangedParams {
        root: None,
        since: "abc123def456".to_string(),
        config: None,
        production: None,
        workspace: None,
    };
    let args = build_check_changed_args(params);
    assert!(args.contains(&"abc123def456".to_string()));
}

// ── Argument building: find_dupes ─────────────────────────────────

#[test]
fn find_dupes_args_minimal() {
    let params = FindDupesParams {
        root: None,
        mode: None,
        min_tokens: None,
        min_lines: None,
        threshold: None,
        skip_local: None,
        cross_language: None,
        top: None,
    };
    let args = build_find_dupes_args(&params).unwrap();
    assert_eq!(args, ["dupes", "--format", "json", "--quiet", "--explain"]);
}

#[test]
fn find_dupes_args_with_all_options() {
    let params = FindDupesParams {
        root: Some("/repo".to_string()),
        mode: Some("semantic".to_string()),
        min_tokens: Some(100),
        min_lines: Some(10),
        threshold: Some(5.5),
        skip_local: Some(true),
        cross_language: Some(true),
        top: Some(5),
    };
    let args = build_find_dupes_args(&params).unwrap();
    assert_eq!(
        args,
        [
            "dupes",
            "--format",
            "json",
            "--quiet",
            "--explain",
            "--root",
            "/repo",
            "--mode",
            "semantic",
            "--min-tokens",
            "100",
            "--min-lines",
            "10",
            "--threshold",
            "5.5",
            "--skip-local",
            "--cross-language",
            "--top",
            "5",
        ]
    );
}

#[test]
fn find_dupes_args_all_valid_modes_accepted() {
    for mode in VALID_DUPES_MODES {
        let params = FindDupesParams {
            root: None,
            mode: Some(mode.to_string()),
            min_tokens: None,
            min_lines: None,
            threshold: None,
            skip_local: None,
            cross_language: None,
            top: None,
        };
        let args = build_find_dupes_args(&params).unwrap();
        assert!(
            args.contains(&mode.to_string()),
            "mode '{mode}' should be in args"
        );
    }
}

#[test]
fn find_dupes_args_invalid_mode_returns_error() {
    let params = FindDupesParams {
        root: None,
        mode: Some("aggressive".to_string()),
        min_tokens: None,
        min_lines: None,
        threshold: None,
        skip_local: None,
        cross_language: None,
        top: None,
    };
    let err = build_find_dupes_args(&params).unwrap_err();
    assert!(err.contains("Invalid mode 'aggressive'"));
    assert!(err.contains("strict"));
    assert!(err.contains("mild"));
    assert!(err.contains("weak"));
    assert!(err.contains("semantic"));
}

#[test]
fn find_dupes_args_skip_local_false_is_omitted() {
    let params = FindDupesParams {
        root: None,
        mode: None,
        min_tokens: None,
        min_lines: None,
        threshold: None,
        skip_local: Some(false),
        cross_language: None,
        top: None,
    };
    let args = build_find_dupes_args(&params).unwrap();
    assert!(!args.contains(&"--skip-local".to_string()));
}

#[test]
fn find_dupes_args_threshold_zero() {
    let params = FindDupesParams {
        root: None,
        mode: None,
        min_tokens: None,
        min_lines: None,
        threshold: Some(0.0),
        skip_local: None,
        cross_language: None,
        top: None,
    };
    let args = build_find_dupes_args(&params).unwrap();
    assert!(args.contains(&"--threshold".to_string()));
    assert!(args.contains(&"0".to_string()));
}

// ── Argument building: fix_preview vs fix_apply ───────────────────

#[test]
fn fix_preview_args_include_dry_run() {
    let params = FixParams {
        root: None,
        config: None,
        production: None,
    };
    let args = build_fix_preview_args(&params);
    assert!(args.contains(&"--dry-run".to_string()));
    assert!(!args.contains(&"--yes".to_string()));
    assert_eq!(args[0], "fix");
}

#[test]
fn fix_apply_args_include_yes_flag() {
    let params = FixParams {
        root: None,
        config: None,
        production: None,
    };
    let args = build_fix_apply_args(&params);
    assert!(args.contains(&"--yes".to_string()));
    assert!(!args.contains(&"--dry-run".to_string()));
    assert_eq!(args[0], "fix");
}

#[test]
fn fix_preview_args_with_all_options() {
    let params = FixParams {
        root: Some("/app".to_string()),
        config: Some("config.json".to_string()),
        production: Some(true),
    };
    let args = build_fix_preview_args(&params);
    assert_eq!(
        args,
        [
            "fix",
            "--dry-run",
            "--format",
            "json",
            "--quiet",
            "--root",
            "/app",
            "--config",
            "config.json",
            "--production",
        ]
    );
}

#[test]
fn fix_apply_args_with_all_options() {
    let params = FixParams {
        root: Some("/app".to_string()),
        config: Some("config.json".to_string()),
        production: Some(true),
    };
    let args = build_fix_apply_args(&params);
    assert_eq!(
        args,
        [
            "fix",
            "--yes",
            "--format",
            "json",
            "--quiet",
            "--root",
            "/app",
            "--config",
            "config.json",
            "--production",
        ]
    );
}

// ── Argument building: project_info ───────────────────────────────

#[test]
fn project_info_args_minimal() {
    let params = ProjectInfoParams {
        root: None,
        config: None,
    };
    let args = build_project_info_args(&params);
    assert_eq!(args, ["list", "--format", "json", "--quiet"]);
}

#[test]
fn project_info_args_with_root_and_config() {
    let params = ProjectInfoParams {
        root: Some("/workspace".to_string()),
        config: Some("fallow.toml".to_string()),
    };
    let args = build_project_info_args(&params);
    assert_eq!(
        args,
        [
            "list",
            "--format",
            "json",
            "--quiet",
            "--root",
            "/workspace",
            "--config",
            "fallow.toml",
        ]
    );
}

// ── Argument building: health ─────────────────────────────────────

#[test]
fn health_args_minimal() {
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
        production: None,
        workspace: None,
        save_snapshot: None,
    };
    let args = build_health_args(&params);
    assert_eq!(args, ["health", "--format", "json", "--quiet", "--explain"]);
}

#[test]
fn health_args_with_all_options() {
    let params = HealthParams {
        root: Some("/src".to_string()),
        max_cyclomatic: Some(25),
        max_cognitive: Some(15),
        top: Some(20),
        sort: Some("cognitive".to_string()),
        changed_since: Some("develop".to_string()),
        complexity: Some(true),
        file_scores: Some(true),
        hotspots: Some(true),
        targets: None,
        since: Some("6m".to_string()),
        min_commits: Some(5),
        workspace: Some("packages/ui".to_string()),
        production: Some(true),
        save_snapshot: None,
    };
    let args = build_health_args(&params);
    assert_eq!(
        args,
        [
            "health",
            "--format",
            "json",
            "--quiet",
            "--explain",
            "--root",
            "/src",
            "--max-cyclomatic",
            "25",
            "--max-cognitive",
            "15",
            "--top",
            "20",
            "--sort",
            "cognitive",
            "--changed-since",
            "develop",
            "--complexity",
            "--file-scores",
            "--hotspots",
            "--since",
            "6m",
            "--min-commits",
            "5",
            "--workspace",
            "packages/ui",
            "--production",
        ]
    );
}

#[test]
fn health_args_partial_options() {
    let params = HealthParams {
        root: None,
        max_cyclomatic: Some(10),
        max_cognitive: None,
        top: None,
        sort: Some("cyclomatic".to_string()),
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
    assert_eq!(
        args,
        [
            "health",
            "--format",
            "json",
            "--quiet",
            "--explain",
            "--max-cyclomatic",
            "10",
            "--sort",
            "cyclomatic",
        ]
    );
}

// ── All tools produce --format json --quiet ───────────────────────

#[test]
fn all_arg_builders_include_format_json_and_quiet() {
    let analyze = build_analyze_args(&AnalyzeParams {
        root: None,
        config: None,
        production: None,
        workspace: None,
        issue_types: None,
    })
    .unwrap();

    let check_changed = build_check_changed_args(CheckChangedParams {
        root: None,
        since: "main".to_string(),
        config: None,
        production: None,
        workspace: None,
    });

    let dupes = build_find_dupes_args(&FindDupesParams {
        root: None,
        mode: None,
        min_tokens: None,
        min_lines: None,
        threshold: None,
        skip_local: None,
        cross_language: None,
        top: None,
    })
    .unwrap();

    let fix_preview = build_fix_preview_args(&FixParams {
        root: None,
        config: None,
        production: None,
    });

    let fix_apply = build_fix_apply_args(&FixParams {
        root: None,
        config: None,
        production: None,
    });

    let project_info = build_project_info_args(&ProjectInfoParams {
        root: None,
        config: None,
    });

    let health = build_health_args(&HealthParams {
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
        workspace: None,
        production: None,
        save_snapshot: None,
    });

    for (name, args) in [
        ("analyze", &analyze),
        ("check_changed", &check_changed),
        ("find_dupes", &dupes),
        ("fix_preview", &fix_preview),
        ("fix_apply", &fix_apply),
        ("project_info", &project_info),
        ("health", &health),
    ] {
        assert!(
            args.contains(&"--format".to_string()),
            "{name} missing --format"
        );
        assert!(args.contains(&"json".to_string()), "{name} missing json");
        assert!(
            args.contains(&"--quiet".to_string()),
            "{name} missing --quiet"
        );
    }
}

// ── Correct subcommand for each tool ──────────────────────────────

#[test]
fn each_tool_uses_correct_subcommand() {
    let analyze = build_analyze_args(&AnalyzeParams {
        root: None,
        config: None,
        production: None,
        workspace: None,
        issue_types: None,
    })
    .unwrap();
    assert_eq!(analyze[0], "check");

    let changed = build_check_changed_args(CheckChangedParams {
        root: None,
        since: "x".to_string(),
        config: None,
        production: None,
        workspace: None,
    });
    assert_eq!(changed[0], "check");

    let dupes = build_find_dupes_args(&FindDupesParams {
        root: None,
        mode: None,
        min_tokens: None,
        min_lines: None,
        threshold: None,
        skip_local: None,
        cross_language: None,
        top: None,
    })
    .unwrap();
    assert_eq!(dupes[0], "dupes");

    let preview = build_fix_preview_args(&FixParams {
        root: None,
        config: None,
        production: None,
    });
    assert_eq!(preview[0], "fix");

    let apply = build_fix_apply_args(&FixParams {
        root: None,
        config: None,
        production: None,
    });
    assert_eq!(apply[0], "fix");

    let info = build_project_info_args(&ProjectInfoParams {
        root: None,
        config: None,
    });
    assert_eq!(info[0], "list");

    let health = build_health_args(&HealthParams {
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
        workspace: None,
        production: None,
        save_snapshot: None,
    });
    assert_eq!(health[0], "health");
}

// ── Explain flag presence ────────────────────────────────────────

#[test]
fn tools_with_explain_include_flag() {
    let analyze = build_analyze_args(&AnalyzeParams {
        root: None,
        config: None,
        production: None,
        workspace: None,
        issue_types: None,
    })
    .unwrap();
    assert!(
        analyze.contains(&"--explain".to_string()),
        "analyze should include --explain"
    );

    let check_changed = build_check_changed_args(CheckChangedParams {
        root: None,
        since: "main".to_string(),
        config: None,
        production: None,
        workspace: None,
    });
    assert!(
        check_changed.contains(&"--explain".to_string()),
        "check_changed should include --explain"
    );

    let dupes = build_find_dupes_args(&FindDupesParams {
        root: None,
        mode: None,
        min_tokens: None,
        min_lines: None,
        threshold: None,
        skip_local: None,
        cross_language: None,
        top: None,
    })
    .unwrap();
    assert!(
        dupes.contains(&"--explain".to_string()),
        "find_dupes should include --explain"
    );

    let health = build_health_args(&HealthParams {
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
        workspace: None,
        production: None,
        save_snapshot: None,
    });
    assert!(
        health.contains(&"--explain".to_string()),
        "health should include --explain"
    );
}

#[test]
fn fix_tools_do_not_include_explain() {
    let preview = build_fix_preview_args(&FixParams {
        root: None,
        config: None,
        production: None,
    });
    assert!(
        !preview.contains(&"--explain".to_string()),
        "fix_preview should not include --explain"
    );

    let apply = build_fix_apply_args(&FixParams {
        root: None,
        config: None,
        production: None,
    });
    assert!(
        !apply.contains(&"--explain".to_string()),
        "fix_apply should not include --explain"
    );
}

#[test]
fn project_info_does_not_include_explain() {
    let args = build_project_info_args(&ProjectInfoParams {
        root: None,
        config: None,
    });
    assert!(
        !args.contains(&"--explain".to_string()),
        "project_info should not include --explain"
    );
}
