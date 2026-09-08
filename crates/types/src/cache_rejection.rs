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
    /// The cache path exists or could not be inspected, but reading it failed.
    Unreadable,
    /// The cache file is larger than the safety ceiling, so it was never
    /// decoded. Reported with both figures so the operator can raise the
    /// configured ceiling or delete the blob.
    Oversize {
        /// On-disk size of the refused cache file in bytes.
        size_bytes: u64,
        /// Ceiling the file exceeded, in bytes.
        ceiling_bytes: u64,
    },
    /// The cache could not be decoded: an older unframed format, foreign data,
    /// or a damaged payload. This does not establish corruption.
    Undecodable,
    /// The decoded cache declares a different format version, so its entries
    /// cannot be read into the current shape.
    VersionMismatch,
    /// The cache was built under a different extraction-affecting config, so
    /// its entries describe a different analysis.
    ConfigHashMismatch,
    /// The graph cache was built for a different project root. Its retained
    /// absolute paths cannot be reused in the relocated checkout. Extraction
    /// entries remain independently reusable through their root-relative keys.
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
            Self::Unreadable => "unreadable",
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
            Self::Unreadable => {
                "cache file could not be read; check the path and permissions".to_string()
            }
            Self::Oversize {
                size_bytes,
                ceiling_bytes,
            } => format!(
                "cache file is {}, over the {} ceiling",
                format_mb(*size_bytes),
                format_mb(*ceiling_bytes)
            ),
            Self::Undecodable => {
                "cache file could not be decoded (older format or damaged data)".to_string()
            }
            Self::VersionMismatch => "cache format version changed".to_string(),
            Self::ConfigHashMismatch => "extraction config changed".to_string(),
            Self::RootMismatch => "cache was written for a different project root".to_string(),
            Self::ModeMismatch => "resolver, entry points, or plugins changed".to_string(),
            Self::FileSetChanged => "the analysed file set changed".to_string(),
            Self::FingerprintChanged => "at least one file changed".to_string(),
        }
    }

    /// Whether the refusal is worth putting on stderr.
    ///
    /// True when the cache was refused for a reason the user can act on: a
    /// stale format, a config or root drift, a blob that would not decode. The
    /// user paid for the blob and got nothing back, and something on disk or in
    /// the config has to change before the next run does better.
    ///
    /// False for `Absent` and for the two content-drift variants. Editing a
    /// file and re-running is the ordinary way to use fallow, so
    /// `FileSetChanged` and `FingerprintChanged` describe a cache doing exactly
    /// what it should: every edit-then-run cycle hit them, `fallow watch` hit
    /// them once per save, and `--quiet` did not suppress the warning. Both
    /// reasons stay on the `doctor` check and the performance table, where a
    /// reader went looking for them.
    #[must_use]
    pub const fn discarded_existing_work(&self) -> bool {
        !matches!(
            self,
            Self::Absent | Self::FileSetChanged | Self::FingerprintChanged
        )
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
    fn actionable_refusals_are_worth_a_warning() {
        for rejection in [
            CacheRejection::Oversize {
                size_bytes: 1,
                ceiling_bytes: 0,
            },
            CacheRejection::Undecodable,
            CacheRejection::Unreadable,
            CacheRejection::VersionMismatch,
            CacheRejection::ConfigHashMismatch,
            CacheRejection::RootMismatch,
            CacheRejection::ModeMismatch,
        ] {
            assert!(
                rejection.discarded_existing_work(),
                "{} needs a change on disk or in the config before the next run does better",
                rejection.id()
            );
        }
    }

    /// Editing a file and re-running is the ordinary way to use fallow, so the
    /// two content-drift reasons fired on every edit-then-run cycle and once
    /// per save under `fallow watch`. They stay in doctor and the performance
    /// table; they must not be a warning.
    #[test]
    fn a_routine_cache_miss_is_not_worth_a_warning() {
        for rejection in [
            CacheRejection::Absent,
            CacheRejection::FileSetChanged,
            CacheRejection::FingerprintChanged,
        ] {
            assert!(
                !rejection.discarded_existing_work(),
                "{} is what a cache is supposed to do after an edit",
                rejection.id()
            );
        }
    }

    /// Without the comma the sentence reads as an excess of 300 MB rather than
    /// a 300 MB file against a 256 MB ceiling.
    #[test]
    fn oversize_names_both_figures_without_reading_as_an_excess() {
        let described = CacheRejection::Oversize {
            size_bytes: 300 * 1024 * 1024,
            ceiling_bytes: 256 * 1024 * 1024,
        }
        .describe();
        assert!(
            described.contains("300.0 MB, over the 256.0 MB ceiling"),
            "{described}"
        );
    }
}
