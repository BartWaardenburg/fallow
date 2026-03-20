use std::io::Read as _;
use std::path::{Path, PathBuf};

use rustc_hash::FxHashSet;

use globset::{Glob, GlobSet, GlobSetBuilder};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::external_plugin::{ExternalPluginDef, discover_external_plugins};
use crate::workspace::WorkspaceConfig;

/// Supported config file names in priority order.
///
/// `find_and_load` checks these names in order within each directory,
/// returning the first match found.
const CONFIG_NAMES: &[&str] = &[".fallowrc.json", "fallow.toml", ".fallow.toml"];

/// User-facing configuration loaded from `.fallowrc.json` or `fallow.toml`.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FallowConfig {
    /// JSON Schema reference (ignored during deserialization).
    #[serde(rename = "$schema", default, skip_serializing)]
    #[schemars(skip)]
    pub schema: Option<String>,

    /// Paths to base config files to extend from.
    /// Paths are resolved relative to the config file containing the `extends`.
    /// Base configs are loaded first, then this config's values override them.
    /// Later entries in the array override earlier ones.
    #[serde(default, skip_serializing)]
    pub extends: Vec<String>,

    /// Additional entry point glob patterns.
    #[serde(default)]
    pub entry: Vec<String>,

    /// Glob patterns to ignore from analysis.
    #[serde(default)]
    pub ignore_patterns: Vec<String>,

    /// Custom framework definitions (inline plugin definitions).
    #[serde(default)]
    pub framework: Vec<ExternalPluginDef>,

    /// Workspace overrides.
    #[serde(default)]
    pub workspaces: Option<WorkspaceConfig>,

    /// Dependencies to ignore (always considered used).
    #[serde(default)]
    pub ignore_dependencies: Vec<String>,

    /// Export ignore rules.
    #[serde(default)]
    pub ignore_exports: Vec<IgnoreExportRule>,

    /// Duplication detection settings.
    #[serde(default)]
    pub duplicates: DuplicatesConfig,

    /// Per-issue-type severity rules.
    #[serde(default)]
    pub rules: RulesConfig,

    /// Production mode: exclude test/dev files, only start/build scripts.
    #[serde(default)]
    pub production: bool,

    /// Paths to external plugin files or directories containing plugin files.
    ///
    /// Supports TOML, JSON, and JSONC formats.
    ///
    /// In addition to these explicit paths, fallow automatically discovers:
    /// - `*.toml`, `*.json`, `*.jsonc` files in `.fallow/plugins/`
    /// - `fallow-plugin-*.{toml,json,jsonc}` files in the project root
    #[serde(default)]
    pub plugins: Vec<String>,

    /// Per-file rule overrides matching oxlint's overrides pattern.
    #[serde(default)]
    pub overrides: Vec<ConfigOverride>,
}

/// Configuration for code duplication detection.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DuplicatesConfig {
    /// Whether duplication detection is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Detection mode: strict, mild, weak, or semantic.
    #[serde(default)]
    pub mode: DetectionMode,

    /// Minimum number of tokens for a clone.
    #[serde(default = "default_min_tokens")]
    pub min_tokens: usize,

    /// Minimum number of lines for a clone.
    #[serde(default = "default_min_lines")]
    pub min_lines: usize,

    /// Maximum allowed duplication percentage (0 = no limit).
    #[serde(default)]
    pub threshold: f64,

    /// Additional ignore patterns for duplication analysis.
    #[serde(default)]
    pub ignore: Vec<String>,

    /// Only report cross-directory duplicates.
    #[serde(default)]
    pub skip_local: bool,

    /// Enable cross-language clone detection by stripping type annotations.
    ///
    /// When enabled, TypeScript type annotations (parameter types, return types,
    /// generics, interfaces, type aliases) are stripped from the token stream,
    /// allowing detection of clones between `.ts` and `.js` files.
    #[serde(default)]
    pub cross_language: bool,

    /// Fine-grained normalization overrides on top of the detection mode.
    #[serde(default)]
    pub normalization: NormalizationConfig,
}

impl Default for DuplicatesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: DetectionMode::default(),
            min_tokens: default_min_tokens(),
            min_lines: default_min_lines(),
            threshold: 0.0,
            ignore: vec![],
            skip_local: false,
            cross_language: false,
            normalization: NormalizationConfig::default(),
        }
    }
}

/// Fine-grained normalization overrides.
///
/// Each option, when set to `Some(true)`, forces that normalization regardless of
/// the detection mode. When set to `Some(false)`, it forces preservation. When
/// `None`, the detection mode's default behavior applies.
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NormalizationConfig {
    /// Blind all identifiers (variable names, function names, etc.) to the same hash.
    /// Default in `semantic` mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_identifiers: Option<bool>,

    /// Blind string literal values to the same hash.
    /// Default in `weak` and `semantic` modes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_string_values: Option<bool>,

    /// Blind numeric literal values to the same hash.
    /// Default in `semantic` mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_numeric_values: Option<bool>,
}

/// Resolved normalization flags: mode defaults merged with user overrides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedNormalization {
    pub ignore_identifiers: bool,
    pub ignore_string_values: bool,
    pub ignore_numeric_values: bool,
}

impl ResolvedNormalization {
    /// Resolve normalization from a detection mode and optional overrides.
    pub fn resolve(mode: DetectionMode, overrides: &NormalizationConfig) -> Self {
        let (default_ids, default_strings, default_numbers) = match mode {
            DetectionMode::Strict | DetectionMode::Mild => (false, false, false),
            DetectionMode::Weak => (false, true, false),
            DetectionMode::Semantic => (true, true, true),
        };

        Self {
            ignore_identifiers: overrides.ignore_identifiers.unwrap_or(default_ids),
            ignore_string_values: overrides.ignore_string_values.unwrap_or(default_strings),
            ignore_numeric_values: overrides.ignore_numeric_values.unwrap_or(default_numbers),
        }
    }
}

/// Detection mode controlling how aggressively tokens are normalized.
///
/// Since fallow uses AST-based tokenization (not lexer-based), whitespace and
/// comments are inherently absent from the token stream. The `Strict` and `Mild`
/// modes are currently equivalent. `Weak` mode additionally blinds string
/// literals. `Semantic` mode blinds all identifiers and literal values for
/// Type-2 (renamed variable) clone detection.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DetectionMode {
    /// All tokens preserved including identifier names and literal values (Type-1 only).
    Strict,
    /// Default mode -- equivalent to strict for AST-based tokenization.
    #[default]
    Mild,
    /// Blind string literal values (structure-preserving).
    Weak,
    /// Blind all identifiers and literal values for structural (Type-2) detection.
    Semantic,
}

impl std::fmt::Display for DetectionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Strict => write!(f, "strict"),
            Self::Mild => write!(f, "mild"),
            Self::Weak => write!(f, "weak"),
            Self::Semantic => write!(f, "semantic"),
        }
    }
}

impl std::str::FromStr for DetectionMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "strict" => Ok(Self::Strict),
            "mild" => Ok(Self::Mild),
            "weak" => Ok(Self::Weak),
            "semantic" => Ok(Self::Semantic),
            other => Err(format!("unknown detection mode: '{other}'")),
        }
    }
}

const fn default_min_tokens() -> usize {
    50
}

const fn default_min_lines() -> usize {
    5
}

/// Output format for results.
///
/// This is CLI-only (via `--format` flag), not stored in config files.
#[derive(Debug, Default, Clone)]
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
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct IgnoreExportRule {
    /// Glob pattern for files.
    pub file: String,
    /// Export names to ignore (`*` for all).
    pub exports: Vec<String>,
}

/// Per-file override entry.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfigOverride {
    /// Glob patterns to match files against (relative to config file location).
    pub files: Vec<String>,
    /// Partial rules — only specified fields override the base rules.
    #[serde(default)]
    pub rules: PartialRulesConfig,
}

/// Partial per-issue-type severity for overrides. All fields optional.
#[derive(Debug, Default, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct PartialRulesConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unused_files: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unused_exports: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unused_types: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unused_dependencies: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unused_dev_dependencies: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unused_enum_members: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unused_class_members: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unresolved_imports: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unlisted_dependencies: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duplicate_exports: Option<Severity>,
}

/// Resolved override with pre-compiled glob matchers.
#[derive(Debug)]
pub struct ResolvedOverride {
    pub matchers: Vec<globset::GlobMatcher>,
    pub rules: PartialRulesConfig,
}

/// Fully resolved configuration with all globs pre-compiled.
#[derive(Debug)]
pub struct ResolvedConfig {
    pub root: PathBuf,
    pub entry_patterns: Vec<String>,
    pub ignore_patterns: GlobSet,
    pub output: OutputFormat,
    pub cache_dir: PathBuf,
    pub threads: usize,
    pub no_cache: bool,
    pub ignore_dependencies: Vec<String>,
    pub ignore_export_rules: Vec<IgnoreExportRule>,
    pub duplicates: DuplicatesConfig,
    pub rules: RulesConfig,
    /// Whether production mode is active.
    pub production: bool,
    /// External plugin definitions (from plugin files + inline framework definitions).
    pub external_plugins: Vec<ExternalPluginDef>,
    /// Per-file rule overrides with pre-compiled glob matchers.
    pub overrides: Vec<ResolvedOverride>,
}

/// Detect config format from file extension.
enum ConfigFormat {
    Toml,
    Json,
}

impl ConfigFormat {
    fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|e| e.to_str()) {
            Some("json") => Self::Json,
            _ => Self::Toml,
        }
    }
}

const MAX_EXTENDS_DEPTH: usize = 10;

/// Deep-merge two JSON values. `base` is lower-priority, `overlay` is higher.
/// Objects: merge field by field. Arrays/scalars: overlay replaces base.
fn deep_merge_json(base: &mut serde_json::Value, overlay: serde_json::Value) {
    match (base, overlay) {
        (serde_json::Value::Object(base_map), serde_json::Value::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                if let Some(base_value) = base_map.get_mut(&key) {
                    deep_merge_json(base_value, value);
                } else {
                    base_map.insert(key, value);
                }
            }
        }
        (base, overlay) => {
            *base = overlay;
        }
    }
}

fn parse_config_to_value(path: &Path) -> Result<serde_json::Value, miette::Report> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| miette::miette!("Failed to read config file {}: {}", path.display(), e))?;

    match ConfigFormat::from_path(path) {
        ConfigFormat::Toml => {
            let toml_value: toml::Value = toml::from_str(&content).map_err(|e| {
                miette::miette!("Failed to parse config file {}: {}", path.display(), e)
            })?;
            serde_json::to_value(toml_value).map_err(|e| {
                miette::miette!(
                    "Failed to convert TOML to JSON for {}: {}",
                    path.display(),
                    e
                )
            })
        }
        ConfigFormat::Json => {
            let mut stripped = String::new();
            json_comments::StripComments::new(content.as_bytes())
                .read_to_string(&mut stripped)
                .map_err(|e| {
                    miette::miette!("Failed to strip comments from {}: {}", path.display(), e)
                })?;
            serde_json::from_str(&stripped).map_err(|e| {
                miette::miette!("Failed to parse config file {}: {}", path.display(), e)
            })
        }
    }
}

fn resolve_extends(
    path: &Path,
    visited: &mut FxHashSet<PathBuf>,
    depth: usize,
) -> Result<serde_json::Value, miette::Report> {
    if depth >= MAX_EXTENDS_DEPTH {
        return Err(miette::miette!(
            "Config extends chain too deep (>={MAX_EXTENDS_DEPTH} levels) at {}",
            path.display()
        ));
    }

    let canonical = path.canonicalize().map_err(|e| {
        miette::miette!(
            "Config file not found or unresolvable: {}: {}",
            path.display(),
            e
        )
    })?;

    if !visited.insert(canonical.clone()) {
        return Err(miette::miette!(
            "Circular extends detected: {} was already visited in the extends chain",
            path.display()
        ));
    }

    let mut value = parse_config_to_value(path)?;

    let extends = value
        .as_object_mut()
        .and_then(|obj| obj.remove("extends"))
        .and_then(|v| match v {
            serde_json::Value::Array(arr) => Some(
                arr.into_iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>(),
            ),
            serde_json::Value::String(s) => Some(vec![s]),
            _ => None,
        })
        .unwrap_or_default();

    if extends.is_empty() {
        return Ok(value);
    }

    let config_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut merged = serde_json::Value::Object(serde_json::Map::new());

    for extend_path_str in &extends {
        if Path::new(extend_path_str).is_absolute() {
            return Err(miette::miette!(
                "extends paths must be relative, got absolute path: {} (in {})",
                extend_path_str,
                path.display()
            ));
        }
        let extend_path = config_dir.join(extend_path_str);
        if !extend_path.exists() {
            return Err(miette::miette!(
                "Extended config file not found: {} (referenced from {})",
                extend_path.display(),
                path.display()
            ));
        }
        let base = resolve_extends(&extend_path, visited, depth + 1)?;
        deep_merge_json(&mut merged, base);
    }

    deep_merge_json(&mut merged, value);
    Ok(merged)
}

impl FallowConfig {
    /// Load config from a fallow config file (TOML or JSON/JSONC).
    ///
    /// The format is detected from the file extension:
    /// - `.toml` → TOML
    /// - `.json` → JSON (with JSONC comment stripping)
    ///
    /// Supports `extends` for config inheritance. Extended configs are loaded
    /// and deep-merged before this config's values are applied.
    pub fn load(path: &Path) -> Result<Self, miette::Report> {
        let mut visited = FxHashSet::default();
        let merged = resolve_extends(path, &mut visited, 0)?;

        serde_json::from_value(merged).map_err(|e| {
            miette::miette!(
                "Failed to deserialize config from {}: {}",
                path.display(),
                e
            )
        })
    }

    /// Find and load config from the current directory or ancestors.
    ///
    /// Checks for config files in priority order:
    /// `.fallowrc.json` > `fallow.toml` > `.fallow.toml`
    ///
    /// Stops searching at the first directory containing `.git` or `package.json`,
    /// to avoid picking up unrelated config files above the project root.
    ///
    /// Returns `Ok(Some(...))` if a config was found and parsed, `Ok(None)` if
    /// no config file exists, and `Err(...)` if a config file exists but fails to parse.
    pub fn find_and_load(start: &Path) -> Result<Option<(Self, PathBuf)>, String> {
        let mut dir = start;
        loop {
            for name in CONFIG_NAMES {
                let candidate = dir.join(name);
                if candidate.exists() {
                    match Self::load(&candidate) {
                        Ok(config) => return Ok(Some((config, candidate))),
                        Err(e) => {
                            return Err(format!("Failed to parse {}: {e}", candidate.display()));
                        }
                    }
                }
            }
            // Stop at project root indicators
            if dir.join(".git").exists() || dir.join("package.json").exists() {
                break;
            }
            dir = match dir.parent() {
                Some(parent) => parent,
                None => break,
            };
        }
        Ok(None)
    }

    /// Generate JSON Schema for the configuration format.
    pub fn json_schema() -> serde_json::Value {
        #[allow(clippy::use_self)] // schemars macro requires the concrete type name
        serde_json::to_value(schemars::schema_for!(FallowConfig)).unwrap_or_default()
    }

    /// Resolve into a fully resolved config with compiled globs.
    #[allow(clippy::print_stderr)]
    pub fn resolve(
        self,
        root: PathBuf,
        output: OutputFormat,
        threads: usize,
        no_cache: bool,
    ) -> ResolvedConfig {
        let mut ignore_builder = GlobSetBuilder::new();
        for pattern in &self.ignore_patterns {
            match Glob::new(pattern) {
                Ok(glob) => {
                    ignore_builder.add(glob);
                }
                Err(e) => {
                    eprintln!("Warning: Invalid ignore glob pattern '{pattern}': {e}");
                }
            }
        }

        // Default ignores
        // Note: `build/` is only ignored at the project root (not `**/build/**`)
        // because nested `build/` directories like `test/build/` may contain source files.
        let default_ignores = [
            "**/node_modules/**",
            "**/dist/**",
            "build/**",
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

        let compiled_ignore_patterns = ignore_builder.build().unwrap_or_default();
        let cache_dir = root.join(".fallow");

        let mut rules = self.rules;

        // In production mode, force unused_dev_dependencies off
        let production = self.production;
        if production {
            rules.unused_dev_dependencies = Severity::Off;
        }

        let mut external_plugins = discover_external_plugins(&root, &self.plugins);
        // Merge inline framework definitions into external plugins
        external_plugins.extend(self.framework);

        // Pre-compile override glob matchers
        let overrides = self
            .overrides
            .into_iter()
            .filter_map(|o| {
                let matchers: Vec<globset::GlobMatcher> = o
                    .files
                    .iter()
                    .filter_map(|pattern| match Glob::new(pattern) {
                        Ok(glob) => Some(glob.compile_matcher()),
                        Err(e) => {
                            eprintln!("Warning: Invalid override glob pattern '{pattern}': {e}");
                            None
                        }
                    })
                    .collect();
                if matchers.is_empty() {
                    None
                } else {
                    Some(ResolvedOverride {
                        matchers,
                        rules: o.rules,
                    })
                }
            })
            .collect();

        ResolvedConfig {
            root,
            entry_patterns: self.entry,
            ignore_patterns: compiled_ignore_patterns,
            output,
            cache_dir,
            threads,
            no_cache,
            ignore_dependencies: self.ignore_dependencies,
            ignore_export_rules: self.ignore_exports,
            duplicates: self.duplicates,
            rules,
            production,
            external_plugins,
            overrides,
        }
    }
}

impl ResolvedConfig {
    /// Resolve the effective rules for a given file path.
    /// Starts with base rules and applies matching overrides in order.
    pub fn resolve_rules_for_path(&self, path: &Path) -> RulesConfig {
        if self.overrides.is_empty() {
            return self.rules.clone();
        }

        let relative = path.strip_prefix(&self.root).unwrap_or(path);
        let relative_str = relative.to_string_lossy();

        let mut rules = self.rules.clone();
        for override_entry in &self.overrides {
            let matches = override_entry
                .matchers
                .iter()
                .any(|m| m.is_match(relative_str.as_ref()));
            if matches {
                rules.apply_partial(&override_entry.rules);
            }
        }
        rules
    }
}

const fn default_true() -> bool {
    true
}

/// Severity level for rules.
///
/// Controls whether an issue type causes CI failure (`error`), is reported
/// without failing (`warn`), or is suppressed entirely (`off`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Report and fail CI (non-zero exit code).
    #[default]
    Error,
    /// Report but don't fail CI.
    Warn,
    /// Don't detect or report.
    Off,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warn => write!(f, "warn"),
            Self::Off => write!(f, "off"),
        }
    }
}

impl std::str::FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "error" => Ok(Self::Error),
            "warn" | "warning" => Ok(Self::Warn),
            "off" | "none" => Ok(Self::Off),
            other => Err(format!(
                "unknown severity: '{other}' (expected error, warn, or off)"
            )),
        }
    }
}

/// Per-issue-type severity configuration.
///
/// Controls which issue types cause CI failure, are reported as warnings,
/// or are suppressed entirely. All fields default to `Severity::Error`.
///
/// Rule names use kebab-case in config files (e.g., `"unused-files": "error"`).
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct RulesConfig {
    #[serde(default)]
    pub unused_files: Severity,
    #[serde(default)]
    pub unused_exports: Severity,
    #[serde(default)]
    pub unused_types: Severity,
    #[serde(default)]
    pub unused_dependencies: Severity,
    #[serde(default)]
    pub unused_dev_dependencies: Severity,
    #[serde(default)]
    pub unused_enum_members: Severity,
    #[serde(default)]
    pub unused_class_members: Severity,
    #[serde(default)]
    pub unresolved_imports: Severity,
    #[serde(default)]
    pub unlisted_dependencies: Severity,
    #[serde(default)]
    pub duplicate_exports: Severity,
}

impl Default for RulesConfig {
    fn default() -> Self {
        Self {
            unused_files: Severity::Error,
            unused_exports: Severity::Error,
            unused_types: Severity::Error,
            unused_dependencies: Severity::Error,
            unused_dev_dependencies: Severity::Error,
            unused_enum_members: Severity::Error,
            unused_class_members: Severity::Error,
            unresolved_imports: Severity::Error,
            unlisted_dependencies: Severity::Error,
            duplicate_exports: Severity::Error,
        }
    }
}

impl RulesConfig {
    /// Apply a partial rules config on top. Only `Some` fields override.
    pub const fn apply_partial(&mut self, partial: &PartialRulesConfig) {
        if let Some(s) = partial.unused_files {
            self.unused_files = s;
        }
        if let Some(s) = partial.unused_exports {
            self.unused_exports = s;
        }
        if let Some(s) = partial.unused_types {
            self.unused_types = s;
        }
        if let Some(s) = partial.unused_dependencies {
            self.unused_dependencies = s;
        }
        if let Some(s) = partial.unused_dev_dependencies {
            self.unused_dev_dependencies = s;
        }
        if let Some(s) = partial.unused_enum_members {
            self.unused_enum_members = s;
        }
        if let Some(s) = partial.unused_class_members {
            self.unused_class_members = s;
        }
        if let Some(s) = partial.unresolved_imports {
            self.unresolved_imports = s;
        }
        if let Some(s) = partial.unlisted_dependencies {
            self.unlisted_dependencies = s;
        }
        if let Some(s) = partial.duplicate_exports {
            self.duplicate_exports = s;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PackageJson;

    /// Create a unique temp directory for this test to avoid parallel test races.
    fn test_dir(name: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("fallow-{name}-{id}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn output_format_default_is_human() {
        let format = OutputFormat::default();
        assert!(matches!(format, OutputFormat::Human));
    }

    #[test]
    fn fallow_config_deserialize_minimal() {
        let toml_str = r#"
entry = ["src/main.ts"]
"#;
        let config: FallowConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.entry, vec!["src/main.ts"]);
        assert!(config.ignore_patterns.is_empty());
    }

    #[test]
    fn fallow_config_deserialize_ignore_exports() {
        let toml_str = r#"
[[ignoreExports]]
file = "src/types/*.ts"
exports = ["*"]

[[ignoreExports]]
file = "src/constants.ts"
exports = ["FOO", "BAR"]
"#;
        let config: FallowConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.ignore_exports.len(), 2);
        assert_eq!(config.ignore_exports[0].file, "src/types/*.ts");
        assert_eq!(config.ignore_exports[0].exports, vec!["*"]);
        assert_eq!(config.ignore_exports[1].exports, vec!["FOO", "BAR"]);
    }

    #[test]
    fn fallow_config_deserialize_ignore_dependencies() {
        let toml_str = r#"
ignoreDependencies = ["autoprefixer", "postcss"]
"#;
        let config: FallowConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.ignore_dependencies, vec!["autoprefixer", "postcss"]);
    }

    #[test]
    fn fallow_config_resolve_default_ignores() {
        let config = FallowConfig {
            schema: None,
            extends: vec![],
            entry: vec![],
            ignore_patterns: vec![],
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            duplicates: DuplicatesConfig::default(),
            rules: RulesConfig::default(),
            production: false,
            plugins: vec![],
            overrides: vec![],
        };
        let resolved = config.resolve(PathBuf::from("/tmp/test"), OutputFormat::Human, 4, true);

        // Default ignores should be compiled
        assert!(resolved.ignore_patterns.is_match("node_modules/foo/bar.ts"));
        assert!(resolved.ignore_patterns.is_match("dist/bundle.js"));
        assert!(resolved.ignore_patterns.is_match("build/output.js"));
        assert!(resolved.ignore_patterns.is_match(".git/config"));
        assert!(resolved.ignore_patterns.is_match("coverage/report.js"));
        assert!(resolved.ignore_patterns.is_match("foo.min.js"));
        assert!(resolved.ignore_patterns.is_match("bar.min.mjs"));
    }

    #[test]
    fn fallow_config_resolve_custom_ignores() {
        let config = FallowConfig {
            schema: None,
            extends: vec![],
            entry: vec!["src/**/*.ts".to_string()],
            ignore_patterns: vec!["**/*.generated.ts".to_string()],
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            duplicates: DuplicatesConfig::default(),
            rules: RulesConfig::default(),
            production: false,
            plugins: vec![],
            overrides: vec![],
        };
        let resolved = config.resolve(PathBuf::from("/tmp/test"), OutputFormat::Json, 4, false);

        assert!(resolved.ignore_patterns.is_match("src/foo.generated.ts"));
        assert_eq!(resolved.entry_patterns, vec!["src/**/*.ts"]);
        assert!(matches!(resolved.output, OutputFormat::Json));
        assert!(!resolved.no_cache);
    }

    #[test]
    fn fallow_config_resolve_cache_dir() {
        let config = FallowConfig {
            schema: None,
            extends: vec![],
            entry: vec![],
            ignore_patterns: vec![],
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            duplicates: DuplicatesConfig::default(),
            rules: RulesConfig::default(),
            production: false,
            plugins: vec![],
            overrides: vec![],
        };
        let resolved = config.resolve(PathBuf::from("/tmp/project"), OutputFormat::Human, 4, true);
        assert_eq!(resolved.cache_dir, PathBuf::from("/tmp/project/.fallow"));
        assert!(resolved.no_cache);
    }

    #[test]
    fn package_json_entry_points_main() {
        let pkg: PackageJson = serde_json::from_str(r#"{"main": "dist/index.js"}"#).unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"dist/index.js".to_string()));
    }

    #[test]
    fn package_json_entry_points_module() {
        let pkg: PackageJson = serde_json::from_str(r#"{"module": "dist/index.mjs"}"#).unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"dist/index.mjs".to_string()));
    }

    #[test]
    fn package_json_entry_points_types() {
        let pkg: PackageJson = serde_json::from_str(r#"{"types": "dist/index.d.ts"}"#).unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"dist/index.d.ts".to_string()));
    }

    #[test]
    fn package_json_entry_points_bin_string() {
        let pkg: PackageJson = serde_json::from_str(r#"{"bin": "bin/cli.js"}"#).unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"bin/cli.js".to_string()));
    }

    #[test]
    fn package_json_entry_points_bin_object() {
        let pkg: PackageJson =
            serde_json::from_str(r#"{"bin": {"cli": "bin/cli.js", "serve": "bin/serve.js"}}"#)
                .unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"bin/cli.js".to_string()));
        assert!(entries.contains(&"bin/serve.js".to_string()));
    }

    #[test]
    fn package_json_entry_points_exports_string() {
        let pkg: PackageJson = serde_json::from_str(r#"{"exports": "./dist/index.js"}"#).unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"./dist/index.js".to_string()));
    }

    #[test]
    fn package_json_entry_points_exports_object() {
        let pkg: PackageJson = serde_json::from_str(
            r#"{"exports": {".": {"import": "./dist/index.mjs", "require": "./dist/index.cjs"}}}"#,
        )
        .unwrap();
        let entries = pkg.entry_points();
        assert!(entries.contains(&"./dist/index.mjs".to_string()));
        assert!(entries.contains(&"./dist/index.cjs".to_string()));
    }

    #[test]
    fn package_json_dependency_names() {
        let pkg: PackageJson = serde_json::from_str(
            r#"{
            "dependencies": {"react": "^18", "lodash": "^4"},
            "devDependencies": {"typescript": "^5"},
            "peerDependencies": {"react-dom": "^18"}
        }"#,
        )
        .unwrap();

        let all = pkg.all_dependency_names();
        assert!(all.contains(&"react".to_string()));
        assert!(all.contains(&"lodash".to_string()));
        assert!(all.contains(&"typescript".to_string()));
        assert!(all.contains(&"react-dom".to_string()));

        let prod = pkg.production_dependency_names();
        assert!(prod.contains(&"react".to_string()));
        assert!(!prod.contains(&"typescript".to_string()));

        let dev = pkg.dev_dependency_names();
        assert!(dev.contains(&"typescript".to_string()));
        assert!(!dev.contains(&"react".to_string()));
    }

    #[test]
    fn package_json_no_dependencies() {
        let pkg: PackageJson = serde_json::from_str(r#"{"name": "test"}"#).unwrap();
        assert!(pkg.all_dependency_names().is_empty());
        assert!(pkg.production_dependency_names().is_empty());
        assert!(pkg.dev_dependency_names().is_empty());
        assert!(pkg.entry_points().is_empty());
    }

    #[test]
    fn rules_default_all_error() {
        let rules = RulesConfig::default();
        assert_eq!(rules.unused_files, Severity::Error);
        assert_eq!(rules.unused_exports, Severity::Error);
        assert_eq!(rules.unused_types, Severity::Error);
        assert_eq!(rules.unused_dependencies, Severity::Error);
        assert_eq!(rules.unused_dev_dependencies, Severity::Error);
        assert_eq!(rules.unused_enum_members, Severity::Error);
        assert_eq!(rules.unused_class_members, Severity::Error);
        assert_eq!(rules.unresolved_imports, Severity::Error);
        assert_eq!(rules.unlisted_dependencies, Severity::Error);
        assert_eq!(rules.duplicate_exports, Severity::Error);
    }

    #[test]
    fn rules_deserialize_kebab_case() {
        let json_str = r#"{
            "rules": {
                "unused-files": "error",
                "unused-exports": "warn",
                "unused-types": "off"
            }
        }"#;
        let config: FallowConfig = serde_json::from_str(json_str).unwrap();
        assert_eq!(config.rules.unused_files, Severity::Error);
        assert_eq!(config.rules.unused_exports, Severity::Warn);
        assert_eq!(config.rules.unused_types, Severity::Off);
        // Unset fields default to error
        assert_eq!(config.rules.unresolved_imports, Severity::Error);
    }

    #[test]
    fn rules_deserialize_toml_kebab_case() {
        let toml_str = r#"
[rules]
unused-files = "error"
unused-exports = "warn"
unused-types = "off"
"#;
        let config: FallowConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.rules.unused_files, Severity::Error);
        assert_eq!(config.rules.unused_exports, Severity::Warn);
        assert_eq!(config.rules.unused_types, Severity::Off);
        // Unset fields default to error
        assert_eq!(config.rules.unresolved_imports, Severity::Error);
    }

    #[test]
    fn severity_from_str() {
        assert_eq!("error".parse::<Severity>().unwrap(), Severity::Error);
        assert_eq!("warn".parse::<Severity>().unwrap(), Severity::Warn);
        assert_eq!("warning".parse::<Severity>().unwrap(), Severity::Warn);
        assert_eq!("off".parse::<Severity>().unwrap(), Severity::Off);
        assert_eq!("none".parse::<Severity>().unwrap(), Severity::Off);
        assert!("invalid".parse::<Severity>().is_err());
    }

    #[test]
    fn config_without_rules_defaults_to_error() {
        let toml_str = r#"
entry = ["src/main.ts"]
"#;
        let config: FallowConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.rules.unused_files, Severity::Error);
        assert_eq!(config.rules.unused_exports, Severity::Error);
    }

    #[test]
    fn fallow_config_denies_unknown_fields() {
        let toml_str = r#"
unknown_field = true
"#;
        let result: Result<FallowConfig, _> = toml::from_str(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn fallow_config_deserialize_json() {
        let json_str = r#"{"entry": ["src/main.ts"]}"#;
        let config: FallowConfig = serde_json::from_str(json_str).unwrap();
        assert_eq!(config.entry, vec!["src/main.ts"]);
    }

    #[test]
    fn fallow_config_deserialize_jsonc() {
        let jsonc_str = r#"{
            // This is a comment
            "entry": ["src/main.ts"],
            "rules": {
                "unused-files": "warn"
            }
        }"#;
        let mut stripped = String::new();
        json_comments::StripComments::new(jsonc_str.as_bytes())
            .read_to_string(&mut stripped)
            .unwrap();
        let config: FallowConfig = serde_json::from_str(&stripped).unwrap();
        assert_eq!(config.entry, vec!["src/main.ts"]);
        assert_eq!(config.rules.unused_files, Severity::Warn);
    }

    #[test]
    fn fallow_config_json_with_schema_field() {
        let json_str = r#"{"$schema": "https://fallow.dev/schema.json", "entry": ["src/main.ts"]}"#;
        let config: FallowConfig = serde_json::from_str(json_str).unwrap();
        assert_eq!(config.entry, vec!["src/main.ts"]);
    }

    #[test]
    fn fallow_config_json_schema_generation() {
        let schema = FallowConfig::json_schema();
        assert!(schema.is_object());
        let obj = schema.as_object().unwrap();
        assert!(obj.contains_key("properties"));
    }

    #[test]
    fn config_format_detection() {
        assert!(matches!(
            ConfigFormat::from_path(Path::new("fallow.toml")),
            ConfigFormat::Toml
        ));
        assert!(matches!(
            ConfigFormat::from_path(Path::new(".fallowrc.json")),
            ConfigFormat::Json
        ));
        assert!(matches!(
            ConfigFormat::from_path(Path::new(".fallow.toml")),
            ConfigFormat::Toml
        ));
    }

    #[test]
    fn config_names_priority_order() {
        assert_eq!(CONFIG_NAMES[0], ".fallowrc.json");
        assert_eq!(CONFIG_NAMES[1], "fallow.toml");
        assert_eq!(CONFIG_NAMES[2], ".fallow.toml");
    }

    #[test]
    fn load_json_config_file() {
        let dir = test_dir("json-config");
        let config_path = dir.join(".fallowrc.json");
        std::fs::write(
            &config_path,
            r#"{"entry": ["src/index.ts"], "rules": {"unused-exports": "warn"}}"#,
        )
        .unwrap();

        let config = FallowConfig::load(&config_path).unwrap();
        assert_eq!(config.entry, vec!["src/index.ts"]);
        assert_eq!(config.rules.unused_exports, Severity::Warn);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_jsonc_config_file() {
        let dir = test_dir("jsonc-config");
        let config_path = dir.join(".fallowrc.json");
        std::fs::write(
            &config_path,
            r#"{
                // Entry points for analysis
                "entry": ["src/index.ts"],
                /* Block comment */
                "rules": {
                    "unused-exports": "warn"
                }
            }"#,
        )
        .unwrap();

        let config = FallowConfig::load(&config_path).unwrap();
        assert_eq!(config.entry, vec!["src/index.ts"]);
        assert_eq!(config.rules.unused_exports, Severity::Warn);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn json_config_ignore_dependencies_camel_case() {
        let json_str = r#"{"ignoreDependencies": ["autoprefixer", "postcss"]}"#;
        let config: FallowConfig = serde_json::from_str(json_str).unwrap();
        assert_eq!(config.ignore_dependencies, vec!["autoprefixer", "postcss"]);
    }

    #[test]
    fn json_config_all_fields() {
        let json_str = r#"{
            "ignoreDependencies": ["lodash"],
            "ignoreExports": [{"file": "src/*.ts", "exports": ["*"]}],
            "rules": {
                "unused-files": "off",
                "unused-exports": "warn",
                "unused-dependencies": "error",
                "unused-dev-dependencies": "off",
                "unused-types": "warn",
                "unused-enum-members": "error",
                "unused-class-members": "off",
                "unresolved-imports": "warn",
                "unlisted-dependencies": "error",
                "duplicate-exports": "off"
            },
            "duplicates": {
                "minTokens": 100,
                "minLines": 10,
                "skipLocal": true
            }
        }"#;
        let config: FallowConfig = serde_json::from_str(json_str).unwrap();
        assert_eq!(config.ignore_dependencies, vec!["lodash"]);
        assert_eq!(config.rules.unused_files, Severity::Off);
        assert_eq!(config.rules.unused_exports, Severity::Warn);
        assert_eq!(config.rules.unused_dependencies, Severity::Error);
        assert_eq!(config.duplicates.min_tokens, 100);
        assert_eq!(config.duplicates.min_lines, 10);
        assert!(config.duplicates.skip_local);
    }

    // ── extends tests ──────────────────────────────────────────────

    #[test]
    fn extends_single_base() {
        let dir = test_dir("extends-single");

        std::fs::write(
            dir.join("base.json"),
            r#"{"rules": {"unused-files": "warn"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join(".fallowrc.json"),
            r#"{"extends": ["base.json"], "entry": ["src/index.ts"]}"#,
        )
        .unwrap();

        let config = FallowConfig::load(&dir.join(".fallowrc.json")).unwrap();
        assert_eq!(config.rules.unused_files, Severity::Warn);
        assert_eq!(config.entry, vec!["src/index.ts"]);
        // Unset fields from base still default
        assert_eq!(config.rules.unused_exports, Severity::Error);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extends_overlay_overrides_base() {
        let dir = test_dir("extends-overlay");

        std::fs::write(
            dir.join("base.json"),
            r#"{"rules": {"unused-files": "warn", "unused-exports": "off"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join(".fallowrc.json"),
            r#"{"extends": ["base.json"], "rules": {"unused-files": "error"}}"#,
        )
        .unwrap();

        let config = FallowConfig::load(&dir.join(".fallowrc.json")).unwrap();
        // Overlay overrides base
        assert_eq!(config.rules.unused_files, Severity::Error);
        // Base value preserved when not overridden
        assert_eq!(config.rules.unused_exports, Severity::Off);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extends_chained() {
        let dir = test_dir("extends-chained");

        std::fs::write(
            dir.join("grandparent.json"),
            r#"{"rules": {"unused-files": "off", "unused-exports": "warn"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("parent.json"),
            r#"{"extends": ["grandparent.json"], "rules": {"unused-files": "warn"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join(".fallowrc.json"),
            r#"{"extends": ["parent.json"]}"#,
        )
        .unwrap();

        let config = FallowConfig::load(&dir.join(".fallowrc.json")).unwrap();
        // grandparent: off -> parent: warn -> child: inherits warn
        assert_eq!(config.rules.unused_files, Severity::Warn);
        // grandparent: warn, not overridden
        assert_eq!(config.rules.unused_exports, Severity::Warn);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extends_circular_detected() {
        let dir = test_dir("extends-circular");

        std::fs::write(dir.join("a.json"), r#"{"extends": ["b.json"]}"#).unwrap();
        std::fs::write(dir.join("b.json"), r#"{"extends": ["a.json"]}"#).unwrap();

        let result = FallowConfig::load(&dir.join("a.json"));
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("Circular extends"),
            "Expected circular error, got: {err_msg}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extends_missing_file_errors() {
        let dir = test_dir("extends-missing");

        std::fs::write(
            dir.join(".fallowrc.json"),
            r#"{"extends": ["nonexistent.json"]}"#,
        )
        .unwrap();

        let result = FallowConfig::load(&dir.join(".fallowrc.json"));
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("not found"),
            "Expected not found error, got: {err_msg}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extends_string_sugar() {
        let dir = test_dir("extends-string");

        std::fs::write(dir.join("base.json"), r#"{"ignorePatterns": ["gen/**"]}"#).unwrap();
        // String form instead of array
        std::fs::write(dir.join(".fallowrc.json"), r#"{"extends": "base.json"}"#).unwrap();

        let config = FallowConfig::load(&dir.join(".fallowrc.json")).unwrap();
        assert_eq!(config.ignore_patterns, vec!["gen/**"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extends_deep_merge_preserves_arrays() {
        let dir = test_dir("extends-array");

        std::fs::write(dir.join("base.json"), r#"{"entry": ["src/a.ts"]}"#).unwrap();
        std::fs::write(
            dir.join(".fallowrc.json"),
            r#"{"extends": ["base.json"], "entry": ["src/b.ts"]}"#,
        )
        .unwrap();

        let config = FallowConfig::load(&dir.join(".fallowrc.json")).unwrap();
        // Arrays are replaced, not merged (overlay replaces base)
        assert_eq!(config.entry, vec!["src/b.ts"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── overrides tests ────────────────────────────────────────────

    #[test]
    fn overrides_deserialize() {
        let json_str = r#"{
            "overrides": [{
                "files": ["*.test.ts"],
                "rules": {
                    "unused-exports": "off"
                }
            }]
        }"#;
        let config: FallowConfig = serde_json::from_str(json_str).unwrap();
        assert_eq!(config.overrides.len(), 1);
        assert_eq!(config.overrides[0].files, vec!["*.test.ts"]);
        assert_eq!(
            config.overrides[0].rules.unused_exports,
            Some(Severity::Off)
        );
        assert_eq!(config.overrides[0].rules.unused_files, None);
    }

    #[test]
    fn apply_partial_only_some_fields() {
        let mut rules = RulesConfig::default();
        let partial = PartialRulesConfig {
            unused_files: Some(Severity::Warn),
            unused_exports: Some(Severity::Off),
            ..Default::default()
        };
        rules.apply_partial(&partial);
        assert_eq!(rules.unused_files, Severity::Warn);
        assert_eq!(rules.unused_exports, Severity::Off);
        // Unset fields unchanged
        assert_eq!(rules.unused_types, Severity::Error);
        assert_eq!(rules.unresolved_imports, Severity::Error);
    }

    #[test]
    fn resolve_rules_for_path_no_overrides() {
        let config = FallowConfig {
            schema: None,
            extends: vec![],
            entry: vec![],
            ignore_patterns: vec![],
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            duplicates: DuplicatesConfig::default(),
            rules: RulesConfig::default(),
            production: false,
            plugins: vec![],
            overrides: vec![],
        };
        let resolved = config.resolve(PathBuf::from("/project"), OutputFormat::Human, 1, true);
        let rules = resolved.resolve_rules_for_path(Path::new("/project/src/foo.ts"));
        assert_eq!(rules.unused_files, Severity::Error);
    }

    #[test]
    fn resolve_rules_for_path_with_matching_override() {
        let config = FallowConfig {
            schema: None,
            extends: vec![],
            entry: vec![],
            ignore_patterns: vec![],
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            duplicates: DuplicatesConfig::default(),
            rules: RulesConfig::default(),
            production: false,
            plugins: vec![],
            overrides: vec![ConfigOverride {
                files: vec!["*.test.ts".to_string()],
                rules: PartialRulesConfig {
                    unused_exports: Some(Severity::Off),
                    ..Default::default()
                },
            }],
        };
        let resolved = config.resolve(PathBuf::from("/project"), OutputFormat::Human, 1, true);

        // Test file matches override
        let test_rules = resolved.resolve_rules_for_path(Path::new("/project/src/utils.test.ts"));
        assert_eq!(test_rules.unused_exports, Severity::Off);
        assert_eq!(test_rules.unused_files, Severity::Error); // not overridden

        // Non-test file does not match
        let src_rules = resolved.resolve_rules_for_path(Path::new("/project/src/utils.ts"));
        assert_eq!(src_rules.unused_exports, Severity::Error);
    }

    #[test]
    fn resolve_rules_for_path_later_override_wins() {
        let config = FallowConfig {
            schema: None,
            extends: vec![],
            entry: vec![],
            ignore_patterns: vec![],
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            duplicates: DuplicatesConfig::default(),
            rules: RulesConfig::default(),
            production: false,
            plugins: vec![],
            overrides: vec![
                ConfigOverride {
                    files: vec!["*.ts".to_string()],
                    rules: PartialRulesConfig {
                        unused_files: Some(Severity::Warn),
                        ..Default::default()
                    },
                },
                ConfigOverride {
                    files: vec!["*.test.ts".to_string()],
                    rules: PartialRulesConfig {
                        unused_files: Some(Severity::Off),
                        ..Default::default()
                    },
                },
            ],
        };
        let resolved = config.resolve(PathBuf::from("/project"), OutputFormat::Human, 1, true);

        // First override matches *.ts, second matches *.test.ts; second wins
        let rules = resolved.resolve_rules_for_path(Path::new("/project/foo.test.ts"));
        assert_eq!(rules.unused_files, Severity::Off);

        // Non-test .ts file only matches first override
        let rules2 = resolved.resolve_rules_for_path(Path::new("/project/foo.ts"));
        assert_eq!(rules2.unused_files, Severity::Warn);
    }
}
