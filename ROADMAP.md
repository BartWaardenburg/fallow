# Fallow Roadmap

> Last updated: 2026-03-22

JavaScript/TypeScript codebases accumulate dead code and duplication faster than any other ecosystem — broad dependency trees, rapid framework churn, and copy-paste-driven development create entropy at scale. AI-assisted development accelerates this: agents generate code but rarely suggest deletions, and code clones have grown significantly since AI assistants became prevalent.

Code analysis should be fast enough to be invisible — part of the feedback loop on every save and every commit, not a quarterly audit. Fallow combines dead code analysis and duplication detection in a single sub-second tool: one install, one config, one CI step.

---

## Current State

**Unused code analysis** covers 13 issue types (unused files, exports, types, dependencies, devDeps, optionalDeps, enum members, class members, unresolved imports, unlisted deps, duplicate exports, circular dependencies, type-only dependencies) with 84 framework plugins (31 with AST-based config parsing), 5 output formats (human, JSON, SARIF, compact, markdown), auto-fix, and a per-issue severity rules system. Production mode, inline suppression, cross-workspace resolution (npm/yarn/pnpm workspaces and TypeScript project references), `--changed-since` for incremental CI, and TypeScript function overload deduplication are all shipped.

**Duplication detection** uses a suffix array with LCP for clone detection — no quadratic pairwise comparison. 4 detection modes (strict, mild, weak, semantic), clone family grouping with refactoring suggestions, baseline tracking for CI adoption, and cross-language TS↔JS matching.

**Integrations**: LSP server with diagnostics, code actions, and Code Lens; VS Code extension with tree views and auto-download; MCP server for AI agent integration; GitHub Action with SARIF upload; external plugin system (`fallow-plugin-*.{toml,json,jsonc}`); migration from knip/jscpd configs.

**Bundler coverage**: Vite, Webpack, Rspack, Rollup, Rolldown, Tsup, Tsdown — all major JS/TS bundlers are supported with entry point and dependency extraction.

**Non-JS files**: Vue/Svelte SFC, Astro frontmatter, MDX imports, CSS/SCSS modules.

**Debug tooling**: `--trace` for exports, files, dependencies, and clone locations; `--performance` for pipeline timing breakdown.

**1.0 readiness validation**: Tested against 5 real-world projects spanning major archetypes — dub.sh (Next.js), elk (Nuxt), nestjs-boilerplate (NestJS), showtime-frontend (React Native/Expo), trpc (pnpm monorepo). Six critical fixes shipped: `export *` chain propagation through multi-level barrels, tsconfig path alias resolution (`TsconfigDiscovery::Auto` for per-file resolution), Nuxt plugin enhancements (app/ directory, `resolve_config()`, path aliases), React Native platform extensions (`.web`/`.ios`/`.android`/`.native`) with hidden dir allowlist, decorated class member skip for DI frameworks, and plugin improvements (workspace dedup, tsdown, Jest mocks/inline config, Docusaurus virtual modules, `path_aliases()` trait). Backwards compatibility policy documented (`docs/backwards-compatibility.md`), JSON output schema formalized (`docs/output-schema.json`).

See the [README](README.md) for full feature details, benchmarks, and configuration reference.

---

## Known Limitations

- **Syntactic analysis only**: No TypeScript type information. Projects using `isolatedModules: true` (required for esbuild/swc/vite) are well-served; legacy tsc-only projects may see false positives on type-only imports.
- **Config parsing ceiling**: AST-based extraction covers static object literals, string arrays, and simple wrappers like `defineConfig(...)`. Computed values (`getPlugins()`), conditionals (`process.env.NODE_ENV`), and nested config factories are out of reach without JS eval.
- **Svelte export false negatives**: All exports from `.svelte` files are skipped because props (`export let`) can't be distinguished from utility exports without Svelte compiler semantics.
- **CSS/SCSS parsing is regex-based**: Handles `@import`, `@use`, `@forward`, `@apply`, `@tailwind` with comment stripping and CSS Module class name extraction. Does not parse full CSS syntax — `composes:` and `:global()`/`:local()` are not tracked.
- **LSP column offsets are byte-based**: May be off for non-ASCII characters. Identical for ASCII-only source files.
- **NestJS/DI class members**: Abstract class methods consumed via dependency injection are not tracked (syntactic analysis cannot trace DI-resolved calls). Users can set `unused_class_members = "off"` for DI-heavy projects.
- **React Native native modules**: Packages auto-linked by the RN/Expo build system (native modules) are not detected as used since linking happens outside JS imports.
- **Library consumer types**: Types exported for external consumers (not used within the repo itself) are flagged as unused. This is correct behavior but noisy for library packages.

---

## Competitive Context

Fallow exists in a small but active space. Here's how it fits:

- **Knip** adopted the Oxc parser in v6.0, making it 2-4x faster than Knip v5. Fallow remains 3-18x faster than Knip 6.0 due to native Rust compilation and rayon-based parallelism — the parser is only one part of the pipeline, and JavaScript overhead in module resolution, graph construction, and analysis still dominates Knip's runtime. On the largest monorepos (20k+ files), knip errors out entirely while fallow completes in under 2 seconds.
- **Biome** has module graph infrastructure and a `noUnusedImports` lint rule, but `noUnusedExports` (cross-file analysis) is not on their published roadmap. If they ship it, Biome becomes the main competitive pressure. Their advantage is bundled formatting/linting; Fallow's advantage is deeper detection (12 issue types, duplication, framework plugins).
- **rev-dep** (Go-based) performs unused export detection but lacks a plugin system. Its author has stated that framework-specific config parsing is "not feasible in Go" — this is Fallow's core differentiation.
- **AI coding tools** (Cursor, Copilot, Claude Code) are complementary demand drivers, not replacements. They generate code but don't track cross-file usage graphs. AI-assisted development increases dead code accumulation, making analysis tools more important, not less.

---

## Future Exploration

These are ideas, not commitments. They ship as 1.x releases based on user demand.

- **More auto-fix targets** — delete unused files (`--allow-remove-files`), remove unused class members, post-fix formatting integration. Auto-fix is the highest-leverage feature for adoption — users want one-command cleanup.
- **Fine-grained incremental analysis** — patch the graph in place, track export-level dependencies. Cache-aware parsing already covers the main bottleneck; this would additionally skip file I/O for unchanged files.
- **Class member instance tracking** — `instance.method()` and `this.property` should count as usage of class member declarations. Currently class member detection has a high false-positive rate on projects with instance-level access patterns.
- **Custom export conditions / unbuilt workspace fallback** — when resolving cross-workspace imports and `build`/`dist` directories don't exist, fall back to `src/` resolution. Many monorepos (TanStack/query, etc.) use custom conditions.
- **VS Code extension screenshots** — visual demos for the Marketplace listing (diagnostics, tree views, Code Lens, code actions).
- **Security framing for unused dependencies** — flag unused deps with known CVEs, integrate with `npm audit` data.
- **Historical trend tracking** — store baselines over time, generate trend reports.

---

## Community & Adoption

- **Framework compatibility matrix** — for each of the top 20 frameworks, document exactly what fallow detects vs. knip
- **Contributing guide** — plugin authoring tutorial, "your first PR" guide
- **Blog posts** — technical deep-dives on the suffix array algorithm, Oxc integration, benchmark methodology

---

## Try It

```bash
npx fallow check    # Unused code — zero config, sub-second
npx fallow dupes    # Duplication — find copy-paste clones
```

[Open an issue](https://github.com/fallow-rs/fallow/issues) if your use case isn't covered.
