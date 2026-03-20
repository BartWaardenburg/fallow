<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="assets/logo.svg">
    <img src="assets/logo.svg" alt="fallow" width="290">
  </picture><br>
  <strong>Dead code and duplication analyzer for JavaScript and TypeScript, built in Rust.</strong><br><br>
  <a href="https://github.com/fallow-rs/fallow/actions/workflows/ci.yml"><img src="https://github.com/fallow-rs/fallow/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/fallow-rs/fallow/actions/workflows/coverage.yml"><img src="https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/fallow-rs/fallow/badges/coverage.json" alt="Coverage"></a>
  <a href="https://crates.io/crates/fallow-cli"><img src="https://img.shields.io/crates/v/fallow-cli.svg" alt="crates.io"></a>
  <a href="https://www.npmjs.com/package/fallow"><img src="https://img.shields.io/npm/v/fallow.svg" alt="npm"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License"></a>
  <a href="https://docs.fallow.tools"><img src="https://img.shields.io/badge/docs-docs.fallow.tools-blue.svg" alt="Documentation"></a>
</p>

---

Finds unused files, exports, dependencies, and types — plus duplicated code blocks across your entire codebase. Dead code and duplication increase bundle sizes, slow CI, and make codebases harder to navigate. fallow finds both in seconds, not minutes. 3-36x faster than [knip](https://knip.dev) v5 (2-14x faster than knip v6) for dead code analysis, 20-33x faster than [jscpd](https://github.com/kucherenko/jscpd) for duplication detection, with no Node.js runtime dependency.

```bash
npx fallow check    # Dead code analysis
npx fallow dupes    # Duplication detection
```

<p align="center">
  <img src="assets/screenshots/fallow-check-output.png" alt="Example fallow check output" width="820">
</p>

## Quick start

```bash
npx fallow check                     # Dead code — zero config, sub-second
npx fallow dupes                     # Duplication — find copy-paste clones
npx fallow dupes --mode semantic     # Catch clones with renamed variables
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
- **Unused enum members** — enum values never referenced
- **Unused class members** — class methods and properties never referenced
- **Unresolved imports** — import specifiers that cannot be resolved
- **Unlisted dependencies** — imported packages missing from `package.json`
- **Duplicate exports** — same symbol exported from multiple modules

## Code duplication

`fallow dupes` finds copy-pasted code blocks across your entire codebase — one tool for both dead code and duplication, no separate jscpd/CPD setup needed. 20-33x faster than jscpd on real-world projects.

```bash
fallow dupes                    # Default: mild mode
fallow dupes --mode semantic    # Catch clones with renamed variables
fallow dupes --skip-local       # Only cross-directory duplicates
fallow dupes --threshold 5      # Fail CI if duplication exceeds 5%
fallow dupes --save-baseline    # Save current duplication as baseline
fallow dupes --baseline         # Fail only on new duplication vs baseline
fallow dupes --trace src/utils.ts:42  # Show all clones of code at this location
```

| Mode | What it catches |
|:-----|:----------------|
| **strict** | Exact token-for-token clones |
| **mild** | Default — normalizes syntax variations |
| **weak** | Clones with different string literal values |
| **semantic** | Clones with renamed variables and different literals |

Clone groups sharing the same file set are grouped into **clone families** with refactoring suggestions (extract function or module).

## Benchmarks

Measured on real-world open-source projects (median of 5 runs, 2 warmup). Apple M5 (10 cores), macOS.

| Project | Files | fallow | knip v5 | knip v6 | vs v5 | vs v6 |
|:--------|------:|-------:|--------:|--------:|------:|------:|
| [zod](https://github.com/colinhacks/zod) | 174 | **23ms** | 590ms | 308ms | **26.1x** | **13.6x** |
| [fastify](https://github.com/fastify/fastify) | 286 | **22ms** | 804ms | 236ms | **36.2x** | **10.6x** |
| [preact](https://github.com/preactjs/preact) | 244 | **24ms** | 799ms | —* | **33.9x** | — |
| synthetic (1,000 files) | 1,000 | **45ms** | 380ms | 196ms | **8.5x** | **4.4x** |
| synthetic (5,000 files) | 5,000 | **201ms** | 646ms | 340ms | **3.2x** | **1.7x** |

\* knip v6 excluded for preact due to a v6 regression on this project.

The speedup narrows on larger projects as actual analysis time dominates over startup: 26-36x on real-world projects vs knip v5 (10-14x vs v6), 3-9x on 1,000+ file projects. fallow stays sub-second even at 5,000 files.

Memory usage is equally striking — fallow uses 10-15x less memory than knip v5 and 3-8x less than knip v6:

| Project | fallow | knip v5 | knip v6 |
|:--------|-------:|--------:|--------:|
| zod (174 files) | **20 MB** | 248 MB | 160 MB |
| fastify (286 files) | **27 MB** | 288 MB | 111 MB |
| synthetic (5,000 files) | **61 MB** | 279 MB | 179 MB |

fallow uses the [Oxc](https://oxc.rs) parser for syntactic analysis and [rayon](https://github.com/rayon-rs/rayon) for parallel parsing — no TypeScript compiler, no Node.js runtime. Dead code detection is a graph problem on import/export edges; you don't need type information for that.

### Duplication detection: fallow dupes vs jscpd

| Project | Files | fallow | jscpd | Speedup |
|:--------|------:|-------:|------:|--------:|
| [zod](https://github.com/colinhacks/zod) | 174 | **49ms** | 1.01s | **20.6x** |
| [fastify](https://github.com/fastify/fastify) | 286 | **82ms** | 2.09s | **25.5x** |
| [preact](https://github.com/preactjs/preact) | 244 | **46ms** | 1.53s | **33.3x** |

fallow dupes uses a suffix array with LCP for clone detection — no quadratic pairwise comparison.

<details>
<summary>Reproduce these benchmarks</summary>

```bash
cd benchmarks
npm install                          # knip v5, jscpd, tinybench
cd knip6 && npm install && cd ..     # knip v6 (optional, for three-way comparison)
npm run generate                     # Generate synthetic fixtures
node download-fixtures.mjs           # Clone real-world projects
node bench.mjs                       # Run dead code benchmarks (fallow vs knip v5 + v6)
node bench-dupes.mjs                 # Run duplication benchmarks (fallow vs jscpd)
```

</details>

## Comparison with knip

| | fallow | knip |
|:--|:-------|:-----|
| Speed vs knip v5 | **3-36x faster** | Baseline |
| Speed vs knip v6 | **2-14x faster** | Baseline |
| Memory usage | **3-15x less** | Baseline |
| Dead code detection | 10 issue types | Comparable |
| Duplication detection | Built-in | Not included |
| Framework plugins | 84 (30 with config parsing) | 140+ (runtime config loading) |
| Runtime dependency | None (standalone binary) | Node.js |
| Config format | JSONC, JSON, TOML | JSON |

knip is a good tool with broader framework coverage. fallow covers the most popular frameworks and adds speed, duplication detection, git-aware analysis (`--changed-since`), baseline comparison (`--baseline`), and SARIF output for GitHub Code Scanning.

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

TOML is also supported (`fallow init --toml` creates `fallow.toml`). See the [full configuration reference](https://docs.fallow.tools/configuration/overview) for all options, including `rules` severity levels, `duplicates` settings, `ignoreExports` rules, and custom framework presets.

### Migrating from knip or jscpd

If you have an existing knip or jscpd config, fallow can migrate it automatically:

```sh
fallow migrate            # Auto-detect knip/jscpd configs, write .fallowrc.json
fallow migrate --toml     # Output as TOML instead
fallow migrate --dry-run  # Preview without writing
```

This reads your knip.json/knip.jsonc/.knip.json/.knip.jsonc and/or .jscpd.json (also checks package.json for embedded configs), maps settings to fallow equivalents, and warns about any fields that can't be migrated.

## Framework support

84 built-in plugins covering frameworks (Next.js, Nuxt, Remix, SvelteKit, Gatsby, Astro, Angular, React Router, TanStack Router, React Native, Expo, NestJS, Docusaurus, Nitro, Capacitor, Sanity, VitePress, next-intl, Relay, Electron, i18next), bundlers (Vite, Webpack, Rspack, Rsbuild, Rollup, Rolldown, Tsup, Tsdown, Parcel), testing (Vitest, Jest, Playwright, Cypress, Mocha, Ava, Storybook, Karma, Cucumber, WebdriverIO), linting & formatting (ESLint, Biome, Stylelint, Commitlint, Prettier, Oxlint, markdownlint, cspell, remark), transpilation & runtime (TypeScript, Babel, SWC, Bun), CSS (Tailwind, PostCSS), databases (Prisma, Drizzle, Knex, TypeORM, Kysely), monorepos (Turborepo, Nx, Changesets, Syncpack), CI/CD (semantic-release, Commitizen), deployment (Wrangler, Sentry), git hooks (husky, lint-staged, lefthook, simple-git-hooks), and more (GraphQL Codegen, MSW, SVGO, SVGR, TypeDoc, openapi-ts, Plop, c8, nyc, nodemon, PM2, dependency-cruiser). If your framework isn't listed, you can add a [custom preset](https://docs.fallow.tools/frameworks/custom-plugins) in your config file.

## CI integration

```yaml
# GitHub Action — posts job summary, uploads SARIF to Code Scanning
- uses: fallow-rs/fallow@v0
  with:
    format: sarif

# Or run directly
- run: npx fallow check --format sarif > results.sarif
```

Supports `--changed-since main` for PR-only analysis, `--baseline` for failing only on new issues, `--format json` for machine-readable output, and per-issue-type severity rules (`error`/`warn`/`off`) for incremental adoption. See the [CI guide](https://docs.fallow.tools/integrations/ci) for full workflow examples.

## Additional features

- **Rules system** — per-issue-type severity (`error`/`warn`/`off`) for incremental CI adoption
- **Inline suppression** — `// fallow-ignore-next-line` and `// fallow-ignore-file` comments to suppress individual findings
- **Watch mode** — `fallow watch` re-analyzes on file changes
- **Auto-fix** — `fallow fix` removes unused exports and dependencies (`--dry-run` to preview)
- **VS Code extension** — tree views for dead code and duplicates, status bar, auto-download of the LSP binary, one-click fixes ([`editors/vscode`](editors/vscode))
- **LSP server** — real-time diagnostics, "remove unused export" code actions, and Code Lens with clickable reference counts above exports (opens Peek References panel)
- **Workspace support** — npm, yarn, and pnpm workspaces (including `pnpm-workspace.yaml`, content-addressable store detection, and injected dependencies) with `exports` field subpath resolution
- **Script binary analysis** — parses `package.json` scripts to detect CLI tool usage, reducing false positives in unused dependency detection
- **Dynamic import resolution** — partial resolution of template literals, `import.meta.glob`, and `require.context`
- **Non-JS file support** — Vue/Svelte SFC (`<script>` block extraction), Astro (frontmatter), MDX (import/export statements), CSS/SCSS (`@import`, `@use`, `@forward`, `@apply`/`@tailwind` as Tailwind dependency usage), CSS Modules (`.module.css`/`.module.scss` class name tracking)
- **Production mode** — `--production` excludes test/story/dev files, only considers start/build scripts, and reports type-only dependencies that could be devDependencies

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

Issue type tokens: `unused-file`, `unused-export`, `unused-type`, `unused-dependency`, `unused-dev-dependency`, `unused-enum-member`, `unused-class-member`, `unresolved-import`, `unlisted-dependency`, `duplicate-export`.

## Limitations

fallow uses syntactic analysis only — no type information. This is what makes it fast, but it means type-level dead code is out of scope. Svelte files skip individual export analysis (props can't be distinguished from utility exports without compiler semantics), so unused exports in `.svelte` files may go undetected. Use [inline suppression comments](#inline-suppression-comments) or [`ignore_exports`](https://docs.fallow.tools/configuration/overview#ignoring-specific-exports) for any remaining edge cases.

## Custom plugins

Need support for an internal framework? Create a `fallow-plugin-<name>.toml` file:

```toml
name = "my-framework"
enablers = ["my-framework"]
entry_points = ["src/routes/**/*.{ts,tsx}"]
always_used = ["src/setup.ts"]
tooling_dependencies = ["my-framework-cli"]

[[used_exports]]
pattern = "src/routes/**/*.{ts,tsx}"
exports = ["default", "loader", "action"]
```

Fallow auto-discovers `fallow-plugin-*.toml` files in your project root and `.fallow/plugins/` directory. See the [Plugin Authoring Guide](docs/plugin-authoring.md) for the full format and examples.

## Learn more

- [Documentation](https://docs.fallow.tools)
- [Migrating from knip](https://docs.fallow.tools/migration/from-knip)
- [Full plugin list](https://docs.fallow.tools/frameworks/built-in)
- [Plugin Authoring Guide](docs/plugin-authoring.md)

## Contributing

Missing a framework plugin? Found a false positive? [Open an issue](https://github.com/fallow-rs/fallow/issues).

```bash
cargo build --workspace && cargo test --workspace
```

## License

MIT
