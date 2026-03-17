# fallow

A blazing-fast dead code analyzer for JavaScript and TypeScript projects, written in Rust.

Fallow finds unused files, exports, dependencies, types, enum members, and class members in your codebase. It's a Rust alternative to [knip](https://github.com/webpro-nl/knip) that is 10-100x faster by leveraging the [Oxc](https://oxc.rs) parser ecosystem.

## Features

- **Unused files** — files not reachable from any entry point
- **Unused exports** — exported symbols never imported elsewhere
- **Unused types** — exported type aliases and interfaces never referenced
- **Unused dependencies** — packages in `dependencies` never imported
- **Unused devDependencies** — packages in `devDependencies` never imported
- **Unused enum members** — individual enum values never accessed
- **Unused class members** — class methods and properties never referenced
- **Unresolved imports** — import specifiers that can't be resolved
- **Unlisted dependencies** — packages imported but not in package.json
- **Duplicate exports** — same name exported from multiple files

### Beyond knip

- **Incremental caching** — only reparse changed files (bincode + xxh3)
- **Watch mode** — file watcher with debounced re-analysis
- **Auto-fix** — remove unused exports, clean package.json
- **LSP server** — real-time diagnostics in your editor
- **Git-aware** — only report new issues (`--changed-since main`)
- **Baseline comparison** — `--save-baseline` / `--baseline` for CI
- **SARIF output** — GitHub Code Scanning integration
- **GitHub Action** — drop-in CI integration

## Installation

### npm

```bash
npm install -g fallow
```

### Cargo

```bash
cargo install fallow-cli
```

### From source

```bash
git clone https://github.com/nicholasgasior/fallow
cd fallow
cargo install --path crates/cli
```

## Usage

```bash
# Run analysis on current directory
fallow check

# Run with JSON output
fallow check --format json

# Run with SARIF output (for GitHub Code Scanning)
fallow check --format sarif

# Watch mode
fallow watch

# Auto-fix issues
fallow fix
fallow fix --dry-run

# Only report issues in files changed since main
fallow check --changed-since main

# Baseline comparison
fallow check --save-baseline baseline.json
fallow check --baseline baseline.json --fail-on-issues

# Initialize config
fallow init

# List detected frameworks and entry points
fallow list --frameworks
fallow list --entry-points
fallow list --files
```

## Configuration

Create a `fallow.toml` in your project root:

```toml
# Additional entry points beyond auto-detected ones
entry = ["src/workers/*.ts", "scripts/*.ts"]

# Patterns to exclude from analysis
ignore = ["**/*.generated.ts", "**/*.d.ts"]

# Dependencies to always consider as used
ignore_dependencies = ["autoprefixer", "@types/node"]

# What to detect
[detect]
unused_files = true
unused_exports = true
unused_dependencies = true
unused_dev_dependencies = true
unused_types = true
unused_enum_members = true
unused_class_members = true
unresolved_imports = true
unlisted_dependencies = true
duplicate_exports = true

# Ignore specific exports
[[ignore_exports]]
file = "src/utils/**"
exports = ["*"]

[[ignore_exports]]
file = "src/types.ts"
exports = ["InternalType"]
```

## Framework Support

Fallow auto-detects frameworks and adjusts analysis accordingly. Each framework defines entry points, always-used files, and exports considered used.

| Framework | Detection |
|-----------|-----------|
| Next.js | `next` in dependencies |
| Vite | `vite` in dependencies |
| Vitest | `vitest` in dependencies |
| Jest | `jest` in dependencies |
| Storybook | `.storybook/main.{ts,js}` exists |
| Remix | `@remix-run/node` in dependencies |
| Astro | `astro` in dependencies |
| Nuxt | `nuxt` in dependencies |
| Angular | `@angular/core` in dependencies |
| Playwright | `@playwright/test` in dependencies |
| Prisma | `prisma` in dependencies |
| ESLint | `eslint` in dependencies |
| TypeScript | `typescript` in dependencies |
| Webpack | `webpack` in dependencies |
| Tailwind CSS | `tailwindcss` in dependencies |
| GraphQL Codegen | `@graphql-codegen/cli` in dependencies |
| React Router | `react-router` in dependencies |

### Custom frameworks

```toml
[[framework]]
name = "my-framework"

[framework.detection]
type = "dependency"
package = "my-framework"

[[framework.entry_points]]
pattern = "src/routes/**/*.ts"

[framework.always_used]
patterns = ["my-framework.config.ts"]

[[framework.used_exports]]
file_pattern = "src/routes/**/*.ts"
exports = ["default", "loader"]
```

## GitHub Action

```yaml
- uses: nicholasgasior/fallow@v1
  with:
    format: sarif
    fail-on-issues: true
    # Only check files changed in this PR
    changed-since: ${{ github.event.pull_request.base.sha }}
```

## Output Formats

- **human** — colored terminal output with file paths and export names
- **json** — machine-readable JSON for tooling integration
- **sarif** — SARIF 2.1.0 for GitHub Code Scanning
- **compact** — one issue per line, grep-friendly

## Architecture

```
Config → File Discovery → Parallel Parsing (rayon + oxc) → Module Resolution (oxc_resolver)
    → Graph Construction → Dead Code Detection → Reporting
```

Key design decisions:

- **No TypeScript compiler** — syntactic analysis only via Oxc parser. This is what makes fallow fast; knip uses `ts.createProgram()` which is the bottleneck.
- **Declarative framework presets** — knip has 140+ JS plugins. ~85% are just glob patterns. Fallow expresses these as data (TOML), not code.
- **Flat edge storage** — contiguous `Vec<Edge>` with range indices for cache-friendly graph traversal.
- **Thread-local allocators** — each rayon thread gets its own `oxc_allocator::Allocator`. Zero contention during parsing.

## Comparison with knip

| Feature | fallow | knip |
|---------|--------|------|
| Language | Rust | TypeScript |
| Parser | Oxc | TypeScript compiler |
| Parallel parsing | Yes (rayon) | No |
| Speed | 10-100x faster | Baseline |
| Unused files | Yes | Yes |
| Unused exports | Yes | Yes |
| Unused types | Yes | Yes |
| Unused deps | Yes | Yes |
| Unused enum members | Yes | Yes |
| Unused class members | Yes | Yes |
| Unresolved imports | Yes | Yes |
| Unlisted deps | Yes | Yes |
| Duplicate exports | Yes | Yes |
| Watch mode | Yes | No |
| Auto-fix | Yes | No |
| LSP server | Yes | No |
| Git-aware | Yes | No |
| Baseline comparison | Yes | No |
| SARIF output | Yes | No |
| Incremental cache | Yes | No |
| Framework presets | 17 | 140+ |

## License

MIT
