//! Specifier classification: bare specifiers, path aliases, and package name extraction.

/// Check if a bare specifier looks like a path alias rather than an npm package.
///
/// Path aliases (e.g., `@/components`, `~/lib`, `#internal`, `~~/utils`) are resolved
/// via tsconfig.json `paths` or package.json `imports`. They should not be cached
/// (resolution depends on the importing file's tsconfig context) and should return
/// `Unresolvable` (not `NpmPackage`) when resolution fails.
pub fn is_path_alias(specifier: &str) -> bool {
    // `#` prefix is Node.js imports maps (package.json "imports" field)
    if specifier.starts_with('#') {
        return true;
    }
    // `~/` and `~~/` prefixes are common alias conventions (e.g., Nuxt, custom tsconfig)
    if specifier.starts_with("~/") || specifier.starts_with("~~/") {
        return true;
    }
    // `@/` is a very common path alias (e.g., `@/components/Foo`)
    if specifier.starts_with("@/") {
        return true;
    }
    // npm scoped packages MUST be lowercase (npm registry requirement).
    // PascalCase `@Scope` or `@Scope/path` patterns are tsconfig path aliases,
    // not npm packages. E.g., `@Components`, `@Hooks/useApi`, `@Services/auth`.
    if specifier.starts_with('@') {
        let scope = specifier.split('/').next().unwrap_or(specifier);
        if scope.len() > 1 && scope.chars().nth(1).is_some_and(|c| c.is_ascii_uppercase()) {
            return true;
        }
    }

    false
}

/// Check if a specifier is a bare specifier (npm package or Node.js imports map entry).
pub fn is_bare_specifier(specifier: &str) -> bool {
    !specifier.starts_with('.')
        && !specifier.starts_with('/')
        && !specifier.contains("://")
        && !specifier.starts_with("data:")
}

/// Extract the npm package name from a specifier.
/// `@scope/pkg/foo/bar` -> `@scope/pkg`
/// `lodash/merge` -> `lodash`
pub fn extract_package_name(specifier: &str) -> String {
    if specifier.starts_with('@') {
        let parts: Vec<&str> = specifier.splitn(3, '/').collect();
        if parts.len() >= 2 {
            format!("{}/{}", parts[0], parts[1])
        } else {
            specifier.to_string()
        }
    } else {
        specifier.split('/').next().unwrap_or(specifier).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_package_name() {
        assert_eq!(extract_package_name("react"), "react");
        assert_eq!(extract_package_name("lodash/merge"), "lodash");
        assert_eq!(extract_package_name("@scope/pkg"), "@scope/pkg");
        assert_eq!(extract_package_name("@scope/pkg/foo"), "@scope/pkg");
    }

    #[test]
    fn test_is_bare_specifier() {
        assert!(is_bare_specifier("react"));
        assert!(is_bare_specifier("@scope/pkg"));
        assert!(is_bare_specifier("#internal/module"));
        assert!(!is_bare_specifier("./utils"));
        assert!(!is_bare_specifier("../lib"));
        assert!(!is_bare_specifier("/absolute"));
    }

    #[test]
    fn test_is_bare_specifier_url_specifiers() {
        assert!(!is_bare_specifier("https://cdn.example.com/lib.js"));
        assert!(!is_bare_specifier("http://example.com/module"));
        assert!(!is_bare_specifier("data:text/javascript,export default 42"));
    }

    // ── is_path_alias ───────────────────────────────────────────────

    #[test]
    fn path_alias_hash_prefix() {
        assert!(is_path_alias("#internal/module"));
        assert!(is_path_alias("#shared"));
    }

    #[test]
    fn path_alias_tilde_prefix() {
        assert!(is_path_alias("~/components/Button"));
        assert!(is_path_alias("~~/utils/helpers"));
    }

    #[test]
    fn path_alias_at_slash_prefix() {
        assert!(is_path_alias("@/components/Button"));
        assert!(is_path_alias("@/lib"));
    }

    #[test]
    fn path_alias_pascal_case_scope() {
        // PascalCase scoped packages are tsconfig aliases, not npm packages
        assert!(is_path_alias("@Components/Button"));
        assert!(is_path_alias("@Hooks/useApi"));
        assert!(is_path_alias("@Services/auth"));
    }

    #[test]
    fn path_alias_lowercase_scope_is_not_alias() {
        // Lowercase scoped packages are regular npm packages
        assert!(!is_path_alias("@babel/core"));
        assert!(!is_path_alias("@types/react"));
        assert!(!is_path_alias("@scope/pkg"));
    }

    #[test]
    fn path_alias_plain_specifier_is_not_alias() {
        assert!(!is_path_alias("react"));
        assert!(!is_path_alias("lodash/merge"));
        assert!(!is_path_alias("my-utils"));
    }

    #[test]
    fn path_alias_tilde_without_slash_is_not_alias() {
        // `~something` without a slash is not a path alias convention
        assert!(!is_path_alias("~something"));
    }

    // ── extract_package_name edge cases ─────────────────────────────

    #[test]
    fn extract_package_name_bare_scope_only() {
        // Edge case: just `@scope` without a package name
        assert_eq!(extract_package_name("@scope"), "@scope");
    }

    #[test]
    fn extract_package_name_deep_subpath() {
        assert_eq!(
            extract_package_name("@scope/pkg/deep/nested/path"),
            "@scope/pkg"
        );
    }

    #[test]
    fn extract_package_name_single_name() {
        assert_eq!(extract_package_name("react"), "react");
    }

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// Any specifier starting with `.` or `/` must NOT be classified as a bare specifier.
            #[test]
            fn relative_paths_are_not_bare(suffix in "[a-zA-Z0-9_/.-]{0,80}") {
                let dot = format!(".{suffix}");
                let slash = format!("/{suffix}");
                prop_assert!(!is_bare_specifier(&dot), "'.{suffix}' was classified as bare");
                prop_assert!(!is_bare_specifier(&slash), "'/{suffix}' was classified as bare");
            }

            /// Scoped packages (@scope/pkg) should extract exactly `@scope/pkg` — two segments.
            #[test]
            fn scoped_package_name_has_two_segments(
                scope in "[a-z][a-z0-9-]{0,20}",
                pkg in "[a-z][a-z0-9-]{0,20}",
                subpath in "(/[a-z0-9-]{1,20}){0,3}",
            ) {
                let specifier = format!("@{scope}/{pkg}{subpath}");
                let extracted = extract_package_name(&specifier);
                let expected = format!("@{scope}/{pkg}");
                prop_assert_eq!(extracted, expected);
            }

            /// Unscoped packages should extract exactly the first path segment.
            #[test]
            fn unscoped_package_name_is_first_segment(
                pkg in "[a-z][a-z0-9-]{0,30}",
                subpath in "(/[a-z0-9-]{1,20}){0,3}",
            ) {
                let specifier = format!("{pkg}{subpath}");
                let extracted = extract_package_name(&specifier);
                prop_assert_eq!(extracted, pkg);
            }

            /// is_bare_specifier and is_path_alias should never panic on arbitrary strings.
            #[test]
            fn bare_specifier_and_path_alias_no_panic(s in "[a-zA-Z0-9@#~/._-]{1,100}") {
                let _ = is_bare_specifier(&s);
                let _ = is_path_alias(&s);
            }

            /// `@/` prefix should always be detected as a path alias.
            #[test]
            fn at_slash_is_path_alias(suffix in "[a-zA-Z0-9_/.-]{0,80}") {
                let specifier = format!("@/{suffix}");
                prop_assert!(is_path_alias(&specifier));
            }

            /// `~/` prefix should always be detected as a path alias.
            #[test]
            fn tilde_slash_is_path_alias(suffix in "[a-zA-Z0-9_/.-]{0,80}") {
                let specifier = format!("~/{suffix}");
                prop_assert!(is_path_alias(&specifier));
            }

            /// `#` prefix should always be detected as a path alias (Node.js imports map).
            #[test]
            fn hash_prefix_is_path_alias(suffix in "[a-zA-Z0-9_/.-]{0,80}") {
                let specifier = format!("#{suffix}");
                prop_assert!(is_path_alias(&specifier));
            }

            /// Extracted package name from node_modules path should never be empty.
            #[test]
            fn node_modules_package_name_never_empty(
                pkg in "[a-z][a-z0-9-]{0,20}",
                file in "[a-z]{1,10}\\.(js|ts|mjs)",
            ) {
                let path = std::path::PathBuf::from(format!("/project/node_modules/{pkg}/{file}"));
                if let Some(name) = crate::resolve::fallbacks::extract_package_name_from_node_modules_path(&path) {
                    prop_assert!(!name.is_empty());
                }
            }
        }
    }
}
