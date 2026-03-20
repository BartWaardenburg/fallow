# Fallow - Rust-native dead code analyzer for JavaScript/TypeScript

## What is this?

Fallow finds unused files, exports, dependencies, types, enum members, class members, unresolved imports, unlisted deps, and duplicate exports in JS/TS projects. It's a Rust alternative to [knip](https://github.com/webpro-nl/knip) that is 3-36x faster than knip v5 (2-14x faster than knip v6) depending on project size by leveraging the Oxc parser ecosystem.

## Project structure

```
crates/
  config/   — Configuration types, custom framework presets, package.json parsing, workspace discovery
  types/    — Shared type definitions (discover, extract, results, suppress, serde_path)
  extract/  — AST extraction engine (visitor.rs, sfc.rs, astro.rs, mdx.rs, css.rs, parse.rs, cache.rs, suppress.rs)
  graph/    — Module graph construction (graph.rs), import resolution (resolve.rs), project state (project.rs)
  core/     — Analysis orchestration: discovery, plugins, scripts, duplicates, cross-reference, caching, progress
    analyze/    — Dead code detection (mod.rs orchestration, predicates.rs, unused_files/exports/deps/members.rs)
    plugins/    — Plugin system + tooling.rs (general tooling dependency detection)
    duplicates/ — Clone detection (families, normalize, tokenize)
  cli/      — CLI binary, split into per-command modules
    check.rs, dupes.rs, watch.rs, fix.rs, init.rs, list.rs, schema.rs, validate.rs
    report/     — Output formatting (mod.rs dispatch, human.rs, json.rs, sarif.rs, compact.rs)
    migrate/    — Config migration (mod.rs, knip.rs, jscpd.rs)
  lsp/      — LSP server, split into modules
    main.rs, diagnostics.rs, code_actions.rs, code_lens.rs, hover.rs
  mcp/      — MCP server for AI agent integration (stdio transport, wraps CLI)
editors/
  vscode/   — VS Code extension (LSP client, tree views, status bar, auto-download)
npm/
  fallow/   — npm wrapper package with optionalDependencies pattern
tests/
  fixtures/ — Integration test fixtures
```

## Architecture

Pipeline: Config → File Discovery → Incremental Parallel Parsing (rayon + oxc_parser, cache-aware) → Script Analysis → Module Resolution (oxc_resolver) → Graph Construction → Re-export Chain Resolution → Dead Code Detection → Reporting

Key modules in fallow-types:
- `discover` — `DiscoveredFile`, `FileId`, `EntryPoint`, `EntryPointSource`
- `extract` — `ModuleInfo`, `ExportInfo`, `ImportInfo`, `ReExportInfo`, `MemberInfo`, `DynamicImportInfo`, `ParseResult`
- `results` — `AnalysisResults` and all issue types
- `suppress` — Inline suppression comment types and issue kind definitions

Key modules in fallow-extract:
- `lib.rs` — Public API: `parse_all_files()` (parallel rayon dispatch, cache-aware), returns `ParseResult` with modules + cache hit/miss statistics
- `visitor.rs` — Oxc AST visitor extracting imports, exports, re-exports, members, whole-object uses, dynamic import patterns
- `sfc.rs` — Vue/Svelte SFC script extraction (HTML comment filtering, `<script src="...">` support, `lang="ts"`/`lang="tsx"` detection)
- `astro.rs` — Astro frontmatter extraction between `---` delimiters
- `mdx.rs` — MDX import/export extraction with multi-line brace tracking
- `css.rs` — CSS Module class name extraction (`.module.css`/`.module.scss` → named exports)
- `parse.rs` — File type dispatcher: routes files to the appropriate parser (JS/TS, SFC, Astro, MDX, CSS)
- `cache.rs` — Incremental bincode cache with xxh3 hashing. Unchanged files skip AST parsing and load from cache; only changed/new files are parsed. Cache is pruned of stale entries (deleted files) on each run.

Key modules in fallow-graph:
- `project.rs` — `ProjectState` struct: owns the file registry (stable FileIds sorted by path) and workspace metadata. Foundation for cross-workspace resolution and future incremental analysis.
- `resolve.rs` — oxc_resolver-based import resolution + glob-based dynamic import pattern resolution + DashMap-backed bare specifier cache for lock-free parallel lookups. Cross-workspace imports resolve through node_modules symlinks via canonicalize. Pnpm content-addressable store detection: `.pnpm` virtual store paths are mapped back to workspace source files for injected dependencies. React Native platform extensions (`.web`/`.ios`/`.android`/`.native`) resolved via `resolve_file` fallback. Per-file tsconfig path alias resolution (`TsconfigDiscovery::Auto`) finds the nearest tsconfig.json for each file.
- `graph.rs` — Module graph with re-export chain propagation. `ModuleGraph::build` delegates to `populate_edges`, `populate_references`, and `mark_reachable` phase methods.

Key modules in fallow-core (re-exports fallow-extract, fallow-graph for backwards compatibility):
- `discover.rs` — File walking + entry point detection (also workspace-aware). FileIds are assigned deterministically by path sort order (not size) for stability across runs. Hidden directory allowlist (`.storybook`, `.well-known`, `.changeset`, `.github`) — other dotdirs are skipped. Only root-level `build/` is ignored (not nested `test/build/` etc.).
- `analyze/` — Module split into focused submodules:
  - `mod.rs` — Orchestration: runs all detectors, collects `AnalysisResults`
  - `predicates.rs` — Lookup tables and helper predicates for detection logic
  - `unused_files.rs` — Unused file detection
  - `unused_exports.rs` — Unused export/type/duplicate export detection
  - `unused_deps.rs` — Unused dependencies, unlisted dependencies, unresolved imports, type-only dependency detection
  - `unused_members.rs` — Unused enum/class member detection
- `scripts.rs` — Shell command parser for package.json scripts: extracts binary names (mapped to package names for dependency usage detection), `--config` args (entry points), and file path args; handles env wrappers, package manager runners, node runners. Shell operators (`&&`, `||`, `;`, `|`, `&`) are split correctly.
- `suppress.rs` — Inline suppression comment parsing (`fallow-ignore-next-line`, `fallow-ignore-file`); 11 issue kinds including `code-duplication`
- `duplicates/families.rs` — Clone family grouping (groups by shared file set) and refactoring suggestion generation (extract function/module)
- `duplicates/normalize.rs` — Configurable token normalization with `ResolvedNormalization`: mode defaults (strict/mild/weak/semantic) merged with user-specified overrides (`ignore_identifiers`, `ignore_string_values`, `ignore_numeric_values`)
- `duplicates/tokenize.rs` — AST-based tokenizer with optional type annotation stripping (`strip_types` flag) for cross-language clone detection between `.ts` and `.js` files
- `cross_reference.rs` — Cross-references duplication findings with dead code analysis: identifies clone instances that are also unused (in unused files or overlapping unused exports) as high-priority combined findings
- `plugins/` — Plugin system: `Plugin` trait, registry (84 built-in plugins, ~31 with AST-based config parsing); `config_parser.rs` provides Oxc-based helpers for extracting imports, string arrays, object keys, require() sources, and string-or-array values from JS/TS/JSON config files; `tooling.rs` contains general tooling dependency detection (`is_known_tooling_dependency`) for dev deps not tied to any single plugin
- `trace.rs` — Debug & trace tooling: trace export usage (`trace_export`), file edges (`trace_file`), dependency usage (`trace_dependency`), clone location (`trace_clone`), and `PipelineTimings` struct for `--performance` output
- `progress.rs` — indicatif progress bars
- `errors.rs` — Error types

Key modules in fallow-cli:
- `main.rs` — CLI definition (clap) + command dispatch. Each subcommand is in its own module.
- `check.rs` — `check` command: analysis pipeline, tracing, filtering, output
- `dupes.rs` — `dupes` command: duplication detection, baseline, cross-reference
- `watch.rs` — `watch` command: file watcher with debounced re-analysis
- `fix.rs` — `fix` command: auto-remove unused exports, enum members, and deps
- `init.rs` — `init` command: generate config files
- `list.rs` — `list` command: show plugins, entry points, files
- `schema.rs` — `schema` + `config-schema` + `plugin-schema` commands
- `validate.rs` — Input validation (control characters, path sanitization)
- `report/` — Output formatting: `mod.rs` (format dispatch), `human.rs`, `json.rs`, `sarif.rs`, `compact.rs`
- `migrate/` — Config migration: `mod.rs` (orchestration), `knip.rs` (knip config), `jscpd.rs` (jscpd config)

Key modules in fallow-lsp:
- `main.rs` — LSP server setup, `LanguageServer` trait impl, event handling
- `diagnostics.rs` — Diagnostic generation for all issue types
- `code_actions.rs` — Quick-fix and refactor code actions
- `code_lens.rs` — Reference count Code Lens above export declarations
- `hover.rs` — Hover information showing export usage, unused status, and duplicate block locations

## Building & Testing

```bash
git config core.hooksPath .githooks  # Enable pre-commit hooks (fmt + clippy)
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
cargo run -- check              # Run analysis
cargo run -- watch              # Watch mode
cargo run -- fix --dry-run      # Auto-fix preview

# Benchmarks (see BENCHMARKS.md for methodology)
cargo bench --bench analysis                           # All Criterion benchmarks
cargo bench --bench analysis -- large_scale_benches/   # 1000+ and 5000+ file benchmarks only
cd benchmarks && npm run generate && npm run bench     # Comparative benchmarks vs knip
cd benchmarks && npm run generate:dupes && npm run bench:dupes  # vs jscpd
```

## Detection capabilities

1. Unused files, exports, types, dependencies, devDependencies
2. Unused enum members, class members (structural extraction + whole-object-use heuristics for Object.values/keys/entries, for..in, spread, computed access)
3. Unresolved imports, unlisted dependencies
4. Duplicate exports across modules
5. Re-export chain resolution through barrel files
6. Vue/Svelte SFC parsing (regex-based `<script>` block extraction, `lang="ts"`/`lang="tsx"` detection, handles `>` in quoted attributes like `generic="T extends Foo<Bar>"`, `<script src="...">` external script support, HTML comment filtering to avoid false matches)
7. Astro component parsing (frontmatter extraction between `---` delimiters, parsed as TypeScript)
8. MDX file parsing (line-based import/export statement extraction with multi-line brace tracking, parsed as JSX)
9. Dynamic import pattern resolution (template literals, string concat, import.meta.glob, require.context → glob matching against discovered files)
10. Inline suppression comments (`// fallow-ignore-next-line [issue-type]`, `// fallow-ignore-file [issue-type]`) — supports all issue types including `code-duplication`
11. Script binary analysis (package.json scripts → binary names mapped to packages, `--config` args as entry points, env wrapper/package manager runner handling)
12. Clone family grouping: groups clone groups sharing the same file set into families with refactoring suggestions (extract function/module)
13. Duplication baseline support: `--save-baseline` / `--baseline` for incremental CI adoption of duplication thresholds
14. Production mode (`--production`): excludes test/dev files, only start/build scripts, detects type-only dependencies
15. Cross-language clone detection (`--cross-language`): strips TypeScript type annotations (parameter types, return types, generics, interfaces, type aliases, `as`/`satisfies` expressions) for `.ts` ↔ `.js` matching
16. Configurable normalization: fine-grained overrides (`ignore_identifiers`, `ignore_string_values`, `ignore_numeric_values`) on top of detection mode defaults for custom "semantic equivalence" definitions
17. Dead code × duplication cross-reference (`check --include-dupes`): identifies clone instances in unused files or overlapping unused exports as combined high-priority findings
18. Debug & trace tooling: `--trace FILE:EXPORT` (trace export usage chain), `--trace-file PATH` (all edges for a file), `--trace-dependency PACKAGE` (where a dep is used), `dupes --trace FILE:LINE` (trace all clones at a location), `--performance` (pipeline timing breakdown). Human and JSON output.
19. CSS Modules (`.module.css`/`.module.scss`): class names extracted as named exports. Default imports (`import styles from '...'`) resolve member accesses (`styles.className`) to named exports via graph-level narrowing. Handles spread/`Object.values` conservatively.
20. Package.json `exports` field subpath resolution: cross-workspace imports through exports maps (e.g., `"./utils": "./dist/utils.js"`) resolve correctly. Output directories (`dist/`, `build/`, `out/`, `esm/`, `cjs/`) are mapped back to `src/` equivalents with source extension fallback, including nested output subdirectories (e.g., `dist/esm/utils.mjs` → `src/utils.ts`), since fallow ignores output directories by default.
21. Pnpm content-addressable store detection: `.pnpm` virtual store paths (e.g., `node_modules/.pnpm/@myorg+ui@1.0.0/node_modules/@myorg/ui/dist/index.js`) are mapped back to workspace source files. Handles injected dependencies, scoped/unscoped packages, and peer dependency suffixes.
22. Package.json entry point fields: `main`, `module`, `types`, `typings`, `source`, `browser` (string or object), `bin` (string or object), and `exports` (recursive). The `source` field is a common convention for pointing to unbuilt source entry points.
23. `export *` chain propagation through multi-level barrel files: re-export chains (`a.ts` → `barrel.ts` → `index.ts` via `export *`) are fully resolved so that transitive usage is tracked correctly.
24. Tsconfig path alias resolution (`TsconfigDiscovery::Auto`): per-file tsconfig discovery resolves path aliases (e.g., `@/utils`) by finding the nearest `tsconfig.json` for each file, supporting monorepos with per-package tsconfig files.
25. React Native platform extensions: `.web.ts`, `.ios.ts`, `.android.ts`, `.native.ts` variants are resolved alongside standard extensions so platform-specific files are not falsely reported as unused.
26. Decorated class member skip: class members with decorators (NestJS `@Get()`, Angular `@Input()`, TypeORM `@Column()`, etc.) are not reported as unused, since decorator-driven frameworks consume them via reflection.
27. Circular dependency detection: Tarjan's SCC algorithm detects import cycles in the module graph. Configurable via `circular-dependencies` rule.

## Framework support (84 plugins)

**Frameworks**: Next.js, Nuxt, Remix, SvelteKit, Gatsby, Astro, Angular, React Router, TanStack Router, React Native, Expo, NestJS, Docusaurus, Nitro, VitePress, Sanity, Capacitor, next-intl, Relay, Electron, i18next
**Bundlers**: Vite, Webpack, Rspack, Rsbuild, Rollup, Rolldown, Tsup, Tsdown, Parcel
**Testing**: Vitest, Jest, Playwright, Cypress, Mocha, Ava, Storybook, Karma, Cucumber, WebdriverIO
**Linting & formatting**: ESLint, Biome, Stylelint, Prettier, Oxlint, markdownlint, CSpell, Remark
**Transpilation & language**: TypeScript, Babel, SWC
**CSS**: Tailwind, PostCSS
**Database & ORM**: Prisma, Drizzle, Knex, TypeORM, Kysely
**Monorepo**: Turborepo, Nx, Changesets, Syncpack
**CI/CD & release**: Commitlint, Commitizen, semantic-release
**Deployment**: Wrangler (Cloudflare), Sentry
**Git hooks**: husky, lint-staged, lefthook, simple-git-hooks
**Media & assets**: SVGO, SVGR
**Code generation & docs**: GraphQL Codegen, TypeDoc, openapi-ts, Plop
**Coverage**: c8, nyc
**Other**: MSW, nodemon, PM2, dependency-cruiser, Bun

- **Plugins** (`crates/core/src/plugins/`) — Single source of truth for all built-in framework support. Each plugin implements the `Plugin` trait with enablers (package.json detection), static patterns (entry points, always-used files, used exports, tooling dependencies), and optional `resolve_config()` for AST-based config parsing via Oxc.
- **Rich config parsing** — All top 10 framework plugins have deep `resolve_config()` implementations:
  - **ESLint**: Legacy plugin/extends/parser short-name resolution, flat config plugin keys, JSON config
  - **Vite**: rollupOptions.input, lib.entry, optimizeDeps include/exclude, ssr.external/noExternal
  - **Jest**: preset, setupFiles, globalSetup/Teardown, testMatch, transform, reporters, testEnvironment, watchPlugins, resolver, snapshotSerializers, testRunner, runner, JSON config
  - **Storybook**: addons, framework (string/object), stories, core.builder, typescript.reactDocgen
  - **Tailwind**: content globs, plugins (require/strings), presets
  - **Webpack**: entry (string/array/object), plugins require(), externals, module.rules loader extraction (loader/use/oneOf)
  - **TypeScript**: extends (string/array TS 5.0+), compilerOptions.types → @types/*, jsxImportSource, compilerOptions.plugins, references[].path, JSONC support
  - **Babel**: presets/plugins with short-name resolution (e.g. "env" → "@babel/preset-env"), extends, JSON/.babelrc support
  - **Rollup**: input entries, external deps
  - **PostCSS**: plugins (object keys, require() calls, string arrays)
  - **Nuxt**: modules, css, plugins, extends, postcss plugins from `nuxt.config.ts`; path aliases (`~`, `~~`, `#shared`)
- **Plugin trait extensions** — `path_aliases()` for framework-specific alias resolution (e.g., Nuxt `~/`, Next.js `@/`); `virtual_module_prefixes()` for framework virtual modules (e.g., Docusaurus `@theme/`, `@docusaurus/`); `TsconfigDiscovery::Auto` for per-file tsconfig path alias resolution across monorepo packages.
- **External plugins** (`crates/config/src/external_plugin.rs`) — Standalone plugin definitions (JSONC, JSON, TOML) or inline via the `framework` config field. Discovered from: `plugins` config field, `.fallow/plugins/` directory, and `fallow-plugin-*.{jsonc,json,toml}` files in project root. Supports entry points, always-used files, used exports, config patterns, tooling dependencies, and rich `detection` logic (`dependency`, `fileExists`, `all`/`any` combinators). Inline `framework` definitions use the same `ExternalPluginDef` schema and are merged into the plugin pipeline. All formats use camelCase field names. `$schema` field supported for IDE autocomplete in JSONC/JSON. See `docs/plugin-authoring.md` for the full format.

## CLI features

- `check` — analyze with --format (human/json/sarif/compact), --changed-since, --baseline, --save-baseline, --fail-on-issues, --include-dupes (cross-reference with duplication), issue type filters (--unused-files, --unused-exports, etc.), --trace FILE:EXPORT (trace export usage), --trace-file PATH (trace file edges), --trace-dependency PACKAGE (trace dependency usage)
- `dupes` — find code duplication with clone families, refactoring suggestions, --baseline/--save-baseline, --mode (strict/mild/weak/semantic), --min-tokens, --min-lines, --threshold, --skip-local, --cross-language, --trace FILE:LINE (trace all clones at a specific location)
- `watch` — file watcher with debounced re-analysis
- `fix` — auto-remove unused exports, enum members, and deps (--dry-run, --yes/--force for non-TTY confirmation, --format json for structured output)
- `init` — create .fallowrc.json (default) or fallow.toml (`--toml`), includes `$schema` for IDE autocomplete
- `migrate` — migrate config from knip and/or jscpd to fallow (--toml, --dry-run, --from PATH; auto-detects knip.json/knip.jsonc/.knip.json/.knip.jsonc/package.json#knip and .jscpd.json/package.json#jscpd)
- `list` — show active plugins, entry points, files (--format json for structured output)
- `schema` — dump CLI interface as machine-readable JSON for agent introspection
- `config-schema` — print JSON Schema for fallow config files (enables IDE validation)
- `plugin-schema` — print JSON Schema for external plugin files (enables IDE validation)
- Global `--workspace <name>` / `-w` flag scopes output to a single workspace package while keeping the full cross-workspace graph
- Global `--performance` flag shows pipeline timing breakdown per stage

- Environment variables: `FALLOW_FORMAT` (default output format), `FALLOW_QUIET` (suppress progress), `FALLOW_BIN` (binary path for MCP)
- Structured JSON errors on stdout when `--format json` is active (exit code 2 errors include `{"error": true, "message": "...", "exit_code": 2}`)
- Control character validation on `--changed-since`, `--workspace`, `--config` string inputs

See `AGENTS.md` for AI agent integration guide.

## MCP server

`fallow-mcp` is an MCP (Model Context Protocol) server that exposes fallow's analysis as tools for AI agents. It uses stdio transport and wraps the `fallow` CLI binary via subprocess.

**Tools:**
- `analyze` — full dead code analysis (wraps `fallow check --format json`)
- `check_changed` — incremental analysis of changed files (wraps `fallow check --changed-since`)
- `find_dupes` — code duplication detection (wraps `fallow dupes --format json`)
- `fix_preview` — dry-run auto-fix preview (wraps `fallow fix --dry-run --format json`)
- `fix_apply` — apply auto-fixes (wraps `fallow fix --yes --format json`) — destructive
- `project_info` — project metadata: plugins, files, entry points (wraps `fallow list --format json`)

**Configuration:** Set `FALLOW_BIN` env var to point to the fallow binary (defaults to `fallow` in PATH).

**Architecture:** Built with `rmcp` (official Rust MCP SDK). Thin subprocess wrapper — all analysis logic stays in the CLI, the MCP crate only handles protocol framing and argument mapping.

## VS Code extension

`editors/vscode/` is a VS Code extension that wraps the `fallow-lsp` binary and provides additional UI features.

**Features:**
- LSP client with auto-detection and auto-download of the `fallow-lsp` binary
- Real-time diagnostics for all 11 dead code issue types via the LSP
- Quick-fix code actions (remove unused export, delete unused file)
- Refactor code action: "Extract duplicate into function" for code duplication (extracts clone instances into shared functions, replaces all instances in the file)
- Duplication diagnostics with related locations (links to all other instances of the same clone group)
- Code Lens showing reference counts above each export declaration with click-to-navigate (opens Peek References panel via `editor.action.showReferences`)
- Tree views in the sidebar: dead code grouped by issue type, duplicates grouped by clone family
- Status bar showing issue count and duplication percentage
- Commands: full analysis, auto-fix, dry-run preview, LSP restart

**Settings:** `fallow.lspPath`, `fallow.autoDownload`, `fallow.issueTypes`, `fallow.duplication.threshold`, `fallow.duplication.mode`, `fallow.production`, `fallow.trace.server`

**Development:**
```bash
cd editors/vscode
npm install
npm run build    # esbuild production bundle
npm run lint     # tsc --noEmit
npm run package  # vsce package
```

## Production mode

`--production` flag (or `production = true` in fallow.toml) for CI pipelines that only care about production code:

- **Excludes test/dev files**: `*.test.*`, `*.spec.*`, `*.stories.*`, `__tests__/**`, `__mocks__/**`, etc.
- **Only start/build scripts**: Only analyzes production-relevant package.json scripts (`start`, `build`, `serve`, `preview`, `prepare` and their pre/post hooks)
- **Skips unused devDependencies**: Forces `unused_dev_dependencies` severity to `off`
- **Reports type-only dependencies**: Detects production dependencies only imported via `import type` (should be devDependencies since types are erased at runtime)

## Configuration format

Supports JSON (with JSONC comment support) and TOML. Config files are searched in priority order:
`.fallowrc.json` > `fallow.toml` > `.fallow.toml`

- `.fallowrc.json` is the default for `fallow init` — matches the Oxc ecosystem (oxlint's `.oxlintrc.json`)
- TOML is still fully supported via `fallow init --toml`
- A `$schema` field in JSON enables IDE autocomplete and validation
- Run `fallow config-schema` to generate the JSON Schema, or reference it from GitHub
- The `schema.json` file is checked into the repo root
- `extends` — inherit from base config files (array of relative paths, deep-merge objects, replace arrays, circular detection, max 10 levels, cross-format JSON↔TOML, string shorthand supported)
- `overrides` — per-path rule configuration: array of `{ "files": ["*.test.ts"], "rules": { "unused-exports": "off" } }` objects; later overrides take precedence; glob matching against project-relative paths
- `ignorePatterns` — array of glob patterns to exclude from analysis (replaces the old `ignore` field)
- No `detect` section — use `rules` with `"off"` severity instead (e.g., `"unused-types": "off"`)
- No `output` in config — output format is CLI-only via `--format` flag

## Rules system

Per-issue-type severity for incremental CI adoption:

```jsonc
// .fallowrc.json
{
  "$schema": "https://raw.githubusercontent.com/fallow-rs/fallow/main/schema.json",
  "rules": {
    "unused-files": "error",       // fail CI (exit 1)
    "unused-exports": "warn",      // report but don't fail
    "unused-types": "off",         // ignore entirely
    "unresolved-imports": "error"
  }
}
```

Or equivalently in TOML:

```toml
[rules]
unused-files = "error"       # fail CI (exit 1)
unused-exports = "warn"      # report but don't fail
unused-types = "off"         # ignore entirely
unresolved-imports = "error"
```

- `error` — report and fail CI (non-zero exit code)
- `warn` — report but exit 0
- `off` — don't detect or report
- All default to `error` when omitted
- `--fail-on-issues` promotes all `warn` to `error` for that run
- Human output colors reflect severity; SARIF levels are dynamic

## Inline suppression comments

- `// fallow-ignore-next-line` — suppress any issue on the next line
- `// fallow-ignore-next-line unused-export` — suppress specific issue type
- `// fallow-ignore-file` — suppress all issues in a file
- `// fallow-ignore-file unused-export` — suppress specific issue type file-wide
- `// fallow-ignore-file code-duplication` — suppress duplication detection for a file
- `// fallow-ignore-next-line code-duplication` — suppress duplication detection for code on the next line

## Key design decisions

- **No TypeScript compiler dependency**: Syntactic analysis only via Oxc. This is the speed advantage.
- **Plugin system**: Single source of truth for framework support. Rust trait-based plugins with static patterns for common cases and optional AST-based config parsing via Oxc for ~31 plugins (no JavaScript evaluation), many with rich config extraction (entry points, dependencies, setup files from config objects). 84 built-in plugins covering the most popular JS/TS frameworks.
- **Flat edge storage**: Contiguous `Vec<Edge>` with range indices for cache-friendly traversal.
- **Lock-free parallel resolution**: Bare specifier cache uses `DashMap` (sharded concurrent map) for contention-free reads under rayon work-stealing.
- **Re-export chain resolution**: Iterative propagation through barrel files with cycle detection.
- **Cross-workspace resolution**: Unified module graph across npm/yarn/pnpm workspaces (pnpm-workspace.yaml). Cross-package imports resolve through node_modules symlinks via `canonicalize()`. Package.json `exports` field subpath imports resolve via oxc_resolver with output→source fallback (dist/build/out/esm/cjs → src). Pnpm content-addressable store paths (`.pnpm` virtual store) are detected and mapped back to workspace source files, handling injected dependencies where `canonicalize()` resolves through the `.pnpm` directory. `--workspace <name>` scopes output to one package while keeping the full graph. `ProjectState` struct owns the file registry with stable FileIds (path-sorted) for future incremental analysis.

## Git conventions

- Conventional commits: `feat:`, `fix:`, `chore:`, `refactor:`, `test:`
- Signed commits (`git commit -S`)
- No AI attribution in commits
