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
</p>

---

Finds unused files, exports, dependencies, and types — plus duplicated code blocks across your entire codebase. Dead code and duplication increase bundle sizes, slow CI, and make codebases harder to navigate. fallow finds both in seconds, not minutes. 3-40x faster than [knip](https://knip.dev) for dead code analysis, 4-75x faster than [jscpd](https://github.com/kucherenko/jscpd) for duplication detection, with no Node.js runtime dependency.

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

`fallow dupes` finds copy-pasted code blocks across your entire codebase — one tool for both dead code and duplication, no separate jscpd/CPD setup needed. 4-75x faster than jscpd on real-world projects.

```bash
fallow dupes                    # Default: mild mode
fallow dupes --mode semantic    # Catch clones with renamed variables
fallow dupes --skip-local       # Only cross-directory duplicates
fallow dupes --threshold 5      # Fail CI if duplication exceeds 5%
fallow dupes --save-baseline    # Save current duplication as baseline
fallow dupes --baseline         # Fail only on new duplication vs baseline
```

| Mode | What it catches |
|:-----|:----------------|
| **strict** | Exact token-for-token clones |
| **mild** | Default — normalizes syntax variations |
| **weak** | Clones with different string literal values |
| **semantic** | Clones with renamed variables and different literals |

Clone groups sharing the same file set are grouped into **clone families** with refactoring suggestions (extract function or module).

## Benchmarks

Measured on real-world open-source projects (median of 5 runs, 2 warmup). fallow v0.3.0 vs knip v5.x.

| Project | Files | fallow | knip | Speedup |
|:--------|------:|-------:|-----:|--------:|
| [zod](https://github.com/colinhacks/zod) | 174 | **16ms** | 582ms | **35.8x** |
| [fastify](https://github.com/fastify/fastify) | 286 | **20ms** | 804ms | **39.4x** |
| [preact](https://github.com/preactjs/preact) | 244 | **29ms** | 771ms | **26.6x** |

fallow uses the [Oxc](https://oxc.rs) parser for syntactic analysis and [rayon](https://github.com/rayon-rs/rayon) for parallel parsing — no TypeScript compiler, no Node.js runtime. Dead code detection is a graph problem on import/export edges; you don't need type information for that.

### Duplication detection: fallow dupes vs jscpd

| Project | Files | fallow | jscpd | Speedup |
|:--------|------:|-------:|------:|--------:|
| [zod](https://github.com/colinhacks/zod) | 174 | **45ms** | 905ms | **20.1x** |
| [fastify](https://github.com/fastify/fastify) | 286 | **546ms** | 2.00s | **3.7x** |
| [preact](https://github.com/preactjs/preact) | 244 | **18ms** | 1.32s | **73.8x** |

fallow dupes uses a suffix array with LCP for clone detection — no quadratic pairwise comparison.

<details>
<summary>Reproduce these benchmarks</summary>

```bash
cd benchmarks
npm install
node download-fixtures.mjs    # Clone real-world projects
node bench.mjs                # Run dead code benchmarks
node bench-dupes.mjs          # Run duplication benchmarks
```

</details>

## Comparison with knip

| | fallow | knip |
|:--|:-------|:-----|
| Speed (real-world) | **3-40x faster** | Baseline |
| Dead code detection | 10 issue types | Comparable |
| Duplication detection | Built-in | Not included |
| Framework plugins | 40 (20 with config parsing) | 140+ (runtime config loading) |
| Runtime dependency | None (standalone binary) | Node.js |
| Config format | JSONC, JSON, TOML | JSON |

knip is a good tool with broader framework coverage. fallow covers the most popular frameworks and adds speed, duplication detection, git-aware analysis (`--changed-since`), baseline comparison (`--baseline`), and SARIF output for GitHub Code Scanning.

## Comparison with jscpd

| | fallow | jscpd |
|:--|:-------|:------|
| Speed (real-world) | **4-75x faster** | Baseline |
| Detection modes | 4 (strict, mild, weak, semantic) | 1 (token-based) |
| Algorithm | Suffix array with LCP | Rabin-Karp rolling hash |
| Dead code integration | Built-in (`fallow check`) | Not included |
| Runtime dependency | None (standalone binary) | Node.js |
| Config format | JSONC, JSON, TOML | JSON |

jscpd is a mature, well-established duplication detector. fallow dupes offers significantly faster performance via suffix arrays instead of pairwise comparison, semantic-aware detection modes (renamed variables, different literals), and the convenience of a single tool for both dead code and duplication analysis.

## Configuration

Create a config file in your project root, or run `fallow init`:

```jsonc
// fallow.jsonc
{
  "$schema": "https://raw.githubusercontent.com/fallow-rs/fallow/main/schema.json",
  "entry": ["src/workers/*.ts", "scripts/*.ts"],
  "ignore": ["**/*.generated.ts", "**/*.d.ts"],
  "ignore_dependencies": ["autoprefixer", "@types/node"],
  "detect": {
    "unused_files": true,
    "unused_exports": true,
    "unused_dependencies": true,
    "unused_types": true,
    "duplicate_exports": true
  },
  // Per-issue-type severity: "error" (fail CI), "warn" (report only), "off" (ignore)
  "rules": {
    "unused_files": "error",
    "unused_exports": "warn",
    "unused_types": "off",
    "unresolved_imports": "error"
  }
}
```

TOML is also supported (`fallow init --toml` creates `fallow.toml`). See the [full configuration reference](https://github.com/fallow-rs/fallow/wiki/Configuration) for all options, including `rules` severity levels, `duplicates` settings, `ignore_exports` rules, and custom framework presets.

## Framework support

40 built-in plugins covering frameworks (Next.js, Nuxt, Remix, Astro, Angular, React Router, React Native, Expo, NestJS, Docusaurus), bundlers (Vite, Webpack, Rollup, Tsup), testing (Vitest, Jest, Playwright, Cypress, Mocha, Ava, Storybook), linting (ESLint, Biome, Stylelint, Commitlint), transpilation (TypeScript, Babel), CSS (Tailwind, PostCSS), databases (Prisma, Drizzle, Knex), monorepos (Turborepo, Nx, Changesets), CI/CD (semantic-release), deployment (Wrangler, Sentry), and more (GraphQL Codegen, MSW). If your framework isn't listed, you can add a [custom preset](https://github.com/fallow-rs/fallow/wiki/Custom-Presets) in your config file.

## CI integration

```yaml
# GitHub Action — posts job summary, uploads SARIF to Code Scanning
- uses: fallow-rs/fallow@v0
  with:
    format: sarif

# Or run directly
- run: npx fallow check --format sarif > results.sarif
```

Supports `--changed-since main` for PR-only analysis, `--baseline` for failing only on new issues, `--format json` for machine-readable output, and per-issue-type severity rules (`error`/`warn`/`off`) for incremental adoption. See the [CI guide](https://github.com/fallow-rs/fallow/wiki/CI-Integration) for full workflow examples.

## Additional features

- **Rules system** — per-issue-type severity (`error`/`warn`/`off`) for incremental CI adoption
- **Inline suppression** — `// fallow-ignore-next-line` and `// fallow-ignore-file` comments to suppress individual findings
- **Watch mode** — `fallow watch` re-analyzes on file changes
- **Auto-fix** — `fallow fix` removes unused exports and dependencies (`--dry-run` to preview)
- **LSP server** — real-time diagnostics and "remove unused export" code actions in your editor
- **Workspace support** — npm, yarn, and pnpm workspaces (including `pnpm-workspace.yaml`)
- **Script binary analysis** — parses `package.json` scripts to detect CLI tool usage, reducing false positives in unused dependency detection
- **Dynamic import resolution** — partial resolution of template literals, `import.meta.glob`, and `require.context`
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

fallow uses syntactic analysis only — no type information. This is what makes it fast, but it means type-level dead code is out of scope, and some edge cases (Svelte `export let` props, Vue/Svelte template-only imports) may produce false positives. Use [inline suppression comments](#inline-suppression-comments) or [`ignore_exports`](https://github.com/fallow-rs/fallow/wiki/Configuration#ignoring-specific-exports) to suppress these.

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

- [Documentation](https://github.com/fallow-rs/fallow/wiki)
- [Migrating from knip](https://github.com/fallow-rs/fallow/wiki/Migrating-from-Knip)
- [Full plugin list](https://github.com/fallow-rs/fallow/wiki/Frameworks)
- [Plugin Authoring Guide](docs/plugin-authoring.md)

## Contributing

Missing a framework plugin? Found a false positive? [Open an issue](https://github.com/fallow-rs/fallow/issues).

```bash
cargo build --workspace && cargo test --workspace
```

## License

MIT
