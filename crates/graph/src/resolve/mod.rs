//! Import specifier resolution using `oxc_resolver`.
//!
//! Resolves all import specifiers across all modules in parallel, mapping each to
//! an internal file, npm package, or unresolvable target. Includes support for
//! tsconfig path aliases, pnpm virtual store paths, React Native platform extensions,
//! and dynamic import pattern matching via glob.

pub(crate) mod fallbacks;
mod path_info;
mod react_native;
mod specifier;
mod types;

pub use path_info::{extract_package_name, is_bare_specifier, is_path_alias};
pub use types::{ResolveResult, ResolvedImport, ResolvedModule, ResolvedReExport};

use std::path::{Path, PathBuf};

use rayon::prelude::*;
use rustc_hash::FxHashMap;

use oxc_span::Span;

use fallow_types::discover::{DiscoveredFile, FileId};
use fallow_types::extract::{
    DynamicImportInfo, DynamicImportPattern, ImportInfo, ImportedName, ModuleInfo, ReExportInfo,
    RequireCallInfo,
};

use fallbacks::make_glob_from_pattern;
use specifier::{create_resolver, resolve_specifier};
use types::ResolveContext;

/// Resolve all imports across all modules in parallel.
pub fn resolve_all_imports(
    modules: &[ModuleInfo],
    files: &[DiscoveredFile],
    workspaces: &[fallow_config::WorkspaceInfo],
    active_plugins: &[String],
    path_aliases: &[(String, String)],
    root: &Path,
) -> Vec<ResolvedModule> {
    // Build workspace name → root index for pnpm store fallback.
    // Canonicalize roots to match path_to_id (which uses canonical paths).
    // Without this, macOS /var → /private/var and similar platform symlinks
    // cause workspace roots to mismatch canonical file paths.
    let canonical_ws_roots: Vec<PathBuf> = workspaces
        .par_iter()
        .map(|ws| ws.root.canonicalize().unwrap_or_else(|_| ws.root.clone()))
        .collect();
    let workspace_roots: FxHashMap<&str, &Path> = workspaces
        .iter()
        .zip(canonical_ws_roots.iter())
        .map(|(ws, canonical)| (ws.name.as_str(), canonical.as_path()))
        .collect();

    // Check if project root is already canonical (no symlinks in path).
    // When true, raw paths == canonical paths for files under root, so we can skip
    // the upfront bulk canonicalize() of all source files (21k+ syscalls on large projects).
    // A lazy CanonicalFallback handles the rare intra-project symlink case.
    let root_is_canonical = root.canonicalize().is_ok_and(|c| c == root);

    // Pre-compute canonical paths ONCE for all files in parallel (avoiding repeated syscalls).
    // Skipped when root is canonical — the lazy fallback below handles edge cases.
    let canonical_paths: Vec<PathBuf> = if root_is_canonical {
        Vec::new()
    } else {
        files
            .par_iter()
            .map(|f| f.path.canonicalize().unwrap_or_else(|_| f.path.clone()))
            .collect()
    };

    // Primary path → FileId index. When root is canonical, uses raw paths (fast).
    // Otherwise uses pre-computed canonical paths (correct for all symlink configurations).
    let path_to_id: FxHashMap<&Path, FileId> = if root_is_canonical {
        files.iter().map(|f| (f.path.as_path(), f.id)).collect()
    } else {
        canonical_paths
            .iter()
            .enumerate()
            .map(|(idx, canonical)| (canonical.as_path(), files[idx].id))
            .collect()
    };

    // Also index by non-canonical path for fallback lookups
    let raw_path_to_id: FxHashMap<&Path, FileId> =
        files.iter().map(|f| (f.path.as_path(), f.id)).collect();

    // FileIds are sequential 0..n, so direct array indexing is faster than FxHashMap.
    let file_paths: Vec<&Path> = files.iter().map(|f| f.path.as_path()).collect();

    // Create resolver ONCE and share across threads (oxc_resolver::Resolver is Send + Sync)
    let resolver = create_resolver(active_plugins);

    // Lazy canonical fallback — only needed when root is canonical (path_to_id uses raw paths).
    // When root is NOT canonical, path_to_id already uses canonical paths, no fallback needed.
    let canonical_fallback = if root_is_canonical {
        Some(types::CanonicalFallback::new(files))
    } else {
        None
    };

    // Shared resolution context — avoids passing 6 arguments to every resolve_specifier call
    let ctx = ResolveContext {
        resolver: &resolver,
        path_to_id: &path_to_id,
        raw_path_to_id: &raw_path_to_id,
        workspace_roots: &workspace_roots,
        path_aliases,
        root,
        canonical_fallback: canonical_fallback.as_ref(),
    };

    // Resolve in parallel — shared resolver instance.
    // Each file resolves its own imports independently (no shared bare specifier cache).
    // oxc_resolver's internal caches (package.json, tsconfig, directory entries) are
    // shared across threads for performance.
    let mut resolved: Vec<ResolvedModule> = modules
        .par_iter()
        .filter_map(|module| {
            let Some(file_path) = file_paths.get(module.file_id.0 as usize) else {
                tracing::warn!(
                    file_id = module.file_id.0,
                    "Skipping module with unknown file_id during resolution"
                );
                return None;
            };

            let mut all_imports = resolve_static_imports(&ctx, file_path, &module.imports);
            all_imports.extend(resolve_require_imports(
                &ctx,
                file_path,
                &module.require_calls,
            ));

            let from_dir = if canonical_paths.is_empty() {
                // Root is canonical — raw paths are canonical
                file_path.parent().unwrap_or(file_path)
            } else {
                canonical_paths
                    .get(module.file_id.0 as usize)
                    .and_then(|p| p.parent())
                    .unwrap_or(file_path)
            };

            Some(ResolvedModule {
                file_id: module.file_id,
                path: file_path.to_path_buf(),
                exports: module.exports.clone(),
                re_exports: resolve_re_exports(&ctx, file_path, &module.re_exports),
                resolved_imports: all_imports,
                resolved_dynamic_imports: resolve_dynamic_imports(
                    &ctx,
                    file_path,
                    &module.dynamic_imports,
                ),
                resolved_dynamic_patterns: resolve_dynamic_patterns(
                    from_dir,
                    &module.dynamic_import_patterns,
                    &canonical_paths,
                    files,
                ),
                member_accesses: module.member_accesses.clone(),
                whole_object_uses: module.whole_object_uses.clone(),
                has_cjs_exports: module.has_cjs_exports,
                unused_import_bindings: module.unused_import_bindings.clone(),
            })
        })
        .collect();

    apply_specifier_upgrades(&mut resolved);

    resolved
}

/// Resolve standard ES module imports (`import x from './y'`).
fn resolve_static_imports(
    ctx: &ResolveContext,
    file_path: &Path,
    imports: &[ImportInfo],
) -> Vec<ResolvedImport> {
    imports
        .iter()
        .map(|imp| ResolvedImport {
            info: imp.clone(),
            target: resolve_specifier(ctx, file_path, &imp.source),
        })
        .collect()
}

/// Resolve dynamic `import()` calls, expanding destructured names into individual imports.
fn resolve_dynamic_imports(
    ctx: &ResolveContext,
    file_path: &Path,
    dynamic_imports: &[DynamicImportInfo],
) -> Vec<ResolvedImport> {
    dynamic_imports
        .iter()
        .flat_map(|imp| resolve_single_dynamic_import(ctx, file_path, imp))
        .collect()
}

/// Convert a single dynamic import into one or more `ResolvedImport` entries.
fn resolve_single_dynamic_import(
    ctx: &ResolveContext,
    file_path: &Path,
    imp: &DynamicImportInfo,
) -> Vec<ResolvedImport> {
    let target = resolve_specifier(ctx, file_path, &imp.source);

    if !imp.destructured_names.is_empty() {
        // `const { a, b } = await import('./x')` -> Named imports
        return imp
            .destructured_names
            .iter()
            .map(|name| ResolvedImport {
                info: ImportInfo {
                    source: imp.source.clone(),
                    imported_name: ImportedName::Named(name.clone()),
                    local_name: name.clone(),
                    is_type_only: false,
                    span: imp.span,
                    source_span: Span::default(),
                },
                target: target.clone(),
            })
            .collect();
    }

    if imp.local_name.is_some() {
        // `const mod = await import('./x')` -> Namespace with local_name
        return vec![ResolvedImport {
            info: ImportInfo {
                source: imp.source.clone(),
                imported_name: ImportedName::Namespace,
                local_name: imp.local_name.clone().unwrap_or_default(),
                is_type_only: false,
                span: imp.span,
                source_span: Span::default(),
            },
            target,
        }];
    }

    // Side-effect only: `await import('./x')` with no assignment
    vec![ResolvedImport {
        info: ImportInfo {
            source: imp.source.clone(),
            imported_name: ImportedName::SideEffect,
            local_name: String::new(),
            is_type_only: false,
            span: imp.span,
            source_span: Span::default(),
        },
        target,
    }]
}

/// Resolve re-export sources (`export { x } from './y'`).
fn resolve_re_exports(
    ctx: &ResolveContext,
    file_path: &Path,
    re_exports: &[ReExportInfo],
) -> Vec<ResolvedReExport> {
    re_exports
        .iter()
        .map(|re| ResolvedReExport {
            info: re.clone(),
            target: resolve_specifier(ctx, file_path, &re.source),
        })
        .collect()
}

/// Resolve CommonJS `require()` calls.
/// Destructured requires become Named imports; others become Namespace (conservative).
fn resolve_require_imports(
    ctx: &ResolveContext,
    file_path: &Path,
    require_calls: &[RequireCallInfo],
) -> Vec<ResolvedImport> {
    require_calls
        .iter()
        .flat_map(|req| resolve_single_require(ctx, file_path, req))
        .collect()
}

/// Convert a single `require()` call into one or more `ResolvedImport` entries.
fn resolve_single_require(
    ctx: &ResolveContext,
    file_path: &Path,
    req: &RequireCallInfo,
) -> Vec<ResolvedImport> {
    let target = resolve_specifier(ctx, file_path, &req.source);

    if req.destructured_names.is_empty() {
        return vec![ResolvedImport {
            info: ImportInfo {
                source: req.source.clone(),
                imported_name: ImportedName::Namespace,
                local_name: req.local_name.clone().unwrap_or_default(),
                is_type_only: false,
                span: req.span,
                source_span: Span::default(),
            },
            target,
        }];
    }

    req.destructured_names
        .iter()
        .map(|name| ResolvedImport {
            info: ImportInfo {
                source: req.source.clone(),
                imported_name: ImportedName::Named(name.clone()),
                local_name: name.clone(),
                is_type_only: false,
                span: req.span,
                source_span: Span::default(),
            },
            target: target.clone(),
        })
        .collect()
}

/// Resolve dynamic import patterns via glob matching against discovered files.
/// When canonical paths are available, uses those for matching. Otherwise falls
/// back to raw file paths from `files` (avoids allocating a separate PathBuf vec).
fn resolve_dynamic_patterns(
    from_dir: &Path,
    patterns: &[DynamicImportPattern],
    canonical_paths: &[PathBuf],
    files: &[DiscoveredFile],
) -> Vec<(DynamicImportPattern, Vec<FileId>)> {
    patterns
        .iter()
        .filter_map(|pattern| {
            let glob_str = make_glob_from_pattern(pattern);
            let matcher = globset::Glob::new(&glob_str)
                .ok()
                .map(|g| g.compile_matcher())?;
            let matched: Vec<FileId> = if canonical_paths.is_empty() {
                // Root is canonical — use raw file paths directly (no extra allocation)
                files
                    .iter()
                    .filter(|f| {
                        f.path.strip_prefix(from_dir).is_ok_and(|relative| {
                            let rel_str = format!("./{}", relative.to_string_lossy());
                            matcher.is_match(&rel_str)
                        })
                    })
                    .map(|f| f.id)
                    .collect()
            } else {
                canonical_paths
                    .iter()
                    .enumerate()
                    .filter(|(_idx, canonical)| {
                        canonical.strip_prefix(from_dir).is_ok_and(|relative| {
                            let rel_str = format!("./{}", relative.to_string_lossy());
                            matcher.is_match(&rel_str)
                        })
                    })
                    .map(|(idx, _)| files[idx].id)
                    .collect()
            };
            if matched.is_empty() {
                None
            } else {
                Some((pattern.clone(), matched))
            }
        })
        .collect()
}

/// Post-resolution pass: deterministic specifier upgrade.
///
/// With `TsconfigDiscovery::Auto`, the same bare specifier (e.g., `preact/hooks`)
/// may resolve to `InternalModule` from files under a tsconfig with path aliases
/// but `NpmPackage` from files without such aliases. The parallel resolution cache
/// makes the per-file result depend on which thread resolved first (non-deterministic).
///
/// Scans all resolved imports/re-exports to find bare specifiers where ANY file resolved
/// to `InternalModule`. For those specifiers, upgrades all `NpmPackage` results to
/// `InternalModule`. This is correct because if any tsconfig context maps a specifier to
/// a project source file, that source file IS the origin of the package.
///
/// Note: if two tsconfigs map the same specifier to different `FileId`s, the first one
/// encountered (by module order = `FileId` order) wins. This is deterministic but may be
/// imprecise for that edge case — both files get connected regardless.
fn apply_specifier_upgrades(resolved: &mut [ResolvedModule]) {
    let mut specifier_upgrades: FxHashMap<String, FileId> = FxHashMap::default();
    for module in resolved.iter() {
        for imp in module
            .resolved_imports
            .iter()
            .chain(module.resolved_dynamic_imports.iter())
        {
            if is_bare_specifier(&imp.info.source)
                && let ResolveResult::InternalModule(file_id) = &imp.target
            {
                specifier_upgrades
                    .entry(imp.info.source.clone())
                    .or_insert(*file_id);
            }
        }
        for re in &module.re_exports {
            if is_bare_specifier(&re.info.source)
                && let ResolveResult::InternalModule(file_id) = &re.target
            {
                specifier_upgrades
                    .entry(re.info.source.clone())
                    .or_insert(*file_id);
            }
        }
    }

    if specifier_upgrades.is_empty() {
        return;
    }

    // Apply upgrades: replace NpmPackage with InternalModule for matched specifiers
    for module in resolved.iter_mut() {
        for imp in module
            .resolved_imports
            .iter_mut()
            .chain(module.resolved_dynamic_imports.iter_mut())
        {
            if matches!(imp.target, ResolveResult::NpmPackage(_))
                && let Some(&file_id) = specifier_upgrades.get(&imp.info.source)
            {
                imp.target = ResolveResult::InternalModule(file_id);
            }
        }
        for re in &mut module.re_exports {
            if matches!(re.target, ResolveResult::NpmPackage(_))
                && let Some(&file_id) = specifier_upgrades.get(&re.info.source)
            {
                re.target = ResolveResult::InternalModule(file_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_span::Span;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn dummy_span() -> Span {
        Span::new(0, 0)
    }

    /// Build a minimal `ResolveContext` backed by a real resolver but with
    /// empty lookup tables. Every specifier resolves to `NpmPackage` or
    /// `Unresolvable`, which is fine — the tests focus on how helper functions
    /// *transform* inputs into `ResolvedImport` / `ResolvedReExport` structs.
    ///
    /// Under Miri this is a no-op: `oxc_resolver` uses the `statx` syscall
    /// (via `rustix`) which Miri does not support.
    #[cfg(not(miri))]
    fn with_empty_ctx<F: FnOnce(&ResolveContext)>(f: F) {
        let resolver = specifier::create_resolver(&[]);
        let path_to_id = FxHashMap::default();
        let raw_path_to_id = FxHashMap::default();
        let workspace_roots = FxHashMap::default();
        let root = PathBuf::from("/project");
        let ctx = ResolveContext {
            resolver: &resolver,
            path_to_id: &path_to_id,
            raw_path_to_id: &raw_path_to_id,
            workspace_roots: &workspace_roots,
            path_aliases: &[],
            root: &root,
            canonical_fallback: None,
        };
        f(&ctx);
    }

    #[cfg(miri)]
    fn with_empty_ctx<F: FnOnce(&ResolveContext)>(_f: F) {
        // oxc_resolver uses statx syscall unsupported by Miri — skip.
    }

    fn make_import(source: &str, imported: ImportedName, local: &str) -> ImportInfo {
        ImportInfo {
            source: source.to_string(),
            imported_name: imported,
            local_name: local.to_string(),
            is_type_only: false,
            span: dummy_span(),
            source_span: Span::default(),
        }
    }

    fn make_re_export(source: &str, imported: &str, exported: &str) -> ReExportInfo {
        ReExportInfo {
            source: source.to_string(),
            imported_name: imported.to_string(),
            exported_name: exported.to_string(),
            is_type_only: false,
        }
    }

    fn make_dynamic(
        source: &str,
        destructured: Vec<&str>,
        local_name: Option<&str>,
    ) -> DynamicImportInfo {
        DynamicImportInfo {
            source: source.to_string(),
            span: dummy_span(),
            destructured_names: destructured.into_iter().map(String::from).collect(),
            local_name: local_name.map(String::from),
        }
    }

    fn make_require(
        source: &str,
        destructured: Vec<&str>,
        local_name: Option<&str>,
    ) -> RequireCallInfo {
        RequireCallInfo {
            source: source.to_string(),
            span: dummy_span(),
            destructured_names: destructured.into_iter().map(String::from).collect(),
            local_name: local_name.map(String::from),
        }
    }

    /// Build a minimal `ResolvedModule` for `apply_specifier_upgrades` tests.
    fn make_resolved_module(
        file_id: u32,
        imports: Vec<ResolvedImport>,
        dynamic_imports: Vec<ResolvedImport>,
        re_exports: Vec<ResolvedReExport>,
    ) -> ResolvedModule {
        ResolvedModule {
            file_id: FileId(file_id),
            path: PathBuf::from(format!("/project/src/file_{file_id}.ts")),
            exports: vec![],
            re_exports,
            resolved_imports: imports,
            resolved_dynamic_imports: dynamic_imports,
            resolved_dynamic_patterns: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
        }
    }

    fn make_resolved_import(source: &str, target: ResolveResult) -> ResolvedImport {
        ResolvedImport {
            info: make_import(source, ImportedName::Named("x".into()), "x"),
            target,
        }
    }

    fn make_resolved_re_export(source: &str, target: ResolveResult) -> ResolvedReExport {
        ResolvedReExport {
            info: make_re_export(source, "x", "x"),
            target,
        }
    }

    // -----------------------------------------------------------------------
    // resolve_static_imports
    // -----------------------------------------------------------------------

    #[test]
    fn static_imports_named() {
        with_empty_ctx(|ctx| {
            let imports = vec![make_import(
                "react",
                ImportedName::Named("useState".into()),
                "useState",
            )];
            let file = Path::new("/project/src/app.ts");
            let result = resolve_static_imports(ctx, file, &imports);

            assert_eq!(result.len(), 1);
            assert_eq!(result[0].info.source, "react");
            assert!(matches!(
                result[0].info.imported_name,
                ImportedName::Named(ref n) if n == "useState"
            ));
        });
    }

    #[test]
    fn static_imports_default() {
        with_empty_ctx(|ctx| {
            let imports = vec![make_import("react", ImportedName::Default, "React")];
            let file = Path::new("/project/src/app.ts");
            let result = resolve_static_imports(ctx, file, &imports);

            assert_eq!(result.len(), 1);
            assert!(matches!(
                result[0].info.imported_name,
                ImportedName::Default
            ));
            assert_eq!(result[0].info.local_name, "React");
        });
    }

    #[test]
    fn static_imports_namespace() {
        with_empty_ctx(|ctx| {
            let imports = vec![make_import("lodash", ImportedName::Namespace, "_")];
            let file = Path::new("/project/src/utils.ts");
            let result = resolve_static_imports(ctx, file, &imports);

            assert_eq!(result.len(), 1);
            assert!(matches!(
                result[0].info.imported_name,
                ImportedName::Namespace
            ));
            assert_eq!(result[0].info.local_name, "_");
        });
    }

    #[test]
    fn static_imports_side_effect() {
        with_empty_ctx(|ctx| {
            let imports = vec![make_import("./styles.css", ImportedName::SideEffect, "")];
            let file = Path::new("/project/src/app.ts");
            let result = resolve_static_imports(ctx, file, &imports);

            assert_eq!(result.len(), 1);
            assert!(matches!(
                result[0].info.imported_name,
                ImportedName::SideEffect
            ));
            assert_eq!(result[0].info.local_name, "");
        });
    }

    #[test]
    fn static_imports_empty_list() {
        with_empty_ctx(|ctx| {
            let file = Path::new("/project/src/app.ts");
            let result = resolve_static_imports(ctx, file, &[]);
            assert!(result.is_empty());
        });
    }

    #[test]
    fn static_imports_multiple() {
        with_empty_ctx(|ctx| {
            let imports = vec![
                make_import("react", ImportedName::Default, "React"),
                make_import("react", ImportedName::Named("useState".into()), "useState"),
                make_import("lodash", ImportedName::Namespace, "_"),
            ];
            let file = Path::new("/project/src/app.ts");
            let result = resolve_static_imports(ctx, file, &imports);

            assert_eq!(result.len(), 3);
            assert_eq!(result[0].info.source, "react");
            assert_eq!(result[1].info.source, "react");
            assert_eq!(result[2].info.source, "lodash");
        });
    }

    #[test]
    fn static_imports_preserves_type_only() {
        with_empty_ctx(|ctx| {
            let imports = vec![ImportInfo {
                source: "react".into(),
                imported_name: ImportedName::Named("FC".into()),
                local_name: "FC".into(),
                is_type_only: true,
                span: dummy_span(),
                source_span: Span::default(),
            }];
            let file = Path::new("/project/src/app.ts");
            let result = resolve_static_imports(ctx, file, &imports);

            assert_eq!(result.len(), 1);
            assert!(result[0].info.is_type_only);
        });
    }

    // -----------------------------------------------------------------------
    // resolve_single_dynamic_import
    // -----------------------------------------------------------------------

    #[test]
    fn dynamic_import_with_destructured_names() {
        with_empty_ctx(|ctx| {
            let imp = make_dynamic("./utils", vec!["foo", "bar"], None);
            let file = Path::new("/project/src/app.ts");
            let result = resolve_single_dynamic_import(ctx, file, &imp);

            assert_eq!(result.len(), 2);
            assert!(matches!(
                result[0].info.imported_name,
                ImportedName::Named(ref n) if n == "foo"
            ));
            assert_eq!(result[0].info.local_name, "foo");
            assert!(matches!(
                result[1].info.imported_name,
                ImportedName::Named(ref n) if n == "bar"
            ));
            assert_eq!(result[1].info.local_name, "bar");
            // Both should have the same source
            assert_eq!(result[0].info.source, "./utils");
            assert_eq!(result[1].info.source, "./utils");
            // Both should be non-type-only
            assert!(!result[0].info.is_type_only);
            assert!(!result[1].info.is_type_only);
        });
    }

    #[test]
    fn dynamic_import_namespace_with_local_name() {
        with_empty_ctx(|ctx| {
            let imp = make_dynamic("./utils", vec![], Some("utils"));
            let file = Path::new("/project/src/app.ts");
            let result = resolve_single_dynamic_import(ctx, file, &imp);

            assert_eq!(result.len(), 1);
            assert!(matches!(
                result[0].info.imported_name,
                ImportedName::Namespace
            ));
            assert_eq!(result[0].info.local_name, "utils");
        });
    }

    #[test]
    fn dynamic_import_side_effect() {
        with_empty_ctx(|ctx| {
            let imp = make_dynamic("./polyfill", vec![], None);
            let file = Path::new("/project/src/app.ts");
            let result = resolve_single_dynamic_import(ctx, file, &imp);

            assert_eq!(result.len(), 1);
            assert!(matches!(
                result[0].info.imported_name,
                ImportedName::SideEffect
            ));
            assert_eq!(result[0].info.local_name, "");
            assert_eq!(result[0].info.source, "./polyfill");
        });
    }

    #[test]
    fn dynamic_import_destructured_takes_priority_over_local_name() {
        // When both destructured_names and local_name are set,
        // destructured_names wins (checked first).
        with_empty_ctx(|ctx| {
            let imp = DynamicImportInfo {
                source: "./mod".into(),
                span: dummy_span(),
                destructured_names: vec!["a".into()],
                local_name: Some("mod".into()),
            };
            let file = Path::new("/project/src/app.ts");
            let result = resolve_single_dynamic_import(ctx, file, &imp);

            assert_eq!(result.len(), 1);
            assert!(matches!(
                result[0].info.imported_name,
                ImportedName::Named(ref n) if n == "a"
            ));
        });
    }

    // -----------------------------------------------------------------------
    // resolve_dynamic_imports (batch)
    // -----------------------------------------------------------------------

    #[test]
    fn dynamic_imports_flattens_multiple() {
        with_empty_ctx(|ctx| {
            let imports = vec![
                make_dynamic("./a", vec!["x", "y"], None),
                make_dynamic("./b", vec![], Some("b")),
                make_dynamic("./c", vec![], None),
            ];
            let file = Path::new("/project/src/app.ts");
            let result = resolve_dynamic_imports(ctx, file, &imports);

            // ./a -> 2 Named, ./b -> 1 Namespace, ./c -> 1 SideEffect = 4 total
            assert_eq!(result.len(), 4);
        });
    }

    #[test]
    fn dynamic_imports_empty_list() {
        with_empty_ctx(|ctx| {
            let file = Path::new("/project/src/app.ts");
            let result = resolve_dynamic_imports(ctx, file, &[]);
            assert!(result.is_empty());
        });
    }

    // -----------------------------------------------------------------------
    // resolve_re_exports
    // -----------------------------------------------------------------------

    #[test]
    fn re_exports_maps_each_entry() {
        with_empty_ctx(|ctx| {
            let re_exports = vec![
                make_re_export("./utils", "helper", "helper"),
                make_re_export("./types", "*", "*"),
            ];
            let file = Path::new("/project/src/index.ts");
            let result = resolve_re_exports(ctx, file, &re_exports);

            assert_eq!(result.len(), 2);
            assert_eq!(result[0].info.source, "./utils");
            assert_eq!(result[0].info.imported_name, "helper");
            assert_eq!(result[0].info.exported_name, "helper");
            assert_eq!(result[1].info.source, "./types");
            assert_eq!(result[1].info.imported_name, "*");
        });
    }

    #[test]
    fn re_exports_empty_list() {
        with_empty_ctx(|ctx| {
            let file = Path::new("/project/src/index.ts");
            let result = resolve_re_exports(ctx, file, &[]);
            assert!(result.is_empty());
        });
    }

    #[test]
    fn re_exports_preserves_type_only() {
        with_empty_ctx(|ctx| {
            let re_exports = vec![ReExportInfo {
                source: "./types".into(),
                imported_name: "MyType".into(),
                exported_name: "MyType".into(),
                is_type_only: true,
            }];
            let file = Path::new("/project/src/index.ts");
            let result = resolve_re_exports(ctx, file, &re_exports);

            assert_eq!(result.len(), 1);
            assert!(result[0].info.is_type_only);
        });
    }

    // -----------------------------------------------------------------------
    // resolve_single_require
    // -----------------------------------------------------------------------

    #[test]
    fn require_namespace_without_destructuring() {
        with_empty_ctx(|ctx| {
            let req = make_require("fs", vec![], Some("fs"));
            let file = Path::new("/project/src/app.js");
            let result = resolve_single_require(ctx, file, &req);

            assert_eq!(result.len(), 1);
            assert!(matches!(
                result[0].info.imported_name,
                ImportedName::Namespace
            ));
            assert_eq!(result[0].info.local_name, "fs");
            assert_eq!(result[0].info.source, "fs");
        });
    }

    #[test]
    fn require_namespace_without_local_name() {
        with_empty_ctx(|ctx| {
            let req = make_require("./side-effect", vec![], None);
            let file = Path::new("/project/src/app.js");
            let result = resolve_single_require(ctx, file, &req);

            assert_eq!(result.len(), 1);
            assert!(matches!(
                result[0].info.imported_name,
                ImportedName::Namespace
            ));
            // No local name -> empty string from unwrap_or_default
            assert_eq!(result[0].info.local_name, "");
        });
    }

    #[test]
    fn require_with_destructured_names() {
        with_empty_ctx(|ctx| {
            let req = make_require("path", vec!["join", "resolve"], None);
            let file = Path::new("/project/src/app.js");
            let result = resolve_single_require(ctx, file, &req);

            assert_eq!(result.len(), 2);
            assert!(matches!(
                result[0].info.imported_name,
                ImportedName::Named(ref n) if n == "join"
            ));
            assert_eq!(result[0].info.local_name, "join");
            assert!(matches!(
                result[1].info.imported_name,
                ImportedName::Named(ref n) if n == "resolve"
            ));
            assert_eq!(result[1].info.local_name, "resolve");
            // Both share the same source
            assert_eq!(result[0].info.source, "path");
            assert_eq!(result[1].info.source, "path");
        });
    }

    #[test]
    fn require_destructured_is_not_type_only() {
        with_empty_ctx(|ctx| {
            let req = make_require("path", vec!["join"], None);
            let file = Path::new("/project/src/app.js");
            let result = resolve_single_require(ctx, file, &req);

            assert_eq!(result.len(), 1);
            assert!(!result[0].info.is_type_only);
        });
    }

    // -----------------------------------------------------------------------
    // resolve_require_imports (batch)
    // -----------------------------------------------------------------------

    #[test]
    fn require_imports_flattens_multiple() {
        with_empty_ctx(|ctx| {
            let reqs = vec![
                make_require("fs", vec![], Some("fs")),
                make_require("path", vec!["join", "resolve"], None),
            ];
            let file = Path::new("/project/src/app.js");
            let result = resolve_require_imports(ctx, file, &reqs);

            // fs -> 1 Namespace, path -> 2 Named = 3 total
            assert_eq!(result.len(), 3);
        });
    }

    #[test]
    fn require_imports_empty_list() {
        with_empty_ctx(|ctx| {
            let file = Path::new("/project/src/app.js");
            let result = resolve_require_imports(ctx, file, &[]);
            assert!(result.is_empty());
        });
    }

    // -----------------------------------------------------------------------
    // apply_specifier_upgrades
    // -----------------------------------------------------------------------

    #[test]
    fn specifier_upgrades_npm_to_internal() {
        // Module 0 resolves `preact/hooks` to InternalModule(FileId(5))
        // Module 1 resolves `preact/hooks` to NpmPackage("preact")
        // After upgrade, module 1 should also point to InternalModule(FileId(5))
        let mut modules = vec![
            make_resolved_module(
                0,
                vec![make_resolved_import(
                    "preact/hooks",
                    ResolveResult::InternalModule(FileId(5)),
                )],
                vec![],
                vec![],
            ),
            make_resolved_module(
                1,
                vec![make_resolved_import(
                    "preact/hooks",
                    ResolveResult::NpmPackage("preact".into()),
                )],
                vec![],
                vec![],
            ),
        ];

        apply_specifier_upgrades(&mut modules);

        assert!(matches!(
            modules[1].resolved_imports[0].target,
            ResolveResult::InternalModule(FileId(5))
        ));
    }

    #[test]
    fn specifier_upgrades_noop_when_no_internal() {
        // All modules resolve `lodash` to NpmPackage — no upgrade should happen
        let mut modules = vec![
            make_resolved_module(
                0,
                vec![make_resolved_import(
                    "lodash",
                    ResolveResult::NpmPackage("lodash".into()),
                )],
                vec![],
                vec![],
            ),
            make_resolved_module(
                1,
                vec![make_resolved_import(
                    "lodash",
                    ResolveResult::NpmPackage("lodash".into()),
                )],
                vec![],
                vec![],
            ),
        ];

        apply_specifier_upgrades(&mut modules);

        assert!(matches!(
            modules[0].resolved_imports[0].target,
            ResolveResult::NpmPackage(_)
        ));
        assert!(matches!(
            modules[1].resolved_imports[0].target,
            ResolveResult::NpmPackage(_)
        ));
    }

    #[test]
    fn specifier_upgrades_empty_modules() {
        let mut modules: Vec<ResolvedModule> = vec![];
        apply_specifier_upgrades(&mut modules);
        assert!(modules.is_empty());
    }

    #[test]
    fn specifier_upgrades_skips_relative_specifiers() {
        // Relative specifiers (./foo) are NOT bare specifiers, so they should
        // never be candidates for upgrade.
        let mut modules = vec![
            make_resolved_module(
                0,
                vec![make_resolved_import(
                    "./utils",
                    ResolveResult::InternalModule(FileId(5)),
                )],
                vec![],
                vec![],
            ),
            make_resolved_module(
                1,
                vec![make_resolved_import(
                    "./utils",
                    ResolveResult::NpmPackage("utils".into()),
                )],
                vec![],
                vec![],
            ),
        ];

        apply_specifier_upgrades(&mut modules);

        // Module 1 should still be NpmPackage — relative specifier not upgraded
        assert!(matches!(
            modules[1].resolved_imports[0].target,
            ResolveResult::NpmPackage(_)
        ));
    }

    #[test]
    fn specifier_upgrades_applies_to_dynamic_imports() {
        let mut modules = vec![
            make_resolved_module(
                0,
                vec![],
                vec![make_resolved_import(
                    "preact/hooks",
                    ResolveResult::InternalModule(FileId(5)),
                )],
                vec![],
            ),
            make_resolved_module(
                1,
                vec![],
                vec![make_resolved_import(
                    "preact/hooks",
                    ResolveResult::NpmPackage("preact".into()),
                )],
                vec![],
            ),
        ];

        apply_specifier_upgrades(&mut modules);

        assert!(matches!(
            modules[1].resolved_dynamic_imports[0].target,
            ResolveResult::InternalModule(FileId(5))
        ));
    }

    #[test]
    fn specifier_upgrades_applies_to_re_exports() {
        let mut modules = vec![
            make_resolved_module(
                0,
                vec![],
                vec![],
                vec![make_resolved_re_export(
                    "preact/hooks",
                    ResolveResult::InternalModule(FileId(5)),
                )],
            ),
            make_resolved_module(
                1,
                vec![],
                vec![],
                vec![make_resolved_re_export(
                    "preact/hooks",
                    ResolveResult::NpmPackage("preact".into()),
                )],
            ),
        ];

        apply_specifier_upgrades(&mut modules);

        assert!(matches!(
            modules[1].re_exports[0].target,
            ResolveResult::InternalModule(FileId(5))
        ));
    }

    #[test]
    fn specifier_upgrades_does_not_downgrade_internal() {
        // If both modules already resolve to InternalModule, nothing changes
        let mut modules = vec![
            make_resolved_module(
                0,
                vec![make_resolved_import(
                    "preact/hooks",
                    ResolveResult::InternalModule(FileId(5)),
                )],
                vec![],
                vec![],
            ),
            make_resolved_module(
                1,
                vec![make_resolved_import(
                    "preact/hooks",
                    ResolveResult::InternalModule(FileId(5)),
                )],
                vec![],
                vec![],
            ),
        ];

        apply_specifier_upgrades(&mut modules);

        assert!(matches!(
            modules[0].resolved_imports[0].target,
            ResolveResult::InternalModule(FileId(5))
        ));
        assert!(matches!(
            modules[1].resolved_imports[0].target,
            ResolveResult::InternalModule(FileId(5))
        ));
    }

    #[test]
    fn specifier_upgrades_first_internal_wins() {
        // Two modules resolve the same bare specifier to different internal files.
        // The first one (by module order) wins.
        let mut modules = vec![
            make_resolved_module(
                0,
                vec![make_resolved_import(
                    "shared-lib",
                    ResolveResult::InternalModule(FileId(10)),
                )],
                vec![],
                vec![],
            ),
            make_resolved_module(
                1,
                vec![make_resolved_import(
                    "shared-lib",
                    ResolveResult::InternalModule(FileId(20)),
                )],
                vec![],
                vec![],
            ),
            make_resolved_module(
                2,
                vec![make_resolved_import(
                    "shared-lib",
                    ResolveResult::NpmPackage("shared-lib".into()),
                )],
                vec![],
                vec![],
            ),
        ];

        apply_specifier_upgrades(&mut modules);

        // Module 2 should be upgraded to the first FileId encountered (10)
        assert!(matches!(
            modules[2].resolved_imports[0].target,
            ResolveResult::InternalModule(FileId(10))
        ));
    }

    #[test]
    fn specifier_upgrades_does_not_touch_unresolvable() {
        // Unresolvable should not be upgraded even if a bare specifier
        // matches an InternalModule elsewhere.
        let mut modules = vec![
            make_resolved_module(
                0,
                vec![make_resolved_import(
                    "my-lib",
                    ResolveResult::InternalModule(FileId(1)),
                )],
                vec![],
                vec![],
            ),
            make_resolved_module(
                1,
                vec![ResolvedImport {
                    info: make_import("my-lib", ImportedName::Default, "myLib"),
                    target: ResolveResult::Unresolvable("my-lib".into()),
                }],
                vec![],
                vec![],
            ),
        ];

        apply_specifier_upgrades(&mut modules);

        // Unresolvable should remain unresolvable
        assert!(matches!(
            modules[1].resolved_imports[0].target,
            ResolveResult::Unresolvable(_)
        ));
    }

    #[test]
    fn specifier_upgrades_cross_import_and_re_export() {
        // An import in module 0 resolves to InternalModule, a re-export in
        // module 1 for the same specifier should also be upgraded.
        let mut modules = vec![
            make_resolved_module(
                0,
                vec![make_resolved_import(
                    "@myorg/utils",
                    ResolveResult::InternalModule(FileId(3)),
                )],
                vec![],
                vec![],
            ),
            make_resolved_module(
                1,
                vec![],
                vec![],
                vec![make_resolved_re_export(
                    "@myorg/utils",
                    ResolveResult::NpmPackage("@myorg/utils".into()),
                )],
            ),
        ];

        apply_specifier_upgrades(&mut modules);

        assert!(matches!(
            modules[1].re_exports[0].target,
            ResolveResult::InternalModule(FileId(3))
        ));
    }

    // -----------------------------------------------------------------------
    // resolve_dynamic_patterns
    // -----------------------------------------------------------------------

    #[test]
    fn dynamic_patterns_matches_files_in_dir() {
        let from_dir = Path::new("/project/src");
        let patterns = vec![DynamicImportPattern {
            prefix: "./locales/".into(),
            suffix: Some(".json".into()),
            span: dummy_span(),
        }];
        let canonical_paths = vec![
            PathBuf::from("/project/src/locales/en.json"),
            PathBuf::from("/project/src/locales/fr.json"),
            PathBuf::from("/project/src/utils.ts"),
        ];
        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: PathBuf::from("/project/src/locales/en.json"),
                size_bytes: 100,
            },
            DiscoveredFile {
                id: FileId(1),
                path: PathBuf::from("/project/src/locales/fr.json"),
                size_bytes: 100,
            },
            DiscoveredFile {
                id: FileId(2),
                path: PathBuf::from("/project/src/utils.ts"),
                size_bytes: 100,
            },
        ];

        let result = resolve_dynamic_patterns(from_dir, &patterns, &canonical_paths, &files);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.len(), 2);
        assert!(result[0].1.contains(&FileId(0)));
        assert!(result[0].1.contains(&FileId(1)));
    }

    #[test]
    fn dynamic_patterns_no_matches_returns_empty() {
        let from_dir = Path::new("/project/src");
        let patterns = vec![DynamicImportPattern {
            prefix: "./locales/".into(),
            suffix: Some(".json".into()),
            span: dummy_span(),
        }];
        let canonical_paths = vec![PathBuf::from("/project/src/utils.ts")];
        let files = vec![DiscoveredFile {
            id: FileId(0),
            path: PathBuf::from("/project/src/utils.ts"),
            size_bytes: 100,
        }];

        let result = resolve_dynamic_patterns(from_dir, &patterns, &canonical_paths, &files);

        assert!(result.is_empty());
    }

    #[test]
    fn dynamic_patterns_empty_patterns_list() {
        let from_dir = Path::new("/project/src");
        let canonical_paths = vec![PathBuf::from("/project/src/utils.ts")];
        let files = vec![DiscoveredFile {
            id: FileId(0),
            path: PathBuf::from("/project/src/utils.ts"),
            size_bytes: 100,
        }];

        let result = resolve_dynamic_patterns(from_dir, &[], &canonical_paths, &files);
        assert!(result.is_empty());
    }

    #[test]
    fn dynamic_patterns_glob_prefix_passthrough() {
        let from_dir = Path::new("/project/src");
        let patterns = vec![DynamicImportPattern {
            prefix: "./**/*.ts".into(),
            suffix: None,
            span: dummy_span(),
        }];
        let canonical_paths = vec![
            PathBuf::from("/project/src/utils.ts"),
            PathBuf::from("/project/src/deep/nested.ts"),
        ];
        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: PathBuf::from("/project/src/utils.ts"),
                size_bytes: 100,
            },
            DiscoveredFile {
                id: FileId(1),
                path: PathBuf::from("/project/src/deep/nested.ts"),
                size_bytes: 100,
            },
        ];

        let result = resolve_dynamic_patterns(from_dir, &patterns, &canonical_paths, &files);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.len(), 2);
    }

    // -----------------------------------------------------------------------
    // Unresolvable specifier handling
    // -----------------------------------------------------------------------

    #[test]
    fn static_import_unresolvable_relative_path() {
        with_empty_ctx(|ctx| {
            let imports = vec![make_import(
                "./nonexistent",
                ImportedName::Default,
                "missing",
            )];
            let file = Path::new("/project/src/app.ts");
            let result = resolve_static_imports(ctx, file, &imports);

            assert_eq!(result.len(), 1);
            assert!(matches!(result[0].target, ResolveResult::Unresolvable(_)));
        });
    }

    #[test]
    fn static_import_bare_specifier_becomes_npm_package() {
        with_empty_ctx(|ctx| {
            let imports = vec![make_import("react", ImportedName::Default, "React")];
            let file = Path::new("/project/src/app.ts");
            let result = resolve_static_imports(ctx, file, &imports);

            assert_eq!(result.len(), 1);
            assert!(matches!(
                result[0].target,
                ResolveResult::NpmPackage(ref pkg) if pkg == "react"
            ));
        });
    }

    #[test]
    fn require_bare_specifier_becomes_npm_package() {
        with_empty_ctx(|ctx| {
            let req = make_require("express", vec![], Some("express"));
            let file = Path::new("/project/src/app.js");
            let result = resolve_single_require(ctx, file, &req);

            assert_eq!(result.len(), 1);
            assert!(matches!(
                result[0].target,
                ResolveResult::NpmPackage(ref pkg) if pkg == "express"
            ));
        });
    }

    #[test]
    fn dynamic_import_unresolvable() {
        with_empty_ctx(|ctx| {
            let imp = make_dynamic("./missing-module", vec![], None);
            let file = Path::new("/project/src/app.ts");
            let result = resolve_single_dynamic_import(ctx, file, &imp);

            assert_eq!(result.len(), 1);
            assert!(matches!(result[0].target, ResolveResult::Unresolvable(_)));
        });
    }

    #[test]
    fn re_export_unresolvable() {
        with_empty_ctx(|ctx| {
            let re_exports = vec![make_re_export("./missing", "foo", "foo")];
            let file = Path::new("/project/src/index.ts");
            let result = resolve_re_exports(ctx, file, &re_exports);

            assert_eq!(result.len(), 1);
            assert!(matches!(result[0].target, ResolveResult::Unresolvable(_)));
        });
    }
}
