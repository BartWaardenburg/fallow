//! Type definitions and constants for import resolution.

use std::path::{Path, PathBuf};

use oxc_resolver::Resolver;
use rustc_hash::FxHashMap;

use fallow_types::discover::FileId;

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
    /// The original import information.
    pub info: fallow_types::extract::ImportInfo,
    /// Where the import resolved to.
    pub target: ResolveResult,
}

/// A resolved re-export with its target.
#[derive(Debug, Clone)]
pub struct ResolvedReExport {
    /// The original re-export information.
    pub info: fallow_types::extract::ReExportInfo,
    /// Where the re-export source resolved to.
    pub target: ResolveResult,
}

/// Fully resolved module with all imports mapped to targets.
#[derive(Debug)]
pub struct ResolvedModule {
    /// Unique file identifier.
    pub file_id: FileId,
    /// Absolute path to the module file.
    pub path: PathBuf,
    /// All export declarations in this module.
    pub exports: Vec<fallow_types::extract::ExportInfo>,
    /// All re-exports with resolved targets.
    pub re_exports: Vec<ResolvedReExport>,
    /// All static imports with resolved targets.
    pub resolved_imports: Vec<ResolvedImport>,
    /// All dynamic imports with resolved targets.
    pub resolved_dynamic_imports: Vec<ResolvedImport>,
    /// Dynamic import patterns matched against discovered files.
    pub resolved_dynamic_patterns: Vec<(fallow_types::extract::DynamicImportPattern, Vec<FileId>)>,
    /// Static member accesses (e.g., `Status.Active`).
    pub member_accesses: Vec<fallow_types::extract::MemberAccess>,
    /// Identifiers used as whole objects (Object.values, for..in, spread, etc.).
    pub whole_object_uses: Vec<String>,
    /// Whether this module uses `CommonJS` exports.
    pub has_cjs_exports: bool,
    /// Local names of import bindings that are never referenced in this file.
    pub unused_import_bindings: Vec<String>,
}

/// Shared context for resolving import specifiers.
///
/// Groups the immutable lookup tables and caches that are shared across all
/// `resolve_specifier` calls within a single `resolve_all_imports` invocation.
pub(super) struct ResolveContext<'a> {
    /// The oxc_resolver instance (configured once, shared across threads).
    pub resolver: &'a Resolver,
    /// Canonical path → FileId lookup.
    pub path_to_id: &'a FxHashMap<&'a Path, FileId>,
    /// Raw (non-canonical) path → FileId lookup.
    pub raw_path_to_id: &'a FxHashMap<&'a Path, FileId>,
    /// Workspace name → canonical root path.
    pub workspace_roots: &'a FxHashMap<&'a str, &'a Path>,
    /// Plugin-provided path aliases (prefix, replacement).
    pub path_aliases: &'a [(String, String)],
    /// Project root directory.
    pub root: &'a Path,
}

/// Known output directory names that may appear in exports map targets.
/// When an exports map points to `./dist/utils.js`, we try replacing these
/// prefixes with `src/` (the conventional source directory) to find the tracked
/// source file.
pub const OUTPUT_DIRS: &[&str] = &["dist", "build", "out", "esm", "cjs"];

/// Source extensions to try when mapping a built output file back to source.
pub const SOURCE_EXTS: &[&str] = &["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"];

/// React Native platform extension prefixes.
/// Metro resolves platform-specific files (e.g., `./foo` -> `./foo.web.tsx` on web).
pub const RN_PLATFORM_PREFIXES: &[&str] = &[".web", ".ios", ".android", ".native"];
