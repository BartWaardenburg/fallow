//! Nuxt framework plugin.
//!
//! Detects Nuxt projects and marks pages, layouts, middleware, server API,
//! plugins, composables, and utils as entry points. Recognizes conventional
//! server API and middleware exports. Parses nuxt.config.ts to extract modules,
//! CSS files, plugins, and other configuration.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

const ENABLERS: &[&str] = &["nuxt"];

const ENTRY_PATTERNS: &[&str] = &[
    // Standard Nuxt directories
    "pages/**/*.{vue,ts,tsx,js,jsx}",
    "layouts/**/*.{vue,ts,tsx,js,jsx}",
    "middleware/**/*.{ts,js}",
    "server/api/**/*.{ts,js}",
    "server/routes/**/*.{ts,js}",
    "server/middleware/**/*.{ts,js}",
    "server/utils/**/*.{ts,js}",
    "plugins/**/*.{ts,js}",
    "composables/**/*.{ts,js}",
    "utils/**/*.{ts,js}",
    "components/**/*.{vue,ts,tsx,js,jsx}",
    // Nuxt auto-scans modules/ for custom modules
    "modules/**/*.{ts,js}",
    // Nuxt 3 app/ directory structure
    "app/pages/**/*.{vue,ts,tsx,js,jsx}",
    "app/layouts/**/*.{vue,ts,tsx,js,jsx}",
    "app/middleware/**/*.{ts,js}",
    "app/plugins/**/*.{ts,js}",
    "app/composables/**/*.{ts,js}",
    "app/utils/**/*.{ts,js}",
    "app/components/**/*.{vue,ts,tsx,js,jsx}",
    "app/modules/**/*.{ts,js}",
];

const CONFIG_PATTERNS: &[&str] = &["nuxt.config.{ts,js}"];

const ALWAYS_USED: &[&str] = &[
    "nuxt.config.{ts,js}",
    "app.vue",
    "app.config.{ts,js}",
    "error.vue",
    // Nuxt 3 app/ directory structure
    "app/app.vue",
    "app/error.vue",
];

/// Implicit dependencies that Nuxt provides — these should not be flagged as unlisted.
const TOOLING_DEPENDENCIES: &[&str] = &[
    "nuxt",
    "@nuxt/devtools",
    "@nuxt/test-utils",
    "@nuxt/schema",
    "@nuxt/kit",
    // Implicit Nuxt runtime dependencies (re-exported by Nuxt at build time)
    "vue",
    "vue-router",
    "ofetch",
    "h3",
    "@unhead/vue",
    "@unhead/schema",
    "nitropack",
    "defu",
    "hookable",
    "ufo",
    "unctx",
    "unenv",
    "ohash",
    "pathe",
    "scule",
    "unimport",
    "unstorage",
    "radix3",
    "cookie-es",
    "crossws",
    "consola",
];

const USED_EXPORTS_SERVER_API: &[&str] = &["default", "defineEventHandler"];
const USED_EXPORTS_MIDDLEWARE: &[&str] = &["default"];

/// Virtual module prefixes provided by Nuxt at build time.
const VIRTUAL_MODULE_PREFIXES: &[&str] = &["#"];

pub struct NuxtPlugin;

impl Plugin for NuxtPlugin {
    fn name(&self) -> &'static str {
        "nuxt"
    }

    fn enablers(&self) -> &'static [&'static str] {
        ENABLERS
    }

    fn entry_patterns(&self) -> &'static [&'static str] {
        ENTRY_PATTERNS
    }

    fn config_patterns(&self) -> &'static [&'static str] {
        CONFIG_PATTERNS
    }

    fn always_used(&self) -> &'static [&'static str] {
        ALWAYS_USED
    }

    fn tooling_dependencies(&self) -> &'static [&'static str] {
        TOOLING_DEPENDENCIES
    }

    fn virtual_module_prefixes(&self) -> &'static [&'static str] {
        VIRTUAL_MODULE_PREFIXES
    }

    fn path_aliases(&self, root: &Path) -> Vec<(&'static str, String)> {
        // Nuxt's srcDir defaults to `app/` when the directory exists, otherwise root.
        let src_dir = if root.join("app").is_dir() {
            "app".to_string()
        } else {
            String::new()
        };
        let mut aliases = vec![
            // ~/  → srcDir (app/ or root)
            ("~/", src_dir.clone()),
            // ~~/ → rootDir (project root)
            ("~~/", String::new()),
            // #shared/ → shared/ directory
            ("#shared/", "shared".to_string()),
            // #server/ → server/ directory
            ("#server/", "server".to_string()),
        ];
        // Also map the bare `~` and `~~` (without trailing slash) for edge cases
        // like `import '~/composables/foo'` — already covered by `~/` prefix.
        // Map #shared (without slash) for bare imports like `import '#shared'`
        aliases.push(("#shared", "shared".to_string()));
        aliases.push(("#server", "server".to_string()));
        aliases
    }

    fn used_exports(&self) -> Vec<(&'static str, &'static [&'static str])> {
        vec![
            ("server/api/**/*.{ts,js}", USED_EXPORTS_SERVER_API),
            ("middleware/**/*.{ts,js}", USED_EXPORTS_MIDDLEWARE),
        ]
    }

    fn resolve_config(&self, config_path: &Path, source: &str, root: &Path) -> PluginResult {
        let mut result = PluginResult::default();

        // Detect whether this project uses the app/ directory structure.
        // In Nuxt 3, `~` resolves to srcDir (defaults to `app/` when the directory exists).
        let has_app_dir = root.join("app").is_dir();

        // Extract import sources as referenced dependencies
        let imports = config_parser::extract_imports(source, config_path);
        for imp in &imports {
            let dep = crate::resolve::extract_package_name(imp);
            result.referenced_dependencies.push(dep);
        }

        // modules: [...] → referenced dependencies (Nuxt modules are npm packages)
        let modules = config_parser::extract_config_string_array(source, config_path, &["modules"]);
        for module in &modules {
            let dep = crate::resolve::extract_package_name(module);
            result.referenced_dependencies.push(dep);
        }

        // css: [...] → always-used files or referenced dependencies
        // Nuxt aliases: `~/` = srcDir (app/ or root), `~~/` = rootDir
        // npm package CSS (e.g., `@unocss/reset/tailwind.css`) → referenced dependency
        let css = config_parser::extract_config_string_array(source, config_path, &["css"]);
        for entry in &css {
            if let Some(stripped) = entry.strip_prefix("~/") {
                // ~ = srcDir: resolve to app/ if it exists, otherwise project root
                if has_app_dir {
                    result.always_used_files.push(format!("app/{stripped}"));
                } else {
                    result.always_used_files.push(stripped.to_string());
                }
            } else if let Some(stripped) = entry.strip_prefix("~~/") {
                // ~~ = rootDir: always relative to project root
                result.always_used_files.push(stripped.to_string());
            } else if entry.starts_with('.') || entry.starts_with('/') {
                // Relative or absolute local path
                result.always_used_files.push(entry.clone());
            } else {
                // npm package CSS (e.g., `@unocss/reset/tailwind.css`, `floating-vue/dist/style.css`)
                let dep = crate::resolve::extract_package_name(entry);
                result.referenced_dependencies.push(dep);
            }
        }

        // postcss.plugins → referenced dependencies (object keys)
        let postcss_plugins =
            config_parser::extract_config_object_keys(source, config_path, &["postcss", "plugins"]);
        for plugin in &postcss_plugins {
            result
                .referenced_dependencies
                .push(crate::resolve::extract_package_name(plugin));
        }

        // plugins: [...] → entry patterns
        let plugins = config_parser::extract_config_string_array(source, config_path, &["plugins"]);
        result.entry_patterns.extend(plugins);

        // extends: [...] → referenced dependencies
        let extends = config_parser::extract_config_string_array(source, config_path, &["extends"]);
        for ext in &extends {
            result
                .referenced_dependencies
                .push(crate::resolve::extract_package_name(ext));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabler_is_nuxt() {
        let plugin = NuxtPlugin;
        assert_eq!(plugin.enablers(), &["nuxt"]);
    }

    #[test]
    fn is_enabled_with_nuxt_dep() {
        let plugin = NuxtPlugin;
        let deps = vec!["nuxt".to_string()];
        assert!(plugin.is_enabled_with_deps(&deps, Path::new("/project")));
    }

    #[test]
    fn is_not_enabled_without_nuxt() {
        let plugin = NuxtPlugin;
        let deps = vec!["vue".to_string()];
        assert!(!plugin.is_enabled_with_deps(&deps, Path::new("/project")));
    }

    #[test]
    fn entry_patterns_include_nuxt_conventions() {
        let plugin = NuxtPlugin;
        let patterns = plugin.entry_patterns();
        assert!(patterns.iter().any(|p| p.starts_with("pages/")));
        assert!(patterns.iter().any(|p| p.starts_with("layouts/")));
        assert!(patterns.iter().any(|p| p.starts_with("server/api/")));
        assert!(patterns.iter().any(|p| p.starts_with("composables/")));
        assert!(patterns.iter().any(|p| p.starts_with("components/")));
    }

    #[test]
    fn entry_patterns_include_app_dir_variants() {
        let plugin = NuxtPlugin;
        let patterns = plugin.entry_patterns();
        assert!(
            patterns.iter().any(|p| p.starts_with("app/pages/")),
            "should include Nuxt 3 app/ directory variants"
        );
    }

    #[test]
    fn virtual_module_prefixes_includes_hash() {
        let plugin = NuxtPlugin;
        assert_eq!(plugin.virtual_module_prefixes(), &["#"]);
    }

    #[test]
    fn used_exports_for_server_api() {
        let plugin = NuxtPlugin;
        let exports = plugin.used_exports();
        let api_entry = exports
            .iter()
            .find(|(pat, _)| *pat == "server/api/**/*.{ts,js}");
        assert!(api_entry.is_some());
        let (_, names) = api_entry.unwrap();
        assert!(names.contains(&"default"));
        assert!(names.contains(&"defineEventHandler"));
    }

    // ── resolve_config tests ─────────────────────────────────────

    #[test]
    fn resolve_config_modules_as_deps() {
        let source = r#"
            export default defineNuxtConfig({
                modules: ["@nuxtjs/tailwindcss", "@pinia/nuxt"]
            });
        "#;
        let plugin = NuxtPlugin;
        let result =
            plugin.resolve_config(Path::new("nuxt.config.ts"), source, Path::new("/project"));
        assert!(
            result
                .referenced_dependencies
                .contains(&"@nuxtjs/tailwindcss".to_string())
        );
        assert!(
            result
                .referenced_dependencies
                .contains(&"@pinia/nuxt".to_string())
        );
    }

    #[test]
    fn resolve_config_css_tilde_resolves_to_root() {
        // Without an `app/` dir, `~/` resolves to project root
        let source = r#"
            export default defineNuxtConfig({
                css: ["~/assets/main.css"]
            });
        "#;
        let plugin = NuxtPlugin;
        let result = plugin.resolve_config(
            Path::new("nuxt.config.ts"),
            source,
            Path::new("/nonexistent"),
        );
        assert!(
            result
                .always_used_files
                .contains(&"assets/main.css".to_string()),
            "~/assets/main.css should resolve to assets/main.css without app/ dir: {:?}",
            result.always_used_files
        );
    }

    #[test]
    fn resolve_config_css_double_tilde_always_root() {
        let source = r#"
            export default defineNuxtConfig({
                css: ["~~/shared/global.css"]
            });
        "#;
        let plugin = NuxtPlugin;
        let result = plugin.resolve_config(
            Path::new("nuxt.config.ts"),
            source,
            Path::new("/nonexistent"),
        );
        assert!(
            result
                .always_used_files
                .contains(&"shared/global.css".to_string()),
            "~~/shared/global.css should resolve to shared/global.css"
        );
    }

    #[test]
    fn resolve_config_css_npm_package() {
        let source = r#"
            export default defineNuxtConfig({
                css: ["@unocss/reset/tailwind.css"]
            });
        "#;
        let plugin = NuxtPlugin;
        let result =
            plugin.resolve_config(Path::new("nuxt.config.ts"), source, Path::new("/project"));
        assert!(
            result
                .referenced_dependencies
                .contains(&"@unocss/reset".to_string()),
            "npm package CSS should be tracked as referenced dependency"
        );
    }

    #[test]
    fn resolve_config_postcss_plugins_as_deps() {
        let source = r#"
            export default defineNuxtConfig({
                postcss: {
                    plugins: {
                        autoprefixer: {},
                        "postcss-nested": {}
                    }
                }
            });
        "#;
        let plugin = NuxtPlugin;
        let result =
            plugin.resolve_config(Path::new("nuxt.config.ts"), source, Path::new("/project"));
        assert!(
            result
                .referenced_dependencies
                .contains(&"autoprefixer".to_string())
        );
        assert!(
            result
                .referenced_dependencies
                .contains(&"postcss-nested".to_string())
        );
    }

    #[test]
    fn resolve_config_extends_as_deps() {
        let source = r#"
            export default defineNuxtConfig({
                extends: ["@nuxt/ui-pro"]
            });
        "#;
        let plugin = NuxtPlugin;
        let result =
            plugin.resolve_config(Path::new("nuxt.config.ts"), source, Path::new("/project"));
        assert!(
            result
                .referenced_dependencies
                .contains(&"@nuxt/ui-pro".to_string())
        );
    }

    #[test]
    fn resolve_config_import_sources_as_deps() {
        let source = r#"
            import { defineNuxtConfig } from "nuxt/config";
            export default defineNuxtConfig({});
        "#;
        let plugin = NuxtPlugin;
        let result =
            plugin.resolve_config(Path::new("nuxt.config.ts"), source, Path::new("/project"));
        assert!(
            result.referenced_dependencies.contains(&"nuxt".to_string()),
            "import source should be extracted as a referenced dependency"
        );
    }

    #[test]
    fn resolve_config_empty_source() {
        let plugin = NuxtPlugin;
        let result = plugin.resolve_config(Path::new("nuxt.config.ts"), "", Path::new("/project"));
        assert!(result.referenced_dependencies.is_empty());
        assert!(result.always_used_files.is_empty());
        assert!(result.entry_patterns.is_empty());
    }

    #[test]
    fn resolve_config_css_relative_path() {
        let source = r#"
            export default defineNuxtConfig({
                css: ["./assets/global.css"]
            });
        "#;
        let plugin = NuxtPlugin;
        let result =
            plugin.resolve_config(Path::new("nuxt.config.ts"), source, Path::new("/project"));
        assert!(
            result
                .always_used_files
                .contains(&"./assets/global.css".to_string()),
            "relative CSS path should be an always-used file"
        );
    }
}
