use crate::params::ProjectInfoParams;

/// Build CLI arguments for the `project_info` tool.
pub fn build_project_info_args(params: &ProjectInfoParams) -> Vec<String> {
    let mut args = vec![
        "list".to_string(),
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
    if params.no_cache == Some(true) {
        args.push("--no-cache".to_string());
    }
    if let Some(threads) = params.threads {
        args.extend(["--threads".to_string(), threads.to_string()]);
    }

    args
}
