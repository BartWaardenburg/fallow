# Fallow - Rust-native dead code analyzer for JavaScript/TypeScript

## What is this?

Fallow finds unused files, exports, dependencies, types, enum members, class members, unresolved imports, unlisted deps, and duplicate exports in JS/TS projects. It's a Rust alternative to [knip](https://github.com/webpro-nl/knip) that is 3-40x faster depending on project size (up to 40x on small projects, 3-10x on 1,000-5,000+ file projects) by leveraging the Oxc parser ecosystem.

## Project structure

```
crates/
  config/   — Configuration types, custom framework presets, package.json parsing, workspace discovery
  core/     — Analysis engine: discovery, parsing, resolution, graph, plugins, caching, progress
  cli/      — CLI binary (check, dupes, watch, fix, init, list, schema commands)
  lsp/      — LSP server with diagnostics, code actions
  mcp/      — MCP server for AI agent integration (stdio transport, wraps CLI)
npm/
  fallow/   — npm wrapper package with optionalDependencies pattern
tests/
  fixtures/ — Integration test fixtures (basic-project, barrel-exports)
```

## Architecture

Pipeline: Config → File Discovery → Parallel Parsing (rayon + oxc_parser) → Script Analysis → Module Resolution (oxc_resolver) → Graph Construction → Re-export Chain Resolution → Dead Code Detection → Reporting

Key modules in fallow-core:
- `project.rs` — `ProjectState` struct: owns the file registry (stable FileIds sorted by path) and workspace metadata. Foundation for cross-workspace resolution and future incremental analysis.
- `discover.rs` — File walking + entry point detection (also workspace-aware). FileIds are assigned deterministically by path sort order (not size) for stability across runs.
- `extract.rs` — AST visitor extracting imports, exports, re-exports, members, whole-object uses, dynamic import patterns; SFC (Vue/Svelte) script extraction; Astro frontmatter extraction; MDX import/export extraction
- `resolve.rs` — oxc_resolver-based import resolution + glob-based dynamic import pattern resolution + DashMap-backed bare specifier cache for lock-free parallel lookups. Cross-workspace imports resolve through node_modules symlinks via canonicalize.
- `graph.rs` — Module graph with re-export chain propagation
- `analyze.rs` — Dead code detection (10 issue types) with inline suppression filtering
- `scripts.rs` — Shell command parser for package.json scripts: extracts binary names (mapped to package names for dependency usage detection), `--config` args (entry points), and file path args; handles env wrappers, package manager runners, node runners
- `suppress.rs` — Inline suppression comment parsing (`fallow-ignore-next-line`, `fallow-ignore-file`); 11 issue kinds including `code-duplication`
- `duplicates/families.rs` — Clone family grouping (groups by shared file set) and refactoring suggestion generation (extract function/module)
- `plugins/` — Plugin system: `Plugin` trait, registry (40 built-in plugins, ~20 with AST-based config parsing); `config_parser.rs` provides Oxc-based helpers for extracting imports, string arrays, object keys, require() sources, and string-or-array values from JS/TS/JSON config files
- `cache.rs` — Incremental bincode cache with xxh3 hashing
- `progress.rs` — indicatif progress bars
- `errors.rs` — Error types

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

# Benchmarks (see BENCHMARKS.md for methodology)
cargo bench --bench analysis                           # All Criterion benchmarks
cargo bench --bench analysis -- large_scale_benches/   # 1000+ and 5000+ file benchmarks only
cd benchmarks && npm run generate && npm run bench     # Comparative benchmarks vs knip
cd benchmarks && npm run generate:dupes && npm run bench:dupes  # vs jscpd
```

## Detection capabilities

1. Unused files, exports, types, dependencies, devDependencies
2. Unused enum members, class members (structural extraction + whole-object-use heuristics for Object.values/keys/entries, for..in, spread, computed access)
3. Unresolved imports, unlisted dependencies
4. Duplicate exports across modules
5. Re-export chain resolution through barrel files
6. Vue/Svelte SFC parsing (regex-based `<script>` block extraction, `lang="ts"`/`lang="tsx"` detection, handles `>` in quoted attributes like `generic="T extends Foo<Bar>"`)
7. Astro component parsing (frontmatter extraction between `---` delimiters, parsed as TypeScript)
8. MDX file parsing (line-based import/export statement extraction with multi-line brace tracking, parsed as JSX)
9. Dynamic import pattern resolution (template literals, string concat, import.meta.glob, require.context → glob matching against discovered files)
10. Inline suppression comments (`// fallow-ignore-next-line [issue-type]`, `// fallow-ignore-file [issue-type]`) — supports all issue types including `code-duplication`
11. Script binary analysis (package.json scripts → binary names mapped to packages, `--config` args as entry points, env wrapper/package manager runner handling)
12. Clone family grouping: groups clone groups sharing the same file set into families with refactoring suggestions (extract function/module)
13. Duplication baseline support: `--save-baseline` / `--baseline` for incremental CI adoption of duplication thresholds
14. Production mode (`--production`): excludes test/dev files, only start/build scripts, detects type-only dependencies

## Framework support (40 plugins)

**Frameworks**: Next.js, Nuxt, Remix, Astro, Angular, React Router, React Native, Expo, NestJS, Docusaurus
**Bundlers**: Vite, Webpack, Rollup, Tsup
**Testing**: Vitest, Jest, Playwright, Cypress, Mocha, Ava, Storybook
**Linting**: ESLint, Biome, Stylelint, Commitlint
**Transpilation**: TypeScript, Babel
**CSS**: Tailwind, PostCSS
**Database**: Prisma, Drizzle, Knex
**Monorepo**: Turborepo, Nx, Changesets
**CI/CD**: semantic-release
**Deployment**: Wrangler (Cloudflare), Sentry
**Other**: GraphQL Codegen, MSW

- **Plugins** (`crates/core/src/plugins/`) — Single source of truth for all built-in framework support. Each plugin implements the `Plugin` trait with enablers (package.json detection), static patterns (entry points, always-used files, used exports, tooling dependencies), and optional `resolve_config()` for AST-based config parsing via Oxc.
- **Rich config parsing** — All top 10 framework plugins have deep `resolve_config()` implementations:
  - **ESLint**: Legacy plugin/extends/parser short-name resolution, flat config plugin keys, JSON config
  - **Vite**: rollupOptions.input, lib.entry, optimizeDeps include/exclude, ssr.external/noExternal
  - **Jest**: preset, setupFiles, globalSetup/Teardown, testMatch, transform, reporters, testEnvironment, watchPlugins, resolver, snapshotSerializers, testRunner, runner, JSON config
  - **Storybook**: addons, framework (string/object), stories, core.builder, typescript.reactDocgen
  - **Tailwind**: content globs, plugins (require/strings), presets
  - **Webpack**: entry (string/array/object), plugins require(), externals, module.rules loader extraction (loader/use/oneOf)
  - **TypeScript**: extends (string/array TS 5.0+), compilerOptions.types → @types/*, jsxImportSource, compilerOptions.plugins, references[].path, JSONC support
  - **Babel**: presets/plugins with short-name resolution (e.g. "env" → "@babel/preset-env"), extends, JSON/.babelrc support
  - **Rollup**: input entries, external deps
  - **PostCSS**: plugins (object keys, require() calls, string arrays)
- **Custom framework presets** (`crates/config/src/framework.rs`) — Users can add custom framework definitions via `fallow.toml` for project-specific entry points and rules.

## CLI features

- `check` — analyze with --format (human/json/sarif/compact), --changed-since, --baseline, --save-baseline, --fail-on-issues, issue type filters (--unused-files, --unused-exports, etc.)
- `dupes` — find code duplication with clone families, refactoring suggestions, --baseline/--save-baseline, --mode (strict/mild/weak/semantic), --min-tokens, --min-lines, --threshold, --skip-local
- `watch` — file watcher with debounced re-analysis
- `fix` — auto-remove unused exports and deps (--dry-run, --format json for structured output)
- `init` — create fallow.jsonc (default) or fallow.toml (`--toml`), includes `$schema` for IDE autocomplete
- `list` — show active plugins, entry points, files (--format json for structured output)
- `schema` — dump CLI interface as machine-readable JSON for agent introspection
- `config-schema` — print JSON Schema for fallow config files (enables IDE validation)
- Global `--workspace <name>` / `-w` flag scopes output to a single workspace package while keeping the full cross-workspace graph

See `AGENTS.md` for AI agent integration guide.

## MCP server

`fallow-mcp` is an MCP (Model Context Protocol) server that exposes fallow's analysis as tools for AI agents. It uses stdio transport and wraps the `fallow` CLI binary via subprocess.

**Tools:**
- `analyze` — full dead code analysis (wraps `fallow check --format json`)
- `check_changed` — incremental analysis of changed files (wraps `fallow check --changed-since`)
- `find_dupes` — code duplication detection (wraps `fallow dupes --format json`)
- `fix_preview` — dry-run auto-fix preview (wraps `fallow fix --dry-run --format json`)
- `fix_apply` — apply auto-fixes (wraps `fallow fix --format json`) — destructive
- `project_info` — project metadata: plugins, files, entry points (wraps `fallow list --format json`)

**Configuration:** Set `FALLOW_BIN` env var to point to the fallow binary (defaults to `fallow` in PATH).

**Architecture:** Built with `rmcp` (official Rust MCP SDK). Thin subprocess wrapper — all analysis logic stays in the CLI, the MCP crate only handles protocol framing and argument mapping.

## Production mode

`--production` flag (or `production = true` in fallow.toml) for CI pipelines that only care about production code:

- **Excludes test/dev files**: `*.test.*`, `*.spec.*`, `*.stories.*`, `__tests__/**`, `__mocks__/**`, etc.
- **Only start/build scripts**: Only analyzes production-relevant package.json scripts (`start`, `build`, `serve`, `preview`, `prepare` and their pre/post hooks)
- **Skips unused devDependencies**: Forces `unused_dev_dependencies` severity to `off`
- **Reports type-only dependencies**: Detects production dependencies only imported via `import type` (should be devDependencies since types are erased at runtime)

## Configuration format

Supports JSONC (default), JSON, and TOML. Config files are searched in priority order:
`fallow.jsonc` > `fallow.json` > `fallow.toml` > `.fallow.toml`

- JSONC is the default for `fallow init` — matches the Oxc ecosystem (oxlint, oxfmt, Biome)
- TOML is still fully supported via `fallow init --toml`
- A `$schema` field in JSON/JSONC enables IDE autocomplete and validation
- Run `fallow config-schema` to generate the JSON Schema, or reference it from GitHub
- The `schema.json` file is checked into the repo root

## Rules system

Per-issue-type severity for incremental CI adoption:

```jsonc
// fallow.jsonc
{
  "$schema": "https://raw.githubusercontent.com/fallow-rs/fallow/main/schema.json",
  "rules": {
    "unused_files": "error",       // fail CI (exit 1)
    "unused_exports": "warn",      // report but don't fail
    "unused_types": "off",         // ignore entirely
    "unresolved_imports": "error"
  }
}
```

Or equivalently in TOML:

```toml
[rules]
unused_files = "error"       # fail CI (exit 1)
unused_exports = "warn"      # report but don't fail
unused_types = "off"         # ignore entirely
unresolved_imports = "error"
```

- `error` — report and fail CI (non-zero exit code)
- `warn` — report but exit 0
- `off` — don't detect or report
- All default to `error` when omitted (backwards compatible)
- `[detect] unused_X = false` still works and forces `Severity::Off`
- `--fail-on-issues` promotes all `warn` to `error` for that run
- Human output colors reflect severity; SARIF levels are dynamic

## Inline suppression comments

- `// fallow-ignore-next-line` — suppress any issue on the next line
- `// fallow-ignore-next-line unused-export` — suppress specific issue type
- `// fallow-ignore-file` — suppress all issues in a file
- `// fallow-ignore-file unused-export` — suppress specific issue type file-wide
- `// fallow-ignore-file code-duplication` — suppress duplication detection for a file
- `// fallow-ignore-next-line code-duplication` — suppress duplication detection for code on the next line

## Key design decisions

- **No TypeScript compiler dependency**: Syntactic analysis only via Oxc. This is the speed advantage.
- **Plugin system**: Single source of truth for framework support. Rust trait-based plugins with static patterns for common cases and optional AST-based config parsing via Oxc for ~20 plugins (no JavaScript evaluation), 15 with rich config extraction (entry points, dependencies, setup files from config objects). 40 built-in plugins covering the most popular JS/TS frameworks.
- **Flat edge storage**: Contiguous `Vec<Edge>` with range indices for cache-friendly traversal.
- **Lock-free parallel resolution**: Bare specifier cache uses `DashMap` (sharded concurrent map) for contention-free reads under rayon work-stealing.
- **Re-export chain resolution**: Iterative propagation through barrel files with cycle detection.
- **Cross-workspace resolution**: Unified module graph across npm/yarn/pnpm workspaces (pnpm-workspace.yaml). Cross-package imports resolve through node_modules symlinks via `canonicalize()`. `--workspace <name>` scopes output to one package while keeping the full graph. `ProjectState` struct owns the file registry with stable FileIds (path-sorted) for future incremental analysis.

## Git conventions

- Conventional commits: `feat:`, `fix:`, `chore:`, `refactor:`, `test:`
- Signed commits (`git commit -S`)
- No AI attribution in commits
