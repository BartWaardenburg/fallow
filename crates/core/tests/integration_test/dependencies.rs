use super::common::{create_config, fixture_path};

// ── Unlisted dependencies integration ──────────────────────────

#[test]
fn unlisted_dependencies_detected() {
    let root = fixture_path("unlisted-deps");
    let config = create_config(root);
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unlisted_names: Vec<&str> = results
        .unlisted_dependencies
        .iter()
        .map(|d| d.package_name.as_str())
        .collect();

    assert!(
        unlisted_names.contains(&"some-pkg"),
        "some-pkg should be detected as unlisted dependency, found: {unlisted_names:?}"
    );
}

// ── Unresolved imports integration ─────────────────────────────

#[test]
fn unresolved_imports_detected() {
    let root = fixture_path("unresolved-imports");
    let config = create_config(root);
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unresolved_specifiers: Vec<&str> = results
        .unresolved_imports
        .iter()
        .map(|u| u.specifier.as_str())
        .collect();

    assert!(
        unresolved_specifiers.contains(&"./nonexistent"),
        "\"./nonexistent\" should be detected as unresolved import, found: {unresolved_specifiers:?}"
    );
}

// ── Unused devDependencies ─────────────────────────────────────

#[test]
fn unused_dev_dependency_detected() {
    let root = fixture_path("unused-dev-deps");
    let config = create_config(root);
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_dev_dep_names: Vec<&str> = results
        .unused_dev_dependencies
        .iter()
        .map(|d| d.package_name.as_str())
        .collect();

    assert!(
        unused_dev_dep_names.contains(&"my-custom-dev-tool"),
        "my-custom-dev-tool should be detected as unused dev dependency, found: {unused_dev_dep_names:?}"
    );
}

// ── Unused optionalDependencies ───────────────────────────────

#[test]
fn unused_optional_dependency_detected() {
    let root = fixture_path("optional-deps");
    let config = create_config(root);
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    let unused_optional_dep_names: Vec<&str> = results
        .unused_optional_dependencies
        .iter()
        .map(|d| d.package_name.as_str())
        .collect();

    assert!(
        unused_optional_dep_names.contains(&"unused-optional-pkg"),
        "unused-optional-pkg should be detected as unused optional dependency, found: {unused_optional_dep_names:?}"
    );
}

// ── Package.json `imports` field (#subpath imports) ─────────

#[test]
fn subpath_imports_resolve_correctly() {
    let root = fixture_path("subpath-imports");
    let config = create_config(root);
    let results = fallow_core::analyze(&config).expect("analysis should succeed");

    // #utils and #components/Button should resolve — no unresolved imports
    assert!(
        results.unresolved_imports.is_empty(),
        "# imports should resolve via package.json imports field, got unresolved: {:?}",
        results
            .unresolved_imports
            .iter()
            .map(|u| u.specifier.as_str())
            .collect::<Vec<_>>()
    );

    // #utils and #components/Button should not be unlisted deps
    assert!(
        results.unlisted_dependencies.is_empty(),
        "# imports should not be reported as unlisted deps, got: {:?}",
        results
            .unlisted_dependencies
            .iter()
            .map(|d| d.package_name.as_str())
            .collect::<Vec<_>>()
    );

    // The `unused` export in utils/index.ts should still be detected
    let unused_export_names: Vec<&str> = results
        .unused_exports
        .iter()
        .map(|e| e.export_name.as_str())
        .collect();
    assert!(
        unused_export_names.contains(&"unused"),
        "unused export should still be detected, got: {unused_export_names:?}"
    );
}
