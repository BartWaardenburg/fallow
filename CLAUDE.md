# Fallow - Rust-native dead code analyzer for JavaScript/TypeScript

## What is this?

Fallow finds unused files, exports, dependencies, types, enum members, class members, unresolved imports, unlisted deps, and duplicate exports in JS/TS projects. It's a Rust alternative to [knip](https://github.com/webpro-nl/knip) that is 25-40x faster on real-world projects by leveraging the Oxc parser ecosystem.

## Project structure

```
crates/
  config/   — Configuration types, custom framework presets, package.json parsing, workspace discovery
  core/     — Analysis engine: discovery, parsing, resolution, graph, plugins, caching, progress
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
- `extract.rs` — AST visitor extracting imports, exports, re-exports, members, whole-object uses, dynamic import patterns; SFC (Vue/Svelte) script extraction
- `resolve.rs` — oxc_resolver-based import resolution + glob-based dynamic import pattern resolution
- `graph.rs` — Module graph with re-export chain propagation
- `analyze.rs` — Dead code detection (10 issue types)
- `plugins/` — Plugin system: `Plugin` trait, registry, AST-based config parsing (40 built-in plugins)
- `cache.rs` — Incremental bincode cache with xxh3 hashing
- `progress.rs` — indicatif progress bars
- `errors.rs` — Error types

## Building & Testing

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
cargo run -- check              # Run analysis
cargo run -- watch              # Watch mode
cargo run -- fix --dry-run      # Auto-fix preview
```

## Detection capabilities

1. Unused files, exports, types, dependencies, devDependencies
2. Unused enum members, class members (structural extraction + whole-object-use heuristics for Object.values/keys/entries, for..in, spread, computed access)
3. Unresolved imports, unlisted dependencies
4. Duplicate exports across modules
5. Re-export chain resolution through barrel files
6. Vue/Svelte SFC parsing (regex-based `<script>` block extraction, `lang="ts"` detection)
7. Dynamic import pattern resolution (template literals, string concat, import.meta.glob, require.context → glob matching against discovered files)

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
- **Custom framework presets** (`crates/config/src/framework.rs`) — Users can add custom framework definitions via `fallow.toml` for project-specific entry points and rules.

## CLI features

- `check` — analyze with --format (human/json/sarif/compact), --changed-since, --baseline, --save-baseline, issue type filters (--unused-files, --unused-exports, etc.)
- `watch` — file watcher with debounced re-analysis
- `fix` — auto-remove unused exports and deps (--dry-run, --format json for structured output)
- `init` — create fallow.toml
- `list` — show active plugins, entry points, files (--format json for structured output)
- `schema` — dump CLI interface as machine-readable JSON for agent introspection

See `AGENTS.md` for AI agent integration guide.

## Key design decisions

- **No TypeScript compiler dependency**: Syntactic analysis only via Oxc. This is the speed advantage.
- **Plugin system**: Single source of truth for framework support. Rust trait-based plugins with AST-based config parsing (no JavaScript evaluation). Static patterns for common cases, dynamic Oxc parsing for tool configs. 40 built-in plugins covering ~99% of the JS/TS ecosystem.
- **Flat edge storage**: Contiguous `Vec<Edge>` with range indices for cache-friendly traversal.
- **Re-export chain resolution**: Iterative propagation through barrel files with cycle detection.
- **Workspace support**: npm/yarn/pnpm workspaces, pnpm-workspace.yaml.

## Git conventions

- Conventional commits: `feat:`, `fix:`, `chore:`, `refactor:`, `test:`
- Signed commits (`git commit -S`)
- No AI attribution in commits
