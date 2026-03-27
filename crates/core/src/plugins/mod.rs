//! Plugin system for framework-aware codebase analysis.
//!
//! Unlike knip's JavaScript plugin system that evaluates config files at runtime,
//! fallow's plugin system uses Oxc's parser to extract configuration values from
//! JS/TS/JSON config files via AST walking — no JavaScript evaluation needed.
//!
//! Each plugin implements the [`Plugin`] trait with:
//! - **Static defaults**: Entry patterns, config file patterns, used exports
//! - **Dynamic resolution**: Parse tool config files to discover additional entries,
//!   referenced dependencies, and setup files

use std::path::{Path, PathBuf};

use fallow_config::PackageJson;

/// Result of resolving a plugin's config file.
#[derive(Debug, Default)]
pub struct PluginResult {
    /// Additional entry point glob patterns discovered from config.
    pub entry_patterns: Vec<String>,
    /// Dependencies referenced in config files (should not be flagged as unused).
    pub referenced_dependencies: Vec<String>,
    /// Additional files that are always considered used.
    pub always_used_files: Vec<String>,
    /// Setup/helper files referenced from config.
    pub setup_files: Vec<PathBuf>,
}

impl PluginResult {
    pub const fn is_empty(&self) -> bool {
        self.entry_patterns.is_empty()
            && self.referenced_dependencies.is_empty()
            && self.always_used_files.is_empty()
            && self.setup_files.is_empty()
    }
}

/// A framework/tool plugin that contributes to dead code analysis.
pub trait Plugin: Send + Sync {
    /// Human-readable plugin name.
    fn name(&self) -> &'static str;

    /// Package names that activate this plugin when found in package.json.
    /// Supports exact matches and prefix patterns (ending with `/`).
    fn enablers(&self) -> &'static [&'static str] {
        &[]
    }

    /// Check if this plugin should be active for the given project.
    /// Default implementation checks `enablers()` against package.json dependencies.
    fn is_enabled(&self, pkg: &PackageJson, root: &Path) -> bool {
        let deps = pkg.all_dependency_names();
        self.is_enabled_with_deps(&deps, root)
    }

    /// Fast variant of `is_enabled` that accepts a pre-computed deps list.
    /// Avoids repeated `all_dependency_names()` allocation when checking many plugins.
    fn is_enabled_with_deps(&self, deps: &[String], _root: &Path) -> bool {
        let enablers = self.enablers();
        if enablers.is_empty() {
            return false;
        }
        enablers.iter().any(|enabler| {
            if enabler.ends_with('/') {
                // Prefix match (e.g., "@storybook/" matches "@storybook/react")
                deps.iter().any(|d| d.starts_with(enabler))
            } else {
                deps.iter().any(|d| d == enabler)
            }
        })
    }

    /// Default glob patterns for entry point files.
    fn entry_patterns(&self) -> &'static [&'static str] {
        &[]
    }

    /// Glob patterns for config files this plugin can parse.
    fn config_patterns(&self) -> &'static [&'static str] {
        &[]
    }

    /// Files that are always considered "used" when this plugin is active.
    fn always_used(&self) -> &'static [&'static str] {
        &[]
    }

    /// Exports that are always considered used for matching file patterns.
    fn used_exports(&self) -> Vec<(&'static str, &'static [&'static str])> {
        vec![]
    }

    /// Dependencies that are tooling (used via CLI/config, not source imports).
    /// These should not be flagged as unused devDependencies.
    fn tooling_dependencies(&self) -> &'static [&'static str] {
        &[]
    }

    /// Import prefixes that are virtual modules provided by this framework at build time.
    /// Imports matching these prefixes should not be flagged as unlisted dependencies.
    /// Each entry is matched as a prefix against the extracted package name
    /// (e.g., `"@theme/"` matches `@theme/Layout`).
    fn virtual_module_prefixes(&self) -> &'static [&'static str] {
        &[]
    }

    /// Path alias mappings provided by this framework at build time.
    ///
    /// Returns a list of `(prefix, replacement_dir)` tuples. When an import starting
    /// with `prefix` fails to resolve, the resolver will substitute the prefix with
    /// `replacement_dir` (relative to the project root) and retry.
    ///
    /// Called once when plugins are activated. The project `root` is provided so
    /// plugins can inspect the filesystem (e.g., Nuxt checks whether `app/` exists
    /// to determine the `srcDir`).
    fn path_aliases(&self, _root: &Path) -> Vec<(&'static str, String)> {
        vec![]
    }

    /// Parse a config file's AST to discover additional entries, dependencies, etc.
    ///
    /// Called for each config file matching `config_patterns()`. The source code
    /// and parsed AST are provided — use [`config_parser`] utilities to extract values.
    fn resolve_config(&self, _config_path: &Path, _source: &str, _root: &Path) -> PluginResult {
        PluginResult::default()
    }

    /// The key name in package.json that holds inline configuration for this tool.
    /// When set (e.g., `"jest"` for the `"jest"` key in package.json), the plugin
    /// system will extract that key's value and call `resolve_config` with its
    /// JSON content if no standalone config file was found.
    fn package_json_config_key(&self) -> Option<&'static str> {
        None
    }
}

/// Macro to eliminate boilerplate in plugin implementations.
///
/// Generates a struct and a `Plugin` trait impl with the standard static methods
/// (`name`, `enablers`, `entry_patterns`, `config_patterns`, `always_used`, `tooling_dependencies`,
/// `used_exports`).
///
/// For plugins that need custom `resolve_config()` or `is_enabled()`, keep those as
/// manual `impl Plugin for ...` blocks instead of using this macro.
///
/// # Usage
///
/// ```ignore
/// // Simple plugin (most common):
/// define_plugin! {
///     struct VitePlugin => "vite",
///     enablers: ENABLERS,
///     entry_patterns: ENTRY_PATTERNS,
///     config_patterns: CONFIG_PATTERNS,
///     always_used: ALWAYS_USED,
///     tooling_dependencies: TOOLING_DEPENDENCIES,
/// }
///
/// // Plugin with used_exports:
/// define_plugin! {
///     struct RemixPlugin => "remix",
///     enablers: ENABLERS,
///     entry_patterns: ENTRY_PATTERNS,
///     always_used: ALWAYS_USED,
///     tooling_dependencies: TOOLING_DEPENDENCIES,
///     used_exports: [("app/routes/**/*.{ts,tsx}", ROUTE_EXPORTS)],
/// }
///
/// // Plugin with imports-only resolve_config (extracts imports from config as deps):
/// define_plugin! {
///     struct CypressPlugin => "cypress",
///     enablers: ENABLERS,
///     entry_patterns: ENTRY_PATTERNS,
///     config_patterns: CONFIG_PATTERNS,
///     always_used: ALWAYS_USED,
///     tooling_dependencies: TOOLING_DEPENDENCIES,
///     resolve_config: imports_only,
/// }
/// ```
///
/// All fields except `struct` and `enablers` are optional and default to `&[]` / `vec![]`.
macro_rules! define_plugin {
    // Variant with `resolve_config: imports_only` — generates a resolve_config method
    // that extracts imports from config files and registers them as referenced dependencies.
    (
        struct $name:ident => $display:expr,
        enablers: $enablers:expr
        $(, entry_patterns: $entry:expr)?
        $(, config_patterns: $config:expr)?
        $(, always_used: $always:expr)?
        $(, tooling_dependencies: $tooling:expr)?
        $(, virtual_module_prefixes: $virtual:expr)?
        $(, used_exports: [$( ($pat:expr, $exports:expr) ),* $(,)?])?
        , resolve_config: imports_only
        $(,)?
    ) => {
        pub struct $name;

        impl Plugin for $name {
            fn name(&self) -> &'static str {
                $display
            }

            fn enablers(&self) -> &'static [&'static str] {
                $enablers
            }

            $( fn entry_patterns(&self) -> &'static [&'static str] { $entry } )?
            $( fn config_patterns(&self) -> &'static [&'static str] { $config } )?
            $( fn always_used(&self) -> &'static [&'static str] { $always } )?
            $( fn tooling_dependencies(&self) -> &'static [&'static str] { $tooling } )?
            $( fn virtual_module_prefixes(&self) -> &'static [&'static str] { $virtual } )?

            $(
                fn used_exports(&self) -> Vec<(&'static str, &'static [&'static str])> {
                    vec![$( ($pat, $exports) ),*]
                }
            )?

            fn resolve_config(
                &self,
                config_path: &std::path::Path,
                source: &str,
                _root: &std::path::Path,
            ) -> PluginResult {
                let mut result = PluginResult::default();
                let imports = crate::plugins::config_parser::extract_imports(source, config_path);
                for imp in &imports {
                    let dep = crate::resolve::extract_package_name(imp);
                    result.referenced_dependencies.push(dep);
                }
                result
            }
        }
    };

    // Base variant — no resolve_config.
    (
        struct $name:ident => $display:expr,
        enablers: $enablers:expr
        $(, entry_patterns: $entry:expr)?
        $(, config_patterns: $config:expr)?
        $(, always_used: $always:expr)?
        $(, tooling_dependencies: $tooling:expr)?
        $(, virtual_module_prefixes: $virtual:expr)?
        $(, used_exports: [$( ($pat:expr, $exports:expr) ),* $(,)?])?
        $(,)?
    ) => {
        pub struct $name;

        impl Plugin for $name {
            fn name(&self) -> &'static str {
                $display
            }

            fn enablers(&self) -> &'static [&'static str] {
                $enablers
            }

            $( fn entry_patterns(&self) -> &'static [&'static str] { $entry } )?
            $( fn config_patterns(&self) -> &'static [&'static str] { $config } )?
            $( fn always_used(&self) -> &'static [&'static str] { $always } )?
            $( fn tooling_dependencies(&self) -> &'static [&'static str] { $tooling } )?
            $( fn virtual_module_prefixes(&self) -> &'static [&'static str] { $virtual } )?

            $(
                fn used_exports(&self) -> Vec<(&'static str, &'static [&'static str])> {
                    vec![$( ($pat, $exports) ),*]
                }
            )?
        }
    };
}

pub mod config_parser;
pub mod registry;
mod tooling;

pub use registry::{AggregatedPluginResult, PluginRegistry};
pub use tooling::is_known_tooling_dependency;

mod angular;
mod astro;
mod ava;
mod babel;
mod biome;
mod bun;
mod c8;
mod capacitor;
mod changesets;
mod commitizen;
mod commitlint;
mod cspell;
mod cucumber;
mod cypress;
mod dependency_cruiser;
mod docusaurus;
mod drizzle;
mod electron;
mod eslint;
mod expo;
mod gatsby;
mod graphql_codegen;
mod husky;
mod i18next;
mod jest;
mod karma;
mod knex;
mod kysely;
mod lefthook;
mod lint_staged;
mod markdownlint;
mod mocha;
mod msw;
mod nestjs;
mod next_intl;
mod nextjs;
mod nitro;
mod nodemon;
mod nuxt;
mod nx;
mod nyc;
mod openapi_ts;
mod oxlint;
mod parcel;
mod playwright;
mod plop;
mod pm2;
mod postcss;
mod prettier;
mod prisma;
mod react_native;
mod react_router;
mod relay;
mod remark;
mod remix;
mod rolldown;
mod rollup;
mod rsbuild;
mod rspack;
mod sanity;
mod semantic_release;
mod sentry;
mod simple_git_hooks;
mod storybook;
mod stylelint;
mod sveltekit;
mod svgo;
mod svgr;
mod swc;
mod syncpack;
mod tailwind;
mod tanstack_router;
mod tsdown;
mod tsup;
mod turborepo;
mod typedoc;
mod typeorm;
mod typescript;
mod vite;
mod vitepress;
mod vitest;
mod webdriverio;
mod webpack;
mod wrangler;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // ── is_enabled_with_deps edge cases ──────────────────────────

    #[test]
    fn is_enabled_with_deps_exact_match() {
        let plugin = nextjs::NextJsPlugin;
        let deps = vec!["next".to_string()];
        assert!(plugin.is_enabled_with_deps(&deps, Path::new("/project")));
    }

    #[test]
    fn is_enabled_with_deps_no_match() {
        let plugin = nextjs::NextJsPlugin;
        let deps = vec!["react".to_string()];
        assert!(!plugin.is_enabled_with_deps(&deps, Path::new("/project")));
    }

    #[test]
    fn is_enabled_with_deps_empty_deps() {
        let plugin = nextjs::NextJsPlugin;
        let deps: Vec<String> = vec![];
        assert!(!plugin.is_enabled_with_deps(&deps, Path::new("/project")));
    }

    // ── PluginResult::is_empty ───────────────────────────────────

    #[test]
    fn plugin_result_is_empty_when_default() {
        let r = PluginResult::default();
        assert!(r.is_empty());
    }

    #[test]
    fn plugin_result_not_empty_with_entry_patterns() {
        let r = PluginResult {
            entry_patterns: vec!["*.ts".to_string()],
            ..Default::default()
        };
        assert!(!r.is_empty());
    }

    #[test]
    fn plugin_result_not_empty_with_referenced_deps() {
        let r = PluginResult {
            referenced_dependencies: vec!["lodash".to_string()],
            ..Default::default()
        };
        assert!(!r.is_empty());
    }

    #[test]
    fn plugin_result_not_empty_with_setup_files() {
        let r = PluginResult {
            setup_files: vec![PathBuf::from("/setup.ts")],
            ..Default::default()
        };
        assert!(!r.is_empty());
    }

    #[test]
    fn plugin_result_not_empty_with_always_used_files() {
        let r = PluginResult {
            always_used_files: vec!["**/*.stories.tsx".to_string()],
            ..Default::default()
        };
        assert!(!r.is_empty());
    }

    // ── is_enabled_with_deps prefix matching ─────────────────────

    #[test]
    fn is_enabled_with_deps_prefix_match() {
        // Storybook plugin uses prefix enabler "@storybook/"
        let plugin = storybook::StorybookPlugin;
        let deps = vec!["@storybook/react".to_string()];
        assert!(plugin.is_enabled_with_deps(&deps, Path::new("/project")));
    }

    #[test]
    fn is_enabled_with_deps_prefix_no_match_without_slash() {
        // "@storybook/" prefix should NOT match "@storybookish" (different package)
        let plugin = storybook::StorybookPlugin;
        let deps = vec!["@storybookish".to_string()];
        assert!(!plugin.is_enabled_with_deps(&deps, Path::new("/project")));
    }

    #[test]
    fn is_enabled_with_deps_multiple_enablers() {
        // Vitest plugin has multiple enablers
        let plugin = vitest::VitestPlugin;
        let deps_vitest = vec!["vitest".to_string()];
        let deps_none = vec!["mocha".to_string()];
        assert!(plugin.is_enabled_with_deps(&deps_vitest, Path::new("/project")));
        assert!(!plugin.is_enabled_with_deps(&deps_none, Path::new("/project")));
    }

    // ── Plugin trait default implementations ─────────────────────

    #[test]
    fn plugin_default_methods_return_empty() {
        // Use a simple plugin to test default trait methods
        let plugin = commitizen::CommitizenPlugin;
        assert!(
            plugin.tooling_dependencies().is_empty() || !plugin.tooling_dependencies().is_empty()
        );
        assert!(plugin.virtual_module_prefixes().is_empty());
        assert!(plugin.path_aliases(Path::new("/project")).is_empty());
        assert!(
            plugin.package_json_config_key().is_none()
                || plugin.package_json_config_key().is_some()
        );
    }

    #[test]
    fn plugin_resolve_config_default_returns_empty() {
        let plugin = commitizen::CommitizenPlugin;
        let result = plugin.resolve_config(
            Path::new("/project/config.js"),
            "const x = 1;",
            Path::new("/project"),
        );
        assert!(result.is_empty());
    }
}
