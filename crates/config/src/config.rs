use std::io::Read as _;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::external_plugin::{ExternalPluginDef, discover_external_plugins};
use crate::framework::FrameworkPreset;
use crate::workspace::WorkspaceConfig;

/// Supported config file names in priority order.
///
/// `find_and_load` checks these names in order within each directory,
/// returning the first match found.
const CONFIG_NAMES: &[&str] = &["fallow.jsonc", "fallow.json", "fallow.toml", ".fallow.toml"];

/// User-facing configuration loaded from `fallow.jsonc`, `fallow.json`, or `fallow.toml`.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FallowConfig {
    /// JSON Schema reference (ignored during deserialization).
    #[serde(rename = "$schema", default, skip_serializing)]
    #[schemars(skip)]
    pub schema: Option<String>,

    /// Additional entry point glob patterns.
    #[serde(default)]
    pub entry: Vec<String>,

    /// Glob patterns to ignore from analysis.
    #[serde(default)]
    pub ignore: Vec<String>,

    /// What to detect.
    #[serde(default)]
    pub detect: DetectConfig,

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

    /// Duplication detection settings.
    #[serde(default)]
    pub duplicates: DuplicatesConfig,

    /// Per-issue-type severity rules.
    #[serde(default)]
    pub rules: RulesConfig,

    /// Production mode: exclude test/dev files, only start/build scripts.
    #[serde(default)]
    pub production: bool,

    /// Paths to external plugin files or directories containing plugin TOML files.
    ///
    /// In addition to these explicit paths, fallow automatically discovers:
    /// - `*.toml` files in `.fallow/plugins/`
    /// - `fallow-plugin-*.toml` files in the project root
    #[serde(default)]
    pub plugins: Vec<String>,
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

/// Controls which analyses to run.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Default, Clone, Deserialize, Serialize, JsonSchema)]
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
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
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
    /// External plugin definitions loaded from TOML files.
    pub external_plugins: Vec<ExternalPluginDef>,
}

/// Detect config format from file extension.
enum ConfigFormat {
    Toml,
    Json,
    Jsonc,
}

impl ConfigFormat {
    fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|e| e.to_str()) {
            Some("toml") => Self::Toml,
            Some("jsonc") => Self::Jsonc,
            Some("json") => Self::Json,
            _ => Self::Toml,
        }
    }
}

impl FallowConfig {
    /// Load config from a fallow config file (TOML, JSON, or JSONC).
    ///
    /// The format is detected from the file extension:
    /// - `.toml` → TOML
    /// - `.json` → JSON
    /// - `.jsonc` → JSONC (JSON with comments)
    pub fn load(path: &Path) -> Result<Self, miette::Report> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| miette::miette!("Failed to read config file {}: {}", path.display(), e))?;

        match ConfigFormat::from_path(path) {
            ConfigFormat::Toml => toml::from_str(&content).map_err(|e| {
                miette::miette!("Failed to parse config file {}: {}", path.display(), e)
            }),
            ConfigFormat::Json => serde_json::from_str(&content).map_err(|e| {
                miette::miette!("Failed to parse config file {}: {}", path.display(), e)
            }),
            ConfigFormat::Jsonc => {
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

    /// Find and load config from the current directory or ancestors.
    ///
    /// Checks for config files in priority order:
    /// `fallow.jsonc` > `fallow.json` > `fallow.toml` > `.fallow.toml`
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
        serde_json::to_value(schemars::schema_for!(FallowConfig)).unwrap_or_default()
    }

    /// Resolve into a fully resolved config with compiled globs.
    pub fn resolve(self, root: PathBuf, threads: usize, no_cache: bool) -> ResolvedConfig {
        let mut ignore_builder = GlobSetBuilder::new();
        for pattern in &self.ignore {
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

        let framework_rules = crate::framework::resolve_framework_rules(&self.framework);

        // Merge detect booleans into rules: detect=false forces Severity::Off
        let mut rules = self.rules;
        rules.merge_detect(&self.detect);

        // In production mode, force unused_dev_dependencies off
        let production = self.production;
        if production {
            rules.unused_dev_dependencies = Severity::Off;
        }

        let external_plugins = discover_external_plugins(&root, &self.plugins);

        ResolvedConfig {
            root,
            entry_patterns: self.entry,
            ignore_patterns,
            detect: self.detect,
            framework_rules,
            output: self.output,
            cache_dir,
            threads,
            no_cache,
            ignore_dependencies: self.ignore_dependencies,
            ignore_export_rules: self.ignore_exports,
            duplicates: self.duplicates,
            rules,
            production,
            external_plugins,
        }
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
/// or are suppressed entirely. All fields default to `Severity::Error`
/// for backwards compatibility.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
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
    /// Merge `DetectConfig` booleans: if `detect.X = false`, force `rules.X = Off`.
    pub fn merge_detect(&mut self, detect: &DetectConfig) {
        if !detect.unused_files {
            self.unused_files = Severity::Off;
        }
        if !detect.unused_exports {
            self.unused_exports = Severity::Off;
        }
        if !detect.unused_types {
            self.unused_types = Severity::Off;
        }
        if !detect.unused_dependencies {
            self.unused_dependencies = Severity::Off;
        }
        if !detect.unused_dev_dependencies {
            self.unused_dev_dependencies = Severity::Off;
        }
        if !detect.unused_enum_members {
            self.unused_enum_members = Severity::Off;
        }
        if !detect.unused_class_members {
            self.unused_class_members = Severity::Off;
        }
        if !detect.unresolved_imports {
            self.unresolved_imports = Severity::Off;
        }
        if !detect.unlisted_dependencies {
            self.unlisted_dependencies = Severity::Off;
        }
        if !detect.duplicate_exports {
            self.duplicate_exports = Severity::Off;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PackageJson;

    #[test]
    fn detect_config_default_all_true() {
        let config = DetectConfig::default();
        assert!(config.unused_files);
        assert!(config.unused_exports);
        assert!(config.unused_dependencies);
        assert!(config.unused_dev_dependencies);
        assert!(config.unused_types);
        assert!(config.unused_enum_members);
        assert!(config.unused_class_members);
        assert!(config.unresolved_imports);
        assert!(config.unlisted_dependencies);
        assert!(config.duplicate_exports);
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
        assert!(config.ignore.is_empty());
        assert!(config.detect.unused_files); // default true
    }

    #[test]
    fn fallow_config_deserialize_detect_overrides() {
        let toml_str = r#"
[detect]
unusedFiles = false
unusedExports = true
unusedDependencies = false
"#;
        let config: FallowConfig = toml::from_str(toml_str).unwrap();
        assert!(!config.detect.unused_files);
        assert!(config.detect.unused_exports);
        assert!(!config.detect.unused_dependencies);
        // Others should default to true
        assert!(config.detect.unused_types);
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
            entry: vec![],
            ignore: vec![],
            detect: DetectConfig::default(),
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            output: OutputFormat::Human,
            duplicates: DuplicatesConfig::default(),
            rules: RulesConfig::default(),
            production: false,
            plugins: vec![],
        };
        let resolved = config.resolve(PathBuf::from("/tmp/test"), 4, true);

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
            entry: vec!["src/**/*.ts".to_string()],
            ignore: vec!["**/*.generated.ts".to_string()],
            detect: DetectConfig::default(),
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            output: OutputFormat::Json,
            duplicates: DuplicatesConfig::default(),
            rules: RulesConfig::default(),
            production: false,
            plugins: vec![],
        };
        let resolved = config.resolve(PathBuf::from("/tmp/test"), 4, false);

        assert!(resolved.ignore_patterns.is_match("src/foo.generated.ts"));
        assert_eq!(resolved.entry_patterns, vec!["src/**/*.ts"]);
        assert!(matches!(resolved.output, OutputFormat::Json));
        assert!(!resolved.no_cache);
    }

    #[test]
    fn fallow_config_resolve_cache_dir() {
        let config = FallowConfig {
            schema: None,
            entry: vec![],
            ignore: vec![],
            detect: DetectConfig::default(),
            framework: vec![],
            workspaces: None,
            ignore_dependencies: vec![],
            ignore_exports: vec![],
            output: OutputFormat::Human,
            duplicates: DuplicatesConfig::default(),
            rules: RulesConfig::default(),
            production: false,
            plugins: vec![],
        };
        let resolved = config.resolve(PathBuf::from("/tmp/project"), 4, true);
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
    fn rules_deserialize_mixed_severities() {
        let toml_str = r#"
[rules]
unusedFiles = "error"
unusedExports = "warn"
unusedTypes = "off"
"#;
        let config: FallowConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.rules.unused_files, Severity::Error);
        assert_eq!(config.rules.unused_exports, Severity::Warn);
        assert_eq!(config.rules.unused_types, Severity::Off);
        // Unset fields default to error
        assert_eq!(config.rules.unresolved_imports, Severity::Error);
    }

    #[test]
    fn detect_false_forces_severity_off() {
        let toml_str = r#"
[detect]
unusedFiles = false

[rules]
unusedFiles = "error"
"#;
        let config: FallowConfig = toml::from_str(toml_str).unwrap();
        let resolved = config.resolve(PathBuf::from("/tmp/test"), 4, true);
        // detect=false overrides rules=error → off
        assert_eq!(resolved.rules.unused_files, Severity::Off);
    }

    #[test]
    fn rules_off_independent_of_detect() {
        let toml_str = r#"
[detect]
unusedFiles = true

[rules]
unusedFiles = "off"
"#;
        let config: FallowConfig = toml::from_str(toml_str).unwrap();
        let resolved = config.resolve(PathBuf::from("/tmp/test"), 4, true);
        // detect=true but rules=off → off (rules win when detect is true)
        assert_eq!(resolved.rules.unused_files, Severity::Off);
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
    fn fallow_config_deserialize_json_camel_case() {
        let json_str = r#"{"entry": ["src/main.ts"], "detect": {"unusedFiles": false}}"#;
        let config: FallowConfig = serde_json::from_str(json_str).unwrap();
        assert_eq!(config.entry, vec!["src/main.ts"]);
        assert!(!config.detect.unused_files);
        assert!(config.detect.unused_exports); // default true
    }

    #[test]
    fn fallow_config_deserialize_jsonc() {
        let jsonc_str = r#"{
            // This is a comment
            "entry": ["src/main.ts"],
            "rules": {
                "unusedFiles": "warn"
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
            ConfigFormat::from_path(Path::new("fallow.json")),
            ConfigFormat::Json
        ));
        assert!(matches!(
            ConfigFormat::from_path(Path::new("fallow.jsonc")),
            ConfigFormat::Jsonc
        ));
        assert!(matches!(
            ConfigFormat::from_path(Path::new(".fallow.toml")),
            ConfigFormat::Toml
        ));
    }

    #[test]
    fn config_names_priority_order() {
        assert_eq!(CONFIG_NAMES[0], "fallow.jsonc");
        assert_eq!(CONFIG_NAMES[1], "fallow.json");
        assert_eq!(CONFIG_NAMES[2], "fallow.toml");
        assert_eq!(CONFIG_NAMES[3], ".fallow.toml");
    }

    #[test]
    fn load_json_config_file() {
        let dir = std::env::temp_dir().join("fallow-test-json-config");
        let _ = std::fs::create_dir_all(&dir);
        let config_path = dir.join("fallow.json");
        std::fs::write(
            &config_path,
            r#"{"entry": ["src/index.ts"], "rules": {"unusedExports": "warn"}}"#,
        )
        .unwrap();

        let config = FallowConfig::load(&config_path).unwrap();
        assert_eq!(config.entry, vec!["src/index.ts"]);
        assert_eq!(config.rules.unused_exports, Severity::Warn);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_jsonc_config_file() {
        let dir = std::env::temp_dir().join("fallow-test-jsonc-config");
        let _ = std::fs::create_dir_all(&dir);
        let config_path = dir.join("fallow.jsonc");
        std::fs::write(
            &config_path,
            r#"{
                // Entry points for analysis
                "entry": ["src/index.ts"],
                /* Block comment */
                "rules": {
                    "unusedExports": "warn"
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
    fn json_config_all_camel_case_fields() {
        let json_str = r#"{
            "ignoreDependencies": ["lodash"],
            "ignoreExports": [{"file": "src/*.ts", "exports": ["*"]}],
            "detect": {
                "unusedFiles": false,
                "unusedExports": true,
                "unusedDependencies": false,
                "unusedDevDependencies": true,
                "unusedTypes": false,
                "unusedEnumMembers": true,
                "unusedClassMembers": false,
                "unresolvedImports": true,
                "unlistedDependencies": false,
                "duplicateExports": true
            },
            "rules": {
                "unusedFiles": "off",
                "unusedExports": "warn",
                "unusedDependencies": "error",
                "unusedDevDependencies": "off",
                "unusedTypes": "warn",
                "unusedEnumMembers": "error",
                "unusedClassMembers": "off",
                "unresolvedImports": "warn",
                "unlistedDependencies": "error",
                "duplicateExports": "off"
            },
            "duplicates": {
                "minTokens": 100,
                "minLines": 10,
                "skipLocal": true
            }
        }"#;
        let config: FallowConfig = serde_json::from_str(json_str).unwrap();
        assert_eq!(config.ignore_dependencies, vec!["lodash"]);
        assert!(!config.detect.unused_files);
        assert!(config.detect.unused_exports);
        assert!(config.detect.unused_enum_members);
        assert!(!config.detect.unused_class_members);
        assert_eq!(config.rules.unused_files, Severity::Off);
        assert_eq!(config.rules.unused_exports, Severity::Warn);
        assert_eq!(config.rules.unused_dependencies, Severity::Error);
        assert_eq!(config.duplicates.min_tokens, 100);
        assert_eq!(config.duplicates.min_lines, 10);
        assert!(config.duplicates.skip_local);
    }
}
