//! Code duplication / clone detection module.
//!
//! This module implements sliding-window hash-based clone detection
//! for JavaScript/TypeScript source files. It supports multiple detection
//! modes from strict (exact matches only) to semantic (structure-aware
//! matching that ignores identifier names and literal values).

pub mod detect;
pub mod normalize;
pub mod tokenize;
pub mod types;

use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use rayon::prelude::*;

use detect::RabinKarpDetector;
use normalize::normalize_and_hash;
use tokenize::tokenize_file;
pub use types::{
    CloneGroup, CloneInstance, DetectionMode, DuplicatesConfig, DuplicationReport, DuplicationStats,
};

use crate::discover::{self, DiscoveredFile};

/// Run duplication detection on the given files.
///
/// This is the main entry point for the duplication analysis. It:
/// 1. Reads and tokenizes all source files in parallel
/// 2. Normalizes tokens according to the detection mode
/// 3. Runs Rabin-Karp clone detection
/// 4. Groups and reports clone instances
pub fn find_duplicates(
    root: &Path,
    files: &[DiscoveredFile],
    config: &DuplicatesConfig,
) -> DuplicationReport {
    let _span = tracing::info_span!("find_duplicates").entered();

    // Build extra ignore patterns for duplication analysis
    let extra_ignores = build_ignore_set(&config.ignore);

    // Step 1 & 2: Tokenize and normalize all files in parallel
    let file_data: Vec<(PathBuf, Vec<normalize::HashedToken>, tokenize::FileTokens)> = files
        .par_iter()
        .filter_map(|file| {
            // Apply extra ignore patterns
            let relative = file.path.strip_prefix(root).unwrap_or(&file.path);
            if let Some(ref ignores) = extra_ignores
                && ignores.is_match(relative)
            {
                return None;
            }

            // Read the file
            let source = std::fs::read_to_string(&file.path).ok()?;

            // Tokenize
            let file_tokens = tokenize_file(&file.path, &source);
            if file_tokens.tokens.is_empty() {
                return None;
            }

            // Normalize and hash
            let hashed = normalize_and_hash(&file_tokens.tokens, config.mode);
            if hashed.len() < config.min_tokens {
                return None;
            }

            Some((file.path.clone(), hashed, file_tokens))
        })
        .collect();

    tracing::info!(
        files = file_data.len(),
        "tokenized files for duplication analysis"
    );

    // Step 3 & 4: Detect clones
    let detector = RabinKarpDetector::new(config.min_tokens, config.min_lines, config.skip_local);
    detector.detect(file_data)
}

/// Run duplication detection on a project directory using auto-discovered files.
///
/// This is a convenience function that handles file discovery internally.
pub fn find_duplicates_in_project(root: &Path, config: &DuplicatesConfig) -> DuplicationReport {
    let resolved = crate::default_config(root);
    let files = discover::discover_files(&resolved);
    find_duplicates(root, &files, config)
}

/// Build a GlobSet from ignore patterns.
fn build_ignore_set(patterns: &[String]) -> Option<GlobSet> {
    if patterns.is_empty() {
        return None;
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        match Glob::new(pattern) {
            Ok(glob) => {
                builder.add(glob);
            }
            Err(e) => {
                tracing::warn!("Invalid duplication ignore pattern '{pattern}': {e}");
            }
        }
    }

    builder.build().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::FileId;

    #[test]
    fn find_duplicates_empty_files() {
        let config = DuplicatesConfig::default();
        let report = find_duplicates(Path::new("/tmp"), &[], &config);
        assert!(report.clone_groups.is_empty());
        assert_eq!(report.stats.total_files, 0);
    }

    #[test]
    fn build_ignore_set_empty() {
        assert!(build_ignore_set(&[]).is_none());
    }

    #[test]
    fn build_ignore_set_valid_patterns() {
        let set = build_ignore_set(&["**/*.test.ts".to_string(), "**/*.spec.ts".to_string()]);
        assert!(set.is_some());
        let set = set.unwrap();
        assert!(set.is_match("src/foo.test.ts"));
        assert!(set.is_match("src/bar.spec.ts"));
        assert!(!set.is_match("src/baz.ts"));
    }

    #[test]
    fn find_duplicates_with_real_files() {
        // Create a temp directory with duplicate files
        let dir = tempfile::tempdir().expect("create temp dir");
        let src_dir = dir.path().join("src");
        std::fs::create_dir_all(&src_dir).expect("create src dir");

        let code = r#"
export function processData(input: string): string {
    const trimmed = input.trim();
    if (trimmed.length === 0) {
        return "";
    }
    const parts = trimmed.split(",");
    const filtered = parts.filter(p => p.length > 0);
    const mapped = filtered.map(p => p.toUpperCase());
    return mapped.join(", ");
}

export function validateInput(data: string): boolean {
    if (data === null || data === undefined) {
        return false;
    }
    const cleaned = data.trim();
    if (cleaned.length < 3) {
        return false;
    }
    return true;
}
"#;

        std::fs::write(src_dir.join("original.ts"), code).expect("write original");
        std::fs::write(src_dir.join("copy.ts"), code).expect("write copy");
        std::fs::write(dir.path().join("package.json"), r#"{"name": "test"}"#)
            .expect("write package.json");

        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: src_dir.join("original.ts"),
                size_bytes: code.len() as u64,
            },
            DiscoveredFile {
                id: FileId(1),
                path: src_dir.join("copy.ts"),
                size_bytes: code.len() as u64,
            },
        ];

        let config = DuplicatesConfig {
            min_tokens: 10,
            min_lines: 2,
            ..DuplicatesConfig::default()
        };

        let report = find_duplicates(dir.path(), &files, &config);
        assert!(
            !report.clone_groups.is_empty(),
            "Should detect clones in identical files"
        );
        assert!(report.stats.files_with_clones >= 2);
    }
}
