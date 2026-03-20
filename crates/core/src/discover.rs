use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use fallow_config::{PackageJson, ResolvedConfig};
use ignore::WalkBuilder;

// Re-export types from fallow-types
pub use fallow_types::discover::{DiscoveredFile, EntryPoint, EntryPointSource, FileId};

const SOURCE_EXTENSIONS: &[&str] = &[
    "ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs", "vue", "svelte", "astro", "mdx", "css",
    "scss",
];

/// Hidden (dot-prefixed) directories that should be included in file discovery.
///
/// Most hidden directories (`.git`, `.cache`, etc.) should be skipped, but certain
/// convention directories contain source or config files that fallow needs to see:
/// - `.storybook` — Storybook configuration (the Storybook plugin depends on this)
/// - `.well-known` — Standard web convention directory
/// - `.changeset` — Changesets configuration
/// - `.github` — GitHub workflows and CI scripts
const ALLOWED_HIDDEN_DIRS: &[&str] = &[".storybook", ".well-known", ".changeset", ".github"];

/// Check if a hidden directory name is on the allowlist.
fn is_allowed_hidden_dir(name: &OsStr) -> bool {
    ALLOWED_HIDDEN_DIRS.iter().any(|&d| OsStr::new(d) == name)
}

/// Check if a hidden directory entry should be allowed through the filter.
///
/// Returns `true` if the entry is not hidden or is on the allowlist.
/// Hidden files (not directories) are always allowed through since the type
/// filter handles them.
fn is_allowed_hidden(entry: &ignore::DirEntry) -> bool {
    let name = entry.file_name();
    let name_str = name.to_string_lossy();

    // Not hidden — always allow
    if !name_str.starts_with('.') {
        return true;
    }

    // Hidden files are fine — the type filter (source extensions) will handle them
    if entry.file_type().is_some_and(|ft| ft.is_file()) {
        return true;
    }

    // Hidden directory — check against the allowlist
    is_allowed_hidden_dir(name)
}

/// Glob patterns for test/dev/story files excluded in production mode.
const PRODUCTION_EXCLUDE_PATTERNS: &[&str] = &[
    // Test files
    "**/*.test.*",
    "**/*.spec.*",
    "**/*.e2e.*",
    "**/*.e2e-spec.*",
    "**/*.bench.*",
    "**/*.fixture.*",
    // Story files
    "**/*.stories.*",
    "**/*.story.*",
    // Test directories
    "**/__tests__/**",
    "**/__mocks__/**",
    "**/__snapshots__/**",
    "**/__fixtures__/**",
    "**/test/**",
    "**/tests/**",
    // Dev/config files at project level
    "**/*.config.*",
    "**/.*.js",
    "**/.*.ts",
    "**/.*.mjs",
    "**/.*.cjs",
];

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

    let mut walk_builder = WalkBuilder::new(&config.root);
    walk_builder
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .types(types)
        .threads(config.threads)
        .filter_entry(is_allowed_hidden);
    let walker = walk_builder.build();

    // Build production exclude matcher if needed
    let production_excludes = if config.production {
        let mut builder = globset::GlobSetBuilder::new();
        for pattern in PRODUCTION_EXCLUDE_PATTERNS {
            if let Ok(glob) = globset::Glob::new(pattern) {
                builder.add(glob);
            }
        }
        builder.build().ok()
    } else {
        None
    };

    let mut files: Vec<DiscoveredFile> = walker
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_some_and(|ft| ft.is_file()))
        .filter(|entry| !config.ignore_patterns.is_match(entry.path()))
        .filter(|entry| {
            // In production mode, exclude test/story/dev files
            production_excludes.as_ref().is_none_or(|excludes| {
                let relative = entry
                    .path()
                    .strip_prefix(&config.root)
                    .unwrap_or(entry.path());
                !excludes.is_match(relative)
            })
        })
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

    // Sort by path for stable, deterministic FileId assignment.
    // The same set of files always produces the same IDs regardless of file
    // size changes, which is the foundation for incremental analysis and
    // cross-run graph caching.
    files.sort_unstable_by(|a, b| a.path.cmp(&b.path));

    // Re-assign IDs after sorting
    for (idx, file) in files.iter_mut().enumerate() {
        file.id = FileId(idx as u32);
    }

    files
}

/// Known output directory names from exports maps.
/// When an entry point path is inside one of these directories, we also try
/// the `src/` equivalent to find the tracked source file.
const OUTPUT_DIRS: &[&str] = &["dist", "build", "out", "esm", "cjs"];

/// Resolve a path relative to a base directory, with security check and extension fallback.
///
/// Returns `Some(EntryPoint)` if the path resolves to an existing file within `canonical_root`,
/// trying source extensions as fallback when the exact path doesn't exist.
/// Also handles exports map targets in output directories (e.g., `./dist/utils.js`)
/// by trying to map back to the source file (e.g., `./src/utils.ts`).
fn resolve_entry_path(
    base: &Path,
    entry: &str,
    canonical_root: &Path,
    source: EntryPointSource,
) -> Option<EntryPoint> {
    let resolved = base.join(entry);
    // Security: ensure resolved path stays within the allowed root
    let canonical_resolved = resolved.canonicalize().unwrap_or(resolved.clone());
    if !canonical_resolved.starts_with(canonical_root) {
        tracing::warn!(path = %entry, "Skipping entry point outside project root");
        return None;
    }

    // If the path is in an output directory (dist/, build/, etc.), try mapping to src/ first.
    // This handles exports map targets like `./dist/utils.js` → `./src/utils.ts`.
    // We check this BEFORE the exists() check because even if the dist file exists,
    // fallow ignores dist/ by default, so we need the source file instead.
    if let Some(source_path) = try_output_to_source_path(base, entry) {
        // Security: ensure the mapped source path stays within the project root
        if let Ok(canonical_source) = source_path.canonicalize()
            && canonical_source.starts_with(canonical_root)
        {
            return Some(EntryPoint {
                path: source_path,
                source,
            });
        }
    }

    if resolved.exists() {
        return Some(EntryPoint {
            path: resolved,
            source,
        });
    }
    // Try with source extensions
    for ext in SOURCE_EXTENSIONS {
        let with_ext = resolved.with_extension(ext);
        if with_ext.exists() {
            return Some(EntryPoint {
                path: with_ext,
                source,
            });
        }
    }
    None
}

/// Try to map an entry path from an output directory to its source equivalent.
///
/// Given `base=/project/packages/ui` and `entry=./dist/utils.js`, this tries:
/// - `/project/packages/ui/src/utils.ts`
/// - `/project/packages/ui/src/utils.tsx`
/// - etc. for all source extensions
///
/// Preserves any path prefix between the package root and the output dir,
/// e.g. `./modules/dist/utils.js` → `base/modules/src/utils.ts`.
///
/// Returns `Some(path)` if a source file is found.
fn try_output_to_source_path(base: &Path, entry: &str) -> Option<PathBuf> {
    let entry_path = Path::new(entry);
    let components: Vec<_> = entry_path.components().collect();

    // Find the last output directory component in the entry path
    let output_pos = components.iter().rposition(|c| {
        if let std::path::Component::Normal(s) = c
            && let Some(name) = s.to_str()
        {
            return OUTPUT_DIRS.contains(&name);
        }
        false
    })?;

    // Build the relative prefix before the output dir, filtering out CurDir (".")
    let prefix: PathBuf = components[..output_pos]
        .iter()
        .filter(|c| !matches!(c, std::path::Component::CurDir))
        .collect();

    // Build the relative path after the output dir (e.g., "utils.js")
    let suffix: PathBuf = components[output_pos + 1..].iter().collect();

    // Try base + prefix + "src" + suffix-with-source-extension
    for ext in SOURCE_EXTENSIONS {
        let source_candidate = base
            .join(&prefix)
            .join("src")
            .join(suffix.with_extension(ext));
        if source_candidate.exists() {
            return Some(source_candidate);
        }
    }

    None
}

/// Default index patterns used when no other entry points are found.
const DEFAULT_INDEX_PATTERNS: &[&str] = &[
    "src/index.{ts,tsx,js,jsx}",
    "src/main.{ts,tsx,js,jsx}",
    "index.{ts,tsx,js,jsx}",
    "main.{ts,tsx,js,jsx}",
];

/// Fall back to default index patterns if no entries were found.
///
/// When `ws_filter` is `Some`, only files whose path starts with the given
/// workspace root are considered (used for workspace-scoped discovery).
fn apply_default_fallback(
    files: &[DiscoveredFile],
    root: &Path,
    ws_filter: Option<&Path>,
) -> Vec<EntryPoint> {
    let default_matchers: Vec<globset::GlobMatcher> = DEFAULT_INDEX_PATTERNS
        .iter()
        .filter_map(|p| globset::Glob::new(p).ok().map(|g| g.compile_matcher()))
        .collect();

    let mut entries = Vec::new();
    for file in files {
        // Use strip_prefix instead of canonicalize for workspace filtering
        if let Some(ws_root) = ws_filter
            && file.path.strip_prefix(ws_root).is_err()
        {
            continue;
        }
        let relative = file.path.strip_prefix(root).unwrap_or(&file.path);
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
    entries
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

    // 1. Manual entries from config — batch all patterns into a single GlobSet
    // for O(files) matching instead of O(patterns × files).
    {
        let mut builder = globset::GlobSetBuilder::new();
        for pattern in &config.entry_patterns {
            if let Ok(glob) = globset::Glob::new(pattern) {
                builder.add(glob);
            }
        }
        if let Ok(glob_set) = builder.build()
            && !glob_set.is_empty()
        {
            for (idx, rel) in relative_paths.iter().enumerate() {
                if glob_set.is_match(rel) {
                    entries.push(EntryPoint {
                        path: files[idx].path.clone(),
                        source: EntryPointSource::ManualEntry,
                    });
                }
            }
        }
    }

    // 2. Package.json entries
    // Pre-compute canonical root once for all resolve_entry_path calls
    let canonical_root = config.root.canonicalize().unwrap_or(config.root.clone());
    let pkg_path = config.root.join("package.json");
    if let Ok(pkg) = PackageJson::load(&pkg_path) {
        for entry_path in pkg.entry_points() {
            if let Some(ep) = resolve_entry_path(
                &config.root,
                &entry_path,
                &canonical_root,
                EntryPointSource::PackageJsonMain,
            ) {
                entries.push(ep);
            }
        }

        // 2b. Package.json scripts — extract file references as entry points
        if let Some(scripts) = &pkg.scripts {
            for script_value in scripts.values() {
                for file_ref in extract_script_file_refs(script_value) {
                    if let Some(ep) = resolve_entry_path(
                        &config.root,
                        &file_ref,
                        &canonical_root,
                        EntryPointSource::PackageJsonScript,
                    ) {
                        entries.push(ep);
                    }
                }
            }
        }

        // Framework rules now flow through PluginRegistry via external_plugins.
    }

    // 4. Auto-discover nested package.json entry points
    // For monorepo-like structures without explicit workspace config, scan for
    // package.json files in subdirectories and use their main/exports as entries.
    discover_nested_package_entries(&config.root, files, &mut entries, &canonical_root);

    // 5. Default index files (if no other entries found)
    if entries.is_empty() {
        entries = apply_default_fallback(files, &config.root, None);
    }

    // Deduplicate by path
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries.dedup_by(|a, b| a.path == b.path);

    entries
}

/// Discover entry points from nested package.json files in subdirectories.
///
/// When a project has subdirectories with their own package.json (e.g., `packages/foo/package.json`),
/// the `main`, `module`, `exports`, and `bin` fields of those package.json files should be treated
/// as entry points. This handles monorepos without explicit workspace configuration.
fn discover_nested_package_entries(
    root: &Path,
    _files: &[DiscoveredFile],
    entries: &mut Vec<EntryPoint>,
    canonical_root: &Path,
) {
    // Walk common monorepo patterns to find nested package.json files
    let search_dirs = ["packages", "apps", "libs", "modules", "plugins"];
    for dir_name in &search_dirs {
        let search_dir = root.join(dir_name);
        if !search_dir.is_dir() {
            continue;
        }
        let Ok(read_dir) = std::fs::read_dir(&search_dir) else {
            continue;
        };
        for entry in read_dir.flatten() {
            let pkg_path = entry.path().join("package.json");
            if !pkg_path.exists() {
                continue;
            }
            let Ok(pkg) = PackageJson::load(&pkg_path) else {
                continue;
            };
            let pkg_dir = entry.path();
            for entry_path in pkg.entry_points() {
                if let Some(ep) = resolve_entry_path(
                    &pkg_dir,
                    &entry_path,
                    canonical_root,
                    EntryPointSource::PackageJsonExports,
                ) {
                    entries.push(ep);
                }
            }
            // Also check scripts in nested package.json
            if let Some(scripts) = &pkg.scripts {
                for script_value in scripts.values() {
                    for file_ref in extract_script_file_refs(script_value) {
                        if let Some(ep) = resolve_entry_path(
                            &pkg_dir,
                            &file_ref,
                            canonical_root,
                            EntryPointSource::PackageJsonScript,
                        ) {
                            entries.push(ep);
                        }
                    }
                }
            }
        }
    }
}

/// Discover entry points for a workspace package.
pub fn discover_workspace_entry_points(
    ws_root: &Path,
    _config: &ResolvedConfig,
    all_files: &[DiscoveredFile],
) -> Vec<EntryPoint> {
    let mut entries = Vec::new();

    let pkg_path = ws_root.join("package.json");
    if let Ok(pkg) = PackageJson::load(&pkg_path) {
        let canonical_ws_root = ws_root.canonicalize().unwrap_or(ws_root.to_path_buf());
        for entry_path in pkg.entry_points() {
            if let Some(ep) = resolve_entry_path(
                ws_root,
                &entry_path,
                &canonical_ws_root,
                EntryPointSource::PackageJsonMain,
            ) {
                entries.push(ep);
            }
        }

        // Scripts field — extract file references as entry points
        if let Some(scripts) = &pkg.scripts {
            for script_value in scripts.values() {
                for file_ref in extract_script_file_refs(script_value) {
                    if let Some(ep) = resolve_entry_path(
                        ws_root,
                        &file_ref,
                        &canonical_ws_root,
                        EntryPointSource::PackageJsonScript,
                    ) {
                        entries.push(ep);
                    }
                }
            }
        }

        // Framework rules now flow through PluginRegistry via external_plugins.
    }

    // Fall back to default index files if no entry points found for this workspace
    if entries.is_empty() {
        entries = apply_default_fallback(all_files, ws_root, None);
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
            // Collect ALL file path arguments after the runner (handles
            // `node --test file1.mjs file2.mjs ...` and similar multi-file patterns)
            for &token in &tokens[start + 1..] {
                if token.starts_with('-') {
                    continue;
                }
                // Must look like a file path (contains '/' or '.' extension)
                if looks_like_file_path(token) {
                    refs.push(token.to_string());
                }
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
    if extensions.iter().any(|ext| token.ends_with(ext)) {
        return true;
    }
    // Only treat tokens with `/` as paths if they look like actual file paths,
    // not URLs or scoped package names like @scope/package
    token.starts_with("./")
        || token.starts_with("../")
        || (token.contains('/') && !token.starts_with('@') && !token.contains("://"))
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

/// Discover entry points from plugin results (dynamic config parsing).
///
/// Converts plugin-discovered patterns and setup files into concrete entry points
/// by matching them against the discovered file list.
pub fn discover_plugin_entry_points(
    plugin_result: &crate::plugins::AggregatedPluginResult,
    config: &ResolvedConfig,
    files: &[DiscoveredFile],
) -> Vec<EntryPoint> {
    let mut entries = Vec::new();

    // Pre-compute relative paths
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

    // Match plugin entry patterns against files using a single GlobSet
    // for O(files) matching instead of O(patterns × files).
    let mut builder = globset::GlobSetBuilder::new();
    for pattern in plugin_result
        .entry_patterns
        .iter()
        .chain(plugin_result.discovered_always_used.iter())
        .chain(plugin_result.always_used.iter())
    {
        if let Ok(glob) = globset::Glob::new(pattern) {
            builder.add(glob);
        }
    }
    if let Ok(glob_set) = builder.build()
        && !glob_set.is_empty()
    {
        for (idx, rel) in relative_paths.iter().enumerate() {
            if glob_set.is_match(rel) {
                entries.push(EntryPoint {
                    path: files[idx].path.clone(),
                    source: EntryPointSource::Plugin {
                        name: "plugin".to_string(),
                    },
                });
            }
        }
    }

    // Add setup files (absolute paths from plugin config parsing)
    for setup_file in &plugin_result.setup_files {
        let resolved = if setup_file.is_absolute() {
            setup_file.clone()
        } else {
            config.root.join(setup_file)
        };
        if resolved.exists() {
            entries.push(EntryPoint {
                path: resolved,
                source: EntryPointSource::Plugin {
                    name: "plugin-setup".to_string(),
                },
            });
        } else {
            // Try with extensions
            for ext in SOURCE_EXTENSIONS {
                let with_ext = resolved.with_extension(ext);
                if with_ext.exists() {
                    entries.push(EntryPoint {
                        path: with_ext,
                        source: EntryPointSource::Plugin {
                            name: "plugin-setup".to_string(),
                        },
                    });
                    break;
                }
            }
        }
    }

    // Deduplicate
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries.dedup_by(|a, b| a.path == b.path);
    entries
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

    // is_allowed_hidden_dir tests
    #[test]
    fn allowed_hidden_dirs() {
        assert!(is_allowed_hidden_dir(OsStr::new(".storybook")));
        assert!(is_allowed_hidden_dir(OsStr::new(".well-known")));
        assert!(is_allowed_hidden_dir(OsStr::new(".changeset")));
        assert!(is_allowed_hidden_dir(OsStr::new(".github")));
    }

    #[test]
    fn disallowed_hidden_dirs() {
        assert!(!is_allowed_hidden_dir(OsStr::new(".git")));
        assert!(!is_allowed_hidden_dir(OsStr::new(".cache")));
        assert!(!is_allowed_hidden_dir(OsStr::new(".vscode")));
        assert!(!is_allowed_hidden_dir(OsStr::new(".fallow")));
        assert!(!is_allowed_hidden_dir(OsStr::new(".next")));
    }

    #[test]
    fn non_hidden_dirs_not_in_allowlist() {
        // Non-hidden names should not match the allowlist (they are always allowed
        // by is_allowed_hidden because they don't start with '.')
        assert!(!is_allowed_hidden_dir(OsStr::new("src")));
        assert!(!is_allowed_hidden_dir(OsStr::new("node_modules")));
    }

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// Valid glob patterns should never panic when compiled via globset.
            #[test]
            fn glob_patterns_never_panic_on_compile(
                prefix in "[a-zA-Z0-9_]{1,20}",
                ext in prop::sample::select(vec!["ts", "tsx", "js", "jsx", "vue", "svelte", "astro", "mdx"]),
            ) {
                let pattern = format!("**/{prefix}*.{ext}");
                // Should not panic — either compiles or returns Err gracefully
                let result = globset::Glob::new(&pattern);
                prop_assert!(result.is_ok(), "Glob::new should not fail for well-formed patterns");
            }

            /// Files with known source extensions should be in the SOURCE_EXTENSIONS list.
            #[test]
            fn known_extensions_in_source_list(
                ext in prop::sample::select(vec!["ts", "tsx", "js", "jsx", "mts", "cts", "mjs", "cjs", "vue", "svelte", "astro", "mdx", "css", "scss"]),
            ) {
                prop_assert!(
                    SOURCE_EXTENSIONS.contains(&ext),
                    "Extension '{ext}' should be in SOURCE_EXTENSIONS"
                );
            }

            /// compile_glob_set should never panic on arbitrary well-formed glob patterns.
            #[test]
            fn compile_glob_set_no_panic(
                patterns in prop::collection::vec("[a-zA-Z0-9_*/.]{1,30}", 0..10),
            ) {
                // Should not panic regardless of input
                let _ = compile_glob_set(&patterns);
            }

            /// looks_like_file_path should never panic on arbitrary strings.
            #[test]
            fn looks_like_file_path_no_panic(s in "[a-zA-Z0-9_./@-]{1,80}") {
                let _ = looks_like_file_path(&s);
            }

            /// looks_like_script_file should never panic on arbitrary strings.
            #[test]
            fn looks_like_script_file_no_panic(s in "[a-zA-Z0-9_./@-]{1,80}") {
                let _ = looks_like_script_file(&s);
            }

            /// extract_script_file_refs should never panic on arbitrary input.
            #[test]
            fn extract_script_file_refs_no_panic(s in "[a-zA-Z0-9 _./@&|;-]{1,200}") {
                let _ = extract_script_file_refs(&s);
            }
        }
    }
}
