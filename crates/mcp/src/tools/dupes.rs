use crate::params::FindDupesParams;

use super::VALID_DUPES_MODES;

/// Build CLI arguments for the `find_dupes` tool.
/// Returns `Err(message)` if an invalid mode is provided.
pub fn build_find_dupes_args(params: &FindDupesParams) -> Result<Vec<String>, String> {
    let mut args = vec![
        "dupes".to_string(),
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
    if let Some(ref workspace) = params.workspace {
        args.extend(["--workspace".to_string(), workspace.clone()]);
    }
    if let Some(ref mode) = params.mode {
        if !VALID_DUPES_MODES.contains(&mode.as_str()) {
            return Err(format!(
                "Invalid mode '{mode}'. Valid values: strict, mild, weak, semantic"
            ));
        }
        args.extend(["--mode".to_string(), mode.clone()]);
    }
    if let Some(min_tokens) = params.min_tokens {
        args.extend(["--min-tokens".to_string(), min_tokens.to_string()]);
    }
    if let Some(min_lines) = params.min_lines {
        args.extend(["--min-lines".to_string(), min_lines.to_string()]);
    }
    if let Some(threshold) = params.threshold {
        args.extend(["--threshold".to_string(), threshold.to_string()]);
    }
    if params.skip_local == Some(true) {
        args.push("--skip-local".to_string());
    }
    if params.cross_language == Some(true) {
        args.push("--cross-language".to_string());
    }
    if let Some(top) = params.top {
        args.extend(["--top".to_string(), top.to_string()]);
    }
    if let Some(ref baseline) = params.baseline {
        args.extend(["--baseline".to_string(), baseline.clone()]);
    }
    if let Some(ref save_baseline) = params.save_baseline {
        args.extend(["--save-baseline".to_string(), save_baseline.clone()]);
    }
    if params.no_cache == Some(true) {
        args.push("--no-cache".to_string());
    }
    if let Some(threads) = params.threads {
        args.extend(["--threads".to_string(), threads.to_string()]);
    }

    Ok(args)
}
