use crate::params::CheckChangedParams;

/// Build CLI arguments for the `check_changed` tool.
pub fn build_check_changed_args(params: CheckChangedParams) -> Vec<String> {
    let mut args = vec![
        "check".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--quiet".to_string(),
        "--explain".to_string(),
        "--changed-since".to_string(),
        params.since,
    ];

    if let Some(ref root) = params.root {
        args.extend(["--root".to_string(), root.clone()]);
    }
    if let Some(ref config) = params.config {
        args.extend(["--config".to_string(), config.clone()]);
    }
    if params.production == Some(true) {
        args.push("--production".to_string());
    }
    if let Some(ref workspace) = params.workspace {
        args.extend(["--workspace".to_string(), workspace.clone()]);
    }

    args
}
