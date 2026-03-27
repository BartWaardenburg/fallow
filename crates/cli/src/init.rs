use std::process::ExitCode;

use fallow_config::{ExternalPluginDef, FallowConfig};

pub fn run_init(root: &std::path::Path, use_toml: bool) -> ExitCode {
    // Check if any config file already exists
    let existing_names = [".fallowrc.json", "fallow.toml", ".fallow.toml"];
    for name in &existing_names {
        let path = root.join(name);
        if path.exists() {
            eprintln!("{name} already exists");
            return ExitCode::from(2);
        }
    }

    if use_toml {
        let config_path = root.join("fallow.toml");
        let default_config = r#"# fallow.toml - Codebase analysis configuration
# See https://docs.fallow.tools for documentation

# Additional entry points (beyond auto-detected ones)
# entry = ["src/workers/*.ts"]

# Patterns to ignore
# ignorePatterns = ["**/*.generated.ts"]

# Dependencies to ignore (always considered used)
# ignoreDependencies = ["autoprefixer"]

# Per-issue-type severity: "error" (fail CI), "warn" (report only), "off" (ignore)
# All default to "error" when omitted.
# [rules]
# unused-files = "error"
# unused-exports = "warn"
# unused-types = "off"
# unresolved-imports = "error"
"#;
        if let Err(e) = std::fs::write(&config_path, default_config) {
            eprintln!("Error: Failed to write fallow.toml: {e}");
            return ExitCode::from(2);
        }
        eprintln!("Created fallow.toml");
    } else {
        let config_path = root.join(".fallowrc.json");
        let default_config = r#"{
  "$schema": "https://raw.githubusercontent.com/fallow-rs/fallow/main/schema.json",
  "rules": {}
}
"#;
        if let Err(e) = std::fs::write(&config_path, default_config) {
            eprintln!("Error: Failed to write .fallowrc.json: {e}");
            return ExitCode::from(2);
        }
        eprintln!("Created .fallowrc.json");
    }
    ExitCode::SUCCESS
}

pub fn run_config_schema() -> ExitCode {
    let schema = FallowConfig::json_schema();
    match serde_json::to_string_pretty(&schema) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: failed to serialize schema: {e}");
            ExitCode::from(2)
        }
    }
}

pub fn run_plugin_schema() -> ExitCode {
    let schema = ExternalPluginDef::json_schema();
    match serde_json::to_string_pretty(&schema) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: failed to serialize plugin schema: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_creates_json_config_by_default() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let exit = run_init(root, false);
        assert_eq!(exit, ExitCode::SUCCESS);
        let path = root.join(".fallowrc.json");
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("$schema"));
        assert!(content.contains("rules"));
    }

    #[test]
    fn init_creates_toml_config_when_requested() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let exit = run_init(root, true);
        assert_eq!(exit, ExitCode::SUCCESS);
        let path = root.join("fallow.toml");
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("fallow.toml"));
        assert!(content.contains("entry"));
        assert!(content.contains("ignorePatterns"));
    }

    #[test]
    fn init_fails_if_fallowrc_json_exists() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join(".fallowrc.json"), "{}").unwrap();
        let exit = run_init(root, false);
        assert_eq!(exit, ExitCode::from(2));
    }

    #[test]
    fn init_fails_if_fallow_toml_exists() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("fallow.toml"), "").unwrap();
        let exit = run_init(root, false);
        assert_eq!(exit, ExitCode::from(2));
    }

    #[test]
    fn init_fails_if_dot_fallow_toml_exists() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join(".fallow.toml"), "").unwrap();
        let exit = run_init(root, true);
        assert_eq!(exit, ExitCode::from(2));
    }

    #[test]
    fn init_json_config_is_valid_json() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        run_init(root, false);
        let content = std::fs::read_to_string(root.join(".fallowrc.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.is_object());
        assert!(parsed["$schema"].is_string());
    }

    #[test]
    fn init_toml_does_not_create_json() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        run_init(root, true);
        assert!(!root.join(".fallowrc.json").exists());
        assert!(root.join("fallow.toml").exists());
    }

    #[test]
    fn init_json_does_not_create_toml() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        run_init(root, false);
        assert!(!root.join("fallow.toml").exists());
        assert!(root.join(".fallowrc.json").exists());
    }

    #[test]
    fn init_existing_config_blocks_both_formats() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // Existing .fallowrc.json should block both JSON and TOML creation
        std::fs::write(root.join(".fallowrc.json"), "{}").unwrap();
        assert_eq!(run_init(root, false), ExitCode::from(2));
        assert_eq!(run_init(root, true), ExitCode::from(2));
    }
}
