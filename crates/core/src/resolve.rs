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
    pub has_cjs_exports: bool,
}

/// Resolve all imports across all modules in parallel.
pub fn resolve_all_imports(
    modules: &[ModuleInfo],
    config: &ResolvedConfig,
    files: &[DiscoveredFile],
) -> Vec<ResolvedModule> {
    // Build path -> FileId index
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

    // Resolve in parallel — each thread gets its own Resolver instance
    modules
        .par_iter()
        .map(|module| {
            let file_path = file_id_to_path
                .get(&module.file_id)
                .expect("file_id must exist");

            let resolver = create_resolver(config);

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

            ResolvedModule {
                file_id: module.file_id,
                path: file_path.clone(),
                exports: module.exports.clone(),
                re_exports,
                resolved_imports: all_imports,
                resolved_dynamic_imports,
                has_cjs_exports: module.has_cjs_exports,
            }
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

    // Configure tsconfig if available
    if let Some(tsconfig_path) = &config.tsconfig_path {
        options.tsconfig = Some(oxc_resolver::TsconfigOptions {
            config_file: tsconfig_path.clone(),
            references: oxc_resolver::TsconfigReferences::Auto,
        });
    } else {
        // Auto-detect tsconfig.json
        let tsconfig = config.root.join("tsconfig.json");
        if tsconfig.exists() {
            options.tsconfig = Some(oxc_resolver::TsconfigOptions {
                config_file: tsconfig,
                references: oxc_resolver::TsconfigReferences::Auto,
            });
        }
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
                    } else {
                        ResolveResult::ExternalFile(canonical)
                    }
                }
                Err(_) => ResolveResult::ExternalFile(resolved_path.to_path_buf()),
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

/// Check if a specifier is a bare specifier (npm package).
fn is_bare_specifier(specifier: &str) -> bool {
    !specifier.starts_with('.') && !specifier.starts_with('/') && !specifier.starts_with('#')
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
        assert!(!is_bare_specifier("./utils"));
        assert!(!is_bare_specifier("../lib"));
        assert!(!is_bare_specifier("/absolute"));
    }
}
