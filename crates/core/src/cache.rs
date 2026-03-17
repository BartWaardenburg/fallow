use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use oxc_span::Span;

use crate::extract::{ExportName, MemberAccess, MemberKind};

/// Cache version — bump when the cache format changes.
const CACHE_VERSION: u32 = 2;

/// Cached module information stored on disk.
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheStore {
    version: u32,
    /// Map from file path to cached module data.
    entries: HashMap<String, CachedModule>,
}

/// Cached data for a single module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedModule {
    /// xxh3 hash of the file content.
    pub content_hash: u64,
    /// Exported symbols.
    pub exports: Vec<CachedExport>,
    /// Import specifiers.
    pub imports: Vec<CachedImport>,
    /// Re-export specifiers.
    pub re_exports: Vec<CachedReExport>,
    /// Dynamic import specifiers.
    pub dynamic_imports: Vec<String>,
    /// Require() specifiers.
    pub require_calls: Vec<String>,
    /// Static member accesses (e.g., `Status.Active`).
    pub member_accesses: Vec<MemberAccess>,
    /// Whether this module uses CJS exports.
    pub has_cjs_exports: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedExport {
    pub name: String,
    pub is_default: bool,
    pub is_type_only: bool,
    pub local_name: Option<String>,
    pub span_start: u32,
    pub span_end: u32,
    pub members: Vec<CachedMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedImport {
    pub source: String,
    pub imported_name: String,
    pub local_name: String,
    pub is_type_only: bool,
    pub is_namespace: bool,
    pub is_default: bool,
    pub is_side_effect: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedReExport {
    pub source: String,
    pub imported_name: String,
    pub exported_name: String,
    pub is_type_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedMember {
    pub name: String,
    pub kind: String,
    pub span_start: u32,
    pub span_end: u32,
}

impl CacheStore {
    /// Create a new empty cache.
    pub fn new() -> Self {
        Self {
            version: CACHE_VERSION,
            entries: HashMap::new(),
        }
    }

    /// Load cache from disk.
    pub fn load(cache_dir: &Path) -> Option<Self> {
        let cache_file = cache_dir.join("cache.bin");
        let data = std::fs::read(&cache_file).ok()?;
        let store: Self = bincode::deserialize(&data).ok()?;
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
        let data =
            bincode::serialize(self).map_err(|e| format!("Failed to serialize cache: {e}"))?;
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

/// Reconstruct a ModuleInfo from a CachedModule.
pub fn cached_to_module(
    cached: &CachedModule,
    file_id: crate::discover::FileId,
) -> crate::extract::ModuleInfo {
    use crate::extract::*;

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
                    kind: match m.kind.as_str() {
                        "enum" => MemberKind::EnumMember,
                        "method" => MemberKind::ClassMethod,
                        _ => MemberKind::ClassProperty,
                    },
                    span: Span::new(m.span_start, m.span_end),
                })
                .collect(),
        })
        .collect();

    let imports = cached
        .imports
        .iter()
        .map(|i| ImportInfo {
            source: i.source.clone(),
            imported_name: if i.is_side_effect {
                ImportedName::SideEffect
            } else if i.is_namespace {
                ImportedName::Namespace
            } else if i.is_default {
                ImportedName::Default
            } else {
                ImportedName::Named(i.imported_name.clone())
            },
            local_name: i.local_name.clone(),
            is_type_only: i.is_type_only,
            span: Span::new(0, 0),
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
        .map(|source| DynamicImportInfo {
            source: source.clone(),
            span: Span::new(0, 0),
        })
        .collect();

    let require_calls = cached
        .require_calls
        .iter()
        .map(|source| RequireCallInfo {
            source: source.clone(),
            span: Span::new(0, 0),
        })
        .collect();

    ModuleInfo {
        file_id,
        exports,
        imports,
        re_exports,
        dynamic_imports,
        require_calls,
        member_accesses: cached.member_accesses.clone(),
        has_cjs_exports: cached.has_cjs_exports,
        content_hash: cached.content_hash,
    }
}

/// Convert a ModuleInfo to a CachedModule for storage.
pub fn module_to_cached(module: &crate::extract::ModuleInfo) -> CachedModule {
    CachedModule {
        content_hash: module.content_hash,
        exports: module
            .exports
            .iter()
            .map(|e| CachedExport {
                name: match &e.name {
                    ExportName::Named(n) => n.clone(),
                    ExportName::Default => "default".to_string(),
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
                        kind: match m.kind {
                            MemberKind::EnumMember => "enum".to_string(),
                            MemberKind::ClassMethod => "method".to_string(),
                            MemberKind::ClassProperty => "property".to_string(),
                        },
                        span_start: m.span.start,
                        span_end: m.span.end,
                    })
                    .collect(),
            })
            .collect(),
        imports: module
            .imports
            .iter()
            .map(|i| CachedImport {
                source: i.source.clone(),
                imported_name: match &i.imported_name {
                    crate::extract::ImportedName::Named(n) => n.clone(),
                    crate::extract::ImportedName::Default => "default".to_string(),
                    crate::extract::ImportedName::Namespace => "*".to_string(),
                    crate::extract::ImportedName::SideEffect => "".to_string(),
                },
                local_name: i.local_name.clone(),
                is_type_only: i.is_type_only,
                is_namespace: matches!(i.imported_name, crate::extract::ImportedName::Namespace),
                is_default: matches!(i.imported_name, crate::extract::ImportedName::Default),
                is_side_effect: matches!(i.imported_name, crate::extract::ImportedName::SideEffect),
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
            .map(|d| d.source.clone())
            .collect(),
        require_calls: module
            .require_calls
            .iter()
            .map(|r| r.source.clone())
            .collect(),
        member_accesses: module.member_accesses.clone(),
        has_cjs_exports: module.has_cjs_exports,
    }
}
