use std::collections::HashMap;
use std::path::{Path, PathBuf};

use dashmap::DashMap;
use oxc_resolver::{ResolveOptions, Resolver};
use rayon::prelude::*;

use fallow_types::discover::{DiscoveredFile, FileId};
use fallow_types::extract::{ImportInfo, ModuleInfo, ReExportInfo};

/// Thread-safe cache for bare specifier resolutions using lock-free concurrent reads.
/// Bare specifiers (like `react`, `lodash/merge`) resolve to the same target
/// regardless of which file imports them (modulo nested `node_modules`, which is rare).
/// Uses `DashMap` (sharded read-write locks) instead of `Mutex<HashMap>` to eliminate
/// contention under rayon's work-stealing on large projects.
struct BareSpecifierCache {
    cache: DashMap<String, ResolveResult>,
}

impl BareSpecifierCache {
    fn new() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }

    fn get(&self, specifier: &str) -> Option<ResolveResult> {
        self.cache.get(specifier).map(|entry| entry.clone())
    }

    fn insert(&self, specifier: String, result: ResolveResult) {
        self.cache.insert(specifier, result);
    }
}

/// Result of resolving an import specifier.
#[derive(Debug, Clone)]
pub enum ResolveResult {
    /// Resolved to a file within the project.
    InternalModule(FileId),
    /// Resolved to a file outside the project (`node_modules`, `.json`, etc.).
    ExternalFile(PathBuf),
    /// Bare specifier — an npm package.
    NpmPackage(String),
    /// Could not resolve.
    Unresolvable(String),
}

/// A resolved import with its target.
#[derive(Debug, Clone)]
pub struct ResolvedImport {
    pub info: ImportInfo,
    pub target: ResolveResult,
}

/// A resolved re-export with its target.
#[derive(Debug, Clone)]
pub struct ResolvedReExport {
    pub info: ReExportInfo,
    pub target: ResolveResult,
}

/// Fully resolved module with all imports mapped to targets.
#[derive(Debug)]
pub struct ResolvedModule {
    pub file_id: FileId,
    pub path: PathBuf,
    pub exports: Vec<fallow_types::extract::ExportInfo>,
    pub re_exports: Vec<ResolvedReExport>,
    pub resolved_imports: Vec<ResolvedImport>,
    pub resolved_dynamic_imports: Vec<ResolvedImport>,
    pub resolved_dynamic_patterns: Vec<(fallow_types::extract::DynamicImportPattern, Vec<FileId>)>,
    pub member_accesses: Vec<fallow_types::extract::MemberAccess>,
    pub whole_object_uses: Vec<String>,
    pub has_cjs_exports: bool,
}

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
    let workspace_roots: HashMap<&str, &Path> = workspaces
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
    let path_to_id: HashMap<&Path, FileId> = canonical_paths
        .iter()
        .enumerate()
        .map(|(idx, canonical)| (canonical.as_path(), files[idx].id))
        .collect();

    // Also index by non-canonical path for fallback lookups
    let raw_path_to_id: HashMap<&Path, FileId> =
        files.iter().map(|f| (f.path.as_path(), f.id)).collect();

    // FileIds are sequential 0..n, so direct array indexing is faster than HashMap.
    let file_paths: Vec<&Path> = files.iter().map(|f| f.path.as_path()).collect();

    // Create resolver ONCE and share across threads (oxc_resolver::Resolver is Send + Sync)
    let resolver = create_resolver(active_plugins);

    // Cache for bare specifier resolutions (e.g., `react`, `lodash/merge`)
    let bare_cache = BareSpecifierCache::new();

    // Resolve in parallel — shared resolver instance
    modules
        .par_iter()
        .filter_map(|module| {
            let Some(file_path) = file_paths.get(module.file_id.0 as usize) else {
                tracing::warn!(
                    file_id = module.file_id.0,
                    "Skipping module with unknown file_id during resolution"
                );
                return None;
            };

            let resolved_imports: Vec<ResolvedImport> = module
                .imports
                .iter()
                .map(|imp| ResolvedImport {
                    info: imp.clone(),
                    target: resolve_specifier(
                        &resolver,
                        file_path,
                        &imp.source,
                        &path_to_id,
                        &raw_path_to_id,
                        &bare_cache,
                        &workspace_roots,
                        path_aliases,
                        root,
                    ),
                })
                .collect();

            let resolved_dynamic_imports: Vec<ResolvedImport> = module
                .dynamic_imports
                .iter()
                .flat_map(|imp| {
                    let target = resolve_specifier(
                        &resolver,
                        file_path,
                        &imp.source,
                        &path_to_id,
                        &raw_path_to_id,
                        &bare_cache,
                        &workspace_roots,
                        path_aliases,
                        root,
                    );
                    if !imp.destructured_names.is_empty() {
                        // `const { a, b } = await import('./x')` → Named imports
                        imp.destructured_names
                            .iter()
                            .map(|name| ResolvedImport {
                                info: ImportInfo {
                                    source: imp.source.clone(),
                                    imported_name: fallow_types::extract::ImportedName::Named(
                                        name.clone(),
                                    ),
                                    local_name: name.clone(),
                                    is_type_only: false,
                                    span: imp.span,
                                },
                                target: target.clone(),
                            })
                            .collect()
                    } else if imp.local_name.is_some() {
                        // `const mod = await import('./x')` → Namespace with local_name
                        vec![ResolvedImport {
                            info: ImportInfo {
                                source: imp.source.clone(),
                                imported_name: fallow_types::extract::ImportedName::Namespace,
                                local_name: imp.local_name.clone().unwrap_or_default(),
                                is_type_only: false,
                                span: imp.span,
                            },
                            target,
                        }]
                    } else {
                        // Side-effect only: `await import('./x')` with no assignment
                        vec![ResolvedImport {
                            info: ImportInfo {
                                source: imp.source.clone(),
                                imported_name: fallow_types::extract::ImportedName::SideEffect,
                                local_name: String::new(),
                                is_type_only: false,
                                span: imp.span,
                            },
                            target,
                        }]
                    }
                })
                .collect();

            let re_exports: Vec<ResolvedReExport> = module
                .re_exports
                .iter()
                .map(|re| ResolvedReExport {
                    info: re.clone(),
                    target: resolve_specifier(
                        &resolver,
                        file_path,
                        &re.source,
                        &path_to_id,
                        &raw_path_to_id,
                        &bare_cache,
                        &workspace_roots,
                        path_aliases,
                        root,
                    ),
                })
                .collect();

            // Also resolve require() calls.
            // Destructured requires → Named imports; others → Namespace (conservative).
            let require_imports: Vec<ResolvedImport> = module
                .require_calls
                .iter()
                .flat_map(|req| {
                    let target = resolve_specifier(
                        &resolver,
                        file_path,
                        &req.source,
                        &path_to_id,
                        &raw_path_to_id,
                        &bare_cache,
                        &workspace_roots,
                        path_aliases,
                        root,
                    );
                    if req.destructured_names.is_empty() {
                        vec![ResolvedImport {
                            info: ImportInfo {
                                source: req.source.clone(),
                                imported_name: fallow_types::extract::ImportedName::Namespace,
                                local_name: req.local_name.clone().unwrap_or_default(),
                                is_type_only: false,
                                span: req.span,
                            },
                            target,
                        }]
                    } else {
                        req.destructured_names
                            .iter()
                            .map(|name| ResolvedImport {
                                info: ImportInfo {
                                    source: req.source.clone(),
                                    imported_name: fallow_types::extract::ImportedName::Named(
                                        name.clone(),
                                    ),
                                    local_name: name.clone(),
                                    is_type_only: false,
                                    span: req.span,
                                },
                                target: target.clone(),
                            })
                            .collect()
                    }
                })
                .collect();

            let mut all_imports = resolved_imports;
            all_imports.extend(require_imports);

            // Resolve dynamic import patterns via glob matching against discovered files.
            // Use pre-computed canonical paths (no syscalls in inner loop).
            let from_dir = canonical_paths
                .get(module.file_id.0 as usize)
                .and_then(|p| p.parent())
                .unwrap_or(file_path);
            let resolved_dynamic_patterns: Vec<(
                fallow_types::extract::DynamicImportPattern,
                Vec<FileId>,
            )> = module
                .dynamic_import_patterns
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
                            if let Ok(relative) = canonical.strip_prefix(from_dir) {
                                let rel_str = format!("./{}", relative.to_string_lossy());
                                matcher.is_match(&rel_str)
                            } else {
                                false
                            }
                        })
                        .map(|(idx, _)| files[idx].id)
                        .collect();
                    if matched.is_empty() {
                        None
                    } else {
                        Some((pattern.clone(), matched))
                    }
                })
                .collect();

            Some(ResolvedModule {
                file_id: module.file_id,
                path: file_path.to_path_buf(),
                exports: module.exports.clone(),
                re_exports,
                resolved_imports: all_imports,
                resolved_dynamic_imports,
                resolved_dynamic_patterns,
                member_accesses: module.member_accesses.clone(),
                whole_object_uses: module.whole_object_uses.clone(),
                has_cjs_exports: module.has_cjs_exports,
            })
        })
        .collect()
}

/// Check if a bare specifier looks like a path alias rather than an npm package.
///
/// Path aliases (e.g., `@/components`, `~/lib`, `#internal`, `~~/utils`) are resolved
/// via tsconfig.json `paths` or package.json `imports`. They should not be cached
/// (resolution depends on the importing file's tsconfig context) and should return
/// `Unresolvable` (not `NpmPackage`) when resolution fails.
pub fn is_path_alias(specifier: &str) -> bool {
    // `#` prefix is Node.js imports maps (package.json "imports" field)
    if specifier.starts_with('#') {
        return true;
    }
    // `~/` and `~~/` prefixes are common alias conventions (e.g., Nuxt, custom tsconfig)
    if specifier.starts_with("~/") || specifier.starts_with("~~/") {
        return true;
    }
    // `@/` is a very common path alias (e.g., `@/components/Foo`)
    if specifier.starts_with("@/") {
        return true;
    }
    // npm scoped packages MUST be lowercase (npm registry requirement).
    // PascalCase `@Scope` or `@Scope/path` patterns are tsconfig path aliases,
    // not npm packages. E.g., `@Components`, `@Hooks/useApi`, `@Services/auth`.
    if specifier.starts_with('@') {
        let scope = specifier.split('/').next().unwrap_or(specifier);
        if scope.len() > 1 && scope.chars().nth(1).is_some_and(|c| c.is_ascii_uppercase()) {
            return true;
        }
    }

    false
}

/// React Native platform extension prefixes.
/// Metro resolves platform-specific files (e.g., `./foo` -> `./foo.web.tsx` on web).
const RN_PLATFORM_PREFIXES: &[&str] = &[".web", ".ios", ".android", ".native"];

/// Check if React Native or Expo plugins are active.
fn has_react_native_plugin(active_plugins: &[String]) -> bool {
    active_plugins
        .iter()
        .any(|p| p == "react-native" || p == "expo")
}

/// Build the resolver extension list, optionally prepending React Native platform
/// extensions when the RN/Expo plugin is active.
fn build_extensions(active_plugins: &[String]) -> Vec<String> {
    let base: Vec<String> = vec![
        ".ts".into(),
        ".tsx".into(),
        ".d.ts".into(),
        ".d.mts".into(),
        ".d.cts".into(),
        ".mts".into(),
        ".cts".into(),
        ".js".into(),
        ".jsx".into(),
        ".mjs".into(),
        ".cjs".into(),
        ".json".into(),
        ".vue".into(),
        ".svelte".into(),
        ".astro".into(),
        ".mdx".into(),
        ".css".into(),
        ".scss".into(),
    ];

    if has_react_native_plugin(active_plugins) {
        let source_exts = [".ts", ".tsx", ".js", ".jsx"];
        let mut rn_extensions: Vec<String> = Vec::new();
        for platform in RN_PLATFORM_PREFIXES {
            for ext in &source_exts {
                rn_extensions.push(format!("{platform}{ext}"));
            }
        }
        rn_extensions.extend(base);
        rn_extensions
    } else {
        base
    }
}

/// Build the resolver `condition_names` list, optionally prepending React Native
/// conditions when the RN/Expo plugin is active.
fn build_condition_names(active_plugins: &[String]) -> Vec<String> {
    let mut names = vec![
        "import".into(),
        "require".into(),
        "default".into(),
        "types".into(),
        "node".into(),
    ];
    if has_react_native_plugin(active_plugins) {
        names.insert(0, "react-native".into());
        names.insert(1, "browser".into());
    }
    names
}

/// Create an `oxc_resolver` instance with standard configuration.
///
/// When React Native or Expo plugins are active, platform-specific extensions
/// (e.g., `.web.tsx`, `.ios.ts`) are prepended to the extension list so that
/// Metro-style platform resolution works correctly.
fn create_resolver(active_plugins: &[String]) -> Resolver {
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
#[allow(clippy::too_many_arguments)]
fn resolve_specifier(
    resolver: &Resolver,
    from_file: &Path,
    specifier: &str,
    path_to_id: &HashMap<&Path, FileId>,
    raw_path_to_id: &HashMap<&Path, FileId>,
    bare_cache: &BareSpecifierCache,
    workspace_roots: &HashMap<&str, &Path>,
    path_aliases: &[(String, String)],
    root: &Path,
) -> ResolveResult {
    // URL imports (https://, http://, data:) are valid but can't be resolved locally
    if specifier.contains("://") || specifier.starts_with("data:") {
        return ResolveResult::ExternalFile(PathBuf::from(specifier));
    }

    // Fast path for bare specifiers: check cache first to avoid repeated resolver work.
    // Path aliases (e.g., `@/components`, `~/lib`) are excluded from caching because
    // they may resolve differently depending on the importing file's tsconfig context.
    let is_bare = is_bare_specifier(specifier);
    let is_alias = is_path_alias(specifier);
    if is_bare
        && !is_alias
        && let Some(cached) = bare_cache.get(specifier)
    {
        return cached;
    }

    // Use resolve_file instead of resolve so that TsconfigDiscovery::Auto works.
    // oxc_resolver's resolve() ignores Auto tsconfig discovery — only resolve_file()
    // walks up from the importing file to find the nearest tsconfig.json and apply
    // its path aliases (e.g., @/ → src/).
    let result = match resolver.resolve_file(from_file, specifier) {
        Ok(resolved) => {
            let resolved_path = resolved.path();
            // Try raw path lookup first (avoids canonicalize syscall in most cases)
            if let Some(&file_id) = raw_path_to_id.get(resolved_path) {
                return ResolveResult::InternalModule(file_id);
            }
            // Fall back to canonical path lookup
            match resolved_path.canonicalize() {
                Ok(canonical) => {
                    if let Some(&file_id) = path_to_id.get(canonical.as_path()) {
                        ResolveResult::InternalModule(file_id)
                    } else if let Some(file_id) = try_source_fallback(&canonical, path_to_id) {
                        // Exports map resolved to a built output (e.g., dist/utils.js)
                        // but the source file (e.g., src/utils.ts) is what we track.
                        ResolveResult::InternalModule(file_id)
                    } else if let Some(file_id) =
                        try_pnpm_workspace_fallback(&canonical, path_to_id, workspace_roots)
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
                    if let Some(file_id) = try_source_fallback(resolved_path, path_to_id) {
                        ResolveResult::InternalModule(file_id)
                    } else if let Some(file_id) =
                        try_pnpm_workspace_fallback(resolved_path, path_to_id, workspace_roots)
                    {
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
                if let Some(resolved) = try_path_alias_fallback(
                    resolver,
                    specifier,
                    path_aliases,
                    root,
                    path_to_id,
                    raw_path_to_id,
                    workspace_roots,
                ) {
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
    };

    // Cache bare specifier results (NpmPackage or failed resolutions) for reuse.
    // Path aliases are excluded — they resolve relative to the importing file's tsconfig.
    if is_bare && !is_alias {
        bare_cache.insert(specifier.to_string(), result.clone());
    }

    result
}

/// Try resolving a specifier using plugin-provided path aliases.
///
/// Substitutes a matching alias prefix (e.g., `~/`) with a directory relative to the
/// project root (e.g., `app/`) and resolves the resulting path. This handles framework
/// aliases like Nuxt's `~/`, `~~/`, `#shared/` that aren't defined in tsconfig.json
/// but map to real filesystem paths.
fn try_path_alias_fallback(
    resolver: &Resolver,
    specifier: &str,
    path_aliases: &[(String, String)],
    root: &Path,
    path_to_id: &HashMap<&Path, FileId>,
    raw_path_to_id: &HashMap<&Path, FileId>,
    workspace_roots: &HashMap<&str, &Path>,
) -> Option<ResolveResult> {
    for (prefix, replacement) in path_aliases {
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
        let root_file = root.join("__resolve_root__");
        if let Ok(resolved) = resolver.resolve_file(&root_file, &substituted) {
            let resolved_path = resolved.path();
            // Try raw path lookup first
            if let Some(&file_id) = raw_path_to_id.get(resolved_path) {
                return Some(ResolveResult::InternalModule(file_id));
            }
            // Fall back to canonical path lookup
            if let Ok(canonical) = resolved_path.canonicalize() {
                if let Some(&file_id) = path_to_id.get(canonical.as_path()) {
                    return Some(ResolveResult::InternalModule(file_id));
                }
                if let Some(file_id) = try_source_fallback(&canonical, path_to_id) {
                    return Some(ResolveResult::InternalModule(file_id));
                }
                if let Some(file_id) =
                    try_pnpm_workspace_fallback(&canonical, path_to_id, workspace_roots)
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

/// Known output directory names that may appear in exports map targets.
/// When an exports map points to `./dist/utils.js`, we try replacing these
/// prefixes with `src/` (the conventional source directory) to find the tracked
/// source file.
const OUTPUT_DIRS: &[&str] = &["dist", "build", "out", "esm", "cjs"];

/// Source extensions to try when mapping a built output file back to source.
const SOURCE_EXTS: &[&str] = &["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"];

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
fn try_source_fallback(resolved: &Path, path_to_id: &HashMap<&Path, FileId>) -> Option<FileId> {
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
fn extract_package_name_from_node_modules_path(path: &Path) -> Option<String> {
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
fn try_pnpm_workspace_fallback(
    path: &Path,
    path_to_id: &HashMap<&Path, FileId>,
    workspace_roots: &HashMap<&str, &Path>,
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
fn make_glob_from_pattern(pattern: &fallow_types::extract::DynamicImportPattern) -> String {
    // If the prefix already contains glob characters (from import.meta.glob), use as-is
    if pattern.prefix.contains('*') || pattern.prefix.contains('{') {
        return pattern.prefix.clone();
    }
    match &pattern.suffix {
        Some(suffix) => format!("{}*{}", pattern.prefix, suffix),
        None => format!("{}*", pattern.prefix),
    }
}

/// Check if a specifier is a bare specifier (npm package or Node.js imports map entry).
fn is_bare_specifier(specifier: &str) -> bool {
    !specifier.starts_with('.')
        && !specifier.starts_with('/')
        && !specifier.contains("://")
        && !specifier.starts_with("data:")
}

/// Extract the npm package name from a specifier.
/// `@scope/pkg/foo/bar` -> `@scope/pkg`
/// `lodash/merge` -> `lodash`
pub fn extract_package_name(specifier: &str) -> String {
    if specifier.starts_with('@') {
        let parts: Vec<&str> = specifier.splitn(3, '/').collect();
        if parts.len() >= 2 {
            format!("{}/{}", parts[0], parts[1])
        } else {
            specifier.to_string()
        }
    } else {
        specifier.split('/').next().unwrap_or(specifier).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_package_name() {
        assert_eq!(extract_package_name("react"), "react");
        assert_eq!(extract_package_name("lodash/merge"), "lodash");
        assert_eq!(extract_package_name("@scope/pkg"), "@scope/pkg");
        assert_eq!(extract_package_name("@scope/pkg/foo"), "@scope/pkg");
    }

    #[test]
    fn test_is_bare_specifier() {
        assert!(is_bare_specifier("react"));
        assert!(is_bare_specifier("@scope/pkg"));
        assert!(is_bare_specifier("#internal/module"));
        assert!(!is_bare_specifier("./utils"));
        assert!(!is_bare_specifier("../lib"));
        assert!(!is_bare_specifier("/absolute"));
    }

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
        let mut path_to_id = HashMap::new();
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
        let mut path_to_id = HashMap::new();
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
        let path_to_id: HashMap<&Path, FileId> = HashMap::new();

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
        let mut path_to_id = HashMap::new();
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
        let mut path_to_id = HashMap::new();
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
        let mut path_to_id = HashMap::new();
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
        let mut path_to_id = HashMap::new();
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
        let mut path_to_id = HashMap::new();
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
        let mut path_to_id = HashMap::new();
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
        let mut path_to_id = HashMap::new();
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
        let mut path_to_id = HashMap::new();
        path_to_id.insert(src_path.as_path(), FileId(0));

        let mut workspace_roots = HashMap::new();
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
        let mut path_to_id = HashMap::new();
        path_to_id.insert(src_path.as_path(), FileId(1));

        let mut workspace_roots = HashMap::new();
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
        let path_to_id: HashMap<&Path, FileId> = HashMap::new();

        let mut workspace_roots = HashMap::new();
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
        let mut path_to_id = HashMap::new();
        path_to_id.insert(src_path.as_path(), FileId(2));

        let mut workspace_roots = HashMap::new();
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
        let mut path_to_id = HashMap::new();
        path_to_id.insert(src_path.as_path(), FileId(3));

        let mut workspace_roots = HashMap::new();
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
        let path_to_id: HashMap<&Path, FileId> = HashMap::new();
        let workspace_roots: HashMap<&str, &Path> = HashMap::new();

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
        let mut path_to_id = HashMap::new();
        path_to_id.insert(src_path.as_path(), FileId(4));

        let mut workspace_roots = HashMap::new();
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

    #[test]
    fn test_has_react_native_plugin_active() {
        let plugins = vec!["react-native".to_string(), "typescript".to_string()];
        assert!(has_react_native_plugin(&plugins));
    }

    #[test]
    fn test_has_expo_plugin_active() {
        let plugins = vec!["expo".to_string(), "typescript".to_string()];
        assert!(has_react_native_plugin(&plugins));
    }

    #[test]
    fn test_has_react_native_plugin_inactive() {
        let plugins = vec!["nextjs".to_string(), "typescript".to_string()];
        assert!(!has_react_native_plugin(&plugins));
    }

    #[test]
    fn test_rn_platform_extensions_prepended() {
        let no_rn = build_extensions(&[]);
        let rn_plugins = vec!["react-native".to_string()];
        let with_rn = build_extensions(&rn_plugins);

        // Without RN, the first extension should be .ts
        assert_eq!(no_rn[0], ".ts");

        // With RN, platform extensions should come first
        assert_eq!(with_rn[0], ".web.ts");
        assert_eq!(with_rn[1], ".web.tsx");
        assert_eq!(with_rn[2], ".web.js");
        assert_eq!(with_rn[3], ".web.jsx");

        // Verify all 4 platforms (web, ios, android, native) x 4 exts = 16
        assert!(with_rn.len() > no_rn.len());
        assert_eq!(
            with_rn.len(),
            no_rn.len() + 16,
            "should add 16 platform extensions (4 platforms x 4 exts)"
        );
    }

    #[test]
    fn test_rn_condition_names_prepended() {
        let no_rn = build_condition_names(&[]);
        let rn_plugins = vec!["react-native".to_string()];
        let with_rn = build_condition_names(&rn_plugins);

        // Without RN, first condition should be "import"
        assert_eq!(no_rn[0], "import");

        // With RN, "react-native" and "browser" should be prepended
        assert_eq!(with_rn[0], "react-native");
        assert_eq!(with_rn[1], "browser");
        assert_eq!(with_rn[2], "import");
    }
}
