//! Read-only inspection of the persisted extraction cache.
//!
//! Exists for `fallow doctor`, which diagnoses project readiness without
//! running an analysis. A refused cache is invisible in every other read-only
//! surface: the run that pays for it is the one that reports it, and doctor
//! never starts one.

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

fn cache_file_size(cache_dir: &Path) -> Option<u64> {
    std::fs::metadata(cache_dir.join("cache.bin"))
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

    #[test]
    fn an_undecodable_cache_reports_its_reason_and_size() {
        let root = tempfile::tempdir().expect("temp root");
        let config = config_for(root.path(), true);
        std::fs::create_dir_all(&config.cache_dir).expect("cache dir");
        std::fs::write(config.cache_dir.join("cache.bin"), b"garbage").expect("corrupt cache");

        let status = inspect_parse_cache(&config);

        assert_eq!(status.rejection, Some(CacheRejection::Undecodable));
        assert_eq!(status.size_bytes, Some(7));
    }
}
