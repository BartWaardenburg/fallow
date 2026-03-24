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

    // Pre-compute canonical paths ONCE for all files in parallel (avoiding repeated syscalls).
    // Each canonicalize() is a syscall — parallelizing over rayon reduces wall time.
    let canonical_paths: Vec<PathBuf> = files
        .par_iter()
        .map(|f| f.path.canonicalize().unwrap_or_else(|_| f.path.clone()))
        .collect();

    // Build path -> FileId index using pre-computed canonical paths
    let path_to_id: FxHashMap<&Path, FileId> = canonical_paths
        .iter()
        .enumerate()
        .map(|(idx, canonical)| (canonical.as_path(), files[idx].id))
        .collect();

    // Also index by non-canonical path for fallback lookups
    let raw_path_to_id: FxHashMap<&Path, FileId> =
        files.iter().map(|f| (f.path.as_path(), f.id)).collect();

    // FileIds are sequential 0..n, so direct array indexing is faster than FxHashMap.
    let file_paths: Vec<&Path> = files.iter().map(|f| f.path.as_path()).collect();

    // Create resolver ONCE and share across threads (oxc_resolver::Resolver is Send + Sync)
    let resolver = create_resolver(active_plugins);

    // Shared resolution context — avoids passing 6 arguments to every resolve_specifier call
    let ctx = ResolveContext {
        resolver: &resolver,
        path_to_id: &path_to_id,
        raw_path_to_id: &raw_path_to_id,
        workspace_roots: &workspace_roots,
        path_aliases,
        root,
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
            all_imports.extend(resolve_require_imports(&ctx, file_path, &module.require_calls));

            let from_dir = canonical_paths
                .get(module.file_id.0 as usize)
                .and_then(|p| p.parent())
                .unwrap_or(file_path);

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
            },
            target: target.clone(),
        })
        .collect()
}

/// Resolve dynamic import patterns via glob matching against discovered files.
/// Uses pre-computed canonical paths (no syscalls in inner loop).
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
            let matched: Vec<FileId> = canonical_paths
                .iter()
                .enumerate()
                .filter(|(_idx, canonical)| {
                    canonical.strip_prefix(from_dir).is_ok_and(|relative| {
                        let rel_str = format!("./{}", relative.to_string_lossy());
                        matcher.is_match(&rel_str)
                    })
                })
                .map(|(idx, _)| files[idx].id)
                .collect();
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
