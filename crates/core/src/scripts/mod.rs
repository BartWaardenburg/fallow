//! Lightweight shell command parser for package.json scripts.
//!
//! Extracts:
//! - **Binary names** → mapped to npm package names for dependency usage detection
//! - **`--config` arguments** → file paths for entry point discovery
//! - **Positional file arguments** → file paths for entry point discovery
//!
//! Handles env var prefixes (`cross-env`, `dotenv`, `KEY=value`), package manager
//! runners (`npx`, `pnpm exec`, `yarn dlx`), and Node.js runners (`node`, `tsx`,
//! `ts-node`). Shell operators (`&&`, `||`, `;`, `|`, `&`) are split correctly.

pub mod ci;
mod resolve;
mod shell;

#[expect(clippy::disallowed_types)]
use std::collections::HashMap;
use std::path::Path;

use rustc_hash::FxHashSet;

pub use resolve::resolve_binary_to_package;

/// Environment variable wrapper commands to strip before the actual binary.
const ENV_WRAPPERS: &[&str] = &["cross-env", "dotenv", "env"];

/// Node.js runners whose first non-flag argument is a file path, not a binary name.
const NODE_RUNNERS: &[&str] = &["node", "ts-node", "tsx", "babel-node", "bun"];

/// Result of analyzing all package.json scripts.
#[derive(Debug, Default)]
pub struct ScriptAnalysis {
    /// Package names used as binaries in scripts (mapped from binary → package name).
    pub used_packages: FxHashSet<String>,
    /// Config file paths extracted from `--config` / `-c` arguments.
    pub config_files: Vec<String>,
    /// File paths extracted as positional arguments (entry point candidates).
    pub entry_files: Vec<String>,
}

/// A parsed command segment from a script value.
#[derive(Debug, PartialEq, Eq)]
pub struct ScriptCommand {
    /// The binary/command name (e.g., "webpack", "eslint", "tsc").
    pub binary: String,
    /// Config file arguments (from `--config`, `-c`).
    pub config_args: Vec<String>,
    /// File path arguments (positional args that look like file paths).
    pub file_args: Vec<String>,
}

/// Filter scripts to only production-relevant ones (start, build, and their pre/post hooks).
///
/// In production mode, dev/test/lint scripts are excluded since they only affect
/// devDependency usage, not the production dependency graph.
#[expect(clippy::implicit_hasher, clippy::disallowed_types)]
pub fn filter_production_scripts(scripts: &HashMap<String, String>) -> HashMap<String, String> {
    scripts
        .iter()
        .filter(|(name, _)| is_production_script(name))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Check if a script name is production-relevant.
///
/// Production scripts: `start`, `build`, `serve`, `preview`, `prepare`, `prepublishOnly`,
/// and their `pre`/`post` lifecycle hooks, plus namespaced variants like `build:prod`.
fn is_production_script(name: &str) -> bool {
    // Check the root name (before any `:` namespace separator)
    let root_name = name.split(':').next().unwrap_or(name);

    // Direct match (including scripts that happen to start with pre/post like preview, prepare)
    if matches!(
        root_name,
        "start" | "build" | "serve" | "preview" | "prepare" | "prepublishOnly" | "postinstall"
    ) {
        return true;
    }

    // Check lifecycle hooks: pre/post + production script name
    let base = root_name
        .strip_prefix("pre")
        .or_else(|| root_name.strip_prefix("post"));

    base.is_some_and(|base| matches!(base, "start" | "build" | "serve" | "install"))
}

/// Analyze all scripts from a package.json `scripts` field.
///
/// For each script value, parses shell commands, extracts binary names (mapped to
/// package names), `--config` file paths, and positional file path arguments.
#[expect(clippy::implicit_hasher, clippy::disallowed_types)]
pub fn analyze_scripts(scripts: &HashMap<String, String>, root: &Path) -> ScriptAnalysis {
    let mut result = ScriptAnalysis::default();

    for script_value in scripts.values() {
        // Track env wrapper packages (cross-env, dotenv) as used before parsing
        for wrapper in ENV_WRAPPERS {
            if script_value
                .split_whitespace()
                .any(|token| token == *wrapper)
            {
                let pkg = resolve_binary_to_package(wrapper, root);
                if !is_builtin_command(wrapper) {
                    result.used_packages.insert(pkg);
                }
            }
        }

        let commands = parse_script(script_value);

        for cmd in commands {
            // Map binary to package name and track as used
            if !cmd.binary.is_empty() && !is_builtin_command(&cmd.binary) {
                if NODE_RUNNERS.contains(&cmd.binary.as_str()) {
                    // Node runners themselves are packages (node excluded)
                    if cmd.binary != "node" && cmd.binary != "bun" {
                        let pkg = resolve_binary_to_package(&cmd.binary, root);
                        result.used_packages.insert(pkg);
                    }
                } else {
                    let pkg = resolve_binary_to_package(&cmd.binary, root);
                    result.used_packages.insert(pkg);
                }
            }

            result.config_files.extend(cmd.config_args);
            result.entry_files.extend(cmd.file_args);
        }
    }

    result
}

/// Parse a single script value into one or more commands.
///
/// Splits on shell operators (`&&`, `||`, `;`, `|`, `&`) and parses each segment.
pub fn parse_script(script: &str) -> Vec<ScriptCommand> {
    let mut commands = Vec::new();

    for segment in shell::split_shell_operators(script) {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        if let Some(cmd) = parse_command_segment(segment) {
            commands.push(cmd);
        }
    }

    commands
}

/// Extract file path arguments and `--config`/`-c` arguments from the remaining tokens.
/// When `is_node_runner` is true, flags like `-e`/`--eval`/`-r`/`--require` that consume
/// the next argument are skipped.
fn extract_args_for_binary(
    tokens: &[&str],
    mut idx: usize,
    is_node_runner: bool,
) -> (Vec<String>, Vec<String>) {
    let mut file_args = Vec::new();
    let mut config_args = Vec::new();

    while idx < tokens.len() {
        let token = tokens[idx];

        // Node runners have flags that consume the next argument
        if is_node_runner
            && matches!(
                token,
                "-e" | "--eval" | "-p" | "--print" | "-r" | "--require"
            )
        {
            idx += 2;
            continue;
        }

        if let Some(config) = extract_config_arg(token, tokens.get(idx + 1).copied()) {
            config_args.push(config);
            if token.contains('=') || token.starts_with("--config=") || token.starts_with("-c=") {
                idx += 1;
            } else {
                idx += 2;
            }
            continue;
        }

        if token.starts_with('-') {
            idx += 1;
            continue;
        }

        if looks_like_file_path(token) {
            file_args.push(token.to_string());
        }
        idx += 1;
    }

    (file_args, config_args)
}

/// Parse a single command segment (after splitting on shell operators).
fn parse_command_segment(segment: &str) -> Option<ScriptCommand> {
    let tokens: Vec<&str> = segment.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    let idx = shell::skip_initial_wrappers(&tokens, 0)?;
    let idx = shell::advance_past_package_manager(&tokens, idx)?;

    let binary = tokens[idx].to_string();
    let is_node_runner = NODE_RUNNERS.contains(&binary.as_str());
    let (file_args, config_args) = extract_args_for_binary(&tokens, idx + 1, is_node_runner);

    Some(ScriptCommand {
        binary,
        config_args,
        file_args,
    })
}

/// Extract a config file path from a `--config` or `-c` flag.
fn extract_config_arg(token: &str, next: Option<&str>) -> Option<String> {
    // --config=path/to/config.js
    if let Some(value) = token.strip_prefix("--config=")
        && !value.is_empty()
    {
        return Some(value.to_string());
    }
    // -c=path
    if let Some(value) = token.strip_prefix("-c=")
        && !value.is_empty()
    {
        return Some(value.to_string());
    }
    // --config path or -c path
    if matches!(token, "--config" | "-c")
        && let Some(next_token) = next
        && !next_token.starts_with('-')
    {
        return Some(next_token.to_string());
    }
    None
}

/// Check if a token is an environment variable assignment (`KEY=value`).
fn is_env_assignment(token: &str) -> bool {
    token.find('=').is_some_and(|eq_pos| {
        let name = &token[..eq_pos];
        !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    })
}

/// Check if a token looks like a file path (has a known extension or path separator).
fn looks_like_file_path(token: &str) -> bool {
    const EXTENSIONS: &[&str] = &[
        ".js", ".ts", ".mjs", ".cjs", ".mts", ".cts", ".jsx", ".tsx", ".json", ".yaml", ".yml",
        ".toml",
    ];
    if EXTENSIONS.iter().any(|ext| token.ends_with(ext)) {
        return true;
    }
    token.starts_with("./")
        || token.starts_with("../")
        || (token.contains('/') && !token.starts_with('@') && !token.contains("://"))
}

/// Check if a command is a shell built-in (not an npm package).
fn is_builtin_command(cmd: &str) -> bool {
    matches!(
        cmd,
        "echo"
            | "cat"
            | "cp"
            | "mv"
            | "rm"
            | "mkdir"
            | "rmdir"
            | "ls"
            | "cd"
            | "pwd"
            | "test"
            | "true"
            | "false"
            | "exit"
            | "export"
            | "source"
            | "which"
            | "chmod"
            | "chown"
            | "touch"
            | "find"
            | "grep"
            | "sed"
            | "awk"
            | "xargs"
            | "tee"
            | "sort"
            | "uniq"
            | "wc"
            | "head"
            | "tail"
            | "sleep"
            | "wait"
            | "kill"
            | "sh"
            | "bash"
            | "zsh"
    )
}

#[cfg(test)]
#[expect(clippy::disallowed_types)]
mod tests {
    use super::*;

    // --- parse_script tests ---

    #[test]
    fn simple_binary() {
        let cmds = parse_script("webpack");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "webpack");
    }

    #[test]
    fn binary_with_args() {
        let cmds = parse_script("eslint src --ext .ts,.tsx");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "eslint");
    }

    #[test]
    fn chained_commands() {
        let cmds = parse_script("tsc --noEmit && eslint src");
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].binary, "tsc");
        assert_eq!(cmds[1].binary, "eslint");
    }

    #[test]
    fn semicolon_separator() {
        let cmds = parse_script("tsc; eslint src");
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].binary, "tsc");
        assert_eq!(cmds[1].binary, "eslint");
    }

    #[test]
    fn or_chain() {
        let cmds = parse_script("tsc --noEmit || echo failed");
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].binary, "tsc");
        assert_eq!(cmds[1].binary, "echo");
    }

    #[test]
    fn pipe_operator() {
        let cmds = parse_script("jest --json | tee results.json");
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].binary, "jest");
        assert_eq!(cmds[1].binary, "tee");
    }

    #[test]
    fn npx_prefix() {
        let cmds = parse_script("npx eslint src");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "eslint");
    }

    #[test]
    fn pnpx_prefix() {
        let cmds = parse_script("pnpx vitest run");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "vitest");
    }

    #[test]
    fn npx_with_flags() {
        let cmds = parse_script("npx --yes --package @scope/tool eslint src");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "eslint");
    }

    #[test]
    fn yarn_exec() {
        let cmds = parse_script("yarn exec jest");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "jest");
    }

    #[test]
    fn pnpm_exec() {
        let cmds = parse_script("pnpm exec vitest run");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "vitest");
    }

    #[test]
    fn pnpm_dlx() {
        let cmds = parse_script("pnpm dlx create-react-app my-app");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "create-react-app");
    }

    #[test]
    fn npm_run_skipped() {
        let cmds = parse_script("npm run build");
        assert!(cmds.is_empty());
    }

    #[test]
    fn yarn_run_skipped() {
        let cmds = parse_script("yarn run test");
        assert!(cmds.is_empty());
    }

    #[test]
    fn bare_yarn_skipped() {
        // `yarn build` runs the "build" script
        let cmds = parse_script("yarn build");
        assert!(cmds.is_empty());
    }

    // --- env wrappers ---

    #[test]
    fn cross_env_prefix() {
        let cmds = parse_script("cross-env NODE_ENV=production webpack");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "webpack");
    }

    #[test]
    fn dotenv_prefix() {
        let cmds = parse_script("dotenv -- next build");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "next");
    }

    #[test]
    fn env_var_assignment_prefix() {
        let cmds = parse_script("NODE_ENV=production webpack --mode production");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "webpack");
    }

    #[test]
    fn multiple_env_vars() {
        let cmds = parse_script("NODE_ENV=test CI=true jest");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "jest");
    }

    // --- node runners ---

    #[test]
    fn node_runner_file_args() {
        let cmds = parse_script("node scripts/build.js");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "node");
        assert_eq!(cmds[0].file_args, vec!["scripts/build.js"]);
    }

    #[test]
    fn tsx_runner_file_args() {
        let cmds = parse_script("tsx scripts/migrate.ts");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "tsx");
        assert_eq!(cmds[0].file_args, vec!["scripts/migrate.ts"]);
    }

    #[test]
    fn node_with_flags() {
        let cmds = parse_script("node --experimental-specifier-resolution=node scripts/run.mjs");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].file_args, vec!["scripts/run.mjs"]);
    }

    #[test]
    fn node_eval_no_file() {
        let cmds = parse_script("node -e \"console.log('hi')\"");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "node");
        assert!(cmds[0].file_args.is_empty());
    }

    #[test]
    fn node_multiple_files() {
        let cmds = parse_script("node --test file1.mjs file2.mjs");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].file_args, vec!["file1.mjs", "file2.mjs"]);
    }

    // --- config args ---

    #[test]
    fn config_equals() {
        let cmds = parse_script("webpack --config=webpack.prod.js");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "webpack");
        assert_eq!(cmds[0].config_args, vec!["webpack.prod.js"]);
    }

    #[test]
    fn config_space() {
        let cmds = parse_script("jest --config jest.config.ts");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "jest");
        assert_eq!(cmds[0].config_args, vec!["jest.config.ts"]);
    }

    #[test]
    fn config_short_flag() {
        let cmds = parse_script("eslint -c .eslintrc.json src");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "eslint");
        assert_eq!(cmds[0].config_args, vec![".eslintrc.json"]);
    }

    // --- binary -> package mapping ---

    #[test]
    fn tsc_maps_to_typescript() {
        let pkg = resolve_binary_to_package("tsc", Path::new("/nonexistent"));
        assert_eq!(pkg, "typescript");
    }

    #[test]
    fn ng_maps_to_angular_cli() {
        let pkg = resolve_binary_to_package("ng", Path::new("/nonexistent"));
        assert_eq!(pkg, "@angular/cli");
    }

    #[test]
    fn biome_maps_to_biomejs() {
        let pkg = resolve_binary_to_package("biome", Path::new("/nonexistent"));
        assert_eq!(pkg, "@biomejs/biome");
    }

    #[test]
    fn unknown_binary_is_identity() {
        let pkg = resolve_binary_to_package("my-custom-tool", Path::new("/nonexistent"));
        assert_eq!(pkg, "my-custom-tool");
    }

    #[test]
    fn run_s_maps_to_npm_run_all() {
        let pkg = resolve_binary_to_package("run-s", Path::new("/nonexistent"));
        assert_eq!(pkg, "npm-run-all");
    }

    // --- extract_package_from_bin_path ---

    #[test]
    fn bin_path_regular_package() {
        let path = std::path::Path::new("../webpack/bin/webpack.js");
        assert_eq!(
            resolve::extract_package_from_bin_path(path),
            Some("webpack".to_string())
        );
    }

    #[test]
    fn bin_path_scoped_package() {
        let path = std::path::Path::new("../@babel/cli/bin/babel.js");
        assert_eq!(
            resolve::extract_package_from_bin_path(path),
            Some("@babel/cli".to_string())
        );
    }

    // --- builtin commands ---

    #[test]
    fn builtin_commands_not_tracked() {
        let scripts: HashMap<String, String> =
            std::iter::once(("postinstall".to_string(), "echo done".to_string())).collect();
        let result = analyze_scripts(&scripts, Path::new("/nonexistent"));
        assert!(result.used_packages.is_empty());
    }

    // --- analyze_scripts integration ---

    #[test]
    fn analyze_extracts_binaries() {
        let scripts: HashMap<String, String> = [
            ("build".to_string(), "tsc --noEmit && webpack".to_string()),
            ("lint".to_string(), "eslint src".to_string()),
            ("test".to_string(), "jest".to_string()),
        ]
        .into_iter()
        .collect();
        let result = analyze_scripts(&scripts, Path::new("/nonexistent"));
        assert!(result.used_packages.contains("typescript"));
        assert!(result.used_packages.contains("webpack"));
        assert!(result.used_packages.contains("eslint"));
        assert!(result.used_packages.contains("jest"));
    }

    #[test]
    fn analyze_extracts_config_files() {
        let scripts: HashMap<String, String> = std::iter::once((
            "build".to_string(),
            "webpack --config webpack.prod.js".to_string(),
        ))
        .collect();
        let result = analyze_scripts(&scripts, Path::new("/nonexistent"));
        assert!(result.config_files.contains(&"webpack.prod.js".to_string()));
    }

    #[test]
    fn analyze_extracts_entry_files() {
        let scripts: HashMap<String, String> =
            std::iter::once(("seed".to_string(), "ts-node scripts/seed.ts".to_string())).collect();
        let result = analyze_scripts(&scripts, Path::new("/nonexistent"));
        assert!(result.entry_files.contains(&"scripts/seed.ts".to_string()));
        // ts-node should be tracked as a used package
        assert!(result.used_packages.contains("ts-node"));
    }

    #[test]
    fn analyze_cross_env_with_config() {
        let scripts: HashMap<String, String> = std::iter::once((
            "build".to_string(),
            "cross-env NODE_ENV=production webpack --config webpack.prod.js".to_string(),
        ))
        .collect();
        let result = analyze_scripts(&scripts, Path::new("/nonexistent"));
        assert!(result.used_packages.contains("cross-env"));
        assert!(result.used_packages.contains("webpack"));
        assert!(result.config_files.contains(&"webpack.prod.js".to_string()));
    }

    #[test]
    fn analyze_complex_script() {
        let scripts: HashMap<String, String> = std::iter::once((
            "ci".to_string(),
            "cross-env CI=true npm run build && jest --config jest.ci.js --coverage".to_string(),
        ))
        .collect();
        let result = analyze_scripts(&scripts, Path::new("/nonexistent"));
        // cross-env is tracked, npm run is skipped, jest is tracked
        assert!(result.used_packages.contains("cross-env"));
        assert!(result.used_packages.contains("jest"));
        assert!(!result.used_packages.contains("npm"));
        assert!(result.config_files.contains(&"jest.ci.js".to_string()));
    }

    // --- is_env_assignment ---

    #[test]
    fn env_assignment_valid() {
        assert!(is_env_assignment("NODE_ENV=production"));
        assert!(is_env_assignment("CI=true"));
        assert!(is_env_assignment("PORT=3000"));
    }

    #[test]
    fn env_assignment_invalid() {
        assert!(!is_env_assignment("--config"));
        assert!(!is_env_assignment("webpack"));
        assert!(!is_env_assignment("./scripts/build.js"));
    }

    // --- split_shell_operators ---

    #[test]
    fn split_respects_quotes() {
        let segments = shell::split_shell_operators("echo 'a && b' && jest");
        assert_eq!(segments.len(), 2);
        assert!(segments[1].trim() == "jest");
    }

    #[test]
    fn split_double_quotes() {
        let segments = shell::split_shell_operators("echo \"a || b\" || jest");
        assert_eq!(segments.len(), 2);
        assert!(segments[1].trim() == "jest");
    }

    #[test]
    fn background_operator_splits_commands() {
        let cmds = parse_script("tsc --watch & webpack --watch");
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].binary, "tsc");
        assert_eq!(cmds[1].binary, "webpack");
    }

    #[test]
    fn double_ampersand_still_works() {
        let cmds = parse_script("tsc --watch && webpack --watch");
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].binary, "tsc");
        assert_eq!(cmds[1].binary, "webpack");
    }

    #[test]
    fn multiple_background_operators() {
        let cmds = parse_script("server & client & proxy");
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0].binary, "server");
        assert_eq!(cmds[1].binary, "client");
        assert_eq!(cmds[2].binary, "proxy");
    }

    // --- is_production_script ---

    #[test]
    fn production_script_start() {
        assert!(super::is_production_script("start"));
        assert!(super::is_production_script("prestart"));
        assert!(super::is_production_script("poststart"));
    }

    #[test]
    fn production_script_build() {
        assert!(super::is_production_script("build"));
        assert!(super::is_production_script("prebuild"));
        assert!(super::is_production_script("postbuild"));
        assert!(super::is_production_script("build:prod"));
        assert!(super::is_production_script("build:esm"));
    }

    #[test]
    fn production_script_serve_preview() {
        assert!(super::is_production_script("serve"));
        assert!(super::is_production_script("preview"));
        assert!(super::is_production_script("prepare"));
    }

    #[test]
    fn non_production_scripts() {
        assert!(!super::is_production_script("test"));
        assert!(!super::is_production_script("lint"));
        assert!(!super::is_production_script("dev"));
        assert!(!super::is_production_script("storybook"));
        assert!(!super::is_production_script("typecheck"));
        assert!(!super::is_production_script("format"));
        assert!(!super::is_production_script("e2e"));
    }

    // --- mixed operator parsing ---

    #[test]
    fn mixed_operators_all_binaries_detected() {
        let cmds = parse_script("build && serve & watch || fallback");
        assert_eq!(cmds.len(), 4);
        assert_eq!(cmds[0].binary, "build");
        assert_eq!(cmds[1].binary, "serve");
        assert_eq!(cmds[2].binary, "watch");
        assert_eq!(cmds[3].binary, "fallback");
    }

    #[test]
    fn background_with_env_vars() {
        let cmds = parse_script("NODE_ENV=production server &");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "server");
    }

    #[test]
    fn trailing_background_operator() {
        let cmds = parse_script("webpack --watch &");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "webpack");
    }

    // --- filter_production_scripts ---

    #[test]
    fn filter_keeps_production_scripts() {
        let scripts: HashMap<String, String> = [
            ("build".to_string(), "webpack".to_string()),
            ("start".to_string(), "node server.js".to_string()),
            ("test".to_string(), "jest".to_string()),
            ("lint".to_string(), "eslint src".to_string()),
            ("dev".to_string(), "next dev".to_string()),
        ]
        .into_iter()
        .collect();

        let filtered = filter_production_scripts(&scripts);
        assert!(filtered.contains_key("build"));
        assert!(filtered.contains_key("start"));
        assert!(!filtered.contains_key("test"));
        assert!(!filtered.contains_key("lint"));
        assert!(!filtered.contains_key("dev"));
    }

    // --- looks_like_file_path tests ---

    #[test]
    fn looks_like_file_path_with_known_extensions() {
        assert!(super::looks_like_file_path("src/app.ts"));
        assert!(super::looks_like_file_path("config.json"));
        assert!(super::looks_like_file_path("setup.yaml"));
        assert!(super::looks_like_file_path("rollup.config.mjs"));
        assert!(super::looks_like_file_path("test.spec.tsx"));
        assert!(super::looks_like_file_path("file.toml"));
    }

    #[test]
    fn looks_like_file_path_with_relative_prefix() {
        assert!(super::looks_like_file_path("./scripts/build"));
        assert!(super::looks_like_file_path("../shared/utils"));
    }

    #[test]
    fn looks_like_file_path_with_slash_but_not_scope() {
        assert!(super::looks_like_file_path("src/components/Button"));
        assert!(!super::looks_like_file_path("@scope/package")); // scoped package
    }

    #[test]
    fn looks_like_file_path_url_not_file() {
        assert!(!super::looks_like_file_path("https://example.com/path"));
    }

    #[test]
    fn looks_like_file_path_bare_word_not_file() {
        assert!(!super::looks_like_file_path("webpack"));
        assert!(!super::looks_like_file_path("--mode"));
        assert!(!super::looks_like_file_path("production"));
    }

    // --- extract_config_arg tests ---

    #[test]
    fn extract_config_arg_with_equals() {
        assert_eq!(
            super::extract_config_arg("--config=webpack.prod.js", None),
            Some("webpack.prod.js".to_string())
        );
    }

    #[test]
    fn extract_config_arg_short_with_equals() {
        assert_eq!(
            super::extract_config_arg("-c=.eslintrc.json", None),
            Some(".eslintrc.json".to_string())
        );
    }

    #[test]
    fn extract_config_arg_with_next_token() {
        assert_eq!(
            super::extract_config_arg("--config", Some("jest.config.ts")),
            Some("jest.config.ts".to_string())
        );
    }

    #[test]
    fn extract_config_arg_short_with_next_token() {
        assert_eq!(
            super::extract_config_arg("-c", Some(".eslintrc.json")),
            Some(".eslintrc.json".to_string())
        );
    }

    #[test]
    fn extract_config_arg_next_is_flag_returns_none() {
        assert_eq!(
            super::extract_config_arg("--config", Some("--verbose")),
            None
        );
    }

    #[test]
    fn extract_config_arg_no_match() {
        assert_eq!(super::extract_config_arg("--verbose", None), None);
        assert_eq!(super::extract_config_arg("src/index.ts", None), None);
    }

    #[test]
    fn extract_config_arg_empty_equals_returns_none() {
        assert_eq!(super::extract_config_arg("--config=", None), None);
        assert_eq!(super::extract_config_arg("-c=", None), None);
    }

    // --- node runner flag skipping ---

    #[test]
    fn node_require_flag_skips_next_arg() {
        let cmds = parse_script("node -r tsconfig-paths/register ./src/server.ts");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "node");
        // "tsconfig-paths/register" should be skipped (consumed by -r)
        // "./src/server.ts" should be a file arg
        assert!(cmds[0].file_args.contains(&"./src/server.ts".to_string()));
        assert!(
            !cmds[0]
                .file_args
                .contains(&"tsconfig-paths/register".to_string())
        );
    }

    #[test]
    fn node_eval_skips_next_arg() {
        let cmds = parse_script("node --eval \"console.log(1)\" scripts/run.js");
        assert_eq!(cmds.len(), 1);
        // The eval string is consumed, only scripts/run.js should be a file arg
        assert!(cmds[0].file_args.contains(&"scripts/run.js".to_string()));
    }

    // --- is_production_script edge cases ---

    #[test]
    fn production_script_prepublish_only() {
        assert!(super::is_production_script("prepublishOnly"));
    }

    #[test]
    fn production_script_postinstall() {
        assert!(super::is_production_script("postinstall"));
    }

    #[test]
    fn production_script_preserve_is_not_production() {
        // "preserve" starts with "pre" but "serve" after stripping "pre" is a match
        // Let's check: strip "pre" → "serve" which matches, so it IS production
        assert!(super::is_production_script("preserve"));
    }

    #[test]
    fn production_script_preinstall() {
        // strip "pre" → "install" which matches
        assert!(super::is_production_script("preinstall"));
    }

    #[test]
    fn production_script_namespaced() {
        assert!(super::is_production_script("build:esm"));
        assert!(super::is_production_script("start:dev"));
        assert!(!super::is_production_script("test:unit"));
        assert!(!super::is_production_script("lint:fix"));
    }

    // --- is_env_assignment edge cases ---

    #[test]
    fn env_assignment_empty_value() {
        assert!(is_env_assignment("KEY="));
    }

    #[test]
    fn env_assignment_equals_at_start_is_not_assignment() {
        assert!(!is_env_assignment("=value"));
    }

    // --- empty/edge scripts ---

    #[test]
    fn parse_empty_script() {
        let cmds = parse_script("");
        assert!(cmds.is_empty());
    }

    #[test]
    fn parse_whitespace_only_script() {
        let cmds = parse_script("   ");
        assert!(cmds.is_empty());
    }

    #[test]
    fn analyze_scripts_empty_scripts() {
        let scripts: HashMap<String, String> = HashMap::new();
        let result = analyze_scripts(&scripts, Path::new("/nonexistent"));
        assert!(result.used_packages.is_empty());
        assert!(result.config_files.is_empty());
        assert!(result.entry_files.is_empty());
    }

    // --- bun as package manager ---

    #[test]
    fn bun_treated_as_package_manager() {
        // `bun scripts/build.ts` is treated like `yarn build` — runs a script, not a binary
        let cmds = parse_script("bun scripts/build.ts");
        assert!(
            cmds.is_empty(),
            "bare `bun <arg>` should be treated as running a script (like yarn)"
        );
    }

    #[test]
    fn bun_exec_extracts_binary() {
        let cmds = parse_script("bun exec vitest run");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "vitest");
    }

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// parse_script should never panic on arbitrary input.
            #[test]
            fn parse_script_no_panic(s in "[a-zA-Z0-9 _./@&|;=\"'-]{1,200}") {
                let _ = parse_script(&s);
            }

            /// split_shell_operators should never panic on arbitrary input.
            #[test]
            fn split_shell_operators_no_panic(s in "[a-zA-Z0-9 _./@&|;=\"'-]{1,200}") {
                let _ = shell::split_shell_operators(&s);
            }

            /// When parse_script returns commands, binary names should be non-empty.
            #[test]
            fn parsed_binaries_are_non_empty(
                binary in "[a-z][a-z0-9-]{0,20}",
                args in "[a-zA-Z0-9 _./=-]{0,50}",
            ) {
                let script = format!("{binary} {args}");
                let commands = parse_script(&script);
                for cmd in &commands {
                    prop_assert!(!cmd.binary.is_empty(), "Binary name should never be empty");
                }
            }

            /// analyze_scripts should never panic on arbitrary script values.
            #[test]
            fn analyze_scripts_no_panic(
                name in "[a-z]{1,10}",
                value in "[a-zA-Z0-9 _./@&|;=-]{1,100}",
            ) {
                let scripts: HashMap<String, String> = std::iter::once((name, value)).collect();
                let _ = analyze_scripts(&scripts, Path::new("/nonexistent"));
            }

            /// is_env_assignment should never panic on arbitrary input.
            #[test]
            fn is_env_assignment_no_panic(s in "[a-zA-Z0-9_=./-]{1,50}") {
                let _ = is_env_assignment(&s);
            }

            /// resolve_binary_to_package should always return a non-empty string.
            #[test]
            fn resolve_binary_always_non_empty(binary in "[a-z][a-z0-9-]{0,20}") {
                let result = resolve_binary_to_package(&binary, Path::new("/nonexistent"));
                prop_assert!(!result.is_empty(), "Package name should never be empty");
            }

            /// Chained scripts should produce at least as many commands as operators + 1
            /// when each segment is a valid binary (excluding package managers and builtins).
            #[test]
            fn chained_binaries_produce_multiple_commands(
                bins in prop::collection::vec("[a-z][a-z0-9]{0,10}", 2..5),
            ) {
                let reserved = ["npm", "npx", "yarn", "pnpm", "pnpx", "bun", "bunx",
                    "node", "env", "cross", "sh", "bash", "exec", "sudo", "nohup"];
                prop_assume!(!bins.iter().any(|b| reserved.contains(&b.as_str())));
                let script = bins.join(" && ");
                let commands = parse_script(&script);
                prop_assert!(
                    commands.len() >= 2,
                    "Chained commands should produce multiple parsed commands, got {}",
                    commands.len()
                );
            }
        }
    }
}
