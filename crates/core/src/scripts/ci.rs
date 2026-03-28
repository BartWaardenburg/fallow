//! CI config file scanner for dependency usage detection.
//!
//! Extracts shell commands from `.gitlab-ci.yml` and `.github/workflows/*.yml`
//! files, parses them for binary invocations (especially `npx`), and maps
//! binaries to npm package names. This prevents false "unused dependency"
//! reports for packages only used in CI pipelines.

use std::path::Path;

use rustc_hash::FxHashSet;

use super::{parse_script, resolve_binary_to_package};

/// Analyze CI config files for package binary invocations.
///
/// Scans GitLab CI and GitHub Actions workflow files for shell commands,
/// extracts binary names, and returns the set of npm package names used.
pub fn analyze_ci_files(root: &Path) -> FxHashSet<String> {
    let _span = tracing::info_span!("analyze_ci_files").entered();
    let mut used_packages = FxHashSet::default();

    // GitLab CI
    let gitlab_ci = root.join(".gitlab-ci.yml");
    if let Ok(content) = std::fs::read_to_string(&gitlab_ci) {
        extract_ci_packages(&content, root, &mut used_packages);
    }

    // GitHub Actions workflows
    let workflows_dir = root.join(".github/workflows");
    if let Ok(entries) = std::fs::read_dir(&workflows_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if (name_str.ends_with(".yml") || name_str.ends_with(".yaml"))
                && let Ok(content) = std::fs::read_to_string(entry.path())
            {
                extract_ci_packages(&content, root, &mut used_packages);
            }
        }
    }

    used_packages
}

/// Extract package names from shell commands found in a CI config file.
///
/// Uses line-based heuristics to find shell command lines in YAML CI configs.
/// This intentionally avoids a full YAML parser to keep dependencies minimal.
/// Since results only mark packages as "used" (never as "unused"), false
/// positives from non-command YAML lines are safe — they only reduce
/// false positive unused dependency reports.
fn extract_ci_packages(content: &str, root: &Path, packages: &mut FxHashSet<String>) {
    for command in extract_ci_commands(content) {
        let parsed = parse_script(&command);
        for cmd in parsed {
            if !cmd.binary.is_empty() && !super::is_builtin_command(&cmd.binary) {
                let pkg = resolve_binary_to_package(&cmd.binary, root);
                packages.insert(pkg);
            }
        }
    }
}

/// Extract shell command strings from a CI config file.
///
/// Recognizes:
/// - YAML list items in script blocks: `  - npx tool --flag`
/// - GitHub Actions run fields: `  run: command`
/// - Multi-line run blocks: `  run: |` followed by indented lines
fn extract_ci_commands(content: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut in_multiline_run = false;
    let mut multiline_indent = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Track multi-line `run: |` blocks (GitHub Actions)
        if in_multiline_run {
            let indent = line.len() - line.trim_start().len();
            if indent > multiline_indent && !trimmed.is_empty() {
                commands.push(trimmed.to_string());
                continue;
            }
            in_multiline_run = false;
            // Fall through to re-classify this line normally
        }

        // GitHub Actions: `run: |` or `- run: command` (multi-line or inline)
        // Check both bare `run:` and list-item `- run:` forms
        let run_value = strip_yaml_key(trimmed, "run")
            .or_else(|| {
                trimmed
                    .strip_prefix("- ")
                    .and_then(|rest| strip_yaml_key(rest.trim(), "run"))
            })
            .map(str::trim);

        if let Some(rest) = run_value {
            if rest == "|" || rest == "|-" || rest == "|+" {
                in_multiline_run = true;
                multiline_indent = line.len() - line.trim_start().len();
            } else if !rest.is_empty() {
                // Inline run: `run: npm test` or `- run: npm test`
                commands.push(rest.to_string());
            }
            continue;
        }

        // YAML list items in script/before_script/after_script blocks
        // GitLab CI: `  - npx @cyclonedx/cyclonedx-npm --output-file sbom.json`
        // These are the most common form of CI commands
        if let Some(rest) = trimmed.strip_prefix("- ") {
            let rest = rest.trim();
            // Skip YAML mappings (key: value), image references, and other non-commands
            if !rest.is_empty()
                && !rest.starts_with('{')
                && !rest.starts_with('[')
                && !is_yaml_mapping(rest)
            {
                commands.push(rest.to_string());
            }
        }
    }

    commands
}

/// Strip a YAML key prefix from a line, returning the value part.
/// Handles `key: value` and `key:` (empty value).
fn strip_yaml_key<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(key)?;
    let rest = rest.strip_prefix(':')?;
    Some(rest)
}

/// Check if a string looks like a YAML mapping (key: value) rather than a shell command.
fn is_yaml_mapping(s: &str) -> bool {
    // Simple heuristic: if the first "word" ends with `:` and doesn't look like
    // a protocol (http:, https:, ftp:), it's likely a YAML key
    if let Some(first_word) = s.split_whitespace().next()
        && first_word.ends_with(':')
        && !first_word.starts_with("http")
        && !first_word.starts_with("ftp")
    {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_ci_commands tests ──────────────────────────────────

    #[test]
    fn gitlab_ci_script_items() {
        let content = r"
stages:
  - build
  - test

build:
  stage: build
  script:
    - npm ci
    - npx @cyclonedx/cyclonedx-npm --output-file sbom.json
    - npm run build
";
        let commands = extract_ci_commands(content);
        assert!(commands.contains(&"npm ci".to_string()));
        assert!(
            commands.contains(&"npx @cyclonedx/cyclonedx-npm --output-file sbom.json".to_string())
        );
        assert!(commands.contains(&"npm run build".to_string()));
    }

    #[test]
    fn github_actions_inline_run() {
        let content = r"
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm ci
      - run: npx eslint src
";
        let commands = extract_ci_commands(content);
        assert!(commands.contains(&"npm ci".to_string()));
        assert!(commands.contains(&"npx eslint src".to_string()));
    }

    #[test]
    fn github_actions_multiline_run() {
        let content = r"
jobs:
  build:
    steps:
      - run: |
          npm ci
          npx @cyclonedx/cyclonedx-npm --output sbom.json
          npm run build
";
        let commands = extract_ci_commands(content);
        assert!(commands.contains(&"npm ci".to_string()));
        assert!(commands.contains(&"npx @cyclonedx/cyclonedx-npm --output sbom.json".to_string()));
        assert!(commands.contains(&"npm run build".to_string()));
    }

    #[test]
    fn yaml_mappings_filtered() {
        let content = r"
image: node:18
stages:
  - build
variables:
  NODE_ENV: production
build:
  script:
    - npm ci
";
        let commands = extract_ci_commands(content);
        // "node:18" and "NODE_ENV: production" should NOT be treated as commands
        assert!(!commands.iter().any(|c| c.contains("node:18")));
        assert!(!commands.iter().any(|c| c.contains("NODE_ENV")));
        assert!(commands.contains(&"npm ci".to_string()));
    }

    #[test]
    fn comments_and_empty_lines_skipped() {
        let content = r"
# This is a comment
  # Indented comment

build:
  script:
    - npm ci
";
        let commands = extract_ci_commands(content);
        assert_eq!(commands, vec!["npm ci"]);
    }

    // ── extract_ci_packages tests ──────────────────────────────────

    #[test]
    fn npx_package_extracted() {
        let content = r"
build:
  script:
    - npx @cyclonedx/cyclonedx-npm --output-file sbom.json
";
        let mut packages = FxHashSet::default();
        extract_ci_packages(content, Path::new("/nonexistent"), &mut packages);
        assert!(
            packages.contains("@cyclonedx/cyclonedx-npm"),
            "packages: {packages:?}"
        );
    }

    #[test]
    fn multiple_binaries_extracted() {
        let content = r"
build:
  script:
    - npx eslint src
    - npx prettier --check .
    - tsc --noEmit
";
        let mut packages = FxHashSet::default();
        extract_ci_packages(content, Path::new("/nonexistent"), &mut packages);
        assert!(packages.contains("eslint"));
        assert!(packages.contains("prettier"));
        assert!(packages.contains("typescript")); // tsc → typescript via resolve
    }

    #[test]
    fn builtin_commands_not_extracted() {
        let content = r"
build:
  script:
    - echo 'hello'
    - mkdir -p dist
    - cp -r build/* dist/
";
        let mut packages = FxHashSet::default();
        extract_ci_packages(content, Path::new("/nonexistent"), &mut packages);
        assert!(
            packages.is_empty(),
            "should not extract built-in commands: {packages:?}"
        );
    }

    #[test]
    fn github_actions_npx_extracted() {
        let content = r"
jobs:
  sbom:
    steps:
      - run: npx @cyclonedx/cyclonedx-npm --output-file sbom.json
";
        let mut packages = FxHashSet::default();
        extract_ci_packages(content, Path::new("/nonexistent"), &mut packages);
        assert!(packages.contains("@cyclonedx/cyclonedx-npm"));
    }

    // ── helper tests ───────────────────────────────────────────────

    #[test]
    fn strip_yaml_key_basic() {
        assert_eq!(strip_yaml_key("run: npm test", "run"), Some(" npm test"));
        assert_eq!(strip_yaml_key("run:", "run"), Some(""));
        assert_eq!(strip_yaml_key("other: value", "run"), None);
    }

    #[test]
    fn is_yaml_mapping_basic() {
        assert!(is_yaml_mapping("NODE_ENV: production"));
        assert!(is_yaml_mapping("image: node:18"));
        assert!(!is_yaml_mapping("npm ci"));
        assert!(!is_yaml_mapping("npx eslint src"));
        assert!(!is_yaml_mapping("https://example.com"));
    }
}
