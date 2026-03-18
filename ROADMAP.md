# Fallow Roadmap

> Last updated: 2026-03-17

Fallow is a Rust-native dead code analyzer for JavaScript/TypeScript, 25-50x faster than knip. This roadmap charts the path from a fast CLI tool to the definitive dead code analysis platform — built for the age of AI-generated code.

---

## Current State (v0.2.0)

- **10 issue types**: unused files, exports, types, dependencies, devDeps, enum members, class members, unresolved imports, unlisted deps, duplicate exports
- **25 framework plugins**: Next.js, Vite, Vitest, Jest, Storybook, Remix, Astro, Nuxt, Angular, Playwright, Prisma, ESLint, TypeScript, Webpack, Tailwind, GraphQL Codegen, React Router, React Native, Expo, Sentry, Drizzle, Knex, MSW, Cypress, PostCSS
- **4 output formats**: human, JSON, SARIF, compact
- **CLI commands**: check, watch, fix, init, list, schema
- **LSP server**: diagnostics for all 10 issue types + quick-fix code actions
- **Performance**: rayon parallelism, oxc_parser, incremental bincode cache, flat graph storage
- **CI features**: `--changed-since`, `--baseline`/`--save-baseline`, `--fail-on-issues`, SARIF for GitHub Code Scanning

---

## Phase 1: Foundation & Parity (v0.2.x)

Close the most impactful gaps versus knip — the ones that cause real false positives.

### 1.1 Config File Parsing

**The single biggest gap.** Knip's 141 plugins extract dependencies and entry points by loading config files at runtime. Fallow's plugins are purely declarative glob patterns. This means fallow misses:
- ESLint plugins referenced in `.eslintrc.json`
- Vite plugins in `vite.config.ts`
- Jest transform/setup files in `jest.config.ts`
- Storybook addons in `.storybook/main.ts`

**Approach**: Extend fallow's existing AST-based config parsing (`resolve_config` in the Plugin trait) to cover the top 20 most-used plugin configs. No JS runtime — use Oxc's parser to extract string literals, array values, and object properties from config files. This preserves fallow's speed advantage while dramatically reducing false positives.

**Priority configs**: ESLint, Vite, Jest, Storybook, Tailwind, Webpack, TypeScript (tsconfig references), Babel, Rollup, PostCSS.

### 1.2 Script Parser

Knip statically analyzes `package.json` scripts to detect binaries and entry files referenced via CLI arguments (e.g., `vitest --config custom.ts`, `tsx scripts/migrate.ts`).

**Approach**: Build a lightweight shell command parser that extracts:
- Binary names (for unlisted binaries detection — new issue type)
- `--config` and positional arguments (for entry point discovery)
- Common patterns: `node`, `tsx`, `ts-node`, `npx`, `pnpm exec`

### 1.3 Production Mode

Add `--production` flag that limits analysis to shipped code only:
- Exclude test files, stories, dev configs
- Only consider `start`/`build` scripts from `package.json`
- Report type-only imports in `dependencies` (should be in `devDependencies`)
- Add `--strict` mode that also verifies workspace isolation

### 1.4 JSDoc/TSDoc Tag Support

Recognize `@public`, `@internal`, `@beta`, and custom tags on exports. This is a standard suppression mechanism that teams already use. Support:
- `@public` — never report as unused
- `@internal` — only report if unused within the project
- Custom tags via config: `tags = ["@api", "@hook"]`

### 1.5 Additional Issue Types

- **Unlisted binaries** — binaries used in scripts but not in dependencies
- **Namespace exports** (`nsExports`/`nsTypes`) — granular tracking when namespace imports are used (`import * as NS from ...`)
- **Optional peer dependencies** — referenced but not installed

### 1.6 More Auto-Fix Targets

Currently only exports and dependencies are auto-fixable. Add:
- Remove unused enum members
- Remove unused class members
- Delete unused files (`--allow-remove-files` safety flag)
- `--fix-type` flag to target specific issue types
- Post-fix formatting integration (Prettier, Biome, dprint)

---

## Phase 2: Plugin Ecosystem (v0.3.x)

Scale from 25 to 80+ framework plugins and make the plugin system externally extensible.

### 2.1 High-Priority Missing Plugins

**Bundlers**: Rollup, Rspack, Rsbuild, Rslib, tsup, tsdown, unbuild, Parcel, Preconstruct, SWC
**Frameworks**: Svelte, SvelteKit, Vue, Gatsby, Docusaurus, Eleventy, Qwik, Nest.js, Nitro, Vike
**Testing**: Mocha, Ava, Cucumber, Stryker, WebdriverIO, Karma, c8, nyc
**Linting/Formatting**: Biome, Prettier, Stylelint, markdownlint, CSpell, oxlint
**CI/DevOps**: GitHub Actions, Travis CI, Nx, moonrepo, Wrangler (Cloudflare Workers)
**Git hooks**: husky, lefthook, lint-staged, simple-git-hooks, yorkie
**Release/Versioning**: Changesets, Semantic Release, Release It!, commitlint
**Other**: Prisma (deeper config), Payload CMS, Sanity, Storybook (deeper), Relay, i18next, Docusaurus

### 2.2 External Plugin API

Allow users to define custom plugins beyond `fallow.toml` framework blocks:
- Load plugin definitions from `fallow-plugin-*.toml` files
- Support a `plugins` directory convention
- Publish a plugin authoring guide
- Consider a plugin registry (community-contributed TOML files)

### 2.3 Compilers for Non-JS File Types

Knip has built-in "compilers" that extract JS/TS imports from non-JS files. Add support for:
- `.vue` single-file components (extract `<script>` block)
- `.svelte` components (extract `<script>` block)
- `.astro` components (extract frontmatter)
- `.mdx` files (extract import statements)
- `.css`/`.scss` with Tailwind (extract `@apply` class references)
- `.prisma` schema files

**Approach**: Regex-based extraction (like knip) for simple cases, Oxc for the extracted JS/TS.

### 2.4 Source Mapping

Map compiled output back to source files using `tsconfig.json` `outDir`. Critical for monorepos where workspace B imports from workspace A's dist output — fallow needs to trace that back to A's source.

---

## Phase 3: LSP & Editor Experience (v0.4.x)

Transform the LSP from MVP diagnostics into a full editor integration.

### 3.1 Incremental Analysis

**The most impactful LSP improvement.** Currently re-analyzes the entire project on every save. Implement:
- Dependency graph tracking: know which files are affected when one file changes
- Only re-parse changed files and re-analyze their dependents
- Target: sub-200ms feedback on save for projects with 1000+ files

### 3.2 VS Code Extension

Package and publish a VS Code extension:
- Auto-download the `fallow-lsp` binary (platform-specific)
- Settings UI for toggling issue types
- Status bar showing issue count
- Tree view of all dead code grouped by type
- One-click fix actions
- Workspace trust integration

### 3.3 Enhanced Code Actions

Beyond the current "remove export" quick-fix:
- **Remove unused import** (when fallow detects the inverse)
- **Delete unused file** with confirmation
- **Remove unused dependency** from package.json
- **Convert to internal** — remove `export` and update all importers
- **Batch fix** — fix all issues of a type in one action

### 3.4 Code Lens

Show usage counts on exports:
```typescript
// 3 references | 0 external uses  ← code lens
export const formatDate = (date: Date) => { ... }
```

### 3.5 Additional LSP Features

- **Hover information**: Show where an export is used (or that it's unused)
- **Workspace diagnostics**: Support `workspace/diagnostic` for project-wide analysis
- **Configuration via LSP settings**: Toggle issue types, severity levels from editor settings
- **Diagnostic severity configuration**: Map issue types to error/warning/info/hint per user preference

### 3.6 Neovim & Other Editors

- Publish setup guides for Neovim (native LSP), Helix, Zed, Sublime Text
- Ensure the LSP binary is distributed alongside the CLI binary in npm releases

---

## Phase 4: AI-Native Integration (v0.5.x)

Position fallow as the code quality backbone for AI-assisted development.

### 4.1 MCP Server

**There is no MCP server for JS/TS dead code analysis.** Fallow should be the first. Expose tools:

| Tool | Description |
|------|-------------|
| `analyze` | Full project analysis, returns structured results |
| `check_file` | Single file export/import status |
| `check_changed` | Only files changed since a git ref |
| `unused_exports` | Focused query for unused exports |
| `unused_deps` | Focused query for unused dependencies |
| `fix_preview` | Dry-run of auto-fix, returns proposed changes |
| `fix_apply` | Apply fixes |
| `project_info` | Entry points, frameworks, file counts |

This lets any MCP-compatible agent (Claude Code, Cursor, Windsurf) query fallow **during** code generation — not just after.

### 4.2 Claude Code Skill

Publish an official fallow skill for Claude Code that:
- Runs `fallow check --format json --quiet` for analysis
- Uses `--changed-since` for PR-scoped checks
- Chains `fix --dry-run` → review → `fix` for safe auto-cleanup
- Surfaces results as actionable tasks

### 4.3 Claude Code Hooks

Pre-built hook configurations:
- **PostToolUse hook**: Run fallow after every file edit, warn if dead code is introduced
- **Pre-commit hook**: Block commits that increase dead code count vs baseline
- **Notification integration**: Surface fallow warnings so AI agents self-correct

### 4.4 GitHub Action

Publish `fallow-action` to the GitHub Marketplace:
- Runs fallow on PRs with SARIF upload to Code Scanning
- Inline annotations on changed lines
- Configurable: fail on new issues, report-only mode, issue type filters
- Caching of fallow binary and analysis cache for fast CI runs

### 4.5 AI-Aware Detection Patterns

Detect patterns specifically common in LLM-generated code:
- **Duplicate function implementations** — same logic in different files (content-hash similarity)
- **Speculative types** — interfaces/types defined but never used as constraints
- **Import/generate mismatch** — imports added but never referenced in the function body
- **Over-generated utilities** — small helper functions used exactly once (candidate for inlining)
- **Orphaned test helpers** — test utilities that no test file imports

### 4.6 Self-Healing Agent Loop

Enable AI agents to use fallow as a feedback signal:
```
Agent generates code
  → fallow check --format json --quiet --changed-since HEAD
  → Agent receives results
  → Agent fixes its own dead code
  → fallow check (verify clean)
```

Document this pattern in AGENTS.md and provide example configurations for Claude Code, Cursor, and Copilot Workspace.

---

## Phase 5: Configuration & Reporting (v0.6.x)

Polish the configuration system and reporting capabilities.

### 5.1 Rules System

Per-issue-type severity configuration:
```toml
[rules]
unused_files = "error"       # fail CI
unused_exports = "warn"      # report but don't fail
unused_types = "off"         # ignore entirely
unresolved_imports = "error"
```

### 5.2 Granular Ignore Controls

- `ignore_binaries` — suppress specific binary warnings
- `ignore_members` — suppress specific member warnings (with regex)
- `ignore_unresolved` — suppress specific unresolved import warnings
- `ignore_workspaces` — exclude specific workspaces from analysis
- `ignore_exports_used_in_file` — don't report exports only used in their own file
- Per-file, per-issue-type ignore: `[[ignore_issues]]` blocks

### 5.3 Additional Reporters

- **Markdown** — table format for PR comments
- **GitHub Actions annotations** — inline annotations on PR diffs
- **Code Climate** — for Code Climate integration
- **Codeowners** — show which team owns each issue (from CODEOWNERS file)
- **Custom reporters** — load reporter from a JS/TS module or external binary
- **Multiple reporters** — output to several formats simultaneously

### 5.4 Debug & Trace Tooling

- `--trace <export-name>` — trace why a specific export is considered used/unused
- `--trace-file <path>` — show all edges for a specific file
- `--trace-dependency <name>` — show where a dependency is used
- `--debug` — verbose output showing each analysis phase
- `--performance` — timing breakdown per phase (discover, parse, resolve, analyze)

### 5.5 Diff Mode

`fallow check --diff <baseline>` — instead of pass/fail, show a human-readable diff:
```
Dead code changes vs baseline:
  +3 unused exports (new)
  -1 unused file (fixed)
  +2 unused dependencies (new)
  Net: +4 issues
```

---

## Phase 6: Advanced Analysis (v1.0)

Features that push beyond knip's capabilities.

### 6.1 Cross-Project Analysis

For organizations with multiple repositories:
- Analyze npm packages to determine which exports are used by downstream consumers
- Integration with package registries to check real-world usage
- `--include-libs` mode to analyze type definitions of dependencies

### 6.2 Historical Trend Tracking

- Store baselines over time (not just current vs previous)
- Generate trend reports: "dead code grew 15% this quarter"
- Identify the most prolific dead code contributors (files, not people)
- Dashboard integration (JSON API for custom dashboards)

### 6.3 Intelligent Grouping

- Group related dead code (e.g., an unused feature spanning 5 files)
- Suggest bulk removals: "these 12 exports form an unused feature — remove all?"
- Detect dead code clusters that could be entire deleted modules

### 6.4 Type-Aware Analysis (Optional)

For users willing to trade speed for accuracy:
- Optional TypeScript program creation for type-level analysis
- Detect unused type parameters, conditional types that always resolve the same way
- Understand `typeof`, `keyof`, mapped types for better export tracking
- Keep this behind a `--type-check` flag to preserve the default speed advantage

### 6.5 Wasm Distribution

Compile fallow to WebAssembly for:
- Browser-based playground / demo
- Embedded analysis in web-based IDEs (StackBlitz, CodeSandbox)
- Serverless function deployment (Cloudflare Workers, Vercel Edge)

---

## Why This Matters Now

LLM-generated code is creating dead code at unprecedented scale:
- **4x growth in code clones** since AI assistants became prevalent (GitClear, 2025)
- **Code refactoring dropped from 25% to under 10%** of changed lines
- Developers using AI score **17% lower on comprehension** of their own code
- Vibe-coded projects accumulate tech debt **3x faster**

AI agents are additive by nature — they generate code but rarely suggest deletions. Every Copilot completion, every Claude Code edit, every Cursor tab-complete risks leaving behind unused imports, speculative types, and orphaned helpers.

Fallow's speed makes it uniquely suited to run **in the loop** — not as a monthly audit, but as real-time feedback during AI-assisted development. The combination of Rust performance, MCP integration, and LSP support positions fallow as the dead code analysis platform for the AI era.

---

## Release Milestones

| Version | Theme | Key Deliverables |
|---------|-------|-----------------|
| **0.2** | Foundation | Config parsing, script parser, production mode, JSDoc tags |
| **0.3** | Ecosystem | 80+ plugins, external plugin API, compilers, source mapping |
| **0.4** | Editor | Incremental LSP, VS Code extension, code lens, enhanced actions |
| **0.5** | AI-Native | MCP server, Claude Code skill/hooks, GitHub Action, AI-aware detection |
| **0.6** | Polish | Rules system, reporters, debug/trace, granular ignores |
| **1.0** | Advanced | Cross-project analysis, trends, intelligent grouping, optional type-check |
