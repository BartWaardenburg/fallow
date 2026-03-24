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
