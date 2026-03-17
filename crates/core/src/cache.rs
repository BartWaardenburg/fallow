use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use oxc_span::Span;

use crate::extract::{ExportName, MemberAccess, MemberKind};

/// Cache version — bump when the cache format changes.
const CACHE_VERSION: u32 = 2;

/// Maximum cache file size to deserialize (256 MB).
const MAX_CACHE_SIZE: usize = 256 * 1024 * 1024;

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
        if data.len() > MAX_CACHE_SIZE {
            tracing::warn!(
                size_mb = data.len() / (1024 * 1024),
                "Cache file exceeds size limit, ignoring"
            );
            return None;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::FileId;
    use crate::extract::*;

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
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            has_cjs_exports: false,
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
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            has_cjs_exports: false,
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
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            has_cjs_exports: false,
        };
        let m2 = CachedModule {
            content_hash: 2,
            exports: vec![],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            has_cjs_exports: false,
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
            has_cjs_exports: false,
            content_hash: 123,
        };

        let cached = module_to_cached(&module);
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
            has_cjs_exports: false,
            content_hash: 456,
        };

        let cached = module_to_cached(&module);
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
            has_cjs_exports: false,
            content_hash: 789,
        };

        let cached = module_to_cached(&module);
        let restored = cached_to_module(&cached, FileId(0));

        assert_eq!(restored.imports.len(), 4);
        assert_eq!(
            restored.imports[0].imported_name,
            ImportedName::Named("foo".to_string())
        );
        assert_eq!(restored.imports[1].imported_name, ImportedName::Default);
        assert_eq!(restored.imports[2].imported_name, ImportedName::Namespace);
        assert_eq!(restored.imports[3].imported_name, ImportedName::SideEffect);
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
            has_cjs_exports: false,
            content_hash: 0,
        };

        let cached = module_to_cached(&module);
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
            }],
            require_calls: vec![RequireCallInfo {
                source: "fs".to_string(),
                span: Span::new(15, 25),
            }],
            member_accesses: vec![MemberAccess {
                object: "Status".to_string(),
                member: "Active".to_string(),
            }],
            has_cjs_exports: true,
            content_hash: 0,
        };

        let cached = module_to_cached(&module);
        let restored = cached_to_module(&cached, FileId(0));

        assert_eq!(restored.dynamic_imports.len(), 1);
        assert_eq!(restored.dynamic_imports[0].source, "./lazy");
        assert_eq!(restored.require_calls.len(), 1);
        assert_eq!(restored.require_calls[0].source, "fs");
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
                    },
                    MemberInfo {
                        name: "greet".to_string(),
                        kind: MemberKind::ClassMethod,
                        span: Span::new(20, 30),
                    },
                    MemberInfo {
                        name: "name".to_string(),
                        kind: MemberKind::ClassProperty,
                        span: Span::new(35, 45),
                    },
                ],
            }],
            imports: vec![],
            re_exports: vec![],
            dynamic_imports: vec![],
            require_calls: vec![],
            member_accesses: vec![],
            has_cjs_exports: false,
            content_hash: 0,
        };

        let cached = module_to_cached(&module);
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
            has_cjs_exports: false,
            content_hash: 0,
        };

        let cached = module_to_cached(&module);
        let restored = cached_to_module(&cached, FileId(0));

        assert!(restored.imports[0].is_type_only);
    }
}
