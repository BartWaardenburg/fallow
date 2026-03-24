//! Shell tokenization: splitting on operators, skipping env wrappers and package managers.

use super::ENV_WRAPPERS;

/// Split a script string on shell operators (`&&`, `||`, `;`, `|`, `&`).
/// Respects single and double quotes.
pub fn split_shell_operators(script: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let bytes = script.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while i < len {
        let b = bytes[i];

        // Toggle quote state
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

        // Inside quotes — skip everything
        if in_single_quote || in_double_quote {
            i += 1;
            continue;
        }

        // Try to match a shell operator and split on it
        if let Some(op_len) = shell_operator_len(bytes, i) {
            segments.push(&script[start..i]);
            i += op_len;
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

/// Return the byte length of a shell operator at position `i`, or `None`.
///
/// Checks two-char operators (`&&`, `||`) before single-char ones (`&`, `|`, `;`)
/// to avoid splitting `&&` as two `&` operators.
fn shell_operator_len(bytes: &[u8], i: usize) -> Option<usize> {
    let b = bytes[i];
    let next = bytes.get(i + 1).copied();

    // Two-character operators: && ||
    if matches!((b, next), (b'&', Some(b'&')) | (b'|', Some(b'|'))) {
        return Some(2);
    }

    // Single-character operators: ; | &
    if b == b';' {
        return Some(1);
    }
    if b == b'|' && next != Some(b'|') {
        return Some(1);
    }
    if b == b'&' && next != Some(b'&') {
        return Some(1);
    }

    None
}

/// Skip env var assignments (`KEY=value`) and env wrapper commands (`cross-env`, `dotenv`, `env`)
/// at the start of a token list. Returns the index of the first real command token, or `None`
/// if all tokens were consumed.
pub fn skip_initial_wrappers(tokens: &[&str], mut idx: usize) -> Option<usize> {
    // Skip env var assignments (KEY=value pairs)
    while idx < tokens.len() && super::is_env_assignment(tokens[idx]) {
        idx += 1;
    }
    if idx >= tokens.len() {
        return None;
    }

    // Skip env wrapper commands (cross-env, dotenv, env)
    while idx < tokens.len() && ENV_WRAPPERS.contains(&tokens[idx]) {
        idx += 1;
        // Skip env var assignments after the wrapper
        while idx < tokens.len() && super::is_env_assignment(tokens[idx]) {
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

    Some(idx)
}

/// Advance past package manager prefixes (`npx`, `pnpx`, `bunx`, `yarn exec`, `pnpm dlx`, etc.).
/// Returns the index of the actual binary token, or `None` if the command delegates to a named
/// script (e.g., `npm run build`, `yarn build`).
pub fn advance_past_package_manager(tokens: &[&str], mut idx: usize) -> Option<usize> {
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

    Some(idx)
}
