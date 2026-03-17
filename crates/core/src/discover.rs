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

    // Sort largest files first for better rayon work-stealing
    files.sort_unstable_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

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
        for entry_path in pkg.entry_points() {
            let resolved = config.root.join(&entry_path);
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

/// Discover files within a workspace directory, continuing FileId numbering.
pub fn discover_workspace_files(
    ws_root: &Path,
    config: &ResolvedConfig,
    start_id: usize,
) -> Vec<DiscoveredFile> {
    let mut types_builder = ignore::types::TypesBuilder::new();
    for ext in SOURCE_EXTENSIONS {
        types_builder
            .add("source", &format!("*.{ext}"))
            .expect("valid glob");
    }
    types_builder.select("source");
    let types = types_builder.build().expect("valid types");

    let walker = WalkBuilder::new(ws_root)
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
                id: FileId((start_id + idx) as u32),
                path: entry.into_path(),
                size_bytes,
            }
        })
        .collect();

    files.sort_unstable_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    for (i, file) in files.iter_mut().enumerate() {
        file.id = FileId((start_id + i) as u32);
    }

    files
}

/// Discover entry points for a workspace package.
pub fn discover_workspace_entry_points(
    ws_root: &Path,
    config: &ResolvedConfig,
    all_files: &[DiscoveredFile],
) -> Vec<EntryPoint> {
    let mut entries = Vec::new();

    let pkg_path = ws_root.join("package.json");
    if let Ok(pkg) = PackageJson::load(&pkg_path) {
        for entry_path in pkg.entry_points() {
            let resolved = ws_root.join(&entry_path);
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

        // Apply framework rules to workspace
        for rule in &config.framework_rules {
            if !is_framework_active(rule, &pkg, ws_root) {
                continue;
            }

            for entry_pat in &rule.entry_points {
                for file in all_files {
                    if glob_matches(&entry_pat.pattern, &file.path, ws_root) {
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
    }

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries.dedup_by(|a, b| a.path == b.path);
    entries
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

/// Recursively walk a directory and check if any file matches the glob.
fn walk_dir_recursive(dir: &Path, root: &Path, matcher: &globset::GlobMatcher) -> bool {
    let entries = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return false,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if walk_dir_recursive(&path, root, matcher) {
                return true;
            }
        } else {
            let relative = path.strip_prefix(root).unwrap_or(&path);
            if matcher.is_match(relative) {
                return true;
            }
        }
    }

    false
}

/// Simple glob matching against a file path relative to root.
fn glob_matches(pattern: &str, path: &Path, root: &Path) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let relative_str = relative.to_string_lossy();

    globset::Glob::new(pattern)
        .ok()
        .map(|g| g.compile_matcher().is_match(relative_str.as_ref()))
        .unwrap_or(false)
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
