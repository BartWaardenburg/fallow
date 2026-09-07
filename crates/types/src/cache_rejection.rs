//! Why a persisted cache was not reused.
//!
//! Both persistent caches (the extraction blob in `fallow-extract` and the
//! module-graph blob in `fallow-graph`) used to collapse every refusal into
//! `None`, so a run that paid full deserialisation cost and then reused
//! nothing looked exactly like a run with no cache at all. The reason is a
//! measurement, not an internal detail: it decides whether a user should fix a
//! config drift, delete a corrupt blob, or accept a legitimate cold run.
//!
//! The variants split by WHO decided. `Absent` through `RootMismatch` are
//! decided inside a loader, before it hands a store back. `ModeMismatch`
//! through `FingerprintChanged` are decided by the caller after the load
//! succeeded, which is exactly the case that costs the most and used to say
//! the least.

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::Serialize;

/// Why a cache load or a cache comparison refused to reuse persisted work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(tag = "reason", rename_all = "kebab-case")]
pub enum CacheRejection {
    /// No cache file exists yet. The only variant that is not a refusal of
    /// existing work: a first run on a project reports this.
    Absent,
    /// The cache file is larger than the safety ceiling, so it was never
    /// decoded. Reported with both figures so the operator can raise the
    /// configured ceiling or delete the blob.
    Oversize {
        /// On-disk size of the refused cache file in bytes.
        size_bytes: u64,
        /// Ceiling the file exceeded, in bytes.
        ceiling_bytes: u64,
    },
    /// The cache file exists but could not be decoded. Usually a blob written
    /// by a different binary, occasionally a truncated write.
    Undecodable,
    /// The decoded cache declares a different format version, so its entries
    /// cannot be read into the current shape.
    VersionMismatch,
    /// The cache was built under a different extraction-affecting config, so
    /// its entries describe a different analysis.
    ConfigHashMismatch,
    /// The cache was built for a different project root. Entries are stored
    /// root-relative, so a blob copied or restored under another root is
    /// refused wholesale instead of missing silently on every lookup.
    RootMismatch,
    /// The graph cache decoded, but it was built with different resolver
    /// options, entry points, or plugin configuration.
    ModeMismatch,
    /// The graph cache decoded, but the set of analysed files changed.
    FileSetChanged,
    /// The graph cache decoded and covers the same files, but at least one
    /// file's content changed.
    FingerprintChanged,
}

impl CacheRejection {
    /// Stable kebab-case identifier for logs, doctor output, and tests.
    #[must_use]
    pub const fn id(&self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Oversize { .. } => "oversize",
            Self::Undecodable => "undecodable",
            Self::VersionMismatch => "version-mismatch",
            Self::ConfigHashMismatch => "config-hash-mismatch",
            Self::RootMismatch => "root-mismatch",
            Self::ModeMismatch => "mode-mismatch",
            Self::FileSetChanged => "file-set-changed",
            Self::FingerprintChanged => "fingerprint-changed",
        }
    }

    /// Short human sentence fragment, suitable inside a perf row or a doctor
    /// message. Never contains a host path.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Absent => "no cache file yet".to_string(),
            Self::Oversize {
                size_bytes,
                ceiling_bytes,
            } => format!(
                "cache file is {} over the {} ceiling",
                format_mb(*size_bytes),
                format_mb(*ceiling_bytes)
            ),
            Self::Undecodable => "cache file could not be decoded".to_string(),
            Self::VersionMismatch => "cache format version changed".to_string(),
            Self::ConfigHashMismatch => "extraction config changed".to_string(),
            Self::RootMismatch => "cache was written for a different project root".to_string(),
            Self::ModeMismatch => "resolver, entry points, or plugins changed".to_string(),
            Self::FileSetChanged => "the analysed file set changed".to_string(),
            Self::FingerprintChanged => "at least one file changed".to_string(),
        }
    }

    /// Whether a cache file was actually present and then refused.
    ///
    /// A refusal of existing work is worth a warning: the user paid for the
    /// blob and got nothing back. A missing file is normal and stays at info.
    #[must_use]
    pub const fn discarded_existing_work(&self) -> bool {
        !matches!(self, Self::Absent)
    }
}

/// Render a byte count as a megabyte figure with one decimal place.
fn format_mb(bytes: u64) -> String {
    #[expect(
        clippy::cast_precision_loss,
        reason = "display-only size figure; precision loss past 2^53 bytes is irrelevant"
    )]
    let mb = bytes as f64 / (1024.0 * 1024.0);
    format!("{mb:.1} MB")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_is_the_only_rejection_that_discarded_nothing() {
        assert!(!CacheRejection::Absent.discarded_existing_work());
        for rejection in [
            CacheRejection::Undecodable,
            CacheRejection::VersionMismatch,
            CacheRejection::ConfigHashMismatch,
            CacheRejection::RootMismatch,
            CacheRejection::ModeMismatch,
            CacheRejection::FileSetChanged,
            CacheRejection::FingerprintChanged,
        ] {
            assert!(
                rejection.discarded_existing_work(),
                "{} refused a cache file that existed",
                rejection.id()
            );
        }
    }

    #[test]
    fn oversize_names_both_figures() {
        let described = CacheRejection::Oversize {
            size_bytes: 300 * 1024 * 1024,
            ceiling_bytes: 256 * 1024 * 1024,
        }
        .describe();
        assert!(described.contains("300.0 MB"), "{described}");
        assert!(described.contains("256.0 MB"), "{described}");
    }
}
