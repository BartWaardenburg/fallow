//! Parcel plugin.
//!
//! Detects Parcel bundler projects and marks config files as always used.

use super::Plugin;

const ENABLERS: &[&str] = &["parcel", "@parcel/"];

const ALWAYS_USED: &[&str] = &[".parcelrc"];

const TOOLING_DEPENDENCIES: &[&str] = &["parcel"];

define_plugin! {
    struct ParcelPlugin => "parcel",
    enablers: ENABLERS,
    always_used: ALWAYS_USED,
    tooling_dependencies: TOOLING_DEPENDENCIES,
}
