---
name: fallow
description: Rust-native codebase analyzer for JavaScript/TypeScript projects. Finds unused code, circular dependencies, code duplication, and complexity hotspots. 3-36x faster than knip v5.
agent-usage: This CLI is frequently invoked by AI coding agents (Claude Code, Cursor, Copilot, etc.) for codebase hygiene tasks.
---

# Fallow CLI -- Agent Integration Guide

Fallow is a codebase analyzer for JS/TS projects. It detects unused files, exports, dependencies, types, enum members, class members, unresolved imports, unlisted dependencies, duplicate exports, circular dependencies, code duplication, and complexity hotspots.

## Rules for AI agents

1. **Always use `--format json`** for machine-readable output. Never parse human-formatted output. Alternatively, set `FALLOW_FORMAT=json` as an environment variable.
2. **Always use `--quiet`** to suppress progress bars and timing info on stderr. Alternatively, set `FALLOW_QUIET=1` as an environment variable.
3. **Always use `--dry-run` before `fix`** mutations. Review the dry-run output, then run `fix --yes` to apply. The `--yes` flag (alias: `--force`) is **required** in non-TTY environments (CI, piped input, agent subprocesses).
4. **Use issue type filter flags** (`--unused-files`, `--unused-exports`, etc.) to limit response scope. This keeps output small and avoids exceeding context windows.
5. **All paths in output are relative** to the project root. Do not join them with an absolute prefix unless you know the working directory.
6. **Do not run `watch`** in agent workflows. It is interactive and never exits.

## Exit codes

| Code | Meaning |
|------|---------|
| 0    | Success (no error-severity issues found) |
| 1    | Error-severity issues found (per `[rules]` config, or `--fail-on-issues` promotes `warn` → `error`) |
| 2    | Error (invalid config, parse failure, etc.) |

**Note:** With the rules system, exit code 1 is triggered by any issue type configured as `"error"` in `[rules]`. Without a `[rules]` section, all issue types default to `"error"` severity.

**JSON error output:** When `--format json` is active and an error occurs (exit code 2), the error is emitted as structured JSON on **stdout** instead of plain text on stderr:

```json
{"error": true, "message": "invalid config: ...", "exit_code": 2}
```

This allows agents to parse errors the same way they parse normal output.

## Environment variables

| Variable | Description |
|----------|-------------|
| `FALLOW_FORMAT` | Default output format (`json`, `human`, `sarif`, `compact`). CLI `--format` flag overrides this. |
| `FALLOW_QUIET` | Set to `1` or `true` to suppress progress output. CLI `--quiet` flag overrides this. |
| `FALLOW_BIN` | Path to fallow binary (used by the `fallow-mcp` server). |

Set `FALLOW_FORMAT=json` and `FALLOW_QUIET=1` in your agent environment to avoid passing `--format json --quiet` on every invocation.

## Commands

### `check` (default)

Run dead code analysis.

```bash
fallow check --format json --quiet
fallow check --format json --quiet --unused-exports
fallow check --format json --quiet --fail-on-issues
fallow check --format json --quiet --changed-since main
```

**Flags:**
- `--format human|json|sarif|compact` -- output format (default: human)
- `--quiet` -- suppress progress and timing on stderr
- `--fail-on-issues` -- exit 1 if any issues are found
- `--changed-since <ref>` -- only analyze files changed since a git ref
- `--baseline <path>` -- compare against a saved baseline
- `--save-baseline <path>` -- save current results as a baseline
- `--production` -- exclude test/story/dev files, only start/build scripts, report type-only dependencies
- `--workspace <name>` -- scope output to a single workspace package (monorepo support)
- `--ci` -- CI mode: equivalent to `--format sarif --fail-on-issues --quiet`
- `--sarif-file <PATH>` -- write SARIF output to a file (in addition to the primary `--format` output)
- `--include-dupes` -- cross-reference dead code with duplication findings
- `--trace <FILE:EXPORT>` -- trace export usage chain
- `--trace-file <PATH>` -- show all edges for a file
- `--trace-dependency <PACKAGE>` -- show where a dependency is used
- Issue type filters: `--unused-files`, `--unused-exports`, `--unused-deps`, `--unused-types`, `--unused-enum-members`, `--unused-class-members`, `--unresolved-imports`, `--unlisted-deps`, `--duplicate-exports`, `--circular-deps`

### `dupes`

Find code duplication / clones across the project.

```bash
fallow dupes --format json --quiet
fallow dupes --format json --quiet --mode semantic
fallow dupes --format json --quiet --threshold 5
```

**Flags:**
- `--mode strict|mild|weak|semantic` -- detection mode (default: mild)
- `--min-tokens <N>` -- minimum token count for a clone (default: 50)
- `--min-lines <N>` -- minimum line count for a clone (default: 5)
- `--threshold <PCT>` -- fail if duplication exceeds this percentage (0 = no limit)
- `--skip-local` -- only report cross-directory duplicates
- `--cross-language` -- strip TypeScript type annotations for `.ts` ↔ `.js` matching
- `--trace <FILE:LINE>` -- trace all clones at a specific source location
- `--baseline <path>` / `--save-baseline <path>` -- incremental CI adoption

### `fix`

Auto-remove unused exports and dependencies.

```bash
fallow fix --dry-run --format json --quiet   # preview first
fallow fix --yes --format json --quiet       # apply changes (--yes required in non-TTY)
```

**Flags:**
- `--dry-run` -- show what would be removed without modifying files
- `--yes` (alias: `--force`) -- skip confirmation prompt; **required** in non-TTY environments (CI, piped input, agent subprocesses). Without `--yes` in a non-TTY context, the command exits with code 2 and an error message.
- `--format json` -- machine-readable output of changes

### `list`

List discovered files, entry points, or detected plugins.

```bash
fallow list --files --format json --quiet
fallow list --entry-points --format json --quiet
fallow list --plugins --format json --quiet
```

**Flags:**
- `--files` -- list all discovered source files
- `--entry-points` -- list all detected entry points
- `--plugins` -- list all active framework plugins

### `init`

Create a config file in the project root. Defaults to `.fallowrc.json` (JSON with JSONC comment support and `$schema` for IDE autocomplete). Use `--toml` for TOML format.

```bash
fallow init          # creates .fallowrc.json
fallow init --toml   # creates fallow.toml
```

### `migrate`

Migrate configuration from knip and/or jscpd to fallow. Auto-detects config files in the project root (knip.json, knip.jsonc, .knip.json, .knip.jsonc, .jscpd.json, and package.json embedded configs).

```bash
fallow migrate              # auto-detect and write .fallowrc.json
fallow migrate --toml       # output as TOML
fallow migrate --dry-run    # preview without writing
fallow migrate --from PATH  # specify source config file
```

Maps knip rules/exclude/include to fallow's rules system, knip ignore/ignoreDependencies to fallow equivalents, and jscpd settings (minTokens, minLines, threshold, mode, skipLocal, ignore) to fallow's duplicates config. Warns about unmappable fields with suggestions.

### `schema`

Dump the full CLI interface definition as machine-readable JSON. Use this for runtime introspection of available commands, flags, and options.

```bash
fallow schema
```

### `config-schema`

Print the JSON Schema for fallow configuration files. Pipe to a file for IDE integration.

```bash
fallow config-schema > schema.json
```

### `plugin-schema`

Print the JSON Schema for external plugin files. Pipe to a file for IDE validation of custom plugins.

```bash
fallow plugin-schema > plugin-schema.json
```

## Output structure

JSON output goes to **stdout**. Progress bars and timing go to **stderr** (suppressed with `--quiet`).

Compact format (`--format compact`) is grep-friendly: one issue per line as `type:path:line:name`.

JSON output includes `version`, `elapsed_ms`, and `total_issues` metadata alongside the issue arrays.

## Common agent workflows

### Audit a project for all dead code

```bash
fallow check --format json --quiet
```

### Check only unused exports (smaller output)

```bash
fallow check --format json --quiet --unused-exports
```

### Check if a PR introduces dead code

```bash
fallow check --format json --quiet --changed-since main --fail-on-issues
```

### Production-only analysis (skip test/dev files)

```bash
fallow check --format json --quiet --production
```

### Analyze a single workspace package (monorepo)

```bash
fallow check --format json --quiet --workspace my-package
```

### Find code duplication

```bash
fallow dupes --format json --quiet
fallow dupes --format json --quiet --mode semantic --threshold 5
```

### Safe auto-fix cycle

```bash
fallow fix --dry-run --format json --quiet   # 1. preview
# agent reviews output
fallow fix --yes --format json --quiet       # 2. apply (--yes required for non-TTY)
fallow check --format json --quiet           # 3. verify
```

### Discover project structure

```bash
fallow list --entry-points --format json --quiet
fallow list --plugins --format json --quiet
```

### Introspect CLI capabilities at runtime

```bash
fallow schema
```

## Configuration

Fallow reads config from the project root in priority order: `.fallowrc.json` > `fallow.toml` > `.fallow.toml`. Run `fallow init` to generate one. Framework presets (Next.js, Vite, Jest, Storybook, etc.) are auto-detected -- no configuration required for most projects.

### Rules (per-issue-type severity)

```jsonc
// .fallowrc.json
{
  "$schema": "https://raw.githubusercontent.com/fallow-rs/fallow/main/schema.json",
  "rules": {
    "unused-files": "error",       // fail CI (exit 1)
    "unused-exports": "warn",      // report but don't fail
    "unused-types": "off"          // ignore entirely
  }
}
```

- `error` (default) -- report and exit 1
- `warn` -- report but exit 0
- `off` -- skip detection entirely
- `--fail-on-issues` promotes all `warn` to `error`

### Inline suppression

Source-level suppression for false positives:

```
// fallow-ignore-next-line
// fallow-ignore-next-line unused-export
// fallow-ignore-file
// fallow-ignore-file unused-export
// fallow-ignore-file code-duplication
// fallow-ignore-next-line code-duplication
```

Issue type tokens: `unused-file`, `unused-export`, `unused-type`, `unused-dependency`, `unused-dev-dependency`, `unused-enum-member`, `unused-class-member`, `unresolved-import`, `unlisted-dependency`, `duplicate-export`, `circular-dependency`, `code-duplication`.

Unknown tokens are silently ignored (the comment has no effect). When an agent adds a suppression comment, always use the exact tokens listed above.

## Agent Skills

For agents that support the [Agent Skills](https://agentskills.io) specification, install structured fallow skills with workflows, gotchas, and patterns:

```bash
# Claude Code
/install fallow-rs/fallow-skills

# Other agents — clone into your agent's skills directory
git clone https://github.com/fallow-rs/fallow-skills.git
```

See the [fallow-skills repository](https://github.com/fallow-rs/fallow-skills) for installation instructions for all supported agents.

## Invariants

- Fallow uses syntactic analysis only (no TypeScript compiler). It partially resolves dynamic imports with static prefixes (template literals, string concatenation, import.meta.glob, require.context) but fully dynamic paths like `import(variable)` are not resolved.
- Re-export chains through barrel files are resolved. An export re-exported from `index.ts` is not falsely flagged if consumed downstream.
- Workspace support (npm/yarn/pnpm) is automatic when `workspaces` is defined in the root `package.json` or `pnpm-workspace.yaml` exists.
- Inline suppression comments are parsed during extraction and cached alongside module data. They are applied during analysis, before results reach the reporting layer.
- Analysis is deterministic: same input always produces the same output.
