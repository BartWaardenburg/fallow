//! Angular framework plugin.
//!
//! Detects Angular projects and marks component, module, service, guard,
//! pipe, directive, resolver, and interceptor files as entry points.
//! Parses `angular.json` to extract styles, scripts, main, and polyfills
//! from build targets as additional entry points.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct AngularPlugin;

const ENABLERS: &[&str] = &["@angular/core"];

const ENTRY_PATTERNS: &[&str] = &[
    // Standard Angular CLI layout
    "src/main.ts",
    "src/app/**/*.component.ts",
    "src/app/**/*.module.ts",
    "src/app/**/*.service.ts",
    "src/app/**/*.guard.ts",
    "src/app/**/*.pipe.ts",
    "src/app/**/*.directive.ts",
    "src/app/**/*.resolver.ts",
    "src/app/**/*.interceptor.ts",
    // Nx monorepo layout (apps and libs under arbitrary paths)
    "**/src/main.ts",
    "**/src/app/**/*.component.ts",
    "**/src/app/**/*.module.ts",
    "**/src/app/**/*.service.ts",
    "**/src/app/**/*.guard.ts",
    "**/src/app/**/*.pipe.ts",
    "**/src/app/**/*.directive.ts",
    "**/src/app/**/*.resolver.ts",
    "**/src/app/**/*.interceptor.ts",
];

const CONFIG_PATTERNS: &[&str] = &["angular.json", ".angular.json"];

const ALWAYS_USED: &[&str] = &[
    "angular.json",
    ".angular.json",
    "src/polyfills.ts",
    "src/environments/**/*.ts",
];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "@angular/cli",
    "@angular-devkit/build-angular",
    "@angular/compiler-cli",
    "@angular/compiler",
    "@angular/build",
    "zone.js",
    "tslib",
    // Peer dependencies of @angular/core that may not be directly imported
    // but are required by the Angular framework at runtime
    "rxjs",
    "@angular/common",
    "@angular/platform-browser",
    "@angular/platform-browser-dynamic",
];

impl Plugin for AngularPlugin {
    fn name(&self) -> &'static str {
        "angular"
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

    fn resolve_config(&self, config_path: &Path, source: &str, _root: &Path) -> PluginResult {
        let mut result = PluginResult::default();

        // angular.json: projects.*.architect.build.options.styles → entry patterns
        // These are CSS/SCSS files loaded by the Angular CLI build system.
        let styles = config_parser::extract_config_object_nested_string_or_array(
            source,
            config_path,
            &["projects"],
            &["architect", "build", "options", "styles"],
        );
        for style in &styles {
            let path = style.trim_start_matches("./");
            result.entry_patterns.push(path.to_string());
        }

        // angular.json: projects.*.architect.build.options.scripts → entry patterns
        let scripts = config_parser::extract_config_object_nested_string_or_array(
            source,
            config_path,
            &["projects"],
            &["architect", "build", "options", "scripts"],
        );
        for script in &scripts {
            let path = script.trim_start_matches("./");
            result.entry_patterns.push(path.to_string());
        }

        // angular.json: projects.*.architect.build.options.main → entry patterns
        // Also check "browser" — newer Angular CLI uses "browser" instead of "main"
        for field in &["main", "browser"] {
            let mains = config_parser::extract_config_object_nested_strings(
                source,
                config_path,
                &["projects"],
                &["architect", "build", "options", field],
            );
            for main in &mains {
                let path = main.trim_start_matches("./");
                result.entry_patterns.push(path.to_string());
            }
        }

        // angular.json: projects.*.architect.build.options.polyfills → entry patterns
        // Can be a string or array
        let polyfills = config_parser::extract_config_object_nested_string_or_array(
            source,
            config_path,
            &["projects"],
            &["architect", "build", "options", "polyfills"],
        );
        for polyfill in &polyfills {
            let trimmed = polyfill.trim_start_matches("./");
            // Skip npm package references like "zone.js" — only add file paths.
            // File paths contain "/" (directory separators) or start with "src/", etc.
            // Bare package names like "zone.js" have no "/" and shouldn't be entry points.
            if trimmed.contains('/') {
                result.entry_patterns.push(trimmed.to_string());
            }
        }

        // angular.json: projects.*.architect.test.options.main → entry patterns
        let test_mains = config_parser::extract_config_object_nested_strings(
            source,
            config_path,
            &["projects"],
            &["architect", "test", "options", "main"],
        );
        for main in &test_mains {
            let path = main.trim_start_matches("./");
            result.entry_patterns.push(path.to_string());
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_config_extracts_styles() {
        let source = r#"{
            "projects": {
                "my-app": {
                    "architect": {
                        "build": {
                            "options": {
                                "styles": ["src/styles.css", "src/theme.scss"]
                            }
                        }
                    }
                }
            }
        }"#;
        let plugin = AngularPlugin;
        let result =
            plugin.resolve_config(Path::new("angular.json"), source, Path::new("/project"));
        assert!(
            result
                .entry_patterns
                .contains(&"src/styles.css".to_string())
        );
        assert!(
            result
                .entry_patterns
                .contains(&"src/theme.scss".to_string())
        );
    }

    #[test]
    fn resolve_config_extracts_main() {
        let source = r#"{
            "projects": {
                "my-app": {
                    "architect": {
                        "build": {
                            "options": {
                                "main": "src/main.ts"
                            }
                        }
                    }
                }
            }
        }"#;
        let plugin = AngularPlugin;
        let result =
            plugin.resolve_config(Path::new("angular.json"), source, Path::new("/project"));
        assert!(result.entry_patterns.contains(&"src/main.ts".to_string()));
    }

    #[test]
    fn resolve_config_extracts_scripts() {
        let source = r#"{
            "projects": {
                "my-app": {
                    "architect": {
                        "build": {
                            "options": {
                                "scripts": ["node_modules/some-lib/dist/script.js"]
                            }
                        }
                    }
                }
            }
        }"#;
        let plugin = AngularPlugin;
        let result =
            plugin.resolve_config(Path::new("angular.json"), source, Path::new("/project"));
        assert!(
            result
                .entry_patterns
                .contains(&"node_modules/some-lib/dist/script.js".to_string())
        );
    }

    #[test]
    fn resolve_config_multiple_projects() {
        let source = r#"{
            "projects": {
                "app-one": {
                    "architect": {
                        "build": {
                            "options": {
                                "styles": ["apps/one/src/styles.css"],
                                "main": "apps/one/src/main.ts"
                            }
                        }
                    }
                },
                "app-two": {
                    "architect": {
                        "build": {
                            "options": {
                                "styles": ["apps/two/src/styles.css"],
                                "main": "apps/two/src/main.ts"
                            }
                        }
                    }
                }
            }
        }"#;
        let plugin = AngularPlugin;
        let result =
            plugin.resolve_config(Path::new("angular.json"), source, Path::new("/project"));
        assert!(
            result
                .entry_patterns
                .contains(&"apps/one/src/styles.css".to_string())
        );
        assert!(
            result
                .entry_patterns
                .contains(&"apps/two/src/styles.css".to_string())
        );
        assert!(
            result
                .entry_patterns
                .contains(&"apps/one/src/main.ts".to_string())
        );
        assert!(
            result
                .entry_patterns
                .contains(&"apps/two/src/main.ts".to_string())
        );
    }

    #[test]
    fn resolve_config_polyfills_skips_packages() {
        let source = r#"{
            "projects": {
                "my-app": {
                    "architect": {
                        "build": {
                            "options": {
                                "polyfills": ["zone.js", "src/polyfills.ts"]
                            }
                        }
                    }
                }
            }
        }"#;
        let plugin = AngularPlugin;
        let result =
            plugin.resolve_config(Path::new("angular.json"), source, Path::new("/project"));
        // zone.js is a package, not a file — should be skipped
        assert!(!result.entry_patterns.contains(&"zone.js".to_string()));
        // src/polyfills.ts is a file path — should be included
        assert!(
            result
                .entry_patterns
                .contains(&"src/polyfills.ts".to_string())
        );
    }
}
