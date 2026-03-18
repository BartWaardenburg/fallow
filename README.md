<p align="center">
  <img src="assets/logo.png" alt="fallow" width="460"><br>
  <strong>Dead code and duplication analyzer for JavaScript and TypeScript, built in Rust.</strong><br><br>
  <a href="https://github.com/BartWaardenburg/fallow/actions/workflows/ci.yml"><img src="https://github.com/BartWaardenburg/fallow/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/BartWaardenburg/fallow/actions/workflows/coverage.yml"><img src="https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/BartWaardenburg/fallow/badges/coverage.json" alt="Coverage"></a>
  <a href="https://crates.io/crates/fallow-cli"><img src="https://img.shields.io/crates/v/fallow-cli.svg" alt="crates.io"></a>
  <a href="https://www.npmjs.com/package/fallow"><img src="https://img.shields.io/npm/v/fallow.svg" alt="npm"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License"></a>
</p>

---

Finds unused files, exports, dependencies, and types — plus duplicated code blocks across your entire codebase. Dead code and duplication increase bundle sizes, slow CI, and make codebases harder to navigate. fallow finds both in seconds, not minutes. 25-40x faster than [knip](https://knip.dev) for dead code analysis, 4-70x faster than [jscpd](https://github.com/kucherenko/jscpd) for duplication detection, with no Node.js runtime dependency.

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
- **Unused dependencies** — packages in `dependencies` never imported
- **Unused devDependencies** — packages in `devDependencies` never imported
- **Unused enum members** — enum values never referenced
- **Unused class members** — class methods and properties never referenced
- **Unresolved imports** — import specifiers that cannot be resolved
- **Unlisted dependencies** — imported packages missing from `package.json`
- **Duplicate exports** — same symbol exported from multiple modules

## Code duplication

`fallow dupes` finds copy-pasted code blocks across your entire codebase — one tool for both dead code and duplication, no separate jscpd/CPD setup needed. 4-70x faster than jscpd on real-world projects.

```bash
fallow dupes                    # Default: mild mode
fallow dupes --mode semantic    # Catch clones with renamed variables
fallow dupes --skip-local       # Only cross-directory duplicates
fallow dupes --threshold 5      # Fail CI if duplication exceeds 5%
```

| Mode | What it catches |
|:-----|:----------------|
| **strict** | Exact token-for-token clones |
| **mild** | Default — normalizes syntax variations |
| **weak** | Clones with different string literal values |
| **semantic** | Clones with renamed variables and different literals |

## Benchmarks

Measured on real-world open-source projects (median of 5 runs, 2 warmup). fallow v0.2.0 vs knip v5.x.

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
| Speed (real-world) | **25-40x faster** | Baseline |
| Dead code detection | 10 issue types | Comparable |
| Duplication detection | Built-in | Not included |
| Framework plugins | 23 (AST-based config parsing) | 140+ (pattern-based) |
| Runtime dependency | None (standalone binary) | Node.js |
| Config format | TOML | JSON |

knip is a good tool with broader framework coverage. fallow covers the most popular frameworks and adds speed, duplication detection, git-aware analysis (`--changed-since`), baseline comparison (`--baseline`), and SARIF output for GitHub Code Scanning.

## Comparison with jscpd

| | fallow | jscpd |
|:--|:-------|:------|
| Speed (real-world) | **4-70x faster** | Baseline |
| Detection modes | 4 (strict, mild, weak, semantic) | 1 (token-based) |
| Algorithm | Suffix array with LCP | Rabin-Karp rolling hash |
| Dead code integration | Built-in (`fallow check`) | Not included |
| Runtime dependency | None (standalone binary) | Node.js |
| Config format | TOML | JSON |

jscpd is a mature, well-established duplication detector. fallow dupes offers significantly faster performance via suffix arrays instead of pairwise comparison, semantic-aware detection modes (renamed variables, different literals), and the convenience of a single tool for both dead code and duplication analysis.

## Configuration

Create `fallow.toml` in your project root, or run `fallow init`:

```toml
entry = ["src/workers/*.ts", "scripts/*.ts"]
ignore = ["**/*.generated.ts", "**/*.d.ts"]
ignore_dependencies = ["autoprefixer", "@types/node"]

[detect]
unused_files = true
unused_exports = true
unused_dependencies = true
unused_types = true
duplicate_exports = true
```

See the [full configuration reference](https://github.com/BartWaardenburg/fallow/wiki/Configuration) for all options, including `[dupes]` settings, `[[ignore_exports]]` rules, and custom framework presets.

## Framework support

Built-in support for Next.js, Vite, Vitest, Jest, Storybook, Remix, Astro, Nuxt, Angular, Playwright, Cypress, Prisma, ESLint, TypeScript, Webpack, Tailwind CSS, React Router, React Native, Expo, Sentry, Drizzle, Knex, and MSW. If your framework isn't listed, you can add a [custom preset](https://github.com/BartWaardenburg/fallow/wiki/Custom-Presets) in `fallow.toml`.

## CI integration

```yaml
# GitHub Action — posts job summary, uploads SARIF to Code Scanning
- uses: BartWaardenburg/fallow@v0
  with:
    format: sarif

# Or run directly
- run: npx fallow check --format sarif > results.sarif
```

Supports `--changed-since main` for PR-only analysis, `--baseline` for failing only on new issues, and `--format json` for machine-readable output. See the [CI guide](https://github.com/BartWaardenburg/fallow/wiki/CI-Integration) for full workflow examples.

## Additional features

- **Watch mode** — `fallow watch` re-analyzes on file changes
- **Auto-fix** — `fallow fix` removes unused exports and dependencies (`--dry-run` to preview)
- **LSP server** — real-time diagnostics and "remove unused export" code actions in your editor
- **Workspace support** — npm, yarn, and pnpm workspaces (including `pnpm-workspace.yaml`)
- **Dynamic import resolution** — partial resolution of template literals, `import.meta.glob`, and `require.context`

## Limitations

fallow uses syntactic analysis only — no type information. This is what makes it fast, but it means type-level dead code is out of scope, and some edge cases (Svelte `export let` props, Vue/Svelte template-only imports) may produce false positives. See [`ignore_exports`](https://github.com/BartWaardenburg/fallow/wiki/Configuration#ignoring-specific-exports) to suppress these.

## Learn more

- [Documentation](https://github.com/BartWaardenburg/fallow/wiki)
- [Migrating from knip](https://github.com/BartWaardenburg/fallow/wiki/Migrating-from-Knip)
- [Full plugin list](https://github.com/BartWaardenburg/fallow/wiki/Frameworks)

## Contributing

Missing a framework plugin? Found a false positive? [Open an issue](https://github.com/BartWaardenburg/fallow/issues).

```bash
cargo build --workspace && cargo test --workspace
```

## License

MIT
