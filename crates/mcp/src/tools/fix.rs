use crate::params::FixParams;

/// Build CLI arguments for the `fix_preview` tool.
pub fn build_fix_preview_args(params: &FixParams) -> Vec<String> {
    let mut args = vec![
        "fix".to_string(),
        "--dry-run".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--quiet".to_string(),
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

    args
}

/// Build CLI arguments for the `fix_apply` tool.
pub fn build_fix_apply_args(params: &FixParams) -> Vec<String> {
    let mut args = vec![
        "fix".to_string(),
        "--yes".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--quiet".to_string(),
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

    args
}
