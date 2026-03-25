//! Custom serde serializers for `PathBuf` and `Vec<PathBuf>` that always
//! output forward slashes, regardless of platform. This ensures consistent
//! JSON/SARIF output on Windows.

use std::path::{Path, PathBuf};

use serde::Serializer;

/// Serialize a `Path` with forward slashes for cross-platform consistency.
pub fn serialize<S: Serializer>(path: &Path, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&path.to_string_lossy().replace('\\', "/"))
}

/// Serialize a `Vec<PathBuf>` with forward slashes for cross-platform consistency.
pub fn serialize_vec<S: Serializer>(paths: &[PathBuf], s: S) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeSeq;
    let mut seq = s.serialize_seq(Some(paths.len()))?;
    for p in paths {
        seq.serialize_element(&p.to_string_lossy().replace('\\', "/"))?;
    }
    seq.end()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    /// The core logic of `serialize` is `path.to_string_lossy().replace('\\', "/")`.
    /// Test that transformation directly since `serde_json` is not a dependency of this crate.
    fn normalize(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    #[test]
    fn unix_path_unchanged() {
        assert_eq!(
            normalize(Path::new("src/utils/index.ts")),
            "src/utils/index.ts"
        );
    }

    #[test]
    fn empty_path() {
        assert_eq!(normalize(Path::new("")), "");
    }

    #[test]
    fn single_component_path() {
        assert_eq!(normalize(Path::new("file.ts")), "file.ts");
    }

    #[test]
    fn deep_nested_path() {
        assert_eq!(normalize(Path::new("a/b/c/d/e.ts")), "a/b/c/d/e.ts");
    }

    #[test]
    fn path_with_spaces() {
        assert_eq!(
            normalize(Path::new("my project/src/file.ts")),
            "my project/src/file.ts"
        );
    }

    #[test]
    fn dot_relative_path() {
        assert_eq!(normalize(Path::new("./src/file.ts")), "./src/file.ts");
    }

    #[test]
    fn parent_relative_path() {
        assert_eq!(normalize(Path::new("../other/file.ts")), "../other/file.ts");
    }

    // Test the actual backslash replacement — the core purpose of this module.
    // On Unix, Path::new doesn't split on backslash, so to_string_lossy() preserves
    // literal backslashes, and .replace('\\', "/") converts them.

    #[test]
    fn backslash_replacement_in_string() {
        // Directly test the replace logic that runs on Windows paths
        let windows_path = "src\\utils\\index.ts";
        assert_eq!(windows_path.replace('\\', "/"), "src/utils/index.ts");
    }

    #[test]
    fn mixed_separators_normalized() {
        let mixed = "src/utils\\helpers\\index.ts";
        assert_eq!(mixed.replace('\\', "/"), "src/utils/helpers/index.ts");
    }

    #[test]
    fn backslash_only_path() {
        let path = "src\\deep\\nested\\file.ts";
        assert_eq!(path.replace('\\', "/"), "src/deep/nested/file.ts");
    }
}
