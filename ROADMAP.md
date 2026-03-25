# Fallow Roadmap

> Last updated: 2026-03-24

Fallow finds and fixes dead code, code duplication, and complexity hotspots in TypeScript/JavaScript projects. Fast, framework-aware, zero-config.

AI-assisted development is accelerating codebase entropy — agents generate code but rarely suggest deletions. Fallow is the cleanup tool that keeps up: sub-second analysis on every commit, one binary, one config, one CI step.

---

## Where we are

Fallow ships dead code analysis (13 issue types), duplication detection (4 modes), and complexity metrics — with 84 framework plugins, 5 output formats, auto-fix, and a severity rules system. Integrations include an LSP server, VS Code extension, MCP server for AI agents, and a GitHub Action with SARIF upload.

Tested against real-world projects spanning Next.js, Nuxt, NestJS, React Native/Expo, and pnpm monorepos. See the [README](README.md) for full details.

---

## Where we're going

### Smarter health analysis

`fallow health` now ships function complexity, per-file maintainability scores (`--file-scores`), and **hotspot analysis** (`--hotspots`) — combining git churn with complexity to surface the files that are complex, changing frequently, and most likely to cause problems. `fallow health --hotspots` answers "where should my team spend its refactoring budget?" with data, not gut feel.

Beyond that: codebase-wide vital signs with trend tracking over time, and regression detection in CI.

### Dependency risk

Fallow already detects unused dependencies. The next step is cross-referencing with vulnerability data: "these 5 unused dependencies have known CVEs — remove them for a free security win." Only fallow can surface this because only fallow knows which deps are actually unused.

### Visualization

`fallow viz` — a self-contained interactive HTML report showing your codebase as a treemap with dead code highlighted. No external dependencies, no server, opens in any browser. Dependency graph, cycle visualization, and duplication heatmaps as additional views.

### Smarter auto-fix + AI-assisted cleanup

Auto-fix currently handles safe, reversible changes: removing unused exports, enum members, and dependencies. These are low-risk — worst case you get a compile error and revert.

Riskier cleanups — deleting unused files, removing class members, restructuring modules — are better handled by AI coding agents that can read context, check for dynamic references, and make judgment calls. Fallow's role is to be the **source of truth** for what's unused: structured JSON output and MCP server tools that agents consume to decide what to do. Fallow detects, the agent decides.

For safe auto-fix: unused class member removal and post-fix formatting integration are next.

### Architecture boundaries

Define import rules between directory-based layers (`src/ui/` cannot import from `src/db/`). Validated against the module graph at Rust speed — same idea as dependency-cruiser but faster and integrated with dead code analysis.

### Static test coverage gaps

Identify exports and files with no test file dependency — without running tests. Uses the module graph to determine which source files are reachable from test files. The CI use case: "your PR adds 3 untested exports."

---

## Ongoing

- **Incremental analysis** — finer-grained caching beyond file-level, to make watch mode and CI even faster on large monorepos
- **Plugin ecosystem** — more framework coverage, better external plugin authoring experience, community contributions
- **Cross-workspace resolution** — custom export conditions, unbuilt workspace fallback for monorepos without build artifacts
- **MCP server completeness** — expose remaining global CLI flags (`--config`, `--baseline`, `--save-baseline`, `--no-cache`, `--threads`) across all MCP tools to complete the API surface

---

## Known limitations

- **Syntactic analysis only** — no TypeScript type information. Projects using `isolatedModules: true` (the modern default) are well-served; legacy tsc-only projects may see false positives.
- **Config parsing ceiling** — AST-based extraction handles static configs. Computed values and conditionals are out of reach without JS eval.
- **Svelte export false negatives** — props (`export let`) can't be distinguished from utility exports without Svelte compiler semantics.
- **NestJS/DI class members** — abstract methods consumed via DI are not tracked. Use `unused_class_members = "off"` for DI-heavy projects.

See the [README](README.md) for the full list.

---

## Competitive context

- **Knip** — the closest alternative. Fallow is 3-18x faster than Knip v6 due to native Rust compilation. On the largest monorepos (20k+ files), knip errors out entirely.
- **Biome** — has module graph infrastructure but hasn't shipped cross-file unused export detection. If they do, they cover ~1 of fallow's 13 issue types.
- **SonarQube** — dominates enterprise code quality but is Java-centric, slow on JS/TS, and lacks framework-aware analysis.
- **AI tools** — complementary, not competitive. AI generates code faster than humans can review it, accelerating the dead code problem fallow solves.

---

```bash
npx fallow              # Run all analyses — zero config, sub-second
npx fallow dead-code    # Unused code only
npx fallow dupes        # Duplication — find copy-paste clones
npx fallow health       # Complexity — find functions that need refactoring
```

[Open an issue](https://github.com/fallow-rs/fallow/issues) if your use case isn't covered.
