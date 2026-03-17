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
                pat("src/app/**/loading.{ts,tsx,js,jsx}"),
                pat("src/app/**/error.{ts,tsx,js,jsx}"),
                pat("src/app/**/not-found.{ts,tsx,js,jsx}"),
                pat("src/app/**/template.{ts,tsx,js,jsx}"),
                pat("src/app/**/default.{ts,tsx,js,jsx}"),
                pat("src/app/**/route.{ts,tsx,js,jsx}"),
                pat("src/app/**/global-error.{ts,tsx,js,jsx}"),
                pat("src/app/**/opengraph-image.{ts,tsx,js,jsx}"),
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
                    exports: strs(&["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]),
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
            detection: Some(FrameworkDetection::Any {
                conditions: vec![
                    FrameworkDetection::Dependency {
                        package: "@remix-run/node".to_string(),
                    },
                    FrameworkDetection::Dependency {
                        package: "@remix-run/react".to_string(),
                    },
                    FrameworkDetection::Dependency {
                        package: "@remix-run/cloudflare".to_string(),
                    },
                ],
            }),
            entry_points: vec![
                pat("app/routes/**/*.{ts,tsx,js,jsx}"),
                pat("app/root.{ts,tsx,js,jsx}"),
                pat("app/entry.client.{ts,tsx,js,jsx}"),
                pat("app/entry.server.{ts,tsx,js,jsx}"),
            ],
            always_used: vec![],
            used_exports: vec![FrameworkUsedExport {
                file_pattern: "app/routes/**/*.{ts,tsx,js,jsx}".to_string(),
                exports: strs(&[
                    "default",
                    "loader",
                    "action",
                    "meta",
                    "links",
                    "headers",
                    "handle",
                    "ErrorBoundary",
                    "HydrateFallback",
                ]),
            }],
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
        // ── Nuxt ────────────────────────────────────────────
        FrameworkRule {
            name: "nuxt".to_string(),
            detection: Some(FrameworkDetection::Dependency {
                package: "nuxt".to_string(),
            }),
            entry_points: vec![
                pat("pages/**/*.{vue,ts,tsx,js,jsx}"),
                pat("layouts/**/*.{vue,ts,tsx,js,jsx}"),
                pat("middleware/**/*.{ts,js}"),
                pat("server/api/**/*.{ts,js}"),
                pat("server/routes/**/*.{ts,js}"),
                pat("server/middleware/**/*.{ts,js}"),
                pat("plugins/**/*.{ts,js}"),
                pat("composables/**/*.{ts,js}"),
                pat("utils/**/*.{ts,js}"),
            ],
            always_used: vec![
                "nuxt.config.{ts,js}".to_string(),
                "app.vue".to_string(),
                "app.config.{ts,js}".to_string(),
                "error.vue".to_string(),
            ],
            used_exports: vec![
                FrameworkUsedExport {
                    file_pattern: "server/api/**/*.{ts,js}".to_string(),
                    exports: strs(&["default", "defineEventHandler"]),
                },
                FrameworkUsedExport {
                    file_pattern: "middleware/**/*.{ts,js}".to_string(),
                    exports: strs(&["default"]),
                },
            ],
        },
        // ── Angular ─────────────────────────────────────────
        FrameworkRule {
            name: "angular".to_string(),
            detection: Some(FrameworkDetection::Dependency {
                package: "@angular/core".to_string(),
            }),
            entry_points: vec![
                pat("src/main.ts"),
                pat("src/app/**/*.component.ts"),
                pat("src/app/**/*.module.ts"),
                pat("src/app/**/*.service.ts"),
                pat("src/app/**/*.guard.ts"),
                pat("src/app/**/*.pipe.ts"),
                pat("src/app/**/*.directive.ts"),
                pat("src/app/**/*.resolver.ts"),
                pat("src/app/**/*.interceptor.ts"),
            ],
            always_used: vec![
                "angular.json".to_string(),
                "src/polyfills.ts".to_string(),
                "src/environments/**/*.ts".to_string(),
            ],
            used_exports: vec![],
        },
        // ── Playwright ──────────────────────────────────────
        FrameworkRule {
            name: "playwright".to_string(),
            detection: Some(FrameworkDetection::Dependency {
                package: "@playwright/test".to_string(),
            }),
            entry_points: vec![
                pat("**/*.spec.{ts,tsx,js,jsx}"),
                pat("**/*.test.{ts,tsx,js,jsx}"),
                pat("tests/**/*.{ts,tsx,js,jsx}"),
                pat("e2e/**/*.{ts,tsx,js,jsx}"),
            ],
            always_used: vec!["playwright.config.{ts,js}".to_string()],
            used_exports: vec![],
        },
        // ── Prisma ──────────────────────────────────────────
        FrameworkRule {
            name: "prisma".to_string(),
            detection: Some(FrameworkDetection::Dependency {
                package: "prisma".to_string(),
            }),
            entry_points: vec![pat("prisma/seed.{ts,js}")],
            always_used: vec!["prisma/schema.prisma".to_string()],
            used_exports: vec![],
        },
        // ── ESLint ──────────────────────────────────────────
        FrameworkRule {
            name: "eslint".to_string(),
            detection: Some(FrameworkDetection::Dependency {
                package: "eslint".to_string(),
            }),
            entry_points: vec![],
            always_used: vec![
                ".eslintrc.{js,cjs,mjs,json,yaml,yml}".to_string(),
                "eslint.config.{js,mjs,cjs,ts,mts,cts}".to_string(),
            ],
            used_exports: vec![FrameworkUsedExport {
                file_pattern: "eslint.config.{js,mjs,cjs,ts,mts,cts}".to_string(),
                exports: strs(&["default"]),
            }],
        },
        // ── TypeScript ──────────────────────────────────────
        FrameworkRule {
            name: "typescript".to_string(),
            detection: Some(FrameworkDetection::Dependency {
                package: "typescript".to_string(),
            }),
            entry_points: vec![],
            always_used: vec!["tsconfig.json".to_string(), "tsconfig.*.json".to_string()],
            used_exports: vec![],
        },
        // ── Webpack ─────────────────────────────────────────
        FrameworkRule {
            name: "webpack".to_string(),
            detection: Some(FrameworkDetection::Dependency {
                package: "webpack".to_string(),
            }),
            entry_points: vec![pat("src/index.{ts,tsx,js,jsx}")],
            always_used: vec![
                "webpack.config.{ts,js,mjs,cjs}".to_string(),
                "webpack.*.config.{ts,js,mjs,cjs}".to_string(),
            ],
            used_exports: vec![],
        },
        // ── Tailwind CSS ────────────────────────────────────
        FrameworkRule {
            name: "tailwind".to_string(),
            detection: Some(FrameworkDetection::Any {
                conditions: vec![
                    FrameworkDetection::Dependency {
                        package: "tailwindcss".to_string(),
                    },
                    FrameworkDetection::Dependency {
                        package: "@tailwindcss/postcss".to_string(),
                    },
                ],
            }),
            entry_points: vec![],
            always_used: vec![
                "tailwind.config.{ts,js,cjs,mjs}".to_string(),
                "postcss.config.{ts,js,cjs,mjs}".to_string(),
            ],
            used_exports: vec![],
        },
        // ── GraphQL Codegen ─────────────────────────────────
        FrameworkRule {
            name: "graphql-codegen".to_string(),
            detection: Some(FrameworkDetection::Dependency {
                package: "@graphql-codegen/cli".to_string(),
            }),
            entry_points: vec![],
            always_used: vec![
                "codegen.{ts,js,yml,yaml}".to_string(),
                "graphql.config.{ts,js,yml,yaml}".to_string(),
            ],
            used_exports: vec![],
        },
        // ── React Router ────────────────────────────────────
        FrameworkRule {
            name: "react-router".to_string(),
            detection: Some(FrameworkDetection::Any {
                conditions: vec![
                    FrameworkDetection::Dependency {
                        package: "react-router".to_string(),
                    },
                    FrameworkDetection::Dependency {
                        package: "react-router-dom".to_string(),
                    },
                    FrameworkDetection::Dependency {
                        package: "@react-router/dev".to_string(),
                    },
                ],
            }),
            entry_points: vec![
                pat("app/routes/**/*.{ts,tsx,js,jsx}"),
                pat("app/root.{ts,tsx,js,jsx}"),
                pat("app/entry.client.{ts,tsx,js,jsx}"),
                pat("app/entry.server.{ts,tsx,js,jsx}"),
            ],
            always_used: vec!["react-router.config.{ts,js}".to_string()],
            used_exports: vec![FrameworkUsedExport {
                file_pattern: "app/routes/**/*.{ts,tsx,js,jsx}".to_string(),
                exports: strs(&[
                    "default",
                    "loader",
                    "clientLoader",
                    "action",
                    "clientAction",
                    "meta",
                    "links",
                    "headers",
                    "handle",
                    "ErrorBoundary",
                    "HydrateFallback",
                    "shouldRevalidate",
                ]),
            }],
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_frameworks_not_empty() {
        let builtins = builtin_frameworks();
        assert!(!builtins.is_empty());
    }

    #[test]
    fn builtin_frameworks_have_names() {
        let builtins = builtin_frameworks();
        for rule in &builtins {
            assert!(!rule.name.is_empty(), "Framework rule should have a name");
        }
    }

    #[test]
    fn builtin_frameworks_known_names() {
        let builtins = builtin_frameworks();
        let names: Vec<&str> = builtins.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"nextjs"));
        assert!(names.contains(&"vite"));
        assert!(names.contains(&"vitest"));
        assert!(names.contains(&"jest"));
        assert!(names.contains(&"storybook"));
        assert!(names.contains(&"remix"));
        assert!(names.contains(&"astro"));
        assert!(names.contains(&"nuxt"));
        assert!(names.contains(&"angular"));
        assert!(names.contains(&"playwright"));
        assert!(names.contains(&"prisma"));
        assert!(names.contains(&"eslint"));
        assert!(names.contains(&"typescript"));
        assert!(names.contains(&"webpack"));
        assert!(names.contains(&"tailwind"));
        assert!(names.contains(&"graphql-codegen"));
        assert!(names.contains(&"react-router"));
    }

    #[test]
    fn resolve_framework_rules_auto_detect() {
        // When enabled is None, all builtins should be included
        let rules = resolve_framework_rules(&None, &[]);
        assert!(!rules.is_empty());
        assert_eq!(rules.len(), builtin_frameworks().len());
    }

    #[test]
    fn resolve_framework_rules_explicit_list() {
        let enabled = Some(vec!["nextjs".to_string(), "vitest".to_string()]);
        let rules = resolve_framework_rules(&enabled, &[]);
        assert_eq!(rules.len(), 2);
        let names: Vec<&str> = rules.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"nextjs"));
        assert!(names.contains(&"vitest"));
    }

    #[test]
    fn resolve_framework_rules_empty_explicit_list() {
        let enabled = Some(vec![]);
        let rules = resolve_framework_rules(&enabled, &[]);
        assert!(rules.is_empty());
    }

    #[test]
    fn resolve_framework_rules_unknown_name_ignored() {
        let enabled = Some(vec!["nonexistent-framework".to_string()]);
        let rules = resolve_framework_rules(&enabled, &[]);
        assert!(rules.is_empty());
    }

    #[test]
    fn resolve_framework_rules_with_custom() {
        let custom = vec![FrameworkPreset {
            name: "custom".to_string(),
            detection: None,
            entry_points: vec![FrameworkEntryPattern {
                pattern: "src/custom/**/*.ts".to_string(),
                requires_export: None,
            }],
            always_used: vec![],
            used_exports: vec![],
        }];
        let rules = resolve_framework_rules(&Some(vec![]), &custom);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name, "custom");
    }

    #[test]
    fn framework_preset_to_rule() {
        let preset = FrameworkPreset {
            name: "test".to_string(),
            detection: Some(FrameworkDetection::Dependency {
                package: "test-pkg".to_string(),
            }),
            entry_points: vec![FrameworkEntryPattern {
                pattern: "src/**/*.test.ts".to_string(),
                requires_export: None,
            }],
            always_used: vec!["setup.ts".to_string()],
            used_exports: vec![FrameworkUsedExport {
                file_pattern: "src/**/*.test.ts".to_string(),
                exports: vec!["default".to_string()],
            }],
        };
        let rule: FrameworkRule = preset.into();
        assert_eq!(rule.name, "test");
        assert!(rule.detection.is_some());
        assert_eq!(rule.entry_points.len(), 1);
        assert_eq!(rule.always_used, vec!["setup.ts"]);
        assert_eq!(rule.used_exports.len(), 1);
    }

    #[test]
    fn framework_detection_deserialize_dependency() {
        let json = r#"{"type": "dependency", "package": "next"}"#;
        let detection: FrameworkDetection = serde_json::from_str(json).unwrap();
        assert!(
            matches!(detection, FrameworkDetection::Dependency { package } if package == "next")
        );
    }

    #[test]
    fn framework_detection_deserialize_file_exists() {
        let json = r#"{"type": "file_exists", "pattern": "tsconfig.json"}"#;
        let detection: FrameworkDetection = serde_json::from_str(json).unwrap();
        assert!(
            matches!(detection, FrameworkDetection::FileExists { pattern } if pattern == "tsconfig.json")
        );
    }

    #[test]
    fn framework_detection_deserialize_all() {
        let json = r#"{"type": "all", "conditions": [{"type": "dependency", "package": "a"}, {"type": "dependency", "package": "b"}]}"#;
        let detection: FrameworkDetection = serde_json::from_str(json).unwrap();
        assert!(
            matches!(detection, FrameworkDetection::All { conditions } if conditions.len() == 2)
        );
    }

    #[test]
    fn framework_detection_deserialize_any() {
        let json = r#"{"type": "any", "conditions": [{"type": "dependency", "package": "a"}]}"#;
        let detection: FrameworkDetection = serde_json::from_str(json).unwrap();
        assert!(
            matches!(detection, FrameworkDetection::Any { conditions } if conditions.len() == 1)
        );
    }

    #[test]
    fn nextjs_has_entry_points() {
        let builtins = builtin_frameworks();
        let nextjs = builtins.iter().find(|r| r.name == "nextjs").unwrap();
        assert!(!nextjs.entry_points.is_empty());
        let patterns: Vec<&str> = nextjs
            .entry_points
            .iter()
            .map(|e| e.pattern.as_str())
            .collect();
        assert!(patterns.iter().any(|p| p.contains("app/**/page")));
        assert!(patterns.iter().any(|p| p.contains("pages/")));
    }

    #[test]
    fn nextjs_has_used_exports() {
        let builtins = builtin_frameworks();
        let nextjs = builtins.iter().find(|r| r.name == "nextjs").unwrap();
        assert!(!nextjs.used_exports.is_empty());
    }

    #[test]
    fn vitest_has_test_entry_points() {
        let builtins = builtin_frameworks();
        let vitest = builtins.iter().find(|r| r.name == "vitest").unwrap();
        let patterns: Vec<&str> = vitest
            .entry_points
            .iter()
            .map(|e| e.pattern.as_str())
            .collect();
        assert!(patterns.iter().any(|p| p.contains("*.test.")));
        assert!(patterns.iter().any(|p| p.contains("*.spec.")));
    }
}
