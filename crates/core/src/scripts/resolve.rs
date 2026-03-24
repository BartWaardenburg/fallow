//! Binary name → npm package name resolution.

use std::path::Path;

/// Known binary-name → package-name mappings where they diverge.
static BINARY_TO_PACKAGE: &[(&str, &str)] = &[
    ("tsc", "typescript"),
    ("tsserver", "typescript"),
    ("ng", "@angular/cli"),
    ("nuxi", "nuxt"),
    ("run-s", "npm-run-all"),
    ("run-p", "npm-run-all"),
    ("run-s2", "npm-run-all2"),
    ("run-p2", "npm-run-all2"),
    ("sb", "storybook"),
    ("biome", "@biomejs/biome"),
    ("oxlint", "oxlint"),
];

/// Resolve a binary name to its npm package name.
///
/// Strategy:
/// 1. Check known binary→package divergence map
/// 2. Read `node_modules/.bin/<binary>` symlink target
/// 3. Fall back: binary name = package name
pub fn resolve_binary_to_package(binary: &str, root: &Path) -> String {
    // 1. Known divergences
    if let Some(&(_, pkg)) = BINARY_TO_PACKAGE.iter().find(|(bin, _)| *bin == binary) {
        return pkg.to_string();
    }

    // 2. Try reading the symlink in node_modules/.bin/
    let bin_link = root.join("node_modules/.bin").join(binary);
    if let Ok(target) = std::fs::read_link(&bin_link)
        && let Some(pkg_name) = extract_package_from_bin_path(&target)
    {
        return pkg_name;
    }

    // 3. Fallback: binary name = package name
    binary.to_string()
}

/// Extract a package name from a `node_modules/.bin` symlink target path.
///
/// Typical symlink targets:
/// - `../webpack/bin/webpack.js` → `webpack`
/// - `../@babel/cli/bin/babel.js` → `@babel/cli`
pub fn extract_package_from_bin_path(target: &std::path::Path) -> Option<String> {
    let target_str = target.to_string_lossy();
    let parts: Vec<&str> = target_str.split('/').collect();

    for (i, part) in parts.iter().enumerate() {
        if *part == ".." {
            continue;
        }
        // Scoped package: @scope/name
        if part.starts_with('@') && i + 1 < parts.len() {
            return Some(format!("{}/{}", part, parts[i + 1]));
        }
        // Regular package
        return Some(part.to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- BINARY_TO_PACKAGE known mappings ---

    #[test]
    fn tsserver_maps_to_typescript() {
        let pkg = resolve_binary_to_package("tsserver", Path::new("/nonexistent"));
        assert_eq!(pkg, "typescript");
    }

    #[test]
    fn nuxi_maps_to_nuxt() {
        let pkg = resolve_binary_to_package("nuxi", Path::new("/nonexistent"));
        assert_eq!(pkg, "nuxt");
    }

    #[test]
    fn run_p_maps_to_npm_run_all() {
        let pkg = resolve_binary_to_package("run-p", Path::new("/nonexistent"));
        assert_eq!(pkg, "npm-run-all");
    }

    #[test]
    fn run_s2_maps_to_npm_run_all2() {
        let pkg = resolve_binary_to_package("run-s2", Path::new("/nonexistent"));
        assert_eq!(pkg, "npm-run-all2");
    }

    #[test]
    fn run_p2_maps_to_npm_run_all2() {
        let pkg = resolve_binary_to_package("run-p2", Path::new("/nonexistent"));
        assert_eq!(pkg, "npm-run-all2");
    }

    #[test]
    fn sb_maps_to_storybook() {
        let pkg = resolve_binary_to_package("sb", Path::new("/nonexistent"));
        assert_eq!(pkg, "storybook");
    }

    #[test]
    fn oxlint_maps_to_oxlint() {
        let pkg = resolve_binary_to_package("oxlint", Path::new("/nonexistent"));
        assert_eq!(pkg, "oxlint");
    }

    // --- Unknown binary falls back to identity ---

    #[test]
    fn unknown_binary_returns_identity() {
        let pkg = resolve_binary_to_package("some-random-tool", Path::new("/nonexistent"));
        assert_eq!(pkg, "some-random-tool");
    }

    #[test]
    fn jest_identity_without_symlink() {
        // jest is not in the divergence map, and no symlink exists at /nonexistent
        let pkg = resolve_binary_to_package("jest", Path::new("/nonexistent"));
        assert_eq!(pkg, "jest");
    }

    #[test]
    fn eslint_identity_without_symlink() {
        let pkg = resolve_binary_to_package("eslint", Path::new("/nonexistent"));
        assert_eq!(pkg, "eslint");
    }

    // --- extract_package_from_bin_path ---

    #[test]
    fn bin_path_simple_package() {
        let path = std::path::Path::new("../eslint/bin/eslint.js");
        assert_eq!(extract_package_from_bin_path(path), Some("eslint".to_string()));
    }

    #[test]
    fn bin_path_scoped_package() {
        let path = std::path::Path::new("../@angular/cli/bin/ng");
        assert_eq!(
            extract_package_from_bin_path(path),
            Some("@angular/cli".to_string())
        );
    }

    #[test]
    fn bin_path_deeply_nested() {
        let path = std::path::Path::new("../../typescript/bin/tsc");
        assert_eq!(
            extract_package_from_bin_path(path),
            Some("typescript".to_string())
        );
    }

    #[test]
    fn bin_path_no_parent_dots() {
        let path = std::path::Path::new("webpack/bin/webpack.js");
        assert_eq!(
            extract_package_from_bin_path(path),
            Some("webpack".to_string())
        );
    }

    #[test]
    fn bin_path_only_dots() {
        let path = std::path::Path::new("../../..");
        assert_eq!(extract_package_from_bin_path(path), None);
    }

    #[test]
    fn bin_path_scoped_with_multiple_parents() {
        let path = std::path::Path::new("../../../@biomejs/biome/bin/biome");
        assert_eq!(
            extract_package_from_bin_path(path),
            Some("@biomejs/biome".to_string())
        );
    }
}
