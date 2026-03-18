# Fallow Roadmap

> Last updated: 2026-03-18

Fallow is a Rust-native dead code and duplication analyzer for JavaScript/TypeScript — the fast alternative to knip and jscpd.

**Two pillars**: Dead code analysis (`fallow check`) and duplication detection (`fallow dupes`) are co-equal. Every phase advances both.

---

## Current State (v0.2.0)

### Dead Code (`check`)
- **10 issue types**: unused files, exports, types, dependencies, devDeps, enum members, class members, unresolved imports, unlisted deps, duplicate exports
- **40 framework plugins**: declarative glob patterns + AST-based config parsing for ~20 plugins (15 with rich config extraction)
- **4 output formats**: human, JSON, SARIF, compact
- **Auto-fix**: remove unused exports and dependencies (`fix --dry-run` to preview)
- **CI features**: `--changed-since`, `--baseline`/`--save-baseline`, `--fail-on-issues`, SARIF for GitHub Code Scanning
- **Rules system**: per-issue-type severity (`error`/`warn`/`off`) in `[rules]` config section
- **Inline suppression**: `// fallow-ignore-next-line` and `// fallow-ignore-file` comments

### Duplication (`dupes`)
- **4 detection modes**: strict (exact tokens), mild (normalized syntax), weak (different literals), semantic (renamed variables)
- **Suffix array with LCP**: no quadratic pairwise comparison — 10x+ faster than jscpd (up to 500x on large projects)
- **Filtering**: `--skip-local`, `--threshold`, `--min-lines`, `--min-tokens`
- **Same output formats**: human, JSON, SARIF, compact

### Shared Infrastructure
- **CLI commands**: check, dupes, watch, fix, init, list, schema
- **LSP server**: diagnostics for all 10 dead code issue types + quick-fix code actions
- **Performance**: rayon parallelism, oxc_parser, incremental bincode cache, flat graph storage

### Known Limitations (honest assessment)

- **Config parsing gap**: Plugins use declarative patterns, not runtime config loading. This causes false positives when frameworks have custom entry points in their config files (e.g., `jest.config.ts` transforms, `vite.config.ts` plugins). This is the #1 priority to fix.
- **No cross-workspace resolution**: Monorepo packages are analyzed independently. Exports used by sibling packages get flagged as unused.
- **Syntactic analysis only**: No TypeScript type information. Projects using `isolatedModules: true` (required for esbuild/swc/vite) are well-served; legacy tsc-only projects may see false positives on type-only imports.
- **Dupes: no cross-file semantic matching yet**: Semantic mode works within single analysis but doesn't yet track cross-project clone families or suggest refactoring targets.

---

## Phase 0: Credibility & Technical Foundation (now)

Ship immediately. These are blockers or trust issues, not features.

### 0.1 README Accuracy ✅

Fixed: comparison table now says "40 (20 with config parsing)" vs knip's "140+ (runtime config loading)". Benchmark speed ranges updated to match actual table data (25-40x for dead code, 4-75x for duplication).

### 0.2 Rules System ✅

Per-issue-type severity (`error`/`warn`/`off`) in `[rules]` section of `fallow.toml`. All 10 issue types configurable individually. Defaults to `error` for backwards compatibility. `--fail-on-issues` promotes `warn` → `error`. Human output colors and SARIF levels reflect configured severity. `[detect] X = false` is preserved and forces `Severity::Off`.

```toml
[rules]
unused_files = "error"       # fail CI
unused_exports = "warn"      # report but don't fail
unused_types = "off"         # ignore entirely
unresolved_imports = "error"
```

Note: Duplication thresholds remain in `[duplicates]` (existing config section) rather than `[rules.dupes]` to avoid schema duplication.

### 0.3 Inline Suppression Comments ✅

Teams need per-instance suppression for false positives before they can trust CI integration:
- `// fallow-ignore-next-line` — suppress any issue on the next line
- `// fallow-ignore-next-line unused-export` — suppress specific issue type
- `// fallow-ignore-file` — suppress all issues in a file
- `// fallow-ignore-file unused-export` — suppress specific issue type for the file

### 0.4 Resolve Contention Under Parallelism ✅

Replaced `Mutex<HashMap>` with `DashMap` (sharded concurrent map) for the bare specifier cache in `resolve.rs`. Lock-free reads after warmup eliminate contention under rayon's work-stealing on large projects.

### 0.5 Duplication Accuracy Baseline ✅

Curated JS/TS benchmark corpus (`tests/benchmark-corpus/`) with 14 files across 4 clone types (Type-1 exact, Type-2 renamed, Type-3 near-miss, Type-4 semantic) plus 3 negative controls. Automated evaluation pipeline (`evaluate.sh` + `evaluate-results.py`) computes precision/recall per mode against 7 annotated clone pairs and 5 negative pairs.

**Results:** strict/mild/weak modes achieve 100% precision and 100% recall on expected pairs with zero false positives. Semantic mode achieves 100% recall with 75% precision (2 FPs from structurally similar TypeScript interface declarations after identifier blinding). Default production settings (`min-tokens=50, min-lines=5`) achieve 100% precision/recall.

Full report: `tests/benchmark-corpus/BASELINE-REPORT.md`

---

## Phase 1: Trustworthy Results (v0.3.0)

The goal: a developer can run `fallow check` on a real project and get results they trust. This is the gate to 1.0.

### 1.1 Config File Parsing (top 10 frameworks) ✅

Extended `resolve_config` in the Plugin trait for all 10 priority frameworks. Uses Oxc's parser to extract string literals, arrays, object keys, and require() sources from config files — no JS runtime needed.

**Implemented** (all 10 have rich AST-based config parsing):
- **ESLint**: Legacy `.eslintrc` plugin/extends/parser short-name resolution (e.g., `"react"` → `eslint-plugin-react`), flat config `plugins` object keys, JSON config support
- **Vite**: `build.rollupOptions.input`, `build.lib.entry` → entry points; `optimizeDeps.include/exclude` → deps; `ssr.external`/`ssr.noExternal` → deps
- **Jest**: `preset`, `setupFiles`, `setupFilesAfterEnv`, `globalSetup`/`globalTeardown` → setup files; `testMatch` → entry patterns; `transform`, `reporters`, `testEnvironment`, `watchPlugins`, `resolver`, `snapshotSerializers`, `testRunner`, `runner` → deps; JSON config support
- **Storybook**: `addons`, `framework` (string/object), `stories` → patterns; `core.builder` → dep; `typescript.reactDocgen` → dep
- **Tailwind**: `content` → always-used file globs; `plugins`/`presets` via require() and shallow strings → deps
- **Webpack**: `entry` → entry points (string/array/object); `plugins` require() → deps; `externals` → deps; `module.rules` loader extraction (`loader`, `use` string/array/objects, `oneOf` recursion)
- **TypeScript**: `extends` → dep or setup file (supports TS 5.0+ array extends); `compilerOptions.types` → `@types/*` deps; `jsxImportSource` → dep; `compilerOptions.plugins[].name` → deps; `references[].path` → setup files; JSONC support
- **Babel**: `presets`/`plugins` with Babel short-name resolution (`"env"` → `"@babel/preset-env"`, `"transform-runtime"` → `"@babel/plugin-transform-runtime"`); `extends` → dep; JSON/`.babelrc` support
- **Rollup**: `input` → entry points; `external` → deps
- **PostCSS**: `plugins` object keys → deps; `plugins` require() → deps; `plugins` string array → deps

**Config parser helpers**: `extract_config_object_keys`, `extract_config_string_or_array`, `extract_config_require_strings`, `find_config_object` JSON/JSONC support via parenthesized expression wrapping.

**Accuracy ceiling**: AST-based extraction covers the majority of real-world configs — static object literals, string arrays, and simple function wrappers like `defineConfig(...)`. Computed values (`getPlugins()`), conditionals (`process.env.NODE_ENV`), and nested config factories (`defineConfig(withSentry(...))`) are out of reach without JS eval.

### 1.2 Cross-Workspace Resolution

**Dealbreaker for monorepo adoption.** Build a unified module graph across all workspace packages:
- Resolve cross-workspace imports via `node_modules` symlinks, `package.json` `exports` field, and tsconfig project references
- Handle pnpm's content-addressable store: detect `.pnpm` paths and map them back to workspace sources
- A single `fallow check` at the workspace root analyzes all packages together
- `--workspace <name>` flag scopes output to one package while keeping the full graph

**Architecture note**: This requires a `ProjectState` struct that owns the module graph, file registry, and resolved modules across workspace boundaries. This also requires stable FileIds — the current `FileId(idx as u32)` assigned by sort order re-indexes everything when files are added/removed. Introduce `ProjectState` with stable ID assignment here — it also unblocks incremental analysis later.

### 1.3 Script Parser ✅

Lightweight shell command parser for `package.json` scripts:
- Extract binary names (for unlisted binaries detection)
- Extract `--config` and positional arguments (for entry point discovery)
- Handle `node`, `tsx`, `ts-node`, `npx`, `pnpm exec` patterns
- Binary → package name mapping via static divergence map + `node_modules/.bin/` symlink resolution
- Env wrapper handling (`cross-env`, `dotenv`, `KEY=value`)
- Integrated into unused dependency detection pipeline

### 1.4 Production Mode

`--production` flag limiting analysis to shipped code:
- Exclude test files, stories, dev configs
- Only consider `start`/`build` scripts
- Report type-only imports in `dependencies` (should be `devDependencies`)

### 1.5 Duplication: Cross-File Clone Families

Extend `fallow dupes` beyond pairwise detection:
- **Clone families**: Group all instances of the same duplicated code across the project, not just pairs
- **Refactoring suggestions**: "These 4 clones could be extracted to a shared function in `utils/`"
- **Duplication trends with baseline**: `fallow dupes --baseline` to track duplication growth over time, mirroring the dead code baseline feature
- **Ignore patterns for dupes**: `ignore_dupes = ["**/*.test.ts", "**/*.stories.ts"]` — test files often have legitimate boilerplate

### 1.6 Large-Scale Benchmarks

Add benchmarks on 1,000+ and 5,000+ file projects for both `check` and `dupes`. Show warm cache vs cold. Publish methodology, hardware specs, and memory usage. The current 3-project, 174-286 file suite doesn't substantiate claims for real adoption.

---

## Phase 2: Ecosystem & Integrations (v0.4.0)

Reach developers where they are: CI, editors, AI tools.

### 2.1 GitHub Action

Publish `fallow-action` to the GitHub Marketplace. This is the highest-leverage adoption driver — it's largely a wrapper around existing capabilities:
- Run `check` and `dupes` on PRs with SARIF upload to Code Scanning
- Inline annotations on changed lines for both dead code and duplication
- Configurable: fail on new issues, duplication threshold, report-only mode
- Cache the fallow binary and analysis cache

### 2.2 MCP Server

First-mover opportunity. The tool already outputs JSON and has a `schema` command — the MCP server is a thin wrapper:

| Tool | Description |
|------|-------------|
| `analyze` | Full dead code analysis, returns structured results |
| `check_changed` | Only files changed since a git ref |
| `find_dupes` | Duplication detection with mode selection |
| `fix_preview` | Dry-run of auto-fix, returns proposed changes |
| `fix_apply` | Apply fixes |
| `project_info` | Entry points, frameworks, file counts, duplication % |

Ship alongside Claude Code skill and hook configurations — these are documentation, not features.

### 2.3 Plugin Authoring Guide & External Plugins

Community-driven plugin growth instead of writing 80 plugins solo:
- Publish a comprehensive plugin authoring guide with examples
- Support `fallow-plugin-*.toml` files for custom framework definitions
- Ship a `plugins` directory convention
- Defer plugin registry until there's proven demand

### 2.4 More Plugins (community-prioritized)

Add the most-requested plugins based on GitHub issues, not a waterfall list. Likely priorities:
- **Frameworks**: SvelteKit, Gatsby, Docusaurus, NestJS
- **Bundlers**: Rspack/Rsbuild, tsup, unbuild
- **Git hooks**: husky, lint-staged, lefthook
- **Release**: Changesets, semantic-release

### 2.5 Compilers for Non-JS File Types

Extend import extraction to `.astro`, `.mdx`, and improve existing `.vue`/`.svelte` SFC support:
- `.astro` components (extract frontmatter imports)
- `.mdx` files (extract import statements)
- `.css`/`.scss` with Tailwind (extract `@apply` class references)
- Audit existing Vue/Svelte regex extraction against edge cases (multiple `<script>` blocks, `<script setup>`, `lang="tsx"`) and upgrade to a proper parser if the regex approach proves insufficient on real projects

### 2.6 Duplication: Semantic Mode Improvements

- **Cross-language awareness**: Detect clones between `.ts` and `.tsx`, or `.js` and `.ts` variants of the same logic
- **Configurable normalization**: Let users define what "semantic equivalence" means (ignore comments, ignore types, ignore variable names — pick your level)
- **Integration with dead code**: If a duplicated block is also unused, surface it as a single high-priority finding rather than two separate reports

---

## Phase 3: Editor Experience (v0.5.0)

### 3.1 Incremental Analysis

**Two-phase approach** (per Rust architect review):

**Phase A (cheap incremental)**: Re-parse only changed files, rebuild the full graph. Graph construction is sub-millisecond; parsing is the bottleneck. This gets 80% of the benefit with 20% of the work.

**Phase B (fine-grained incremental, post-1.0)**: Patch the graph in place, track export-level dependencies, incremental re-export chain propagation. This requires redesigning the flat `Vec<Edge>` storage to support insertion/removal.

### 3.2 VS Code Extension

- Auto-download the `fallow-lsp` binary (platform-specific)
- Settings UI for toggling issue types and duplication thresholds
- Status bar: dead code count + duplication %
- Tree view: dead code by type, duplicated blocks by clone family
- One-click fix actions

### 3.3 Enhanced Code Actions & Code Lens

- Usage counts on exports (code lens)
- "Remove unused export", "Delete unused file", "Remove unused dependency"
- "Extract duplicate" — for duplication: offer to extract a clone family into a shared function
- Hover: show where an export is used, or show other locations of a duplicate block

### 3.4 Debug & Trace Tooling

- `--trace <export-name>` — why is this export considered used/unused?
- `--trace-file <path>` — all edges for a file
- `--trace-dependency <name>` — where is this dep used?
- `fallow dupes --trace <file:line>` — show all clones of the block at this location
- `--performance` — timing breakdown per phase

---

## 1.0: Stable & Trustworthy

**1.0 criteria** — not a feature milestone, a quality milestone:

- [ ] Trustworthy results on the top 20 JS/TS project archetypes (Next.js, Vite, monorepo, NestJS, React Native)
- [ ] Cross-workspace resolution works for npm, yarn, and pnpm workspaces
- [x] Config parsing for top 10 frameworks with documented accuracy ceiling
- [x] Rules system with per-issue-type severity
- [x] Inline suppression comments
- [ ] Stable config format (`fallow.toml`) with backwards compatibility promise
- [ ] Stable JSON output schema for CI consumers
- [ ] GitHub Action published
- [ ] MCP server published
- [ ] VS Code extension published
- [ ] Duplication detection with clone families and baseline tracking
- [ ] Large-scale benchmarks published (1000+ files, warm/cold cache, memory)
- [ ] Migration guide from knip with worked examples + `fallow migrate` CLI command

Phase 0 items are prerequisites — they ship before Phase 1 starts, so these boxes should already be checked by then.

---

## Post-1.0: Exploration

These are ideas, not commitments. They ship as 1.x releases based on user demand.

### Historical Trend Tracking
Store baselines over time. Generate trend reports for both dead code and duplication: "dead code grew 15% this quarter, duplication dropped 3%." Dashboard-friendly JSON API.

### Intelligent Grouping
Group related dead code (e.g., an unused feature spanning 5 files). For duplication: suggest bulk refactors for clone families that share a common abstraction opportunity.

### Additional Reporters
Markdown (PR comments), Code Climate, Codeowners integration. Custom reporters via external binary.

### More Auto-Fix Targets
Remove unused enum/class members, delete unused files (`--allow-remove-files`), `--fix-type` flag, post-fix formatting integration.

### JSDoc/TSDoc Tag Support
`@public` (never report as unused), `@internal` (only report if unused within project), custom tags.

### Supply Chain Security
Sigstore binary signing, SBOM generation, reproducible builds. Important for enterprise adoption but not an adoption blocker today.

### Cloud & Hosted Features
Remote analysis cache, trend dashboard, team management. Details TBD based on adoption and demand.

---

## Community & Adoption (ongoing, not phased)

These are not gated on any release — they should happen continuously:

- **Documentation site**: Move from GitHub wiki to a proper docs site (Starlight, Nextra, or similar)
- **CHANGELOG**: Maintain a changelog from v0.3 onward
- **Migration tooling**: `fallow migrate` command that reads knip config and generates `fallow.toml`, plus a written migration guide with worked examples
- **Communication**: GitHub Discussions for support, feedback, and RFCs
- **Contributing guide**: Plugin authoring tutorial, "your first PR" guide, issue templates
- **Compatibility matrix**: For each of the top 20 frameworks, document exactly what fallow detects vs. knip — let users make informed choices
- **Blog posts**: Technical deep-dives on the suffix array algorithm, the Oxc parser integration, benchmark methodology
- **Backwards compatibility policy**: State explicitly how config format and JSON output changes are handled across versions

**If capacity is constrained**: The GitHub Action (2.1) and plugin authoring guide (2.3) are the highest-leverage items to prioritize. The VS Code extension (3.2) can be deferred — the LSP already works in any editor natively.

---

## Why This Matters

JavaScript/TypeScript codebases accumulate dead code and duplication faster than any other ecosystem — broad dependency trees, rapid framework churn, and copy-paste-driven development create entropy at scale. AI-assisted development accelerates this: agents generate code but rarely suggest deletions, and code clones have grown significantly since AI assistants became prevalent.

Fallow should be fast enough to run on every save and every commit — not as a monthly audit, but as continuous feedback. The combination of dead code analysis and duplication detection in a single sub-second tool means one integration covers both problems.

---

## Release Milestones

| Version | Theme | Key Deliverables |
|---------|-------|-----------------|
| **0.2** | Current | 10 issue types, 40 plugins, 4 duplication modes, LSP, CI features |
| **0.3** | Trust | Config parsing, cross-workspace, rules system, script parser, dupes clone families, large-scale benchmarks |
| **0.4** | Reach | GitHub Action, MCP server, plugin authoring guide, more plugins, SFC compilers, dupes semantic improvements |
| **0.5** | Editor | Incremental analysis, VS Code extension, code lens, trace tooling |
| **1.0** | Stable | Quality milestone — trustworthy results, stable formats, full docs, migration tooling |
