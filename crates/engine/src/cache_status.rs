//! Read-only inspection of the persisted caches.
//!
//! Exists for `fallow doctor`, which diagnoses project readiness without
//! running an analysis. A refused cache is invisible in every other read-only
//! surface: the run that pays for it is the one that reports it, and doctor
//! never starts one.
//!
//! Both persisted caches are inspected. A warm run reuses the extraction blob
//! and the module graph independently, and the graph blob is the larger of the
//! two on a real project, so reporting only the extraction cache told a user
//! their caches were healthy while the expensive half was being discarded on
//! every run.

use std::path::Path;

use fallow_config::ResolvedConfig;
use fallow_types::cache_rejection::CacheRejection;

/// On-disk state of the extraction cache for one project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseCacheStatus {
    /// Why the cache would not be reused by a run with this configuration, or
    /// `None` when a run would load it.
    pub rejection: Option<CacheRejection>,
    /// Size of `cache.bin` on disk, when the file exists.
    pub size_bytes: Option<u64>,
}

/// Inspect the persisted extraction cache the way an analysis run would.
///
/// This decodes the blob, which is the only way to learn the version and the
/// config hash it was written under, so the cost is that of a cache load and
/// nothing more: no analysis, no writes, no network. `config.no_cache` is
/// deliberately ignored, because the question is what state the cache is in,
/// not whether this particular invocation would consult it.
///
/// The expected config hash is recomputed rather than read off `config`:
/// `ResolvedConfig::cache_config_hash` is zero whenever caching is disabled,
/// which is exactly how a caller that only inspects resolves its config, and
/// comparing a real cache against that zero reported every healthy cache as
/// config drift.
#[must_use]
pub fn inspect_parse_cache(config: &ResolvedConfig) -> ParseCacheStatus {
    let size_bytes = cache_file_size(&config.cache_dir);
    let rejection = fallow_extract::cache::CacheStore::load(
        &config.cache_dir,
        &config.root,
        fallow_config::cache_config_hash(&config.external_plugins),
        crate::project_config::resolve_cache_max_size_bytes(config),
    )
    .err();
    ParseCacheStatus {
        rejection,
        size_bytes,
    }
}

/// On-disk state of the persisted module graph for one project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphCacheStatus {
    /// Why the persisted graph could not be loaded, or `None` when it decodes
    /// into the current shape.
    ///
    /// Loading is all this can answer. Whether a run would REUSE the graph also
    /// depends on the resolver options, entry points, and per-file
    /// fingerprints, and comparing those means running discovery and
    /// extraction, which doctor deliberately does not do.
    pub rejection: Option<CacheRejection>,
    /// Size of `graph-cache.bin` on disk, when the file exists.
    pub size_bytes: Option<u64>,
}

/// Inspect the persisted module graph the way an analysis run would load it.
///
/// Read-only: no analysis, no writes, no network. `config.no_cache` is ignored
/// for the same reason as in [`inspect_parse_cache`]: the question is what
/// state the cache is in, not whether this invocation would consult it.
#[must_use]
pub fn inspect_graph_cache(config: &ResolvedConfig) -> GraphCacheStatus {
    let size_bytes = cache_entry_size(&config.cache_dir, fallow_graph::cache::GRAPH_CACHE_FILE);
    let rejection = fallow_graph::cache::GraphCacheStore::load(&config.cache_dir).err();
    GraphCacheStatus {
        rejection,
        size_bytes,
    }
}

fn cache_file_size(cache_dir: &Path) -> Option<u64> {
    cache_entry_size(cache_dir, "cache.bin")
}

fn cache_entry_size(cache_dir: &Path, file_name: &str) -> Option<u64> {
    std::fs::metadata(cache_dir.join(file_name))
        .ok()
        .map(|metadata| metadata.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    use fallow_config::{FallowConfig, OutputFormat};

    fn config_for(root: &Path, no_cache: bool) -> ResolvedConfig {
        FallowConfig::default().resolve(
            root.to_path_buf(),
            OutputFormat::Json,
            1,
            no_cache,
            true,
            None,
        )
    }

    #[test]
    fn an_absent_cache_reports_absent_with_no_size() {
        let root = tempfile::tempdir().expect("temp root");
        let status = inspect_parse_cache(&config_for(root.path(), true));

        assert_eq!(status.rejection, Some(CacheRejection::Absent));
        assert_eq!(status.size_bytes, None);
    }

    /// A caller that only inspects resolves its config with caching disabled,
    /// which zeroes `cache_config_hash`. Comparing a real cache against that
    /// zero reported every healthy cache as config drift, so the expected hash
    /// is recomputed from the same inputs a run uses.
    #[test]
    fn a_cache_written_by_a_run_reads_as_reusable_from_an_inspecting_config() {
        let root = tempfile::tempdir().expect("temp root");
        let analysis = config_for(root.path(), false);
        let mut store = fallow_extract::cache::CacheStore::new(&analysis.root);
        store
            .save(
                &analysis.cache_dir,
                analysis.cache_config_hash,
                fallow_extract::cache::DEFAULT_CACHE_MAX_SIZE,
            )
            .expect("save cache as a run would");

        let status = inspect_parse_cache(&config_for(root.path(), true));

        assert_eq!(status.rejection, None);
        assert!(status.size_bytes.is_some_and(|bytes| bytes > 0));
    }

    /// A blob this binary cannot frame was written by a build with a different
    /// cache format. Reporting that as a decode failure told upgrading users
    /// their cache was corrupt.
    #[test]
    fn a_cache_from_another_build_reports_a_format_change_with_its_size() {
        let root = tempfile::tempdir().expect("temp root");
        let config = config_for(root.path(), true);
        std::fs::create_dir_all(&config.cache_dir).expect("cache dir");
        std::fs::write(config.cache_dir.join("cache.bin"), b"garbage").expect("foreign cache");

        let status = inspect_parse_cache(&config);

        assert_eq!(status.rejection, Some(CacheRejection::VersionMismatch));
        assert_eq!(status.size_bytes, Some(7));
    }

    #[test]
    fn an_absent_graph_cache_reports_absent_with_no_size() {
        let root = tempfile::tempdir().expect("temp root");
        let status = inspect_graph_cache(&config_for(root.path(), true));

        assert_eq!(status.rejection, Some(CacheRejection::Absent));
        assert_eq!(status.size_bytes, None);
    }

    #[test]
    fn a_graph_cache_from_another_build_reports_a_format_change_with_its_size() {
        let root = tempfile::tempdir().expect("temp root");
        let config = config_for(root.path(), true);
        std::fs::create_dir_all(&config.cache_dir).expect("cache dir");
        std::fs::write(
            config.cache_dir.join(fallow_graph::cache::GRAPH_CACHE_FILE),
            b"garbage",
        )
        .expect("foreign graph cache");

        let status = inspect_graph_cache(&config);

        assert_eq!(status.rejection, Some(CacheRejection::VersionMismatch));
        assert_eq!(status.size_bytes, Some(7));
    }
}
