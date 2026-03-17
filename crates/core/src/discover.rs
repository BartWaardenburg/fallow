use std::path::{Path, PathBuf};

use fallow_config::{FrameworkDetection, PackageJson, ResolvedConfig};
use ignore::WalkBuilder;

/// A discovered source file on disk.
#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    /// Unique file index.
    pub id: FileId,
    /// Absolute path.
    pub path: PathBuf,
    /// File size in bytes (for sorting largest-first).
    pub size_bytes: u64,
}

/// Compact file identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub u32);

/// An entry point into the module graph.
#[derive(Debug, Clone)]
pub struct EntryPoint {
    pub path: PathBuf,
    pub source: EntryPointSource,
}

/// Where an entry point was discovered from.
#[derive(Debug, Clone)]
pub enum EntryPointSource {
    PackageJsonMain,
    PackageJsonModule,
    PackageJsonExports,
    PackageJsonBin,
    PackageJsonScript,
    FrameworkRule { name: String },
    TestFile,
    DefaultIndex,
    ManualEntry,
}

const SOURCE_EXTENSIONS: &[&str] = &["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"];

/// Discover all source files in the project.
pub fn discover_files(config: &ResolvedConfig) -> Vec<DiscoveredFile> {
    let _span = tracing::info_span!("discover_files").entered();

    let mut types_builder = ignore::types::TypesBuilder::new();
    for ext in SOURCE_EXTENSIONS {
        types_builder
            .add("source", &format!("*.{ext}"))
            .expect("valid glob");
    }
    types_builder.select("source");
    let types = types_builder.build().expect("valid types");

    let walker = WalkBuilder::new(&config.root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .types(types)
        .threads(config.threads)
        .build();

    let mut files: Vec<DiscoveredFile> = walker
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_some_and(|ft| ft.is_file()))
        .filter(|entry| !config.ignore_patterns.is_match(entry.path()))
        .enumerate()
        .map(|(idx, entry)| {
            let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
            DiscoveredFile {
                id: FileId(idx as u32),
                path: entry.into_path(),
                size_bytes,
            }
        })
        .collect();

    // Sort largest files first for better rayon work-stealing, with path as tiebreaker for determinism
    files.sort_unstable_by(|a, b| {
        b.size_bytes
            .cmp(&a.size_bytes)
            .then_with(|| a.path.cmp(&b.path))
    });

    // Re-assign IDs after sorting
    for (idx, file) in files.iter_mut().enumerate() {
        file.id = FileId(idx as u32);
    }

    files
}

/// Discover entry points from package.json, framework rules, and defaults.
pub fn discover_entry_points(config: &ResolvedConfig, files: &[DiscoveredFile]) -> Vec<EntryPoint> {
    let _span = tracing::info_span!("discover_entry_points").entered();
    let mut entries = Vec::new();

    // Pre-compute relative paths for all files (once, not per pattern)
    let relative_paths: Vec<String> = files
        .iter()
        .map(|f| {
            f.path
                .strip_prefix(&config.root)
                .unwrap_or(&f.path)
                .to_string_lossy()
                .into_owned()
        })
        .collect();

    // 1. Manual entries from config — pre-compile all patterns
    for pattern in &config.entry_patterns {
        if let Ok(glob) = globset::Glob::new(pattern) {
            let matcher = glob.compile_matcher();
            for (idx, rel) in relative_paths.iter().enumerate() {
                if matcher.is_match(rel) {
                    entries.push(EntryPoint {
                        path: files[idx].path.clone(),
                        source: EntryPointSource::ManualEntry,
                    });
                }
            }
        }
    }

    // 2. Package.json entries
    let pkg_path = config.root.join("package.json");
    if let Ok(pkg) = PackageJson::load(&pkg_path) {
        let canonical_root = config.root.canonicalize().unwrap_or(config.root.clone());
        for entry_path in pkg.entry_points() {
            let resolved = config.root.join(&entry_path);
            // Security: ensure resolved path stays within project root
            let canonical_resolved = resolved.canonicalize().unwrap_or(resolved.clone());
            if !canonical_resolved.starts_with(&canonical_root) {
                tracing::warn!(path = %entry_path, "Skipping entry point outside project root");
                continue;
            }
            if resolved.exists() {
                entries.push(EntryPoint {
                    path: resolved,
                    source: EntryPointSource::PackageJsonMain,
                });
            } else {
                // Try with extensions
                for ext in SOURCE_EXTENSIONS {
                    let with_ext = resolved.with_extension(ext);
                    if with_ext.exists() {
                        entries.push(EntryPoint {
                            path: with_ext,
                            source: EntryPointSource::PackageJsonMain,
                        });
                        break;
                    }
                }
            }
        }

        // 2b. Package.json scripts — extract file references as entry points
        if let Some(scripts) = &pkg.scripts {
            for script_value in scripts.values() {
                for file_ref in extract_script_file_refs(script_value) {
                    let resolved = config.root.join(&file_ref);
                    // Security: ensure resolved path stays within project root
                    let canonical_resolved = resolved.canonicalize().unwrap_or(resolved.clone());
                    if !canonical_resolved.starts_with(&canonical_root) {
                        tracing::warn!(path = %file_ref, "Skipping script entry point outside project root");
                        continue;
                    }
                    if resolved.exists() {
                        entries.push(EntryPoint {
                            path: resolved,
                            source: EntryPointSource::PackageJsonScript,
                        });
                    } else {
                        // Try with extensions
                        for ext in SOURCE_EXTENSIONS {
                            let with_ext = resolved.with_extension(ext);
                            if with_ext.exists() {
                                entries.push(EntryPoint {
                                    path: with_ext,
                                    source: EntryPointSource::PackageJsonScript,
                                });
                                break;
                            }
                        }
                    }
                }
            }
        }

        // 3. Framework rules — cache active status + pre-compile pattern matchers
        let active_rules: Vec<&fallow_config::FrameworkRule> = config
            .framework_rules
            .iter()
            .filter(|rule| is_framework_active(rule, &pkg, &config.root))
            .collect();

        for rule in &active_rules {
            // Pre-compile entry point matchers
            let entry_matchers: Vec<globset::GlobMatcher> = rule
                .entry_points
                .iter()
                .filter_map(|ep| {
                    globset::Glob::new(&ep.pattern)
                        .ok()
                        .map(|g| g.compile_matcher())
                })
                .collect();

            // Pre-compile always_used matchers
            let always_matchers: Vec<globset::GlobMatcher> = rule
                .always_used
                .iter()
                .filter_map(|p| globset::Glob::new(p).ok().map(|g| g.compile_matcher()))
                .collect();

            // Single pass over files for all matchers of this rule
            for (idx, rel) in relative_paths.iter().enumerate() {
                let matched = entry_matchers.iter().any(|m| m.is_match(rel))
                    || always_matchers.iter().any(|m| m.is_match(rel));
                if matched {
                    entries.push(EntryPoint {
                        path: files[idx].path.clone(),
                        source: EntryPointSource::FrameworkRule {
                            name: rule.name.clone(),
                        },
                    });
                }
            }
        }
    }

    // 4. Default index files (if no other entries found)
    if entries.is_empty() {
        let default_patterns = [
            "src/index.{ts,tsx,js,jsx}",
            "src/main.{ts,tsx,js,jsx}",
            "index.{ts,tsx,js,jsx}",
            "main.{ts,tsx,js,jsx}",
        ];
        // Pre-compile default matchers
        let default_matchers: Vec<globset::GlobMatcher> = default_patterns
            .iter()
            .filter_map(|p| globset::Glob::new(p).ok().map(|g| g.compile_matcher()))
            .collect();

        for (idx, rel) in relative_paths.iter().enumerate() {
            if default_matchers.iter().any(|m| m.is_match(rel)) {
                entries.push(EntryPoint {
                    path: files[idx].path.clone(),
                    source: EntryPointSource::DefaultIndex,
                });
            }
        }
    }

    // Deduplicate by path
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries.dedup_by(|a, b| a.path == b.path);

    entries
}

/// Check if a framework rule is active based on its detection config.
fn is_framework_active(
    rule: &fallow_config::FrameworkRule,
    pkg: &PackageJson,
    root: &Path,
) -> bool {
    match &rule.detection {
        None => true, // No detection = always active
        Some(detection) => check_detection(detection, pkg, root),
    }
}

fn check_detection(detection: &FrameworkDetection, pkg: &PackageJson, root: &Path) -> bool {
    match detection {
        FrameworkDetection::Dependency { package } => {
            pkg.all_dependency_names().iter().any(|d| d == package)
        }
        FrameworkDetection::FileExists { pattern } => file_exists_glob(pattern, root),
        FrameworkDetection::All { conditions } => {
            conditions.iter().all(|c| check_detection(c, pkg, root))
        }
        FrameworkDetection::Any { conditions } => {
            conditions.iter().any(|c| check_detection(c, pkg, root))
        }
    }
}

/// Discover entry points for a workspace package.
pub fn discover_workspace_entry_points(
    ws_root: &Path,
    config: &ResolvedConfig,
    all_files: &[DiscoveredFile],
) -> Vec<EntryPoint> {
    let mut entries = Vec::new();

    // Also load root package.json for framework detection (monorepo deps are often at root)
    let root_pkg = PackageJson::load(&config.root.join("package.json")).ok();

    let pkg_path = ws_root.join("package.json");
    if let Ok(pkg) = PackageJson::load(&pkg_path) {
        let canonical_ws_root = ws_root.canonicalize().unwrap_or(ws_root.to_path_buf());
        for entry_path in pkg.entry_points() {
            let resolved = ws_root.join(&entry_path);
            // Security: ensure resolved path stays within workspace root
            let canonical_resolved = resolved.canonicalize().unwrap_or(resolved.clone());
            if !canonical_resolved.starts_with(&canonical_ws_root) {
                tracing::warn!(path = %entry_path, "Skipping entry point outside workspace root");
                continue;
            }
            if resolved.exists() {
                entries.push(EntryPoint {
                    path: resolved,
                    source: EntryPointSource::PackageJsonMain,
                });
            } else {
                for ext in SOURCE_EXTENSIONS {
                    let with_ext = resolved.with_extension(ext);
                    if with_ext.exists() {
                        entries.push(EntryPoint {
                            path: with_ext,
                            source: EntryPointSource::PackageJsonMain,
                        });
                        break;
                    }
                }
            }
        }

        // Scripts field — extract file references as entry points
        if let Some(scripts) = &pkg.scripts {
            for script_value in scripts.values() {
                for file_ref in extract_script_file_refs(script_value) {
                    let resolved = ws_root.join(&file_ref);
                    // Security: ensure resolved path stays within workspace root
                    let canonical_resolved = resolved.canonicalize().unwrap_or(resolved.clone());
                    if !canonical_resolved.starts_with(&canonical_ws_root) {
                        tracing::warn!(path = %file_ref, "Skipping script entry point outside workspace root");
                        continue;
                    }
                    if resolved.exists() {
                        entries.push(EntryPoint {
                            path: resolved,
                            source: EntryPointSource::PackageJsonScript,
                        });
                    } else {
                        for ext in SOURCE_EXTENSIONS {
                            let with_ext = resolved.with_extension(ext);
                            if with_ext.exists() {
                                entries.push(EntryPoint {
                                    path: with_ext,
                                    source: EntryPointSource::PackageJsonScript,
                                });
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Apply framework rules to workspace.
        // Check activation against BOTH workspace and root package deps (monorepo hoisting).
        for rule in &config.framework_rules {
            let ws_active = is_framework_active(rule, &pkg, ws_root);
            let root_active = root_pkg
                .as_ref()
                .map(|rpkg| is_framework_active(rule, rpkg, &config.root))
                .unwrap_or(false);

            if !ws_active && !root_active {
                continue;
            }

            // Pre-compile entry point matchers to avoid recompiling per file
            let entry_matchers: Vec<globset::GlobMatcher> = rule
                .entry_points
                .iter()
                .filter_map(|ep| {
                    globset::Glob::new(&ep.pattern)
                        .ok()
                        .map(|g| g.compile_matcher())
                })
                .collect();

            // Pre-compile always_used matchers
            let always_matchers: Vec<globset::GlobMatcher> = rule
                .always_used
                .iter()
                .filter_map(|p| globset::Glob::new(p).ok().map(|g| g.compile_matcher()))
                .collect();

            // Only consider files within this workspace for framework rule matching
            let canonical_ws = ws_root.canonicalize().unwrap_or(ws_root.to_path_buf());
            for file in all_files {
                let canonical_file = file.path.canonicalize().unwrap_or(file.path.clone());
                if !canonical_file.starts_with(&canonical_ws) {
                    continue;
                }
                let relative = file.path.strip_prefix(ws_root).unwrap_or(&file.path);
                let relative_str = relative.to_string_lossy();
                let matched = entry_matchers
                    .iter()
                    .any(|m| m.is_match(relative_str.as_ref()))
                    || always_matchers
                        .iter()
                        .any(|m| m.is_match(relative_str.as_ref()));
                if matched {
                    entries.push(EntryPoint {
                        path: file.path.clone(),
                        source: EntryPointSource::FrameworkRule {
                            name: rule.name.clone(),
                        },
                    });
                }
            }
        }
    }

    // Fall back to default index files if no entry points found for this workspace
    if entries.is_empty() {
        let default_patterns = [
            "src/index.{ts,tsx,js,jsx}",
            "src/main.{ts,tsx,js,jsx}",
            "index.{ts,tsx,js,jsx}",
            "main.{ts,tsx,js,jsx}",
        ];
        let default_matchers: Vec<globset::GlobMatcher> = default_patterns
            .iter()
            .filter_map(|p| globset::Glob::new(p).ok().map(|g| g.compile_matcher()))
            .collect();

        let canonical_ws = ws_root.canonicalize().unwrap_or(ws_root.to_path_buf());
        for file in all_files {
            let canonical_file = file.path.canonicalize().unwrap_or(file.path.clone());
            if !canonical_file.starts_with(&canonical_ws) {
                continue;
            }
            let relative = file.path.strip_prefix(ws_root).unwrap_or(&file.path);
            let relative_str = relative.to_string_lossy();
            if default_matchers
                .iter()
                .any(|m| m.is_match(relative_str.as_ref()))
            {
                entries.push(EntryPoint {
                    path: file.path.clone(),
                    source: EntryPointSource::DefaultIndex,
                });
            }
        }
    }

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries.dedup_by(|a, b| a.path == b.path);
    entries
}

/// Extract file path references from a package.json script value.
///
/// Recognises patterns like:
/// - `node path/to/script.js`
/// - `ts-node path/to/script.ts`
/// - `tsx path/to/script.ts`
/// - `npx ts-node path/to/script.ts`
/// - Bare file paths ending in `.js`, `.ts`, `.mjs`, `.cjs`, `.mts`, `.cts`
///
/// Script values are split by `&&`, `||`, and `;` to handle chained commands.
fn extract_script_file_refs(script: &str) -> Vec<String> {
    let mut refs = Vec::new();

    // Runners whose next argument is a file path
    const RUNNERS: &[&str] = &["node", "ts-node", "tsx", "babel-node"];

    // Split on shell operators to handle chained commands
    for segment in script.split(&['&', '|', ';'][..]) {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }

        let tokens: Vec<&str> = segment.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        // Skip leading `npx`/`pnpx`/`yarn`/`pnpm exec` to find the actual command
        let mut start = 0;
        if matches!(tokens.first(), Some(&"npx" | &"pnpx")) {
            start = 1;
        } else if tokens.len() >= 2 && matches!(tokens[0], "yarn" | "pnpm") && tokens[1] == "exec" {
            start = 2;
        }

        if start >= tokens.len() {
            continue;
        }

        let cmd = tokens[start];

        // Check if the command is a known runner
        if RUNNERS.contains(&cmd) {
            // Find the first non-flag argument after the runner
            for &token in &tokens[start + 1..] {
                if token.starts_with('-') {
                    continue;
                }
                // Must look like a file path (contains '/' or '.' extension)
                if looks_like_file_path(token) {
                    refs.push(token.to_string());
                }
                break;
            }
        } else {
            // Scan all tokens for bare file paths (e.g. `./scripts/build.js`)
            for &token in &tokens[start..] {
                if token.starts_with('-') {
                    continue;
                }
                if looks_like_script_file(token) {
                    refs.push(token.to_string());
                }
            }
        }
    }

    refs
}

/// Check if a token looks like a file path argument (has a directory separator or a
/// JS/TS file extension).
fn looks_like_file_path(token: &str) -> bool {
    let extensions = [".js", ".ts", ".mjs", ".cjs", ".mts", ".cts", ".jsx", ".tsx"];
    extensions.iter().any(|ext| token.ends_with(ext)) || token.contains('/')
}

/// Check if a token looks like a standalone script file reference (must have a
/// JS/TS extension and a path-like structure, not a bare command name).
fn looks_like_script_file(token: &str) -> bool {
    let extensions = [".js", ".ts", ".mjs", ".cjs", ".mts", ".cts", ".jsx", ".tsx"];
    if !extensions.iter().any(|ext| token.ends_with(ext)) {
        return false;
    }
    // Must contain a path separator or start with ./ to distinguish from
    // bare package names like `webpack.js`
    token.contains('/') || token.starts_with("./") || token.starts_with("../")
}

/// Check whether any file matching a glob pattern exists under root.
///
/// Uses `globset::Glob` for pattern compilation (supports brace expansion like
/// `{ts,js}`) and walks the static prefix directory to find matches.
fn file_exists_glob(pattern: &str, root: &Path) -> bool {
    let matcher = match globset::Glob::new(pattern) {
        Ok(g) => g.compile_matcher(),
        Err(_) => return false,
    };

    // Extract the static directory prefix from the pattern to narrow the walk.
    // E.g. for ".storybook/main.{ts,js}" the prefix is ".storybook".
    let prefix: PathBuf = Path::new(pattern)
        .components()
        .take_while(|c| {
            let s = c.as_os_str().to_string_lossy();
            !s.contains('*') && !s.contains('?') && !s.contains('{') && !s.contains('[')
        })
        .collect();

    let search_dir = if prefix.as_os_str().is_empty() {
        root.to_path_buf()
    } else {
        // prefix may be an exact directory or include the filename portion.
        let joined = root.join(&prefix);
        if joined.is_dir() {
            joined
        } else if let Some(parent) = joined.parent() {
            // Only use parent if it's NOT the root itself (avoid walking entire project)
            if parent != root && parent.is_dir() {
                parent.to_path_buf()
            } else {
                // The prefix directory doesn't exist — no match possible
                return false;
            }
        } else {
            return false;
        }
    };

    if !search_dir.is_dir() {
        return false;
    }

    walk_dir_recursive(&search_dir, root, &matcher)
}

/// Maximum recursion depth for directory walking to prevent infinite loops on symlink cycles.
const MAX_WALK_DEPTH: usize = 20;

/// Recursively walk a directory and check if any file matches the glob.
fn walk_dir_recursive(dir: &Path, root: &Path, matcher: &globset::GlobMatcher) -> bool {
    walk_dir_recursive_depth(dir, root, matcher, 0)
}

/// Inner recursive walker with depth tracking.
fn walk_dir_recursive_depth(
    dir: &Path,
    root: &Path,
    matcher: &globset::GlobMatcher,
    depth: usize,
) -> bool {
    if depth >= MAX_WALK_DEPTH {
        tracing::warn!(
            dir = %dir.display(),
            "Maximum directory walk depth reached, possible symlink cycle"
        );
        return false;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return false,
    };

    for entry in entries.flatten() {
        // Use symlink_metadata to avoid following symlinks (prevents cycles)
        let is_real_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        if is_real_dir {
            if walk_dir_recursive_depth(&entry.path(), root, matcher, depth + 1) {
                return true;
            }
        } else {
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap_or(&path);
            if matcher.is_match(relative) {
                return true;
            }
        }
    }

    false
}

/// Pre-compile a set of glob patterns for efficient matching against many paths.
pub fn compile_glob_set(patterns: &[String]) -> Option<globset::GlobSet> {
    if patterns.is_empty() {
        return None;
    }
    let mut builder = globset::GlobSetBuilder::new();
    for pattern in patterns {
        if let Ok(glob) = globset::Glob::new(pattern) {
            builder.add(glob);
        }
    }
    builder.build().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // extract_script_file_refs tests (Issue 3)
    #[test]
    fn script_node_runner() {
        let refs = extract_script_file_refs("node utilities/generate-coverage-badge.js");
        assert_eq!(refs, vec!["utilities/generate-coverage-badge.js"]);
    }

    #[test]
    fn script_ts_node_runner() {
        let refs = extract_script_file_refs("ts-node scripts/seed.ts");
        assert_eq!(refs, vec!["scripts/seed.ts"]);
    }

    #[test]
    fn script_tsx_runner() {
        let refs = extract_script_file_refs("tsx scripts/migrate.ts");
        assert_eq!(refs, vec!["scripts/migrate.ts"]);
    }

    #[test]
    fn script_npx_prefix() {
        let refs = extract_script_file_refs("npx ts-node scripts/generate.ts");
        assert_eq!(refs, vec!["scripts/generate.ts"]);
    }

    #[test]
    fn script_chained_commands() {
        let refs = extract_script_file_refs("node scripts/build.js && node scripts/post-build.js");
        assert_eq!(refs, vec!["scripts/build.js", "scripts/post-build.js"]);
    }

    #[test]
    fn script_with_flags() {
        let refs = extract_script_file_refs(
            "node --experimental-specifier-resolution=node scripts/run.mjs",
        );
        assert_eq!(refs, vec!["scripts/run.mjs"]);
    }

    #[test]
    fn script_no_file_ref() {
        let refs = extract_script_file_refs("next build");
        assert!(refs.is_empty());
    }

    #[test]
    fn script_bare_file_path() {
        let refs = extract_script_file_refs("echo done && node ./scripts/check.js");
        assert_eq!(refs, vec!["./scripts/check.js"]);
    }

    #[test]
    fn script_semicolon_separator() {
        let refs = extract_script_file_refs("node scripts/a.js; node scripts/b.ts");
        assert_eq!(refs, vec!["scripts/a.js", "scripts/b.ts"]);
    }

    // looks_like_file_path tests
    #[test]
    fn file_path_with_extension() {
        assert!(looks_like_file_path("scripts/build.js"));
        assert!(looks_like_file_path("scripts/build.ts"));
        assert!(looks_like_file_path("scripts/build.mjs"));
    }

    #[test]
    fn file_path_with_slash() {
        assert!(looks_like_file_path("scripts/build"));
    }

    #[test]
    fn not_file_path() {
        assert!(!looks_like_file_path("--watch"));
        assert!(!looks_like_file_path("build"));
    }

    // looks_like_script_file tests
    #[test]
    fn script_file_with_path() {
        assert!(looks_like_script_file("scripts/build.js"));
        assert!(looks_like_script_file("./scripts/build.ts"));
        assert!(looks_like_script_file("../scripts/build.mjs"));
    }

    #[test]
    fn not_script_file_bare_name() {
        // Bare names without path separator should not match
        assert!(!looks_like_script_file("webpack.js"));
        assert!(!looks_like_script_file("build"));
    }
}
