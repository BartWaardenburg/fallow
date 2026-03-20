# Fallow Roadmap

> Last updated: 2026-03-19

JavaScript/TypeScript codebases accumulate dead code and duplication faster than any other ecosystem — broad dependency trees, rapid framework churn, and copy-paste-driven development create entropy at scale. AI-assisted development accelerates this: agents generate code but rarely suggest deletions, and code clones have grown significantly since AI assistants became prevalent.

Code analysis should be fast enough to be invisible — part of the feedback loop on every save and every commit, not a quarterly audit. Fallow combines dead code analysis and duplication detection in a single sub-second tool: one install, one config, one CI step.

---

## Current State (v0.3.x)

**Dead code analysis** covers 10 issue types (unused files, exports, types, dependencies, devDeps, enum members, class members, unresolved imports, unlisted deps, duplicate exports) with 79 framework plugins (30 with AST-based config parsing), 4 output formats (human, JSON, SARIF, compact), auto-fix, and a per-issue severity rules system. Production mode, inline suppression, cross-workspace resolution (npm/yarn/pnpm), and `--changed-since` for incremental CI are all shipped.

**Duplication detection** uses a suffix array with LCP for clone detection — no quadratic pairwise comparison. 4 detection modes (strict, mild, weak, semantic), clone family grouping with refactoring suggestions, baseline tracking for CI adoption, and cross-language TS↔JS matching.

**Integrations**: LSP server with diagnostics, code actions, and Code Lens; VS Code extension with tree views and auto-download; MCP server for AI agent integration; GitHub Action with SARIF upload; external plugin system (`fallow-plugin-*.toml`); migration from knip/jscpd configs.

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

## Towards 1.0

**1.0 is a quality milestone, not a feature milestone.** The config format has been stable since v0.2 — 1.0 adds a formal backwards compatibility guarantee.

### 1.0 Criteria

- [x] **Trustworthy results on the top 20 JS/TS project archetypes** — validated on 5 representative real-world projects (dub.sh, elk, nestjs-boilerplate, showtime-frontend, trpc). FP rates reduced to <30% across all archetypes. Six critical fixes shipped to address cross-archetype issues.
- [x] **Stable config format** — no breaking changes to `.fallowrc.json`/`fallow.toml` without a major version bump. Backwards compatibility policy documented (`docs/backwards-compatibility.md`).
- [x] **Stable JSON output schema** — CI consumers can depend on the JSON structure without breaking across minor versions. Schema documented (`docs/output-schema.json`); `schema_version` field in JSON output (independent of tool version).

### Remaining work

- **tsconfig project references** — cross-workspace resolution currently handles `exports` field subpath resolution and pnpm store detection, but TypeScript project references are not yet followed.
- **Enhanced IDE experience** — hover information showing where an export is used or where other instances of a duplicate block exist.

---

## Post-1.0: Exploration

These are ideas, not commitments. They ship as 1.x releases based on user demand.

- **Fine-grained incremental analysis** — patch the graph in place, track export-level dependencies. Cache-aware parsing already covers the main bottleneck; this would additionally skip file I/O for unchanged files.
- **Historical trend tracking** — store baselines over time, generate trend reports: "dead code grew 15% this quarter, duplication dropped 3%."
- **Intelligent grouping** — group related dead code (e.g., an unused feature spanning 5 files). For duplication: suggest bulk refactors for clone families.
- **More auto-fix targets** — remove unused enum/class members, delete unused files (`--allow-remove-files`), post-fix formatting integration.
- **JSDoc/TSDoc tag support** — `@public` (never report as unused), `@internal` (only report if unused within project).
- **Additional reporters** — Markdown (PR comments), Code Climate, Codeowners integration.

---

## Community & Adoption

These are not gated on any release — they happen continuously:

- **Documentation site** — move from GitHub wiki to a proper docs site
- **Compatibility matrix** — for each of the top 20 frameworks, document exactly what fallow detects vs. knip
- **Contributing guide** — plugin authoring tutorial, "your first PR" guide
- **Blog posts** — technical deep-dives on the suffix array algorithm, Oxc integration, benchmark methodology

---

## Try It

```bash
npx fallow check    # Dead code — zero config, sub-second
npx fallow dupes    # Duplication — find copy-paste clones
```

[Open an issue](https://github.com/fallow-rs/fallow/issues) if your use case isn't covered.
