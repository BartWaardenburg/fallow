use std::io::Read as _;
use std::path::{Path, PathBuf};

/// Parse `tsconfig.json` at the project root and extract `references[].path` directories.
///
/// Returns directories that exist on disk. tsconfig.json is JSONC (comments + trailing commas),
/// so we strip both before parsing.
pub(super) fn parse_tsconfig_references(root: &Path) -> Vec<PathBuf> {
    let tsconfig_path = root.join("tsconfig.json");
    let Ok(content) = std::fs::read_to_string(&tsconfig_path) else {
        return Vec::new();
    };

    // Strip UTF-8 BOM if present (common in Windows-authored tsconfig files)
    let content = content.trim_start_matches('\u{FEFF}');

    // Strip JSONC comments
    let mut stripped = String::new();
    if json_comments::StripComments::new(content.as_bytes())
        .read_to_string(&mut stripped)
        .is_err()
    {
        return Vec::new();
    }

    // Strip trailing commas (common in tsconfig.json)
    let cleaned = strip_trailing_commas(&stripped);

    let Ok(value) = serde_json::from_str::<serde_json::Value>(&cleaned) else {
        return Vec::new();
    };

    let Some(refs) = value.get("references").and_then(|v| v.as_array()) else {
        return Vec::new();
    };

    refs.iter()
        .filter_map(|r| {
            r.get("path").and_then(|p| p.as_str()).map(|p| {
                // strip_prefix removes exactly one leading "./" (unlike trim_start_matches
                // which would strip repeatedly)
                let cleaned = p.strip_prefix("./").unwrap_or(p);
                root.join(cleaned)
            })
        })
        .filter(|p| p.is_dir())
        .collect()
}

/// Strip trailing commas before `]` and `}` in JSON-like content.
///
/// tsconfig.json commonly uses trailing commas which are valid JSONC but not valid JSON.
/// This strips them so `serde_json` can parse the content.
pub(super) fn strip_trailing_commas(input: &str) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut result = Vec::with_capacity(len);
    let mut in_string = false;
    let mut i = 0;

    while i < len {
        let b = bytes[i];

        if in_string {
            result.push(b);
            if b == b'\\' && i + 1 < len {
                // Push escaped character and skip it
                i += 1;
                result.push(bytes[i]);
            } else if b == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if b == b'"' {
            in_string = true;
            result.push(b);
            i += 1;
            continue;
        }

        if b == b',' {
            // Look ahead past whitespace for ] or }
            let mut j = i + 1;
            while j < len && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < len && (bytes[j] == b']' || bytes[j] == b'}') {
                // Skip the trailing comma
                i += 1;
                continue;
            }
        }

        result.push(b);
        i += 1;
    }

    // We only removed ASCII commas and preserved all other bytes unchanged,
    // so the result is valid UTF-8 if the input was. Use from_utf8 to be safe.
    String::from_utf8(result).unwrap_or_else(|_| input.to_string())
}

/// Expand a workspace glob pattern to matching directories.
///
/// Returns `(original_path, canonical_path)` tuples so callers can skip redundant
/// `canonicalize()` calls. Only directories containing a `package.json` are
/// canonicalized — this avoids expensive syscalls on the many non-workspace
/// directories that globs like `packages/*` or `**` can match.
///
/// `canonical_root` is pre-computed to avoid repeated `canonicalize()` syscalls.
pub(super) fn expand_workspace_glob(
    root: &Path,
    pattern: &str,
    canonical_root: &Path,
) -> Vec<(PathBuf, PathBuf)> {
    let full_pattern = root.join(pattern).to_string_lossy().to_string();
    match glob::glob(&full_pattern) {
        Ok(paths) => paths
            .filter_map(Result::ok)
            .filter(|p| p.is_dir())
            // Fast pre-filter: skip directories without package.json before
            // paying the cost of canonicalize() (the P0 perf fix — avoids
            // canonicalizing 759+ non-workspace dirs in large monorepos).
            .filter(|p| p.join("package.json").exists())
            .filter_map(|p| {
                // Security: ensure workspace directory is within project root
                p.canonicalize()
                    .ok()
                    .filter(|cp| cp.starts_with(canonical_root))
                    .map(|cp| (p, cp))
            })
            .collect(),
        Err(e) => {
            tracing::warn!("invalid workspace glob pattern '{pattern}': {e}");
            Vec::new()
        }
    }
}

/// Parse pnpm-workspace.yaml to extract package patterns.
pub(super) fn parse_pnpm_workspace_yaml(content: &str) -> Vec<String> {
    // Simple YAML parsing for the common format:
    // packages:
    //   - 'packages/*'
    //   - 'apps/*'
    let mut patterns = Vec::new();
    let mut in_packages = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "packages:" {
            in_packages = true;
            continue;
        }
        if in_packages {
            if trimmed.starts_with("- ") {
                let value = trimmed
                    .strip_prefix("- ")
                    .unwrap_or(trimmed)
                    .trim_matches('\'')
                    .trim_matches('"');
                patterns.push(value.to_string());
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                break; // New top-level key
            }
        }
    }

    patterns
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pnpm_workspace_basic() {
        let yaml = "packages:\n  - 'packages/*'\n  - 'apps/*'\n";
        let patterns = parse_pnpm_workspace_yaml(yaml);
        assert_eq!(patterns, vec!["packages/*", "apps/*"]);
    }

    #[test]
    fn parse_pnpm_workspace_double_quotes() {
        let yaml = "packages:\n  - \"packages/*\"\n  - \"apps/*\"\n";
        let patterns = parse_pnpm_workspace_yaml(yaml);
        assert_eq!(patterns, vec!["packages/*", "apps/*"]);
    }

    #[test]
    fn parse_pnpm_workspace_no_quotes() {
        let yaml = "packages:\n  - packages/*\n  - apps/*\n";
        let patterns = parse_pnpm_workspace_yaml(yaml);
        assert_eq!(patterns, vec!["packages/*", "apps/*"]);
    }

    #[test]
    fn parse_pnpm_workspace_empty() {
        let yaml = "";
        let patterns = parse_pnpm_workspace_yaml(yaml);
        assert!(patterns.is_empty());
    }

    #[test]
    fn parse_pnpm_workspace_no_packages_key() {
        let yaml = "other:\n  - something\n";
        let patterns = parse_pnpm_workspace_yaml(yaml);
        assert!(patterns.is_empty());
    }

    #[test]
    fn parse_pnpm_workspace_with_comments() {
        let yaml = "packages:\n  # Comment\n  - 'packages/*'\n";
        let patterns = parse_pnpm_workspace_yaml(yaml);
        assert_eq!(patterns, vec!["packages/*"]);
    }

    #[test]
    fn parse_pnpm_workspace_stops_at_next_key() {
        let yaml = "packages:\n  - 'packages/*'\ncatalog:\n  react: ^18\n";
        let patterns = parse_pnpm_workspace_yaml(yaml);
        assert_eq!(patterns, vec!["packages/*"]);
    }

    #[test]
    fn strip_trailing_commas_basic() {
        assert_eq!(
            strip_trailing_commas(r#"{"a": 1, "b": 2,}"#),
            r#"{"a": 1, "b": 2}"#
        );
    }

    #[test]
    fn strip_trailing_commas_array() {
        assert_eq!(strip_trailing_commas(r"[1, 2, 3,]"), r"[1, 2, 3]");
    }

    #[test]
    fn strip_trailing_commas_with_whitespace() {
        assert_eq!(
            strip_trailing_commas("{\n  \"a\": 1,\n}"),
            "{\n  \"a\": 1\n}"
        );
    }

    #[test]
    fn strip_trailing_commas_preserves_strings() {
        // Commas inside strings should NOT be stripped
        assert_eq!(
            strip_trailing_commas(r#"{"a": "hello,}"}"#),
            r#"{"a": "hello,}"}"#
        );
    }

    #[test]
    fn strip_trailing_commas_nested() {
        let input = r#"{"refs": [{"path": "./a",}, {"path": "./b",},],}"#;
        let expected = r#"{"refs": [{"path": "./a"}, {"path": "./b"}]}"#;
        assert_eq!(strip_trailing_commas(input), expected);
    }

    #[test]
    fn strip_trailing_commas_escaped_quotes() {
        assert_eq!(
            strip_trailing_commas(r#"{"a": "he\"llo,}",}"#),
            r#"{"a": "he\"llo,}"}"#
        );
    }

    #[test]
    fn tsconfig_references_from_dir() {
        let temp_dir = std::env::temp_dir().join("fallow-test-tsconfig-refs");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(temp_dir.join("packages/core")).unwrap();
        std::fs::create_dir_all(temp_dir.join("packages/ui")).unwrap();

        std::fs::write(
            temp_dir.join("tsconfig.json"),
            r#"{
                // Root tsconfig with project references
                "references": [
                    {"path": "./packages/core"},
                    {"path": "./packages/ui"},
                ],
            }"#,
        )
        .unwrap();

        let refs = parse_tsconfig_references(&temp_dir);
        assert_eq!(refs.len(), 2);
        assert!(refs.iter().any(|p| p.ends_with("packages/core")));
        assert!(refs.iter().any(|p| p.ends_with("packages/ui")));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn tsconfig_references_no_file() {
        let refs = parse_tsconfig_references(std::path::Path::new("/nonexistent"));
        assert!(refs.is_empty());
    }

    #[test]
    fn tsconfig_references_no_references_field() {
        let temp_dir = std::env::temp_dir().join("fallow-test-tsconfig-no-refs");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        std::fs::write(
            temp_dir.join("tsconfig.json"),
            r#"{"compilerOptions": {"strict": true}}"#,
        )
        .unwrap();

        let refs = parse_tsconfig_references(&temp_dir);
        assert!(refs.is_empty());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn tsconfig_references_skips_nonexistent_dirs() {
        let temp_dir = std::env::temp_dir().join("fallow-test-tsconfig-missing-dir");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(temp_dir.join("packages/core")).unwrap();

        std::fs::write(
            temp_dir.join("tsconfig.json"),
            r#"{"references": [{"path": "./packages/core"}, {"path": "./packages/missing"}]}"#,
        )
        .unwrap();

        let refs = parse_tsconfig_references(&temp_dir);
        assert_eq!(refs.len(), 1);
        assert!(refs[0].ends_with("packages/core"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn strip_trailing_commas_no_commas() {
        let input = r#"{"a": 1, "b": [2, 3]}"#;
        assert_eq!(strip_trailing_commas(input), input);
    }

    #[test]
    fn strip_trailing_commas_empty_input() {
        assert_eq!(strip_trailing_commas(""), "");
    }

    #[test]
    fn strip_trailing_commas_nested_objects() {
        let input = "{\n  \"a\": {\n    \"b\": 1,\n    \"c\": 2,\n  },\n  \"d\": 3,\n}";
        let expected = "{\n  \"a\": {\n    \"b\": 1,\n    \"c\": 2\n  },\n  \"d\": 3\n}";
        assert_eq!(strip_trailing_commas(input), expected);
    }

    #[test]
    fn strip_trailing_commas_array_of_objects() {
        let input = r#"[{"a": 1,}, {"b": 2,},]"#;
        let expected = r#"[{"a": 1}, {"b": 2}]"#;
        assert_eq!(strip_trailing_commas(input), expected);
    }

    #[test]
    fn tsconfig_references_malformed_json() {
        let temp_dir = std::env::temp_dir().join("fallow-test-tsconfig-malformed");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        std::fs::write(
            temp_dir.join("tsconfig.json"),
            r"{ this is not valid json at all",
        )
        .unwrap();

        let refs = parse_tsconfig_references(&temp_dir);
        assert!(refs.is_empty());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn tsconfig_references_empty_array() {
        let temp_dir = std::env::temp_dir().join("fallow-test-tsconfig-empty-refs");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        std::fs::write(temp_dir.join("tsconfig.json"), r#"{"references": []}"#).unwrap();

        let refs = parse_tsconfig_references(&temp_dir);
        assert!(refs.is_empty());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn parse_pnpm_workspace_malformed() {
        // Garbage input should return empty, not panic
        let patterns = parse_pnpm_workspace_yaml(":::not yaml at all:::");
        assert!(patterns.is_empty());
    }

    #[test]
    fn parse_pnpm_workspace_packages_key_empty_list() {
        let yaml = "packages:\nother:\n  - something\n";
        let patterns = parse_pnpm_workspace_yaml(yaml);
        assert!(patterns.is_empty());
    }

    #[test]
    fn expand_workspace_glob_exact_path() {
        let temp_dir = std::env::temp_dir().join("fallow-test-expand-exact");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(temp_dir.join("packages/core")).unwrap();
        std::fs::write(
            temp_dir.join("packages/core/package.json"),
            r#"{"name": "core"}"#,
        )
        .unwrap();

        let canonical_root = temp_dir.canonicalize().unwrap();
        let results = expand_workspace_glob(&temp_dir, "packages/core", &canonical_root);
        assert_eq!(results.len(), 1);
        assert!(results[0].0.ends_with("packages/core"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn expand_workspace_glob_star() {
        let temp_dir = std::env::temp_dir().join("fallow-test-expand-star");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(temp_dir.join("packages/a")).unwrap();
        std::fs::create_dir_all(temp_dir.join("packages/b")).unwrap();
        std::fs::create_dir_all(temp_dir.join("packages/c")).unwrap();
        std::fs::write(temp_dir.join("packages/a/package.json"), r#"{"name": "a"}"#).unwrap();
        std::fs::write(temp_dir.join("packages/b/package.json"), r#"{"name": "b"}"#).unwrap();
        // c has no package.json — should be excluded

        let canonical_root = temp_dir.canonicalize().unwrap();
        let results = expand_workspace_glob(&temp_dir, "packages/*", &canonical_root);
        assert_eq!(results.len(), 2);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn expand_workspace_glob_nested() {
        let temp_dir = std::env::temp_dir().join("fallow-test-expand-nested");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(temp_dir.join("packages/scope/a")).unwrap();
        std::fs::create_dir_all(temp_dir.join("packages/scope/b")).unwrap();
        std::fs::write(
            temp_dir.join("packages/scope/a/package.json"),
            r#"{"name": "@scope/a"}"#,
        )
        .unwrap();
        std::fs::write(
            temp_dir.join("packages/scope/b/package.json"),
            r#"{"name": "@scope/b"}"#,
        )
        .unwrap();

        let canonical_root = temp_dir.canonicalize().unwrap();
        let results = expand_workspace_glob(&temp_dir, "packages/**/*", &canonical_root);
        assert_eq!(results.len(), 2);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn expand_workspace_glob_no_matches() {
        let temp_dir = std::env::temp_dir().join("fallow-test-expand-nomatch");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let canonical_root = temp_dir.canonicalize().unwrap();
        let results = expand_workspace_glob(&temp_dir, "nonexistent/*", &canonical_root);
        assert!(results.is_empty());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
