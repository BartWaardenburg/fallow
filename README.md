<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/fallow-rs/fallow/main/assets/logo-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/fallow-rs/fallow/main/assets/logo.svg">
    <img src="https://raw.githubusercontent.com/fallow-rs/fallow/main/assets/logo.svg" alt="fallow" width="290">
  </picture>
</p>

<p align="center">
  <strong>Find dead code, duplication, and complexity hotspots in TypeScript & JavaScript.</strong><br>
  <strong>Rust-native. Zero config. Sub-second.</strong>
</p>

<p align="center">
  <a href="https://github.com/fallow-rs/fallow/actions/workflows/ci.yml"><img src="https://github.com/fallow-rs/fallow/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://crates.io/crates/fallow-cli"><img src="https://img.shields.io/crates/v/fallow-cli.svg" alt="crates.io"></a>
  <a href="https://www.npmjs.com/package/fallow"><img src="https://img.shields.io/npm/v/fallow.svg" alt="npm"></a>
  <a href="https://github.com/fallow-rs/fallow/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License"></a>
  <a href="https://docs.fallow.tools"><img src="https://img.shields.io/badge/docs-docs.fallow.tools-blue.svg" alt="Documentation"></a>
</p>

---

```bash
npx fallow
```

```
 Dead code   3 unused files, 12 unused exports, 2 unused deps       18ms
 Duplication 4 clone groups (2.1% of codebase)                      31ms
 Complexity  7 functions exceed thresholds                           4ms
 Total       26 issues across 847 files                             53ms
```

84 framework plugins. No Node.js runtime. No config file needed.

## Install

```bash
npx fallow                  # Run without installing
npm install -g fallow       # Or install globally (macOS, Linux, Windows)
cargo install fallow-cli    # Or via Cargo
```

## Commands

```bash
fallow                      # Run all three analyses
fallow dead-code            # Dead code only
fallow dupes                # Duplication only
fallow health               # Complexity only
fallow fix --dry-run        # Preview auto-removal of dead exports and deps
fallow watch                # Re-analyze on file changes
```

## Dead code

Finds unused files, exports, dependencies, types, enum members, class members, unresolved imports, unlisted dependencies, duplicate exports, circular dependencies, and type-only dependencies.

```bash
fallow dead-code                          # All dead code issues
fallow dead-code --unused-exports         # Only unused exports
fallow dead-code --circular-deps          # Only circular dependencies
fallow dead-code --production             # Exclude test/dev files
fallow dead-code --changed-since main     # Only changed files (for PRs)
```

## Duplication

Finds copy-pasted code blocks across your codebase. Suffix-array algorithm -- no quadratic pairwise comparison.

```bash
fallow dupes                              # Default (mild mode)
fallow dupes --mode semantic              # Catch clones with renamed variables
fallow dupes --skip-local                 # Only cross-directory duplicates
fallow dupes --trace src/utils.ts:42      # Show all clones of code at this location
```

Four detection modes: **strict** (exact tokens), **mild** (default, AST-based), **weak** (different string literals), **semantic** (renamed variables and literals).

## Complexity

Surfaces the most complex functions in your codebase and identifies where to spend refactoring effort.

```bash
fallow health                             # Functions exceeding thresholds
fallow health --top 20                    # 20 most complex functions
fallow health --file-scores               # Per-file maintainability index (0-100)
fallow health --hotspots                  # Riskiest files (git churn x complexity)
fallow health --targets                   # Ranked refactoring recommendations
fallow health --changed-since main        # Only changed files
```

## CI integration

```yaml
# GitHub Action
- uses: fallow-rs/fallow@v1

# Or run directly
- run: npx fallow --ci
```

`--ci` enables SARIF output, quiet mode, and non-zero exit on issues. Also supports:

- `--changed-since main` -- analyze only files touched in a PR
- `--baseline` / `--save-baseline` -- fail only on **new** issues
- `--format sarif` -- upload to GitHub Code Scanning
- `--format json` / `--format markdown` -- for custom workflows
Adopt incrementally -- surface issues without blocking CI, then promote when ready:

```jsonc
{ "rules": { "unused-files": "error", "unused-exports": "warn", "circular-dependencies": "off" } }
```

## Configuration

Works out of the box. When you need to customize, create `.fallowrc.json` or run `fallow init`:

```jsonc
// .fallowrc.json
{
  "$schema": "https://raw.githubusercontent.com/fallow-rs/fallow/main/schema.json",
  "entry": ["src/workers/*.ts", "scripts/*.ts"],
  "ignorePatterns": ["**/*.generated.ts"],
  "ignoreDependencies": ["autoprefixer"],
  "rules": {
    "unused-files": "error",
    "unused-exports": "warn",
    "unused-types": "off"
  },
  "health": {
    "maxCyclomatic": 20,
    "maxCognitive": 15
  }
}
```

TOML also supported (`fallow init --toml`). Migrating from knip or jscpd? Run `fallow migrate`.

See the [full configuration reference](https://docs.fallow.tools/configuration/overview) for all options.

## Framework plugins

84 built-in plugins detect entry points and used exports for your framework automatically.

| Category | Plugins |
|---|---|
| **Frameworks** | Next.js, Nuxt, Remix, SvelteKit, Gatsby, Astro, Angular, NestJS, Expo, Electron, and more |
| **Bundlers** | Vite, Webpack, Rspack, Rsbuild, Rollup, Rolldown, Tsup, Tsdown, Parcel |
| **Testing** | Vitest, Jest, Playwright, Cypress, Storybook, Mocha, Ava |
| **Databases** | Prisma, Drizzle, Knex, TypeORM, Kysely |
| **Monorepos** | Turborepo, Nx, Changesets, Syncpack |

[Full plugin list](https://docs.fallow.tools/frameworks/built-in) -- missing one? Add a [custom plugin](https://docs.fallow.tools/frameworks/custom-plugins) or [open an issue](https://github.com/fallow-rs/fallow/issues).

## Editor & AI support

- **VS Code extension** -- tree views, status bar, one-click fixes, auto-download LSP binary ([Marketplace](https://github.com/fallow-rs/fallow/tree/main/editors/vscode))
- **LSP server** -- real-time diagnostics, hover info, code actions, Code Lens with reference counts
- **MCP server** -- AI agent integration for Claude Code, Cursor, Windsurf ([fallow-skills](https://github.com/fallow-rs/fallow-skills))

## Performance

Benchmarked on real open-source projects (median of 5 runs, Apple M5).

### Dead code: fallow vs knip

| Project | Files | fallow | knip v5 | knip v6 | vs v5 | vs v6 |
|:--------|------:|-------:|--------:|--------:|------:|------:|
| [zod](https://github.com/colinhacks/zod) | 174 | **19ms** | 639ms | 334ms | 34x | 18x |
| [fastify](https://github.com/fastify/fastify) | 286 | **24ms** | 1.13s | 289ms | 46x | 12x |
| [TanStack/query](https://github.com/TanStack/query) | 901 | **148ms** | 2.75s | 1.41s | 19x | 10x |
| [svelte](https://github.com/sveltejs/svelte) | 3,337 | **325ms** | 1.93s | 860ms | 6x | 3x |
| [next.js](https://github.com/vercel/next.js) | 20,416 | **1.48s** | -- | -- | -- | -- |

knip errors out on next.js. fallow completes in under 2 seconds.

### Duplication: fallow vs jscpd

| Project | Files | fallow | jscpd | Speedup |
|:--------|------:|-------:|------:|--------:|
| [fastify](https://github.com/fastify/fastify) | 286 | **84ms** | 2.83s | 34x |
| [vue/core](https://github.com/vuejs/core) | 522 | **120ms** | 3.13s | 26x |
| [next.js](https://github.com/vercel/next.js) | 20,416 | **3.16s** | 24.64s | 8x |

No TypeScript compiler, no Node.js runtime. [How it works](https://docs.fallow.tools/explanations/architecture) | [Reproduce benchmarks](https://github.com/fallow-rs/fallow/tree/main/benchmarks)

## Suppressing findings

```ts
// fallow-ignore-next-line unused-export
export const keepThis = 1;

// fallow-ignore-file
// Suppress all issues in this file
```

Also supports `/** @public */` JSDoc tags for library exports consumed externally.

## Limitations

fallow uses syntactic analysis -- no type information. This is what makes it fast, but type-level dead code is out of scope. Use [inline suppression comments](#suppressing-findings) or [`ignoreExports`](https://docs.fallow.tools/configuration/overview#ignoring-specific-exports) for edge cases.

## Documentation

- [Getting started](https://docs.fallow.tools)
- [Configuration reference](https://docs.fallow.tools/configuration/overview)
- [CI integration guide](https://docs.fallow.tools/integrations/ci)
- [Migrating from knip](https://docs.fallow.tools/migration/from-knip)
- [Plugin authoring guide](https://github.com/fallow-rs/fallow/blob/main/docs/plugin-authoring.md)

## Contributing

Missing a framework plugin? Found a false positive? [Open an issue](https://github.com/fallow-rs/fallow/issues).

```bash
cargo build --workspace && cargo test --workspace
```

## License

MIT
