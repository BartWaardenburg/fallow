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

    // 1. Manual entries from config
    for pattern in &config.entry_patterns {
        for file in files {
            if glob_matches(pattern, &file.path, &config.root) {
                entries.push(EntryPoint {
                    path: file.path.clone(),
                    source: EntryPointSource::ManualEntry,
                });
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

        // 3. Framework rules
        for rule in &config.framework_rules {
            if !is_framework_active(rule, &pkg, &config.root) {
                continue;
            }

            for entry_pat in &rule.entry_points {
                for file in files {
                    if glob_matches(&entry_pat.pattern, &file.path, &config.root) {
                        entries.push(EntryPoint {
                            path: file.path.clone(),
                            source: EntryPointSource::FrameworkRule {
                                name: rule.name.clone(),
                            },
                        });
                    }
                }
            }

            for pattern in &rule.always_used {
                for file in files {
                    if glob_matches(pattern, &file.path, &config.root) {
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

    // 4. Default index files (if no other entries found)
    if entries.is_empty() {
        let default_patterns = [
            "src/index.{ts,tsx,js,jsx}",
            "src/main.{ts,tsx,js,jsx}",
            "index.{ts,tsx,js,jsx}",
            "main.{ts,tsx,js,jsx}",
        ];
        for pattern in &default_patterns {
            for file in files {
                if glob_matches(pattern, &file.path, &config.root) {
                    entries.push(EntryPoint {
                        path: file.path.clone(),
                        source: EntryPointSource::DefaultIndex,
                    });
                }
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
        FrameworkDetection::FileExists { pattern } => {
            let full = root.join(pattern);
            // Simple check — for glob patterns we'd need to actually glob
            full.exists() || {
                globset::Glob::new(pattern)
                    .ok()
                    .and_then(|g| g.compile_matcher().is_match(root).then_some(true))
                    .unwrap_or(false)
            }
        }
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

/// Simple glob matching against a file path relative to root.
fn glob_matches(pattern: &str, path: &Path, root: &Path) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let relative_str = relative.to_string_lossy();

    globset::Glob::new(pattern)
        .ok()
        .map(|g| g.compile_matcher().is_match(relative_str.as_ref()))
        .unwrap_or(false)
}
