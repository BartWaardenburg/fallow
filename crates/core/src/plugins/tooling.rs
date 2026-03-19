//! General tooling dependency detection.
//!
//! Known dev dependencies that are tooling (used by CLI/config, not imported in
//! application code). These complement the per-plugin `tooling_dependencies()`
//! lists with dependencies that aren't tied to any single plugin.

/// Prefixes of package names that are always dev tooling.
const GENERAL_TOOLING_PREFIXES: &[&str] = &[
    "@types/",
    "eslint",
    "@typescript-eslint",
    "husky",
    "lint-staged",
    "commitlint",
    "@commitlint",
    "stylelint",
    "postcss",
    "autoprefixer",
    "tailwindcss",
    "@tailwindcss",
    "@vitest/",
    "@jest/",
    "@testing-library/",
    "@playwright/",
    "@storybook/",
    "storybook",
    "@babel/",
    "babel-",
    "@react-native-community/cli",
    "@react-native/",
    "secretlint",
    "@secretlint/",
    "oxlint",
    "@semantic-release/",
    "semantic-release",
    "@release-it/",
    "@lerna-lite/",
    "@changesets/",
    "@graphql-codegen/",
    "@rollup/",
    "@biomejs/",
];

/// Exact package names that are always dev tooling.
const GENERAL_TOOLING_EXACT: &[&str] = &[
    "typescript",
    "prettier",
    "turbo",
    "concurrently",
    "cross-env",
    "rimraf",
    "npm-run-all",
    "npm-run-all2",
    "nodemon",
    "ts-node",
    "tsx",
    "knip",
    "fallow",
    "jest",
    "vitest",
    "happy-dom",
    "jsdom",
    "vite",
    "sass",
    "sass-embedded",
    "webpack",
    "webpack-cli",
    "webpack-dev-server",
    "esbuild",
    "rollup",
    "swc",
    "@swc/core",
    "@swc/jest",
    "terser",
    "cssnano",
    "sharp",
    "release-it",
    "lerna",
    "dotenv-cli",
    "dotenv-flow",
    "oxfmt",
    "jscpd",
    "npm-check-updates",
    "markdownlint-cli",
    "npm-package-json-lint",
    "synp",
    "flow-bin",
    "i18next-parser",
    "i18next-conv",
    "webpack-bundle-analyzer",
    "vite-plugin-svgr",
    "vite-plugin-eslint",
    "@vitejs/plugin-vue",
    "@vitejs/plugin-react",
    "next-sitemap",
    "tsup",
    "unbuild",
    "typedoc",
    "nx",
    "@manypkg/cli",
    "vue-tsc",
    "@vue/tsconfig",
    "@tsconfig/node20",
    "@tsconfig/react-native",
    "@typescript/native-preview",
    "tw-animate-css",
    "@ianvs/prettier-plugin-sort-imports",
    "prettier-plugin-tailwindcss",
    "prettier-plugin-organize-imports",
    "@vitejs/plugin-react-swc",
    "@vitejs/plugin-legacy",
];

/// Check whether a package is a known tooling/dev dependency by name.
///
/// This is the single source of truth for general tooling detection.
/// Per-plugin tooling dependencies are declared via `Plugin::tooling_dependencies()`
/// and aggregated separately in `AggregatedPluginResult`.
pub fn is_known_tooling_dependency(name: &str) -> bool {
    GENERAL_TOOLING_PREFIXES.iter().any(|p| name.starts_with(p))
        || GENERAL_TOOLING_EXACT.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Prefix matching ──────────────────────────────────────────

    #[test]
    fn types_prefix_matches_scoped() {
        assert!(is_known_tooling_dependency("@types/node"));
        assert!(is_known_tooling_dependency("@types/react"));
        assert!(is_known_tooling_dependency("@types/express"));
    }

    #[test]
    fn types_prefix_does_not_match_similar_names() {
        // "type-fest" should NOT match "@types/" prefix
        assert!(!is_known_tooling_dependency("type-fest"));
        assert!(!is_known_tooling_dependency("typesafe-actions"));
    }

    #[test]
    fn storybook_prefix_matches() {
        assert!(is_known_tooling_dependency("@storybook/react"));
        assert!(is_known_tooling_dependency("@storybook/addon-essentials"));
        assert!(is_known_tooling_dependency("storybook"));
    }

    #[test]
    fn testing_library_prefix_matches() {
        assert!(is_known_tooling_dependency("@testing-library/react"));
        assert!(is_known_tooling_dependency("@testing-library/jest-dom"));
    }

    #[test]
    fn babel_prefix_matches() {
        assert!(is_known_tooling_dependency("@babel/core"));
        assert!(is_known_tooling_dependency("babel-loader"));
        assert!(is_known_tooling_dependency("babel-jest"));
    }

    #[test]
    fn vitest_prefix_matches() {
        assert!(is_known_tooling_dependency("@vitest/coverage-v8"));
        assert!(is_known_tooling_dependency("@vitest/ui"));
    }

    #[test]
    fn eslint_prefix_matches() {
        assert!(is_known_tooling_dependency("eslint"));
        assert!(is_known_tooling_dependency("eslint-plugin-react"));
        assert!(is_known_tooling_dependency("eslint-config-next"));
    }

    #[test]
    fn biomejs_prefix_matches() {
        assert!(is_known_tooling_dependency("@biomejs/biome"));
    }

    // ── Exact matching ───────────────────────────────────────────

    #[test]
    fn exact_typescript_matches() {
        assert!(is_known_tooling_dependency("typescript"));
    }

    #[test]
    fn exact_prettier_matches() {
        assert!(is_known_tooling_dependency("prettier"));
    }

    #[test]
    fn exact_vitest_matches() {
        assert!(is_known_tooling_dependency("vitest"));
    }

    #[test]
    fn exact_jest_matches() {
        assert!(is_known_tooling_dependency("jest"));
    }

    #[test]
    fn exact_vite_matches() {
        assert!(is_known_tooling_dependency("vite"));
    }

    #[test]
    fn exact_esbuild_matches() {
        assert!(is_known_tooling_dependency("esbuild"));
    }

    #[test]
    fn exact_tsup_matches() {
        assert!(is_known_tooling_dependency("tsup"));
    }

    #[test]
    fn exact_turbo_matches() {
        assert!(is_known_tooling_dependency("turbo"));
    }

    // ── Non-tooling dependencies ─────────────────────────────────

    #[test]
    fn common_runtime_deps_not_tooling() {
        assert!(!is_known_tooling_dependency("react"));
        assert!(!is_known_tooling_dependency("react-dom"));
        assert!(!is_known_tooling_dependency("express"));
        assert!(!is_known_tooling_dependency("lodash"));
        assert!(!is_known_tooling_dependency("next"));
        assert!(!is_known_tooling_dependency("vue"));
        assert!(!is_known_tooling_dependency("axios"));
    }

    #[test]
    fn empty_string_not_tooling() {
        assert!(!is_known_tooling_dependency(""));
    }

    #[test]
    fn near_miss_not_tooling() {
        // These look similar to tooling but should NOT match
        assert!(!is_known_tooling_dependency("type-fest"));
        assert!(!is_known_tooling_dependency("typestyle"));
        assert!(!is_known_tooling_dependency("prettier-bytes")); // not the exact "prettier"
        // Note: "prettier-bytes" starts with "prettier" but only prefix matches
        // check the prefixes list — "prettier" is NOT in GENERAL_TOOLING_PREFIXES,
        // it's in GENERAL_TOOLING_EXACT. So "prettier-bytes" should not match.
    }

    #[test]
    fn sass_variants_are_tooling() {
        assert!(is_known_tooling_dependency("sass"));
        assert!(is_known_tooling_dependency("sass-embedded"));
    }

    #[test]
    fn prettier_plugins_are_tooling() {
        assert!(is_known_tooling_dependency(
            "@ianvs/prettier-plugin-sort-imports"
        ));
        assert!(is_known_tooling_dependency("prettier-plugin-tailwindcss"));
    }
}
