use crate::params::HealthParams;

/// Build CLI arguments for the `check_health` tool.
pub fn build_health_args(params: &HealthParams) -> Vec<String> {
    let mut args = vec![
        "health".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--quiet".to_string(),
        "--explain".to_string(),
    ];

    if let Some(ref root) = params.root {
        args.extend(["--root".to_string(), root.clone()]);
    }
    if let Some(max_cyclomatic) = params.max_cyclomatic {
        args.extend(["--max-cyclomatic".to_string(), max_cyclomatic.to_string()]);
    }
    if let Some(max_cognitive) = params.max_cognitive {
        args.extend(["--max-cognitive".to_string(), max_cognitive.to_string()]);
    }
    if let Some(top) = params.top {
        args.extend(["--top".to_string(), top.to_string()]);
    }
    if let Some(ref sort) = params.sort {
        args.extend(["--sort".to_string(), sort.clone()]);
    }
    if let Some(ref changed_since) = params.changed_since {
        args.extend(["--changed-since".to_string(), changed_since.clone()]);
    }
    if params.complexity == Some(true) {
        args.push("--complexity".to_string());
    }
    if params.file_scores == Some(true) {
        args.push("--file-scores".to_string());
    }
    if params.hotspots == Some(true) {
        args.push("--hotspots".to_string());
    }
    if let Some(ref since) = params.since {
        args.extend(["--since".to_string(), since.clone()]);
    }
    if let Some(min_commits) = params.min_commits {
        args.extend(["--min-commits".to_string(), min_commits.to_string()]);
    }
    if let Some(ref workspace) = params.workspace {
        args.extend(["--workspace".to_string(), workspace.clone()]);
    }
    if params.production == Some(true) {
        args.push("--production".to_string());
    }

    args
}
