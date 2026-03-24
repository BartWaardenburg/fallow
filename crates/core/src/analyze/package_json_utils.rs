use std::path::Path;

/// Find the 1-based line number of a dependency key in a package.json file.
///
/// Searches the raw file content for `"<package_name>"` followed by `:` on the
/// same line. Skips JSONC comment lines. Returns 1 if not found (safe fallback).
pub fn find_dep_line_in_json(content: &str, package_name: &str) -> u32 {
    let needle = format!("\"{package_name}\"");
    let mut in_block_comment = false;
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        // Track block comments
        if in_block_comment {
            if let Some(end) = trimmed.find("*/") {
                // Block comment ends on this line — check the remainder after `*/`
                let rest = &trimmed[end + 2..];
                in_block_comment = false;
                if let Some(pos) = rest.find(&*needle) {
                    let after = &rest[pos + needle.len()..];
                    if after.trim_start().starts_with(':') {
                        return (i + 1) as u32;
                    }
                }
            }
            continue;
        }
        // Skip line comments
        if trimmed.starts_with("//") {
            continue;
        }
        // Start of block comment
        if let Some(after_open) = trimmed.strip_prefix("/*") {
            if let Some(end) = after_open.find("*/") {
                // Single-line block comment — check remainder after `*/`
                let rest = &after_open[end + 2..];
                if let Some(pos) = rest.find(&*needle) {
                    let after = &rest[pos + needle.len()..];
                    if after.trim_start().starts_with(':') {
                        return (i + 1) as u32;
                    }
                }
            } else {
                in_block_comment = true;
            }
            continue;
        }
        if let Some(pos) = line.find(&needle) {
            // Verify it's a key (followed by `:` after optional whitespace)
            let after = &line[pos + needle.len()..];
            if after.trim_start().starts_with(':') {
                return (i + 1) as u32;
            }
        }
    }
    1
}

/// Read a package.json file's raw text for line-number scanning.
pub fn read_pkg_json_content(pkg_path: &Path) -> Option<String> {
    std::fs::read_to_string(pkg_path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_dep_line_finds_dependency_key() {
        let content = r#"{
  "name": "my-app",
  "dependencies": {
    "react": "^18.0.0",
    "lodash": "^4.17.21"
  }
}"#;
        assert_eq!(find_dep_line_in_json(content, "lodash"), 5);
        assert_eq!(find_dep_line_in_json(content, "react"), 4);
    }

    #[test]
    fn find_dep_line_returns_1_when_not_found() {
        let content = r#"{ "dependencies": {} }"#;
        assert_eq!(find_dep_line_in_json(content, "missing"), 1);
    }

    #[test]
    fn find_dep_line_handles_scoped_packages() {
        let content = r#"{
  "devDependencies": {
    "@typescript-eslint/parser": "^6.0.0"
  }
}"#;
        assert_eq!(
            find_dep_line_in_json(content, "@typescript-eslint/parser"),
            3
        );
    }

    #[test]
    fn find_dep_line_skips_line_comments() {
        let content = r#"{
  // "lodash": "old version",
  "dependencies": {
    "lodash": "^4.17.21"
  }
}"#;
        assert_eq!(find_dep_line_in_json(content, "lodash"), 4);
    }

    #[test]
    fn find_dep_line_skips_block_comments() {
        let content = r#"{
  /* "lodash": "old" */
  "dependencies": {
    "lodash": "^4.17.21"
  }
}"#;
        assert_eq!(find_dep_line_in_json(content, "lodash"), 4);
    }

    #[test]
    fn find_dep_line_skips_multiline_block_comment() {
        let content = r#"{
  /*
    "lodash": "commented out",
    "react": "also commented"
  */
  "dependencies": {
    "lodash": "^4.17.21"
  }
}"#;
        // "lodash" inside the multi-line block comment is skipped; real one is on line 7
        assert_eq!(find_dep_line_in_json(content, "lodash"), 7);
    }

    #[test]
    fn find_dep_line_after_block_comment_end_on_same_line() {
        // Single-line block comment: the remainder after `*/` is scanned for the dep key.
        let content = r#"{
  /* comment */ "lodash": "^4.17.21"
}"#;
        assert_eq!(find_dep_line_in_json(content, "lodash"), 2);
    }

    #[test]
    fn find_dep_line_dep_inside_and_after_block_comment() {
        // The dep name appears inside the comment AND as a real key after it.
        // Must match the post-comment occurrence, not the in-comment one.
        let content = "{\n  /* \"lodash\": \"old\" */ \"lodash\": \"^4.17.21\"\n}";
        assert_eq!(find_dep_line_in_json(content, "lodash"), 2);
    }

    #[test]
    fn find_dep_line_minimal_block_comment() {
        // Minimal block comment `/**/` followed by a dep key.
        let content = "{\n  /**/ \"lodash\": \"^4.17.21\"\n}";
        assert_eq!(find_dep_line_in_json(content, "lodash"), 2);
    }

    #[test]
    fn find_dep_line_multiline_block_comment_end_with_dep_on_remainder() {
        // Tests the branch where a multi-line block comment ends and the dep key
        // appears on the remainder of the same line after "*/"
        let content = "{\n  /* start of comment\n  end */ \"lodash\": \"^4.17.21\"\n}";
        // Line 1: {
        // Line 2: /* start of comment    <-- sets in_block_comment = true
        // Line 3: end */ "lodash": "^4.17.21"  <-- comment ends, remainder has dep
        assert_eq!(find_dep_line_in_json(content, "lodash"), 3);
    }

    #[test]
    fn find_dep_line_block_comment_end_without_dep_on_remainder() {
        // Block comment ends but the remainder does NOT have the dep key
        let content =
            "{\n  /* start\n  end */ \"other\": \"1.0.0\",\n  \"lodash\": \"^4.17.21\"\n}";
        // The dep "lodash" is on line 4, after the block comment ends on line 3
        assert_eq!(find_dep_line_in_json(content, "lodash"), 4);
    }

    #[test]
    fn find_dep_line_value_not_key_is_not_matched() {
        // "lodash" appears as a VALUE, not a key -- should not match
        let content = r#"{
  "dependencies": {
    "my-lodash-wrapper": "lodash"
  }
}"#;
        // "lodash" appears in the value but NOT as a key (not followed by ":")
        // "my-lodash-wrapper" IS a key.
        assert_eq!(find_dep_line_in_json(content, "lodash"), 1);
        assert_eq!(find_dep_line_in_json(content, "my-lodash-wrapper"), 3);
    }

    #[test]
    fn find_dep_line_empty_content() {
        assert_eq!(find_dep_line_in_json("", "lodash"), 1);
    }

    #[test]
    fn find_dep_line_multiple_dep_sections() {
        let content = r#"{
  "dependencies": {
    "react": "^18.0.0"
  },
  "devDependencies": {
    "react": "^18.0.0"
  }
}"#;
        // Should find the FIRST occurrence (line 3)
        assert_eq!(find_dep_line_in_json(content, "react"), 3);
    }

    #[test]
    fn find_dep_line_malformed_content() {
        // Non-JSON content should not panic and should return 1 (fallback)
        assert_eq!(find_dep_line_in_json("this is not json at all", "lodash"), 1);
        assert_eq!(find_dep_line_in_json("{{{", "lodash"), 1);
        assert_eq!(find_dep_line_in_json("null", "lodash"), 1);
    }

    #[test]
    fn read_pkg_json_content_nonexistent_path() {
        let result = read_pkg_json_content(Path::new("/nonexistent/path/package.json"));
        assert!(result.is_none(), "nonexistent path should return None");
    }

    #[test]
    fn read_pkg_json_content_valid_path() {
        // Create a temp file and read it back
        let dir = std::env::temp_dir().join("fallow_test_read_pkg");
        let _ = std::fs::create_dir_all(&dir);
        let pkg_path = dir.join("package.json");
        std::fs::write(&pkg_path, r#"{"name": "test"}"#).expect("write temp file");

        let result = read_pkg_json_content(&pkg_path);
        assert!(result.is_some(), "valid path should return Some");
        assert_eq!(result.unwrap(), r#"{"name": "test"}"#);

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_dep_line_deeply_nested_scoped_package() {
        // Scoped package with multiple levels in the path
        let content = r#"{
  "dependencies": {
    "@babel/plugin-transform-runtime": "^7.0.0",
    "@babel/core": "^7.0.0"
  }
}"#;
        assert_eq!(
            find_dep_line_in_json(content, "@babel/plugin-transform-runtime"),
            3
        );
        assert_eq!(find_dep_line_in_json(content, "@babel/core"), 4);
    }

    #[test]
    fn find_dep_line_with_trailing_comma_jsonc() {
        // JSONC allows trailing commas — verify the search still works
        let content = "{\n  \"dependencies\": {\n    \"lodash\": \"^4.17.21\",\n  }\n}";
        assert_eq!(find_dep_line_in_json(content, "lodash"), 3);
    }
}
