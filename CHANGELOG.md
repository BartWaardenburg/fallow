# Changelog

All notable changes to fallow are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2.4.0] - 2026-03-27

### Added

- **Test-only production dependency detection** -- new issue type (14th) flags production dependencies that are only imported by test files, suggesting they be moved to `devDependencies`. Full pipeline: all 6 output formats, LSP diagnostics, baseline support, inline suppression, severity rules (default: `warn`).
- **`tap` test runner support** -- `tap` and `@tapjs/*` packages recognized as tooling dependencies
- **`type-only-dependency` inline suppression** -- `// fallow-ignore-next-line type-only-dependency` now works (was a pre-existing gap where the suppression comment was silently ignored)

### Fixed

- **Tooling over-exemption** -- removed blanket `@babel/`, `babel-`, and `@rollup/` prefix exemptions from unused dependency detection. These packages are now verified against actual config files via plugin parsing, catching ~8 previously missed unused dev dependencies.
- **ESLint shared config following** -- ESLint plugin now reads imported config packages one level deep to discover peer dependencies referenced by shared configs
- **Prettier config parsing** -- new PrettierPlugin extracts plugin references from `.prettierrc` and `prettier.config.*` files
- **Storybook addon object form** -- Storybook plugin now handles `{ name: '@storybook/addon-essentials' }` object entries alongside string entries in the `addons` array

## [2.3.1] - 2026-03-27

### Added

- **Comprehensive test coverage** — 1,200+ new tests across all crates bringing total to 4,700+ unit/integration tests, 101 snapshot tests, 30 property-based tests, 14 doc tests, and 7 conformance fixtures, achieving 91% line coverage
- **CJS `module.exports.foo` detection** — individual property assignments like `module.exports.foo = fn` are now extracted as named exports, closing ~60 missed findings on CJS-heavy projects
- **Conformance test harness** — `tests/conformance/verify-fixtures.sh` and `verify-expected.py` provide automated expected-output verification for 7 analysis scenarios (barrel resolution, circular deps, suppression, type-only imports, and more)

### Fixed

- **Unreachable module export blindspot** — modules not reachable from entry points but containing a mix of used/unused exports were previously skipped entirely; now each export is evaluated individually
- **Dead test file removed** — `unused_exports_tests.rs` (924 lines) was never compiled due to a missing module directive and contained a type mismatch; inline tests already covered all cases

## [2.3.0] - 2026-03-27

### Added

- **GitLab CI rich MR comments** — new `FALLOW_COMMENT` and `FALLOW_REVIEW` variables enable rich MR summary comments with collapsible sections and inline review discussions with suggestion blocks, matching the GitHub Action's review quality
- **GitLab inline review discussions** — posts findings as positioned `DiffNote` discussions on MR diffs with "Why this matters" sections, actionable fix steps, docs links, and one-click suppress instructions
- **GitLab suggestion blocks** — unused export findings include `suggestion:-0+0` blocks for one-click `export` keyword removal directly in the MR diff
- **Comment merging pipeline** — groups unused exports per file into single comments, deduplicates clone group findings, drops redundant refactoring targets, and merges same-line findings with numbered headers
- **Auto `--changed-since` in GitLab MR context** — automatically scopes analysis to changed files using `CI_MERGE_REQUEST_DIFF_BASE_SHA` when running in merge request pipelines
- **Package manager detection** — review comments and annotations now show correct install/uninstall commands (`npm uninstall`, `pnpm remove`, or `yarn remove`) based on lock file detection
- **GitLab comment cleanup** — automatically removes previous fallow comments and discussions on re-runs to prevent comment spam

### Changed

- **GitLab CI template modularized** — inline jq scripts extracted to separate files in `ci/jq/` and `ci/scripts/`, downloaded at runtime for maintainability
- **`diff_refs` from MR API** — GitLab inline discussions now fetch `base_sha`, `start_sha`, `head_sha` from the MR API instead of CI environment variables, matching the ictu-mcp pattern and improving positioning accuracy after rebases
- **Suggestion block ANSI-C quoting** — GitHub Action suggestion blocks fixed to use `$'...'` quoting for correct newline rendering

### Fixed

- **Subshell variable loss** — review comment counters (`POSTED`/`SKIPPED`) now use process substitution instead of pipe subshell to correctly track posting results
- **Comment body assignment** — `comment.sh` separates jq execution from string concatenation to correctly detect jq failures
- **Combined mode null arrays** — `jq -s 'add'` replaced with explicit `jq -n --argjson` to prevent null output when all comment arrays are empty

## [2.2.3] - 2026-03-27

### Added

- **Auto-changed-since for PRs** — GitHub Action automatically scopes analysis to changed files in pull requests using `--changed-since` with the merge base, eliminating the need for manual configuration
- **Enriched PR annotations** — inline annotations now include actionable context (export names, dependency names, file paths) with improved formatting for duplication annotations
- **VS Code status bar and tree views** — expanded extension UX with project health status bar, issue tree view, and pnpm workspace support
- **`analyze_with_parse_result` API** — new public function in `fallow-core` that accepts pre-parsed modules, enabling callers to skip the parsing stage when modules are already available

### Changed

- **Health pipeline optimization** — `fallow health --file-scores` no longer runs the analysis pipeline twice; pre-parsed modules are reused via the new `analyze_with_parse_result` API
- **O(1) tooling dependency lookups** — `GENERAL_TOOLING_EXACT` (76 entries) converted from linear slice scan to `OnceLock<FxHashSet>` for constant-time lookups
- **O(1) unused import binding lookups** — `ResolvedModule.unused_import_bindings` converted from `Vec<String>` to `FxHashSet<String>` in hot-path reference population
- **Optimized member export referencing** — `mark_member_exports_referenced` now uses `FxHashSet<&str>` and avoids per-export `to_string()` allocation
- **Report dispatcher unified** — new `ReportContext` struct replaces individual parameters across all 3 report dispatch functions for consistent signatures
- **`define_plugin!` macro extended** — supports `resolve_config: imports_only` variant; Cypress, Commitlint, Remark plugins migrated
- **Comprehensive code deduplication** — `emit_json()`, `plural()`, `build_json_envelope()`, shared `sample_results` test helper, fix module helpers, config parser shared traversal

### Fixed

- **Watch mode reload stability** — hardened debounce behavior and related cleanup
- **Windows CI path normalization** — discovery tests now normalize path separators for cross-platform compatibility
- **GitHub Action `pull_request_target` handling** — correctly detects and handles `pull_request_target` events in auto-changed-since logic

### Removed

- **1,986 lines of dead code** — removed orphaned `crates/graph/src/graph/build/` directory that was never compiled (abandoned refactoring artifact)

## [2.2.2] - 2026-03-27

### Added

- **CodeClimate output format** — `--format codeclimate` for GitLab Code Quality integration, with deterministic FNV-1a fingerprints and proper severity mapping
- **GitHub Actions inline annotations** — `--format annotations` emits `::warning` / `::error` workflow commands for inline PR annotations without any Action dependency
- **Real-world conformance benchmarks** — CI now validates against 8 real-world projects (zod, preact, vite, next.js, angular, nuxt, svelte, vue-core)
- **~283 new tests** — comprehensive coverage for complexity metrics, JSDoc @public tags, config extends/merge, re-export chain propagation, dynamic import patterns, declaration extraction, visitor helpers, analysis predicates, cycle detection, and file discovery

### Fixed

- **CodeClimate fingerprint stability** — use FNV-1a instead of `DefaultHasher` for deterministic cross-run fingerprints; include group index in duplication fingerprints
- **Circular dependency annotations** — sanitize chain strings and guard against empty files in annotation output
- **npm/pnpm install stdout leak** — suppress package manager install stdout that leaked into JSON report output
- **Duplicate exports comparison** — handle dict locations correctly in `duplicate_exports` comparison

## [2.2.1] - 2026-03-26

### Changed

- **Parallel workspace processing** — workspace entry point discovery and plugin runs now execute in parallel using rayon, with sequential merge for deterministic results. Up to 21% faster on monorepos (vite: 507ms → 399ms, next.js: 1532ms → 1371ms)
- **Lazy canonicalize** — skips upfront bulk `canonicalize()` of all source files when the project root is already canonical (common case). A `OnceLock`-based fallback handles the rare intra-project symlink edge case on demand. Saves up to 148ms on 20k-file projects
- **O(1) plugin dedup** — workspace plugin name and virtual module prefix deduplication uses `FxHashSet` instead of `Vec::contains` (O(n²) → O(n))

### Fixed

- **Benchmark accuracy** — benchmark script now correctly excludes knip runs that crash (exit code 2) instead of counting crash timings as valid results. Also guards against null status from timeouts
- **Updated benchmark numbers** — rebenchmarked all projects with honest error handling. Speed claims updated: 5-41x vs knip v5 (was 3-36x), 2-18x vs knip v6 (was 2-14x), 8-26x vs jscpd (was 20-33x)

## [2.2.0] - 2026-03-26

### Added

- **Efficiency score** — refactoring targets now include an `efficiency` field (`priority / effort`) and are sorted by efficiency descending, surfacing quick wins first
- **Confidence levels** — each target includes a `confidence` field (`high`/`medium`/`low`) based on data source reliability: `high` for graph/AST analysis, `medium` for heuristic thresholds, `low` for git-dependent recommendations
- **Adaptive thresholds** — fan-in/fan-out normalization uses percentile-based thresholds (p95/p90/p75/p25) from the project's distribution instead of hardcoded constants, with floors to prevent degenerate values in small projects
- **Target thresholds in JSON** — `target_thresholds` object in health JSON output exposes the computed adaptive thresholds for programmatic consumers
- **Effort summary** — human output shows effort breakdown after the targets header (e.g., `16 low effort · 34 medium · 43 high`)
- **Machine-parseable compact categories** — compact output uses underscore-delimited category labels (`circular_dep`, `dead_code`) instead of space-separated labels

### Changed

- **Human output: efficiency as primary score** — the hero number is now efficiency (sort key), with priority shown as a dimmed secondary value
- **Human output: labeled metadata** — effort and confidence on line 2 are now prefixed (`effort:low · confidence:high`) for self-documenting output
- **Markdown table: 5 columns** — reduced from 7 to 5 columns by merging effort/confidence and dropping the separate priority column
- **SARIF messages** — now include priority, efficiency, and confidence values

### Fixed

- **Cycle path deduplication** — `evidence.cycle_path` no longer contains duplicate entries when a file participates in multiple cycles
- **GitLab CI template** — uses Alpine image and detects package manager correctly
- **Benchmark alert threshold** — corrected for `customBiggerIsBetter` benchmarks
- **SARIF version redaction** in test fixtures
- **MCP analyze tool description** — corrected to match `dead-code` command

## [2.1.0] - 2026-03-25

### Added

- **GitLab CI template** (`ci/gitlab-ci.yml`) — includable template with full feature parity to the GitHub Action: Code Quality reports (CodeClimate format) for inline MR annotations, MR comment summaries, incremental caching, and all fallow commands/options via `FALLOW_*` variables
- **GitHub Action: test workflow** — CI validation for SARIF, JSON, dupes, fix, zero-issues, and PR comment scenarios

### Fixed

- **`list --no-cache`** — the `--no-cache` flag now works correctly with the `list` command
- **GitHub Action: `check` → `dead-code` rename** — completed the rename across all case statements, SARIF fallback, and job summary dispatch
- **`dead-code` subcommand** in backwards-compatibility stable interface list

## [2.0.1] - 2026-03-25

### Added

- **MCP server: all global CLI flags exposed** — `--baseline`, `--save-baseline`, `--no-cache`, `--threads` now available on all MCP tools; `--config` gap-filled on `find_dupes`/`check_health`; `--workspace` gap-filled on `find_dupes`/`fix_preview`/`fix_apply`
- **GitHub Action: 13 new inputs** — `no-cache`, `threads`, `only`, `skip`, `cross-language`, `file-scores`, `hotspots`, `targets`, `complexity`, `since`, `min-commits`, `save-snapshot`, `issue-types`
- **GitHub Action: `dead-code` alias support** — all case statements now handle both `dead-code` and legacy `check` command names
- **GitHub Action: bare invocation support** — combined issue count extraction, job summary, and PR comments work when no command is specified

### Fixed

- **GitHub Action: `fix` without `--dry-run` now adds `--yes`** — previously would hang in CI waiting for TTY input

## [2.0.0] - 2026-03-25

### Breaking

- **Bare `fallow` now runs all analyses** (check + dupes + health). Previously it ran only dead code analysis. Use `fallow dead-code` or `fallow --only check` for the old behavior.
- **`check` renamed to `dead-code`**: `fallow dead-code` is now the canonical name for dead code analysis. `fallow check` remains as a hidden alias for backwards compatibility.
- **`--ci`, `--fail-on-issues`, `--sarif-file` are now global flags**: work with all commands (check, dupes, health, bare). Previously check-only.
- **Combined JSON output**: bare `fallow --format json` produces `{ "check": {...}, "dupes": {...}, "health": {...} }` instead of a flat check-only object.
- **Combined SARIF output**: bare `fallow --format sarif` produces a multi-run SARIF document with one run per analysis.
- **MCP server tools** now use `dead-code` subcommand internally.

### Added

- **Combined analysis command**: bare `fallow` runs dead code + duplication + complexity in a single invocation with combined output across all 5 formats
- **`--only` / `--skip` flags**: select which analyses to run when using bare `fallow` (e.g., `fallow --only check,dupes`, `fallow --skip health`)
- **`dead-code` command alias**: `fallow dead-code` as the canonical name for dead code analysis (replaces `check`)
- **`--ci` on all commands**: `fallow dupes --ci` and `fallow health --ci` now work (SARIF + quiet + fail-on-issues)
- **Vital signs snapshots** (`fallow health --save-snapshot`): save codebase health metrics for trend tracking
- **Execute/run split**: internal refactor enabling combined mode — `execute_check`, `execute_dupes`, `execute_health` return results without printing

### Changed

- Codebase refactored: 10 large files split into focused submodules for maintainability

## [1.9.0] - 2026-03-25

### Added
- **Refactoring targets** (`fallow health --targets`): ranked, actionable recommendations that synthesize complexity, coupling, churn, and dead code signals into a prioritized list of files to refactor
  - Seven recommendation rules evaluated in priority order: urgent churn+complexity, break circular dependency, split high-impact file, remove dead code, extract complex functions, reduce coupling
  - Priority formula: `min(density,1)×30 + hotspot×25 + dead_code×20 + fan_in_norm×15 + fan_out_norm×10`
  - Contributing factors with raw `value` and `threshold` for programmatic use
  - **Effort estimation** (`low`/`medium`/`high`) based on file size, function count, and fan-in — shown in all output formats
  - **Evidence linking**: structured data for AI agents — unused export names, complex function names with line numbers, cycle member paths
  - **Baseline support**: `--save-baseline` / `--baseline` now includes refactoring targets for tracking progress over time
  - All five output formats: human (category · effort labels), JSON (with evidence), compact, markdown (Effort column), SARIF (warning-level findings)
  - MCP server: `targets` parameter on `check_health` tool
- VS Code extension overhaul: LSP notifications, walkthrough, diagnostics improvements, tree view enhancements
- Clickable documentation links in all diagnostics + JSON schema for config
- ~100 unit tests across 6 crates

### Fixed
- 11 bug fixes across parser, analysis, baseline, LSP, and performance
- Empty tree views now show "no issues found" after analysis
- `hasAnalyzed` context set from CLI analysis to clear welcome views
- Removed buggy extract duplicate code action
- Qualified `Span` in benchmark to resolve ambiguous import

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

[Unreleased]: https://github.com/fallow-rs/fallow/compare/v2.4.0...HEAD
[2.4.0]: https://github.com/fallow-rs/fallow/compare/v2.3.1...v2.4.0
[2.3.1]: https://github.com/fallow-rs/fallow/compare/v2.3.0...v2.3.1
[2.3.0]: https://github.com/fallow-rs/fallow/compare/v2.2.3...v2.3.0
[2.2.3]: https://github.com/fallow-rs/fallow/compare/v2.2.1...v2.2.3
[2.2.1]: https://github.com/fallow-rs/fallow/compare/v2.2.0...v2.2.1
[2.2.0]: https://github.com/fallow-rs/fallow/compare/v2.1.0...v2.2.0
[2.1.0]: https://github.com/fallow-rs/fallow/compare/v2.0.1...v2.1.0
[2.0.1]: https://github.com/fallow-rs/fallow/compare/v2.0.0...v2.0.1
[2.0.0]: https://github.com/fallow-rs/fallow/compare/v1.9.0...v2.0.0
[1.9.0]: https://github.com/fallow-rs/fallow/compare/v1.8.1...v1.9.0
[1.8.1]: https://github.com/fallow-rs/fallow/compare/v1.8.0...v1.8.1
[1.8.0]: https://github.com/fallow-rs/fallow/compare/v1.7.0...v1.8.0
[1.7.0]: https://github.com/fallow-rs/fallow/compare/v1.6.1...v1.7.0
[1.6.1]: https://github.com/fallow-rs/fallow/compare/v1.6.0...v1.6.1
[1.6.0]: https://github.com/fallow-rs/fallow/compare/v1.5.0...v1.6.0
[2.1.0]: https://github.com/fallow-rs/fallow/compare/v2.0.1...v2.1.0
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
[2.3.0]: https://github.com/fallow-rs/fallow/compare/v2.2.3...v2.3.0
[2.2.3]: https://github.com/fallow-rs/fallow/compare/v2.2.2...v2.2.3
[2.2.2]: https://github.com/fallow-rs/fallow/compare/v2.2.1...v2.2.2
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
