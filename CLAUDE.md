# Fallow - Rust-native dead code analyzer for JavaScript/TypeScript

## What is this?

Fallow finds unused files, exports, dependencies, and types in JS/TS projects. It's a Rust alternative to [knip](https://github.com/webpro-nl/knip) that is 10-100x faster by leveraging the Oxc parser ecosystem.

## Project structure

```
crates/
  config/   — Configuration types, framework presets, package.json parsing
  core/     — Analysis engine: file discovery, parsing, module resolution, graph, analysis
  cli/      — CLI binary (clap-based)
  lsp/      — LSP server for real-time editor integration
reference/
  knip/     — Knip source code for reference (gitignored)
```

## Architecture

Pipeline: Config → File Discovery → Parallel Parsing (rayon + oxc_parser) → Module Resolution (oxc_resolver) → Graph Construction → Dead Code Detection → Reporting

Key crates used:
- `oxc_parser` + `oxc_ast` + `oxc_ast_visit` for parsing JS/TS/JSX/TSX
- `oxc_resolver` for import path resolution (tsconfig paths, package.json exports, etc.)
- `rayon` for parallel file parsing
- `ignore` (ripgrep's crate) for fast .gitignore-aware file walking
- `tower-lsp` for the LSP server

## Building

```bash
cargo build --workspace
cargo test --workspace
cargo run -- check              # Run analysis on current directory
cargo run -- check --format json
cargo run -- init               # Create fallow.toml
cargo run -- list --frameworks  # Show detected frameworks
```

## Framework support

Frameworks are defined declaratively in `crates/config/src/framework.rs` as `FrameworkRule` structs (no JS plugins needed). Each defines:
- Detection (which dependency or file indicates this framework)
- Entry point patterns
- Always-used file patterns
- Exports considered always-used (e.g., Next.js route handlers)

Currently supported: Next.js, Vite, Vitest, Jest, Storybook, Remix, Astro.

## Key design decisions

- **No TypeScript compiler dependency**: We do syntactic analysis only (AST parsing for imports/exports). This is what makes us fast — knip uses `ts.createProgram()` which is the bottleneck.
- **Declarative framework presets instead of JS plugins**: Knip has 140+ JS plugins. ~85% are just glob patterns. We express these as data, not code.
- **Flat edge storage**: The module graph uses contiguous `Vec<Edge>` with range indices for cache-friendly traversal.
- **Thread-local allocators**: Each rayon thread gets its own `oxc_allocator::Allocator`. Zero contention during parsing.

## Reference: knip comparison targets

Knip detects: unused files, exports, types, dependencies, devDependencies, enum members, class members, duplicates, unlisted binaries, unresolved imports. We should match all of these.

## Git conventions

- Conventional commits: `feat:`, `fix:`, `chore:`, `refactor:`, `test:`
- Signed commits (`git commit -S`)
- No AI attribution in commits
