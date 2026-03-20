//! Lightweight shell command parser for package.json scripts.
//!
//! Extracts:
//! - **Binary names** → mapped to npm package names for dependency usage detection
//! - **`--config` arguments** → file paths for entry point discovery
//! - **Positional file arguments** → file paths for entry point discovery
//!
//! Handles env var prefixes (`cross-env`, `dotenv`, `KEY=value`), package manager
//! runners (`npx`, `pnpm exec`, `yarn dlx`), and Node.js runners (`node`, `tsx`,
//! `ts-node`). Shell operators (`&&`, `||`, `;`, `|`) are split correctly.

#[allow(clippy::disallowed_types)]
use std::collections::HashMap;
use std::path::Path;

use rustc_hash::FxHashSet;

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
#[derive(Debug, PartialEq)]
pub struct ScriptCommand {
    /// The binary/command name (e.g., "webpack", "eslint", "tsc").
    pub binary: String,
    /// Config file arguments (from `--config`, `-c`).
    pub config_args: Vec<String>,
    /// File path arguments (positional args that look like file paths).
    pub file_args: Vec<String>,
}

/// Known binary-name → package-name mappings where they diverge.
static BINARY_TO_PACKAGE: &[(&str, &str)] = &[
    ("tsc", "typescript"),
    ("tsserver", "typescript"),
    ("ng", "@angular/cli"),
    ("nuxi", "nuxt"),
    ("run-s", "npm-run-all"),
    ("run-p", "npm-run-all"),
    ("run-s2", "npm-run-all2"),
    ("run-p2", "npm-run-all2"),
    ("sb", "storybook"),
    ("biome", "@biomejs/biome"),
    ("oxlint", "oxlint"),
];

/// Environment variable wrapper commands to strip before the actual binary.
const ENV_WRAPPERS: &[&str] = &["cross-env", "dotenv", "env"];

/// Node.js runners whose first non-flag argument is a file path, not a binary name.
const NODE_RUNNERS: &[&str] = &["node", "ts-node", "tsx", "babel-node", "bun"];

/// Filter scripts to only production-relevant ones (start, build, and their pre/post hooks).
///
/// In production mode, dev/test/lint scripts are excluded since they only affect
/// devDependency usage, not the production dependency graph.
#[allow(clippy::implicit_hasher, clippy::disallowed_types)]
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
#[allow(clippy::implicit_hasher, clippy::disallowed_types)]
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
/// Splits on shell operators (`&&`, `||`, `;`, `|`) and parses each segment.
pub fn parse_script(script: &str) -> Vec<ScriptCommand> {
    let mut commands = Vec::new();

    for segment in split_shell_operators(script) {
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

/// Split a script string on shell operators (`&&`, `||`, `;`, `|`).
/// Respects single and double quotes.
fn split_shell_operators(script: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let bytes = script.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while i < len {
        let b = bytes[i];

        if b == b'\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            i += 1;
            continue;
        }
        if b == b'"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            i += 1;
            continue;
        }
        if in_single_quote || in_double_quote {
            i += 1;
            continue;
        }

        // && or ||
        if i + 1 < len
            && ((b == b'&' && bytes[i + 1] == b'&') || (b == b'|' && bytes[i + 1] == b'|'))
        {
            segments.push(&script[start..i]);
            i += 2;
            start = i;
            continue;
        }

        // ; or single | (pipe — only first command provides the binary)
        if b == b';' || (b == b'|' && (i + 1 >= len || bytes[i + 1] != b'|')) {
            segments.push(&script[start..i]);
            i += 1;
            start = i;
            continue;
        }

        i += 1;
    }

    if start < len {
        segments.push(&script[start..]);
    }

    segments
}

/// Parse a single command segment (after splitting on shell operators).
#[allow(clippy::cognitive_complexity)] // Shell command parsing naturally has many branches
fn parse_command_segment(segment: &str) -> Option<ScriptCommand> {
    let tokens: Vec<&str> = segment.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    let mut idx = 0;

    // Skip env var assignments (KEY=value pairs)
    while idx < tokens.len() && is_env_assignment(tokens[idx]) {
        idx += 1;
    }
    if idx >= tokens.len() {
        return None;
    }

    // Skip env wrapper commands (cross-env, dotenv, env)
    while idx < tokens.len() && ENV_WRAPPERS.contains(&tokens[idx]) {
        idx += 1;
        // Skip env var assignments after the wrapper
        while idx < tokens.len() && is_env_assignment(tokens[idx]) {
            idx += 1;
        }
        // dotenv uses -- as separator
        if idx < tokens.len() && tokens[idx] == "--" {
            idx += 1;
        }
    }
    if idx >= tokens.len() {
        return None;
    }

    // Handle package manager prefixes
    let token = tokens[idx];
    if matches!(token, "npx" | "pnpx" | "bunx") {
        idx += 1;
        // Skip npx flags (--yes, --no-install, -p, --package)
        while idx < tokens.len() && tokens[idx].starts_with('-') {
            let flag = tokens[idx];
            idx += 1;
            // --package <name> consumes the next argument
            if matches!(flag, "--package" | "-p") && idx < tokens.len() {
                idx += 1;
            }
        }
    } else if matches!(token, "yarn" | "pnpm" | "npm" | "bun") {
        if idx + 1 < tokens.len() {
            let subcmd = tokens[idx + 1];
            if subcmd == "exec" || subcmd == "dlx" {
                idx += 2;
            } else if matches!(subcmd, "run" | "run-script") {
                // Delegates to a named script, not a binary invocation
                return None;
            } else {
                // Bare `yarn <name>` runs a script — skip
                return None;
            }
        } else {
            return None;
        }
    }
    if idx >= tokens.len() {
        return None;
    }

    let binary = tokens[idx].to_string();
    idx += 1;

    // If the binary is a node runner, extract file paths from arguments
    if NODE_RUNNERS.contains(&binary.as_str()) {
        let mut file_args = Vec::new();
        let mut config_args = Vec::new();

        while idx < tokens.len() {
            let token = tokens[idx];

            // Skip flags that consume the next argument
            if matches!(
                token,
                "-e" | "--eval" | "-p" | "--print" | "-r" | "--require"
            ) {
                idx += 2;
                continue;
            }

            if token.starts_with('-') {
                if let Some(config) = extract_config_arg(token, tokens.get(idx + 1).copied()) {
                    config_args.push(config);
                    if !token.contains('=') {
                        idx += 1;
                    }
                }
                idx += 1;
                continue;
            }

            if looks_like_file_path(token) {
                file_args.push(token.to_string());
            }
            idx += 1;
        }

        return Some(ScriptCommand {
            binary,
            config_args,
            file_args,
        });
    }

    // For other binaries, extract config args and file args
    let mut config_args = Vec::new();
    let mut file_args = Vec::new();

    while idx < tokens.len() {
        let token = tokens[idx];

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

/// Resolve a binary name to its npm package name.
///
/// Strategy:
/// 1. Check known binary→package divergence map
/// 2. Read `node_modules/.bin/<binary>` symlink target
/// 3. Fall back: binary name = package name
pub fn resolve_binary_to_package(binary: &str, root: &Path) -> String {
    // 1. Known divergences
    if let Some(&(_, pkg)) = BINARY_TO_PACKAGE.iter().find(|(bin, _)| *bin == binary) {
        return pkg.to_string();
    }

    // 2. Try reading the symlink in node_modules/.bin/
    let bin_link = root.join("node_modules/.bin").join(binary);
    if let Ok(target) = std::fs::read_link(&bin_link)
        && let Some(pkg_name) = extract_package_from_bin_path(&target)
    {
        return pkg_name;
    }

    // 3. Fallback: binary name = package name
    binary.to_string()
}

/// Extract a package name from a `node_modules/.bin` symlink target path.
///
/// Typical symlink targets:
/// - `../webpack/bin/webpack.js` → `webpack`
/// - `../@babel/cli/bin/babel.js` → `@babel/cli`
fn extract_package_from_bin_path(target: &std::path::Path) -> Option<String> {
    let target_str = target.to_string_lossy();
    let parts: Vec<&str> = target_str.split('/').collect();

    for (i, part) in parts.iter().enumerate() {
        if *part == ".." {
            continue;
        }
        // Scoped package: @scope/name
        if part.starts_with('@') && i + 1 < parts.len() {
            return Some(format!("{}/{}", part, parts[i + 1]));
        }
        // Regular package
        return Some(part.to_string());
    }

    None
}

#[cfg(test)]
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

    // --- binary → package mapping ---

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
            extract_package_from_bin_path(path),
            Some("webpack".to_string())
        );
    }

    #[test]
    fn bin_path_scoped_package() {
        let path = std::path::Path::new("../@babel/cli/bin/babel.js");
        assert_eq!(
            extract_package_from_bin_path(path),
            Some("@babel/cli".to_string())
        );
    }

    // --- builtin commands ---

    #[test]
    fn builtin_commands_not_tracked() {
        let scripts: HashMap<String, String> =
            [("postinstall".to_string(), "echo done".to_string())]
                .into_iter()
                .collect();
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
        let scripts: HashMap<String, String> = [(
            "build".to_string(),
            "webpack --config webpack.prod.js".to_string(),
        )]
        .into_iter()
        .collect();
        let result = analyze_scripts(&scripts, Path::new("/nonexistent"));
        assert!(result.config_files.contains(&"webpack.prod.js".to_string()));
    }

    #[test]
    fn analyze_extracts_entry_files() {
        let scripts: HashMap<String, String> =
            [("seed".to_string(), "ts-node scripts/seed.ts".to_string())]
                .into_iter()
                .collect();
        let result = analyze_scripts(&scripts, Path::new("/nonexistent"));
        assert!(result.entry_files.contains(&"scripts/seed.ts".to_string()));
        // ts-node should be tracked as a used package
        assert!(result.used_packages.contains("ts-node"));
    }

    #[test]
    fn analyze_cross_env_with_config() {
        let scripts: HashMap<String, String> = [(
            "build".to_string(),
            "cross-env NODE_ENV=production webpack --config webpack.prod.js".to_string(),
        )]
        .into_iter()
        .collect();
        let result = analyze_scripts(&scripts, Path::new("/nonexistent"));
        assert!(result.used_packages.contains("cross-env"));
        assert!(result.used_packages.contains("webpack"));
        assert!(result.config_files.contains(&"webpack.prod.js".to_string()));
    }

    #[test]
    fn analyze_complex_script() {
        let scripts: HashMap<String, String> = [(
            "ci".to_string(),
            "cross-env CI=true npm run build && jest --config jest.ci.js --coverage".to_string(),
        )]
        .into_iter()
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
        let segments = split_shell_operators("echo 'a && b' && jest");
        assert_eq!(segments.len(), 2);
        assert!(segments[1].trim() == "jest");
    }

    #[test]
    fn split_double_quotes() {
        let segments = split_shell_operators("echo \"a || b\" || jest");
        assert_eq!(segments.len(), 2);
        assert!(segments[1].trim() == "jest");
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
                let _ = split_shell_operators(&s);
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
                let scripts: HashMap<String, String> =
                    [(name, value)].into_iter().collect();
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
