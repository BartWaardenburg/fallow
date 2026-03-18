//! Plugin system for framework-aware dead code analysis.
//!
//! Unlike knip's JavaScript plugin system that evaluates config files at runtime,
//! fallow's plugin system uses Oxc's parser to extract configuration values from
//! JS/TS/JSON config files via AST walking — no JavaScript evaluation needed.
//!
//! Each plugin implements the [`Plugin`] trait with:
//! - **Static defaults**: Entry patterns, config file patterns, used exports
//! - **Dynamic resolution**: Parse tool config files to discover additional entries,
//!   referenced dependencies, and setup files

pub mod config_parser;

mod angular;
mod astro;
mod ava;
mod babel;
mod biome;
mod changesets;
mod commitlint;
mod cypress;
mod docusaurus;
mod drizzle;
mod eslint;
mod expo;
mod graphql_codegen;
mod jest;
mod knex;
mod mocha;
mod msw;
mod nestjs;
mod nextjs;
mod nuxt;
mod nx;
mod playwright;
mod postcss;
mod prisma;
mod react_native;
mod react_router;
mod remix;
mod rollup;
mod semantic_release;
mod sentry;
mod storybook;
mod stylelint;
mod tailwind;
mod tsup;
mod turborepo;
mod typescript;
mod vite;
mod vitest;
mod webpack;
mod wrangler;

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
    pub fn is_empty(&self) -> bool {
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
    fn is_enabled(&self, pkg: &PackageJson, _root: &Path) -> bool {
        let deps = pkg.all_dependency_names();
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

    /// Parse a config file's AST to discover additional entries, dependencies, etc.
    ///
    /// Called for each config file matching `config_patterns()`. The source code
    /// and parsed AST are provided — use [`config_parser`] utilities to extract values.
    fn resolve_config(&self, _config_path: &Path, _source: &str, _root: &Path) -> PluginResult {
        PluginResult::default()
    }
}

/// Registry of all available plugins.
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
}

/// Aggregated results from all active plugins for a project.
#[derive(Debug, Default)]
pub struct AggregatedPluginResult {
    /// All entry point patterns from active plugins.
    pub entry_patterns: Vec<String>,
    /// All config file patterns from active plugins.
    pub config_patterns: Vec<String>,
    /// All always-used file patterns from active plugins.
    pub always_used: Vec<String>,
    /// All used export rules from active plugins.
    pub used_exports: Vec<(String, Vec<String>)>,
    /// Dependencies referenced in config files (should not be flagged unused).
    pub referenced_dependencies: Vec<String>,
    /// Additional always-used files discovered from config parsing.
    pub discovered_always_used: Vec<String>,
    /// Setup files discovered from config parsing.
    pub setup_files: Vec<PathBuf>,
    /// Tooling dependencies (should not be flagged as unused devDeps).
    pub tooling_dependencies: Vec<String>,
    /// Names of active plugins.
    pub active_plugins: Vec<String>,
}

impl PluginRegistry {
    /// Create a registry with all built-in plugins.
    pub fn new() -> Self {
        let plugins: Vec<Box<dyn Plugin>> = vec![
            // Frameworks
            Box::new(nextjs::NextJsPlugin),
            Box::new(nuxt::NuxtPlugin),
            Box::new(remix::RemixPlugin),
            Box::new(astro::AstroPlugin),
            Box::new(angular::AngularPlugin),
            Box::new(react_router::ReactRouterPlugin),
            Box::new(react_native::ReactNativePlugin),
            Box::new(expo::ExpoPlugin),
            Box::new(nestjs::NestJsPlugin),
            Box::new(docusaurus::DocusaurusPlugin),
            // Bundlers
            Box::new(vite::VitePlugin),
            Box::new(webpack::WebpackPlugin),
            Box::new(rollup::RollupPlugin),
            Box::new(tsup::TsupPlugin),
            // Testing
            Box::new(vitest::VitestPlugin),
            Box::new(jest::JestPlugin),
            Box::new(playwright::PlaywrightPlugin),
            Box::new(cypress::CypressPlugin),
            Box::new(mocha::MochaPlugin),
            Box::new(ava::AvaPlugin),
            Box::new(storybook::StorybookPlugin),
            // Linting & formatting
            Box::new(eslint::EslintPlugin),
            Box::new(biome::BiomePlugin),
            Box::new(stylelint::StylelintPlugin),
            // Transpilation & language
            Box::new(typescript::TypeScriptPlugin),
            Box::new(babel::BabelPlugin),
            // CSS
            Box::new(tailwind::TailwindPlugin),
            Box::new(postcss::PostCssPlugin),
            // Database & ORM
            Box::new(prisma::PrismaPlugin),
            Box::new(drizzle::DrizzlePlugin),
            Box::new(knex::KnexPlugin),
            // Monorepo
            Box::new(turborepo::TurborepoPlugin),
            Box::new(nx::NxPlugin),
            Box::new(changesets::ChangesetsPlugin),
            // CI/CD & release
            Box::new(commitlint::CommitlintPlugin),
            Box::new(semantic_release::SemanticReleasePlugin),
            // Deployment
            Box::new(wrangler::WranglerPlugin),
            Box::new(sentry::SentryPlugin),
            // Other tools
            Box::new(graphql_codegen::GraphqlCodegenPlugin),
            Box::new(msw::MswPlugin),
        ];
        Self { plugins }
    }

    /// Run all plugins against a project, returning aggregated results.
    ///
    /// This discovers which plugins are active, collects their static patterns,
    /// then parses any config files to extract dynamic information.
    pub fn run(
        &self,
        pkg: &PackageJson,
        root: &Path,
        discovered_files: &[PathBuf],
    ) -> AggregatedPluginResult {
        let _span = tracing::info_span!("run_plugins").entered();
        let mut result = AggregatedPluginResult::default();

        // Phase 1: Determine which plugins are active
        let active: Vec<&dyn Plugin> = self
            .plugins
            .iter()
            .filter(|p| p.is_enabled(pkg, root))
            .map(|p| p.as_ref())
            .collect();

        tracing::info!(
            plugins = active
                .iter()
                .map(|p| p.name())
                .collect::<Vec<_>>()
                .join(", "),
            "active plugins"
        );

        // Phase 2: Collect static patterns from active plugins
        for plugin in &active {
            result.active_plugins.push(plugin.name().to_string());

            for pat in plugin.entry_patterns() {
                result.entry_patterns.push((*pat).to_string());
            }
            for pat in plugin.config_patterns() {
                result.config_patterns.push((*pat).to_string());
            }
            for pat in plugin.always_used() {
                result.always_used.push((*pat).to_string());
            }
            for (file_pat, exports) in plugin.used_exports() {
                result.used_exports.push((
                    file_pat.to_string(),
                    exports.iter().map(|s| s.to_string()).collect(),
                ));
            }
            for dep in plugin.tooling_dependencies() {
                result.tooling_dependencies.push((*dep).to_string());
            }
        }

        // Phase 3: Find and parse config files for dynamic resolution
        // Pre-compile all config patterns
        let config_matchers: Vec<(&dyn Plugin, Vec<globset::GlobMatcher>)> = active
            .iter()
            .filter(|p| !p.config_patterns().is_empty())
            .map(|p| {
                let matchers: Vec<globset::GlobMatcher> = p
                    .config_patterns()
                    .iter()
                    .filter_map(|pat| globset::Glob::new(pat).ok().map(|g| g.compile_matcher()))
                    .collect();
                (*p, matchers)
            })
            .collect();

        if !config_matchers.is_empty() {
            // Build relative paths for matching
            let relative_files: Vec<(&PathBuf, String)> = discovered_files
                .iter()
                .map(|f| {
                    let rel = f
                        .strip_prefix(root)
                        .unwrap_or(f)
                        .to_string_lossy()
                        .into_owned();
                    (f, rel)
                })
                .collect();

            for (plugin, matchers) in &config_matchers {
                for (abs_path, rel_path) in &relative_files {
                    if matchers.iter().any(|m| m.is_match(rel_path.as_str())) {
                        // Found a config file — parse it
                        if let Ok(source) = std::fs::read_to_string(abs_path) {
                            let plugin_result = plugin.resolve_config(abs_path, &source, root);
                            if !plugin_result.is_empty() {
                                tracing::debug!(
                                    plugin = plugin.name(),
                                    config = rel_path.as_str(),
                                    entries = plugin_result.entry_patterns.len(),
                                    deps = plugin_result.referenced_dependencies.len(),
                                    "resolved config"
                                );
                                result.entry_patterns.extend(plugin_result.entry_patterns);
                                result
                                    .referenced_dependencies
                                    .extend(plugin_result.referenced_dependencies);
                                result
                                    .discovered_always_used
                                    .extend(plugin_result.always_used_files);
                                result.setup_files.extend(plugin_result.setup_files);
                            }
                        }
                    }
                }
            }
        }

        result
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
