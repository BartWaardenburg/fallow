//! Resolution of static ES module imports (`import x from './y'`).

use std::path::Path;

use fallow_types::extract::ImportInfo;

use super::specifier::resolve_specifier;
use super::types::ResolveContext;
use super::ResolvedImport;

/// Resolve standard ES module imports (`import x from './y'`).
pub(super) fn resolve_static_imports(
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
