use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};

use crate::framework::FrameworkPreset;
use crate::workspace::WorkspaceConfig;

/// User-facing configuration loaded from `fallow.toml`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FallowConfig {
    /// Project root (defaults to config file location).
    #[serde(default)]
    pub root: Option<PathBuf>,

    /// Additional entry point glob patterns.
    #[serde(default)]
    pub entry: Vec<String>,

    /// Glob patterns to ignore from analysis.
    #[serde(default)]
    pub ignore: Vec<String>,

    /// What to detect.
    #[serde(default)]
    pub detect: DetectConfig,

    /// Framework presets to activate (auto-detected by default).
    #[serde(default)]
    pub frameworks: Option<Vec<String>>,

    /// Custom framework definitions.
    #[serde(default)]
    pub framework: Vec<FrameworkPreset>,

    /// Workspace overrides.
    #[serde(default)]
    pub workspaces: Option<WorkspaceConfig>,

    /// Dependencies to ignore (always considered used).
    #[serde(default)]
    pub ignore_dependencies: Vec<String>,

    /// Export ignore rules.
    #[serde(default)]
    pub ignore_exports: Vec<IgnoreExportRule>,

    /// Output format.
    #[serde(default)]
    pub output: OutputFormat,
}

/// Controls which analyses to run.
#[derive(Debug, Deserialize, Serialize)]
pub struct DetectConfig {
    /// Detect unused files (not reachable from entry points).
    #[serde(default = "default_true")]
    pub unused_files: bool,

    /// Detect unused exports (exported but never imported).
    #[serde(default = "default_true")]
    pub unused_exports: bool,

    /// Detect unused production dependencies.
    #[serde(default = "default_true")]
    pub unused_dependencies: bool,

    /// Detect unused dev dependencies.
    #[serde(default = "default_true")]
    pub unused_dev_dependencies: bool,

    /// Detect unused type exports.
    #[serde(default = "default_true")]
    pub unused_types: bool,

    /// Detect unused enum members.
    #[serde(default = "default_true")]
    pub unused_enum_members: bool,

    /// Detect unused class members.
    #[serde(default = "default_true")]
    pub unused_class_members: bool,

    /// Detect unresolved imports.
    #[serde(default = "default_true")]
    pub unresolved_imports: bool,

    /// Detect unlisted dependencies (used but not in package.json).
    #[serde(default = "default_true")]
    pub unlisted_dependencies: bool,

    /// Detect duplicate exports.
    #[serde(default = "default_true")]
    pub duplicate_exports: bool,
}

impl Default for DetectConfig {
    fn default() -> Self {
        Self {
            unused_files: true,
            unused_exports: true,
            unused_dependencies: true,
            unused_dev_dependencies: true,
            unused_types: true,
            unused_enum_members: true,
            unused_class_members: true,
            unresolved_imports: true,
            unlisted_dependencies: true,
            duplicate_exports: true,
        }
    }
}

/// Output format for results.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Human-readable terminal output with source context.
    #[default]
    Human,
    /// Machine-readable JSON.
    Json,
    /// SARIF format for GitHub Code Scanning.
    Sarif,
    /// One issue per line (grep-friendly).
    Compact,
}

/// Rule for ignoring specific exports.
#[derive(Debug, Deserialize, Serialize)]
pub struct IgnoreExportRule {
    /// Glob pattern for files.
    pub file: String,
    /// Export names to ignore (`*` for all).
    pub exports: Vec<String>,
}

/// Fully resolved configuration with all globs pre-compiled.
#[derive(Debug)]
pub struct ResolvedConfig {
    pub root: PathBuf,
    pub entry_patterns: Vec<String>,
    pub ignore_patterns: GlobSet,
    pub detect: DetectConfig,
    pub framework_rules: Vec<crate::framework::FrameworkRule>,
    pub tsconfig_path: Option<PathBuf>,
    pub output: OutputFormat,
    pub cache_dir: PathBuf,
    pub threads: usize,
    pub no_cache: bool,
    pub ignore_dependencies: Vec<String>,
    pub ignore_export_rules: Vec<IgnoreExportRule>,
}

impl FallowConfig {
    /// Load config from a `fallow.toml` file.
    pub fn load(path: &Path) -> Result<Self, miette::Report> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            miette::miette!("Failed to read config file {}: {}", path.display(), e)
        })?;
        toml::from_str(&content).map_err(|e| {
            miette::miette!("Failed to parse config file {}: {}", path.display(), e)
        })
    }

    /// Find and load config from the current directory or ancestors.
    pub fn find_and_load(start: &Path) -> Option<(Self, PathBuf)> {
        let config_names = ["fallow.toml", ".fallow.toml"];

        let mut dir = start;
        loop {
            for name in &config_names {
                let candidate = dir.join(name);
                if candidate.exists() {
                    match Self::load(&candidate) {
                        Ok(config) => return Some((config, candidate)),
                        Err(_) => continue,
                    }
                }
            }
            match dir.parent() {
                Some(parent) => dir = parent,
                None => return None,
            }
        }
    }

    /// Resolve into a fully resolved config with compiled globs.
    pub fn resolve(self, root: PathBuf, threads: usize, no_cache: bool) -> ResolvedConfig {
        let mut ignore_builder = GlobSetBuilder::new();
        for pattern in &self.ignore {
            if let Ok(glob) = Glob::new(pattern) {
                ignore_builder.add(glob);
            }
        }

        // Default ignores
        let default_ignores = [
            "**/node_modules/**",
            "**/dist/**",
            "**/build/**",
            "**/.git/**",
            "**/coverage/**",
            "**/*.min.js",
            "**/*.min.mjs",
        ];
        for pattern in &default_ignores {
            if let Ok(glob) = Glob::new(pattern) {
                ignore_builder.add(glob);
            }
        }

        let ignore_patterns = ignore_builder.build().unwrap_or_default();
        let cache_dir = root.join(".fallow");

        let framework_rules = crate::framework::resolve_framework_rules(
            &self.frameworks,
            &self.framework,
        );

        ResolvedConfig {
            root,
            entry_patterns: self.entry,
            ignore_patterns,
            detect: self.detect,
            framework_rules,
            tsconfig_path: None,
            output: self.output,
            cache_dir,
            threads,
            no_cache,
            ignore_dependencies: self.ignore_dependencies,
            ignore_export_rules: self.ignore_exports,
        }
    }
}

const fn default_true() -> bool {
    true
}
