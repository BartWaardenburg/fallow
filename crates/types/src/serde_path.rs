//! Custom serde serializers for `PathBuf` and `Vec<PathBuf>` that always
//! output forward slashes, regardless of platform. This ensures consistent
//! JSON/SARIF output on Windows.

use std::path::{Path, PathBuf};

use serde::Serializer;

pub fn serialize<S: Serializer>(path: &Path, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&path.to_string_lossy().replace('\\', "/"))
}

pub fn serialize_vec<S: Serializer>(paths: &[PathBuf], s: S) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeSeq;
    let mut seq = s.serialize_seq(Some(paths.len()))?;
    for p in paths {
        seq.serialize_element(&p.to_string_lossy().replace('\\', "/"))?;
    }
    seq.end()
}
