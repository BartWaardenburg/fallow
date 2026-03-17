use serde::{Deserialize, Serialize};

/// Declarative framework detection and entry point configuration.
/// This replaces knip's JavaScript plugin system with pure TOML definitions.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FrameworkPreset {
    /// Unique name for this framework.
    pub name: String,

    /// How to detect if this framework is in use.
    #[serde(default)]
    pub detection: Option<FrameworkDetection>,

    /// Glob patterns for files that are entry points.
    #[serde(default)]
    pub entry_points: Vec<FrameworkEntryPattern>,

    /// Files that are always considered "used".
    #[serde(default)]
    pub always_used: Vec<String>,

    /// Exports that are always considered used in matching files.
    #[serde(default)]
    pub used_exports: Vec<FrameworkUsedExport>,
}

/// How to detect if a framework is in use.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FrameworkDetection {
    /// Framework detected if this package is in dependencies.
    Dependency { package: String },
    /// Framework detected if this file pattern matches.
    FileExists { pattern: String },
    /// All conditions must be true.
    All { conditions: Vec<FrameworkDetection> },
    /// Any condition must be true.
    Any { conditions: Vec<FrameworkDetection> },
}

/// Entry point pattern from a framework.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FrameworkEntryPattern {
    /// Glob pattern for entry point files.
    pub pattern: String,
    /// Only consider as entry if this export exists.
    #[serde(default)]
    pub requires_export: Option<String>,
}

/// Exports considered used for files matching a pattern.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FrameworkUsedExport {
    /// Files matching this glob pattern.
    pub file_pattern: String,
    /// These exports are always considered used.
    pub exports: Vec<String>,
}

/// Resolved framework rule (after loading built-in + custom presets).
#[derive(Debug, Clone)]
pub struct FrameworkRule {
    pub name: String,
    pub detection: Option<FrameworkDetection>,
    pub entry_points: Vec<FrameworkEntryPattern>,
    pub always_used: Vec<String>,
    pub used_exports: Vec<FrameworkUsedExport>,
}

impl From<FrameworkPreset> for FrameworkRule {
    fn from(preset: FrameworkPreset) -> Self {
        Self {
            name: preset.name,
            detection: preset.detection,
            entry_points: preset.entry_points,
            always_used: preset.always_used,
            used_exports: preset.used_exports,
        }
    }
}

/// Load built-in framework definitions and merge with user-defined ones.
pub fn resolve_framework_rules(
    enabled: &Option<Vec<String>>,
    custom: &[FrameworkPreset],
) -> Vec<FrameworkRule> {
    let mut rules = Vec::new();

    // Load built-in frameworks
    let builtins = builtin_frameworks();

    match enabled {
        // Explicit list: only enable these
        Some(names) => {
            for name in names {
                if let Some(rule) = builtins.iter().find(|r| &r.name == name) {
                    rules.push(rule.clone());
                }
            }
        }
        // Auto-detect: include all built-ins (detection is checked at runtime)
        None => {
            rules.extend(builtins);
        }
    }

    // Add custom framework definitions
    for preset in custom {
        rules.push(FrameworkRule::from(preset.clone()));
    }

    rules
}

/// Built-in framework definitions.
fn builtin_frameworks() -> Vec<FrameworkRule> {
    vec![
        // ── Next.js ──────────────────────────────────────────
        FrameworkRule {
            name: "nextjs".to_string(),
            detection: Some(FrameworkDetection::Dependency {
                package: "next".to_string(),
            }),
            entry_points: vec![
                pat("app/**/page.{ts,tsx,js,jsx}"),
                pat("app/**/layout.{ts,tsx,js,jsx}"),
                pat("app/**/loading.{ts,tsx,js,jsx}"),
                pat("app/**/error.{ts,tsx,js,jsx}"),
                pat("app/**/not-found.{ts,tsx,js,jsx}"),
                pat("app/**/template.{ts,tsx,js,jsx}"),
                pat("app/**/default.{ts,tsx,js,jsx}"),
                pat("app/**/route.{ts,tsx,js,jsx}"),
                pat("app/**/global-error.{ts,tsx,js,jsx}"),
                pat("app/**/opengraph-image.{ts,tsx,js,jsx}"),
                pat("pages/**/*.{ts,tsx,js,jsx}"),
                pat("src/app/**/page.{ts,tsx,js,jsx}"),
                pat("src/app/**/layout.{ts,tsx,js,jsx}"),
                pat("src/pages/**/*.{ts,tsx,js,jsx}"),
                pat("src/middleware.{ts,js}"),
                pat("middleware.{ts,js}"),
                pat("instrumentation.{ts,js}"),
            ],
            always_used: vec![
                "next.config.{ts,js,mjs,cjs}".to_string(),
                "next-env.d.ts".to_string(),
            ],
            used_exports: vec![
                FrameworkUsedExport {
                    file_pattern: "app/**/page.{ts,tsx,js,jsx}".to_string(),
                    exports: strs(&["default"]),
                },
                FrameworkUsedExport {
                    file_pattern: "app/**/layout.{ts,tsx,js,jsx}".to_string(),
                    exports: strs(&[
                        "default",
                        "metadata",
                        "generateMetadata",
                        "generateStaticParams",
                    ]),
                },
                FrameworkUsedExport {
                    file_pattern: "app/**/route.{ts,tsx,js,jsx}".to_string(),
                    exports: strs(&[
                        "GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS",
                    ]),
                },
                FrameworkUsedExport {
                    file_pattern: "pages/**/*.{ts,tsx,js,jsx}".to_string(),
                    exports: strs(&[
                        "default",
                        "getStaticProps",
                        "getStaticPaths",
                        "getServerSideProps",
                    ]),
                },
            ],
        },
        // ── Vite ─────────────────────────────────────────────
        FrameworkRule {
            name: "vite".to_string(),
            detection: Some(FrameworkDetection::Dependency {
                package: "vite".to_string(),
            }),
            entry_points: vec![
                pat("src/main.{ts,tsx,js,jsx}"),
                pat("src/index.{ts,tsx,js,jsx}"),
                pat("index.html"),
            ],
            always_used: vec!["vite.config.{ts,js,mts,mjs}".to_string()],
            used_exports: vec![],
        },
        // ── Vitest ───────────────────────────────────────────
        FrameworkRule {
            name: "vitest".to_string(),
            detection: Some(FrameworkDetection::Dependency {
                package: "vitest".to_string(),
            }),
            entry_points: vec![
                pat("**/*.test.{ts,tsx,js,jsx}"),
                pat("**/*.spec.{ts,tsx,js,jsx}"),
                pat("**/__tests__/**/*.{ts,tsx,js,jsx}"),
            ],
            always_used: vec![
                "vitest.config.{ts,js,mts}".to_string(),
                "vitest.setup.{ts,js}".to_string(),
            ],
            used_exports: vec![],
        },
        // ── Jest ─────────────────────────────────────────────
        FrameworkRule {
            name: "jest".to_string(),
            detection: Some(FrameworkDetection::Dependency {
                package: "jest".to_string(),
            }),
            entry_points: vec![
                pat("**/*.test.{ts,tsx,js,jsx}"),
                pat("**/*.spec.{ts,tsx,js,jsx}"),
                pat("**/__tests__/**/*.{ts,tsx,js,jsx}"),
            ],
            always_used: vec![
                "jest.config.{ts,js,mjs,cjs}".to_string(),
                "jest.setup.{ts,js}".to_string(),
            ],
            used_exports: vec![],
        },
        // ── Storybook ────────────────────────────────────────
        FrameworkRule {
            name: "storybook".to_string(),
            detection: Some(FrameworkDetection::FileExists {
                pattern: ".storybook/main.{ts,js}".to_string(),
            }),
            entry_points: vec![
                pat("**/*.stories.{ts,tsx,js,jsx,mdx}"),
                pat(".storybook/**/*.{ts,tsx,js,jsx}"),
            ],
            always_used: vec![
                ".storybook/main.{ts,js}".to_string(),
                ".storybook/preview.{ts,tsx,js,jsx}".to_string(),
            ],
            used_exports: vec![],
        },
        // ── Remix ────────────────────────────────────────────
        FrameworkRule {
            name: "remix".to_string(),
            detection: Some(FrameworkDetection::Dependency {
                package: "@remix-run/node".to_string(),
            }),
            entry_points: vec![
                pat("app/routes/**/*.{ts,tsx,js,jsx}"),
                pat("app/root.{ts,tsx,js,jsx}"),
                pat("app/entry.client.{ts,tsx,js,jsx}"),
                pat("app/entry.server.{ts,tsx,js,jsx}"),
            ],
            always_used: vec![],
            used_exports: vec![
                FrameworkUsedExport {
                    file_pattern: "app/routes/**/*.{ts,tsx,js,jsx}".to_string(),
                    exports: strs(&[
                        "default", "loader", "action", "meta", "links", "headers",
                        "handle", "ErrorBoundary", "HydrateFallback",
                    ]),
                },
            ],
        },
        // ── Astro ────────────────────────────────────────────
        FrameworkRule {
            name: "astro".to_string(),
            detection: Some(FrameworkDetection::Dependency {
                package: "astro".to_string(),
            }),
            entry_points: vec![
                pat("src/pages/**/*.{astro,ts,tsx,js,jsx,md,mdx}"),
                pat("src/layouts/**/*.astro"),
                pat("src/content/**/*.{ts,js,md,mdx}"),
            ],
            always_used: vec!["astro.config.{ts,js,mjs}".to_string()],
            used_exports: vec![],
        },
    ]
}

fn pat(pattern: &str) -> FrameworkEntryPattern {
    FrameworkEntryPattern {
        pattern: pattern.to_string(),
        requires_export: None,
    }
}

fn strs(values: &[&str]) -> Vec<String> {
    values.iter().map(|s| s.to_string()).collect()
}
