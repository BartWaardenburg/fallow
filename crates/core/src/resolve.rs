use std::collections::HashMap;
use std::path::{Path, PathBuf};

use fallow_config::ResolvedConfig;
use oxc_resolver::{ResolveOptions, Resolver};
use rayon::prelude::*;

use crate::discover::{DiscoveredFile, FileId};
use crate::extract::{ImportInfo, ModuleInfo, ReExportInfo};

/// Result of resolving an import specifier.
#[derive(Debug, Clone)]
pub enum ResolveResult {
    /// Resolved to a file within the project.
    InternalModule(FileId),
    /// Resolved to a file outside the project (node_modules, .json, etc.).
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
    pub exports: Vec<crate::extract::ExportInfo>,
    pub re_exports: Vec<ResolvedReExport>,
    pub resolved_imports: Vec<ResolvedImport>,
    pub resolved_dynamic_imports: Vec<ResolvedImport>,
    pub member_accesses: Vec<crate::extract::MemberAccess>,
    pub has_cjs_exports: bool,
}

/// Resolve all imports across all modules in parallel.
pub fn resolve_all_imports(
    modules: &[ModuleInfo],
    config: &ResolvedConfig,
    files: &[DiscoveredFile],
) -> Vec<ResolvedModule> {
    // Build path -> FileId index (canonicalize once here)
    let path_to_id: HashMap<PathBuf, FileId> = files
        .iter()
        .filter_map(|f| {
            f.path
                .canonicalize()
                .ok()
                .map(|canonical| (canonical, f.id))
        })
        .collect();

    let file_id_to_path: HashMap<FileId, PathBuf> =
        files.iter().map(|f| (f.id, f.path.clone())).collect();

    // Create resolver ONCE and share across threads (oxc_resolver::Resolver is Send + Sync)
    let resolver = create_resolver(config);

    // Resolve in parallel — shared resolver instance
    modules
        .par_iter()
        .filter_map(|module| {
            let file_path = match file_id_to_path.get(&module.file_id) {
                Some(p) => p,
                None => {
                    tracing::warn!(
                        file_id = module.file_id.0,
                        "Skipping module with unknown file_id during resolution"
                    );
                    return None;
                }
            };

            let resolved_imports: Vec<ResolvedImport> = module
                .imports
                .iter()
                .map(|imp| ResolvedImport {
                    info: imp.clone(),
                    target: resolve_specifier(&resolver, file_path, &imp.source, &path_to_id),
                })
                .collect();

            let resolved_dynamic_imports: Vec<ResolvedImport> = module
                .dynamic_imports
                .iter()
                .map(|imp| ResolvedImport {
                    info: ImportInfo {
                        source: imp.source.clone(),
                        imported_name: crate::extract::ImportedName::SideEffect,
                        local_name: String::new(),
                        is_type_only: false,
                        span: imp.span,
                    },
                    target: resolve_specifier(&resolver, file_path, &imp.source, &path_to_id),
                })
                .collect();

            let re_exports: Vec<ResolvedReExport> = module
                .re_exports
                .iter()
                .map(|re| ResolvedReExport {
                    info: re.clone(),
                    target: resolve_specifier(&resolver, file_path, &re.source, &path_to_id),
                })
                .collect();

            // Also resolve require() calls
            let require_imports: Vec<ResolvedImport> = module
                .require_calls
                .iter()
                .map(|req| ResolvedImport {
                    info: ImportInfo {
                        source: req.source.clone(),
                        imported_name: crate::extract::ImportedName::SideEffect,
                        local_name: String::new(),
                        is_type_only: false,
                        span: req.span,
                    },
                    target: resolve_specifier(&resolver, file_path, &req.source, &path_to_id),
                })
                .collect();

            let mut all_imports = resolved_imports;
            all_imports.extend(require_imports);

            Some(ResolvedModule {
                file_id: module.file_id,
                path: file_path.clone(),
                exports: module.exports.clone(),
                re_exports,
                resolved_imports: all_imports,
                resolved_dynamic_imports,
                member_accesses: module.member_accesses.clone(),
                has_cjs_exports: module.has_cjs_exports,
            })
        })
        .collect()
}

/// Create an oxc_resolver instance with standard configuration.
fn create_resolver(config: &ResolvedConfig) -> Resolver {
    let mut options = ResolveOptions {
        extensions: vec![
            ".ts".into(),
            ".tsx".into(),
            ".mts".into(),
            ".cts".into(),
            ".js".into(),
            ".jsx".into(),
            ".mjs".into(),
            ".cjs".into(),
            ".json".into(),
        ],
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
        condition_names: vec![
            "import".into(),
            "require".into(),
            "default".into(),
            "types".into(),
            "node".into(),
        ],
        main_fields: vec!["module".into(), "main".into()],
        ..Default::default()
    };

    // Auto-detect tsconfig.json (check common variants at project root)
    let tsconfig_candidates = ["tsconfig.json", "tsconfig.app.json", "tsconfig.build.json"];
    let root_tsconfig = tsconfig_candidates
        .iter()
        .map(|name| config.root.join(name))
        .find(|p| p.exists());

    if let Some(tsconfig) = root_tsconfig {
        // Use manual config with auto references to also discover workspace tsconfigs
        options.tsconfig = Some(oxc_resolver::TsconfigDiscovery::Manual(
            oxc_resolver::TsconfigOptions {
                config_file: tsconfig,
                references: oxc_resolver::TsconfigReferences::Auto,
            },
        ));
    } else {
        // No root tsconfig found — use auto-discovery mode so oxc_resolver
        // can find the nearest tsconfig.json for each file (important for
        // workspace packages that have their own tsconfig)
        options.tsconfig = Some(oxc_resolver::TsconfigDiscovery::Auto);
    }

    Resolver::new(options)
}

/// Resolve a single import specifier to a target.
fn resolve_specifier(
    resolver: &Resolver,
    from_file: &Path,
    specifier: &str,
    path_to_id: &HashMap<PathBuf, FileId>,
) -> ResolveResult {
    let dir = from_file.parent().unwrap_or(from_file);

    match resolver.resolve(dir, specifier) {
        Ok(resolved) => {
            let resolved_path = resolved.path();
            match resolved_path.canonicalize() {
                Ok(canonical) => {
                    if let Some(&file_id) = path_to_id.get(&canonical) {
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
                    if let Some(pkg_name) =
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
            if is_bare_specifier(specifier) {
                let pkg_name = extract_package_name(specifier);
                ResolveResult::NpmPackage(pkg_name)
            } else {
                ResolveResult::Unresolvable(specifier.to_string())
            }
        }
    }
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

/// Check if a specifier is a bare specifier (npm package or Node.js imports map entry).
fn is_bare_specifier(specifier: &str) -> bool {
    !specifier.starts_with('.') && !specifier.starts_with('/')
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
}
