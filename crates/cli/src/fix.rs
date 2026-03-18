use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fallow_config::OutputFormat;

/// Atomically write content to a file via a temporary file and rename.
fn atomic_write(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let tmp_path = path.with_extension("fallow-tmp");
    let result = (|| -> std::io::Result<()> {
        let mut file = std::fs::File::create(&tmp_path)?;
        file.write_all(content)?;
        file.sync_all()?;
        std::fs::rename(&tmp_path, path)?;
        Ok(())
    })();

    if result.is_err() {
        // Clean up temp file if write or rename failed
        let _ = std::fs::remove_file(&tmp_path);
    }

    result
}

struct ExportFix {
    line_idx: usize,
    export_name: String,
}

/// Apply export fixes to source files, returning JSON fix entries.
fn apply_export_fixes(
    root: &Path,
    exports_by_file: &HashMap<PathBuf, Vec<&fallow_core::results::UnusedExport>>,
    output: &OutputFormat,
    dry_run: bool,
    fixes: &mut Vec<serde_json::Value>,
) -> bool {
    let mut had_write_error = false;

    for (path, file_exports) in exports_by_file {
        // Security: ensure path is within project root
        if !path.starts_with(root) {
            tracing::warn!(path = %path.display(), "Skipping fix for path outside project root");
            continue;
        }
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        // Detect line ending style
        let line_ending = if content.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let lines: Vec<&str> = content.split(line_ending).collect();

        let mut line_fixes: Vec<ExportFix> = Vec::new();
        for export in file_exports {
            // Use the 1-indexed line field from the export directly
            let line_idx = export.line.saturating_sub(1) as usize;

            if line_idx >= lines.len() {
                continue;
            }

            let line = lines[line_idx];
            let trimmed = line.trim_start();

            // Skip lines that don't start with "export "
            if !trimmed.starts_with("export ") {
                continue;
            }

            let after_export = trimmed.strip_prefix("export ").unwrap_or(trimmed);

            // Handle `export default` cases
            if after_export.starts_with("default ") {
                let after_default = after_export
                    .strip_prefix("default ")
                    .unwrap_or(after_export);
                if after_default.starts_with("function ")
                    || after_default.starts_with("async function ")
                    || after_default.starts_with("class ")
                    || after_default.starts_with("abstract class ")
                {
                    // `export default function Foo` -> `function Foo`
                    // `export default async function Foo` -> `async function Foo`
                    // `export default class Foo` -> `class Foo`
                    // `export default abstract class Foo` -> `abstract class Foo`
                    // handled below via line_fixes
                } else {
                    // `export default expression` -> skip (can't safely remove)
                    continue;
                }
            }

            line_fixes.push(ExportFix {
                line_idx,
                export_name: export.export_name.clone(),
            });
        }

        if line_fixes.is_empty() {
            continue;
        }

        // Sort by line index descending so we can work backwards without shifting indices
        line_fixes.sort_by(|a, b| b.line_idx.cmp(&a.line_idx));

        // Deduplicate by line_idx (multiple exports on the same line shouldn't be applied twice)
        line_fixes.dedup_by_key(|f| f.line_idx);

        let relative = path.strip_prefix(root).unwrap_or(path);

        if dry_run {
            for fix in &line_fixes {
                if !matches!(output, OutputFormat::Json) {
                    eprintln!(
                        "Would remove export from {}:{} `{}`",
                        relative.display(),
                        fix.line_idx + 1,
                        fix.export_name,
                    );
                }
                fixes.push(serde_json::json!({
                    "type": "remove_export",
                    "path": relative.display().to_string(),
                    "line": fix.line_idx + 1,
                    "name": fix.export_name,
                }));
            }
        } else {
            // Apply all fixes to a single in-memory copy
            let mut new_lines: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
            for fix in &line_fixes {
                let line = &new_lines[fix.line_idx];
                let indent = line.len() - line.trim_start().len();
                let trimmed = line.trim_start();
                let after_export = trimmed.strip_prefix("export ").unwrap_or(trimmed);

                let replacement = if after_export.starts_with("default function ")
                    || after_export.starts_with("default async function ")
                    || after_export.starts_with("default class ")
                    || after_export.starts_with("default abstract class ")
                {
                    // `export default function Foo` -> `function Foo`
                    after_export
                        .strip_prefix("default ")
                        .unwrap_or(after_export)
                } else {
                    after_export
                };

                new_lines[fix.line_idx] = format!("{}{}", &" ".repeat(indent), replacement);
            }
            let mut new_content = new_lines.join(line_ending);
            if content.ends_with(line_ending) && !new_content.ends_with(line_ending) {
                new_content.push_str(line_ending);
            }

            let success = match atomic_write(path, new_content.as_bytes()) {
                Ok(()) => true,
                Err(e) => {
                    had_write_error = true;
                    eprintln!("Error: failed to write {}: {e}", relative.display());
                    false
                }
            };

            for fix in &line_fixes {
                fixes.push(serde_json::json!({
                    "type": "remove_export",
                    "path": relative.display().to_string(),
                    "line": fix.line_idx + 1,
                    "name": fix.export_name,
                    "applied": success,
                }));
            }
        }
    }

    had_write_error
}

/// Apply dependency fixes to package.json files (root and workspace), returning JSON fix entries.
fn apply_dependency_fixes(
    root: &Path,
    results: &fallow_core::results::AnalysisResults,
    output: &OutputFormat,
    dry_run: bool,
    fixes: &mut Vec<serde_json::Value>,
) -> bool {
    let mut had_write_error = false;

    if results.unused_dependencies.is_empty() && results.unused_dev_dependencies.is_empty() {
        return had_write_error;
    }

    // Group all unused deps by their package.json path so we can batch edits per file
    let mut deps_by_pkg: std::collections::HashMap<&Path, Vec<(&str, &str)>> =
        std::collections::HashMap::new();
    for dep in &results.unused_dependencies {
        deps_by_pkg
            .entry(&dep.path)
            .or_default()
            .push((&dep.package_name, "dependencies"));
    }
    for dep in &results.unused_dev_dependencies {
        deps_by_pkg
            .entry(&dep.path)
            .or_default()
            .push((&dep.package_name, "devDependencies"));
    }

    let _ = root; // root was previously used to construct the path; now deps carry their own path

    for (pkg_path, removals) in &deps_by_pkg {
        if let Ok(content) = std::fs::read_to_string(pkg_path)
            && let Ok(mut pkg_value) = serde_json::from_str::<serde_json::Value>(&content)
        {
            let mut changed = false;

            for &(package_name, location) in removals {
                if let Some(deps) = pkg_value.get_mut(location)
                    && let Some(obj) = deps.as_object_mut()
                    && obj.remove(package_name).is_some()
                {
                    if dry_run {
                        if !matches!(output, OutputFormat::Json) {
                            eprintln!(
                                "Would remove `{package_name}` from {location} in {}",
                                pkg_path.display()
                            );
                        }
                        fixes.push(serde_json::json!({
                            "type": "remove_dependency",
                            "package": package_name,
                            "location": location,
                            "file": pkg_path.display().to_string(),
                        }));
                    } else {
                        changed = true;
                        fixes.push(serde_json::json!({
                            "type": "remove_dependency",
                            "package": package_name,
                            "location": location,
                            "file": pkg_path.display().to_string(),
                            "applied": true,
                        }));
                    }
                }
            }

            if changed && !dry_run {
                match serde_json::to_string_pretty(&pkg_value) {
                    Ok(new_json) => {
                        let pkg_content = new_json + "\n";
                        if let Err(e) = atomic_write(pkg_path, pkg_content.as_bytes()) {
                            had_write_error = true;
                            eprintln!("Error: failed to write {}: {e}", pkg_path.display());
                        }
                    }
                    Err(e) => {
                        had_write_error = true;
                        eprintln!("Error: failed to serialize {}: {e}", pkg_path.display());
                    }
                }
            }
        }
    }

    had_write_error
}

pub(crate) fn run_fix(
    root: &Path,
    config_path: &Option<PathBuf>,
    output: OutputFormat,
    no_cache: bool,
    threads: usize,
    quiet: bool,
    dry_run: bool,
) -> ExitCode {
    let config = match super::load_config(root, config_path, OutputFormat::Human, no_cache, threads)
    {
        Ok(c) => c,
        Err(code) => return code,
    };

    let results = match fallow_core::analyze(&config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Analysis error: {e}");
            return ExitCode::from(2);
        }
    };

    if results.total_issues() == 0 {
        if matches!(output, OutputFormat::Json) {
            match serde_json::to_string_pretty(&serde_json::json!({
                "dry_run": dry_run,
                "fixes": [],
                "total_fixed": 0
            })) {
                Ok(json) => println!("{json}"),
                Err(e) => {
                    eprintln!("Error: failed to serialize fix output: {e}");
                    return ExitCode::from(2);
                }
            }
        } else if !quiet {
            eprintln!("No issues to fix.");
        }
        return ExitCode::SUCCESS;
    }

    let mut fixes: Vec<serde_json::Value> = Vec::new();

    // Group exports by file path so we can apply all fixes to a single in-memory copy.
    let mut exports_by_file: HashMap<PathBuf, Vec<&fallow_core::results::UnusedExport>> =
        HashMap::new();
    for export in &results.unused_exports {
        exports_by_file
            .entry(export.path.clone())
            .or_default()
            .push(export);
    }

    let mut had_write_error =
        apply_export_fixes(root, &exports_by_file, &output, dry_run, &mut fixes);

    if apply_dependency_fixes(root, &results, &output, dry_run, &mut fixes) {
        had_write_error = true;
    }

    if matches!(output, OutputFormat::Json) {
        let applied_count = fixes
            .iter()
            .filter(|f| f.get("applied").and_then(|v| v.as_bool()).unwrap_or(false))
            .count();
        match serde_json::to_string_pretty(&serde_json::json!({
            "dry_run": dry_run,
            "fixes": fixes,
            "total_fixed": applied_count
        })) {
            Ok(json) => println!("{json}"),
            Err(e) => {
                eprintln!("Error: failed to serialize fix output: {e}");
                return ExitCode::from(2);
            }
        }
    } else if !quiet {
        let fixed_count = fixes.len();
        if dry_run {
            eprintln!("Dry run complete. No files were modified.");
        } else {
            eprintln!("Fixed {} issue(s).", fixed_count);
        }
    }

    if had_write_error {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    }
}
