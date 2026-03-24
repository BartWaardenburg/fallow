# Fallow - Rust-native codebase analyzer for TypeScript/JavaScript

Fallow finds unused files, exports, dependencies, types, enum members, class members, unresolved imports, unlisted deps, duplicate exports, circular dependencies, and code duplication in JS/TS projects. Rust alternative to [knip](https://github.com/webpro-nl/knip) built on the Oxc parser ecosystem.

## Project structure

```
crates/
  config/   — Configuration types, custom framework presets, package.json parsing, workspace discovery
  types/    — Shared type definitions (discover, extract, results, suppress, serde_path)
  extract/  — AST extraction engine (visitor.rs, complexity.rs, sfc.rs, astro.rs, mdx.rs, css.rs, parse.rs, cache.rs, suppress.rs, tests/)
  graph/    — Module graph construction (graph/), import resolution (resolve.rs), project state (project.rs)
  core/     — Analysis orchestration: discovery, plugins, scripts, duplicates, cross-reference, caching, progress
    analyze/    — Dead code detection (mod.rs orchestration, predicates.rs, unused_files/exports/deps/members.rs)
    plugins/    — Plugin system + tooling.rs (general tooling dependency detection)
    duplicates/ — Clone detection (families, normalize, tokenize)
  cli/      — CLI binary, split into per-command modules
    check.rs, dupes.rs, health.rs, watch.rs, fix/, init.rs, list.rs, schema.rs, validate.rs
    report/     — Output formatting (mod.rs dispatch, human.rs, json.rs, sarif.rs, compact.rs, markdown.rs)
    migrate/    — Config migration (mod.rs, knip.rs, jscpd.rs)
  lsp/      — LSP server, split into modules
    main.rs, diagnostics.rs, code_actions.rs, code_lens.rs, hover.rs
  mcp/      — MCP server for AI agent integration (stdio transport, wraps CLI)
editors/
  vscode/   — VS Code extension (LSP client, tree views, status bar, auto-download)
npm/
  fallow/   — npm wrapper package with optionalDependencies pattern
tests/
  fixtures/ — Integration test fixtures
```

## Architecture

Pipeline: Config → File Discovery → Incremental Parallel Parsing (rayon + oxc_parser + oxc_semantic, cache-aware) → Script Analysis → Module Resolution (oxc_resolver) → Graph Construction → Re-export Chain Resolution → Dead Code Detection → Reporting

## Building & Testing

```bash
git config core.hooksPath .githooks  # Enable pre-commit hooks (fmt + clippy)
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
cargo run -- check              # Run analysis
cargo run -- watch              # Watch mode
cargo run -- fix --dry-run      # Auto-fix preview
```

## Code conventions

- All clippy suppressions use `#[expect(clippy::...)]` not `#[allow]` — warns when suppression becomes unnecessary
- Const size assertions on hot-path structs (`ModuleNode`, `ModuleInfo`, `ExportInfo`, `ImportInfo`, `Edge`)
- Config files: `.fallowrc.json` > `fallow.toml` > `.fallow.toml`
- No `detect` section in config — use `rules` with `"off"` severity
- No `output` in config — output format is CLI-only via `--format`
- Rules severity: `error` (fail CI, default) | `warn` (exit 0) | `off` (skip)
- Inline suppression: `// fallow-ignore-next-line [issue-type]` and `// fallow-ignore-file [issue-type]`
- Environment variables: `FALLOW_FORMAT`, `FALLOW_QUIET`, `FALLOW_BIN` (binary path for MCP)

## Key design decisions

- **No TypeScript compiler**: Syntactic analysis via Oxc parser + `oxc_semantic` for scope-aware binding analysis. No type resolution, no tsc.
- **Flat edge storage**: Contiguous `Vec<Edge>` with range indices for cache-friendly traversal.
- **Re-export chain resolution**: Iterative propagation through barrel files with cycle detection.
- **FileIds are path-sorted** (not insertion order) for stable cross-run identity.
- **Hidden directory allowlist**: `.storybook`, `.well-known`, `.changeset`, `.github` — other dotdirs are skipped. Only root-level `build/` is ignored.

## Git conventions

- Conventional commits: `feat:`, `fix:`, `chore:`, `refactor:`, `test:`
- Signed commits (`git commit -S`)
- No AI attribution in commits

See `AGENTS.md` for AI agent integration guide.
