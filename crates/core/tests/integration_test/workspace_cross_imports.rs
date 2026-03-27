use super::common::{create_config, fixture_path};

#[test]
fn workspace_cross_import_resolves() {
    let root = fixture_path("workspace-cross-imports");

    // Set up node_modules symlinks for cross-workspace resolution
    let nm = root.join("node_modules").join("@myorg");
    let _ = std::fs::create_dir_all(&nm);
    #[cfg(unix)]
    {
        let _ = std::os::unix::fs::symlink(root.join("packages/core"), nm.join("core"));
    }
    #[cfg(windows)]
    {
        let _ = std::os::windows::fs::symlink_dir(root.join("packages/core"), nm.join("core"));
    }

    let config = create_config(root);
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    // No unresolved imports — cross-workspace @myorg/core should resolve
    assert!(
        results.unresolved_imports.is_empty(),
        "cross-workspace imports should resolve, found unresolved: {:?}",
        results
            .unresolved_imports
            .iter()
            .map(|i| &i.specifier)
            .collect::<Vec<_>>()
    );
}

#[test]
fn workspace_cross_import_detects_orphan() {
    let root = fixture_path("workspace-cross-imports");

    // Set up node_modules symlinks
    let nm = root.join("node_modules").join("@myorg");
    let _ = std::fs::create_dir_all(&nm);
    #[cfg(unix)]
    {
        let _ = std::os::unix::fs::symlink(root.join("packages/core"), nm.join("core"));
    }
    #[cfg(windows)]
    {
        let _ = std::os::windows::fs::symlink_dir(root.join("packages/core"), nm.join("core"));
    }

    let config = create_config(root);
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_file_names: Vec<String> = results
        .unused_files
        .iter()
        .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    assert!(
        unused_file_names.contains(&"orphan.ts".to_string()),
        "orphan.ts should be unused, found: {unused_file_names:?}"
    );
}

#[test]
fn workspace_cross_import_detects_unused_export() {
    let root = fixture_path("workspace-cross-imports");

    // Set up node_modules symlinks
    let nm = root.join("node_modules").join("@myorg");
    let _ = std::fs::create_dir_all(&nm);
    #[cfg(unix)]
    {
        let _ = std::os::unix::fs::symlink(root.join("packages/core"), nm.join("core"));
    }
    #[cfg(windows)]
    {
        let _ = std::os::windows::fs::symlink_dir(root.join("packages/core"), nm.join("core"));
    }

    let config = create_config(root);
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();

    // unusedCoreExport is not imported by the web package
    assert!(
        unused_export_names.contains(&"unusedCoreExport"),
        "unusedCoreExport should be unused, found: {unused_export_names:?}"
    );

    // coreHelper IS imported by web, should NOT be flagged
    assert!(
        !unused_export_names.contains(&"coreHelper"),
        "coreHelper should NOT be unused (imported by web), found: {unused_export_names:?}"
    );
}
