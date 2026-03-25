//! Resolution of dynamic `import()` calls and dynamic import patterns.

use std::path::{Path, PathBuf};

use oxc_span::Span;

use fallow_types::discover::{DiscoveredFile, FileId};
use fallow_types::extract::{DynamicImportInfo, DynamicImportPattern, ImportInfo, ImportedName};

use super::fallbacks::make_glob_from_pattern;
use super::specifier::resolve_specifier;
use super::types::ResolveContext;
use super::ResolvedImport;

/// Resolve dynamic `import()` calls, expanding destructured names into individual imports.
pub(super) fn resolve_dynamic_imports(
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
pub(super) fn resolve_single_dynamic_import(
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

/// Resolve dynamic import patterns via glob matching against discovered files.
/// Uses pre-computed canonical paths (no syscalls in inner loop).
pub(super) fn resolve_dynamic_patterns(
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
