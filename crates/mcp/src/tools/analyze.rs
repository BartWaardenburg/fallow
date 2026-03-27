use crate::params::AnalyzeParams;

use super::ISSUE_TYPE_FLAGS;

/// Build CLI arguments for the `analyze` tool.
/// Returns `Err(message)` if an invalid issue type is provided.
pub fn build_analyze_args(params: &AnalyzeParams) -> Result<Vec<String>, String> {
    let mut args = vec![
        "dead-code".to_string(),
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
            if let Some(&(_, flag)) = ISSUE_TYPE_FLAGS.iter().find(|&&(name, _)| name == t) {
                args.push(flag.to_string());
            } else {
                let valid = ISSUE_TYPE_FLAGS
                    .iter()
                    .map(|&(n, _)| n)
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(format!("Unknown issue type '{t}'. Valid values: {valid}"));
            }
        }
    }
    if let Some(ref baseline) = params.baseline {
        args.extend(["--baseline".to_string(), baseline.clone()]);
    }
    if let Some(ref save_baseline) = params.save_baseline {
        args.extend(["--save-baseline".to_string(), save_baseline.clone()]);
    }
    if params.fail_on_regression == Some(true) {
        args.push("--fail-on-regression".to_string());
    }
    if let Some(ref tolerance) = params.tolerance {
        args.extend(["--tolerance".to_string(), tolerance.clone()]);
    }
    if let Some(ref regression_baseline) = params.regression_baseline {
        args.extend([
            "--regression-baseline".to_string(),
            regression_baseline.clone(),
        ]);
    }
    if let Some(ref save_regression_baseline) = params.save_regression_baseline {
        args.extend([
            "--save-regression-baseline".to_string(),
            save_regression_baseline.clone(),
        ]);
    }
    if params.no_cache == Some(true) {
        args.push("--no-cache".to_string());
    }
    if let Some(threads) = params.threads {
        args.extend(["--threads".to_string(), threads.to_string()]);
    }

    Ok(args)
}
