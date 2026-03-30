---
paths:
  - "crates/mcp/**"
---

# fallow-mcp crate

MCP server exposing fallow analysis as tools for AI agents. Stdio transport, wraps `fallow` CLI via subprocess.

## Tools
- `analyze` — full dead code analysis (`fallow dead-code --format json`)
- `check_changed` — incremental analysis (`fallow dead-code --changed-since`)
- `find_dupes` — code duplication (`fallow dupes --format json`)
- `check_health` — complexity metrics (`fallow health --format json`), supports `file_scores`, `hotspots`, `targets`, `since`, `min_commits` params
- `fix_preview` — dry-run auto-fix (`fallow fix --dry-run --format json`)
- `fix_apply` — apply auto-fixes (`fallow fix --yes --format json`) — destructive
- `project_info` — project metadata (`fallow list --format json`)

## Global flags (available on all tools)
- `no_cache` (bool) — disable incremental parse cache
- `threads` (usize) — parser thread count

## Flags on analysis tools (analyze, check_changed, find_dupes, check_health)
- `baseline` (string) — compare against saved baseline
- `save_baseline` (string) — save results as baseline

## Gap fills in v2.1
- `config` added to `find_dupes`, `check_health`
- `workspace` added to `find_dupes`, `fix_preview`, `fix_apply`

All params structs except `CheckChangedParams` derive `Default` for ergonomic test construction.

Built with `rmcp` (official Rust MCP SDK). Thin subprocess wrapper — all analysis logic stays in the CLI.
Set `FALLOW_BIN` env var to point to the fallow binary (defaults to `fallow` in PATH).
