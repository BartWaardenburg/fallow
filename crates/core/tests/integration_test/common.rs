use std::path::PathBuf;

use fallow_config::{FallowConfig, OutputFormat, RulesConfig};

pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
        .join(name)
}

pub fn create_config(root: PathBuf) -> fallow_config::ResolvedConfig {
    FallowConfig {
        schema: None,
        extends: vec![],
        entry: vec![],
        ignore_patterns: vec![],
        framework: vec![],
        workspaces: None,
        ignore_dependencies: vec![],
        ignore_exports: vec![],
        duplicates: fallow_config::DuplicatesConfig::default(),
        rules: RulesConfig::default(),
        production: false,
        plugins: vec![],
        overrides: vec![],
    }
    .resolve(root, OutputFormat::Human, 4, true, true)
}

pub fn create_config_with_cache(
    root: PathBuf,
    cache_dir: std::path::PathBuf,
) -> fallow_config::ResolvedConfig {
    let mut config = FallowConfig {
        schema: None,
        extends: vec![],
        entry: vec![],
        ignore_patterns: vec![],
        framework: vec![],
        workspaces: None,
        ignore_dependencies: vec![],
        ignore_exports: vec![],
        duplicates: fallow_config::DuplicatesConfig::default(),
        rules: RulesConfig::default(),
        production: false,
        plugins: vec![],
        overrides: vec![],
    }
    .resolve(root, OutputFormat::Human, 4, false, true); // no_cache = false to enable caching
    config.cache_dir = cache_dir;
    config
}
