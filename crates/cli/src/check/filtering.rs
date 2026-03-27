use std::process::ExitCode;

use fallow_config::{OutputFormat, discover_workspaces};

use crate::emit_error;

// ── Workspace filtering ──────────────────────────────────────────

/// Scope results to a single workspace package.
///
/// The full cross-workspace graph is still built (so cross-package imports
/// are resolved), but only issues from files under `ws_root` are reported.
pub fn filter_to_workspace(
    results: &mut fallow_core::results::AnalysisResults,
    ws_root: &std::path::Path,
) {
    // File-scoped issues: retain only those under the workspace root
    results.unused_files.retain(|f| f.path.starts_with(ws_root));
    results
        .unused_exports
        .retain(|e| e.path.starts_with(ws_root));
    results.unused_types.retain(|e| e.path.starts_with(ws_root));
    results
        .unused_enum_members
        .retain(|m| m.path.starts_with(ws_root));
    results
        .unused_class_members
        .retain(|m| m.path.starts_with(ws_root));
    results
        .unresolved_imports
        .retain(|i| i.path.starts_with(ws_root));

    // Dependency issues: scope to workspace's own package.json
    let ws_pkg = ws_root.join("package.json");
    results.unused_dependencies.retain(|d| d.path == ws_pkg);
    results.unused_dev_dependencies.retain(|d| d.path == ws_pkg);
    results
        .unused_optional_dependencies
        .retain(|d| d.path == ws_pkg);
    results.type_only_dependencies.retain(|d| d.path == ws_pkg);

    // Unlisted deps: keep only if any importing file is in this workspace
    results
        .unlisted_dependencies
        .retain(|d| d.imported_from.iter().any(|s| s.path.starts_with(ws_root)));

    // Duplicate exports: filter locations to workspace, drop groups with < 2
    for dup in &mut results.duplicate_exports {
        dup.locations.retain(|loc| loc.path.starts_with(ws_root));
    }
    results.duplicate_exports.retain(|d| d.locations.len() >= 2);

    // Circular deps: keep cycles where at least one file is in this workspace
    results
        .circular_dependencies
        .retain(|c| c.files.iter().any(|f| f.starts_with(ws_root)));
}

/// Resolve `--workspace <name>` to a workspace root path, or exit with an error.
pub fn resolve_workspace_filter(
    root: &std::path::Path,
    workspace_name: &str,
    output: &OutputFormat,
) -> Result<std::path::PathBuf, ExitCode> {
    let workspaces = discover_workspaces(root);
    if workspaces.is_empty() {
        let msg = format!(
            "--workspace '{workspace_name}' specified but no workspaces found. \
             Ensure root package.json has a \"workspaces\" field, pnpm-workspace.yaml exists, \
             or tsconfig.json has \"references\"."
        );
        return Err(emit_error(&msg, 2, output));
    }

    workspaces
        .iter()
        .find(|ws| ws.name == workspace_name)
        .map_or_else(
            || {
                let names: Vec<&str> = workspaces.iter().map(|ws| ws.name.as_str()).collect();
                let msg = format!(
                    "workspace '{workspace_name}' not found. Available workspaces: {}",
                    names.join(", ")
                );
                Err(emit_error(&msg, 2, output))
            },
            |ws| Ok(ws.root.clone()),
        )
}

// ── Changed-file filtering ───────────────────────────────────────

/// Filter results to only include issues in `changed_files`.
///
/// Dependency-level issues (unused deps, dev deps, optional deps, type-only deps) are
/// intentionally NOT filtered here. Unlike file-level issues, a dependency being "unused"
/// is a function of the entire import graph and can't be attributed to individual changed
/// source files. Compare with `filter_to_workspace`, which DOES filter dependencies by
/// their owning package.json — a different, well-defined scope.
pub(super) fn filter_changed_files(
    results: &mut fallow_core::results::AnalysisResults,
    changed_files: &rustc_hash::FxHashSet<std::path::PathBuf>,
) {
    results
        .unused_files
        .retain(|f| changed_files.contains(&f.path));
    results
        .unused_exports
        .retain(|e| changed_files.contains(&e.path));
    results
        .unused_types
        .retain(|e| changed_files.contains(&e.path));
    results
        .unused_enum_members
        .retain(|m| changed_files.contains(&m.path));
    results
        .unused_class_members
        .retain(|m| changed_files.contains(&m.path));
    results
        .unresolved_imports
        .retain(|i| changed_files.contains(&i.path));

    // Unlisted deps: keep only if any importing file is changed
    results.unlisted_dependencies.retain(|d| {
        d.imported_from
            .iter()
            .any(|s| changed_files.contains(&s.path))
    });

    // Duplicate exports: filter locations to changed files, drop groups with < 2
    for dup in &mut results.duplicate_exports {
        dup.locations
            .retain(|loc| changed_files.contains(&loc.path));
    }
    results.duplicate_exports.retain(|d| d.locations.len() >= 2);

    // Circular deps: keep cycles where at least one file is changed
    results
        .circular_dependencies
        .retain(|c| c.files.iter().any(|f| changed_files.contains(f)));
}

// ── Changed files ────────────────────────────────────────────────

/// Get files changed since a git ref.
pub fn get_changed_files(
    root: &std::path::Path,
    git_ref: &str,
) -> Option<rustc_hash::FxHashSet<std::path::PathBuf>> {
    let output = match std::process::Command::new("git")
        .args(["diff", "--name-only", &format!("{git_ref}...HEAD")])
        .current_dir(root)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            // git binary not found or not executable — could be a non-git project
            eprintln!("Warning: --changed-since ignored: failed to run git: {e}");
            return None;
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not a git repository") {
            // Not a git repo — silently skip the filter (could be OK)
            eprintln!("Warning: --changed-since ignored: not a git repository");
        } else {
            // Likely a bad ref — warn the user
            eprintln!(
                "Warning: --changed-since failed for ref '{}': {}",
                git_ref,
                stderr.trim()
            );
        }
        return None;
    }

    let files: rustc_hash::FxHashSet<std::path::PathBuf> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| root.join(line))
        .collect();

    Some(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fallow_core::extract::MemberKind;
    use fallow_core::results::*;
    use std::path::PathBuf;

    #[test]
    fn filter_to_workspace_keeps_files_under_ws_root() {
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: PathBuf::from("/project/packages/ui/src/button.ts"),
        });
        results.unused_files.push(UnusedFile {
            path: PathBuf::from("/project/packages/api/src/handler.ts"),
        });

        let ws_root = PathBuf::from("/project/packages/ui");
        filter_to_workspace(&mut results, &ws_root);

        assert_eq!(results.unused_files.len(), 1);
        assert_eq!(
            results.unused_files[0].path,
            PathBuf::from("/project/packages/ui/src/button.ts")
        );
    }

    #[test]
    fn filter_to_workspace_scopes_dependencies_to_ws_package_json() {
        let mut results = AnalysisResults::default();
        results.unused_dependencies.push(UnusedDependency {
            package_name: "lodash".into(),
            location: DependencyLocation::Dependencies,
            path: PathBuf::from("/project/package.json"),
            line: 5,
        });
        results.unused_dependencies.push(UnusedDependency {
            package_name: "react".into(),
            location: DependencyLocation::Dependencies,
            path: PathBuf::from("/project/packages/ui/package.json"),
            line: 5,
        });
        results.unused_dev_dependencies.push(UnusedDependency {
            package_name: "vitest".into(),
            location: DependencyLocation::DevDependencies,
            path: PathBuf::from("/project/packages/ui/package.json"),
            line: 5,
        });

        let ws_root = PathBuf::from("/project/packages/ui");
        filter_to_workspace(&mut results, &ws_root);

        assert_eq!(results.unused_dependencies.len(), 1);
        assert_eq!(results.unused_dependencies[0].package_name, "react");
        assert_eq!(results.unused_dev_dependencies.len(), 1);
        assert_eq!(results.unused_dev_dependencies[0].package_name, "vitest");
    }

    #[test]
    fn filter_to_workspace_scopes_unlisted_deps_by_importer() {
        let mut results = AnalysisResults::default();
        results.unlisted_dependencies.push(UnlistedDependency {
            package_name: "chalk".into(),
            imported_from: vec![ImportSite {
                path: PathBuf::from("/project/packages/ui/src/a.ts"),
                line: 1,
                col: 0,
            }],
        });
        results.unlisted_dependencies.push(UnlistedDependency {
            package_name: "debug".into(),
            imported_from: vec![ImportSite {
                path: PathBuf::from("/project/packages/api/src/b.ts"),
                line: 1,
                col: 0,
            }],
        });

        let ws_root = PathBuf::from("/project/packages/ui");
        filter_to_workspace(&mut results, &ws_root);

        assert_eq!(results.unlisted_dependencies.len(), 1);
        assert_eq!(results.unlisted_dependencies[0].package_name, "chalk");
    }

    #[test]
    fn filter_to_workspace_drops_duplicate_exports_below_two_locations() {
        let mut results = AnalysisResults::default();
        results.duplicate_exports.push(DuplicateExport {
            export_name: "helper".into(),
            locations: vec![
                DuplicateLocation {
                    path: PathBuf::from("/project/packages/ui/src/a.ts"),
                    line: 15,
                    col: 0,
                },
                DuplicateLocation {
                    path: PathBuf::from("/project/packages/api/src/b.ts"),
                    line: 30,
                    col: 0,
                },
            ],
        });
        results.duplicate_exports.push(DuplicateExport {
            export_name: "utils".into(),
            locations: vec![
                DuplicateLocation {
                    path: PathBuf::from("/project/packages/ui/src/c.ts"),
                    line: 15,
                    col: 0,
                },
                DuplicateLocation {
                    path: PathBuf::from("/project/packages/ui/src/d.ts"),
                    line: 30,
                    col: 0,
                },
            ],
        });

        let ws_root = PathBuf::from("/project/packages/ui");
        filter_to_workspace(&mut results, &ws_root);

        // "helper" had only 1 location in workspace — dropped
        // "utils" had 2 locations in workspace — kept
        assert_eq!(results.duplicate_exports.len(), 1);
        assert_eq!(results.duplicate_exports[0].export_name, "utils");
    }

    #[test]
    fn filter_to_workspace_scopes_exports_and_types() {
        let mut results = AnalysisResults::default();
        results.unused_exports.push(UnusedExport {
            path: PathBuf::from("/project/packages/ui/src/a.ts"),
            export_name: "A".into(),
            is_type_only: false,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        results.unused_exports.push(UnusedExport {
            path: PathBuf::from("/project/packages/api/src/b.ts"),
            export_name: "B".into(),
            is_type_only: false,
            line: 2,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        results.unused_types.push(UnusedExport {
            path: PathBuf::from("/project/packages/ui/src/types.ts"),
            export_name: "T".into(),
            is_type_only: true,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });

        let ws_root = PathBuf::from("/project/packages/ui");
        filter_to_workspace(&mut results, &ws_root);

        assert_eq!(results.unused_exports.len(), 1);
        assert_eq!(results.unused_exports[0].export_name, "A");
        assert_eq!(results.unused_types.len(), 1);
        assert_eq!(results.unused_types[0].export_name, "T");
    }

    #[test]
    fn filter_to_workspace_scopes_type_only_dependencies() {
        let mut results = AnalysisResults::default();
        results.type_only_dependencies.push(TypeOnlyDependency {
            package_name: "zod".into(),
            path: PathBuf::from("/project/packages/ui/package.json"),
            line: 8,
        });
        results.type_only_dependencies.push(TypeOnlyDependency {
            package_name: "yup".into(),
            path: PathBuf::from("/project/package.json"),
            line: 8,
        });

        let ws_root = PathBuf::from("/project/packages/ui");
        filter_to_workspace(&mut results, &ws_root);

        assert_eq!(results.type_only_dependencies.len(), 1);
        assert_eq!(results.type_only_dependencies[0].package_name, "zod");
    }

    #[test]
    fn filter_to_workspace_scopes_enum_and_class_members() {
        let mut results = AnalysisResults::default();
        results.unused_enum_members.push(UnusedMember {
            path: PathBuf::from("/project/packages/ui/src/enums.ts"),
            parent_name: "Color".into(),
            member_name: "Red".into(),
            kind: MemberKind::EnumMember,
            line: 2,
            col: 0,
        });
        results.unused_enum_members.push(UnusedMember {
            path: PathBuf::from("/project/packages/api/src/enums.ts"),
            parent_name: "Status".into(),
            member_name: "Active".into(),
            kind: MemberKind::EnumMember,
            line: 3,
            col: 0,
        });
        results.unused_class_members.push(UnusedMember {
            path: PathBuf::from("/project/packages/ui/src/service.ts"),
            parent_name: "Svc".into(),
            member_name: "init".into(),
            kind: MemberKind::ClassMethod,
            line: 5,
            col: 0,
        });

        let ws_root = PathBuf::from("/project/packages/ui");
        filter_to_workspace(&mut results, &ws_root);

        assert_eq!(results.unused_enum_members.len(), 1);
        assert_eq!(results.unused_enum_members[0].member_name, "Red");
        assert_eq!(results.unused_class_members.len(), 1);
        assert_eq!(results.unused_class_members[0].member_name, "init");
    }

    // ── filter_changed_files ────────────────────────────────────────

    #[test]
    fn filter_changed_files_keeps_only_changed() {
        let mut results = AnalysisResults::default();
        results.unused_files.push(UnusedFile {
            path: PathBuf::from("/project/src/a.ts"),
        });
        results.unused_files.push(UnusedFile {
            path: PathBuf::from("/project/src/b.ts"),
        });

        let mut changed = rustc_hash::FxHashSet::default();
        changed.insert(PathBuf::from("/project/src/a.ts"));

        filter_changed_files(&mut results, &changed);

        assert_eq!(results.unused_files.len(), 1);
        assert_eq!(
            results.unused_files[0].path,
            PathBuf::from("/project/src/a.ts")
        );
    }

    #[test]
    fn filter_changed_files_preserves_unused_deps() {
        let mut results = AnalysisResults::default();
        results.unused_dependencies.push(UnusedDependency {
            package_name: "lodash".into(),
            location: DependencyLocation::Dependencies,
            path: PathBuf::from("/project/package.json"),
            line: 5,
        });
        results.unused_dev_dependencies.push(UnusedDependency {
            package_name: "jest".into(),
            location: DependencyLocation::DevDependencies,
            path: PathBuf::from("/project/package.json"),
            line: 10,
        });

        let changed = rustc_hash::FxHashSet::default(); // empty set

        filter_changed_files(&mut results, &changed);

        // Dependency-level issues are NOT filtered by changed files
        assert_eq!(results.unused_dependencies.len(), 1);
        assert_eq!(results.unused_dev_dependencies.len(), 1);
    }

    #[test]
    fn filter_changed_files_filters_exports_by_path() {
        let mut results = AnalysisResults::default();
        results.unused_exports.push(UnusedExport {
            path: PathBuf::from("/project/src/a.ts"),
            export_name: "foo".into(),
            is_type_only: false,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });
        results.unused_exports.push(UnusedExport {
            path: PathBuf::from("/project/src/b.ts"),
            export_name: "bar".into(),
            is_type_only: false,
            line: 1,
            col: 0,
            span_start: 0,
            is_re_export: false,
        });

        let mut changed = rustc_hash::FxHashSet::default();
        changed.insert(PathBuf::from("/project/src/b.ts"));

        filter_changed_files(&mut results, &changed);

        assert_eq!(results.unused_exports.len(), 1);
        assert_eq!(results.unused_exports[0].export_name, "bar");
    }

    #[test]
    fn filter_changed_files_drops_duplicate_exports_below_two() {
        let mut results = AnalysisResults::default();
        results.duplicate_exports.push(DuplicateExport {
            export_name: "helper".into(),
            locations: vec![
                DuplicateLocation {
                    path: PathBuf::from("/project/src/a.ts"),
                    line: 1,
                    col: 0,
                },
                DuplicateLocation {
                    path: PathBuf::from("/project/src/b.ts"),
                    line: 2,
                    col: 0,
                },
            ],
        });

        let mut changed = rustc_hash::FxHashSet::default();
        changed.insert(PathBuf::from("/project/src/a.ts"));

        filter_changed_files(&mut results, &changed);

        // Only one location is in changed files -> group dropped
        assert!(results.duplicate_exports.is_empty());
    }

    #[test]
    fn filter_changed_files_keeps_circular_deps_if_any_file_changed() {
        let mut results = AnalysisResults::default();
        results.circular_dependencies.push(CircularDependency {
            files: vec![
                PathBuf::from("/project/src/a.ts"),
                PathBuf::from("/project/src/b.ts"),
            ],
            length: 2,
            line: 1,
            col: 0,
        });

        let mut changed = rustc_hash::FxHashSet::default();
        changed.insert(PathBuf::from("/project/src/b.ts"));

        filter_changed_files(&mut results, &changed);

        assert_eq!(results.circular_dependencies.len(), 1);
    }

    #[test]
    fn filter_changed_files_removes_circular_deps_if_no_file_changed() {
        let mut results = AnalysisResults::default();
        results.circular_dependencies.push(CircularDependency {
            files: vec![
                PathBuf::from("/project/src/a.ts"),
                PathBuf::from("/project/src/b.ts"),
            ],
            length: 2,
            line: 1,
            col: 0,
        });

        let mut changed = rustc_hash::FxHashSet::default();
        changed.insert(PathBuf::from("/project/src/c.ts"));

        filter_changed_files(&mut results, &changed);

        assert!(results.circular_dependencies.is_empty());
    }

    #[test]
    fn filter_changed_files_keeps_unlisted_dep_if_importer_changed() {
        let mut results = AnalysisResults::default();
        results.unlisted_dependencies.push(UnlistedDependency {
            package_name: "chalk".into(),
            imported_from: vec![ImportSite {
                path: PathBuf::from("/project/src/a.ts"),
                line: 1,
                col: 0,
            }],
        });

        let mut changed = rustc_hash::FxHashSet::default();
        changed.insert(PathBuf::from("/project/src/a.ts"));

        filter_changed_files(&mut results, &changed);

        assert_eq!(results.unlisted_dependencies.len(), 1);
    }

    #[test]
    fn filter_changed_files_removes_unlisted_dep_if_no_importer_changed() {
        let mut results = AnalysisResults::default();
        results.unlisted_dependencies.push(UnlistedDependency {
            package_name: "chalk".into(),
            imported_from: vec![ImportSite {
                path: PathBuf::from("/project/src/a.ts"),
                line: 1,
                col: 0,
            }],
        });

        let mut changed = rustc_hash::FxHashSet::default();
        changed.insert(PathBuf::from("/project/src/b.ts"));

        filter_changed_files(&mut results, &changed);

        assert!(results.unlisted_dependencies.is_empty());
    }
}
