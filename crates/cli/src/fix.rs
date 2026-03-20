use rustc_hash::FxHashMap;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fallow_config::OutputFormat;
use tempfile::NamedTempFile;

/// Atomically write content to a file via a temporary file and rename.
fn atomic_write(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut tmp = NamedTempFile::new_in(dir)?;
    tmp.write_all(content)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

struct ExportFix {
    line_idx: usize,
    export_name: String,
}

/// Apply export fixes to source files, returning JSON fix entries.
fn apply_export_fixes(
    root: &Path,
    exports_by_file: &FxHashMap<PathBuf, Vec<&fallow_core::results::UnusedExport>>,
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
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
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

struct EnumMemberFix {
    line_idx: usize,
    member_name: String,
    parent_name: String,
}

/// Apply enum member fixes to source files, returning JSON fix entries.
///
/// Removes unused enum members from their declarations. Handles:
/// - Multi-line enums: removes the entire line containing the member
/// - Single-line enums: removes the member token from the line
/// - Trailing commas: cleans up when the last member is removed
/// - All members removed: leaves the enum body empty (`enum Foo {}`)
fn apply_enum_member_fixes(
    root: &Path,
    members_by_file: &FxHashMap<PathBuf, Vec<&fallow_core::results::UnusedMember>>,
    output: &OutputFormat,
    dry_run: bool,
    fixes: &mut Vec<serde_json::Value>,
) -> bool {
    let mut had_write_error = false;

    for (path, file_members) in members_by_file {
        // Security: ensure path is within project root
        if !path.starts_with(root) {
            tracing::warn!(path = %path.display(), "Skipping fix for path outside project root");
            continue;
        }
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        let line_ending = if content.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let lines: Vec<&str> = content.split(line_ending).collect();

        let mut member_fixes: Vec<EnumMemberFix> = Vec::new();
        for member in file_members {
            let line_idx = member.line.saturating_sub(1) as usize;
            if line_idx >= lines.len() {
                continue;
            }

            // Safety check: the line should contain the member name
            let line = lines[line_idx];
            if !line.contains(&member.member_name) {
                continue;
            }

            member_fixes.push(EnumMemberFix {
                line_idx,
                member_name: member.member_name.clone(),
                parent_name: member.parent_name.clone(),
            });
        }

        if member_fixes.is_empty() {
            continue;
        }

        // Sort by line index descending so we can work backwards
        member_fixes.sort_by(|a, b| b.line_idx.cmp(&a.line_idx));
        // Deduplicate by line_idx
        member_fixes.dedup_by_key(|f| f.line_idx);

        let relative = path.strip_prefix(root).unwrap_or(path);

        if dry_run {
            for fix in &member_fixes {
                if !matches!(output, OutputFormat::Json) {
                    eprintln!(
                        "Would remove enum member from {}:{} `{}.{}`",
                        relative.display(),
                        fix.line_idx + 1,
                        fix.parent_name,
                        fix.member_name,
                    );
                }
                fixes.push(serde_json::json!({
                    "type": "remove_enum_member",
                    "path": relative.display().to_string(),
                    "line": fix.line_idx + 1,
                    "parent": fix.parent_name,
                    "name": fix.member_name,
                }));
            }
        } else {
            let mut new_lines: Vec<String> = lines.iter().map(|l| l.to_string()).collect();

            // Check if this is a single-line enum (opening and closing brace on same line)
            // by looking for patterns like `enum Foo { A, B, C }`
            // We need to handle multi-member single-line enums differently.
            //
            // Build a set of line indices to remove for multi-line enums.
            // For single-line enums, we edit the line in-place.

            // Process fixes in descending line order
            for fix in &member_fixes {
                let line = &new_lines[fix.line_idx];

                // Detect single-line enum: line contains both `{` and `}`
                if line.contains('{') && line.contains('}') {
                    // Single-line enum: remove the member token from the line
                    let new_line = remove_member_from_single_line(line, &fix.member_name);
                    new_lines[fix.line_idx] = new_line;
                } else {
                    // Multi-line enum: mark this line for removal
                    // We remove the line entirely, then fix trailing comma issues
                    new_lines[fix.line_idx] = String::new();
                }
            }

            // For multi-line removals, clean up: remove empty lines and fix trailing commas.
            // We need to find enum bodies and ensure the last member doesn't have a dangling comma issue.
            // Actually, we need to handle a subtlety: if we removed the LAST member in a multi-line
            // enum, the previous member's line now becomes the last one and may not need a trailing comma
            // (though trailing commas in TS enums are always valid, so we leave them).
            //
            // The main task: remove the blank lines we created.
            // We also need to handle the case where ALL members were removed from an enum.

            // Remove blank lines that were marked for deletion, working backwards
            let remove_indices: Vec<usize> = member_fixes
                .iter()
                .filter(|f| {
                    // Only remove lines from multi-line enums (not single-line which were edited in-place)
                    let orig_line = &lines[f.line_idx];
                    !(orig_line.contains('{') && orig_line.contains('}'))
                })
                .map(|f| f.line_idx)
                .collect();

            // Remove in descending order (already sorted)
            for &idx in &remove_indices {
                new_lines.remove(idx);
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

            for fix in &member_fixes {
                fixes.push(serde_json::json!({
                    "type": "remove_enum_member",
                    "path": relative.display().to_string(),
                    "line": fix.line_idx + 1,
                    "parent": fix.parent_name,
                    "name": fix.member_name,
                    "applied": success,
                }));
            }
        }
    }

    had_write_error
}

/// Remove a single member from a single-line enum like `enum Foo { A, B, C }`.
///
/// Returns the modified line with the member removed and commas cleaned up.
fn remove_member_from_single_line(line: &str, member_name: &str) -> String {
    // Find the content between { and }
    let Some(open) = line.find('{') else {
        return line.to_string();
    };
    let Some(close) = line.rfind('}') else {
        return line.to_string();
    };
    if open >= close {
        return line.to_string();
    }

    let prefix = &line[..=open];
    let suffix = &line[close..];
    let inner = &line[open + 1..close];

    // Split inner by comma to get individual member tokens
    let parts: Vec<&str> = inner.split(',').collect();

    // Filter out the part that matches the member name.
    // A member part might be " Active", " Active = 'active'", etc.
    let filtered: Vec<String> = parts
        .iter()
        .filter(|part| {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                return false;
            }
            // Extract just the identifier name (before any `=` sign)
            let ident = trimmed.split('=').next().unwrap_or(trimmed).trim();
            ident != member_name
        })
        .map(|part| part.trim().to_string())
        .collect();

    if filtered.is_empty() {
        // All members removed — leave empty enum body: `enum Foo {}`
        format!("{}{}", prefix.trim_end(), suffix.trim_start(),)
    } else {
        // Reconstruct with consistent formatting: `{ A, B }`
        let members_str = filtered.join(", ");
        format!("{prefix} {members_str} {suffix}")
    }
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
    let mut deps_by_pkg: FxHashMap<&Path, Vec<(&str, &str)>> = FxHashMap::default();
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

#[allow(clippy::struct_excessive_bools)]
pub struct FixOptions<'a> {
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

pub fn run_fix(opts: &FixOptions<'_>) -> ExitCode {
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
    let mut exports_by_file: FxHashMap<PathBuf, Vec<&fallow_core::results::UnusedExport>> =
        FxHashMap::default();
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

    // Group unused enum members by file path for batch editing.
    if !results.unused_enum_members.is_empty() {
        let mut enum_members_by_file: FxHashMap<PathBuf, Vec<&fallow_core::results::UnusedMember>> =
            FxHashMap::default();
        for member in &results.unused_enum_members {
            enum_members_by_file
                .entry(member.path.clone())
                .or_default()
                .push(member);
        }

        if apply_enum_member_fixes(
            opts.root,
            &enum_members_by_file,
            &opts.output,
            opts.dry_run,
            &mut fixes,
        ) {
            had_write_error = true;
        }
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
            eprintln!("Fixed {fixed_count} issue(s).");
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
    use fallow_core::extract::MemberKind;
    use fallow_core::results::{UnusedExport, UnusedMember};

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
        // Only the target file should exist — no stray temp files
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name(), "test.ts");
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
        let mut exports_by_file: FxHashMap<PathBuf, Vec<&UnusedExport>> = FxHashMap::default();
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
        let mut exports_by_file: FxHashMap<PathBuf, Vec<&UnusedExport>> = FxHashMap::default();
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
        let mut exports_by_file: FxHashMap<PathBuf, Vec<&UnusedExport>> = FxHashMap::default();
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
        let mut exports_by_file: FxHashMap<PathBuf, Vec<&UnusedExport>> = FxHashMap::default();
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
        let mut exports_by_file: FxHashMap<PathBuf, Vec<&UnusedExport>> = FxHashMap::default();
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
        let mut exports_by_file: FxHashMap<PathBuf, Vec<&UnusedExport>> = FxHashMap::default();
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
        let mut exports_by_file: FxHashMap<PathBuf, Vec<&UnusedExport>> = FxHashMap::default();
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
        let mut exports_by_file: FxHashMap<PathBuf, Vec<&UnusedExport>> = FxHashMap::default();
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
        let mut exports_by_file: FxHashMap<PathBuf, Vec<&UnusedExport>> = FxHashMap::default();
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
        let mut exports_by_file: FxHashMap<PathBuf, Vec<&UnusedExport>> = FxHashMap::default();
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
        let mut exports_by_file: FxHashMap<PathBuf, Vec<&UnusedExport>> = FxHashMap::default();
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
        let mut exports_by_file: FxHashMap<PathBuf, Vec<&UnusedExport>> = FxHashMap::default();
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
        let mut exports_by_file: FxHashMap<PathBuf, Vec<&UnusedExport>> = FxHashMap::default();
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

    // ── apply_enum_member_fixes ─────────────────────────────────

    fn make_enum_member(path: &Path, parent: &str, name: &str, line: u32) -> UnusedMember {
        UnusedMember {
            path: path.to_path_buf(),
            parent_name: parent.to_string(),
            member_name: name.to_string(),
            kind: MemberKind::EnumMember,
            line,
            col: 0,
        }
    }

    #[test]
    fn enum_fix_removes_single_member_from_multi_member_enum() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("status.ts");
        std::fs::write(
            &file,
            "export enum Status {\n  Active,\n  Inactive,\n  Pending,\n}\n",
        )
        .unwrap();

        let member = make_enum_member(&file, "Status", "Inactive", 3);
        let mut members_by_file: FxHashMap<PathBuf, Vec<&UnusedMember>> = FxHashMap::default();
        members_by_file.insert(file.clone(), vec![&member]);

        let mut fixes = Vec::new();
        let had_error = apply_enum_member_fixes(
            root,
            &members_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        assert!(!had_error);
        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "export enum Status {\n  Active,\n  Pending,\n}\n");
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0]["type"], "remove_enum_member");
        assert_eq!(fixes[0]["parent"], "Status");
        assert_eq!(fixes[0]["name"], "Inactive");
        assert_eq!(fixes[0]["applied"], true);
    }

    #[test]
    fn enum_fix_removes_multiple_members_from_same_enum() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("status.ts");
        std::fs::write(
            &file,
            "export enum Status {\n  Active,\n  Inactive,\n  Pending,\n}\n",
        )
        .unwrap();

        let m1 = make_enum_member(&file, "Status", "Active", 2);
        let m2 = make_enum_member(&file, "Status", "Pending", 4);
        let mut members_by_file: FxHashMap<PathBuf, Vec<&UnusedMember>> = FxHashMap::default();
        members_by_file.insert(file.clone(), vec![&m1, &m2]);

        let mut fixes = Vec::new();
        apply_enum_member_fixes(
            root,
            &members_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "export enum Status {\n  Inactive,\n}\n");
        assert_eq!(fixes.len(), 2);
    }

    #[test]
    fn enum_fix_removes_all_members_leaves_empty_body() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("status.ts");
        std::fs::write(&file, "export enum Status {\n  Active,\n  Inactive,\n}\n").unwrap();

        let m1 = make_enum_member(&file, "Status", "Active", 2);
        let m2 = make_enum_member(&file, "Status", "Inactive", 3);
        let mut members_by_file: FxHashMap<PathBuf, Vec<&UnusedMember>> = FxHashMap::default();
        members_by_file.insert(file.clone(), vec![&m1, &m2]);

        let mut fixes = Vec::new();
        apply_enum_member_fixes(
            root,
            &members_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "export enum Status {\n}\n");
        assert_eq!(fixes.len(), 2);
    }

    #[test]
    fn enum_fix_handles_members_with_values() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("status.ts");
        std::fs::write(
            &file,
            "export enum Status {\n  Active = \"active\",\n  Inactive = \"inactive\",\n  Pending = 2,\n}\n",
        )
        .unwrap();

        let member = make_enum_member(&file, "Status", "Inactive", 3);
        let mut members_by_file: FxHashMap<PathBuf, Vec<&UnusedMember>> = FxHashMap::default();
        members_by_file.insert(file.clone(), vec![&member]);

        let mut fixes = Vec::new();
        apply_enum_member_fixes(
            root,
            &members_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(
            content,
            "export enum Status {\n  Active = \"active\",\n  Pending = 2,\n}\n"
        );
    }

    #[test]
    fn enum_fix_single_line_enum() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("status.ts");
        std::fs::write(&file, "enum Status { Active, Inactive, Pending }\n").unwrap();

        let member = make_enum_member(&file, "Status", "Inactive", 1);
        let mut members_by_file: FxHashMap<PathBuf, Vec<&UnusedMember>> = FxHashMap::default();
        members_by_file.insert(file.clone(), vec![&member]);

        let mut fixes = Vec::new();
        apply_enum_member_fixes(
            root,
            &members_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "enum Status { Active, Pending }\n");
    }

    #[test]
    fn enum_fix_single_line_removes_all_members() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("status.ts");
        std::fs::write(&file, "enum Status { Active }\n").unwrap();

        let member = make_enum_member(&file, "Status", "Active", 1);
        let mut members_by_file: FxHashMap<PathBuf, Vec<&UnusedMember>> = FxHashMap::default();
        members_by_file.insert(file.clone(), vec![&member]);

        let mut fixes = Vec::new();
        apply_enum_member_fixes(
            root,
            &members_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "enum Status {}\n");
    }

    #[test]
    fn enum_fix_single_line_with_values() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("status.ts");
        std::fs::write(
            &file,
            "enum Status { Active = \"active\", Inactive = \"inactive\" }\n",
        )
        .unwrap();

        let member = make_enum_member(&file, "Status", "Active", 1);
        let mut members_by_file: FxHashMap<PathBuf, Vec<&UnusedMember>> = FxHashMap::default();
        members_by_file.insert(file.clone(), vec![&member]);

        let mut fixes = Vec::new();
        apply_enum_member_fixes(
            root,
            &members_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "enum Status { Inactive = \"inactive\" }\n");
    }

    #[test]
    fn enum_fix_dry_run_does_not_modify_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("status.ts");
        let original = "export enum Status {\n  Active,\n  Inactive,\n}\n";
        std::fs::write(&file, original).unwrap();

        let member = make_enum_member(&file, "Status", "Active", 2);
        let mut members_by_file: FxHashMap<PathBuf, Vec<&UnusedMember>> = FxHashMap::default();
        members_by_file.insert(file.clone(), vec![&member]);

        let mut fixes = Vec::new();
        apply_enum_member_fixes(
            root,
            &members_by_file,
            &OutputFormat::Json,
            true,
            &mut fixes,
        );

        assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0]["type"], "remove_enum_member");
        assert_eq!(fixes[0]["name"], "Active");
        assert!(fixes[0].get("applied").is_none());
    }

    #[test]
    fn enum_fix_preserves_crlf_line_endings() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("status.ts");
        std::fs::write(
            &file,
            "export enum Status {\r\n  Active,\r\n  Inactive,\r\n  Pending,\r\n}\r\n",
        )
        .unwrap();

        let member = make_enum_member(&file, "Status", "Inactive", 3);
        let mut members_by_file: FxHashMap<PathBuf, Vec<&UnusedMember>> = FxHashMap::default();
        members_by_file.insert(file.clone(), vec![&member]);

        let mut fixes = Vec::new();
        apply_enum_member_fixes(
            root,
            &members_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(
            content,
            "export enum Status {\r\n  Active,\r\n  Pending,\r\n}\r\n"
        );
    }

    #[test]
    fn enum_fix_preserves_indentation() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("status.ts");
        std::fs::write(
            &file,
            "    export enum Status {\n        Active,\n        Inactive,\n    }\n",
        )
        .unwrap();

        let member = make_enum_member(&file, "Status", "Active", 2);
        let mut members_by_file: FxHashMap<PathBuf, Vec<&UnusedMember>> = FxHashMap::default();
        members_by_file.insert(file.clone(), vec![&member]);

        let mut fixes = Vec::new();
        apply_enum_member_fixes(
            root,
            &members_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(
            content,
            "    export enum Status {\n        Inactive,\n    }\n"
        );
    }

    #[test]
    fn enum_fix_skips_path_outside_project_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("project");
        std::fs::create_dir_all(&root).unwrap();
        let outside_file = dir.path().join("outside.ts");
        let original = "enum Status {\n  Active,\n  Inactive,\n}\n";
        std::fs::write(&outside_file, original).unwrap();

        let member = make_enum_member(&outside_file, "Status", "Active", 2);
        let mut members_by_file: FxHashMap<PathBuf, Vec<&UnusedMember>> = FxHashMap::default();
        members_by_file.insert(outside_file.clone(), vec![&member]);

        let mut fixes = Vec::new();
        apply_enum_member_fixes(
            &root,
            &members_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        assert_eq!(std::fs::read_to_string(&outside_file).unwrap(), original);
        assert!(fixes.is_empty());
    }

    #[test]
    fn enum_fix_skips_line_without_member_name() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("status.ts");
        let original = "enum Status {\n  Active,\n  Inactive,\n}\n";
        std::fs::write(&file, original).unwrap();

        // Point at line 2 (Active), but claim the member name is "Missing"
        let member = make_enum_member(&file, "Status", "Missing", 2);
        let mut members_by_file: FxHashMap<PathBuf, Vec<&UnusedMember>> = FxHashMap::default();
        members_by_file.insert(file.clone(), vec![&member]);

        let mut fixes = Vec::new();
        apply_enum_member_fixes(
            root,
            &members_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
        assert!(fixes.is_empty());
    }

    #[test]
    fn enum_fix_skips_out_of_bounds_line() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("status.ts");
        let original = "enum Status {\n  Active,\n}\n";
        std::fs::write(&file, original).unwrap();

        let member = make_enum_member(&file, "Status", "Active", 999);
        let mut members_by_file: FxHashMap<PathBuf, Vec<&UnusedMember>> = FxHashMap::default();
        members_by_file.insert(file.clone(), vec![&member]);

        let mut fixes = Vec::new();
        apply_enum_member_fixes(
            root,
            &members_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
        assert!(fixes.is_empty());
    }

    #[test]
    fn enum_fix_removes_last_member_of_multi_line_enum() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("status.ts");
        std::fs::write(&file, "enum Status {\n  Active,\n  Inactive,\n}\n").unwrap();

        // Remove the last member
        let member = make_enum_member(&file, "Status", "Inactive", 3);
        let mut members_by_file: FxHashMap<PathBuf, Vec<&UnusedMember>> = FxHashMap::default();
        members_by_file.insert(file.clone(), vec![&member]);

        let mut fixes = Vec::new();
        apply_enum_member_fixes(
            root,
            &members_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "enum Status {\n  Active,\n}\n");
    }

    #[test]
    fn enum_fix_handles_numeric_values() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let file = root.join("priority.ts");
        std::fs::write(
            &file,
            "enum Priority {\n  Low = 0,\n  Medium = 1,\n  High = 2,\n}\n",
        )
        .unwrap();

        let member = make_enum_member(&file, "Priority", "Medium", 3);
        let mut members_by_file: FxHashMap<PathBuf, Vec<&UnusedMember>> = FxHashMap::default();
        members_by_file.insert(file.clone(), vec![&member]);

        let mut fixes = Vec::new();
        apply_enum_member_fixes(
            root,
            &members_by_file,
            &OutputFormat::Human,
            false,
            &mut fixes,
        );

        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "enum Priority {\n  Low = 0,\n  High = 2,\n}\n");
    }

    // ── remove_member_from_single_line unit tests ───────────────

    #[test]
    fn single_line_remove_first_member() {
        let result = remove_member_from_single_line("enum Foo { A, B, C }", "A");
        assert_eq!(result, "enum Foo { B, C }");
    }

    #[test]
    fn single_line_remove_middle_member() {
        let result = remove_member_from_single_line("enum Foo { A, B, C }", "B");
        assert_eq!(result, "enum Foo { A, C }");
    }

    #[test]
    fn single_line_remove_last_member() {
        let result = remove_member_from_single_line("enum Foo { A, B, C }", "C");
        assert_eq!(result, "enum Foo { A, B }");
    }

    #[test]
    fn single_line_remove_only_member() {
        let result = remove_member_from_single_line("enum Foo { A }", "A");
        assert_eq!(result, "enum Foo {}");
    }

    #[test]
    fn single_line_remove_member_with_value() {
        let result = remove_member_from_single_line("enum Foo { A = 1, B = 2, C = 3 }", "B");
        assert_eq!(result, "enum Foo { A = 1, C = 3 }");
    }

    #[test]
    fn single_line_remove_member_with_string_value() {
        let result = remove_member_from_single_line("enum Foo { A = \"a\", B = \"b\" }", "A");
        assert_eq!(result, "enum Foo { B = \"b\" }");
    }
}
