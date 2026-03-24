use crate::params::AnalyzeParams;

use super::ISSUE_TYPE_FLAGS;

/// Build CLI arguments for the `analyze` tool.
/// Returns `Err(message)` if an invalid issue type is provided.
pub fn build_analyze_args(params: &AnalyzeParams) -> Result<Vec<String>, String> {
    let mut args = vec![
        "check".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--quiet".to_string(),
        "--explain".to_string(),
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
    if let Some(ref types) = params.issue_types {
        for t in types {
            match ISSUE_TYPE_FLAGS.iter().find(|&&(name, _)| name == t) {
                Some(&(_, flag)) => args.push(flag.to_string()),
                None => {
                    let valid = ISSUE_TYPE_FLAGS
                        .iter()
                        .map(|&(n, _)| n)
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(format!("Unknown issue type '{t}'. Valid values: {valid}"));
                }
            }
        }
    }

    Ok(args)
}
