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

use rustc_hash::FxHashSet;
use std::path::{Path, PathBuf};

use fallow_config::{ExternalPluginDef, PackageJson, PluginDetection};

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
/// ```
///
/// All fields except `struct` and `enablers` are optional and default to `&[]` / `vec![]`.
macro_rules! define_plugin {
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
mod tooling;

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

/// Registry of all available plugins (built-in + external).
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
    external_plugins: Vec<ExternalPluginDef>,
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
    /// Package names discovered as used in package.json scripts (binary invocations).
    pub script_used_packages: FxHashSet<String>,
    /// Import prefixes for virtual modules provided by active frameworks.
    /// Imports matching these prefixes should not be flagged as unlisted dependencies.
    pub virtual_module_prefixes: Vec<String>,
    /// Path alias mappings from active plugins (prefix → replacement directory).
    /// Used by the resolver to substitute import prefixes before re-resolving.
    pub path_aliases: Vec<(String, String)>,
    /// Names of active plugins.
    pub active_plugins: Vec<String>,
}

impl PluginRegistry {
    /// Create a registry with all built-in plugins and optional external plugins.
    pub fn new(external: Vec<ExternalPluginDef>) -> Self {
        let plugins: Vec<Box<dyn Plugin>> = vec![
            // Frameworks
            Box::new(nextjs::NextJsPlugin),
            Box::new(nuxt::NuxtPlugin),
            Box::new(remix::RemixPlugin),
            Box::new(astro::AstroPlugin),
            Box::new(angular::AngularPlugin),
            Box::new(react_router::ReactRouterPlugin),
            Box::new(tanstack_router::TanstackRouterPlugin),
            Box::new(react_native::ReactNativePlugin),
            Box::new(expo::ExpoPlugin),
            Box::new(nestjs::NestJsPlugin),
            Box::new(docusaurus::DocusaurusPlugin),
            Box::new(gatsby::GatsbyPlugin),
            Box::new(sveltekit::SvelteKitPlugin),
            Box::new(nitro::NitroPlugin),
            Box::new(capacitor::CapacitorPlugin),
            Box::new(sanity::SanityPlugin),
            Box::new(vitepress::VitePressPlugin),
            Box::new(next_intl::NextIntlPlugin),
            Box::new(relay::RelayPlugin),
            Box::new(electron::ElectronPlugin),
            Box::new(i18next::I18nextPlugin),
            // Bundlers
            Box::new(vite::VitePlugin),
            Box::new(webpack::WebpackPlugin),
            Box::new(rollup::RollupPlugin),
            Box::new(rolldown::RolldownPlugin),
            Box::new(rspack::RspackPlugin),
            Box::new(rsbuild::RsbuildPlugin),
            Box::new(tsup::TsupPlugin),
            Box::new(tsdown::TsdownPlugin),
            Box::new(parcel::ParcelPlugin),
            // Testing
            Box::new(vitest::VitestPlugin),
            Box::new(jest::JestPlugin),
            Box::new(playwright::PlaywrightPlugin),
            Box::new(cypress::CypressPlugin),
            Box::new(mocha::MochaPlugin),
            Box::new(ava::AvaPlugin),
            Box::new(storybook::StorybookPlugin),
            Box::new(karma::KarmaPlugin),
            Box::new(cucumber::CucumberPlugin),
            Box::new(webdriverio::WebdriverioPlugin),
            // Linting & formatting
            Box::new(eslint::EslintPlugin),
            Box::new(biome::BiomePlugin),
            Box::new(stylelint::StylelintPlugin),
            Box::new(prettier::PrettierPlugin),
            Box::new(oxlint::OxlintPlugin),
            Box::new(markdownlint::MarkdownlintPlugin),
            Box::new(cspell::CspellPlugin),
            Box::new(remark::RemarkPlugin),
            // Transpilation & language
            Box::new(typescript::TypeScriptPlugin),
            Box::new(babel::BabelPlugin),
            Box::new(swc::SwcPlugin),
            // CSS
            Box::new(tailwind::TailwindPlugin),
            Box::new(postcss::PostCssPlugin),
            // Database & ORM
            Box::new(prisma::PrismaPlugin),
            Box::new(drizzle::DrizzlePlugin),
            Box::new(knex::KnexPlugin),
            Box::new(typeorm::TypeormPlugin),
            Box::new(kysely::KyselyPlugin),
            // Monorepo
            Box::new(turborepo::TurborepoPlugin),
            Box::new(nx::NxPlugin),
            Box::new(changesets::ChangesetsPlugin),
            Box::new(syncpack::SyncpackPlugin),
            // CI/CD & release
            Box::new(commitlint::CommitlintPlugin),
            Box::new(commitizen::CommitizenPlugin),
            Box::new(semantic_release::SemanticReleasePlugin),
            // Deployment
            Box::new(wrangler::WranglerPlugin),
            Box::new(sentry::SentryPlugin),
            // Git hooks
            Box::new(husky::HuskyPlugin),
            Box::new(lint_staged::LintStagedPlugin),
            Box::new(lefthook::LefthookPlugin),
            Box::new(simple_git_hooks::SimpleGitHooksPlugin),
            // Media & assets
            Box::new(svgo::SvgoPlugin),
            Box::new(svgr::SvgrPlugin),
            // Code generation & docs
            Box::new(graphql_codegen::GraphqlCodegenPlugin),
            Box::new(typedoc::TypedocPlugin),
            Box::new(openapi_ts::OpenapiTsPlugin),
            Box::new(plop::PlopPlugin),
            // Coverage
            Box::new(c8::C8Plugin),
            Box::new(nyc::NycPlugin),
            // Other tools
            Box::new(msw::MswPlugin),
            Box::new(nodemon::NodemonPlugin),
            Box::new(pm2::Pm2Plugin),
            Box::new(dependency_cruiser::DependencyCruiserPlugin),
            // Runtime
            Box::new(bun::BunPlugin),
        ];
        Self {
            plugins,
            external_plugins: external,
        }
    }

    /// Run all plugins against a project, returning aggregated results.
    ///
    /// This discovers which plugins are active, collects their static patterns,
    /// then parses any config files to extract dynamic information.
    #[allow(clippy::cognitive_complexity)] // Plugin discovery and aggregation logic is inherently complex
    pub fn run(
        &self,
        pkg: &PackageJson,
        root: &Path,
        discovered_files: &[PathBuf],
    ) -> AggregatedPluginResult {
        let _span = tracing::info_span!("run_plugins").entered();
        let mut result = AggregatedPluginResult::default();

        // Phase 1: Determine which plugins are active
        // Compute deps once to avoid repeated Vec<String> allocation per plugin
        let all_deps = pkg.all_dependency_names();
        let active: Vec<&dyn Plugin> = self
            .plugins
            .iter()
            .filter(|p| p.is_enabled_with_deps(&all_deps, root))
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
            for prefix in plugin.virtual_module_prefixes() {
                result.virtual_module_prefixes.push((*prefix).to_string());
            }
            for (prefix, replacement) in plugin.path_aliases(root) {
                result.path_aliases.push((prefix.to_string(), replacement));
            }
        }

        // Phase 2b: Process external plugins (includes inline framework definitions)
        // Reuse `all_deps` from Phase 1 (already computed above)
        let all_dep_refs: Vec<&str> = all_deps.iter().map(|s| s.as_str()).collect();
        for ext in &self.external_plugins {
            #[allow(clippy::option_if_let_else)] // Chained if-let-else is more readable here
            let is_active = if let Some(detection) = &ext.detection {
                check_plugin_detection(detection, &all_dep_refs, root, discovered_files)
            } else if !ext.enablers.is_empty() {
                ext.enablers.iter().any(|enabler| {
                    if enabler.ends_with('/') {
                        all_deps.iter().any(|d| d.starts_with(enabler))
                    } else {
                        all_deps.iter().any(|d| d == enabler)
                    }
                })
            } else {
                false
            };
            if is_active {
                result.active_plugins.push(ext.name.clone());
                result.entry_patterns.extend(ext.entry_points.clone());
                // Track config patterns for introspection (not used for AST parsing —
                // external plugins cannot do resolve_config())
                result.config_patterns.extend(ext.config_patterns.clone());
                result.always_used.extend(ext.config_patterns.clone());
                result.always_used.extend(ext.always_used.clone());
                result
                    .tooling_dependencies
                    .extend(ext.tooling_dependencies.clone());
                for ue in &ext.used_exports {
                    result
                        .used_exports
                        .push((ue.pattern.clone(), ue.exports.clone()));
                }
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

        // Build relative paths for matching (used by Phase 3 and 4)
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

        if !config_matchers.is_empty() {
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

        // Phase 4: Package.json inline config fallback
        // For plugins that define `package_json_config_key()`, check if the root
        // package.json contains that key and no standalone config file was found.
        for plugin in &active {
            if let Some(key) = plugin.package_json_config_key() {
                // Check if any config file was already found for this plugin
                let has_config_file = !plugin.config_patterns().is_empty()
                    && config_matchers.iter().any(|(p, matchers)| {
                        p.name() == plugin.name()
                            && relative_files
                                .iter()
                                .any(|(_, rel)| matchers.iter().any(|m| m.is_match(rel.as_str())))
                    });
                if !has_config_file {
                    // Try to extract the key from package.json
                    let pkg_path = root.join("package.json");
                    if let Ok(content) = std::fs::read_to_string(&pkg_path)
                        && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
                        && let Some(config_value) = json.get(key)
                    {
                        let config_json = serde_json::to_string(config_value).unwrap_or_default();
                        let fake_path = root.join(format!("{key}.config.json"));
                        let plugin_result = plugin.resolve_config(&fake_path, &config_json, root);
                        if !plugin_result.is_empty() {
                            tracing::debug!(
                                plugin = plugin.name(),
                                key = key,
                                "resolved inline package.json config"
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

        result
    }

    /// Fast variant of `run()` for workspace packages.
    ///
    /// Reuses pre-compiled config matchers and pre-computed relative files from the root
    /// project run, avoiding repeated glob compilation and path computation per workspace.
    /// Skips external plugins (they only activate at root level) and package.json inline
    /// config (workspace packages rarely have inline configs).
    pub fn run_workspace_fast(
        &self,
        pkg: &PackageJson,
        root: &Path,
        precompiled_config_matchers: &[(&dyn Plugin, Vec<globset::GlobMatcher>)],
        relative_files: &[(&PathBuf, String)],
    ) -> AggregatedPluginResult {
        let _span = tracing::info_span!("run_plugins").entered();
        let mut result = AggregatedPluginResult::default();

        // Phase 1: Determine which plugins are active (with pre-computed deps)
        let all_deps = pkg.all_dependency_names();
        let active: Vec<&dyn Plugin> = self
            .plugins
            .iter()
            .filter(|p| p.is_enabled_with_deps(&all_deps, root))
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

        // Early exit if no plugins are active (common for leaf workspace packages)
        if active.is_empty() {
            return result;
        }

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
            for prefix in plugin.virtual_module_prefixes() {
                result.virtual_module_prefixes.push((*prefix).to_string());
            }
            for (prefix, replacement) in plugin.path_aliases(root) {
                result.path_aliases.push((prefix.to_string(), replacement));
            }
        }

        // Phase 3: Find and parse config files using pre-compiled matchers
        // Only check matchers for plugins that are active in this workspace
        let active_names: FxHashSet<&str> = active.iter().map(|p| p.name()).collect();
        let workspace_matchers: Vec<_> = precompiled_config_matchers
            .iter()
            .filter(|(p, _)| active_names.contains(p.name()))
            .collect();

        if !workspace_matchers.is_empty() {
            for (plugin, matchers) in workspace_matchers {
                for (abs_path, rel_path) in relative_files {
                    if matchers.iter().any(|m| m.is_match(rel_path.as_str()))
                        && let Ok(source) = std::fs::read_to_string(abs_path)
                    {
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

        result
    }

    /// Pre-compile config pattern glob matchers for all plugins that have config patterns.
    /// Returns a vec of (plugin, matchers) pairs that can be reused across multiple `run_workspace_fast` calls.
    pub fn precompile_config_matchers(&self) -> Vec<(&dyn Plugin, Vec<globset::GlobMatcher>)> {
        self.plugins
            .iter()
            .filter(|p| !p.config_patterns().is_empty())
            .map(|p| {
                let matchers: Vec<globset::GlobMatcher> = p
                    .config_patterns()
                    .iter()
                    .filter_map(|pat| globset::Glob::new(pat).ok().map(|g| g.compile_matcher()))
                    .collect();
                (p.as_ref(), matchers)
            })
            .collect()
    }
}

/// Check if a `PluginDetection` condition is satisfied.
fn check_plugin_detection(
    detection: &PluginDetection,
    all_deps: &[&str],
    root: &Path,
    discovered_files: &[PathBuf],
) -> bool {
    match detection {
        PluginDetection::Dependency { package } => all_deps.iter().any(|d| *d == package),
        PluginDetection::FileExists { pattern } => {
            // Check against discovered files first (fast path)
            if let Ok(matcher) = globset::Glob::new(pattern).map(|g| g.compile_matcher()) {
                for file in discovered_files {
                    let relative = file.strip_prefix(root).unwrap_or(file);
                    if matcher.is_match(relative) {
                        return true;
                    }
                }
            }
            // Fall back to glob on disk for non-source files (e.g., config files)
            let full_pattern = root.join(pattern).to_string_lossy().to_string();
            glob::glob(&full_pattern)
                .ok()
                .is_some_and(|mut g| g.next().is_some())
        }
        PluginDetection::All { conditions } => conditions
            .iter()
            .all(|c| check_plugin_detection(c, all_deps, root, discovered_files)),
        PluginDetection::Any { conditions } => conditions
            .iter()
            .any(|c| check_plugin_detection(c, all_deps, root, discovered_files)),
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new(vec![])
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_types)]
mod tests {
    use super::*;
    use fallow_config::{ExternalPluginDef, ExternalUsedExport, PluginDetection};
    use std::collections::HashMap;

    /// Helper: build a PackageJson with given dependency names.
    fn make_pkg(deps: &[&str]) -> PackageJson {
        let map: HashMap<String, String> =
            deps.iter().map(|d| (d.to_string(), "*".into())).collect();
        PackageJson {
            dependencies: Some(map),
            ..Default::default()
        }
    }

    /// Helper: build a PackageJson with dev dependencies.
    fn make_pkg_dev(deps: &[&str]) -> PackageJson {
        let map: HashMap<String, String> =
            deps.iter().map(|d| (d.to_string(), "*".into())).collect();
        PackageJson {
            dev_dependencies: Some(map),
            ..Default::default()
        }
    }

    // ── Plugin detection via enablers ────────────────────────────

    #[test]
    fn nextjs_detected_when_next_in_deps() {
        let registry = PluginRegistry::default();
        let pkg = make_pkg(&["next", "react"]);
        let result = registry.run(&pkg, Path::new("/project"), &[]);
        assert!(
            result.active_plugins.contains(&"nextjs".to_string()),
            "nextjs plugin should be active when 'next' is in deps"
        );
    }

    #[test]
    fn nextjs_not_detected_without_next() {
        let registry = PluginRegistry::default();
        let pkg = make_pkg(&["react", "react-dom"]);
        let result = registry.run(&pkg, Path::new("/project"), &[]);
        assert!(
            !result.active_plugins.contains(&"nextjs".to_string()),
            "nextjs plugin should not be active without 'next' in deps"
        );
    }

    #[test]
    fn prefix_enabler_matches_scoped_packages() {
        // Storybook uses "@storybook/" prefix matcher
        let registry = PluginRegistry::default();
        let pkg = make_pkg(&["@storybook/react"]);
        let result = registry.run(&pkg, Path::new("/project"), &[]);
        assert!(
            result.active_plugins.contains(&"storybook".to_string()),
            "storybook should activate via prefix match on @storybook/react"
        );
    }

    #[test]
    fn prefix_enabler_does_not_match_without_slash() {
        // "storybook" (exact) should match, but "@storybook" (without /) should not match via prefix
        let registry = PluginRegistry::default();
        // This only has a package called "@storybookish" — it should NOT match
        let mut map = HashMap::new();
        map.insert("@storybookish".to_string(), "*".to_string());
        let pkg = PackageJson {
            dependencies: Some(map),
            ..Default::default()
        };
        let result = registry.run(&pkg, Path::new("/project"), &[]);
        assert!(
            !result.active_plugins.contains(&"storybook".to_string()),
            "storybook should not activate for '@storybookish' (no slash prefix match)"
        );
    }

    #[test]
    fn multiple_plugins_detected_simultaneously() {
        let registry = PluginRegistry::default();
        let pkg = make_pkg(&["next", "vitest", "typescript"]);
        let result = registry.run(&pkg, Path::new("/project"), &[]);
        assert!(result.active_plugins.contains(&"nextjs".to_string()));
        assert!(result.active_plugins.contains(&"vitest".to_string()));
        assert!(result.active_plugins.contains(&"typescript".to_string()));
    }

    #[test]
    fn no_plugins_for_empty_deps() {
        let registry = PluginRegistry::default();
        let pkg = PackageJson::default();
        let result = registry.run(&pkg, Path::new("/project"), &[]);
        assert!(
            result.active_plugins.is_empty(),
            "no plugins should activate with empty package.json"
        );
    }

    // ── Aggregation: entry patterns, tooling deps ────────────────

    #[test]
    fn active_plugin_contributes_entry_patterns() {
        let registry = PluginRegistry::default();
        let pkg = make_pkg(&["next"]);
        let result = registry.run(&pkg, Path::new("/project"), &[]);
        // Next.js should contribute App Router entry patterns
        assert!(
            result
                .entry_patterns
                .iter()
                .any(|p| p.contains("app/**/page")),
            "nextjs plugin should add app/**/page entry pattern"
        );
    }

    #[test]
    fn inactive_plugin_does_not_contribute_entry_patterns() {
        let registry = PluginRegistry::default();
        let pkg = make_pkg(&["react"]);
        let result = registry.run(&pkg, Path::new("/project"), &[]);
        // Next.js patterns should not be present
        assert!(
            !result
                .entry_patterns
                .iter()
                .any(|p| p.contains("app/**/page")),
            "nextjs patterns should not appear when plugin is inactive"
        );
    }

    #[test]
    fn active_plugin_contributes_tooling_deps() {
        let registry = PluginRegistry::default();
        let pkg = make_pkg(&["next"]);
        let result = registry.run(&pkg, Path::new("/project"), &[]);
        assert!(
            result.tooling_dependencies.contains(&"next".to_string()),
            "nextjs plugin should list 'next' as a tooling dependency"
        );
    }

    #[test]
    fn dev_deps_also_trigger_plugins() {
        let registry = PluginRegistry::default();
        let pkg = make_pkg_dev(&["vitest"]);
        let result = registry.run(&pkg, Path::new("/project"), &[]);
        assert!(
            result.active_plugins.contains(&"vitest".to_string()),
            "vitest should activate from devDependencies"
        );
    }

    // ── External plugins ─────────────────────────────────────────

    #[test]
    fn external_plugin_detected_by_enablers() {
        let ext = ExternalPluginDef {
            schema: None,
            name: "my-framework".to_string(),
            detection: None,
            enablers: vec!["my-framework".to_string()],
            entry_points: vec!["src/routes/**/*.ts".to_string()],
            config_patterns: vec![],
            always_used: vec!["my.config.ts".to_string()],
            tooling_dependencies: vec!["my-framework-cli".to_string()],
            used_exports: vec![],
        };
        let registry = PluginRegistry::new(vec![ext]);
        let pkg = make_pkg(&["my-framework"]);
        let result = registry.run(&pkg, Path::new("/project"), &[]);
        assert!(result.active_plugins.contains(&"my-framework".to_string()));
        assert!(
            result
                .entry_patterns
                .contains(&"src/routes/**/*.ts".to_string())
        );
        assert!(
            result
                .tooling_dependencies
                .contains(&"my-framework-cli".to_string())
        );
    }

    #[test]
    fn external_plugin_not_detected_when_dep_missing() {
        let ext = ExternalPluginDef {
            schema: None,
            name: "my-framework".to_string(),
            detection: None,
            enablers: vec!["my-framework".to_string()],
            entry_points: vec!["src/routes/**/*.ts".to_string()],
            config_patterns: vec![],
            always_used: vec![],
            tooling_dependencies: vec![],
            used_exports: vec![],
        };
        let registry = PluginRegistry::new(vec![ext]);
        let pkg = make_pkg(&["react"]);
        let result = registry.run(&pkg, Path::new("/project"), &[]);
        assert!(!result.active_plugins.contains(&"my-framework".to_string()));
        assert!(
            !result
                .entry_patterns
                .contains(&"src/routes/**/*.ts".to_string())
        );
    }

    #[test]
    fn external_plugin_prefix_enabler() {
        let ext = ExternalPluginDef {
            schema: None,
            name: "custom-plugin".to_string(),
            detection: None,
            enablers: vec!["@custom/".to_string()],
            entry_points: vec!["custom/**/*.ts".to_string()],
            config_patterns: vec![],
            always_used: vec![],
            tooling_dependencies: vec![],
            used_exports: vec![],
        };
        let registry = PluginRegistry::new(vec![ext]);
        let pkg = make_pkg(&["@custom/core"]);
        let result = registry.run(&pkg, Path::new("/project"), &[]);
        assert!(result.active_plugins.contains(&"custom-plugin".to_string()));
    }

    #[test]
    fn external_plugin_detection_dependency() {
        let ext = ExternalPluginDef {
            schema: None,
            name: "detected-plugin".to_string(),
            detection: Some(PluginDetection::Dependency {
                package: "special-dep".to_string(),
            }),
            enablers: vec![],
            entry_points: vec!["special/**/*.ts".to_string()],
            config_patterns: vec![],
            always_used: vec![],
            tooling_dependencies: vec![],
            used_exports: vec![],
        };
        let registry = PluginRegistry::new(vec![ext]);
        let pkg = make_pkg(&["special-dep"]);
        let result = registry.run(&pkg, Path::new("/project"), &[]);
        assert!(
            result
                .active_plugins
                .contains(&"detected-plugin".to_string())
        );
    }

    #[test]
    fn external_plugin_detection_any_combinator() {
        let ext = ExternalPluginDef {
            schema: None,
            name: "any-plugin".to_string(),
            detection: Some(PluginDetection::Any {
                conditions: vec![
                    PluginDetection::Dependency {
                        package: "pkg-a".to_string(),
                    },
                    PluginDetection::Dependency {
                        package: "pkg-b".to_string(),
                    },
                ],
            }),
            enablers: vec![],
            entry_points: vec!["any/**/*.ts".to_string()],
            config_patterns: vec![],
            always_used: vec![],
            tooling_dependencies: vec![],
            used_exports: vec![],
        };
        let registry = PluginRegistry::new(vec![ext]);
        // Only pkg-b present — should still match via Any
        let pkg = make_pkg(&["pkg-b"]);
        let result = registry.run(&pkg, Path::new("/project"), &[]);
        assert!(result.active_plugins.contains(&"any-plugin".to_string()));
    }

    #[test]
    fn external_plugin_detection_all_combinator_fails_partial() {
        let ext = ExternalPluginDef {
            schema: None,
            name: "all-plugin".to_string(),
            detection: Some(PluginDetection::All {
                conditions: vec![
                    PluginDetection::Dependency {
                        package: "pkg-a".to_string(),
                    },
                    PluginDetection::Dependency {
                        package: "pkg-b".to_string(),
                    },
                ],
            }),
            enablers: vec![],
            entry_points: vec![],
            config_patterns: vec![],
            always_used: vec![],
            tooling_dependencies: vec![],
            used_exports: vec![],
        };
        let registry = PluginRegistry::new(vec![ext]);
        // Only pkg-a present — All requires both
        let pkg = make_pkg(&["pkg-a"]);
        let result = registry.run(&pkg, Path::new("/project"), &[]);
        assert!(!result.active_plugins.contains(&"all-plugin".to_string()));
    }

    #[test]
    fn external_plugin_used_exports_aggregated() {
        let ext = ExternalPluginDef {
            schema: None,
            name: "ue-plugin".to_string(),
            detection: None,
            enablers: vec!["ue-dep".to_string()],
            entry_points: vec![],
            config_patterns: vec![],
            always_used: vec![],
            tooling_dependencies: vec![],
            used_exports: vec![ExternalUsedExport {
                pattern: "pages/**/*.tsx".to_string(),
                exports: vec!["default".to_string(), "getServerSideProps".to_string()],
            }],
        };
        let registry = PluginRegistry::new(vec![ext]);
        let pkg = make_pkg(&["ue-dep"]);
        let result = registry.run(&pkg, Path::new("/project"), &[]);
        assert!(result.used_exports.iter().any(|(pat, exports)| {
            pat == "pages/**/*.tsx" && exports.contains(&"default".to_string())
        }));
    }

    #[test]
    fn external_plugin_without_enablers_or_detection_stays_inactive() {
        let ext = ExternalPluginDef {
            schema: None,
            name: "orphan-plugin".to_string(),
            detection: None,
            enablers: vec![],
            entry_points: vec!["orphan/**/*.ts".to_string()],
            config_patterns: vec![],
            always_used: vec![],
            tooling_dependencies: vec![],
            used_exports: vec![],
        };
        let registry = PluginRegistry::new(vec![ext]);
        let pkg = make_pkg(&["anything"]);
        let result = registry.run(&pkg, Path::new("/project"), &[]);
        assert!(!result.active_plugins.contains(&"orphan-plugin".to_string()));
    }

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
    fn is_enabled_with_empty_deps() {
        let plugin = nextjs::NextJsPlugin;
        let deps: Vec<String> = vec![];
        assert!(!plugin.is_enabled_with_deps(&deps, Path::new("/project")));
    }

    // ── Virtual module prefixes ──────────────────────────────────

    #[test]
    fn nuxt_contributes_virtual_module_prefixes() {
        let registry = PluginRegistry::default();
        let pkg = make_pkg(&["nuxt"]);
        let result = registry.run(&pkg, Path::new("/project"), &[]);
        assert!(
            result.virtual_module_prefixes.contains(&"#".to_string()),
            "nuxt should contribute '#' virtual module prefix"
        );
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

    // ── Precompile config matchers ───────────────────────────────

    #[test]
    fn precompile_config_matchers_returns_entries() {
        let registry = PluginRegistry::default();
        let matchers = registry.precompile_config_matchers();
        // At minimum, nextjs, vite, jest, typescript, etc. all have config patterns
        assert!(
            !matchers.is_empty(),
            "precompile_config_matchers should return entries for plugins with config patterns"
        );
    }

    #[test]
    fn precompile_config_matchers_only_for_plugins_with_patterns() {
        let registry = PluginRegistry::default();
        let matchers = registry.precompile_config_matchers();
        for (plugin, _) in &matchers {
            assert!(
                !plugin.config_patterns().is_empty(),
                "plugin '{}' in matchers should have config patterns",
                plugin.name()
            );
        }
    }
}
