<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/fallow-rs/fallow/main/assets/logo-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/fallow-rs/fallow/main/assets/logo.svg">
    <img src="https://raw.githubusercontent.com/fallow-rs/fallow/main/assets/logo.svg" alt="fallow" width="290">
  </picture><br>
  <strong>The codebase analyzer for TypeScript and JavaScript, built in Rust.</strong><br><br>
  <a href="https://github.com/fallow-rs/fallow/actions/workflows/ci.yml"><img src="https://github.com/fallow-rs/fallow/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/fallow-rs/fallow/actions/workflows/coverage.yml"><img src="https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/fallow-rs/fallow/badges/coverage.json" alt="Coverage"></a>
  <a href="https://crates.io/crates/fallow-cli"><img src="https://img.shields.io/crates/v/fallow-cli.svg" alt="crates.io"></a>
  <a href="https://www.npmjs.com/package/fallow"><img src="https://img.shields.io/npm/v/fallow.svg" alt="npm"></a>
  <a href="https://github.com/fallow-rs/fallow/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License"></a>
  <a href="https://docs.fallow.tools"><img src="https://img.shields.io/badge/docs-docs.fallow.tools-blue.svg" alt="Documentation"></a>
</p>

---

Unused code, circular dependencies, code duplication, and complexity hotspots. Found in seconds, not minutes. fallow analyzes your entire codebase for unused files, exports, dependencies, and types, detects circular dependencies, finds duplicated code blocks, and surfaces high-complexity functions and risky hotspots. 3-36x faster than [knip](https://knip.dev) v5 (2-14x faster than knip v6) for unused code analysis, 20-33x faster than [jscpd](https://github.com/kucherenko/jscpd) for duplication detection, with no Node.js runtime dependency.

```bash
npx fallow check    # Unused code analysis
npx fallow dupes    # Duplication detection
npx fallow health   # Complexity — find high-complexity functions
```

<p align="center">
  <img src="https://raw.githubusercontent.com/fallow-rs/fallow/main/assets/screenshots/fallow-demo.gif" alt="Example fallow check output" width="820">
</p>

## Quick start

```bash
npx fallow check                     # Unused code — zero config, sub-second
npx fallow dupes                     # Duplication — find copy-paste clones
npx fallow dupes --mode semantic     # Catch clones with renamed variables
npx fallow health                    # Complexity — find high-complexity functions
npx fallow fix --dry-run             # Preview auto-removal of dead exports and deps
```

Install globally:

```bash
npm install -g fallow        # Prebuilt binaries for macOS, Linux, Windows
cargo install fallow-cli     # Or via cargo
```

## What it finds

- **Unused files** — not reachable from any entry point
- **Unused exports** — exported symbols never imported elsewhere
- **Unused types** — type aliases and interfaces never referenced
- **Unused dependencies** — packages in `dependencies` never imported or used as script binaries
- **Unused devDependencies** — packages in `devDependencies` never imported or used as script binaries
- **Unused optionalDependencies** — packages in `optionalDependencies` never imported or used as script binaries
- **Unused enum members** — enum values never referenced
- **Unused class members** — class methods and properties never referenced (tracks instance usage: `const svc = new MyService(); svc.greet()` counts `greet` as used)
- **Unresolved imports** — import specifiers that cannot be resolved
- **Unlisted dependencies** — imported packages missing from `package.json`
- **Duplicate exports** — same symbol exported from multiple modules
- **Circular dependencies** — import cycles detected via Tarjan's SCC algorithm
- **Type-only dependencies** — production deps only used via `import type` (could be devDependencies)
- **Complexity metrics** — cyclomatic and cognitive complexity per function, with configurable thresholds
- **Function overload deduplication** — multiple overload signatures for the same function are deduplicated to avoid false positives

## Code duplication

`fallow dupes` finds copy-pasted code blocks across your entire codebase — one tool for both dead code and duplication, no separate jscpd/CPD setup needed. 20-33x faster than jscpd on real-world projects.

```bash
fallow dupes                    # Default: mild mode
fallow dupes --mode semantic    # Catch clones with renamed variables
fallow dupes --skip-local       # Only cross-directory duplicates
fallow dupes --threshold 5      # Fail CI if duplication exceeds 5%
fallow dupes --changed-since main  # Only duplication in changed files
fallow dupes --save-baseline    # Save current duplication as baseline
fallow dupes --baseline         # Fail only on new duplication vs baseline
fallow dupes --top 10                 # Show only the 10 largest clone groups
fallow dupes --trace src/utils.ts:42  # Show all clones of code at this location
```

| Mode | What it catches |
|:-----|:----------------|
| **strict** | Exact token-for-token clones |
| **mild** | Default — equivalent to strict with AST-based tokenization (whitespace/comments already absent) |
| **weak** | Clones with different string literal values |
| **semantic** | Clones with renamed variables and different literals |

Clone groups sharing the same file set are grouped into **clone families** with refactoring suggestions (extract function or module).

## Code health

`fallow health` reports cyclomatic and cognitive complexity for every function in your codebase, surfacing the most complex functions that are candidates for refactoring.

```bash
fallow health                          # Report functions exceeding default thresholds
fallow health --max-cyclomatic 15      # Custom cyclomatic complexity threshold
fallow health --max-cognitive 10       # Custom cognitive complexity threshold
fallow health --top 20                 # Show the 20 most complex functions
fallow health --sort cognitive         # Sort by cognitive complexity
fallow health --changed-since main     # Only analyze changed files
fallow health --file-scores            # Per-file maintainability index (0–100)
fallow health --hotspots               # Identify riskiest files (churn × complexity)
fallow health --hotspots --since 3m    # Hotspots from the last 3 months
fallow health --format json            # Machine-readable output
```

`--file-scores` computes a per-file maintainability index combining complexity density, dead code ratio (value exports only, excluding types), and coupling (fan-out with logarithmic scaling). Barrel files are excluded by default. Formula: `100 - (complexity_density × 30) - (dead_code_ratio × 20) - min(ln(fan_out+1) × 4, 15)`, clamped to 0–100. Higher is better.

`--hotspots` combines git churn with complexity to answer "where should we spend refactoring budget?" Score: `normalized_churn × normalized_complexity × 100` (0–100, higher = riskier). Churn uses recency-weighted commit count (half-life 90 days). Fan-in shown as "blast radius" column. Accepts `--since` (durations like `6m`/`90d`/`1y` or ISO dates, default 6 months) and `--min-commits` (default 3). Trend detection labels files as accelerating, stable, or cooling.

## Benchmarks

Measured on real-world open-source projects (median of 5 runs, 2 warmup). Apple M5 (10 cores), macOS.

| Project | Files | fallow | knip v5 | knip v6 | vs v5 | vs v6 |
|:--------|------:|-------:|--------:|--------:|------:|------:|
| [zod](https://github.com/colinhacks/zod) | 174 | **19ms** | 639ms | 334ms | **34x** | **18x** |
| [preact](https://github.com/preactjs/preact) | 244 | **20ms** | 819ms | —* | **41x** | — |
| [fastify](https://github.com/fastify/fastify) | 286 | **24ms** | 1.13s | 289ms | **46x** | **12x** |
| [vue/core](https://github.com/vuejs/core) | 522 | **63ms** | 702ms | 299ms | **11x** | **5x** |
| [TanStack/query](https://github.com/TanStack/query) | 901 | **148ms** | 2.75s | 1.41s | **19x** | **10x** |
| [svelte](https://github.com/sveltejs/svelte) | 3,337 | **325ms** | 1.93s | 860ms | **6x** | **3x** |
| [next.js](https://github.com/vercel/next.js) | 20,416 | **1.48s** | —† | —† | — | — |

\* knip v6 excluded for preact due to a v6 regression.
† knip errors out on next.js (exits without producing valid results). fallow is the only tool that completes.

6-46x faster than knip v5, 3-18x faster than knip v6 on projects where both tools complete. On the largest monorepos (20k+ files), knip errors out while fallow completes in under 2 seconds.

Memory usage is equally striking — fallow uses 4-11x less memory than knip v5 and 3-6x less than knip v6:

| Project | fallow | knip v5 | knip v6 |
|:--------|-------:|--------:|--------:|
| zod (174 files) | **21 MB** | 250 MB | 161 MB |
| fastify (286 files) | **27 MB** | 289 MB | 107 MB |
| TanStack/query (901 files) | **59 MB** | 673 MB | 354 MB |
| svelte (3,337 files) | **67 MB** | 460 MB | 243 MB |

fallow uses the [Oxc](https://oxc.rs) parser for syntactic analysis, [oxc_semantic](https://docs.rs/oxc_semantic) for scope-aware binding analysis, and [rayon](https://github.com/rayon-rs/rayon) for parallel parsing — no TypeScript compiler, no Node.js runtime. Dead code detection is a graph problem on import/export edges; you don't need type information for that.

### Duplication detection: fallow dupes vs jscpd

| Project | Files | fallow | jscpd | Speedup |
|:--------|------:|-------:|------:|--------:|
| [zod](https://github.com/colinhacks/zod) | 174 | **46ms** | 909ms | **20x** |
| [preact](https://github.com/preactjs/preact) | 244 | **44ms** | 1.33s | **30x** |
| [fastify](https://github.com/fastify/fastify) | 286 | **84ms** | 2.83s | **34x** |
| [vue/core](https://github.com/vuejs/core) | 522 | **120ms** | 3.13s | **26x** |
| [svelte](https://github.com/sveltejs/svelte) | 3,337 | **400ms** | 3.63s | **9x** |
| [next.js](https://github.com/vercel/next.js) | 20,416 | **3.16s** | 24.64s | **8x** |

8-34x faster across all project sizes. fallow dupes uses a suffix array with LCP for clone detection — no quadratic pairwise comparison.

<details>
<summary>Reproduce these benchmarks</summary>

```bash
cd benchmarks
npm install                          # knip v5, jscpd, tinybench
cd knip6 && npm install && cd ..     # knip v6 (optional, for three-way comparison)
npm run generate                     # Generate synthetic fixtures
node download-fixtures.mjs           # Clone real-world projects (8 projects)
node bench.mjs                       # Run dead code benchmarks (fallow vs knip v5 + v6)
node bench-dupes.mjs                 # Run duplication benchmarks (fallow vs jscpd)
node bench-circular.mjs              # Run circular dep benchmarks (fallow vs madge + dpdm)
```

</details>

## Comparison with knip

| | fallow | knip |
|:--|:-------|:-----|
| Speed vs knip v5 | **6-46x faster** | Baseline |
| Speed vs knip v6 | **3-18x faster** | Baseline |
| Memory usage | **3-11x less** | Baseline |
| Dead code detection | 13 issue types | Comparable |
| Complexity analysis | Built-in (cyclomatic + cognitive, file scores, hotspots) | Not included |
| Duplication detection | Built-in | Not included |
| Framework plugins | 84 (31 with config parsing) | 140+ (runtime config loading) |
| Runtime dependency | None (standalone binary) | Node.js |
| Config format | JSONC, JSON, TOML | JSON |

knip is a good tool with broader framework coverage. fallow covers the most popular frameworks and adds speed, duplication detection, complexity analysis with hotspot detection, git-aware analysis (`--changed-since`), baseline comparison (`--baseline`), and SARIF output for GitHub Code Scanning.

## Comparison with jscpd

| | fallow | jscpd |
|:--|:-------|:------|
| Speed (real-world) | **20-33x faster** | Baseline |
| Detection modes | 4 (strict, mild, weak, semantic) | 1 (token-based) |
| Algorithm | Suffix array with LCP | Rabin-Karp rolling hash |
| Dead code integration | Built-in (`fallow check`) | Not included |
| Runtime dependency | None (standalone binary) | Node.js |
| Config format | JSONC, JSON, TOML | JSON |

jscpd is a mature, well-established duplication detector. fallow dupes offers significantly faster performance via suffix arrays instead of pairwise comparison, semantic-aware detection modes (renamed variables, different literals), and the convenience of a single tool for both dead code and duplication analysis.

## Configuration

Create a config file in your project root, or run `fallow init`:

```jsonc
// .fallowrc.json
{
  "$schema": "https://raw.githubusercontent.com/fallow-rs/fallow/main/schema.json",
  "entry": ["src/workers/*.ts", "scripts/*.ts"],
  "ignorePatterns": ["**/*.generated.ts", "**/*.d.ts"],
  "ignoreDependencies": ["autoprefixer", "@types/node"],
  // Per-issue-type severity: "error" (fail CI), "warn" (report only), "off" (ignore)
  "rules": {
    "unused-files": "error",
    "unused-exports": "warn",
    "unused-types": "off",
    "unresolved-imports": "error"
  }
}
```

Complexity thresholds for `fallow health` can be configured in your config file:

```jsonc
// .fallowrc.json
{
  "health": {
    "maxCyclomatic": 20,
    "maxCognitive": 15,
    "ignore": ["src/generated/**"]
  }
}
```

Or in TOML:

```toml
[health]
maxCyclomatic = 20
maxCognitive = 15
ignore = ["src/generated/**"]
```

TOML is also supported (`fallow init --toml` creates `fallow.toml`). See the [full configuration reference](https://docs.fallow.tools/configuration/overview) for all options, including `rules` severity levels, `duplicates` settings, `health` thresholds, `ignoreExports` rules, and custom framework presets.

### Migrating from knip or jscpd

If you have an existing knip or jscpd config, fallow can migrate it automatically:

```sh
fallow migrate            # Auto-detect knip/jscpd configs, write .fallowrc.json
fallow migrate --toml     # Output as TOML instead
fallow migrate --dry-run  # Preview without writing
```

This reads your knip.json/knip.jsonc/.knip.json/.knip.jsonc and/or .jscpd.json (also checks package.json for embedded configs), maps settings to fallow equivalents, and warns about any fields that can't be migrated.

## Framework support

84 built-in plugins — if your framework isn't listed, you can add a [custom preset](https://docs.fallow.tools/frameworks/custom-plugins) in your config file.

| Category | Plugins |
|---|---|
| **Frameworks** | Next.js, Nuxt, Remix, SvelteKit, Gatsby, Astro, Angular, React Router, TanStack Router, React Native, Expo, NestJS, Docusaurus, Nitro, Capacitor, Sanity, VitePress, next-intl, Relay, Electron, i18next |
| **Bundlers** | Vite, Webpack, Rspack, Rsbuild, Rollup, Rolldown, Tsup, Tsdown, Parcel |
| **Testing** | Vitest, Jest, Playwright, Cypress, Mocha, Ava, Storybook, Karma, Cucumber, WebdriverIO |
| **Linting & Formatting** | ESLint, Biome, Stylelint, Prettier, Oxlint, markdownlint, cspell, remark |
| **Transpilation & Language** | TypeScript, Babel, SWC |
| **CSS** | Tailwind, PostCSS |
| **Databases** | Prisma, Drizzle, Knex, TypeORM, Kysely |
| **Monorepos** | Turborepo, Nx, Changesets, Syncpack |
| **CI/CD & Release** | Commitlint, Commitizen, semantic-release |
| **Runtime** | Bun |
| **Deployment** | Wrangler, Sentry |
| **Git Hooks** | husky, lint-staged, lefthook, simple-git-hooks |
| **Code Generation & Docs** | GraphQL Codegen, TypeDoc, openapi-ts, Plop |
| **Media & Assets** | SVGO, SVGR |
| **Coverage** | c8, nyc |
| **Other** | MSW, nodemon, PM2, dependency-cruiser |

## CI integration

```yaml
# GitHub Action — posts job summary, uploads SARIF to Code Scanning
- uses: fallow-rs/fallow@v1
  with:
    format: sarif

# Or run directly
- run: npx fallow check --format sarif > results.sarif

# Or run directly with CI mode
- run: npx fallow check --ci
```

Supports `--changed-since main` for PR-only analysis, `--baseline` for failing only on new issues, `--format json` for machine-readable output, `--format markdown` for PR comment workflows, and per-issue-type severity rules (`error`/`warn`/`off`) for incremental adoption. See the [CI guide](https://docs.fallow.tools/integrations/ci) for full workflow examples.

## Additional features

- **Rules system** — per-issue-type severity (`error`/`warn`/`off`) for incremental CI adoption
- **Inline suppression** — `// fallow-ignore-next-line` and `// fallow-ignore-file` comments to suppress individual findings
- **Watch mode** — `fallow watch` re-analyzes on file changes
- **Auto-fix** — `fallow fix` removes unused exports, dependencies, and enum members (`--dry-run` to preview)
- **VS Code extension** — tree views for unused code and duplicates, status bar, auto-download of the LSP binary, one-click fixes ([`editors/vscode`](https://github.com/fallow-rs/fallow/tree/main/editors/vscode))
- **LSP server** — real-time diagnostics, hover information, "remove unused export" code actions, and Code Lens with clickable reference counts above exports (opens Peek References panel)
- **Workspace support** — npm, yarn, and pnpm workspaces (including `pnpm-workspace.yaml`, content-addressable store detection, and injected dependencies) with `exports` field subpath resolution. TypeScript project references (`tsconfig.json` `references`) are also discovered as workspaces
- **Script binary analysis** — parses `package.json` scripts to detect CLI tool usage, reducing false positives in unused dependency detection
- **Dynamic import resolution** — partial resolution of template literals, `import.meta.glob`, and `require.context`
- **Non-JS file support** — Vue/Svelte SFC (`<script>` block extraction), Astro (frontmatter), MDX (import/export statements), CSS/SCSS (`@import`, `@use`, `@forward`, `@apply`/`@tailwind` as Tailwind dependency usage), CSS Modules (`.module.css`/`.module.scss` class name tracking)
- **Production mode** — `--production` excludes test/story/dev files, only considers start/build scripts, and reports type-only dependencies that could be devDependencies
- **Circular dependency detection** — finds import cycles using Tarjan's SCC algorithm; configurable via `"circular-dependencies"` rule. Unique feature not available in knip.
- **Complexity metrics** — `fallow health` reports cyclomatic and cognitive complexity per function with configurable thresholds (`--max-cyclomatic`, `--max-cognitive`), top-N ranking (`--top`), and `--changed-since` for PR-scoped analysis. `--file-scores` adds per-file maintainability index combining complexity density, dead code ratio, and fan-out
- **JSDoc `@public` tag** — exports annotated with `/** @public */` are never reported as unused, for library authors whose exports are consumed by external projects

## Inline suppression comments

Suppress specific findings directly in source code — useful for false positives or intentional exceptions:

```ts
// fallow-ignore-next-line
export const keepThis = 1;

// fallow-ignore-next-line unused-export
export const keepThisToo = 2;

// fallow-ignore-file unused-export
// Suppresses all unused-export findings in this file
```

| Comment | Effect |
|:--------|:-------|
| `// fallow-ignore-next-line` | Suppress all issues on the next line |
| `// fallow-ignore-next-line unused-export` | Suppress a specific issue type on the next line |
| `// fallow-ignore-file` | Suppress all issues in the file |
| `// fallow-ignore-file unused-export` | Suppress a specific issue type for the file |

Issue type tokens: `unused-file`, `unused-export`, `unused-type`, `unused-dependency`, `unused-dev-dependency`, `unused-optional-dependency`, `unused-enum-member`, `unused-class-member`, `unresolved-import`, `unlisted-dependency`, `duplicate-export`, `circular-dependency`, `type-only-dependency`, `code-duplication`.

## Limitations

fallow uses syntactic analysis only — no type information. This is what makes it fast, but it means type-level dead code is out of scope. Svelte files skip individual export analysis (props can't be distinguished from utility exports without compiler semantics), so unused exports in `.svelte` files may go undetected. Use [inline suppression comments](#inline-suppression-comments) or [`ignore_exports`](https://docs.fallow.tools/configuration/overview#ignoring-specific-exports) for any remaining edge cases.

## Custom plugins

Need support for an internal framework? Create a `fallow-plugin-<name>.jsonc` file:

```jsonc
{
  "$schema": "https://raw.githubusercontent.com/fallow-rs/fallow/main/plugin-schema.json",
  "name": "my-framework",
  "enablers": ["my-framework"],
  "entryPoints": ["src/routes/**/*.{ts,tsx}"],
  "alwaysUsed": ["src/setup.ts"],
  "toolingDependencies": ["my-framework-cli"],
  "usedExports": [
    { "pattern": "src/routes/**/*.{ts,tsx}", "exports": ["default", "loader", "action"] }
  ]
}
```

Fallow auto-discovers `fallow-plugin-*.{jsonc,json,toml}` files in your project root and `.fallow/plugins/` directory. See the [Plugin Authoring Guide](https://github.com/fallow-rs/fallow/blob/main/docs/plugin-authoring.md) for the full format and examples.

## Learn more

- [Documentation](https://docs.fallow.tools)
- [Migrating from knip](https://docs.fallow.tools/migration/from-knip)
- [Full plugin list](https://docs.fallow.tools/frameworks/built-in)
- [Plugin Authoring Guide](https://github.com/fallow-rs/fallow/blob/main/docs/plugin-authoring.md)
- [Agent Skills](https://github.com/fallow-rs/fallow-skills) — Codebase analysis skills for Claude Code, Cursor, Windsurf, and other AI agents

## Contributing

Missing a framework plugin? Found a false positive? [Open an issue](https://github.com/fallow-rs/fallow/issues).

```bash
cargo build --workspace && cargo test --workspace
```

## License

MIT
