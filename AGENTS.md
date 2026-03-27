---
name: fallow
description: Rust-native codebase analyzer for TypeScript/JavaScript projects. Finds unused code, circular dependencies, code duplication, and complexity hotspots. 5-41x faster than knip v5.
agent-usage: This CLI is frequently invoked by AI coding agents (Claude Code, Cursor, Copilot, etc.) for codebase hygiene tasks.
---

# Fallow CLI -- Agent Integration Guide

Fallow is a codebase analyzer for JS/TS projects. It detects unused files, exports, dependencies, types, enum members, class members, unresolved imports, unlisted dependencies, duplicate exports, circular dependencies, code duplication, and function complexity.

## Rules for AI agents

1. **Always use `--format json`** for machine-readable output. Never parse human-formatted output. Alternatively, set `FALLOW_FORMAT=json` as an environment variable.
2. **Always use `--quiet`** to suppress progress bars and timing info on stderr. Alternatively, set `FALLOW_QUIET=1` as an environment variable.
3. **Always use `--dry-run` before `fix`** mutations. Review the dry-run output, then run `fix --yes` to apply. The `--yes` flag (alias: `--force`) is **required** in non-TTY environments (CI, piped input, agent subprocesses).
4. **Use issue type filter flags** (`--unused-files`, `--unused-exports`, etc.) to limit response scope. This keeps output small and avoids exceeding context windows.
5. **All paths in output are relative** to the project root. Do not join them with an absolute prefix unless you know the working directory.
6. **Use `--explain`** to include a `_meta` object in JSON output with metric definitions, value ranges, and interpretation hints. The MCP server enables this automatically. This helps you understand what values like `complexity_density: 0.12` or `trend: accelerating` mean without consulting external docs.
7. **Do not run `watch`** in agent workflows. It is interactive and never exits.

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
| `FALLOW_FORMAT` | Default output format (`json`, `human`, `sarif`, `compact`, `markdown`, `codeclimate`). CLI `--format` flag overrides this. |
| `FALLOW_QUIET` | Set to `1` or `true` to suppress progress output. CLI `--quiet` flag overrides this. |
| `FALLOW_BIN` | Path to fallow binary (used by the `fallow-mcp` server). |

Set `FALLOW_FORMAT=json` and `FALLOW_QUIET=1` in your agent environment to avoid passing `--format json --quiet` on every invocation.

## Global flags

These flags work with any subcommand:

| Flag | Description |
|------|-------------|
| `--root <PATH>` / `-r` | Project root directory (default: cwd) |
| `--config <PATH>` / `-c` | Path to config file (.fallowrc.json / fallow.toml) |
| `--format <FMT>` / `-f` | Output format: human, json, sarif, compact, markdown, codeclimate |
| `--quiet` / `-q` | Suppress progress and timing on stderr |
| `--production` | Exclude test/story/dev files |
| `--workspace <NAME>` / `-w` | Scope to a workspace package |
| `--changed-since <REF>` | Only analyze files changed since a git ref |
| `--baseline <PATH>` | Compare against a saved baseline (report only new issues) |
| `--save-baseline <PATH>` | Save current results as a baseline file |
| `--no-cache` | Disable incremental parse cache (force full re-parse) |
| `--threads <N>` | Number of parser threads (default: available CPU cores) |
| `--explain` | Include `_meta` with metric definitions in JSON output |
| `--ci` | CI mode: `--format sarif --fail-on-issues --quiet` |
| `--fail-on-issues` | Exit 1 if any issues found |
| `--fail-on-regression` | Fail if issue count increased beyond tolerance vs regression baseline |
| `--tolerance <N>` | Allowed increase: `"2%"` (percentage) or `"5"` (absolute). Default: `"0"` |
| `--regression-baseline <PATH>` | Path to regression baseline file (overrides config-embedded baseline) |
| `--save-regression-baseline [PATH]` | Save current counts as regression baseline. No path = write into config file |
| `--sarif-file <PATH>` | Write SARIF alongside primary output |
| `--performance` | Show pipeline timing breakdown |

## Commands

### Bare `fallow` (no subcommand)

Runs all analyses: check + dupes + health. Use `--only`/`--skip` to select.

```bash
fallow --format json --quiet              # all three analyses
fallow --only dead-code --format json --quiet # just dead code
fallow --skip health --format json        # check + dupes only
```

**Flags:**
- `--only <dead-code,dupes,health>` -- run only these analyses (comma-separated)
- `--skip <dead-code,dupes,health>` -- skip these analyses (comma-separated)
- `--ci` -- CI mode: sarif + quiet + fail-on-issues
- `--fail-on-issues` -- exit 1 if any issues are found

### `dead-code`

Run dead code analysis. Legacy alias: `fallow check`.

```bash
fallow dead-code --format json --quiet
fallow dead-code --format json --quiet --unused-exports
fallow dead-code --format json --quiet --fail-on-issues
fallow dead-code --format json --quiet --changed-since main
```

**Flags:**
- `--format human|json|sarif|compact|markdown` -- output format (default: human)
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
fallow dupes --format json --quiet --changed-since main
```

**Flags:**
- `--mode strict|mild|weak|semantic` -- detection mode (default: mild)
- `--min-tokens <N>` -- minimum token count for a clone (default: 50)
- `--min-lines <N>` -- minimum line count for a clone (default: 5)
- `--threshold <PCT>` -- fail if duplication exceeds this percentage (0 = no limit)
- `--skip-local` -- only report cross-directory duplicates
- `--cross-language` -- strip TypeScript type annotations for `.ts` ↔ `.js` matching
- `--changed-since <ref>` -- only report duplication in files changed since a git ref
- `--trace <FILE:LINE>` -- trace all clones at a specific source location
- `--top <N>` -- show only the N largest clone groups (sorted by line count descending)
- `--baseline <path>` / `--save-baseline <path>` -- incremental CI adoption

### `health`

Analyze function complexity (cyclomatic and cognitive), per-file health scores, and hotspots.

```bash
fallow health --format json --quiet
fallow health --format json --quiet --max-cyclomatic 15
fallow health --format json --quiet --top 10 --sort cognitive
fallow health --format json --quiet --file-scores
fallow health --format json --quiet --hotspots
fallow health --format json --quiet --hotspots --since 3m --min-commits 5
fallow health --format json --quiet --targets
```

**Flags:**
- `--max-cyclomatic <N>` -- cyclomatic complexity threshold (default: 20)
- `--max-cognitive <N>` -- cognitive complexity threshold (default: 15)
- `--top <N>` -- only show the top N most complex functions (and file scores/hotspots/targets)
- `--sort cyclomatic|cognitive|lines` -- sort order for results
- `--complexity` -- show only complexity findings section (functions exceeding thresholds)
- `--file-scores` -- compute per-file maintainability index (fan-in, fan-out, dead code ratio, complexity density). Runs the full analysis pipeline.
- `--hotspots` -- identify files that are both complex and frequently changing (combines git churn with complexity). Requires a git repository.
- `--targets` -- ranked refactoring recommendations based on complexity, coupling, churn, and dead code signals. Sorted by efficiency (priority/effort) to surface quick wins. Categories: churn+complexity, circular dep, high impact, dead code, complexity, coupling.
- `--since <DURATION>` -- git history window for hotspot analysis (default: 6m). Accepts durations (6m, 90d, 1y, 2w) or ISO dates (2025-06-01).
- `--min-commits <N>` -- minimum commits for a file to appear in hotspot ranking (default: 3)
- `--save-snapshot [PATH]` -- save vital signs snapshot for trend tracking. Defaults to `.fallow/snapshots/<timestamp>.json`. Forces file-scores + hotspot computation.
- `--format human|json|compact|markdown|sarif` -- output format (default: human)

**Exit codes:** 0 = no functions exceed thresholds, 1 = findings exist.

**JSON output** includes a `findings` array, a `summary` object, and a `vital_signs` object (project-wide metrics: `dead_file_pct`, `dead_export_pct`, `avg_cyclomatic`, `p90_cyclomatic`, `maintainability_avg`, `hotspot_count`, `circular_dep_count`, `unused_dep_count`; null when data source not available). With `--file-scores`, also includes a `file_scores` array with per-file metrics and `summary.files_scored` / `summary.average_maintainability`. With `--targets`, includes a `targets` array with `path`, `priority`, `efficiency` (priority/effort — default sort), `recommendation`, `category`, `effort` (low/medium/high), `confidence` (high/medium/low — based on data source reliability), `factors` (with raw `value`/`threshold`), and `evidence` (unused export names, complex function names+lines, cycle paths). A `target_thresholds` object exposes the adaptive percentile-based thresholds (`fan_in_p95`, `fan_in_p75`, `fan_out_p95`, `fan_out_p90`) used for scoring. Target baselines are supported via `--save-baseline` / `--baseline`.

**Vital signs snapshots:** `--save-snapshot` persists a `VitalSignsSnapshot` JSON file containing `vital_signs` (metrics), `counts` (raw numerators/denominators), and git metadata (`git_sha`, `git_branch`, `shallow_clone`). Snapshot schema version is independent of the report schema_version.

### `fix`

Auto-remove unused exports, dependencies, and enum members.

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

Compact format (`--format compact`) is grep-friendly: one issue per line. Format varies by issue type: `unused-export:path:line:name`, `unused-file:path`, `unused-dep:package_name`, `circular-dependency:path:0:chain`, etc.

JSON output includes `schema_version`, `version`, `elapsed_ms`, and `total_issues` metadata alongside the issue arrays.

## Common agent workflows

### Full codebase analysis (all three: dead code + duplication + complexity)

```bash
fallow --format json --quiet
```

### Audit a project for unused code only

```bash
fallow dead-code --format json --quiet
```

### Check only unused exports (smaller output)

```bash
fallow dead-code --format json --quiet --unused-exports
```

### Check if a PR introduces unused code

```bash
fallow dead-code --format json --quiet --changed-since main --fail-on-issues
```

### Production-only analysis (skip test/dev files)

```bash
fallow dead-code --format json --quiet --production
```

### Analyze a single workspace package (monorepo)

```bash
fallow dead-code --format json --quiet --workspace my-package
```

### Find code duplication

```bash
fallow dupes --format json --quiet
fallow dupes --format json --quiet --mode semantic --threshold 5
```

### Find complex functions

```bash
fallow health --format json --quiet
fallow health --format json --quiet --top 10 --sort cognitive
fallow health --format json --quiet --file-scores   # includes per-file maintainability index
fallow health --format json --quiet --hotspots      # identify riskiest files (churn x complexity)
fallow health --format json --quiet --targets       # ranked refactoring recommendations
```

### Safe auto-fix cycle

```bash
fallow fix --dry-run --format json --quiet   # 1. preview
# agent reviews output
fallow fix --yes --format json --quiet       # 2. apply (--yes required for non-TTY)
fallow dead-code --format json --quiet       # 3. verify
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

## CI integration

### GitHub Actions

```yaml
- uses: fallow-rs/fallow@v1
  with:
    format: sarif
    fail-on-issues: true
```

The action supports SARIF upload to GitHub Code Scanning, PR comments, and all fallow commands/options.

### GitLab CI

```yaml
include:
  - remote: 'https://raw.githubusercontent.com/fallow-rs/fallow/main/ci/gitlab-ci.yml'

fallow:
  extends: .fallow
  variables:
    FALLOW_COMMAND: "dead-code"
    FALLOW_COMMENT: "true"
```

The template generates GitLab Code Quality reports (CodeClimate format) for inline MR annotations, supports MR comments with rich formatting, inline review discussions with suggestion blocks, and all fallow commands. Key variables:

| Variable | Description |
|----------|-------------|
| `FALLOW_COMMAND` | Fallow command to run (`dead-code`, `dupes`, `health`, or bare for all) |
| `FALLOW_COMMENT` | Set to `true` to post a summary comment on the MR |
| `FALLOW_REVIEW` | Set to `true` to post inline review discussions on changed lines |
| `FALLOW_MAX_COMMENTS` | Maximum number of inline review comments per MR (default: 25) |
| `FALLOW_ROOT` | Project root directory |
| `FALLOW_FAIL_ON_ISSUES` | Set to `true` to fail the job when issues are found |
| `FALLOW_CHANGED_SINCE` | Git ref for incremental analysis (auto-set to MR target branch) |

All variables use the `FALLOW_` prefix. MR comments and reviews require a `GITLAB_TOKEN` CI/CD variable (project access token with `api` scope) or enabling job token API access in project settings.

### Any CI

```bash
npx fallow --ci  # equivalent to: --format sarif --fail-on-issues --quiet
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

### JSDoc `@public` tag

Exports annotated with `/** @public */` are never reported as unused. This is intended for library authors whose exports are consumed by external projects outside the analyzed repository.

```ts
/** @public */
export function createClient() { ... }  // Never reported as unused

/** @api public */
export type ClientOptions = { ... }     // TSDoc @api convention also supported
```

Only `/** */` JSDoc block comments are recognized — line comments (`// @public`) have no effect.

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
- Workspace support (npm/yarn/pnpm) is automatic when `workspaces` is defined in the root `package.json` or `pnpm-workspace.yaml` exists. TypeScript project references (`tsconfig.json` `references`) are also discovered as workspaces.
- Inline suppression comments are parsed during extraction and cached alongside module data. They are applied during analysis, before results reach the reporting layer.
- Analysis is deterministic: same input always produces the same output.
