# Fallow Roadmap

> Last updated: 2026-03-19

Fallow is a Rust-native dead code and duplication analyzer for JavaScript/TypeScript — the fast alternative to knip and jscpd.

**Two pillars**: Dead code analysis (`fallow check`) and duplication detection (`fallow dupes`) are co-equal. Every phase advances both.

---

## Current State (v0.3.x)

### Dead Code (`check`)
- **10 issue types**: unused files, exports, types, dependencies, devDeps, enum members, class members, unresolved imports, unlisted deps, duplicate exports
- **46 framework plugins**: declarative glob patterns + AST-based config parsing for ~20 plugins (15 with rich config extraction)
- **Deep config parsing** for all top 10 frameworks: ESLint, Vite, Jest, Storybook, Tailwind, Webpack, TypeScript, Babel, Rollup, PostCSS — extracts entry points, dependencies, setup files, and tooling references from config objects via Oxc AST analysis (no JS runtime)
- **4 output formats**: human, JSON, SARIF, compact
- **Auto-fix**: remove unused exports and dependencies (`fix --dry-run` to preview)
- **CI features**: `--changed-since`, `--baseline`/`--save-baseline`, `--fail-on-issues`, SARIF for GitHub Code Scanning
- **Rules system**: per-issue-type severity (`error`/`warn`/`off`) in config. All 10 issue types configurable. `--fail-on-issues` promotes `warn` → `error`
- **Inline suppression**: `// fallow-ignore-next-line [issue-type]` and `// fallow-ignore-file [issue-type]` comments, supporting all issue types including `code-duplication`
- **Production mode**: `--production` flag excludes test/dev files, limits to production scripts, skips devDep warnings, reports type-only imports in production deps
- **Script parser**: extracts binary names (mapped to packages), `--config` args (entry points), file path args from `package.json` scripts; handles env wrappers and package manager runners

### Duplication (`dupes`)
- **4 detection modes**: strict (exact tokens), mild (normalized syntax), weak (different literals), semantic (renamed variables)
- **Suffix array with LCP**: no quadratic pairwise comparison — 10x+ faster than jscpd (up to 500x on large projects)
- **Clone families**: groups clone groups sharing the same file set with refactoring suggestions (extract function/module)
- **Baseline tracking**: `--save-baseline` / `--baseline` for incremental CI adoption of duplication thresholds
- **Filtering**: `--skip-local`, `--threshold`, `--min-lines`, `--min-tokens`, `duplicates.ignore` config globs
- **Cross-language detection**: `--cross-language` strips TypeScript type annotations for `.ts` ↔ `.js` matching
- **Configurable normalization**: fine-grained overrides (`ignore_identifiers`, `ignore_string_values`, `ignore_numeric_values`) on top of detection mode defaults
- **Dead code cross-reference**: `check --include-dupes` identifies clone instances in unused files or overlapping unused exports as combined findings

### Debug & Trace Tooling
- `check --trace FILE:EXPORT` — why is this export considered used/unused?
- `check --trace-file PATH` — all edges for a file
- `check --trace-dependency PACKAGE` — where is a dependency used?
- `dupes --trace FILE:LINE` — show all clones of the code block at a specific location
- `--performance` — pipeline timing breakdown per phase

### Integrations
- **CLI commands**: check, dupes, watch, fix, init, migrate, list, schema, config-schema
- **Config format**: JSONC (default), JSON, TOML — with `$schema` support for IDE autocomplete/validation
- **LSP server**: diagnostics for all 10 dead code issue types, quick-fix code actions, Code Lens with export reference counts
- **VS Code extension**: tree views for dead code and duplicates, status bar, Code Lens, auto-download of LSP binary, one-click fixes
- **MCP server**: AI agent integration via stdio transport (analyze, check_changed, find_dupes, fix_preview, fix_apply, project_info)
- **GitHub Action**: SARIF upload, PR comments, baseline support, analysis caching
- **External plugins**: `fallow-plugin-*.toml` for community-driven framework support without writing Rust
- **Migration tooling**: `fallow migrate` converts knip and jscpd configs

### Non-JS File Support
- Vue/Svelte SFC (`<script>` block extraction with `lang="ts"`/`lang="tsx"`, `<script src="...">`, HTML comment filtering)
- Astro (frontmatter extraction between `---` delimiters)
- MDX (line-based import/export extraction with multi-line brace tracking)
- CSS/SCSS (`@import`, `@use`, `@forward` as module dependencies; `@apply`/`@tailwind` as Tailwind dependency usage; CSS Modules class name export tracking)

### Performance
- rayon parallelism, oxc_parser, incremental bincode cache, flat graph storage, DashMap lock-free bare specifier cache
- Large-scale benchmarks on 1,000+ and 5,000+ file projects with warm/cold cache measurements
- Curated duplication benchmark corpus with 100% precision/recall on default settings

### Known Limitations

- **Syntactic analysis only**: No TypeScript type information. Projects using `isolatedModules: true` (required for esbuild/swc/vite) are well-served; legacy tsc-only projects may see false positives on type-only imports.
- **Config parsing ceiling**: AST-based extraction covers static object literals, string arrays, and simple wrappers like `defineConfig(...)`. Computed values (`getPlugins()`), conditionals (`process.env.NODE_ENV`), and nested config factories are out of reach without JS eval.
- **CSS/SCSS parsing is regex-based**: Handles `@import`, `@use`, `@forward`, `@apply`, `@tailwind` with comment stripping, and CSS Module class name extraction. Does not parse full CSS syntax. SCSS partials (`_variables.scss` from `@use "variables"`) rely on the resolver, not SCSS-specific partial resolution.
- **LSP column offsets are byte-based**: Diagnostics and Code Lens use byte column offsets from the Oxc parser, but the LSP spec requires UTF-16 code unit offsets. Identical for ASCII; may be off for non-ASCII characters before the target position on the same line.

---

## Next: Towards 1.0

### Incremental Analysis

**Two-phase approach** (per Rust architect review):

**Phase A (cheap incremental)**: Re-parse only changed files, rebuild the full graph. Graph construction is sub-millisecond; parsing is the bottleneck. This gets 80% of the benefit with 20% of the work.

**Phase B (fine-grained incremental, post-1.0)**: Patch the graph in place, track export-level dependencies, incremental re-export chain propagation. This requires redesigning the flat `Vec<Edge>` storage to support insertion/removal.

### Enhanced Code Actions

- "Extract duplicate" — for duplication: offer to extract a clone family into a shared function
- Hover: show where an export is used, or show other locations of a duplicate block
- Code Lens: click to navigate to reference locations (currently display-only)

### Cross-Workspace: Remaining Edge Cases

The basic cross-workspace resolution works (unified module graph, `--workspace` scoping, symlink resolution via `canonicalize`, `exports` field subpath resolution with output→source mapping). Remaining work:
- pnpm content-addressable store: detect `.pnpm` paths and map them back to workspace sources
- tsconfig project references
- Conditional exports with nested output subdirectories (e.g., `"./utils": { "import": "./dist/esm/utils.mjs" }` — the `esm/` subdirectory inside `dist/` is not stripped during source fallback)

### 1.0 Criteria

**1.0 is a quality milestone, not a feature milestone:**

- [ ] Trustworthy results on the top 20 JS/TS project archetypes (Next.js, Vite, monorepo, NestJS, React Native)
- [ ] Stable config format with backwards compatibility promise
- [ ] Stable JSON output schema for CI consumers

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
- **Communication**: GitHub Discussions for support, feedback, and RFCs
- **Contributing guide**: Plugin authoring tutorial, "your first PR" guide, issue templates
- **Compatibility matrix**: For each of the top 20 frameworks, document exactly what fallow detects vs. knip — let users make informed choices
- **Blog posts**: Technical deep-dives on the suffix array algorithm, the Oxc parser integration, benchmark methodology
- **Backwards compatibility policy**: State explicitly how config format and JSON output changes are handled across versions

---

## Why This Matters

JavaScript/TypeScript codebases accumulate dead code and duplication faster than any other ecosystem — broad dependency trees, rapid framework churn, and copy-paste-driven development create entropy at scale. AI-assisted development accelerates this: agents generate code but rarely suggest deletions, and code clones have grown significantly since AI assistants became prevalent.

Fallow should be fast enough to run on every save and every commit — not as a monthly audit, but as continuous feedback. The combination of dead code analysis and duplication detection in a single sub-second tool means one integration covers both problems.
