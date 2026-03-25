//! Resolution fallback strategies for import specifiers.
//!
//! Handles path alias fallbacks, output-to-source directory mapping, pnpm virtual
//! store detection, node_modules package extraction, and dynamic import glob patterns.

use std::path::{Path, PathBuf};

use rustc_hash::FxHashMap;

use fallow_types::discover::FileId;

use super::types::{OUTPUT_DIRS, ResolveContext, ResolveResult, SOURCE_EXTS};

/// Try resolving a specifier using plugin-provided path aliases.
///
/// Substitutes a matching alias prefix (e.g., `~/`) with a directory relative to the
/// project root (e.g., `app/`) and resolves the resulting path. This handles framework
/// aliases like Nuxt's `~/`, `~~/`, `#shared/` that aren't defined in tsconfig.json
/// but map to real filesystem paths.
pub(super) fn try_path_alias_fallback(
    ctx: &ResolveContext<'_>,
    specifier: &str,
) -> Option<ResolveResult> {
    for (prefix, replacement) in ctx.path_aliases {
        if !specifier.starts_with(prefix.as_str()) {
            continue;
        }

        let remainder = &specifier[prefix.len()..];
        // Build the substituted path relative to root.
        // If replacement is empty, remainder is relative to root directly.
        let substituted = if replacement.is_empty() {
            format!("./{remainder}")
        } else {
            format!("./{replacement}/{remainder}")
        };

        // Resolve from a synthetic file at the project root so relative paths work.
        // Use a dummy file path in the root directory.
        let root_file = ctx.root.join("__resolve_root__");
        if let Ok(resolved) = ctx.resolver.resolve_file(&root_file, &substituted) {
            let resolved_path = resolved.path();
            // Try raw path lookup first
            if let Some(&file_id) = ctx.raw_path_to_id.get(resolved_path) {
                return Some(ResolveResult::InternalModule(file_id));
            }
            // Fall back to canonical path lookup
            if let Ok(canonical) = resolved_path.canonicalize() {
                if let Some(&file_id) = ctx.path_to_id.get(canonical.as_path()) {
                    return Some(ResolveResult::InternalModule(file_id));
                }
                if let Some(file_id) = try_source_fallback(&canonical, ctx.path_to_id) {
                    return Some(ResolveResult::InternalModule(file_id));
                }
                if let Some(file_id) =
                    try_pnpm_workspace_fallback(&canonical, ctx.path_to_id, ctx.workspace_roots)
                {
                    return Some(ResolveResult::InternalModule(file_id));
                }
                if let Some(pkg_name) = extract_package_name_from_node_modules_path(&canonical) {
                    return Some(ResolveResult::NpmPackage(pkg_name));
                }
                return Some(ResolveResult::ExternalFile(canonical));
            }
        }
    }
    None
}

/// Try to map a resolved output path (e.g., `packages/ui/dist/utils.js`) back to
/// the corresponding source file (e.g., `packages/ui/src/utils.ts`).
///
/// This handles cross-workspace imports that go through `exports` maps pointing to
/// built output directories. Since fallow ignores `dist/`, `build/`, etc. by default,
/// the resolved path won't be in the file set, but the source file will be.
///
/// Nested output subdirectories (e.g., `dist/esm/utils.mjs`, `build/cjs/index.cjs`)
/// are handled by finding the last output directory component (closest to the file,
/// avoiding false matches on parent directories) and then walking backwards to collect
/// all consecutive output directory components before it.
pub(super) fn try_source_fallback(
    resolved: &Path,
    path_to_id: &FxHashMap<&Path, FileId>,
) -> Option<FileId> {
    let components: Vec<_> = resolved.components().collect();

    let is_output_dir = |c: &std::path::Component| -> bool {
        if let std::path::Component::Normal(s) = c
            && let Some(name) = s.to_str()
        {
            return OUTPUT_DIRS.contains(&name);
        }
        false
    };

    // Find the LAST output directory component (closest to the file).
    // Using rposition avoids false matches on parent directories that happen to
    // be named "build", "dist", etc.
    let last_output_pos = components.iter().rposition(&is_output_dir)?;

    // Walk backwards to find the start of consecutive output directory components.
    // e.g., for `dist/esm/utils.mjs`, rposition finds `esm`, then we walk back to `dist`.
    let mut first_output_pos = last_output_pos;
    while first_output_pos > 0 && is_output_dir(&components[first_output_pos - 1]) {
        first_output_pos -= 1;
    }

    // Build the path prefix (everything before the first consecutive output dir)
    let prefix: PathBuf = components[..first_output_pos].iter().collect();

    // Build the relative path after the last consecutive output dir
    let suffix: PathBuf = components[last_output_pos + 1..].iter().collect();
    suffix.file_stem()?; // Ensure the suffix has a filename

    // Try replacing the output dirs with "src" and each source extension
    for ext in SOURCE_EXTS {
        let source_candidate = prefix.join("src").join(suffix.with_extension(ext));
        if let Some(&file_id) = path_to_id.get(source_candidate.as_path()) {
            return Some(file_id);
        }
    }

    None
}

/// Extract npm package name from a resolved path inside `node_modules`.
///
/// Given a path like `/project/node_modules/react/index.js`, returns `Some("react")`.
/// Given a path like `/project/node_modules/@scope/pkg/dist/index.js`, returns `Some("@scope/pkg")`.
/// Returns `None` if the path doesn't contain a `node_modules` segment.
pub(super) fn extract_package_name_from_node_modules_path(path: &Path) -> Option<String> {
    let components: Vec<&str> = path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect();

    // Find the last "node_modules" component (handles nested node_modules)
    let nm_idx = components.iter().rposition(|&c| c == "node_modules")?;

    let after = &components[nm_idx + 1..];
    if after.is_empty() {
        return None;
    }

    if after[0].starts_with('@') {
        // Scoped package: @scope/pkg
        if after.len() >= 2 {
            Some(format!("{}/{}", after[0], after[1]))
        } else {
            Some(after[0].to_string())
        }
    } else {
        Some(after[0].to_string())
    }
}

/// Try to map a pnpm virtual store path back to a workspace source file.
///
/// When pnpm uses injected dependencies or certain linking strategies, canonical
/// paths go through `.pnpm`:
///   `/project/node_modules/.pnpm/@myorg+ui@1.0.0/node_modules/@myorg/ui/dist/index.js`
///
/// This function detects such paths, extracts the package name, checks if it
/// matches a workspace package, and tries to find the source file in that workspace.
pub(super) fn try_pnpm_workspace_fallback(
    path: &Path,
    path_to_id: &FxHashMap<&Path, FileId>,
    workspace_roots: &FxHashMap<&str, &Path>,
) -> Option<FileId> {
    // Only relevant for paths containing .pnpm
    let components: Vec<&str> = path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect();

    // Find .pnpm component
    let pnpm_idx = components.iter().position(|&c| c == ".pnpm")?;

    // After .pnpm, find the inner node_modules (the actual package location)
    // Structure: .pnpm/<name>@<version>/node_modules/<package>/...
    let after_pnpm = &components[pnpm_idx + 1..];

    // Find "node_modules" inside the .pnpm directory
    let inner_nm_idx = after_pnpm.iter().position(|&c| c == "node_modules")?;
    let after_inner_nm = &after_pnpm[inner_nm_idx + 1..];

    if after_inner_nm.is_empty() {
        return None;
    }

    // Extract package name (handle scoped packages)
    let (pkg_name, pkg_name_components) = if after_inner_nm[0].starts_with('@') {
        if after_inner_nm.len() >= 2 {
            (format!("{}/{}", after_inner_nm[0], after_inner_nm[1]), 2)
        } else {
            return None;
        }
    } else {
        (after_inner_nm[0].to_string(), 1)
    };

    // Check if this package is a workspace package
    let ws_root = workspace_roots.get(pkg_name.as_str())?;

    // Get the relative path within the package (after the package name components)
    let relative_parts = &after_inner_nm[pkg_name_components..];
    if relative_parts.is_empty() {
        return None;
    }

    let relative_path: PathBuf = relative_parts.iter().collect();

    // Try direct file lookup in workspace root
    let direct = ws_root.join(&relative_path);
    if let Some(&file_id) = path_to_id.get(direct.as_path()) {
        return Some(file_id);
    }

    // Try source fallback (dist/ → src/ etc.) within the workspace
    try_source_fallback(&direct, path_to_id)
}

/// Convert a `DynamicImportPattern` to a glob string for file matching.
pub(super) fn make_glob_from_pattern(
    pattern: &fallow_types::extract::DynamicImportPattern,
) -> String {
    // If the prefix already contains glob characters (from import.meta.glob), use as-is
    if pattern.prefix.contains('*') || pattern.prefix.contains('{') {
        return pattern.prefix.clone();
    }
    pattern.suffix.as_ref().map_or_else(
        || format!("{}*", pattern.prefix),
        |suffix| format!("{}*{}", pattern.prefix, suffix),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_package_name_from_node_modules_path_regular() {
        let path = PathBuf::from("/project/node_modules/react/index.js");
        assert_eq!(
            extract_package_name_from_node_modules_path(&path),
            Some("react".to_string())
        );
    }

    #[test]
    fn test_extract_package_name_from_node_modules_path_scoped() {
        let path = PathBuf::from("/project/node_modules/@babel/core/lib/index.js");
        assert_eq!(
            extract_package_name_from_node_modules_path(&path),
            Some("@babel/core".to_string())
        );
    }

    #[test]
    fn test_extract_package_name_from_node_modules_path_nested() {
        // Nested node_modules: should use the last (innermost) one
        let path = PathBuf::from("/project/node_modules/pkg-a/node_modules/pkg-b/dist/index.js");
        assert_eq!(
            extract_package_name_from_node_modules_path(&path),
            Some("pkg-b".to_string())
        );
    }

    #[test]
    fn test_extract_package_name_from_node_modules_path_deep_subpath() {
        let path = PathBuf::from("/project/node_modules/react-dom/cjs/react-dom.production.min.js");
        assert_eq!(
            extract_package_name_from_node_modules_path(&path),
            Some("react-dom".to_string())
        );
    }

    #[test]
    fn test_extract_package_name_from_node_modules_path_no_node_modules() {
        let path = PathBuf::from("/project/src/components/Button.tsx");
        assert_eq!(extract_package_name_from_node_modules_path(&path), None);
    }

    #[test]
    fn test_extract_package_name_from_node_modules_path_just_node_modules() {
        let path = PathBuf::from("/project/node_modules");
        assert_eq!(extract_package_name_from_node_modules_path(&path), None);
    }

    #[test]
    fn test_extract_package_name_from_node_modules_path_scoped_only_scope() {
        // Edge case: path ends at scope without package name
        let path = PathBuf::from("/project/node_modules/@scope");
        assert_eq!(
            extract_package_name_from_node_modules_path(&path),
            Some("@scope".to_string())
        );
    }

    #[test]
    fn test_resolve_specifier_node_modules_returns_npm_package() {
        // When oxc_resolver resolves to a node_modules path that is NOT in path_to_id,
        // it should return NpmPackage instead of ExternalFile.
        // We can't easily test resolve_specifier directly without a real resolver,
        // but the extract_package_name_from_node_modules_path function covers the
        // core logic that was missing.
        let path =
            PathBuf::from("/project/node_modules/styled-components/dist/styled-components.esm.js");
        assert_eq!(
            extract_package_name_from_node_modules_path(&path),
            Some("styled-components".to_string())
        );

        let path = PathBuf::from("/project/node_modules/next/dist/server/next.js");
        assert_eq!(
            extract_package_name_from_node_modules_path(&path),
            Some("next".to_string())
        );
    }

    #[test]
    fn test_try_source_fallback_dist_to_src() {
        let src_path = PathBuf::from("/project/packages/ui/src/utils.ts");
        let mut path_to_id = FxHashMap::default();
        path_to_id.insert(src_path.as_path(), FileId(0));

        let dist_path = PathBuf::from("/project/packages/ui/dist/utils.js");
        assert_eq!(
            try_source_fallback(&dist_path, &path_to_id),
            Some(FileId(0)),
            "dist/utils.js should fall back to src/utils.ts"
        );
    }

    #[test]
    fn test_try_source_fallback_build_to_src() {
        let src_path = PathBuf::from("/project/packages/core/src/index.tsx");
        let mut path_to_id = FxHashMap::default();
        path_to_id.insert(src_path.as_path(), FileId(1));

        let build_path = PathBuf::from("/project/packages/core/build/index.js");
        assert_eq!(
            try_source_fallback(&build_path, &path_to_id),
            Some(FileId(1)),
            "build/index.js should fall back to src/index.tsx"
        );
    }

    #[test]
    fn test_try_source_fallback_no_match() {
        let path_to_id: FxHashMap<&Path, FileId> = FxHashMap::default();

        let dist_path = PathBuf::from("/project/packages/ui/dist/utils.js");
        assert_eq!(
            try_source_fallback(&dist_path, &path_to_id),
            None,
            "should return None when no source file exists"
        );
    }

    #[test]
    fn test_try_source_fallback_non_output_dir() {
        let src_path = PathBuf::from("/project/packages/ui/src/utils.ts");
        let mut path_to_id = FxHashMap::default();
        path_to_id.insert(src_path.as_path(), FileId(0));

        // A path that's not in an output directory should not trigger fallback
        let normal_path = PathBuf::from("/project/packages/ui/scripts/utils.js");
        assert_eq!(
            try_source_fallback(&normal_path, &path_to_id),
            None,
            "non-output directory path should not trigger fallback"
        );
    }

    #[test]
    fn test_try_source_fallback_nested_path() {
        let src_path = PathBuf::from("/project/packages/ui/src/components/Button.ts");
        let mut path_to_id = FxHashMap::default();
        path_to_id.insert(src_path.as_path(), FileId(2));

        let dist_path = PathBuf::from("/project/packages/ui/dist/components/Button.js");
        assert_eq!(
            try_source_fallback(&dist_path, &path_to_id),
            Some(FileId(2)),
            "nested dist path should fall back to nested src path"
        );
    }

    #[test]
    fn test_try_source_fallback_nested_dist_esm() {
        let src_path = PathBuf::from("/project/packages/ui/src/utils.ts");
        let mut path_to_id = FxHashMap::default();
        path_to_id.insert(src_path.as_path(), FileId(0));

        let dist_path = PathBuf::from("/project/packages/ui/dist/esm/utils.mjs");
        assert_eq!(
            try_source_fallback(&dist_path, &path_to_id),
            Some(FileId(0)),
            "dist/esm/utils.mjs should fall back to src/utils.ts"
        );
    }

    #[test]
    fn test_try_source_fallback_nested_build_cjs() {
        let src_path = PathBuf::from("/project/packages/core/src/index.ts");
        let mut path_to_id = FxHashMap::default();
        path_to_id.insert(src_path.as_path(), FileId(1));

        let build_path = PathBuf::from("/project/packages/core/build/cjs/index.cjs");
        assert_eq!(
            try_source_fallback(&build_path, &path_to_id),
            Some(FileId(1)),
            "build/cjs/index.cjs should fall back to src/index.ts"
        );
    }

    #[test]
    fn test_try_source_fallback_nested_dist_esm_deep_path() {
        let src_path = PathBuf::from("/project/packages/ui/src/components/Button.ts");
        let mut path_to_id = FxHashMap::default();
        path_to_id.insert(src_path.as_path(), FileId(2));

        let dist_path = PathBuf::from("/project/packages/ui/dist/esm/components/Button.mjs");
        assert_eq!(
            try_source_fallback(&dist_path, &path_to_id),
            Some(FileId(2)),
            "dist/esm/components/Button.mjs should fall back to src/components/Button.ts"
        );
    }

    #[test]
    fn test_try_source_fallback_triple_nested_output_dirs() {
        let src_path = PathBuf::from("/project/packages/ui/src/utils.ts");
        let mut path_to_id = FxHashMap::default();
        path_to_id.insert(src_path.as_path(), FileId(0));

        let dist_path = PathBuf::from("/project/packages/ui/out/dist/esm/utils.mjs");
        assert_eq!(
            try_source_fallback(&dist_path, &path_to_id),
            Some(FileId(0)),
            "out/dist/esm/utils.mjs should fall back to src/utils.ts"
        );
    }

    #[test]
    fn test_try_source_fallback_parent_dir_named_build() {
        let src_path = PathBuf::from("/home/user/build/my-project/src/utils.ts");
        let mut path_to_id = FxHashMap::default();
        path_to_id.insert(src_path.as_path(), FileId(0));

        let dist_path = PathBuf::from("/home/user/build/my-project/dist/utils.js");
        assert_eq!(
            try_source_fallback(&dist_path, &path_to_id),
            Some(FileId(0)),
            "should resolve dist/ within project, not match parent 'build' dir"
        );
    }

    #[test]
    fn test_pnpm_store_path_extract_package_name() {
        // pnpm virtual store paths should correctly extract package name
        let path =
            PathBuf::from("/project/node_modules/.pnpm/react@18.2.0/node_modules/react/index.js");
        assert_eq!(
            extract_package_name_from_node_modules_path(&path),
            Some("react".to_string())
        );
    }

    #[test]
    fn test_pnpm_store_path_scoped_package() {
        let path = PathBuf::from(
            "/project/node_modules/.pnpm/@babel+core@7.24.0/node_modules/@babel/core/lib/index.js",
        );
        assert_eq!(
            extract_package_name_from_node_modules_path(&path),
            Some("@babel/core".to_string())
        );
    }

    #[test]
    fn test_pnpm_store_path_with_peer_deps() {
        let path = PathBuf::from(
            "/project/node_modules/.pnpm/webpack@5.0.0_esbuild@0.19.0/node_modules/webpack/lib/index.js",
        );
        assert_eq!(
            extract_package_name_from_node_modules_path(&path),
            Some("webpack".to_string())
        );
    }

    #[test]
    fn test_try_pnpm_workspace_fallback_dist_to_src() {
        let src_path = PathBuf::from("/project/packages/ui/src/utils.ts");
        let mut path_to_id = FxHashMap::default();
        path_to_id.insert(src_path.as_path(), FileId(0));

        let mut workspace_roots = FxHashMap::default();
        let ws_root = PathBuf::from("/project/packages/ui");
        workspace_roots.insert("@myorg/ui", ws_root.as_path());

        // pnpm virtual store path with dist/ output
        let pnpm_path = PathBuf::from(
            "/project/node_modules/.pnpm/@myorg+ui@1.0.0/node_modules/@myorg/ui/dist/utils.js",
        );
        assert_eq!(
            try_pnpm_workspace_fallback(&pnpm_path, &path_to_id, &workspace_roots),
            Some(FileId(0)),
            ".pnpm workspace path should fall back to src/utils.ts"
        );
    }

    #[test]
    fn test_try_pnpm_workspace_fallback_direct_source() {
        let src_path = PathBuf::from("/project/packages/core/src/index.ts");
        let mut path_to_id = FxHashMap::default();
        path_to_id.insert(src_path.as_path(), FileId(1));

        let mut workspace_roots = FxHashMap::default();
        let ws_root = PathBuf::from("/project/packages/core");
        workspace_roots.insert("@myorg/core", ws_root.as_path());

        // pnpm path pointing directly to src/
        let pnpm_path = PathBuf::from(
            "/project/node_modules/.pnpm/@myorg+core@workspace/node_modules/@myorg/core/src/index.ts",
        );
        assert_eq!(
            try_pnpm_workspace_fallback(&pnpm_path, &path_to_id, &workspace_roots),
            Some(FileId(1)),
            ".pnpm workspace path with src/ should resolve directly"
        );
    }

    #[test]
    fn test_try_pnpm_workspace_fallback_non_workspace_package() {
        let path_to_id: FxHashMap<&Path, FileId> = FxHashMap::default();

        let mut workspace_roots = FxHashMap::default();
        let ws_root = PathBuf::from("/project/packages/ui");
        workspace_roots.insert("@myorg/ui", ws_root.as_path());

        // External package (not a workspace) — should return None
        let pnpm_path =
            PathBuf::from("/project/node_modules/.pnpm/react@18.2.0/node_modules/react/index.js");
        assert_eq!(
            try_pnpm_workspace_fallback(&pnpm_path, &path_to_id, &workspace_roots),
            None,
            "non-workspace package in .pnpm should return None"
        );
    }

    #[test]
    fn test_try_pnpm_workspace_fallback_unscoped_package() {
        let src_path = PathBuf::from("/project/packages/utils/src/index.ts");
        let mut path_to_id = FxHashMap::default();
        path_to_id.insert(src_path.as_path(), FileId(2));

        let mut workspace_roots = FxHashMap::default();
        let ws_root = PathBuf::from("/project/packages/utils");
        workspace_roots.insert("my-utils", ws_root.as_path());

        // Unscoped workspace package in pnpm store
        let pnpm_path = PathBuf::from(
            "/project/node_modules/.pnpm/my-utils@1.0.0/node_modules/my-utils/dist/index.js",
        );
        assert_eq!(
            try_pnpm_workspace_fallback(&pnpm_path, &path_to_id, &workspace_roots),
            Some(FileId(2)),
            "unscoped workspace package in .pnpm should resolve"
        );
    }

    #[test]
    fn test_try_pnpm_workspace_fallback_nested_path() {
        let src_path = PathBuf::from("/project/packages/ui/src/components/Button.ts");
        let mut path_to_id = FxHashMap::default();
        path_to_id.insert(src_path.as_path(), FileId(3));

        let mut workspace_roots = FxHashMap::default();
        let ws_root = PathBuf::from("/project/packages/ui");
        workspace_roots.insert("@myorg/ui", ws_root.as_path());

        // Nested path within the package
        let pnpm_path = PathBuf::from(
            "/project/node_modules/.pnpm/@myorg+ui@1.0.0/node_modules/@myorg/ui/dist/components/Button.js",
        );
        assert_eq!(
            try_pnpm_workspace_fallback(&pnpm_path, &path_to_id, &workspace_roots),
            Some(FileId(3)),
            "nested .pnpm workspace path should resolve through source fallback"
        );
    }

    #[test]
    fn test_try_pnpm_workspace_fallback_no_pnpm() {
        let path_to_id: FxHashMap<&Path, FileId> = FxHashMap::default();
        let workspace_roots: FxHashMap<&str, &Path> = FxHashMap::default();

        // Regular path without .pnpm — should return None immediately
        let regular_path = PathBuf::from("/project/node_modules/react/index.js");
        assert_eq!(
            try_pnpm_workspace_fallback(&regular_path, &path_to_id, &workspace_roots),
            None,
        );
    }

    #[test]
    fn test_try_pnpm_workspace_fallback_with_peer_deps() {
        let src_path = PathBuf::from("/project/packages/ui/src/index.ts");
        let mut path_to_id = FxHashMap::default();
        path_to_id.insert(src_path.as_path(), FileId(4));

        let mut workspace_roots = FxHashMap::default();
        let ws_root = PathBuf::from("/project/packages/ui");
        workspace_roots.insert("@myorg/ui", ws_root.as_path());

        // pnpm path with peer dependency suffix
        let pnpm_path = PathBuf::from(
            "/project/node_modules/.pnpm/@myorg+ui@1.0.0_react@18.2.0/node_modules/@myorg/ui/dist/index.js",
        );
        assert_eq!(
            try_pnpm_workspace_fallback(&pnpm_path, &path_to_id, &workspace_roots),
            Some(FileId(4)),
            ".pnpm path with peer dep suffix should still resolve"
        );
    }

    // ── make_glob_from_pattern ───────────────────────────────────────

    #[test]
    fn make_glob_prefix_only_no_suffix() {
        let pattern = fallow_types::extract::DynamicImportPattern {
            prefix: "./locales/".to_string(),
            suffix: None,
            span: oxc_span::Span::default(),
        };
        assert_eq!(make_glob_from_pattern(&pattern), "./locales/*");
    }

    #[test]
    fn make_glob_prefix_with_suffix() {
        let pattern = fallow_types::extract::DynamicImportPattern {
            prefix: "./locales/".to_string(),
            suffix: Some(".json".to_string()),
            span: oxc_span::Span::default(),
        };
        assert_eq!(make_glob_from_pattern(&pattern), "./locales/*.json");
    }

    #[test]
    fn make_glob_passthrough_star() {
        // Prefix already contains glob characters — use as-is
        let pattern = fallow_types::extract::DynamicImportPattern {
            prefix: "./pages/**/*.tsx".to_string(),
            suffix: None,
            span: oxc_span::Span::default(),
        };
        assert_eq!(make_glob_from_pattern(&pattern), "./pages/**/*.tsx");
    }

    #[test]
    fn make_glob_passthrough_brace() {
        let pattern = fallow_types::extract::DynamicImportPattern {
            prefix: "./i18n/{en,de,fr}.json".to_string(),
            suffix: None,
            span: oxc_span::Span::default(),
        };
        assert_eq!(make_glob_from_pattern(&pattern), "./i18n/{en,de,fr}.json");
    }

    #[test]
    fn make_glob_empty_prefix_no_suffix() {
        let pattern = fallow_types::extract::DynamicImportPattern {
            prefix: String::new(),
            suffix: None,
            span: oxc_span::Span::default(),
        };
        assert_eq!(make_glob_from_pattern(&pattern), "*");
    }

    #[test]
    fn make_glob_empty_prefix_with_suffix() {
        let pattern = fallow_types::extract::DynamicImportPattern {
            prefix: String::new(),
            suffix: Some(".ts".to_string()),
            span: oxc_span::Span::default(),
        };
        assert_eq!(make_glob_from_pattern(&pattern), "*.ts");
    }
}
