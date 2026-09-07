//! Persisted graph-cache store: coarse all-or-nothing load / save of a
//! previously-built [`ModuleGraph`].
//!
//! Mirrors the extraction cache store (`fallow_extract::cache::store`): the
//! payload is postcard-encoded, written atomically via a sibling `.tmp` file
//! plus best-effort fsync and rename, and a `.gitignore` is written alongside
//! so `.fallow/` is never committed. Every IO error is swallowed (the graph
//! cache is best-effort and must never fail analysis); a corrupt or
//! version-mismatched file misses and the graph is rebuilt fresh, but the
//! loader names WHY it missed through [`CacheRejection`] so the run can report
//! a refusal instead of leaving it indistinguishable from a first run.

use std::path::Path;

use fallow_types::cache_rejection::CacheRejection;
use serde::{Deserialize, Serialize};

use super::{CachedResolvedProject, GRAPH_CACHE_VERSION, GraphCacheManifest};
use crate::graph::ModuleGraph;

/// Filename of the persisted graph cache inside the cache directory.
const GRAPH_CACHE_FILE: &str = "graph-cache.bin";

/// On-disk graph cache entry: a manifest plus the graph it validates.
#[derive(Serialize, Deserialize)]
pub struct GraphCacheStore {
    /// Schema version. Checked on load; a mismatch misses so a stale file from
    /// an older binary is never deserialized into the wrong shape.
    pub version: u32,
    /// Inputs that must match the current run for the graph to be trusted.
    pub manifest: GraphCacheManifest,
    /// The previously-built graph. Its `namespace_imported` bitset is
    /// `#[serde(skip)]`, so the loader reconstructs it from the edge set.
    pub graph: ModuleGraph,
    /// Resolver output aligned with the cached graph. Exact manifest hits use
    /// it alongside the graph; stable-key resolver hits remap it and rebuild
    /// the graph with current `FileId`s.
    pub resolved_project: CachedResolvedProject,
}

impl GraphCacheStore {
    /// Load the persisted graph cache from `cache_dir`.
    ///
    /// # Errors
    ///
    /// Returns the [`CacheRejection`] that decided against reuse: the file is
    /// missing, undecodable, or written for a different
    /// `GRAPH_CACHE_VERSION`. The caller compares the loaded manifest against
    /// the current inputs before trusting the graph or resolver payload, and
    /// reports its own rejection reason for that comparison.
    ///
    /// A file that existed and was then refused logs at warn: the run paid the
    /// read and the decode and reused nothing. A missing file stays quiet.
    pub fn load(cache_dir: &Path) -> Result<Self, CacheRejection> {
        let cache_file = cache_dir.join(GRAPH_CACHE_FILE);
        let data = std::fs::read(&cache_file).map_err(|_| CacheRejection::Absent)?;
        let mut store: Self = match postcard::from_bytes(&data) {
            Ok(store) => store,
            Err(_) => {
                tracing::warn!(
                    "Graph cache could not be decoded, rebuilding (one-time cost after version bump)"
                );
                return Err(CacheRejection::Undecodable);
            }
        };
        if store.version != GRAPH_CACHE_VERSION {
            tracing::warn!(
                cached_version = store.version,
                expected_version = GRAPH_CACHE_VERSION,
                "Graph cache format upgraded, rebuilding (one-time cost after version bump)"
            );
            return Err(CacheRejection::VersionMismatch);
        }
        // `namespace_imported` is `#[serde(skip)]`; rebuild it from the persisted
        // edges so the loaded graph is byte-identical to a fresh build.
        store.graph.reconstruct_namespace_imported();
        Ok(store)
    }

    /// Persist this graph cache to `cache_dir`, best-effort.
    ///
    /// Creates the cache directory, writes a `.gitignore`, encodes the store
    /// with postcard, and writes `graph-cache.bin` atomically. Every IO error
    /// is logged at debug and swallowed; the graph cache must never fail the
    /// surrounding analysis run.
    pub fn save(&self, cache_dir: &Path) {
        if let Err(error) = std::fs::create_dir_all(cache_dir) {
            tracing::debug!("Failed to create graph cache dir: {error}");
            return;
        }
        if let Err(error) = write_cache_gitignore(cache_dir) {
            tracing::debug!("Failed to write graph cache .gitignore: {error}");
            // Continue: a missing .gitignore does not invalidate the cache file.
        }

        let encoded = match postcard::to_allocvec(self) {
            Ok(bytes) => bytes,
            Err(error) => {
                tracing::debug!("Failed to encode graph cache: {error}");
                return;
            }
        };

        let cache_file = cache_dir.join(GRAPH_CACHE_FILE);
        if let Err(error) = atomic_write(&cache_file, &encoded) {
            tracing::debug!("Failed to write graph cache: {error}");
        }
    }
}

/// Write `.fallow/.gitignore` (`*\n`) so the cache directory is never committed.
fn write_cache_gitignore(cache_dir: &Path) -> std::io::Result<()> {
    std::fs::write(cache_dir.join(".gitignore"), "*\n")
}

/// Write `data` atomically via a sibling `.tmp` file, best-effort fsync, then
/// rename. Copied from the extraction cache store so the two caches share the
/// same crash-safe write semantics.
fn atomic_write(cache_file: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp_file = match cache_file.file_name() {
        Some(name) => cache_file.with_file_name({
            let mut s = name.to_os_string();
            s.push(".tmp");
            s
        }),
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "graph cache file path has no filename component",
            ));
        }
    };

    {
        use std::io::Write as _;
        let mut f = std::fs::File::create(&tmp_file)?;
        f.write_all(data)?;
        let _ = f.sync_all();
    }

    std::fs::rename(&tmp_file, cache_file)
}
