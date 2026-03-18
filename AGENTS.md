---
name: fallow
description: Rust-native dead code analyzer for JavaScript/TypeScript projects. 10-100x faster alternative to knip.
agent-usage: This CLI is frequently invoked by AI coding agents (Claude Code, Cursor, Copilot, etc.) for codebase hygiene tasks.
---

# Fallow CLI -- Agent Integration Guide

Fallow detects unused files, exports, dependencies, types, enum members, class members, unresolved imports, unlisted dependencies, and duplicate exports in JS/TS projects.

## Rules for AI agents

1. **Always use `--format json`** for machine-readable output. Never parse human-formatted output.
2. **Always use `--quiet`** to suppress progress bars and timing info on stderr.
3. **Always use `--dry-run` before `fix`** mutations. Review the dry-run output, then run `fix` only if the changes are correct.
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
- Issue type filters: `--unused-files`, `--unused-exports`, `--unused-deps`, `--unused-types`, `--unused-enum-members`, `--unused-class-members`, `--unresolved-imports`, `--unlisted-deps`, `--duplicate-exports`

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
- `--baseline <path>` / `--save-baseline <path>` -- incremental CI adoption

### `fix`

Auto-remove unused exports and dependencies.

```bash
fallow fix --dry-run --format json --quiet   # preview first
fallow fix --format json --quiet             # apply changes
```

**Flags:**
- `--dry-run` -- show what would be removed without modifying files
- `--format json` -- machine-readable output of changes

### `list`

List discovered files, entry points, or detected frameworks.

```bash
fallow list --files --format json --quiet
fallow list --entry-points --format json --quiet
fallow list --frameworks --format json --quiet
```

### `init`

Create a config file in the project root. Defaults to `fallow.jsonc` (JSONC with comments and `$schema` for IDE autocomplete). Use `--toml` for TOML format.

```bash
fallow init          # creates fallow.jsonc
fallow init --toml   # creates fallow.toml
```

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
fallow fix --format json --quiet             # 2. apply
fallow check --format json --quiet           # 3. verify
```

### Discover project structure

```bash
fallow list --entry-points --format json --quiet
fallow list --frameworks --format json --quiet
```

### Introspect CLI capabilities at runtime

```bash
fallow schema
```

## Configuration

Fallow reads config from the project root in priority order: `fallow.jsonc` > `fallow.json` > `fallow.toml` > `.fallow.toml`. Run `fallow init` to generate one. Framework presets (Next.js, Vite, Jest, Storybook, etc.) are auto-detected -- no configuration required for most projects.

### Rules (per-issue-type severity)

```jsonc
// fallow.jsonc
{
  "$schema": "https://raw.githubusercontent.com/fallow-rs/fallow/main/schema.json",
  "rules": {
    "unused_files": "error",       // fail CI (exit 1)
    "unused_exports": "warn",      // report but don't fail
    "unused_types": "off"          // ignore entirely
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
```

Issue type tokens: `unused-file`, `unused-export`, `unused-type`, `unused-dependency`, `unused-dev-dependency`, `unused-enum-member`, `unused-class-member`, `unresolved-import`, `unlisted-dependency`, `duplicate-export`.

Unknown tokens are silently ignored (the comment has no effect). When an agent adds a suppression comment, always use the exact tokens listed above.

## Invariants

- Fallow uses syntactic analysis only (no TypeScript compiler). It partially resolves dynamic imports with static prefixes (template literals, string concatenation, import.meta.glob, require.context) but fully dynamic paths like `import(variable)` are not resolved.
- Re-export chains through barrel files are resolved. An export re-exported from `index.ts` is not falsely flagged if consumed downstream.
- Workspace support (npm/yarn/pnpm) is automatic when `workspaces` is defined in the root `package.json` or `pnpm-workspace.yaml` exists.
- Inline suppression comments are parsed during extraction and cached alongside module data. They are applied during analysis, before results reach the reporting layer.
- Analysis is deterministic: same input always produces the same output.
