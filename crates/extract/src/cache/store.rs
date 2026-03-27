//! Cache store: load, save, and query cached module data.

use std::path::Path;

use rustc_hash::FxHashMap;

use bincode::{Decode, Encode};

use super::types::{CACHE_VERSION, CachedModule, MAX_CACHE_SIZE};

/// Cached module information stored on disk.
#[derive(Debug, Encode, Decode)]
pub struct CacheStore {
    version: u32,
    /// Map from file path to cached module data.
    entries: FxHashMap<String, CachedModule>,
}

impl CacheStore {
    /// Create a new empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CACHE_VERSION,
            entries: FxHashMap::default(),
        }
    }

    /// Load cache from disk.
    #[must_use]
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
    ///
    /// # Errors
    ///
    /// Returns an error string when the cache directory cannot be created,
    /// the cache cannot be serialized, or the cache file cannot be written.
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for CacheStore {
    fn default() -> Self {
        Self::new()
    }
}
