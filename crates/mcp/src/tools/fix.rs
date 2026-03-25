use crate::params::FixParams;

/// Append shared fix flags to the argument list.
fn push_fix_common(args: &mut Vec<String>, params: &FixParams) {
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
    if params.no_cache == Some(true) {
        args.push("--no-cache".to_string());
    }
    if let Some(threads) = params.threads {
        args.extend(["--threads".to_string(), threads.to_string()]);
    }
}

/// Build CLI arguments for the `fix_preview` tool.
pub fn build_fix_preview_args(params: &FixParams) -> Vec<String> {
    let mut args = vec![
        "fix".to_string(),
        "--dry-run".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--quiet".to_string(),
    ];
    push_fix_common(&mut args, params);
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
    push_fix_common(&mut args, params);
    args
}
