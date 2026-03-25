# Fallow — VS Code Extension Roadmap

> Last updated: 2026-03-25

---

## The thesis

The CLI produces codebase intelligence. The VS Code extension is where developers *act* on it. Today the extension surfaces diagnostics and dead code lists — but ignores all health data (complexity, maintainability, hotspots, churn). The health track is fallow's differentiation layer; if users never see it in their editor, it only exists in CI logs nobody reads.

**Strategic position:** The extension is the daily touchpoint that builds habit → habit drives adoption → adoption drives conversion to the cloud tier. Every health metric that only lives in `fallow health` terminal output is a missed retention opportunity.

**Design constraint:** Extensions that try to show everything get disabled. The best extensions are quiet — they surface information at the moment of decision and stay invisible otherwise. Every feature must answer: *what decision does this help the user make, and when?*

---

## Current state

The extension provides:

- **LSP-driven:** diagnostics (12 issue types), code lens (reference counts), hover (usage info), code actions (remove unused, delete file, extract duplicate)
- **CLI-driven:** two tree views (dead code, duplicates), status bar (`Fallow: 9 issues | 0.0% duplication`), auto-fix command
- **Infrastructure:** auto-download binary, config panel, output channel

**Not surfaced at all:** complexity per function, maintainability index per file, hotspot scores, churn data, fan-in/fan-out, duplication statistics, clone families, refactoring suggestions, health baselines, trend data.

**Not using these VS Code APIs:** FileDecorationProvider, WebviewViewProvider, inlay hints, text decorations, Testing API, Timeline provider, Comments API. *(Now using: progress API, walkthroughs, Quick Pick with navigate-on-click.)*

---

## Expert panel review (2026-03-25)

The plan was reviewed by a panel of 6 experts (VS Code extension developer, UX designer, staff engineer end-user, performance specialist, productivity researcher, junior developer). Key findings:

| Issue | Consensus |
|-------|-----------|
| Information overload risk | 12 simultaneous visual treatments = "visual assault." Every datum gets one canonical home + at most one ambient cue. |
| Dashboard vanity | Force-directed graphs get demoed once and forgotten. <5% sustained engagement in studies. Cut full dashboard; build compact sidebar widget instead. |
| Walkthrough buried | Extensions with Getting Started walkthroughs show 40% higher 30-day retention. Must be Phase 1, not afterthought. |
| Wrong APIs | Comments API for suggestions = wrong pattern (use code actions). DiagnosticTag.Deprecated for type-only deps = wrong semantics (use Unnecessary). Call Hierarchy for imports = semantic mismatch (use custom tree view). |
| No data freshness strategy | Every feature depends on analysis data. Cold start, stale state, incremental invalidation must be solved before building features. |
| Batch operations too late | Core power-user workflow ("select 15 exports, remove all") should be Phase 2, not Phase 5. |
| Status bar real estate | Two status bar items is one too many. Consolidate to one with rich markdown tooltip. |
| Progressive disclosure | Minimal defaults, power users opt in. Ship quiet, let users turn up the volume. |
| Performance at scale | FileDecorationProvider, InlayHintsProvider called on every viewport change. Pre-computed O(1) index required. |

---

## Phase ordering

Ordered by: **what drives daily adoption, what surfaces the health differentiation, what justifies itself through user demand.**

### Phase 0: Data architecture (prerequisite) — SHIPPED

Every feature in this roadmap depends on analysis data being available and current. Without a freshness strategy, features will show stale data or nothing — both erode trust.

**Caching architecture:**

- **Cold start:** Graceful "no data yet" state for every surface. Tree views show welcome content with "Run Analysis" button. Code lens absent (not "loading..."). File decorations absent (not zero badges). Status bar shows `$(search) Fallow` with no counts. **SHIPPED** (welcome views, no decorations until analysis)
- **Warm cache:** Last analysis results stored in LSP server memory as pre-computed indexes. File → issues map for O(1) lookups. Export → usage map for instant code lens/hover.
- **Incremental invalidation:** On file save, mark that file's data as stale. Full re-analysis debounced (500ms, already implemented). Between re-analyses, serve last-known data with no staleness indicator (analysis is fast enough that brief staleness is acceptable).
- **Pre-computed indexes:** Build on analysis completion, not on provider calls. Providers do lookups only. **SHIPPED** (implemented as cached_diagnostics + fallow/analysisComplete notification)

```
Analysis completes
  → build file_issues: HashMap<PathBuf, Vec<Issue>>
  → build file_decorations: HashMap<Uri, FileDecoration>
  → build export_metrics: HashMap<(PathBuf, String), ExportMetrics>
  → build function_complexity: HashMap<(PathBuf, u32), FunctionComplexity>
  → fire onDidChangeFileDecorations(changed_uris)
  → fire code lens refresh
  → update status bar
```

**LSP protocol extensions needed:**

| Custom notification/request | Data | Consumer |
|---|---|---|
| `fallow/analysisComplete` | Summary stats (issue count, health score, duplication %) | Status bar, sidebar widget | **SHIPPED** |
| `fallow/fileMetrics` (request) | Per-file: fan-in, fan-out, maintainability index, complexity density | Health tree view, hover |
| `fallow/healthSummary` (request) | Project-wide: health score, hotspot count, top offenders | Sidebar widget, dashboard |

**Implementation:** Changes to `crates/lsp/src/main.rs` — add index-building after `run_analysis()`, expose via custom LSP methods. No new crate needed.

**Effort:** M (1-2 weeks). Must land before any other phase ships.

---

### Phase 1: Foundation + onboarding — SHIPPED

**Goal:** Make the existing extension trustworthy, discoverable, and immediately useful. This phase ships no new surfaces — it polishes the ones that exist and adds onboarding.

**1a. Walkthrough (Getting Started)** (effort: S, days) — **SHIPPED**

Task-oriented, not feature-oriented:

1. "Find your first unused export" — completes when tree view is opened
2. "Understand a diagnostic" — completes when a diagnostic doc link is clicked
3. "Remove dead code" — completes when auto-fix runs
4. "Check your project's health" — completes when health tree view is opened
5. "Configure rules" — completes when a fallow setting is modified

Implementation: JSON contribution in `package.json` + markdown content. Simplest API in VS Code — one day of work, 40% retention improvement.

**1b. Diagnostic polish** (effort: S, days) — **SHIPPED**

- `DiagnosticTag.Unnecessary` on all unused items (exports, types, members, files) → standard VS Code fade styling. Single highest-impact visual change — universally understood, zero noise. **Was already present.**
- `code` with `target` URL → every diagnostic code becomes a clickable link to documentation. `unused-export` → `https://fallow.dev/rules/unused-export`. Invaluable for onboarding. **SHIPPED** (code_description)
- `relatedInformation` for circular dependencies → shows full cycle path as clickable location links. Circular deps are inherently non-local; related info gives context without navigation. **SHIPPED**
- `relatedInformation` for duplicate exports → links to the other files containing the same export name. **SHIPPED**
- Duplication severity changed from HINT to INFORMATION. **SHIPPED**
- Duplication diagnostic range extended to end of line. **SHIPPED**
- **Drop** `DiagnosticTag.Deprecated` for type-only deps — wrong semantics. "Deprecated" means API is being phased out, not "this should be a devDependency." Use `Unnecessary` or plain `Warning`.

Changes: `crates/lsp/src/diagnostics.rs` only. No extension-side changes needed — LSP protocol handles everything.

**1c. Status bar** (effort: S, days) — **SHIPPED**

Current: `$(search) Fallow: 9 issues | 0.0% duplication`

Revised:
- Single item (not two). Status bar real estate is precious. **SHIPPED**
- Background color: `statusBarItem.warningBackground` when issues > 0, `statusBarItem.errorBackground` when errors (unresolved imports) exist, default when clean. **SHIPPED** (color-coded background)
- Rich markdown tooltip (supported since VS Code 1.78): **SHIPPED** (breakdown + command links)

```markdown
**Fallow** — Project Health

| | |
|---|---|
| $(error) Errors | 1 unresolved import |
| $(warning) Warnings | 3 unused exports, 2 unused files |
| $(info) Hints | 2 circular deps |
| $(copy) Duplication | 0.0% |

---
[$(play) Run Analysis](command:fallow.analyze) · [$(wrench) Auto-Fix](command:fallow.fix) · [$(output) Output](command:fallow.showOutput)
```

- Click action: opens Quick Pick with options (Run Analysis, Show Tree View, Auto-Fix, Show Output) instead of directly running analysis.

Changes: `editors/vscode/src/statusBar.ts` only.

**1d. Progress API** (effort: S, days) — **SHIPPED**

- `ProgressLocation.Window` for quick re-analyses on save (<2s) — subtle spinner, no notification popup.
- `ProgressLocation.Notification` with cancellation for full workspace analysis — shows "Fallow: Analyzing... (Parsing files)" with cancel button. **SHIPPED** (progress notification during CLI analysis)
- Result notification with "Open Sidebar" action. **SHIPPED**
- Debounce: don't fire progress notification if analysis completes within 500ms.

Changes: `editors/vscode/src/commands.ts`.

**1e. Settings & config** (effort: S, days) — **SHIPPED**

- All visual features independently toggleable with sensible defaults (off for power-user features).
- JSON Schema for `.fallowrc.json` via `jsonValidation` contribution in `package.json` — provides autocomplete, validation, and hover docs in the config file for free. No custom editor needed. **SHIPPED**
- Structured settings categories in VS Code Settings UI (`fallow.diagnostics.*`, `fallow.display.*`, `fallow.health.*`).

**Additional items shipped (not originally in roadmap):**

- Tree view path resolution fix (relative paths now work)
- Per-category icons in tree view
- TreeView.badge with issue count
- Welcome views for empty state
- Fix preview Quick Pick with navigate-on-click
- Post-fix flow: save → fix → restart LSP → re-analyze
- Sidebar icon viewBox fix
- `fallow.openSidebar` and `fallow.openSettings` commands
- Extension version synced to 1.8.1

---

### Phase 2: Actions & navigation

**Goal:** Give users tools to *act* on findings efficiently. The extension should enable "cleanup sessions" — dedicated time to reduce tech debt with batch operations and fast navigation.

**2a. Consolidated tree view** (effort: M, 1-2 weeks)

Merge dead code + duplicates into a single "Code Health" tree with collapsible sections. Three trees feels fragmented; one tree with sections is more navigable.

```
▶ Unused Code (9)
  ▶ Unused Files (2)
  ▶ Unused Exports (3)
  ▶ Unused Types (1)
  ▶ Circular Dependencies (2)
  ▸ ...
▶ Code Duplication (0.0%)
  ▶ Clone Families
▶ Complexity Hotspots (5)               ← NEW: health data
  ▸ processData() — cyclomatic: 23      [src/parser.ts:45]
  ▸ resolveImports() — complexity: 18   [src/resolver.ts:12]
▶ Churn Hotspots (3)                    ← NEW: health data
  ▸ src/parser.ts — score: 92, ↑ accelerating
  ▸ src/resolver.ts — score: 78, → stable
▶ Low Maintainability (4)              ← NEW: health data
  ▸ src/legacy.ts — index: 34/100
```

Feature details:
- `TreeView.badge` → total issue count on sidebar icon
- Welcome view with "Run Analysis" button when no results exist
- `resourceUri` on file items → automatic file type icons from theme
- Rich markdown tooltips: why is this flagged, what can you do about it
- Sort by impact: cascading removals first, highest complexity first
- `getParent()` implementation for `reveal()` support (click diagnostic → highlights tree item)

**2b. Batch operations** (effort: M, 1-2 weeks)

- Checkboxes on dead code items via `TreeItemCheckboxState`
- "Remove Selected" command (toolbar button + keyboard shortcut)
- "Suppress Selected" command (adds `// fallow-ignore-next-line` comments)
- Batch preview: shows diff of all changes before applying
- "Select All in Category" for quick full-category operations

This is the #1 requested workflow: select 15 unused exports across 8 files, click "Remove All", review diff, commit.

**2c. Quick Pick navigator** (effort: S, days)

`fallow.goToIssue` command (keyboard shortcut: `Ctrl+Shift+F` / `Cmd+Shift+F` or similar):

- All issues searchable by file name, export name, issue type
- Grouped by category with `QuickPickItemKind.Separator`
- File path in `description`, line number in `detail`
- Fuzzy search built-in
- "Recently viewed" items pinned at top

`fallow.goToHotspot` command:
- Complexity and churn hotspots searchable
- Score in description, trend indicator in detail

**2d. Enhanced code actions** (effort: S, days)

- Remove unused dependency: removes the dependency line from package.json (JSON-aware edit, handles trailing commas) — as QuickFix in hover popup
- Extract duplicate into function: re-implement as QuickFix (was REFACTOR_EXTRACT, removed due to buggy code generation — syntax errors in extracted function). Must handle: indentation, function signature, return types, parameter extraction.
- Suppress issue: inserts `// fallow-ignore-next-line <issue-type>` comment (new)
- Suppress file: inserts `// fallow-ignore-file <issue-type>` at top (new)
- Existing: remove unused export (QuickFix), delete unused file (QuickFix)

---

### Phase 3: File explorer integration

**Goal:** Ambient awareness without leaving the normal workflow. File decorations are visible during all file browsing — PR reviews, feature exploration, refactoring.

**3a. FileDecorationProvider** (effort: M, 1-2 weeks)

- Pre-computed `HashMap<Uri, FileDecoration>` built on analysis completion
- Badge: issue count per file (1-2 characters, e.g., "3")
- Color: `ThemeColor` tinting by highest severity — error (red), warning (yellow), hint (blue)
- Tooltip: "3 unused exports, 1 unresolved import"
- `onDidChangeFileDecorations` fired with **specific URIs only** — never `undefined` (which forces full explorer re-render and kills performance in monorepos)
- **No folder propagation** — aggregating badges up the directory tree is computationally expensive for deep nesting and provides marginal value

**Priority system** when a file has multiple issue types: show highest severity badge.

| Severity | Badge | Color |
|---|---|---|
| Error (unresolved import) | "!" | `list.errorForeground` |
| Warning (unused file/dep) | issue count | `list.warningForeground` |
| Hint (unused export) | issue count | `list.deemphasizedForeground` |

**Performance contract:** `provideFileDecoration` must be a synchronous hash map lookup. No async, no computation. Build the map once on analysis completion, serve from it until next analysis.

Changes: new `editors/vscode/src/fileDecorations.ts`, register in `extension.ts`.

**3b. Context menu** (effort: S, days)

Minimal additions to `explorer/context`:
- "Fallow: Analyze File" — triggers single-file analysis
- "Fallow: Suppress Issues..." — opens Quick Pick showing file's issues for selective suppression (not a one-click "suppress all" — too dangerous)

---

### Phase 4: Selective editor enhancements

**Goal:** In-editor health visualization for developers who want it. **All features opt-in, off by default.** This phase adds visual density that power users value but would overwhelm new users.

**4a. Code lens — actionable items only** (effort: S, days)

Extend existing reference-count code lens with:
- "Remove unused export" on exports with 0 references — clickable, triggers code action
- "Show N duplicates" on clone blocks — opens reference picker showing all instances
- Use `resolveCodeLens` for lazy data loading (only compute when lens scrolls into view)

**Not adding:** complexity code lens (hover is the right surface — complexity is informational, not actionable at the line level).

**4b. Complexity in hover** (effort: S, days)

When hovering over a function signature, show:

```markdown
**Complexity: 15** (threshold: 10)

| Metric | Value | Threshold |
|--------|-------|-----------|
| Cyclomatic | 15 | 10 |
| Cognitive | 23 | 15 |
| Lines | 87 | — |

Consider extracting into smaller functions.
[Learn more](https://fallow.dev/rules/complexity)
```

Only shown for functions that exceed at least one threshold. No hover noise for simple functions.

Changes: `crates/lsp/src/hover.rs` — needs function complexity data from `ModuleInfo`.

**4c. Gutter indicator** (effort: S, days)

- Small colored dot in gutter for lines with any diagnostic issue
- Single `TextEditorDecorationType` with `gutterIconPath` (SVG, ~200 bytes)
- Enables vertical scanning: "where are the issues in this file?"
- Colors match diagnostic severity (error red, warning yellow, hint blue)
- Toggled via `fallow.display.gutterIndicators` setting (default: off)

**4d. Duplication indicator** (effort: S, days)

- Faint dotted left border on duplicated code ranges
- Not always visible — appears on hover/focus within the range
- Hover reveals: "Duplicated in `utils.ts:45-67` and `helpers.ts:23-45`" with clickable links
- Background tint only when cursor is within the range (very subtle, semi-transparent)
- Toggled via `fallow.display.duplicationHighlights` setting (default: off)

**What was cut (and why):**

| Proposed | Why cut |
|---|---|
| Inlay hints for ref counts | Code lens already shows ref counts — showing both is redundant noise |
| Inlay hints for complexity | Hover is the right surface for informational data; inlay hints are persistent clutter |
| Inlay hints for "unused"/"circular" | DiagnosticTag.Unnecessary fade already handles this |
| Opacity 0.5 dimming | DiagnosticTag.Unnecessary already provides standard fade — don't double-apply |
| Colored left-border per complexity | Too many visual layers; complexity belongs in hover |
| Background highlighting for duplication | Too intrusive always-on; replaced with hover-activated subtle border |
| Overview ruler marks | Multiple colors create meaningless rainbow; diagnostics already appear in overview ruler via the standard diagnostic rendering |
| 12 simultaneous visual treatments | Reduced to 4, all opt-in |

---

### Phase 5: Compact sidebar summary

**Goal:** Give tech leads a glanceable project health overview without leaving the editor. Not a dashboard — a widget.

**5a. WebviewViewProvider in sidebar** (effort: M, 2-3 weeks)

Compact panel in the Fallow sidebar (below the tree view):

- Health score gauge (0-100, color-coded arc)
- Issue count breakdown (sparkline or mini bar chart)
- Duplication percentage
- Top 3 hotspot files (clickable)
- Trend sparkline (if snapshot history exists)
- "Last analyzed: 2 minutes ago" timestamp

**Design constraints:**
- No interactivity beyond clicking items to navigate to files
- Must work in light, dark, and high-contrast themes
- Updates on analysis completion (no polling)
- Max 200px height in sidebar — compact, not sprawling
- Store historical scores in `globalState` for trend data (lightweight, ~100 bytes per snapshot)

**What this is NOT:**
- Not a force-directed graph (cut — low sustained engagement, high engineering cost, unusable at >200 nodes)
- Not a duplication heatmap (cut — niche, expensive to build)
- Not a full webview dashboard panel (cut — opened once, forgotten)

The sidebar widget succeeds if it answers one question on a glance: "Is my project healthy?"

---

### Phase 6: Power user features

**Goal:** Features for advanced users who have already adopted fallow and want deeper integration. All opt-in.

**6a. Testing API — rules as tests** (effort: M, 1-2 weeks)

Each Fallow rule appears in VS Code's Test Explorer:

```
▶ Fallow
  ▶ Unused Exports ✕ (3 findings)
    ▸ ✕ foo in src/utils.ts:12
    ▸ ✕ bar in src/helpers.ts:34
    ▸ ✕ baz in src/legacy.ts:56
  ▶ Unused Files ✕ (2 findings)
    ▸ ✕ src/old-parser.ts
    ▸ ✕ src/deprecated.ts
  ▶ Circular Dependencies ✓
  ▶ Code Duplication ✓
```

**Why this matters:** Maps to a mental model developers already have — red means fix it, green means clean. The Test Explorer is an established surface that users already know how to use (run, filter, navigate). Research shows this drives adoption more than custom visualizations.

- Pass = zero findings for that rule
- Fail = findings exist, each as a child test case with location
- Gutter run/fail icons in affected files
- Continuous run mode maps to `fallow watch`
- CI integration: same semantics as `fallow check` exit codes

**6b. Inlay hints — reference counts** (effort: S, days)

Off by default (`fallow.display.inlayHints.referenceCount`):

- Show `N refs` after export declarations: `export const processData = ... // 5 refs`
- Pre-computed from cached export usage data — hash map lookup in provider
- No complexity inlay hints (hover is the right surface)
- No "unused"/"circular" inlay hints (diagnostics already handle this)

**6c. Import tree view** (effort: M, 1-2 weeks)

Custom tree view (not Call Hierarchy API — avoids "callers"/"callees" label mismatch):

- `fallow.showImports` command on any file
- "Imported by" (fan-in): which files import this module
- "Imports from" (fan-out): which modules this file imports
- Circular dependency chains highlighted with icon
- Available from editor context menu and command palette

Serves the "why is this export considered used?" debugging use case — trace the import chain from entry point to this file.

---

### Phase 7: Polish & scaling

**Goal:** Stability, performance, and validation before expanding scope.

- **Performance profiling** across all features — identify and fix jank from decoration/provider calls
- **Multi-root workspace** polish — aggregate vs. per-root status bar, grouped tree view sections, per-workspace file decorations
- **Extension activation** optimization — lazy loading via `onLanguage:typescript` (not `*`), dynamic import for Phase 5+ features
- **Telemetry** for feature engagement — privacy-respecting, opt-in, local-only counters. Which features do users actually enable? Use this to decide what to invest in next.
- **Evaluate webview dashboard** — if sidebar widget adoption is high and users request more detail, consider a full panel. Not before.

---

## Scoring matrix

Scored on **Adoption impact** (does this drive daily usage?), **Health surfacing** (does this expose health data?), **Effort** (engineering cost), and **Risk** (performance, maintenance burden, UX noise). Each 1-10.


| # | Feature | Adoption | Health | Effort (lower=easier) | Risk | Phase | Status |
|---|---------|----------|--------|----------------------|------|-------|--------|
| 1 | Walkthrough (Getting Started) | **10** | 2 | **2** | 1 | 1a | Shipped |
| 2 | DiagnosticTag.Unnecessary | **9** | 3 | **1** | 1 | 1b | Shipped |
| 3 | Diagnostic doc links | **8** | 2 | **1** | 1 | 1b | Shipped |
| 4 | relatedInformation for cycles | 6 | 4 | **2** | 1 | 1b | Shipped |
| 5 | Status bar polish + tooltip | 7 | 5 | **2** | 1 | 1c | Shipped |
| 6 | Progress API | 5 | 1 | **2** | 1 | 1d | Shipped |
| 7 | JSON Schema for config | 6 | 1 | **1** | 1 | 1e | Shipped |
| 8 | Consolidated tree view | **8** | **8** | 5 | 2 | 2a | Open |
| 9 | **Batch operations** | **9** | 3 | 5 | 3 | 2b | Open |
| 10 | Quick Pick navigator | **8** | 5 | **3** | 1 | 2c | Open |
| 11 | Suppress code actions | 7 | 2 | **2** | 1 | 2d | Open |
| 12 | FileDecorationProvider | 7 | 4 | 4 | 5 | 3a | Open |
| 13 | Code lens (actionable) | 6 | 3 | **3** | 2 | 4a | Open |
| 14 | Complexity in hover | 5 | **7** | **3** | 2 | 4b | Open |
| 15 | Gutter indicators | 4 | 3 | **2** | 2 | 4c | Open |
| 16 | Duplication indicators | 4 | 4 | **3** | 3 | 4d | Open |
| 17 | Sidebar summary widget | 6 | **8** | 6 | 4 | 5a | Open |
| 18 | **Testing API (rules as tests)** | **8** | 5 | 5 | 3 | 6a | Open |
| 19 | Inlay hints (ref counts) | 4 | 3 | **3** | 4 | 6b | Open |
| 20 | Import tree view | 5 | 5 | 5 | 2 | 6c | Open |

Effort: 1-3 = days, 4-6 = weeks, 7-10 = months.

**Reading the matrix:** Features that score high on Adoption with low Effort are no-brainers: walkthrough (#1), DiagnosticTag (#2), doc links (#3), Quick Pick (#10). Features that score high on Health surfacing are the differentiation play: consolidated tree (#8), sidebar widget (#17), complexity hover (#14). Features with high Risk need careful implementation: FileDecorationProvider (#12, performance-critical), inlay hints (#19, viewport-change frequency).

---

## Dependency graph

```
Phase 0 (data architecture) ──→ everything else
                               │
Phase 1 (foundation) ──────────┤
                               │
Phase 2 (actions) ─────────────┤──→ Phase 3 (file explorer)
                               │
                               ├──→ Phase 4 (editor enhancements)
                               │
                               ├──→ Phase 5 (sidebar widget)
                               │         needs health data from Phase 0
                               │
                               └──→ Phase 6 (power user)
                                          │
Phase 7 (polish) ──────────────────────────┘
```

Phases 3, 4, 5, 6 are independent of each other and can ship in parallel or any order after Phase 2. Phase 5 needs the health data LSP extensions from Phase 0 to be meaningful.

---

## LSP changes required

| Phase | Change | File | Effort |
|---|---|---|---|
| 0 | Pre-computed file → issues index | `crates/lsp/src/main.rs` | M |
| 0 | `fallow/analysisComplete` notification | `crates/lsp/src/main.rs` | S |
| 0 | `fallow/fileMetrics` request handler | `crates/lsp/src/main.rs` | S |
| 0 | `fallow/healthSummary` request handler | `crates/lsp/src/main.rs` | S |
| 1b | DiagnosticTag + code URLs + relatedInfo | `crates/lsp/src/diagnostics.rs` | S |
| 4b | Function complexity in hover | `crates/lsp/src/hover.rs` | S |

All LSP changes are additive — no breaking changes to existing protocol.

---

## What was cut (and why)

Maintaining this list prevents re-proposing features that were deliberately rejected.

| Feature | Why cut | Revisit when |
|---|---|---|
| Force-directed dependency graph (webview) | <5% sustained engagement in studies. Unusable at >200 nodes (real monorepos have thousands). Engineering cost disproportionate to value. | User demand after sidebar widget ships; build on viz CLI (Phase 4 of health roadmap) instead. |
| Duplication heatmap matrix | Niche use case, high engineering cost, low discoverability. | Never — the tree view with clone families serves this need. |
| Full webview dashboard panel | "Opened once, forgotten." Destination features don't work in flow-oriented tools. | Only if sidebar widget shows high engagement AND users request more detail. |
| Comments API for refactoring suggestions | Wrong API — designed for collaborative review threads, not automated suggestions. Renders with avatars and reply boxes. | Never — use code actions with `CodeActionKind.RefactorExtract` instead. |
| Call Hierarchy for imports | VS Code labels it "callers"/"callees" — can't customize. Semantic mismatch confuses users. | Never — custom tree view gives full label control. |
| DiagnosticTag.Deprecated for type-only deps | "Deprecated" means API is being phased out, not "should be devDependency." Wrong semantics. | Never — use Unnecessary or plain Warning. |
| Two status bar items | Status bar real estate is precious; users hate crowded status bars. | Never — rich tooltip handles density. |
| Custom config editor (webview) | High maintenance: every config change requires updating both schema AND custom UI. JSON Schema + native VS Code JSON support provides 80% of value at 5% of cost. | Never — JSON Schema is strictly better for maintenance. |
| Inlay hints for complexity | Persistent inline noise for informational data. Hover is the right surface — shows when you want it, invisible when you don't. | Only if users explicitly request it after using complexity hover. |
| Timeline Provider | Very low discoverability (collapsed by default in Explorer). Requires historical data storage infrastructure. | After trend tracking (health roadmap Phase 2) ships and stores snapshots. |
| Semantic tokens for unused code | DiagnosticTag.Unnecessary already provides fade styling through the standard diagnostic system. Custom semantic tokens would duplicate this. | Never. |
| Notebook support | Jupyter/Deno notebooks are edge case for the target audience. | When usage data shows significant notebook adoption in Fallow user base. |

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Extension becomes too noisy, users disable it | Medium | Critical — kills adoption | Progressive disclosure: minimal defaults, all visual features opt-in. Ship quiet. |
| FileDecorationProvider causes explorer jank | Medium | High — visible to all users | Pre-computed index, O(1) lookups, specific URI change events. Profile before shipping. |
| LSP custom methods create tight coupling | Low | Medium — harder to support other editors | Keep custom methods optional (extension works without them, just with less data). Standard LSP features (diagnostics, code actions, hover, code lens) remain primary. |
| Webview sidebar widget maintenance burden | Medium | Medium — themes, CSP, state persistence | Keep it simple: no interactivity, no frameworks, plain HTML/CSS. Test all three theme modes. |
| Health data staleness confuses users | Medium | Medium — erodes trust | Show "last analyzed" timestamp. Don't show partial data — either full analysis or nothing. |
| Feature creep from user requests | High | Medium — scope bloat, maintenance | This roadmap is the scope. New features must replace existing ones in the plan or score higher on the matrix. |

---

## Relationship to other roadmaps

- **[Health roadmap](roadmap-health.md):** The extension roadmap depends on health features shipping in the CLI/LSP first. Phase 2a (consolidated tree with health sections) needs file-level health scores (health Phase 1a — shipped). Phase 5a (sidebar widget with hotspots) needs hotspot analysis (health Phase 1b — shipped). Phase 5a trend sparkline needs snapshot history (health Phase 2a — open).
- **[Viz spec](viz-spec.md):** The health roadmap Phase 4d proposes VS Code webview integration for the viz CLI. This extension roadmap deliberately does NOT include that — the sidebar widget (Phase 5a) is the extension's health surface. If viz webview ships, it would be a separate command (`fallow.showVisualization`) that opens the viz HTML in a webview panel, independent of this roadmap.
- **Cloud product:** The extension could surface cloud dashboard links in the sidebar widget and status bar tooltip, but that's a Phase 7+ concern. Build the local experience first.

---

## Design principles

1. **One canonical home per datum.** Every piece of information gets one primary surface and at most one ambient indicator. Not nine.
2. **Actionable over informational.** If the user can't *do* something about it, don't show it persistently. Put it in hover or on-demand views.
3. **Progressive disclosure.** Ship quiet. Let power users turn up the volume. Minimal defaults, explicit opt-in.
4. **Performance is a feature.** Pre-compute everything. O(1) in providers. Never block the UI thread. Never fire `onDidChangeFileDecorations(undefined)`.
5. **Ship, measure, iterate.** Let user demand drive later phases. The worst outcome is building all phases and discovering users only wanted Phase 1.
6. **The extension is a retention tool.** Every hour a developer spends in the editor with Fallow visible is an hour of habit formation. Habits drive adoption. Adoption drives conversion. Don't break the habit by being annoying.
