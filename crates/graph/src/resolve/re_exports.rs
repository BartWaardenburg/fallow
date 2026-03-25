//! Resolution of re-export sources (`export { x } from './y'`).

use std::path::Path;

use fallow_types::extract::ReExportInfo;

use super::specifier::resolve_specifier;
use super::types::ResolveContext;
use super::ResolvedReExport;

/// Resolve re-export sources (`export { x } from './y'`).
pub(super) fn resolve_re_exports(
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
