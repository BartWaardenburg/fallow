//! Resolution of CommonJS `require()` calls.

use std::path::Path;

use oxc_span::Span;

use fallow_types::extract::{ImportInfo, ImportedName, RequireCallInfo};

use super::specifier::resolve_specifier;
use super::types::ResolveContext;
use super::ResolvedImport;

/// Resolve CommonJS `require()` calls.
/// Destructured requires become Named imports; others become Namespace (conservative).
pub(super) fn resolve_require_imports(
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
pub(super) fn resolve_single_require(
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
