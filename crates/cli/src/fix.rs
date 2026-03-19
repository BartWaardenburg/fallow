use std::collections::HashMap;
use std::io::{IsTerminal, Write};
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

pub(crate) struct FixOptions<'a> {
    pub root: &'a Path,
    pub config_path: &'a Option<PathBuf>,
    pub output: OutputFormat,
    pub no_cache: bool,
    pub threads: usize,
    pub quiet: bool,
    pub dry_run: bool,
    pub yes: bool,
    pub production: bool,
}

pub(crate) fn run_fix(opts: &FixOptions<'_>) -> ExitCode {
    // In non-TTY environments (CI, AI agents), require --yes or --dry-run
    // to prevent accidental destructive operations.
    if !opts.dry_run && !opts.yes && !std::io::stdin().is_terminal() {
        let msg = "fix command requires --yes (or --force) in non-interactive environments. \
                   Use --dry-run to preview changes first, then pass --yes to confirm.";
        return super::emit_error(msg, 2, &opts.output);
    }

    let config = match super::load_config(
        opts.root,
        opts.config_path,
        opts.output.clone(),
        opts.no_cache,
        opts.threads,
        opts.production,
    ) {
        Ok(c) => c,
        Err(code) => return code,
    };

    let results = match fallow_core::analyze(&config) {
        Ok(r) => r,
        Err(e) => {
            return super::emit_error(&format!("Analysis error: {e}"), 2, &opts.output);
        }
    };

    if results.total_issues() == 0 {
        if matches!(opts.output, OutputFormat::Json) {
            match serde_json::to_string_pretty(&serde_json::json!({
                "dry_run": opts.dry_run,
                "fixes": [],
                "total_fixed": 0
            })) {
                Ok(json) => println!("{json}"),
                Err(e) => {
                    eprintln!("Error: failed to serialize fix output: {e}");
                    return ExitCode::from(2);
                }
            }
        } else if !opts.quiet {
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

    let mut had_write_error = apply_export_fixes(
        opts.root,
        &exports_by_file,
        &opts.output,
        opts.dry_run,
        &mut fixes,
    );

    if apply_dependency_fixes(opts.root, &results, &opts.output, opts.dry_run, &mut fixes) {
        had_write_error = true;
    }

    if matches!(opts.output, OutputFormat::Json) {
        let applied_count = fixes
            .iter()
            .filter(|f| f.get("applied").and_then(|v| v.as_bool()).unwrap_or(false))
            .count();
        match serde_json::to_string_pretty(&serde_json::json!({
            "dry_run": opts.dry_run,
            "fixes": fixes,
            "total_fixed": applied_count
        })) {
            Ok(json) => println!("{json}"),
            Err(e) => {
                eprintln!("Error: failed to serialize fix output: {e}");
                return ExitCode::from(2);
            }
        }
    } else if !opts.quiet {
        let fixed_count = fixes.len();
        if opts.dry_run {
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

#[cfg(test)]
mod tests {
    use super::*;
    use fallow_core::results::UnusedExport;

    // ── atomic_write ─────────────────────────────────────────────

    #[test]
    fn atomic_write_creates_file_with_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.ts");
        atomic_write(&path, b"hello world").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello world");
    }

    #[test]
    fn atomic_write_overwrites_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.ts");
        std::fs::write(&path, "old content").unwrap();
        atomic_write(&path, b"new content").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new content");
    }

    #[test]
    fn atomic_write_no_leftover_temp_on_success() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.ts");
        atomic_write(&path, b"data").unwrap();
        let tmp_path = path.with_extension("fallow-tmp");
        assert!(!tmp_path.exists());
    }

    // ── apply_export_fixes (dry run) ─────────────────────────────

    fn make_export(path: &Path, name: &str, line: u32) -> UnusedExport {
        UnusedExport {
            path: path.to_path_buf(),
            export_name: name.to_string(),
            is_type_only: false,
            line,
            col: 0,
            span_start: 0,
            is_re_export: false,
        }
    }

    #[test]
    fn dry_run_export_fix_does_not_modify_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("src/utils.ts");
        std::fs::create_dir_all(root.join("src")).unwrap();
        let original = "export function foo() {}\nexport function bar() {}\n";
        std::fs::write(&file, original).unwrap();

        let export = make_export(&file, "foo", 1);
        let mut exports_by_file: HashMap<PathBuf, Vec<&UnusedExport>> = HashMap::new();
        exports_by_file.insert(file.clone(), vec![&export]);

        let mut fixes = Vec::new();
        apply_export_fixes(
            root,
            &exports_by_file,
            &OutputFormat::Json,
            true,
            &mut fixes,
        );

        // File should not be modified
        assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
        // Fix should be reported
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0]["type"], "remove_export");
        assert_eq!(fixes[0]["name"], "foo");
        assert!(fixes[0].get("applied").is_none());
    }

    #[test]
    fn actual_export_fix_removes_export_keyword() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("utils.ts");
        std::fs::write(&file, "export function foo() {}\nexport const bar = 1;\n").unwrap();

        let export = make_export(&file, "foo", 1);
        let mut exports_by_file: HashMap<PathBuf, Vec<&UnusedExport>> = HashMap::new();
        exports_by_file.insert(file.clone(), vec![&export]);

        let mut fixes = Vec::new();
        let had_error = apply_export_fixes(
            root,
            &exports_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        assert!(!had_error);
        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "function foo() {}\nexport const bar = 1;\n");
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0]["applied"], true);
    }

    #[test]
    fn export_fix_removes_default_from_function() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("component.ts");
        std::fs::write(&file, "export default function App() {}\n").unwrap();

        let export = make_export(&file, "default", 1);
        let mut exports_by_file: HashMap<PathBuf, Vec<&UnusedExport>> = HashMap::new();
        exports_by_file.insert(file.clone(), vec![&export]);

        let mut fixes = Vec::new();
        apply_export_fixes(
            root,
            &exports_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "function App() {}\n");
    }

    #[test]
    fn export_fix_removes_default_from_class() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("service.ts");
        std::fs::write(&file, "export default class MyService {}\n").unwrap();

        let export = make_export(&file, "default", 1);
        let mut exports_by_file: HashMap<PathBuf, Vec<&UnusedExport>> = HashMap::new();
        exports_by_file.insert(file.clone(), vec![&export]);

        let mut fixes = Vec::new();
        apply_export_fixes(
            root,
            &exports_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "class MyService {}\n");
    }

    #[test]
    fn export_fix_removes_default_from_abstract_class() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("base.ts");
        std::fs::write(&file, "export default abstract class Base {}\n").unwrap();

        let export = make_export(&file, "default", 1);
        let mut exports_by_file: HashMap<PathBuf, Vec<&UnusedExport>> = HashMap::new();
        exports_by_file.insert(file.clone(), vec![&export]);

        let mut fixes = Vec::new();
        apply_export_fixes(
            root,
            &exports_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "abstract class Base {}\n");
    }

    #[test]
    fn export_fix_removes_default_from_async_function() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("handler.ts");
        std::fs::write(&file, "export default async function handler() {}\n").unwrap();

        let export = make_export(&file, "default", 1);
        let mut exports_by_file: HashMap<PathBuf, Vec<&UnusedExport>> = HashMap::new();
        exports_by_file.insert(file.clone(), vec![&export]);

        let mut fixes = Vec::new();
        apply_export_fixes(
            root,
            &exports_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "async function handler() {}\n");
    }

    #[test]
    fn export_fix_skips_default_expression_export() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("config.ts");
        let original = "export default { key: 'value' };\n";
        std::fs::write(&file, original).unwrap();

        let export = make_export(&file, "default", 1);
        let mut exports_by_file: HashMap<PathBuf, Vec<&UnusedExport>> = HashMap::new();
        exports_by_file.insert(file.clone(), vec![&export]);

        let mut fixes = Vec::new();
        apply_export_fixes(
            root,
            &exports_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        // File unchanged — expression defaults are not safely removable
        assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
        assert!(fixes.is_empty());
    }

    #[test]
    fn export_fix_preserves_indentation() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("mod.ts");
        std::fs::write(&file, "  export const x = 1;\n").unwrap();

        let export = make_export(&file, "x", 1);
        let mut exports_by_file: HashMap<PathBuf, Vec<&UnusedExport>> = HashMap::new();
        exports_by_file.insert(file.clone(), vec![&export]);

        let mut fixes = Vec::new();
        apply_export_fixes(
            root,
            &exports_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "  const x = 1;\n");
    }

    #[test]
    fn export_fix_preserves_crlf_line_endings() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("win.ts");
        std::fs::write(
            &file,
            "export function foo() {}\r\nexport function bar() {}\r\n",
        )
        .unwrap();

        let export = make_export(&file, "foo", 1);
        let mut exports_by_file: HashMap<PathBuf, Vec<&UnusedExport>> = HashMap::new();
        exports_by_file.insert(file.clone(), vec![&export]);

        let mut fixes = Vec::new();
        apply_export_fixes(
            root,
            &exports_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "function foo() {}\r\nexport function bar() {}\r\n");
    }

    #[test]
    fn export_fix_skips_path_outside_project_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("project");
        std::fs::create_dir_all(&root).unwrap();
        let outside_file = dir.path().join("outside.ts");
        let original = "export function evil() {}\n";
        std::fs::write(&outside_file, original).unwrap();

        let export = make_export(&outside_file, "evil", 1);
        let mut exports_by_file: HashMap<PathBuf, Vec<&UnusedExport>> = HashMap::new();
        exports_by_file.insert(outside_file.clone(), vec![&export]);

        let mut fixes = Vec::new();
        apply_export_fixes(
            &root,
            &exports_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        // File should be untouched and no fixes generated
        assert_eq!(std::fs::read_to_string(&outside_file).unwrap(), original);
        assert!(fixes.is_empty());
    }

    #[test]
    fn export_fix_skips_line_not_starting_with_export() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("tricky.ts");
        let original = "const foo = 'export something';\n";
        std::fs::write(&file, original).unwrap();

        let export = make_export(&file, "foo", 1);
        let mut exports_by_file: HashMap<PathBuf, Vec<&UnusedExport>> = HashMap::new();
        exports_by_file.insert(file.clone(), vec![&export]);

        let mut fixes = Vec::new();
        apply_export_fixes(
            root,
            &exports_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
        assert!(fixes.is_empty());
    }

    #[test]
    fn export_fix_handles_multiple_exports_in_same_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("multi.ts");
        std::fs::write(
            &file,
            "export function a() {}\nexport const b = 1;\nexport class C {}\n",
        )
        .unwrap();

        let e1 = make_export(&file, "a", 1);
        let e2 = make_export(&file, "C", 3);
        let mut exports_by_file: HashMap<PathBuf, Vec<&UnusedExport>> = HashMap::new();
        exports_by_file.insert(file.clone(), vec![&e1, &e2]);

        let mut fixes = Vec::new();
        apply_export_fixes(
            root,
            &exports_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(
            content,
            "function a() {}\nexport const b = 1;\nclass C {}\n"
        );
        assert_eq!(fixes.len(), 2);
    }

    // ── apply_dependency_fixes ────────────────────────────────────

    #[test]
    fn dependency_fix_dry_run_does_not_modify_package_json() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let pkg_path = root.join("package.json");
        let original =
            r#"{"dependencies": {"lodash": "^4.0.0"}, "devDependencies": {"jest": "^29.0.0"}}"#;
        std::fs::write(&pkg_path, original).unwrap();

        let mut results = fallow_core::results::AnalysisResults::default();
        results
            .unused_dependencies
            .push(fallow_core::results::UnusedDependency {
                package_name: "lodash".into(),
                location: fallow_core::results::DependencyLocation::Dependencies,
                path: pkg_path.clone(),
            });

        let mut fixes = Vec::new();
        apply_dependency_fixes(root, &results, &OutputFormat::Json, true, &mut fixes);

        // package.json should not change
        assert_eq!(std::fs::read_to_string(&pkg_path).unwrap(), original);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0]["type"], "remove_dependency");
        assert_eq!(fixes[0]["package"], "lodash");
    }

    #[test]
    fn dependency_fix_removes_unused_dep_from_package_json() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let pkg_path = root.join("package.json");
        std::fs::write(
            &pkg_path,
            r#"{"dependencies": {"lodash": "^4.0.0", "react": "^18.0.0"}}"#,
        )
        .unwrap();

        let mut results = fallow_core::results::AnalysisResults::default();
        results
            .unused_dependencies
            .push(fallow_core::results::UnusedDependency {
                package_name: "lodash".into(),
                location: fallow_core::results::DependencyLocation::Dependencies,
                path: pkg_path.clone(),
            });

        let mut fixes = Vec::new();
        let had_error =
            apply_dependency_fixes(root, &results, &OutputFormat::Human, false, &mut fixes);

        assert!(!had_error);
        let content = std::fs::read_to_string(&pkg_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        let deps = parsed["dependencies"].as_object().unwrap();
        assert!(!deps.contains_key("lodash"));
        assert!(deps.contains_key("react"));
    }

    #[test]
    fn dependency_fix_empty_results_returns_early() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let results = fallow_core::results::AnalysisResults::default();
        let mut fixes = Vec::new();
        let had_error =
            apply_dependency_fixes(root, &results, &OutputFormat::Human, false, &mut fixes);
        assert!(!had_error);
        assert!(fixes.is_empty());
    }

    #[test]
    fn export_fix_skips_out_of_bounds_line() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("short.ts");
        std::fs::write(&file, "export function a() {}\n").unwrap();

        // Line 999 is way out of bounds
        let export = make_export(&file, "ghost", 999);
        let mut exports_by_file: HashMap<PathBuf, Vec<&UnusedExport>> = HashMap::new();
        exports_by_file.insert(file.clone(), vec![&export]);

        let mut fixes = Vec::new();
        apply_export_fixes(
            root,
            &exports_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        // File unchanged, no fixes
        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "export function a() {}\n");
        assert!(fixes.is_empty());
    }
}
