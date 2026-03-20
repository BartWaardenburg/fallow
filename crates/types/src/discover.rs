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
const _: () = assert!(std::mem::size_of::<DiscoveredFile>() == 40);

/// An entry point into the module graph.
#[derive(Debug, Clone)]
pub struct EntryPoint {
    pub path: PathBuf,
    pub source: EntryPointSource,
}

/// Where an entry point was discovered from.
#[derive(Debug, Clone)]
pub enum EntryPointSource {
    PackageJsonMain,
    PackageJsonModule,
    PackageJsonExports,
    PackageJsonBin,
    PackageJsonScript,
    Plugin { name: String },
    TestFile,
    DefaultIndex,
    ManualEntry,
}
