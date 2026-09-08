//! Shared source-file fingerprint inputs for cache invalidation.

use std::fs::Metadata;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// File metadata used to decide whether a source-derived cache entry is fresh.
///
/// This is intentionally metadata-only. Callers that need content validation
/// can combine it with their existing content hash, while cheap caches can use
/// the same freshness shape without inventing their own `(mtime, size)` tuple.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceFingerprint {
    /// Source file modification time as nanoseconds since the Unix epoch.
    ///
    /// A value of `0` means the timestamp could not be read. Fast metadata-only
    /// cache hits should treat that as unknown and miss conservatively.
    pub mtime_ns: u64,
    /// Source file inode change time (ctime) as nanoseconds since the Unix
    /// epoch, or `0` when the platform does not expose it.
    ///
    /// mtime alone is writer-controlled: an editor, a `git checkout`, a
    /// codemod, or `touch -r` can restore it byte-for-byte after rewriting a
    /// file. When the replacement happens to keep the same length the
    /// `(mtime, size)` pair is unchanged and a metadata-only cache hit serves
    /// stale analysis for genuinely different content. ctime moves on every
    /// inode write and cannot be restored through the normal filesystem API,
    /// so pairing it with mtime makes the metadata-only fast path trustworthy.
    ///
    /// Only Unix reports it (`stat.st_ctime`). Windows keeps `0`, which costs
    /// the metadata-only fast path (the caller falls through to the read plus
    /// content-hash comparison and still hits) until someone wires up
    /// `FILE_BASIC_INFO.ChangeTime`.
    pub ctime_ns: u64,
    /// Source file size in bytes.
    pub file_size: u64,
}

impl SourceFingerprint {
    /// Build a fingerprint from explicit metadata parts, with no known ctime.
    ///
    /// A fingerprint built this way is never
    /// [trustworthy without content](Self::is_trustworthy_without_content).
    #[must_use]
    pub const fn new(mtime_ns: u64, file_size: u64) -> Self {
        Self {
            mtime_ns,
            ctime_ns: 0,
            file_size,
        }
    }

    /// Build a fingerprint from explicit metadata parts, including ctime.
    #[must_use]
    pub const fn with_ctime(mtime_ns: u64, ctime_ns: u64, file_size: u64) -> Self {
        Self {
            mtime_ns,
            ctime_ns,
            file_size,
        }
    }

    /// Build a fingerprint from filesystem metadata.
    #[must_use]
    pub fn from_metadata(metadata: &Metadata) -> Self {
        Self {
            mtime_ns: metadata_mtime_ns(metadata),
            ctime_ns: metadata_ctime_ns(metadata),
            file_size: metadata.len(),
        }
    }

    /// Returns true when the modification time is known.
    #[must_use]
    pub const fn has_known_mtime(self) -> bool {
        self.mtime_ns > 0
    }

    /// Returns true when this fingerprint may stand in for the file's content.
    ///
    /// Requires both timestamps: mtime detects the ordinary edit, ctime detects
    /// the same-size edit whose mtime was restored. A caller that gets `false`
    /// must fall through to reading the file and comparing content hashes.
    #[must_use]
    pub const fn is_trustworthy_without_content(self) -> bool {
        self.mtime_ns > 0 && self.ctime_ns > 0
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "filesystem mtimes used for cache invalidation fit in u64 nanoseconds for supported dates"
)]
fn metadata_mtime_ns(metadata: &Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_nanos() as u64)
}

/// Unix inode change time in nanoseconds since the epoch.
///
/// Pre-epoch and unreadable values collapse to `0`, which the fast-path gate
/// reads as "unknown" and therefore untrustworthy.
#[cfg(unix)]
fn metadata_ctime_ns(metadata: &Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;

    let seconds = u64::try_from(metadata.ctime()).unwrap_or(0);
    let nanos = u64::try_from(metadata.ctime_nsec()).unwrap_or(0);
    seconds.saturating_mul(1_000_000_000).saturating_add(nanos)
}

/// Non-Unix platforms do not expose an inode change time.
#[cfg(not(unix))]
fn metadata_ctime_ns(_metadata: &Metadata) -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_fingerprint_preserves_explicit_parts() {
        let fingerprint = SourceFingerprint::new(123, 456);
        assert_eq!(fingerprint.mtime_ns, 123);
        assert_eq!(fingerprint.file_size, 456);
        assert!(fingerprint.has_known_mtime());
    }

    #[test]
    fn source_fingerprint_zero_mtime_is_unknown() {
        let fingerprint = SourceFingerprint::new(0, 456);
        assert!(!fingerprint.has_known_mtime());
    }

    #[test]
    fn source_fingerprint_without_ctime_is_never_content_trustworthy() {
        let fingerprint = SourceFingerprint::new(123, 456);
        assert!(fingerprint.has_known_mtime());
        assert!(!fingerprint.is_trustworthy_without_content());
    }

    #[test]
    fn source_fingerprint_with_both_timestamps_is_content_trustworthy() {
        let fingerprint = SourceFingerprint::with_ctime(123, 789, 456);
        assert_eq!(fingerprint.ctime_ns, 789);
        assert!(fingerprint.is_trustworthy_without_content());
    }

    #[test]
    fn source_fingerprint_differs_when_only_ctime_moved() {
        let before = SourceFingerprint::with_ctime(123, 700, 456);
        let after = SourceFingerprint::with_ctime(123, 800, 456);
        assert_ne!(before, after);
    }

    #[test]
    #[cfg_attr(miri, ignore = "filesystem metadata is blocked by Miri isolation")]
    fn source_fingerprint_from_metadata_sets_size() {
        let metadata = std::fs::metadata(".").expect("metadata");

        let fingerprint = SourceFingerprint::from_metadata(&metadata);

        assert_eq!(fingerprint.file_size, metadata.len());
    }

    #[cfg(unix)]
    #[test]
    #[cfg_attr(miri, ignore = "filesystem metadata is blocked by Miri isolation")]
    fn source_fingerprint_from_metadata_reads_unix_ctime() {
        let metadata = std::fs::metadata(".").expect("metadata");

        let fingerprint = SourceFingerprint::from_metadata(&metadata);

        assert!(fingerprint.ctime_ns > 0);
        assert!(fingerprint.is_trustworthy_without_content());
    }
}
