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
#[cfg(target_pointer_width = "64")]
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
}
