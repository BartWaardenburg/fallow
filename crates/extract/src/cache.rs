//! Incremental parse cache with bincode serialization.
//!
//! Stores parsed module information (exports, imports, re-exports) on disk so
//! unchanged files can skip AST parsing on subsequent runs. Uses xxh3 content
//! hashing to detect changes.

use std::path::Path;

use rustc_hash::FxHashMap;

use bincode::{Decode, Encode};

use oxc_span::Span;

use crate::{ExportName, MemberAccess, MemberKind};

/// Cache version — bump when the cache format changes.
const CACHE_VERSION: u32 = 12;

/// Maximum cache file size to deserialize (256 MB).
const MAX_CACHE_SIZE: usize = 256 * 1024 * 1024;

/// Cached module information stored on disk.
#[derive(Debug, Encode, Decode)]
pub struct CacheStore {
    version: u32,
    /// Map from file path to cached module data.
    entries: FxHashMap<String, CachedModule>,
}

/// Cached data for a single module.
#[derive(Debug, Clone, Encode, Decode)]
pub struct CachedModule {
    /// xxh3 hash of the file content.
    pub content_hash: u64,
    /// File modification time (seconds since epoch) for fast cache validation.
    /// When mtime+size match the on-disk file, we skip reading file content entirely.
    pub mtime_secs: u64,
    /// File size in bytes for fast cache validation.
    pub file_size: u64,
    /// Exported symbols.
    pub exports: Vec<CachedExport>,
    /// Import specifiers.
    pub imports: Vec<CachedImport>,
    /// Re-export specifiers.
    pub re_exports: Vec<CachedReExport>,
    /// Dynamic import specifiers.
    pub dynamic_imports: Vec<CachedDynamicImport>,
    /// `require()` specifiers.
    pub require_calls: Vec<CachedRequireCall>,
    /// Static member accesses (e.g., `Status.Active`).
    pub member_accesses: Vec<MemberAccess>,
    /// Identifiers used as whole objects (Object.values, for..in, spread, etc.).
    pub whole_object_uses: Vec<String>,
    /// Dynamic import patterns with partial static resolution.
    pub dynamic_import_patterns: Vec<CachedDynamicImportPattern>,
    /// Whether this module uses CJS exports.
    pub has_cjs_exports: bool,
    /// Local names of import bindings that are never referenced in this file.
    pub unused_import_bindings: Vec<String>,
    /// Inline suppression directives.
    pub suppressions: Vec<CachedSuppression>,
    /// Pre-computed line-start byte offsets for O(log N) byte-to-line/col conversion.
    pub line_offsets: Vec<u32>,
}

/// Cached suppression directive.
#[derive(Debug, Clone, Encode, Decode)]
pub struct CachedSuppression {
    /// 1-based line this suppression applies to. 0 = file-wide.
    pub line: u32,
    /// 0 = suppress all, 1-10 = `IssueKind` discriminant.
    pub kind: u8,
}

/// Cached export data for a single export declaration.
#[derive(Debug, Clone, Encode, Decode)]
pub struct CachedExport {
    /// Export name (or "default" for default exports).
    pub name: String,
    /// Whether this is a default export.
    pub is_default: bool,
    /// Whether this is a type-only export.
    pub is_type_only: bool,
    /// The local binding name, if different.
    pub local_name: Option<String>,
    /// Byte offset of the export span start.
    pub span_start: u32,
    /// Byte offset of the export span end.
    pub span_end: u32,
    /// Members of this export (for enums and classes).
    pub members: Vec<CachedMember>,
}

/// Import kind discriminant for `CachedImport`:
/// 0 = Named, 1 = Default, 2 = Namespace, 3 = `SideEffect`.
const IMPORT_KIND_NAMED: u8 = 0;
const IMPORT_KIND_DEFAULT: u8 = 1;
const IMPORT_KIND_NAMESPACE: u8 = 2;
const IMPORT_KIND_SIDE_EFFECT: u8 = 3;

/// Cached import data for a single import declaration.
#[derive(Debug, Clone, Encode, Decode)]
pub struct CachedImport {
    /// The import specifier.
    pub source: String,
    /// For Named imports, the imported symbol name. Empty for other kinds.
    pub imported_name: String,
    /// The local binding name.
    pub local_name: String,
    /// Whether this is a type-only import.
    pub is_type_only: bool,
    /// Import kind: 0=Named, 1=Default, 2=Namespace, 3=SideEffect.
    pub kind: u8,
    /// Byte offset of the import span start.
    pub span_start: u32,
    /// Byte offset of the import span end.
    pub span_end: u32,
}

/// Cached dynamic import data.
#[derive(Debug, Clone, Encode, Decode)]
pub struct CachedDynamicImport {
    /// The import specifier.
    pub source: String,
    /// Byte offset of the span start.
    pub span_start: u32,
    /// Byte offset of the span end.
    pub span_end: u32,
    /// Names destructured from the import result.
    pub destructured_names: Vec<String>,
    /// Local variable name for namespace imports.
    pub local_name: Option<String>,
}

/// Cached `require()` call data.
#[derive(Debug, Clone, Encode, Decode)]
pub struct CachedRequireCall {
    /// The require specifier.
    pub source: String,
    /// Byte offset of the span start.
    pub span_start: u32,
    /// Byte offset of the span end.
    pub span_end: u32,
    /// Names destructured from the require result.
    pub destructured_names: Vec<String>,
    /// Local variable name for namespace requires.
    pub local_name: Option<String>,
}

/// Cached re-export data.
#[derive(Debug, Clone, Encode, Decode)]
pub struct CachedReExport {
    /// The module being re-exported from.
    pub source: String,
    /// Name imported from the source.
    pub imported_name: String,
    /// Name exported from this module.
    pub exported_name: String,
    /// Whether this is a type-only re-export.
    pub is_type_only: bool,
}

/// Cached enum or class member data.
#[derive(Debug, Clone, Encode, Decode)]
pub struct CachedMember {
    /// Member name.
    pub name: String,
    /// Member kind (enum, method, or property).
    pub kind: MemberKind,
    /// Byte offset of the span start.
    pub span_start: u32,
    /// Byte offset of the span end.
    pub span_end: u32,
    /// Whether this member has decorators.
    pub has_decorator: bool,
}

/// Cached dynamic import pattern data (template literals, `import.meta.glob`).
#[derive(Debug, Clone, Encode, Decode)]
pub struct CachedDynamicImportPattern {
    /// Static prefix of the import path.
    pub prefix: String,
    /// Static suffix, if any.
    pub suffix: Option<String>,
    /// Byte offset of the span start.
    pub span_start: u32,
    /// Byte offset of the span end.
    pub span_end: u32,
}

impl CacheStore {
    /// Create a new empty cache.
    pub fn new() -> Self {
        Self {
            version: CACHE_VERSION,
            entries: FxHashMap::default(),
        }
    }

    /// Load cache from disk.
    pub fn load(cache_dir: &Path) -> Option<Self> {
        let cache_file = cache_dir.join("cache.bin");
        let data = std::fs::read(&cache_file).ok()?;
        if data.len() > MAX_CACHE_SIZE {
            tracing::warn!(
                size_mb = data.len() / (1024 * 1024),
                "Cache file exceeds size limit, ignoring"
            );
            return None;
        }
        let (store, _): (Self, usize) =
            bincode::decode_from_slice(&data, bincode::config::standard()).ok()?;
        if store.version != CACHE_VERSION {
            return None;
        }
        Some(store)
    }

    /// Save cache to disk.
    pub fn save(&self, cache_dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(cache_dir)
            .map_err(|e| format!("Failed to create cache dir: {e}"))?;
        let cache_file = cache_dir.join("cache.bin");
        let data = bincode::encode_to_vec(self, bincode::config::standard())
            .map_err(|e| format!("Failed to serialize cache: {e}"))?;
        std::fs::write(&cache_file, data).map_err(|e| format!("Failed to write cache: {e}"))?;
        Ok(())
    }

    /// Look up a cached module by path and content hash.
    /// Returns None if not cached or hash mismatch.
    pub fn get(&self, path: &Path, content_hash: u64) -> Option<&CachedModule> {
        let key = path.to_string_lossy().to_string();
        let entry = self.entries.get(&key)?;
        if entry.content_hash == content_hash {
            Some(entry)
        } else {
            None
        }
    }

    /// Insert or update a cached module.
    pub fn insert(&mut self, path: &Path, module: CachedModule) {
        let key = path.to_string_lossy().to_string();
        self.entries.insert(key, module);
    }

    /// Fast cache lookup using only file metadata (mtime + size).
    ///
    /// If the cached entry has matching mtime and size, the file content
    /// almost certainly has not changed, so we can skip reading the file
    /// entirely. This turns a cache hit from `stat() + read() + hash`
    /// into just `stat()`.
    pub fn get_by_metadata(
        &self,
        path: &Path,
        mtime_secs: u64,
        file_size: u64,
    ) -> Option<&CachedModule> {
        let key = path.to_string_lossy().to_string();
        let entry = self.entries.get(&key)?;
        if entry.mtime_secs == mtime_secs && entry.file_size == file_size && mtime_secs > 0 {
            Some(entry)
        } else {
            None
        }
    }

    /// Look up a cached module by path only (ignoring hash).
    /// Used to check whether a module's content hash matches without
    /// requiring the caller to know the hash upfront.
    pub fn get_by_path_only(&self, path: &Path) -> Option<&CachedModule> {
        let key = path.to_string_lossy().to_string();
        self.entries.get(&key)
    }

    /// Remove cache entries for files that are no longer in the project.
    /// Keeps the cache from growing unboundedly as files are deleted.
    pub fn retain_paths(&mut self, files: &[fallow_types::discover::DiscoveredFile]) {
        use rustc_hash::FxHashSet;
        let current_paths: FxHashSet<String> = files
            .iter()
            .map(|f| f.path.to_string_lossy().to_string())
            .collect();
        self.entries.retain(|key, _| current_paths.contains(key));
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for CacheStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Reconstruct a [`ModuleInfo`](crate::ModuleInfo) from a [`CachedModule`].
pub fn cached_to_module(
    cached: &CachedModule,
    file_id: fallow_types::discover::FileId,
) -> crate::ModuleInfo {
    use crate::*;

    let exports = cached
        .exports
        .iter()
        .map(|e| ExportInfo {
            name: if e.is_default {
                ExportName::Default
            } else {
                ExportName::Named(e.name.clone())
            },
            local_name: e.local_name.clone(),
            is_type_only: e.is_type_only,
            span: Span::new(e.span_start, e.span_end),
            members: e
                .members
                .iter()
                .map(|m| MemberInfo {
                    name: m.name.clone(),
                    kind: m.kind.clone(),
                    span: Span::new(m.span_start, m.span_end),
                    has_decorator: m.has_decorator,
                })
                .collect(),
        })
        .collect();

    let imports = cached
        .imports
        .iter()
        .map(|i| ImportInfo {
            source: i.source.clone(),
            imported_name: match i.kind {
                IMPORT_KIND_DEFAULT => ImportedName::Default,
                IMPORT_KIND_NAMESPACE => ImportedName::Namespace,
                IMPORT_KIND_SIDE_EFFECT => ImportedName::SideEffect,
                // IMPORT_KIND_NAMED (0) and any unknown value default to Named
                _ => ImportedName::Named(i.imported_name.clone()),
            },
            local_name: i.local_name.clone(),
            is_type_only: i.is_type_only,
            span: Span::new(i.span_start, i.span_end),
        })
        .collect();

    let re_exports = cached
        .re_exports
        .iter()
        .map(|r| ReExportInfo {
            source: r.source.clone(),
            imported_name: r.imported_name.clone(),
            exported_name: r.exported_name.clone(),
            is_type_only: r.is_type_only,
        })
        .collect();

    let dynamic_imports = cached
        .dynamic_imports
        .iter()
        .map(|d| DynamicImportInfo {
            source: d.source.clone(),
            span: Span::new(d.span_start, d.span_end),
            destructured_names: d.destructured_names.clone(),
            local_name: d.local_name.clone(),
        })
        .collect();

    let require_calls = cached
        .require_calls
        .iter()
        .map(|r| RequireCallInfo {
            source: r.source.clone(),
            span: Span::new(r.span_start, r.span_end),
            destructured_names: r.destructured_names.clone(),
            local_name: r.local_name.clone(),
        })
        .collect();

    let dynamic_import_patterns = cached
        .dynamic_import_patterns
        .iter()
        .map(|p| crate::DynamicImportPattern {
            prefix: p.prefix.clone(),
            suffix: p.suffix.clone(),
            span: Span::new(p.span_start, p.span_end),
        })
        .collect();

    let suppressions = cached
        .suppressions
        .iter()
        .map(|s| crate::suppress::Suppression {
            line: s.line,
            kind: if s.kind == 0 {
                None
            } else {
                crate::suppress::IssueKind::from_discriminant(s.kind)
            },
        })
        .collect();

    ModuleInfo {
        file_id,
        exports,
        imports,
        re_exports,
        dynamic_imports,
        dynamic_import_patterns,
        require_calls,
        member_accesses: cached.member_accesses.clone(),
        whole_object_uses: cached.whole_object_uses.clone(),
        has_cjs_exports: cached.has_cjs_exports,
        content_hash: cached.content_hash,
        suppressions,
        unused_import_bindings: cached.unused_import_bindings.clone(),
        line_offsets: cached.line_offsets.clone(),
    }
}

/// Convert a [`ModuleInfo`](crate::ModuleInfo) to a [`CachedModule`] for storage.
///
/// `mtime_secs` and `file_size` come from `std::fs::metadata()` at parse time
/// and enable fast cache validation on subsequent runs (skip file read when
/// mtime+size match).
pub fn module_to_cached(
    module: &crate::ModuleInfo,
    mtime_secs: u64,
    file_size: u64,
) -> CachedModule {
    CachedModule {
        content_hash: module.content_hash,
        mtime_secs,
        file_size,
        exports: module
            .exports
            .iter()
            .map(|e| CachedExport {
                name: match &e.name {
                    ExportName::Named(n) => n.clone(),
                    ExportName::Default => String::new(),
                },
                is_default: matches!(e.name, ExportName::Default),
                is_type_only: e.is_type_only,
                local_name: e.local_name.clone(),
                span_start: e.span.start,
                span_end: e.span.end,
                members: e
                    .members
                    .iter()
                    .map(|m| CachedMember {
                        name: m.name.clone(),
                        kind: m.kind.clone(),
                        span_start: m.span.start,
                        span_end: m.span.end,
                        has_decorator: m.has_decorator,
                    })
                    .collect(),
            })
            .collect(),
        imports: module
            .imports
            .iter()
            .map(|i| {
                let (kind, imported_name) = match &i.imported_name {
                    crate::ImportedName::Named(n) => (IMPORT_KIND_NAMED, n.clone()),
                    crate::ImportedName::Default => (IMPORT_KIND_DEFAULT, String::new()),
                    crate::ImportedName::Namespace => (IMPORT_KIND_NAMESPACE, String::new()),
                    crate::ImportedName::SideEffect => (IMPORT_KIND_SIDE_EFFECT, String::new()),
                };
                CachedImport {
                    source: i.source.clone(),
                    imported_name,
                    local_name: i.local_name.clone(),
                    is_type_only: i.is_type_only,
                    kind,
                    span_start: i.span.start,
                    span_end: i.span.end,
                }
            })
            .collect(),
        re_exports: module
            .re_exports
            .iter()
            .map(|r| CachedReExport {
                source: r.source.clone(),
                imported_name: r.imported_name.clone(),
                exported_name: r.exported_name.clone(),
                is_type_only: r.is_type_only,
            })
            .collect(),
        dynamic_imports: module
            .dynamic_imports
            .iter()
            .map(|d| CachedDynamicImport {
                source: d.source.clone(),
                span_start: d.span.start,
                span_end: d.span.end,
                destructured_names: d.destructured_names.clone(),
                local_name: d.local_name.clone(),
            })
            .collect(),
        require_calls: module
            .require_calls
            .iter()
            .map(|r| CachedRequireCall {
                source: r.source.clone(),
                span_start: r.span.start,
                span_end: r.span.end,
                destructured_names: r.destructured_names.clone(),
                local_name: r.local_name.clone(),
            })
            .collect(),
        member_accesses: module.member_accesses.clone(),
        whole_object_uses: module.whole_object_uses.clone(),
        dynamic_import_patterns: module
            .dynamic_import_patterns
            .iter()
            .map(|p| CachedDynamicImportPattern {
                prefix: p.prefix.clone(),
                suffix: p.suffix.clone(),
                span_start: p.span.start,
                span_end: p.span.end,
            })
            .collect(),
        has_cjs_exports: module.has_cjs_exports,
        unused_import_bindings: module.unused_import_bindings.clone(),
        suppressions: module
            .suppressions
            .iter()
            .map(|s| CachedSuppression {
                line: s.line,
                kind: s.kind.map_or(0, |k| k.to_discriminant()),
            })
            .collect(),
        line_offsets: module.line_offsets.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use fallow_types::discover::FileId;

    #[test]
    fn cache_store_new_is_empty() {
        let store = CacheStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn cache_store_default_is_empty() {
        let store = CacheStore::default();
        assert!(store.is_empty());
    }

    #[test]
    fn cache_store_insert_and_get() {
        let mut store = CacheStore::new();
        let module = CachedModule {
            content_hash: 42,
            mtime_secs: 0,
            file_size: 0,
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            suppressions: vec![],
            line_offsets: vec![],
        };
        store.insert(Path::new("test.ts"), module);
        assert_eq!(store.len(), 1);
        assert!(!store.is_empty());
        assert!(store.get(Path::new("test.ts"), 42).is_some());
    }

    #[test]
    fn cache_store_hash_mismatch_returns_none() {
        let mut store = CacheStore::new();
        let module = CachedModule {
            content_hash: 42,
            mtime_secs: 0,
            file_size: 0,
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            suppressions: vec![],
            line_offsets: vec![],
        };
        store.insert(Path::new("test.ts"), module);
        assert!(store.get(Path::new("test.ts"), 99).is_none());
    }

    #[test]
    fn cache_store_missing_key_returns_none() {
        let store = CacheStore::new();
        assert!(store.get(Path::new("nonexistent.ts"), 42).is_none());
    }

    #[test]
    fn cache_store_overwrite_entry() {
        let mut store = CacheStore::new();
        let m1 = CachedModule {
            content_hash: 1,
            mtime_secs: 0,
            file_size: 0,
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            suppressions: vec![],
            line_offsets: vec![],
        };
        let m2 = CachedModule {
            content_hash: 2,
            mtime_secs: 0,
            file_size: 0,
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            suppressions: vec![],
            line_offsets: vec![],
        };
        store.insert(Path::new("test.ts"), m1);
        store.insert(Path::new("test.ts"), m2);
        assert_eq!(store.len(), 1);
        assert!(store.get(Path::new("test.ts"), 1).is_none());
        assert!(store.get(Path::new("test.ts"), 2).is_some());
    }

    #[test]
    fn module_to_cached_roundtrip_named_export() {
        let module = ModuleInfo {
            file_id: FileId(0),
            exports: vec![ExportInfo {
                name: ExportName::Named("foo".to_string()),
                local_name: Some("foo".to_string()),
                is_type_only: false,
                span: Span::new(10, 20),
                members: vec![],
            }],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            content_hash: 123,
            suppressions: vec![],
            line_offsets: vec![],
        };

        let cached = module_to_cached(&module, 0, 0);
        let restored = cached_to_module(&cached, FileId(0));

        assert_eq!(restored.exports.len(), 1);
        assert_eq!(
            restored.exports[0].name,
            ExportName::Named("foo".to_string())
        );
        assert!(!restored.exports[0].is_type_only);
        assert_eq!(restored.exports[0].span.start, 10);
        assert_eq!(restored.exports[0].span.end, 20);
        assert_eq!(restored.content_hash, 123);
    }

    #[test]
    fn module_to_cached_roundtrip_default_export() {
        let module = ModuleInfo {
            file_id: FileId(0),
            exports: vec![ExportInfo {
                name: ExportName::Default,
                local_name: None,
                is_type_only: false,
                span: Span::new(0, 10),
                members: vec![],
            }],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            content_hash: 456,
            suppressions: vec![],
            line_offsets: vec![],
        };

        let cached = module_to_cached(&module, 0, 0);
        let restored = cached_to_module(&cached, FileId(0));

        assert_eq!(restored.exports[0].name, ExportName::Default);
    }

    #[test]
    fn module_to_cached_roundtrip_imports() {
        let module = ModuleInfo {
            file_id: FileId(0),
            exports: vec![],
            imports: vec![
                ImportInfo {
                    source: "./utils".to_string(),
                    imported_name: ImportedName::Named("foo".to_string()),
                    local_name: "foo".to_string(),
                    is_type_only: false,
                    span: Span::new(0, 10),
                },
                ImportInfo {
                    source: "react".to_string(),
                    imported_name: ImportedName::Default,
                    local_name: "React".to_string(),
                    is_type_only: false,
                    span: Span::new(15, 30),
                },
                ImportInfo {
                    source: "./all".to_string(),
                    imported_name: ImportedName::Namespace,
                    local_name: "all".to_string(),
                    is_type_only: false,
                    span: Span::new(35, 50),
                },
                ImportInfo {
                    source: "./styles.css".to_string(),
                    imported_name: ImportedName::SideEffect,
                    local_name: String::new(),
                    is_type_only: false,
                    span: Span::new(55, 70),
                },
            ],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            content_hash: 789,
            suppressions: vec![],
            line_offsets: vec![],
        };

        let cached = module_to_cached(&module, 0, 0);
        let restored = cached_to_module(&cached, FileId(0));

        assert_eq!(restored.imports.len(), 4);
        assert_eq!(
            restored.imports[0].imported_name,
            ImportedName::Named("foo".to_string())
        );
        assert_eq!(restored.imports[0].span.start, 0);
        assert_eq!(restored.imports[0].span.end, 10);
        assert_eq!(restored.imports[1].imported_name, ImportedName::Default);
        assert_eq!(restored.imports[1].span.start, 15);
        assert_eq!(restored.imports[1].span.end, 30);
        assert_eq!(restored.imports[2].imported_name, ImportedName::Namespace);
        assert_eq!(restored.imports[2].span.start, 35);
        assert_eq!(restored.imports[2].span.end, 50);
        assert_eq!(restored.imports[3].imported_name, ImportedName::SideEffect);
        assert_eq!(restored.imports[3].span.start, 55);
        assert_eq!(restored.imports[3].span.end, 70);
    }

    #[test]
    fn module_to_cached_roundtrip_re_exports() {
        let module = ModuleInfo {
            file_id: FileId(0),
            exports: vec![],
            imports: vec![],
            re_exports: vec![ReExportInfo {
                source: "./module".to_string(),
                imported_name: "foo".to_string(),
                exported_name: "bar".to_string(),
                is_type_only: true,
            }],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            content_hash: 0,
            suppressions: vec![],
            line_offsets: vec![],
        };

        let cached = module_to_cached(&module, 0, 0);
        let restored = cached_to_module(&cached, FileId(0));

        assert_eq!(restored.re_exports.len(), 1);
        assert_eq!(restored.re_exports[0].source, "./module");
        assert_eq!(restored.re_exports[0].imported_name, "foo");
        assert_eq!(restored.re_exports[0].exported_name, "bar");
        assert!(restored.re_exports[0].is_type_only);
    }

    #[test]
    fn module_to_cached_roundtrip_dynamic_imports() {
        let module = ModuleInfo {
            file_id: FileId(0),
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![DynamicImportInfo {
                source: "./lazy".to_string(),
                span: Span::new(0, 10),
                destructured_names: Vec::new(),
                local_name: None,
            }],
            require_calls: vec![RequireCallInfo {
                source: "fs".to_string(),
                span: Span::new(15, 25),
                destructured_names: Vec::new(),
                local_name: None,
            }],
            member_accesses: vec![MemberAccess {
                object: "Status".to_string(),
                member: "Active".to_string(),
            }],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: true,
            content_hash: 0,
            suppressions: vec![],
            unused_import_bindings: vec![],
            line_offsets: vec![],
        };

        let cached = module_to_cached(&module, 0, 0);
        let restored = cached_to_module(&cached, FileId(0));

        assert_eq!(restored.dynamic_imports.len(), 1);
        assert_eq!(restored.dynamic_imports[0].source, "./lazy");
        assert_eq!(restored.dynamic_imports[0].span.start, 0);
        assert_eq!(restored.dynamic_imports[0].span.end, 10);
        assert_eq!(restored.require_calls.len(), 1);
        assert_eq!(restored.require_calls[0].source, "fs");
        assert_eq!(restored.require_calls[0].span.start, 15);
        assert_eq!(restored.require_calls[0].span.end, 25);
        assert_eq!(restored.member_accesses.len(), 1);
        assert_eq!(restored.member_accesses[0].object, "Status");
        assert_eq!(restored.member_accesses[0].member, "Active");
        assert!(restored.has_cjs_exports);
    }

    #[test]
    fn module_to_cached_roundtrip_members() {
        let module = ModuleInfo {
            file_id: FileId(0),
            exports: vec![ExportInfo {
                name: ExportName::Named("Color".to_string()),
                local_name: Some("Color".to_string()),
                is_type_only: false,
                span: Span::new(0, 50),
                members: vec![
                    MemberInfo {
                        name: "Red".to_string(),
                        kind: MemberKind::EnumMember,
                        span: Span::new(10, 15),
                        has_decorator: false,
                    },
                    MemberInfo {
                        name: "greet".to_string(),
                        kind: MemberKind::ClassMethod,
                        span: Span::new(20, 30),
                        has_decorator: false,
                    },
                    MemberInfo {
                        name: "name".to_string(),
                        kind: MemberKind::ClassProperty,
                        span: Span::new(35, 45),
                        has_decorator: false,
                    },
                ],
            }],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            content_hash: 0,
            suppressions: vec![],
            line_offsets: vec![],
        };

        let cached = module_to_cached(&module, 0, 0);
        let restored = cached_to_module(&cached, FileId(0));

        assert_eq!(restored.exports[0].members.len(), 3);
        assert_eq!(restored.exports[0].members[0].kind, MemberKind::EnumMember);
        assert_eq!(restored.exports[0].members[1].kind, MemberKind::ClassMethod);
        assert_eq!(
            restored.exports[0].members[2].kind,
            MemberKind::ClassProperty
        );
    }

    #[test]
    fn cache_load_nonexistent_returns_none() {
        let result = CacheStore::load(Path::new("/nonexistent/path"));
        assert!(result.is_none());
    }

    /// Create a unique temporary directory for cache tests.
    fn test_cache_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir()
            .join("fallow_cache_tests")
            .join(name)
            .join(format!("{}", std::process::id()));
        // Clean up any leftover from previous runs
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn cache_save_and_load_roundtrip() {
        let dir = test_cache_dir("roundtrip");
        let mut store = CacheStore::new();
        let module = CachedModule {
            content_hash: 42,
            mtime_secs: 0,
            file_size: 0,
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            suppressions: vec![],
            line_offsets: vec![],
        };
        store.insert(Path::new("test.ts"), module);
        store.save(&dir).unwrap();

        let loaded = CacheStore::load(&dir);
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.len(), 1);
        assert!(loaded.get(Path::new("test.ts"), 42).is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cache_version_mismatch_returns_none() {
        let dir = test_cache_dir("version_mismatch");
        let mut store = CacheStore::new();
        let module = CachedModule {
            content_hash: 42,
            mtime_secs: 0,
            file_size: 0,
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            suppressions: vec![],
            line_offsets: vec![],
        };
        store.insert(Path::new("test.ts"), module);
        store.save(&dir).unwrap();

        // Verify the cache loads correctly before tampering
        assert!(CacheStore::load(&dir).is_some());

        // Read raw bytes and modify the version field.
        // With bincode standard config, u32 is varint-encoded.
        // The version (CACHE_VERSION) is the first encoded field.
        // Replace the first byte with a different version value (e.g., 255)
        // to simulate a version mismatch.
        let cache_file = dir.join("cache.bin");
        let mut data = std::fs::read(&cache_file).unwrap();
        assert!(!data.is_empty());
        data[0] = 255; // Corrupt the version byte
        std::fs::write(&cache_file, &data).unwrap();

        // Loading should return None due to version mismatch
        let result = CacheStore::load(&dir);
        assert!(result.is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn module_to_cached_roundtrip_type_only_import() {
        let module = ModuleInfo {
            file_id: FileId(0),
            exports: vec![],
            imports: vec![ImportInfo {
                source: "./types".to_string(),
                imported_name: ImportedName::Named("Foo".to_string()),
                local_name: "Foo".to_string(),
                is_type_only: true,
                span: Span::new(0, 10),
            }],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            content_hash: 0,
            suppressions: vec![],
            line_offsets: vec![],
        };

        let cached = module_to_cached(&module, 0, 0);
        let restored = cached_to_module(&cached, FileId(0));

        assert!(restored.imports[0].is_type_only);
        assert_eq!(restored.imports[0].span.start, 0);
        assert_eq!(restored.imports[0].span.end, 10);
    }

    #[test]
    fn get_by_path_only_returns_entry_regardless_of_hash() {
        let mut store = CacheStore::new();
        let module = CachedModule {
            content_hash: 42,
            mtime_secs: 0,
            file_size: 0,
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            suppressions: vec![],
            line_offsets: vec![],
        };
        store.insert(Path::new("test.ts"), module);

        // get_by_path_only should return the entry without checking hash
        let result = store.get_by_path_only(Path::new("test.ts"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().content_hash, 42);
    }

    #[test]
    fn get_by_path_only_returns_none_for_missing() {
        let store = CacheStore::new();
        assert!(
            store
                .get_by_path_only(Path::new("nonexistent.ts"))
                .is_none()
        );
    }

    #[test]
    fn retain_paths_removes_stale_entries() {
        use fallow_types::discover::DiscoveredFile;
        use std::path::PathBuf;

        let mut store = CacheStore::new();
        let m = || CachedModule {
            content_hash: 1,
            mtime_secs: 0,
            file_size: 0,
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            suppressions: vec![],
            line_offsets: vec![],
        };

        store.insert(Path::new("/project/a.ts"), m());
        store.insert(Path::new("/project/b.ts"), m());
        store.insert(Path::new("/project/c.ts"), m());
        assert_eq!(store.len(), 3);

        // Only a.ts and c.ts still exist in the project
        let files = vec![
            DiscoveredFile {
                id: FileId(0),
                path: PathBuf::from("/project/a.ts"),
                size_bytes: 100,
            },
            DiscoveredFile {
                id: FileId(1),
                path: PathBuf::from("/project/c.ts"),
                size_bytes: 50,
            },
        ];

        store.retain_paths(&files);
        assert_eq!(store.len(), 2);
        assert!(store.get_by_path_only(Path::new("/project/a.ts")).is_some());
        assert!(store.get_by_path_only(Path::new("/project/b.ts")).is_none());
        assert!(store.get_by_path_only(Path::new("/project/c.ts")).is_some());
    }

    #[test]
    fn retain_paths_with_empty_files_clears_cache() {
        let mut store = CacheStore::new();
        let m = CachedModule {
            content_hash: 1,
            mtime_secs: 0,
            file_size: 0,
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            suppressions: vec![],
            line_offsets: vec![],
        };
        store.insert(Path::new("a.ts"), m);
        assert_eq!(store.len(), 1);

        store.retain_paths(&[]);
        assert!(store.is_empty());
    }

    #[test]
    fn get_by_metadata_returns_entry_on_match() {
        let mut store = CacheStore::new();
        let module = CachedModule {
            content_hash: 42,
            mtime_secs: 1000,
            file_size: 500,
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            suppressions: vec![],
            line_offsets: vec![],
        };
        store.insert(Path::new("test.ts"), module);

        let result = store.get_by_metadata(Path::new("test.ts"), 1000, 500);
        assert!(result.is_some());
        assert_eq!(result.unwrap().content_hash, 42);
    }

    #[test]
    fn get_by_metadata_returns_none_on_mtime_mismatch() {
        let mut store = CacheStore::new();
        let module = CachedModule {
            content_hash: 42,
            mtime_secs: 1000,
            file_size: 500,
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            suppressions: vec![],
            line_offsets: vec![],
        };
        store.insert(Path::new("test.ts"), module);

        assert!(
            store
                .get_by_metadata(Path::new("test.ts"), 2000, 500)
                .is_none()
        );
    }

    #[test]
    fn get_by_metadata_returns_none_on_size_mismatch() {
        let mut store = CacheStore::new();
        let module = CachedModule {
            content_hash: 42,
            mtime_secs: 1000,
            file_size: 500,
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            suppressions: vec![],
            line_offsets: vec![],
        };
        store.insert(Path::new("test.ts"), module);

        assert!(
            store
                .get_by_metadata(Path::new("test.ts"), 1000, 999)
                .is_none()
        );
    }

    #[test]
    fn get_by_metadata_returns_none_for_zero_mtime() {
        let mut store = CacheStore::new();
        let module = CachedModule {
            content_hash: 42,
            mtime_secs: 0,
            file_size: 500,
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            suppressions: vec![],
            line_offsets: vec![],
        };
        store.insert(Path::new("test.ts"), module);

        // Zero mtime should never match (falls through to content hash check)
        assert!(
            store
                .get_by_metadata(Path::new("test.ts"), 0, 500)
                .is_none()
        );
    }

    #[test]
    fn get_by_metadata_returns_none_for_missing_file() {
        let store = CacheStore::new();
        assert!(
            store
                .get_by_metadata(Path::new("nonexistent.ts"), 1000, 500)
                .is_none()
        );
    }

    #[test]
    fn module_to_cached_stores_mtime_and_size() {
        let module = ModuleInfo {
            file_id: FileId(0),
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            content_hash: 42,
            suppressions: vec![],
            line_offsets: vec![],
        };

        let cached = module_to_cached(&module, 12345, 6789);
        assert_eq!(cached.mtime_secs, 12345);
        assert_eq!(cached.file_size, 6789);
        assert_eq!(cached.content_hash, 42);
    }

    #[test]
    fn module_to_cached_roundtrip_line_offsets() {
        let module = ModuleInfo {
            file_id: FileId(0),
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            whole_object_uses: vec![],
            dynamic_import_patterns: vec![],
            has_cjs_exports: false,
            unused_import_bindings: vec![],
            content_hash: 0,
            suppressions: vec![],
            line_offsets: vec![0, 15, 30, 45],
        };
        let cached = module_to_cached(&module, 0, 0);
        let restored = cached_to_module(&cached, FileId(0));
        assert_eq!(restored.line_offsets, vec![0, 15, 30, 45]);
    }
}
