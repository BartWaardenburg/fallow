# Changelog

All notable changes to fallow are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.8.1] - 2026-03-25

### Added
- LSP pull-model diagnostics (`textDocument/diagnostic`) for Cursor/VSCodium compatibility
- Multi-root workspace support in LSP: discovers monorepo workspaces via `package.json` workspaces, pnpm-workspace.yaml, and tsconfig references
- Precise diagnostic ranges for unresolved imports: squiggly underline now covers only the module specifier string literal, not the entire import line
- `source_span` field on `ImportInfo` for precise source string literal positions

### Fixed
- LSP `textDocument/diagnostic` errors in Cursor (Method not found -32601)
- Unused dependency diagnostics now appear on the correct line in package.json (was always line 1)
- Circular dependency diagnostics now appear on the import statement that starts the cycle
- File-level diagnostics (unused files, unlisted deps) highlight the entire first line instead of a zero-width marker
- Stale cached diagnostics cleared when issues are resolved between analysis runs
- Extract cache version bumped to 16 for new `ImportInfo.source_span` field

### Changed
- Internal refactoring: split large modules (human.rs, visitor.rs, resolve.rs, registry.rs, etc.) into focused submodules
- Improved test coverage across graph, extract, core, and LSP crates

## [1.8.0] - 2026-03-24

### Added
- **Human output polish**: completely redesigned terminal output across all three commands for readability at scale
  - Per-section footers with one-line explanations and docs links (always shown, replaces `--explain` for human output)
  - Mirrored directory detection in dupes: collapses `src/ ↔ deno/lib/` patterns into a single group
  - Circular dependency hub grouping: cycles sharing the same file are grouped with path elision
  - Duplicate exports stacked vertically with path elision for long monorepo paths
  - Global truncation: all sections default to 10 items with `... and N more` overflow hints
  - Thousands separators for large numbers (e.g., `5,433` lines)
- **`--complexity` flag** for `fallow health`: select only the complexity findings section
- **Health defaults to all sections**: `fallow health` now shows complexity + file-scores + hotspots by default. Use `--complexity`, `--file-scores`, or `--hotspots` to select specific sections.
- **`--explain` flag**: adds `_meta` object with metric definitions and docs URLs to JSON output. Human output always includes explanations.
- **`--top` flag** for `fallow dupes`: limit the number of clone groups shown
- `elide_common_prefix` utility for shorter paths in dependency chains and duplicate exports

### Changed
- Summary footer uses shortened labels (`27 files · 89 exports` instead of `27 unused files · 89 unused exports`)
- Health footer compacted to single line (`✗ 22 above threshold · 3111 analyzed · MI 90.5`)
- Dupes footer uses `✗` prefix consistent with check and health
- Health complexity findings use two-line format (function name on line 1, metrics on line 2)
- Single-group clone families suppressed from default output
- Tree connectors (├─ └─) replaced with simple indentation in dupes
- All docs URLs updated to `/explanations/{dead-code,health,duplication}` with section anchors
- Config warnings use `tracing::warn!` instead of `eprintln!`
- CI workflow permissions scoped to per-job level

## [1.7.0] - 2026-03-24

### Added
- Hotspot analysis (`fallow health --hotspots`): combines git churn history with complexity data to surface the riskiest files in a codebase. Score formula: `normalized_churn × normalized_complexity × 100` (0-100 scale). Recency-weighted commit count with exponential decay (half-life 90 days). Trend detection labels files as accelerating, stable, or cooling.
  - `--since` accepts durations (`6m`, `90d`, `1y`, `2w`) and ISO dates (`2025-06-01`), default 6 months
  - `--min-commits` threshold (default 3) excludes low-activity files from ranking
  - Fan-in shown as separate "blast radius" column
  - Shallow clone detection with warning
  - Available in all output formats (human, JSON, compact, markdown)
  - MCP `check_health` tool supports `hotspots`, `since`, and `min_commits` parameters

### Changed
- Health human output uses two-line format for hotspots and file scores: score/MI on first line with path (dimmed directory, bold filename), metrics on indented second line
- Renamed "lines" to "churn" in hotspot output across all formats to avoid ambiguity with file length

### Fixed
- MCP tests on Windows: skip `/bin/sh`-dependent tests on non-Unix platforms
- Typo checker false positive and Windows path separator in list tests

## [1.6.1] - 2026-03-24

### Added
- Per-file health scores (`fallow health --file-scores`): maintainability index combining complexity density, dead code ratio, fan-in, and fan-out. Available in all output formats (human, JSON, compact, markdown, SARIF). MCP `check_health` tool supports `file_scores: true` parameter.
- Markdown and SARIF output formats for health command
- Workspace scoping (`--workspace`) and baseline support (`--baseline`/`--save-baseline`) for health command

### Fixed
- Re-export chain propagation for entry-point-only exports: exports consumed solely via re-export from an entry point barrel (e.g., `export { render } from './render'` in `src/index.js`) are no longer falsely reported as unused. Fixes false positives for the common library barrel pattern.
- Entry point star re-exports (`export * from './source'`) now correctly exclude the default export per ES spec
- Deterministic bare specifier resolution: removed the `BareSpecifierCache` that caused non-deterministic results in multi-threaded mode when per-file tsconfig path aliases resolved the same specifier to different targets. Replaced with a deterministic post-resolution pass that upgrades `NpmPackage` to `InternalModule`. Analysis results are now identical between `--threads 1` and default multi-threaded mode.
- Hard error on health baseline I/O failures instead of silent fallback
- MCP server test and review findings addressed

## [1.6.0] - 2026-03-24

### Added
- Precise line/column locations in SARIF output for all issue types that previously defaulted to line 1
  - Unlisted dependencies: SARIF now points to the actual import statement in the source file
  - Unused dependencies (all variants): SARIF now points to the dependency entry line in package.json
  - Type-only dependencies: SARIF now points to the dependency entry line in package.json
  - Circular dependencies: SARIF now points to the import statement that starts the cycle
- `ImportSite` type in JSON output for unlisted dependencies (replaces plain path strings with `{path, line, col}` objects)
- `find_import_span_start()` method on `ModuleGraph` for looking up import statement locations between modules

### Changed
- JSON output `schema_version` bumped to 3 (new fields on `UnusedDependency`, `TypeOnlyDependency`, `CircularDependency`)
- `UnlistedDependency.imported_from` changed from `string[]` to `ImportSite[]` in JSON output
- `UnusedDependency` now includes `line` field (1-based line in package.json)
- `TypeOnlyDependency` now includes `line` field (1-based line in package.json)
- `CircularDependency` now includes `line` and `col` fields (location of the cycle-starting import)

### Fixed
- GitHub Code Scanning alerts for unlisted/unused dependencies no longer point to `package.json:1`
- GitHub Code Scanning alerts for circular dependencies no longer point to line 1 of the first file
- `output-schema.json` `DuplicateExport.locations` corrected from `string[]` to proper `DuplicateLocation[]` objects
- `output-schema.json` now includes missing `unused_optional_dependencies` field

## [1.5.0] - 2026-03-24

### Added
- `fallow health` command: per-function cyclomatic complexity (McCabe classic) and cognitive complexity (SonarSource algorithm) analysis with configurable thresholds (default: cyclomatic 20, cognitive 15), `--top N`, `--sort`, `--changed-since`, and human/JSON/compact output formats
- `[health]` config section with `maxCyclomatic`, `maxCognitive`, and `ignore` glob patterns
- `check_health` MCP tool for AI agent integration
- Function name capture from class methods, object method shorthand, variable declarators, property definitions, and `export default function()`

### Changed
- Duplicate export locations now include line and column numbers (JSON output `schema_version` bumped to 2)
- GitHub Action job summary redesigned with tables, icons, and collapsible sections
- Cache version bumped to 15 (old caches auto-invalidate)

### Fixed
- SARIF URIs now percent-encode brackets for Next.js dynamic routes (`[slug]` → `%5Bslug%5D`)
- JSON output uses relative paths for readable CI summaries
- GHAS availability check before SARIF upload in GitHub Action

## [1.4.0] - 2026-03-23

### Added
- Class instance member tracking: `const svc = new MyService(); svc.greet()` correctly tracks `greet` as a used class member, reducing false positives for class-based code (NestJS services, Angular components, etc.)
- JSDoc `@public` tag support: exports annotated with `/** @public */` (or `/** @api public */`) are never reported as unused, designed for library authors whose exports are consumed by external projects
- Type-only dependencies rule: production dependencies only imported via `import type` are reported as `type-only-dependency` (they should be devDependencies since types are erased at runtime)
- Progress spinners showing analysis pipeline stages (discovery, parsing, resolution, analysis) for better UX on larger projects
- VS Code extension published on Open VSX for Cursor and VSCodium support
- Historical performance metric tracking with GitHub Pages dashboard
- Package.json `#subpath` imports integration test

### Changed
- VS Code extension bundler migrated from esbuild to Rolldown
- Large modules split into focused submodules for maintainability: `discover.rs`, `config.rs`, `resolve.rs`, `detect.rs`, `cache.rs`, `visitor.rs`, `check.rs`, and integration tests
- Watch mode now shows changed file paths and clears screen between runs

### Fixed
- SARIF output now includes `tool.driver.version` and `$schema` fields
- Conformance workflow JSON output handling

## [1.3.1] - 2026-03-23

### Added
- `dupes --changed-since`: duplication detection now supports `--changed-since` to only report clone groups involving changed files
- Angular plugin `resolve_config()`: parses `angular.json` to extract `styles`, `scripts`, `main`, `browser`, and `polyfills` from build targets as entry points; adds Angular peer dependencies (`rxjs`, `@angular/common`, `@angular/platform-browser`, `@angular/build`) to tooling deps; widens entry patterns for Nx monorepo layouts
- Nx plugin `resolve_config()`: parses `project.json` to extract executor references as dependencies and `options.main` as entry points; adds `@nx/angular`, `@nx/storybook`, `@nx/webpack`, and other `@nx/*` packages to tooling deps
- JSON config file filesystem fallback: plugins with JSON config patterns (e.g., `angular.json`, `**/project.json`) are now discovered via filesystem scan when they're not in the JS/TS discovered file set; workspace-level plugins also check the project root for config files
- File-based plugin activation for ESLint and Vitest: plugins now activate when their config files exist in a workspace, not just when the package is in `dependencies`. Fixes false positives in monorepos where `eslint`/`vitest` are only in the root `package.json`
- Vitest plugin marks `setupTests.{ts,tsx,js,jsx}` and `test-setup.{ts,tsx,js,jsx}` as always-used when active, fixing false positives for test setup files referenced via imported/spread base configs
- Nested package entry discovery now searches `services/`, `tools/`, and `utils/` directories in addition to `packages/`, `apps/`, `libs/`, `modules/`, `plugins/`
- Infrastructure entry point detection: Dockerfiles, Procfiles, and `fly.toml` are scanned for source file references

### Fixed
- Bare specifier cache poisoning: the resolver cache for bare specifiers (e.g., `@scope/pkg`) now only caches results from successful `oxc_resolver` resolution; previously, when resolution failed for a tsconfig path alias that looked like an npm package, the `NpmPackage` fallback was cached and prevented correct resolution for all subsequent files
- Production dependency false positives: `plugin_tooling` dependencies (e.g., `zone.js`, `@angular/compiler`) are now excluded from unused production dependency detection, matching the existing dev dependency behavior
- Workspace entry pattern double-prefixing: entry patterns from `resolve_config()` that are already project-root-relative (e.g., `apps/client/src/styles.css` from `angular.json`) are no longer double-prefixed with the workspace path
- `check --changed-since` now filters all issue types: unlisted dependencies, duplicate exports, and circular dependencies were previously unfiltered
- Duplication stats recomputation: `recompute_stats` now deduplicates overlapping line ranges per file (matching the original `compute_stats` logic), fixing inflated `duplicated_lines` counts after baseline or `--changed-since` filtering
- Plugin entry point attribution: entry points discovered by plugins now show the correct plugin name (e.g., `Plugin { name: "nextjs" }`) instead of the generic `Plugin { name: "plugin" }`

## [1.3.0] - 2026-03-22

### Added
- Unused optionalDependencies detection: packages in `optionalDependencies` never imported or used as script binaries are now reported
- Function overload deduplication: multiple overload signatures for the same function name are deduplicated to avoid false positive unused-export reports

### Changed
- Workspace discovery performance: canonicalize optimization reduces syscall overhead during workspace resolution
- Re-export chain performance: `matches_str` optimization for faster re-export chain propagation

### Fixed
- Ecosystem test script: `--root` flag and install commands corrected for cross-project validation

## [1.2.0] - 2026-03-22

### Added
- Unused import binding detection via `oxc_semantic`: imports where the bound name is never read in the importing file no longer count as references to the exported symbol, improving unused-export detection precision
- Namespace destructuring detection: `const { foo, bar } = ns` after `import * as ns` (and dynamic import / require namespaces) now correctly tracks accessed members for narrowing

### Fixed
- Namespace imports with whole-object consumption patterns (`Object.values(ns)`, `{...ns}`, `for..in ns`, `const { a, ...rest } = ns`) now correctly mark all exports as referenced instead of being skipped by the entry-point guard

## [1.1.0] - 2026-03-21

### Added
- Markdown output format (`--format markdown`)
- Oxc-inspired code quality: workspace-level clippy configuration with `all`, `pedantic`, `nursery`, `cargo` lint groups and 13 restriction lints
- `#[expect]` over `#[allow]` for all clippy suppressions (warns when suppression becomes unnecessary)
- Const size assertions on hot-path types (`ModuleNode`, `ModuleInfo`, `ExportInfo`, `ImportInfo`, `Edge`) to prevent accidental struct bloat
- VS Code extension icon and marketplace metadata

### Changed
- Module graph split into focused submodules (`types.rs`, `build.rs`, `reachability.rs`, `re_exports.rs`, `cycles.rs`)
- Dev profile optimized with `debug = false` and selective `opt-level` for proc-macro crates

### Fixed
- Windows build: restrict `ModuleNode` size assertion to Unix (`PathBuf` layout differs on Windows MSVC)

## [1.0.4] - 2026-03-21

### Fixed
- CI failures across typos, cargo-deny, docs, MSRV, and clippy
- Windows symlink support for workspace integration test

## [1.0.3] - 2026-03-20

### Fixed
- npm publish: switch to OIDC trusted publishing (no NPM_TOKEN secret needed)

## [1.0.2] - 2026-03-20

### Fixed
- npm publish: add registry authentication to release workflow

## [1.0.1] - 2026-03-20

### Fixed
- Windows build: restrict `DiscoveredFile` size assertion to unix (different `PathBuf` layout on Windows MSVC)

## [1.0.0] - 2026-03-20

### Added
- TypeScript project references: workspace discovery from `tsconfig.json` `references` field (additive with npm/pnpm workspaces, deduplicated by canonical path)
- Elementary cycle enumeration for circular dependencies (individual cycles within SCCs, max 20 per SCC, deterministic output)
- CSS Modules support (`.module.css`/`.module.scss`) with class name export tracking
- `fallow migrate` command to convert knip and jscpd configs
- CSS/SCSS file discovery with `@import`, `@use`, `@forward`, and `@apply` extraction
- Cross-workspace `exports` field subpath resolution with output→source fallback
- Pnpm content-addressable store detection for injected dependencies
- Cache-aware incremental parsing with `--performance` cache hit/miss stats
- Code Lens with export reference counts and click-to-navigate in LSP
- Duplication diagnostics and "Extract duplicate into function" code action in LSP
- VS Code extension CI, LSP binary builds, and marketplace publishing
- Nuxt `resolve_config()` for deep config parsing (modules, css, plugins, extends, postcss, path aliases)
- Circular dependency benchmarks vs madge and dpdm
- Inline suppression for circular dependencies (`fallow-ignore-file circular-dependency`)
- Backwards compatibility policy (`docs/backwards-compatibility.md`)
- JSON output schema documentation (`docs/output-schema.json`)

### Fixed
- UTF-8 boundary handling in duplication detection (multi-byte character safety)
- Exports map resolution robustness for cross-workspace imports
- Nested output subdirectory mapping (e.g., `dist/esm/utils.mjs` → `src/utils.ts`)
- Trace path matching for monorepo compatibility (canonicalized vs user-provided paths)

## [0.3.0] - 2026-03-18

### Added
- Production mode (`--production`) — excludes test/dev files, limits to production scripts, reports type-only imports
- Clone families with refactoring suggestions (extract function/module)
- Config schema generation (`fallow config-schema`) with `$schema` support for IDE autocomplete
- Duplication baselines (`--save-baseline` / `--baseline`) for incremental CI adoption
- Deep `resolve_config()` for all top 10 framework plugins (ESLint, Vite, Jest, Storybook, Tailwind, Webpack, TypeScript, Babel, Rollup, PostCSS)
- JSONC as default config format (with JSON and TOML still supported)
- Pre-commit hooks for fmt and clippy checks

### Changed
- Migrated repository to fallow-rs organization

### Fixed
- Detection accuracy: star re-export tracking, workspace plugins, JSX fallback
- GitHub Action YAML parse error and benchmark workflow

### Performance
- Optimized resolve, discovery, and duplication detection hotpaths

## [0.2.0] - 2026-03-18

### Added
- Code duplication detection (`fallow dupes`) with suffix array + LCP algorithm
- 4 detection modes: strict, mild, weak, semantic
- Duplication benchmarks comparing fallow dupes vs jscpd (4-75x faster)
- Plugin system with 40 built-in framework plugins as single source of truth
- Dynamic import resolution (template literals, `import.meta.glob`, `require.context`)
- Vue/Svelte SFC parsing with `<script>` block extraction
- Dynamic member heuristics (Object.values/keys/entries, for..in, spread)
- GitHub Action with job summaries, PR comments, and `--sarif-file` flag
- LSP diagnostics for all 10 issue types

### Fixed
- CJS require analysis and namespace narrowing for detection accuracy

### Performance
- Clone detection pipeline optimized up to 177x faster vs initial implementation

## [0.1.7] - 2026-03-17

### Added
- Roadmap and project documentation

### Changed
- Improved human report formatting

## [0.1.6] - 2026-03-17

### Fixed
- Reduced false positives based on knip comparison across 6 public projects

## [0.1.5] - 2026-03-17

### Added
- Plugin system, baseline comparison, and `fallow fix` command
- Comprehensive test coverage for high-priority gaps
- CI regression tracking benchmarks

### Fixed
- Reduced false positives based on knip comparison across 31 projects
- Byte column counting, strip_prefix, and HashSet reuse
- Deduplicated imported_from paths in unlisted dependency reports

## [0.1.4] - 2026-03-17

### Fixed
- Reduced false positives and improved re-export chain detection
- Repository URL casing for GitHub provenance

## [0.1.3] - 2026-03-17

### Fixed
- npm publish path resolution

### Changed
- Updated GitHub Actions to Node.js 24, added npm READMEs

## [0.1.2] - 2026-03-17

### Fixed
- Repository URLs and npm publish error surfacing

## [0.1.1] - 2026-03-17

### Added
- Initial public release
- Dead code analysis with 10 issue types
- npm publishing pipeline with platform-specific binaries (macOS, Linux, Windows)
- LSP server with diagnostics and code actions
- CLI commands: check, watch, fix, init, list, schema
- 4 output formats: human, JSON, SARIF, compact
- Rules system with per-issue-type severity
- Inline suppression comments
- `--changed-since` and `--fail-on-issues` for CI
- Cross-workspace resolution for npm/yarn/pnpm workspaces

[Unreleased]: https://github.com/fallow-rs/fallow/compare/v1.8.1...HEAD
[1.8.1]: https://github.com/fallow-rs/fallow/compare/v1.8.0...v1.8.1
[1.8.0]: https://github.com/fallow-rs/fallow/compare/v1.7.0...v1.8.0
[1.7.0]: https://github.com/fallow-rs/fallow/compare/v1.6.1...v1.7.0
[1.6.1]: https://github.com/fallow-rs/fallow/compare/v1.6.0...v1.6.1
[1.6.0]: https://github.com/fallow-rs/fallow/compare/v1.5.0...v1.6.0
[1.5.0]: https://github.com/fallow-rs/fallow/compare/v1.4.0...v1.5.0
[1.4.0]: https://github.com/fallow-rs/fallow/compare/v1.3.1...v1.4.0
[1.3.1]: https://github.com/fallow-rs/fallow/compare/v1.3.0...v1.3.1
[1.3.0]: https://github.com/fallow-rs/fallow/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/fallow-rs/fallow/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/fallow-rs/fallow/compare/v1.0.4...v1.1.0
[1.0.4]: https://github.com/fallow-rs/fallow/compare/v1.0.3...v1.0.4
[1.0.3]: https://github.com/fallow-rs/fallow/compare/v1.0.2...v1.0.3
[1.0.2]: https://github.com/fallow-rs/fallow/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/fallow-rs/fallow/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/fallow-rs/fallow/compare/v0.3.0...v1.0.0
[0.3.0]: https://github.com/fallow-rs/fallow/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/fallow-rs/fallow/compare/v0.1.7...v0.2.0
[0.1.7]: https://github.com/fallow-rs/fallow/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/fallow-rs/fallow/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/fallow-rs/fallow/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/fallow-rs/fallow/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/fallow-rs/fallow/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/fallow-rs/fallow/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/fallow-rs/fallow/releases/tag/v0.1.1
