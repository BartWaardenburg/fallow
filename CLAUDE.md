# Fallow - Rust-native dead code analyzer for JavaScript/TypeScript

## What is this?

Fallow finds unused files, exports, dependencies, types, enum members, class members, unresolved imports, unlisted deps, and duplicate exports in JS/TS projects. It's a Rust alternative to [knip](https://github.com/webpro-nl/knip) that is 10-100x faster by leveraging the Oxc parser ecosystem.

## Project structure

```
crates/
  config/   — Configuration types, framework presets, package.json parsing, workspace discovery
  core/     — Analysis engine: discovery, parsing, resolution, graph, analysis, caching, progress
  cli/      — CLI binary (check, watch, fix, init, list commands)
  lsp/      — LSP server with diagnostics, code actions
npm/
  fallow/   — npm wrapper package with optionalDependencies pattern
tests/
  fixtures/ — Integration test fixtures (basic-project, barrel-exports)
```

## Architecture

Pipeline: Config → File Discovery → Parallel Parsing (rayon + oxc_parser) → Module Resolution (oxc_resolver) → Graph Construction → Re-export Chain Resolution → Dead Code Detection → Reporting

Key modules in fallow-core:
- `discover.rs` — File walking + entry point detection (also workspace-aware)
- `extract.rs` — AST visitor extracting imports, exports, re-exports, members
- `resolve.rs` — oxc_resolver-based import resolution
- `graph.rs` — Module graph with re-export chain propagation
- `analyze.rs` — Dead code detection (10 issue types)
- `cache.rs` — Incremental bincode cache with xxh3 hashing
- `progress.rs` — indicatif progress bars
- `errors.rs` — Error types

## Building & Testing

```bash
cargo build --workspace
cargo test --workspace          # 41 tests (15 unit + 26 integration)
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
cargo run -- check              # Run analysis
cargo run -- watch              # Watch mode
cargo run -- fix --dry-run      # Auto-fix preview
```

## Detection capabilities

1. Unused files, exports, types, dependencies, devDependencies
2. Unused enum members, class members (structural extraction)
3. Unresolved imports, unlisted dependencies
4. Duplicate exports across modules
5. Re-export chain resolution through barrel files

## Framework support (17 presets)

Next.js, Vite, Vitest, Jest, Storybook, Remix, Astro, Nuxt, Angular, Playwright, Prisma, ESLint, TypeScript, Webpack, Tailwind, GraphQL Codegen, React Router

Defined declaratively in `crates/config/src/framework.rs`.

## CLI features

- `check` — analyze with --format (human/json/sarif/compact), --changed-since, --baseline, --save-baseline
- `watch` — file watcher with debounced re-analysis
- `fix` — auto-remove unused exports and deps (--dry-run)
- `init` — create fallow.toml
- `list` — show frameworks, entry points, files

## Key design decisions

- **No TypeScript compiler dependency**: Syntactic analysis only via Oxc. This is the speed advantage.
- **Declarative framework presets**: Data (TOML), not code. Knip has 140+ JS plugins; ~85% are just glob patterns.
- **Flat edge storage**: Contiguous `Vec<Edge>` with range indices for cache-friendly traversal.
- **Re-export chain resolution**: Iterative propagation through barrel files with cycle detection.
- **Workspace support**: npm/yarn/pnpm workspaces, pnpm-workspace.yaml.

## Git conventions

- Conventional commits: `feat:`, `fix:`, `chore:`, `refactor:`, `test:`
- Signed commits (`git commit -S`)
- No AI attribution in commits
