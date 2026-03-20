use std::path::{Path, PathBuf};

use rustc_hash::FxHashMap;

use fallow_config::WorkspaceInfo;

use fallow_types::discover::{DiscoveredFile, FileId};

/// Centralized project state owning the file registry and workspace metadata.
///
/// Provides:
/// - Stable `FileId` assignment (deterministic by path, not by size)
/// - O(1) path-to-id lookups for cross-workspace module resolution
/// - Workspace-aware queries (which workspace owns a file, files in a workspace)
///
/// Future incremental analysis will persist the id assignment across runs so
/// that adding/removing files does not invalidate cached graph data.
pub struct ProjectState {
    files: Vec<DiscoveredFile>,
    path_to_id: FxHashMap<PathBuf, FileId>,
    workspaces: Vec<WorkspaceInfo>,
}

impl ProjectState {
    /// Build a new project state from discovered files and workspaces.
    pub fn new(files: Vec<DiscoveredFile>, workspaces: Vec<WorkspaceInfo>) -> Self {
        let path_to_id = files.iter().map(|f| (f.path.clone(), f.id)).collect();
        Self {
            files,
            path_to_id,
            workspaces,
        }
    }

    /// All discovered files, indexed by `FileId`.
    pub fn files(&self) -> &[DiscoveredFile] {
        &self.files
    }

    /// All discovered workspace packages.
    pub fn workspaces(&self) -> &[WorkspaceInfo] {
        &self.workspaces
    }

    /// Look up a file by its `FileId`.
    pub fn file_by_id(&self, id: FileId) -> Option<&DiscoveredFile> {
        self.files.get(id.0 as usize)
    }

    /// Look up a `FileId` by absolute path.
    pub fn id_for_path(&self, path: &Path) -> Option<FileId> {
        self.path_to_id.get(path).copied()
    }

    /// Find which workspace a file belongs to, if any.
    pub fn workspace_for_file(&self, id: FileId) -> Option<&WorkspaceInfo> {
        let path = &self.files.get(id.0 as usize)?.path;
        self.workspaces.iter().find(|ws| path.starts_with(&ws.root))
    }

    /// Look up a workspace by package name.
    pub fn workspace_by_name(&self, name: &str) -> Option<&WorkspaceInfo> {
        self.workspaces.iter().find(|ws| ws.name == name)
    }

    /// Get all `FileId`s for files within a workspace.
    pub fn files_in_workspace(&self, ws: &WorkspaceInfo) -> Vec<FileId> {
        self.files
            .iter()
            .filter(|f| f.path.starts_with(&ws.root))
            .map(|f| f.id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_file(id: u32, path: &str) -> DiscoveredFile {
        DiscoveredFile {
            id: FileId(id),
            path: PathBuf::from(path),
            size_bytes: 100,
        }
    }

    fn make_workspace(name: &str, root: &str) -> WorkspaceInfo {
        WorkspaceInfo {
            root: PathBuf::from(root),
            name: name.to_string(),
            is_internal_dependency: false,
        }
    }

    #[test]
    fn id_for_path_lookup() {
        let files = vec![
            make_file(0, "/project/packages/a/src/index.ts"),
            make_file(1, "/project/packages/b/src/index.ts"),
        ];
        let state = ProjectState::new(files, vec![]);
        assert_eq!(
            state.id_for_path(Path::new("/project/packages/a/src/index.ts")),
            Some(FileId(0))
        );
        assert_eq!(
            state.id_for_path(Path::new("/project/packages/b/src/index.ts")),
            Some(FileId(1))
        );
        assert_eq!(state.id_for_path(Path::new("/project/missing.ts")), None);
    }

    #[test]
    fn workspace_for_file_lookup() {
        let files = vec![
            make_file(0, "/project/packages/ui/src/button.ts"),
            make_file(1, "/project/src/app.ts"),
        ];
        let workspaces = vec![make_workspace("ui", "/project/packages/ui")];
        let state = ProjectState::new(files, workspaces);

        assert_eq!(
            state.workspace_for_file(FileId(0)).map(|ws| &ws.name),
            Some(&"ui".to_string())
        );
        assert!(state.workspace_for_file(FileId(1)).is_none());
    }

    #[test]
    fn workspace_by_name_lookup() {
        let workspaces = vec![
            make_workspace("ui", "/project/packages/ui"),
            make_workspace("core", "/project/packages/core"),
        ];
        let state = ProjectState::new(vec![], workspaces);

        assert!(state.workspace_by_name("ui").is_some());
        assert!(state.workspace_by_name("core").is_some());
        assert!(state.workspace_by_name("missing").is_none());
    }

    #[test]
    fn files_in_workspace() {
        let files = vec![
            make_file(0, "/project/packages/ui/src/a.ts"),
            make_file(1, "/project/packages/ui/src/b.ts"),
            make_file(2, "/project/packages/core/src/c.ts"),
            make_file(3, "/project/src/app.ts"),
        ];
        let workspaces = vec![
            make_workspace("ui", "/project/packages/ui"),
            make_workspace("core", "/project/packages/core"),
        ];
        let state = ProjectState::new(files, workspaces);

        let ui_ws = state.workspace_by_name("ui").unwrap();
        let ui_files = state.files_in_workspace(ui_ws);
        assert_eq!(ui_files, vec![FileId(0), FileId(1)]);

        let core_ws = state.workspace_by_name("core").unwrap();
        let core_files = state.files_in_workspace(core_ws);
        assert_eq!(core_files, vec![FileId(2)]);
    }
}
