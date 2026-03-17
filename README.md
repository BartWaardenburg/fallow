<p align="center">
  <img src="assets/logo.png" alt="fallow" width="280">
</p>

<p align="center">
  <strong>Find dead code in JavaScript and TypeScript projects. Written in Rust.</strong>
</p>

<p align="center">
  <a href="https://github.com/BartWaardenburg/fallow/actions/workflows/ci.yml"><img src="https://github.com/BartWaardenburg/fallow/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/BartWaardenburg/fallow/actions/workflows/coverage.yml"><img src="https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/BartWaardenburg/fallow/badges/coverage.json" alt="Coverage"></a>
  <a href="https://crates.io/crates/fallow-cli"><img src="https://img.shields.io/crates/v/fallow-cli.svg" alt="crates.io"></a>
  <a href="https://www.npmjs.com/package/fallow"><img src="https://img.shields.io/npm/v/fallow.svg" alt="npm"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License"></a>
</p>

<p align="center">
  <a href="#quick-start">Quick Start</a> ·
  <a href="#what-it-finds">What It Finds</a> ·
  <a href="#configuration">Configuration</a> ·
  <a href="#ci-integration">CI Integration</a>
</p>

---

Fallow detects unused files, exports, dependencies, types, enum members, and class members across your codebase. It is a drop-in alternative to [knip](https://knip.dev) that runs **10–100x faster** by using the [Oxc](https://oxc.rs) parser instead of the TypeScript compiler.

<!-- TODO: Replace with an actual terminal recording (e.g. VHS/asciinema SVG) -->

```
$ fallow check

Unused files (3)
------------------------------------------------------------
  src/legacy/oldUtils.ts
  src/components/DeprecatedBanner.tsx
  src/lib/unused-helper.ts

Unused exports (5)
------------------------------------------------------------
  src/utils/format.ts  `formatCurrency`
  src/utils/format.ts  `formatPercentage`
  src/hooks/useAuth.ts  `AuthContext`
  src/api/client.ts  `legacyFetch`
  src/types/models.ts  `DeprecatedUser`

Unused dependencies (2)
------------------------------------------------------------
  moment
  lodash

Unlisted dependencies (1)
------------------------------------------------------------
  chalk

Found 11 issues (0.42s)
```

## Why fallow?

**knip is good. fallow is fast.**

knip calls `ts.createProgram()` on every run — it loads the full TypeScript compiler to type-check your project, even though dead code detection is a syntactic problem. For a 2,000-file monorepo, that means 30–60 seconds of waiting.

fallow takes a different approach:

- **Oxc parser** — syntactic analysis only, no type-checking overhead
- **Parallel parsing** — every file parsed on its own rayon thread with a thread-local allocator, zero contention
- **Incremental cache** — xxh3 content hashing + bincode serialization, only changed files are reparsed
- **Flat graph storage** — contiguous `Vec<Edge>` with range indices for cache-friendly traversal

The result: sub-second analysis on codebases where knip takes a minute.

<!-- TODO: Add hyperfine benchmark chart (assets/benchmark.svg) -->
<!-- hyperfine --warmup 3 'fallow check --quiet' 'npx knip --no-progress' -->

## Quick start

```bash
npx fallow check
```

Or install globally:

```bash
npm install -g fallow        # npm (prebuilt binaries via optionalDependencies)
cargo install fallow-cli     # cargo
```

## What it finds

| Issue | Description |
|:------|:------------|
| **Unused files** | Not reachable from any entry point |
| **Unused exports** | Exported symbols never imported elsewhere |
| **Unused types** | Type aliases and interfaces never referenced |
| **Unused dependencies** | Packages in `dependencies` never imported |
| **Unused devDependencies** | Packages in `devDependencies` never imported |
| **Unused enum members** | Individual enum values never accessed |
| **Unused class members** | Methods and properties never referenced externally |
| **Unresolved imports** | Import specifiers that cannot be resolved |
| **Unlisted dependencies** | Imported packages missing from `package.json` |
| **Duplicate exports** | Same symbol exported from multiple modules |

## Features knip doesn't have

- **Watch mode** — file watcher with 500ms debounce, re-analyzes on save
- **Auto-fix** — remove unused exports, clean dependencies from `package.json`
- **LSP server** — real-time diagnostics and quick-fix code actions in your editor
- **Incremental cache** — only reparse changed files across runs
- **Git-aware** — only report issues in files changed since a branch (`--changed-since main`)
- **Baseline comparison** — save a snapshot, only fail CI on *new* issues
- **SARIF output** — native GitHub Code Scanning integration
- **GitHub Action** — drop-in CI with one line of YAML

## Usage

```bash
fallow check                          # Analyze current directory
fallow check --format json            # Machine-readable JSON
fallow check --format sarif           # SARIF for GitHub Code Scanning
fallow check --format compact         # One issue per line, grep-friendly

fallow watch                          # Re-analyze on file changes

fallow fix                            # Auto-remove unused exports and deps
fallow fix --dry-run                  # Preview what would change

fallow check --changed-since main     # Only issues in changed files
fallow check --save-baseline b.json   # Snapshot current state
fallow check --baseline b.json        # Fail only on new issues

fallow init                           # Create fallow.toml
fallow list --frameworks              # Show detected frameworks
fallow list --entry-points            # Show entry points
fallow list --files                   # Show discovered files
```

## Configuration

Create `fallow.toml` in your project root, or run `fallow init`:

```toml
# Additional entry points beyond auto-detected ones
entry = ["src/workers/*.ts", "scripts/*.ts"]

# Patterns to exclude from analysis
ignore = ["**/*.generated.ts", "**/*.d.ts"]

# Dependencies to always consider as used
ignore_dependencies = ["autoprefixer", "@types/node"]

# What to detect (all default to true)
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
```

<details>
<summary><strong>Ignoring specific exports</strong></summary>

```toml
# Ignore all exports from utility files
[[ignore_exports]]
file = "src/utils/**"
exports = ["*"]

# Ignore a specific export
[[ignore_exports]]
file = "src/types.ts"
exports = ["InternalType"]
```

</details>

<details>
<summary><strong>Custom framework presets</strong></summary>

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

</details>

## Framework support

Fallow auto-detects 17 frameworks and adjusts entry points and used exports accordingly. Frameworks are defined as **declarative TOML presets**, not JavaScript plugins — because ~85% of knip's 140+ plugins are just glob patterns expressed as code.

| Framework | Detection |
|:----------|:----------|
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

## CI integration

### GitHub Action

```yaml
- uses: bartwaardenburg/fallow@v1
  with:
    format: sarif           # Uploads to GitHub Code Scanning automatically
    fail-on-issues: true
    changed-since: ${{ github.event.pull_request.base.sha }}
```

### Baseline workflow

Prevent dead code from growing without blocking PRs on existing debt:

```yaml
# On main: save baseline
- run: fallow check --save-baseline fallow-baseline.json --quiet
- uses: actions/upload-artifact@v4
  with:
    name: fallow-baseline
    path: fallow-baseline.json

# On PRs: fail only on new issues
- uses: actions/download-artifact@v4
  with:
    name: fallow-baseline
- run: fallow check --baseline fallow-baseline.json --fail-on-issues
```

### Output formats

| Format | Use case |
|:-------|:---------|
| `human` | Colored terminal output with grouped categories |
| `json` | Machine-readable, full result data |
| `sarif` | SARIF 2.1.0 — GitHub Code Scanning, VS Code SARIF Viewer |
| `compact` | One issue per line — `grep`, `awk`, shell scripts |

## How it works

```
                      ┌─────────────────────┐
                      │   fallow.toml +      │
                      │   package.json       │
                      └─────────┬───────────┘
                                │
                      ┌─────────▼───────────┐
                      │   File Discovery     │
                      │   (ignore crate +    │
                      │    workspace-aware)  │
                      └─────────┬───────────┘
                                │
                ┌───────────────┼───────────────┐
                │               │               │
         ┌──────▼──────┐ ┌─────▼──────┐ ┌──────▼──────┐
         │  Parse (oxc) │ │ Parse (oxc) │ │ Parse (oxc) │  ← rayon threads
         │  thread-local│ │ thread-local│ │ thread-local│    with own allocators
         │  allocator   │ │ allocator   │ │ allocator   │
         └──────┬──────┘ └─────┬──────┘ └──────┬──────┘
                │               │               │
                └───────────────┼───────────────┘
                                │
                      ┌─────────▼───────────┐
                      │  Module Resolution   │
                      │  (oxc_resolver)      │
                      └─────────┬───────────┘
                                │
                      ┌─────────▼───────────┐
                      │  Graph Construction  │
                      │  (flat Vec<Edge>     │
                      │   + range indices)   │
                      └─────────┬───────────┘
                                │
                      ┌─────────▼───────────┐
                      │  Re-export Chain     │
                      │  Resolution          │
                      │  (iterative, max 20  │
                      │   rounds, cycle-safe)│
                      └─────────┬───────────┘
                                │
                      ┌─────────▼───────────┐
                      │  BFS Reachability    │
                      │  from entry points   │
                      │  (FixedBitSet)       │
                      └─────────┬───────────┘
                                │
                      ┌─────────▼───────────┐
                      │  Dead Code Detection │
                      │  (10 issue types)    │
                      └─────────────────────┘
```

**Key design decisions:**

1. **No TypeScript compiler.** knip's bottleneck is `ts.createProgram()`. Dead code detection is a graph problem on import/export edges — you don't need type information. Oxc gives us a full AST at native speed.

2. **Declarative framework presets.** knip's 140+ plugins are mostly `{ entry: ["glob"], project: ["glob"] }` wrapped in JavaScript. Fallow expresses the same thing as TOML data. Adding a framework is editing a struct, not writing a plugin.

3. **Flat edge storage.** Module graph edges live in a single contiguous `Vec<Edge>`. Each node stores a `Range<usize>` into this vec. This is dramatically more cache-friendly than `HashMap<NodeId, Vec<Edge>>` and matters when traversing graphs with tens of thousands of edges.

4. **Thread-local Oxc allocators.** Each rayon thread owns its own `oxc_allocator::Allocator`. No Arc, no Mutex, no contention during the parsing phase.

5. **Iterative re-export resolution.** Barrel files (`index.ts` re-exporting from submodules) create chains that need to be resolved transitively. Fallow propagates usage through these chains iteratively with cycle detection, up to 20 rounds.

## Comparison with knip

| | fallow | knip |
|:--|:-------|:-----|
| Language | Rust | TypeScript |
| Parser | Oxc (syntactic only) | TypeScript compiler |
| Parallelism | rayon (all cores) | Single-threaded |
| Speed | **10–100x faster** | Baseline |
| Watch mode | Yes | No |
| Auto-fix | Yes | No |
| LSP server | Yes | No |
| Incremental cache | Yes | No |
| Git-aware analysis | Yes | No |
| Baseline comparison | Yes | No |
| SARIF output | Yes | No |
| Framework presets | 17 | 140+ |
| Detection types | 10 | 10 |

fallow intentionally covers the same 10 detection types as knip. The difference is in speed, developer experience, and CI integration — not in what it detects.

Knip has broader framework coverage (140+ plugins). Fallow covers the 17 most common frameworks. If your framework isn't supported, you can [add a custom preset](#custom-framework-presets) in `fallow.toml`.

## Building from source

```bash
git clone https://github.com/bartwaardenburg/fallow
cd fallow
cargo install --path crates/cli
```

```bash
cargo test --workspace           # Run all tests
cargo clippy --workspace         # Lint
cargo bench -p fallow-core       # Run benchmarks
```

## License

MIT
