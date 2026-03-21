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
