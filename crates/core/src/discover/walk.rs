use std::ffi::OsStr;

use fallow_config::ResolvedConfig;
use fallow_types::discover::{DiscoveredFile, FileId};
use ignore::WalkBuilder;

use super::ALLOWED_HIDDEN_DIRS;

pub const SOURCE_EXTENSIONS: &[&str] = &[
    "ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs", "vue", "svelte", "astro", "mdx", "css",
    "scss",
];

/// Glob patterns for test/dev/story files excluded in production mode.
pub const PRODUCTION_EXCLUDE_PATTERNS: &[&str] = &[
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

/// Check if a hidden directory name is on the allowlist.
pub fn is_allowed_hidden_dir(name: &OsStr) -> bool {
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
                    .unwrap_or_else(|_| entry.path());
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

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

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

    // SOURCE_EXTENSIONS tests
    #[test]
    fn source_extensions_include_typescript() {
        assert!(SOURCE_EXTENSIONS.contains(&"ts"));
        assert!(SOURCE_EXTENSIONS.contains(&"tsx"));
        assert!(SOURCE_EXTENSIONS.contains(&"mts"));
        assert!(SOURCE_EXTENSIONS.contains(&"cts"));
    }

    #[test]
    fn source_extensions_include_javascript() {
        assert!(SOURCE_EXTENSIONS.contains(&"js"));
        assert!(SOURCE_EXTENSIONS.contains(&"jsx"));
        assert!(SOURCE_EXTENSIONS.contains(&"mjs"));
        assert!(SOURCE_EXTENSIONS.contains(&"cjs"));
    }

    #[test]
    fn source_extensions_include_sfc_formats() {
        assert!(SOURCE_EXTENSIONS.contains(&"vue"));
        assert!(SOURCE_EXTENSIONS.contains(&"svelte"));
        assert!(SOURCE_EXTENSIONS.contains(&"astro"));
    }

    #[test]
    fn source_extensions_include_styles() {
        assert!(SOURCE_EXTENSIONS.contains(&"css"));
        assert!(SOURCE_EXTENSIONS.contains(&"scss"));
    }

    #[test]
    fn source_extensions_exclude_non_source() {
        assert!(!SOURCE_EXTENSIONS.contains(&"json"));
        assert!(!SOURCE_EXTENSIONS.contains(&"html"));
        assert!(!SOURCE_EXTENSIONS.contains(&"yaml"));
        assert!(!SOURCE_EXTENSIONS.contains(&"md"));
        assert!(!SOURCE_EXTENSIONS.contains(&"png"));
    }

    // PRODUCTION_EXCLUDE_PATTERNS tests
    #[test]
    fn production_excludes_test_patterns() {
        let has_test_pattern = PRODUCTION_EXCLUDE_PATTERNS
            .iter()
            .any(|p| p.contains("test") || p.contains("spec"));
        assert!(has_test_pattern, "should exclude test files in production");
    }

    #[test]
    fn production_excludes_story_patterns() {
        let has_story_pattern = PRODUCTION_EXCLUDE_PATTERNS
            .iter()
            .any(|p| p.contains("stories") || p.contains("story"));
        assert!(
            has_story_pattern,
            "should exclude story files in production"
        );
    }

    #[test]
    fn production_excludes_config_patterns() {
        let has_config_pattern = PRODUCTION_EXCLUDE_PATTERNS
            .iter()
            .any(|p| p.contains("config"));
        assert!(
            has_config_pattern,
            "should exclude config files in production"
        );
    }

    #[test]
    fn production_patterns_are_valid_globs() {
        for pattern in PRODUCTION_EXCLUDE_PATTERNS {
            assert!(
                globset::Glob::new(pattern).is_ok(),
                "pattern '{pattern}' should be a valid glob"
            );
        }
    }
}
