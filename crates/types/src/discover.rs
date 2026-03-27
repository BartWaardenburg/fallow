//! File discovery types: discovered files, file IDs, and entry points.

use std::path::PathBuf;

/// A discovered source file on disk.
#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    /// Unique file index.
    pub id: FileId,
    /// Absolute path.
    pub path: PathBuf,
    /// File size in bytes (for sorting largest-first).
    pub size_bytes: u64,
}

/// Compact file identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub u32);

// Size assertions to prevent memory regressions in hot-path types.
// These types are stored in large Vecs (one per project file) and iterated
// in tight loops during discovery, parsing, and graph construction.
const _: () = assert!(std::mem::size_of::<FileId>() == 4);
#[cfg(all(target_pointer_width = "64", unix))]
const _: () = assert!(std::mem::size_of::<DiscoveredFile>() == 40);

/// An entry point into the module graph.
#[derive(Debug, Clone)]
pub struct EntryPoint {
    /// Absolute path to the entry point file.
    pub path: PathBuf,
    /// How this entry point was discovered.
    pub source: EntryPointSource,
}

/// Where an entry point was discovered from.
#[derive(Debug, Clone)]
pub enum EntryPointSource {
    /// The `main` field in package.json.
    PackageJsonMain,
    /// The `module` field in package.json.
    PackageJsonModule,
    /// The `exports` field in package.json.
    PackageJsonExports,
    /// The `bin` field in package.json.
    PackageJsonBin,
    /// A script command in package.json.
    PackageJsonScript,
    /// Detected by a framework plugin.
    Plugin {
        /// Name of the plugin that detected this entry point.
        name: String,
    },
    /// A test file (e.g., `*.test.ts`, `*.spec.ts`).
    TestFile,
    /// A default index file (e.g., `src/index.ts`).
    DefaultIndex,
    /// Manually configured in fallow config.
    ManualEntry,
    /// Discovered from infrastructure config files (Dockerfile, Procfile, fly.toml).
    InfrastructureConfig,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // ── FileId ──────────────────────────────────────────────────────

    #[test]
    fn file_id_equality() {
        assert_eq!(FileId(0), FileId(0));
        assert_eq!(FileId(42), FileId(42));
        assert_ne!(FileId(0), FileId(1));
    }

    #[test]
    fn file_id_copy_semantics() {
        let a = FileId(5);
        let b = a; // Copy, not move
        assert_eq!(a, b);
    }

    #[test]
    fn file_id_hash_consistent() {
        let id = FileId(99);
        let hash1 = {
            let mut h = DefaultHasher::new();
            id.hash(&mut h);
            h.finish()
        };
        let hash2 = {
            let mut h = DefaultHasher::new();
            id.hash(&mut h);
            h.finish()
        };
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn file_id_equal_values_same_hash() {
        let a = FileId(7);
        let b = FileId(7);
        let hash_a = {
            let mut h = DefaultHasher::new();
            a.hash(&mut h);
            h.finish()
        };
        let hash_b = {
            let mut h = DefaultHasher::new();
            b.hash(&mut h);
            h.finish()
        };
        assert_eq!(hash_a, hash_b);
    }

    #[test]
    fn file_id_inner_value_accessible() {
        let id = FileId(123);
        assert_eq!(id.0, 123);
    }

    #[test]
    fn file_id_debug_format() {
        let id = FileId(42);
        let debug = format!("{id:?}");
        assert!(
            debug.contains("42"),
            "Debug should show inner value: {debug}"
        );
    }

    // ── DiscoveredFile ──────────────────────────────────────────────

    #[test]
    fn discovered_file_clone() {
        let original = DiscoveredFile {
            id: FileId(0),
            path: PathBuf::from("/project/src/index.ts"),
            size_bytes: 1024,
        };
        let cloned = original.clone();
        assert_eq!(cloned.id, original.id);
        assert_eq!(cloned.path, original.path);
        assert_eq!(cloned.size_bytes, original.size_bytes);
    }

    #[test]
    fn discovered_file_zero_size() {
        let file = DiscoveredFile {
            id: FileId(0),
            path: PathBuf::from("/empty.ts"),
            size_bytes: 0,
        };
        assert_eq!(file.size_bytes, 0);
    }

    #[test]
    fn discovered_file_large_size() {
        let file = DiscoveredFile {
            id: FileId(0),
            path: PathBuf::from("/large.ts"),
            size_bytes: u64::MAX,
        };
        assert_eq!(file.size_bytes, u64::MAX);
    }

    // ── EntryPoint ──────────────────────────────────────────────────

    #[test]
    fn entry_point_clone() {
        let ep = EntryPoint {
            path: PathBuf::from("/project/src/main.ts"),
            source: EntryPointSource::PackageJsonMain,
        };
        let cloned = ep.clone();
        assert_eq!(cloned.path, ep.path);
        assert!(matches!(cloned.source, EntryPointSource::PackageJsonMain));
    }

    // ── EntryPointSource ────────────────────────────────────────────

    #[test]
    fn entry_point_source_all_variants_constructible() {
        // Verify all variants can be constructed (compile-time coverage)
        let _ = EntryPointSource::PackageJsonMain;
        let _ = EntryPointSource::PackageJsonModule;
        let _ = EntryPointSource::PackageJsonExports;
        let _ = EntryPointSource::PackageJsonBin;
        let _ = EntryPointSource::PackageJsonScript;
        let _ = EntryPointSource::Plugin {
            name: "next".to_string(),
        };
        let _ = EntryPointSource::TestFile;
        let _ = EntryPointSource::DefaultIndex;
        let _ = EntryPointSource::ManualEntry;
        let _ = EntryPointSource::InfrastructureConfig;
    }

    #[test]
    fn entry_point_source_plugin_preserves_name() {
        let source = EntryPointSource::Plugin {
            name: "vitest".to_string(),
        };
        match source {
            EntryPointSource::Plugin { name } => assert_eq!(name, "vitest"),
            _ => panic!("expected Plugin variant"),
        }
    }

    #[test]
    fn entry_point_source_plugin_clone_preserves_name() {
        let source = EntryPointSource::Plugin {
            name: "storybook".to_string(),
        };
        // Use source after clone to verify both copies are valid
        let cloned = source.clone();
        // Verify original is still usable
        assert!(matches!(&source, EntryPointSource::Plugin { name } if name == "storybook"));
        // Verify clone has the same data
        match cloned {
            EntryPointSource::Plugin { name } => assert_eq!(name, "storybook"),
            _ => panic!("expected Plugin variant after clone"),
        }
    }

    #[test]
    fn entry_point_source_debug_format() {
        let source = EntryPointSource::PackageJsonMain;
        let debug = format!("{source:?}");
        assert!(
            debug.contains("PackageJsonMain"),
            "Debug should name the variant: {debug}"
        );

        let plugin = EntryPointSource::Plugin {
            name: "remix".to_string(),
        };
        let debug = format!("{plugin:?}");
        assert!(
            debug.contains("remix"),
            "Debug should show plugin name: {debug}"
        );
    }
}
