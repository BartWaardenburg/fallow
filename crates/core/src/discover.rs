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

const SOURCE_EXTENSIONS: &[&str] = &[
    "ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs", "vue", "svelte", "astro", "mdx", "css",
    "scss",
];

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

    let walker = WalkBuilder::new(&config.root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .types(types)
        .threads(config.threads)
        .build();

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
            if let Some(ref excludes) = production_excludes {
                let relative = entry
                    .path()
                    .strip_prefix(&config.root)
                    .unwrap_or(entry.path());
                !excludes.is_match(relative)
            } else {
                true
            }
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
        return Some(EntryPoint {
            path: source_path,
            source,
        });
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

/// Pre-compile entry point and always_used glob matchers from a framework rule.
fn compile_rule_matchers(
    rule: &fallow_config::FrameworkRule,
) -> (Vec<globset::GlobMatcher>, Vec<globset::GlobMatcher>) {
    let entry_matchers: Vec<globset::GlobMatcher> = rule
        .entry_points
        .iter()
        .filter_map(|ep| {
            globset::Glob::new(&ep.pattern)
                .ok()
                .map(|g| g.compile_matcher())
        })
        .collect();

    let always_matchers: Vec<globset::GlobMatcher> = rule
        .always_used
        .iter()
        .filter_map(|p| globset::Glob::new(p).ok().map(|g| g.compile_matcher()))
        .collect();

    (entry_matchers, always_matchers)
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

        // 3. Framework rules — cache active status + pre-compile pattern matchers
        let active_rules: Vec<&fallow_config::FrameworkRule> = config
            .framework_rules
            .iter()
            .filter(|rule| is_framework_active(rule, &pkg, &config.root))
            .collect();

        for rule in &active_rules {
            let (entry_matchers, always_matchers) = compile_rule_matchers(rule);

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

    // 4. Auto-discover nested package.json entry points
    // For monorepo-like structures without explicit workspace config, scan for
    // package.json files in subdirectories and use their main/exports as entries.
    discover_nested_package_entries(&config.root, files, &mut entries);

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
) {
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    // Walk common monorepo patterns to find nested package.json files
    let search_dirs = ["packages", "apps", "libs", "modules", "plugins"];
    for dir_name in &search_dirs {
        let search_dir = root.join(dir_name);
        if !search_dir.is_dir() {
            continue;
        }
        let read_dir = match std::fs::read_dir(&search_dir) {
            Ok(rd) => rd,
            Err(_) => continue,
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
                    &canonical_root,
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
                            &canonical_root,
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

        // Apply framework rules to workspace.
        // Check activation against BOTH workspace and root package deps (monorepo hoisting).
        // Use path prefix matching instead of per-file canonicalize (avoids O(files×workspaces) syscalls)
        for rule in &config.framework_rules {
            let ws_active = is_framework_active(rule, &pkg, ws_root);
            let root_active = root_pkg
                .as_ref()
                .map(|rpkg| is_framework_active(rule, rpkg, &config.root))
                .unwrap_or(false);

            if !ws_active && !root_active {
                continue;
            }

            let (entry_matchers, always_matchers) = compile_rule_matchers(rule);

            // Only consider files within this workspace — use strip_prefix instead of canonicalize
            for file in all_files {
                let relative = match file.path.strip_prefix(ws_root) {
                    Ok(rel) => rel,
                    Err(_) => continue,
                };
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

    // Match plugin entry patterns against files
    let all_patterns: Vec<&str> = plugin_result
        .entry_patterns
        .iter()
        .chain(plugin_result.discovered_always_used.iter())
        .chain(plugin_result.always_used.iter())
        .map(|s| s.as_str())
        .collect();

    let matchers: Vec<globset::GlobMatcher> = all_patterns
        .iter()
        .filter_map(|p| globset::Glob::new(p).ok().map(|g| g.compile_matcher()))
        .collect();

    for (idx, rel) in relative_paths.iter().enumerate() {
        if matchers.iter().any(|m| m.is_match(rel)) {
            entries.push(EntryPoint {
                path: files[idx].path.clone(),
                source: EntryPointSource::FrameworkRule {
                    name: "plugin".to_string(),
                },
            });
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
                source: EntryPointSource::FrameworkRule {
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
                        source: EntryPointSource::FrameworkRule {
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
}
