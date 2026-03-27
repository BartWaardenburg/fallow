mod package_json;
mod parsers;

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use package_json::PackageJson;
use parsers::{expand_workspace_glob, parse_pnpm_workspace_yaml, parse_tsconfig_references};

/// Workspace configuration for monorepo support.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct WorkspaceConfig {
    /// Additional workspace patterns (beyond what's in root package.json).
    #[serde(default)]
    pub patterns: Vec<String>,
}

/// Discovered workspace info from package.json, pnpm-workspace.yaml, or tsconfig.json references.
#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    /// Workspace root path.
    pub root: PathBuf,
    /// Package name from package.json.
    pub name: String,
    /// Whether this workspace is depended on by other workspaces.
    pub is_internal_dependency: bool,
}

/// Discover all workspace packages in a monorepo.
///
/// Sources (additive, deduplicated by canonical path):
/// 1. `package.json` `workspaces` field
/// 2. `pnpm-workspace.yaml` `packages` field
/// 3. `tsconfig.json` `references` field (TypeScript project references)
#[must_use]
pub fn discover_workspaces(root: &Path) -> Vec<WorkspaceInfo> {
    let patterns = collect_workspace_patterns(root);
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    let mut workspaces = expand_patterns_to_workspaces(root, &patterns, &canonical_root);
    workspaces.extend(collect_tsconfig_workspaces(root, &canonical_root));

    if workspaces.is_empty() {
        return Vec::new();
    }

    mark_internal_dependencies(&mut workspaces);
    workspaces.into_iter().map(|(ws, _)| ws).collect()
}

/// Collect glob patterns from `package.json` `workspaces` field and `pnpm-workspace.yaml`.
fn collect_workspace_patterns(root: &Path) -> Vec<String> {
    let mut patterns = Vec::new();

    // Check root package.json for workspace patterns
    let pkg_path = root.join("package.json");
    if let Ok(pkg) = PackageJson::load(&pkg_path) {
        patterns.extend(pkg.workspace_patterns());
    }

    // Check pnpm-workspace.yaml
    let pnpm_workspace = root.join("pnpm-workspace.yaml");
    if pnpm_workspace.exists()
        && let Ok(content) = std::fs::read_to_string(&pnpm_workspace)
    {
        patterns.extend(parse_pnpm_workspace_yaml(&content));
    }

    patterns
}

/// Expand workspace glob patterns to discover workspace directories.
///
/// Handles positive/negated pattern splitting, glob matching, and package.json
/// loading for each matched directory.
fn expand_patterns_to_workspaces(
    root: &Path,
    patterns: &[String],
    canonical_root: &Path,
) -> Vec<(WorkspaceInfo, Vec<String>)> {
    if patterns.is_empty() {
        return Vec::new();
    }

    let mut workspaces = Vec::new();

    // Separate positive and negated patterns.
    // Negated patterns (e.g., `!**/test/**`) are used as exclusion filters —
    // the `glob` crate does not support `!` prefixed patterns natively.
    let (positive, negative): (Vec<&String>, Vec<&String>) =
        patterns.iter().partition(|p| !p.starts_with('!'));
    let negation_matchers: Vec<globset::GlobMatcher> = negative
        .iter()
        .filter_map(|p| {
            let stripped = p.strip_prefix('!').unwrap_or(p);
            globset::Glob::new(stripped)
                .ok()
                .map(|g| g.compile_matcher())
        })
        .collect();

    for pattern in &positive {
        // Normalize the pattern for directory matching:
        // - `packages/*` → glob for `packages/*` (find all subdirs)
        // - `packages/` → glob for `packages/*` (trailing slash means "contents of")
        // - `apps`       → glob for `apps` (exact directory)
        let glob_pattern = if pattern.ends_with('/') {
            format!("{pattern}*")
        } else if !pattern.contains('*') && !pattern.contains('?') && !pattern.contains('{') {
            // Bare directory name — treat as exact match
            (*pattern).clone()
        } else {
            (*pattern).clone()
        };

        // Walk directories matching the glob.
        // expand_workspace_glob already filters to dirs with package.json
        // and returns (original_path, canonical_path) — no redundant canonicalize().
        let matched_dirs = expand_workspace_glob(root, &glob_pattern, canonical_root);
        for (dir, canonical_dir) in matched_dirs {
            // Skip workspace entries that point to the project root itself
            // (e.g. pnpm-workspace.yaml listing `.` as a workspace)
            if canonical_dir == *canonical_root {
                continue;
            }

            // Check against negation patterns — skip directories that match any negated pattern
            let relative = dir.strip_prefix(root).unwrap_or(&dir);
            let relative_str = relative.to_string_lossy();
            if negation_matchers
                .iter()
                .any(|m| m.is_match(relative_str.as_ref()))
            {
                continue;
            }

            // package.json existence already checked in expand_workspace_glob
            let ws_pkg_path = dir.join("package.json");
            if let Ok(pkg) = PackageJson::load(&ws_pkg_path) {
                // Collect dependency names during initial load to avoid
                // re-reading package.json later.
                let dep_names = pkg.all_dependency_names();
                let name = pkg.name.unwrap_or_else(|| {
                    dir.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                });
                workspaces.push((
                    WorkspaceInfo {
                        root: dir,
                        name,
                        is_internal_dependency: false,
                    },
                    dep_names,
                ));
            }
        }
    }

    workspaces
}

/// Discover workspaces from TypeScript project references in `tsconfig.json`.
///
/// Referenced directories are added as workspaces, supplementing npm/pnpm workspaces.
/// This enables cross-workspace resolution for TypeScript composite projects.
fn collect_tsconfig_workspaces(
    root: &Path,
    canonical_root: &Path,
) -> Vec<(WorkspaceInfo, Vec<String>)> {
    let mut workspaces = Vec::new();

    for dir in parse_tsconfig_references(root) {
        let canonical_dir = dir.canonicalize().unwrap_or_else(|_| dir.clone());
        // Security: skip references pointing to project root or outside it
        if canonical_dir == *canonical_root || !canonical_dir.starts_with(canonical_root) {
            continue;
        }

        // Read package.json if available; otherwise use directory name
        let ws_pkg_path = dir.join("package.json");
        let (name, dep_names) = if ws_pkg_path.exists() {
            if let Ok(pkg) = PackageJson::load(&ws_pkg_path) {
                let deps = pkg.all_dependency_names();
                let n = pkg.name.unwrap_or_else(|| dir_name(&dir));
                (n, deps)
            } else {
                (dir_name(&dir), Vec::new())
            }
        } else {
            // No package.json — use directory name, no deps.
            // Valid for TypeScript-only composite projects.
            (dir_name(&dir), Vec::new())
        };

        workspaces.push((
            WorkspaceInfo {
                root: dir,
                name,
                is_internal_dependency: false,
            },
            dep_names,
        ));
    }

    workspaces
}

/// Deduplicate workspaces by canonical path and mark internal dependencies.
///
/// Overlapping sources (npm workspaces + tsconfig references pointing to the same
/// directory) are collapsed. npm-discovered entries take precedence (they appear first).
/// Workspaces depended on by other workspaces are marked as `is_internal_dependency`.
fn mark_internal_dependencies(workspaces: &mut Vec<(WorkspaceInfo, Vec<String>)>) {
    // Deduplicate by canonical path
    {
        let mut seen = rustc_hash::FxHashSet::default();
        workspaces.retain(|(ws, _)| {
            let canonical = ws.root.canonicalize().unwrap_or_else(|_| ws.root.clone());
            seen.insert(canonical)
        });
    }

    // Mark workspaces that are depended on by other workspaces.
    // Uses dep names collected during initial package.json load
    // to avoid re-reading all workspace package.json files.
    let all_dep_names: rustc_hash::FxHashSet<String> = workspaces
        .iter()
        .flat_map(|(_, deps)| deps.iter().cloned())
        .collect();
    for (ws, _) in &mut *workspaces {
        ws.is_internal_dependency = all_dep_names.contains(&ws.name);
    }
}

/// Extract the directory name as a string, for workspace name fallback.
fn dir_name(dir: &Path) -> String {
    dir.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_workspaces_from_tsconfig_references() {
        let temp_dir = std::env::temp_dir().join("fallow-test-ws-tsconfig-refs");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(temp_dir.join("packages/core")).unwrap();
        std::fs::create_dir_all(temp_dir.join("packages/ui")).unwrap();

        // No package.json workspaces — only tsconfig references
        std::fs::write(
            temp_dir.join("tsconfig.json"),
            r#"{"references": [{"path": "./packages/core"}, {"path": "./packages/ui"}]}"#,
        )
        .unwrap();

        // core has package.json with a name
        std::fs::write(
            temp_dir.join("packages/core/package.json"),
            r#"{"name": "@project/core"}"#,
        )
        .unwrap();

        // ui has NO package.json — name should fall back to directory name
        let workspaces = discover_workspaces(&temp_dir);
        assert_eq!(workspaces.len(), 2);
        assert!(workspaces.iter().any(|ws| ws.name == "@project/core"));
        assert!(workspaces.iter().any(|ws| ws.name == "ui"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn tsconfig_references_outside_root_rejected() {
        let temp_dir = std::env::temp_dir().join("fallow-test-tsconfig-outside");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(temp_dir.join("project/packages/core")).unwrap();
        // "outside" is a sibling of "project", not inside it
        std::fs::create_dir_all(temp_dir.join("outside")).unwrap();

        std::fs::write(
            temp_dir.join("project/tsconfig.json"),
            r#"{"references": [{"path": "./packages/core"}, {"path": "../outside"}]}"#,
        )
        .unwrap();

        // Security: "../outside" points outside the project root and should be rejected
        let workspaces = discover_workspaces(&temp_dir.join("project"));
        assert_eq!(
            workspaces.len(),
            1,
            "reference outside project root should be rejected: {workspaces:?}"
        );
        assert!(
            workspaces[0]
                .root
                .to_string_lossy()
                .contains("packages/core")
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
