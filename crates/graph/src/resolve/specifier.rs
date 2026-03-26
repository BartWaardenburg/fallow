//! Main resolution engine: creates the oxc_resolver instance and resolves individual specifiers.

use std::path::{Path, PathBuf};

use oxc_resolver::{ResolveOptions, Resolver};

use super::fallbacks::{
    extract_package_name_from_node_modules_path, try_path_alias_fallback,
    try_pnpm_workspace_fallback, try_source_fallback,
};
use super::path_info::{extract_package_name, is_bare_specifier, is_path_alias};
use super::react_native::{build_condition_names, build_extensions};
use super::types::{ResolveContext, ResolveResult};

/// Create an `oxc_resolver` instance with standard configuration.
///
/// When React Native or Expo plugins are active, platform-specific extensions
/// (e.g., `.web.tsx`, `.ios.ts`) are prepended to the extension list so that
/// Metro-style platform resolution works correctly.
pub(super) fn create_resolver(active_plugins: &[String]) -> Resolver {
    let mut options = ResolveOptions {
        extensions: build_extensions(active_plugins),
        // Support TypeScript's node16/nodenext module resolution where .ts files
        // are imported with .js extensions (e.g., `import './api.js'` for `api.ts`).
        extension_alias: vec![
            (
                ".js".into(),
                vec![".ts".into(), ".tsx".into(), ".js".into()],
            ),
            (".jsx".into(), vec![".tsx".into(), ".jsx".into()]),
            (".mjs".into(), vec![".mts".into(), ".mjs".into()]),
            (".cjs".into(), vec![".cts".into(), ".cjs".into()]),
        ],
        condition_names: build_condition_names(active_plugins),
        main_fields: vec!["module".into(), "main".into()],
        ..Default::default()
    };

    // Always use auto-discovery mode so oxc_resolver finds the nearest tsconfig.json
    // for each file. This is critical for monorepos where workspace packages have
    // their own tsconfig with path aliases (e.g., `~/*` → `./src/*`). Manual mode
    // with a root tsconfig only uses that single tsconfig's paths for ALL files,
    // missing workspace-specific aliases. Auto mode walks up from each file to find
    // the nearest tsconfig.json and follows `extends` chains, so workspace tsconfigs
    // that extend a root tsconfig still inherit root-level paths.
    options.tsconfig = Some(oxc_resolver::TsconfigDiscovery::Auto);

    Resolver::new(options)
}

/// Resolve a single import specifier to a target.
pub(super) fn resolve_specifier(
    ctx: &ResolveContext<'_>,
    from_file: &Path,
    specifier: &str,
) -> ResolveResult {
    // URL imports (https://, http://, data:) are valid but can't be resolved locally
    if specifier.contains("://") || specifier.starts_with("data:") {
        return ResolveResult::ExternalFile(PathBuf::from(specifier));
    }

    // Bare specifier classification (used for fallback logic below).
    let is_bare = is_bare_specifier(specifier);
    let is_alias = is_path_alias(specifier);

    // Use resolve_file instead of resolve so that TsconfigDiscovery::Auto works.
    // oxc_resolver's resolve() ignores Auto tsconfig discovery — only resolve_file()
    // walks up from the importing file to find the nearest tsconfig.json and apply
    // its path aliases (e.g., @/ → src/).
    //
    match ctx.resolver.resolve_file(from_file, specifier) {
        Ok(resolved) => {
            let resolved_path = resolved.path();
            // Try raw path lookup first (avoids canonicalize syscall in most cases)
            if let Some(&file_id) = ctx.raw_path_to_id.get(resolved_path) {
                return ResolveResult::InternalModule(file_id);
            }

            // Fast path for bare specifiers resolving to node_modules: if the resolved
            // path is in node_modules (but not pnpm's .pnpm virtual store) and the
            // package is not a workspace package, skip the expensive canonicalize()
            // syscall and go directly to NpmPackage. Workspace packages need the full
            // fallback chain (source fallback, pnpm fallback) to map dist→src.
            // Note: the byte pattern check handles Unix and Windows separators separately.
            // Paths with mixed separators fall through to canonicalize() (perf-only cost).
            if is_bare
                && !resolved_path
                    .as_os_str()
                    .as_encoded_bytes()
                    .windows(7)
                    .any(|w| w == b"/.pnpm/" || w == b"\\.pnpm\\")
                && let Some(pkg_name) = extract_package_name_from_node_modules_path(resolved_path)
                && !ctx.workspace_roots.contains_key(pkg_name.as_str())
            {
                return ResolveResult::NpmPackage(pkg_name);
            }

            // Fall back to canonical path lookup
            match resolved_path.canonicalize() {
                Ok(canonical) => {
                    if let Some(&file_id) = ctx.path_to_id.get(canonical.as_path()) {
                        ResolveResult::InternalModule(file_id)
                    } else if let Some(fallback) = ctx.canonical_fallback
                        && let Some(file_id) = fallback.get(&canonical)
                    {
                        // Intra-project symlink: raw path differs from canonical path.
                        // The lazy fallback resolves this without upfront bulk canonicalize.
                        ResolveResult::InternalModule(file_id)
                    } else if let Some(file_id) = try_source_fallback(&canonical, ctx.path_to_id) {
                        // Exports map resolved to a built output (e.g., dist/utils.js)
                        // but the source file (e.g., src/utils.ts) is what we track.
                        ResolveResult::InternalModule(file_id)
                    } else if let Some(file_id) =
                        try_pnpm_workspace_fallback(&canonical, ctx.path_to_id, ctx.workspace_roots)
                    {
                        ResolveResult::InternalModule(file_id)
                    } else if let Some(pkg_name) =
                        extract_package_name_from_node_modules_path(&canonical)
                    {
                        ResolveResult::NpmPackage(pkg_name)
                    } else {
                        ResolveResult::ExternalFile(canonical)
                    }
                }
                Err(_) => {
                    // Path doesn't exist on disk — try source fallback on the raw path
                    if let Some(file_id) = try_source_fallback(resolved_path, ctx.path_to_id) {
                        ResolveResult::InternalModule(file_id)
                    } else if let Some(file_id) = try_pnpm_workspace_fallback(
                        resolved_path,
                        ctx.path_to_id,
                        ctx.workspace_roots,
                    ) {
                        ResolveResult::InternalModule(file_id)
                    } else if let Some(pkg_name) =
                        extract_package_name_from_node_modules_path(resolved_path)
                    {
                        ResolveResult::NpmPackage(pkg_name)
                    } else {
                        ResolveResult::ExternalFile(resolved_path.to_path_buf())
                    }
                }
            }
        }
        Err(_) => {
            if is_alias {
                // Try plugin-provided path aliases before giving up.
                // These substitute import prefixes (e.g., `~/` → `app/`) and re-resolve
                // as relative imports from the project root.
                if let Some(resolved) = try_path_alias_fallback(ctx, specifier) {
                    resolved
                } else {
                    // Path aliases that fail resolution are unresolvable, not npm packages.
                    // Classifying them as NpmPackage would cause false "unlisted dependency" reports.
                    ResolveResult::Unresolvable(specifier.to_string())
                }
            } else if is_bare {
                let pkg_name = extract_package_name(specifier);
                ResolveResult::NpmPackage(pkg_name)
            } else {
                ResolveResult::Unresolvable(specifier.to_string())
            }
        }
    }
}
