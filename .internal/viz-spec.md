# `fallow viz` — Architecture Specification

> Last updated: 2026-03-25
> Reviewed by expert panel: 2026-03-25

---

## Overview

Self-contained interactive HTML reports with zero external dependencies. The primary output is a single HTML file that opens in any browser — no Node.js, no Graphviz, no server required. Secondary formats (DOT, Mermaid, JSON) available for CI integration, PR comments, and custom tooling.

**Strategic position:** Viz is Phase 4 in the [business roadmap](roadmap-health.md) — a marketing/acquisition tool (Revenue 3/10, Acquisition 9/10, Uniqueness 10/10). The treemap dead code overlay is unique in the entire JS/TS ecosystem. No other tool answers "where is my dead code?" visually. This is the screenshot that goes viral on Hacker News and the demo that makes a tech lead say "I need to try this."

---

## Expert panel review (2026-03-25)

Reviewed by: data visualization engineer (ex-Observable/D3), Rust systems engineer (cargo-tarpaulin contributor), frontend performance specialist (ex-Chrome DevTools), developer tools PM (ex-GitHub/GitLab/Vercel), UX researcher (JetBrains), monorepo architect end-user (5200-file TypeScript monorepo).

| Issue | Consensus |
|-------|-----------|
| LOC estimates too low | Interaction complexity (text rendering, pan/zoom momentum, animation) adds 40-60%. Realistic total: ~2500 LOC (not 1500), ~35-45KB minified (not 15-25KB). Still well within acceptable limits. |
| Force-directed layout as default | Force-directed hides import directionality. Use hierarchical (Sugiyama) as default, force-directed as toggle. |
| Data payload splitting | Split `VizData` into compact layer (treemap) and detail layer (graph). Treemap-only users get ~200KB, not 5MB. |
| Mermaid misplaced in 4c | Mermaid cycle output is string formatting (~80 LOC). Ship with treemap MVP (4a), not as a separate phase. |
| VS Code integration premature | The standalone HTML already works in VS Code's Simple Browser. Defer 4d indefinitely until user demand is validated. |
| Missing complexity data | Treemap colored by complexity is the #1 requested view for engineering managers. Add optional `complexity` field to `VizFile`. |
| Duplication heatmap as separate view | Low incremental value. Make duplication an overlay on the existing treemap (already allowed via `--include-dupes`). |
| Missing export-to-PNG | Developers share screenshots in PRs and Slack. `canvas.toDataURL()` is ~15 LOC with outsized impact. |
| Missing shareable state | Encode view state (zoom, focus, filters) in URL hash for team sharing. |
| Missing onboarding | Treemaps are NOT intuitive. Need color legend + brief overlay for first-time users. |
| Feature-gate in Cargo | CI users shouldn't pay binary size for embedded viz assets. Use `--features viz`. |

---

## CLI

```bash
fallow viz                                        # HTML treemap, opens browser
fallow viz -o report.html                         # Save to specific file
fallow viz --no-open                              # Generate without opening
fallow viz --focus src/api/ --depth 3             # Subgraph around a module
fallow viz --focus src/api/ --direction upstream   # What depends on this module?
fallow viz --workspace my-package                 # Scope to workspace
fallow viz --unused-only                          # Only show unused code subgraph
fallow viz --include-dupes                        # Include duplication overlay
fallow viz --include-complexity                   # Include health/complexity data (runs health analysis)
fallow viz --exclude "__tests__/**"               # Exclude patterns from viz
fallow viz --min-size 500                         # Hide files smaller than 500 bytes
fallow viz --format dot | dot -Tsvg > graph.svg   # Graphviz DOT output
fallow viz --format mermaid --cycles-only         # Mermaid for PR comments
fallow viz --format mermaid --max-nodes 40        # Truncate for GitHub rendering limits
fallow viz --format json > graph.json             # Raw graph data for custom tooling
fallow viz --baseline .fallow/baseline.json       # Include trend delta in summary stats
```

### Flags

- `--format html|dot|mermaid|json` — output format (default: `html`). Note: uses a separate `VizFormat` enum, not the global `--format` (which accepts `human|json|sarif|compact|markdown`). The global `--format` flag is ignored for the `viz` subcommand.
- `--output <path>` / `-o` — output file path (default: temp file for HTML, stdout for others)
- `--no-open` — don't auto-open browser (HTML format)
- `--focus <path>` — subgraph rooted at a specific module or directory
- `--direction downstream|upstream` — focus direction (default: `downstream`). Downstream = "what does this import?" Upstream = "what imports this?"
- `--depth N` — limit traversal depth from focus point (default: 3)
- `--cluster-by directory|package` — group nodes by folder or workspace package
- `--include-external` — show npm package nodes (collapsed by default)
- `--workspace <name>` — scope to a single workspace package. Cross-workspace edges are shown but visually de-emphasized (dashed lines).
- `--unused-only` — filter to unused code and its connections only
- `--cycles-only` — show only circular dependency cycles (especially useful with `--format mermaid`)
- `--include-dupes` — include duplication data as overlay
- `--include-complexity` — include per-file complexity scores (triggers health analysis pipeline)
- `--exclude <pattern>` — glob pattern to exclude from visualization (repeatable)
- `--min-size <bytes>` — hide files smaller than this threshold (reduces visual noise from config/barrel files)
- `--max-nodes N` — truncate output for formats with rendering limits (Mermaid/DOT). Default: unlimited for HTML, 50 for Mermaid, 200 for DOT.
- `--baseline <path>` — path to a baseline snapshot. Summary stats show deltas ("12% dead code, down from 15%").

### Error handling

- **Output path not writable:** Print error with suggested alternative path.
- **No browser available (headless CI):** Print the output file path to stdout. Never fail — `--no-open` is implied in non-interactive terminals.
- **Temp directory full:** Fall back to writing in current directory as `fallow-report.html`.
- **Analysis failure:** Degrade gracefully — generate viz with available data, show warning banner in the report.

---

## Architecture

### Rust-side

Module directory pattern (matches existing codebase conventions):

```
crates/cli/src/
  viz/
    mod.rs          — subcommand definition, flag parsing, orchestration
    serialize.rs    — ModuleGraph → VizData conversion (path normalization, workspace handling)
    html.rs         — HTML generation with embedded assets
    dot.rs          — DOT output with directory clustering (~200 LOC)
    mermaid.rs      — Mermaid output for cycles (~80 LOC)
```

**Pipeline integration:** The `viz` command needs data that the current pipeline doesn't fully expose:

1. **`ModuleGraph`** — already computed by `fallow_core::analyze()` but optionally returned in `AnalysisOutput`. For viz, always retain it.
2. **`DiscoveredFile` list** — currently consumed during analysis and not returned. Must be threaded through `AnalysisOutput` or re-discovered. Needed for `size_bytes` and path data.
3. **Complexity data** — currently a separate pipeline (`health` command). For `--include-complexity`, either merge pipelines or run health analysis as a second pass. Recommendation: second pass — keeps pipelines independent, complexity is optional.
4. **`export_usages`** — currently `#[serde(skip)]` in `AnalysisResults`. Viz serializer must explicitly include it for reference counts.

This requires a moderate refactoring of `AnalysisOutput` to retain `DiscoveredFile` data. Not trivial, but not a redesign — it's adding a field and threading it through.

**Asset embedding:**

- HTML template with embedded JS/CSS via `include_str!("viz-assets/viz.min.js")` — same pattern as cargo-tarpaulin's HTML coverage reports
- Data injected as `window.__FALLOW_DATA__` in a script tag (compact layer) and `window.__FALLOW_DETAIL__` (detail layer, only when graph view is included)
- Auto-opens via `open` crate behind `viz` feature flag
- No compression of embedded data — raw JSON in script tag. For 1000-file projects (80% case), data is 200-500KB. For 5000-file monorepos, 3-5MB. Both are acceptable. Compression adds runtime dependency (pako) and complexity for marginal gain.

**Cargo feature flag:**

```toml
[features]
default = []
viz = ["dep:open"]
```

CI users install without viz: `cargo install fallow` (no extra binary size for embedded assets). Local users: `cargo install fallow --features viz`. The `viz` subcommand prints a helpful error when invoked without the feature enabled.

### Frontend

Developed separately in `viz-frontend/`, bundled via esbuild:

- Custom Canvas-based rendering — no Cytoscape, no D3, no heavy libraries
- Strip treemap layout algorithm (aspect-ratio-adaptive variant of Squarify — Bruls et al. 2000, ~120 LOC)
- Hierarchical layout (simplified Sugiyama — ~300 LOC) as default graph layout
- Force-directed layout (~250 LOC) as optional toggle
- Pan/zoom/pinch with momentum scrolling, zoom-to-cursor, trackpad vs. touch handling (~450 LOC)
- Spatial hit-testing: rect intersection for treemap (~50 LOC), quadtree for graph (~150 LOC)
- Canvas text rendering: multi-line truncation, font-size selection by rect dimensions, contrast-aware text color (~250 LOC)
- Drill-down animation: cubic-bezier easing, position interpolation, 250ms duration (~150 LOC)
- Preact for minimal UI shell (sidebar, toolbar, search, legend, onboarding) — 8KB vs React's 131KB
- Web Worker for graph layout simulation
- esbuild bundles to single JS + CSS, checked into `crates/cli/viz-assets/` (no Node.js needed for `cargo build`)

### Realistic size estimates

| Component | LOC | Minified (KB) |
|-----------|-----|---------------|
| Treemap (layout + render + labels + drill-down animation) | ~520 | ~8 |
| Graph (hierarchical + force-directed + render + interactions) | ~700 | ~12 |
| Pan/zoom/pinch (momentum, trackpad, touch, zoom-to-cursor) | ~450 | ~4 |
| Hit testing (quadtree for graph, rect test for treemap) | ~200 | ~2 |
| Search (fuzzy match + zoom-to-result) | ~100 | ~2 |
| Worker communication protocol | ~80 | ~1 |
| Export-to-PNG | ~20 | ~0.3 |
| URL hash state encoding/decoding | ~60 | ~1 |
| Context menu (copy path, open) | ~50 | ~0.5 |
| Preact (UI shell) | — | ~8 |
| CSS | — | ~4 |
| **Total** | **~2200** | **~43** |

For comparison: Cytoscape.js alone is 380KB, D3.js is 250KB, ECharts treemap is 300KB+. 43KB is small enough to be noise in the total HTML file size (which is dominated by the data payload).

**Note:** The 43KB estimate includes both treemap AND graph code. Since both ship in the same self-contained HTML file, they cannot be lazy-loaded from a CDN. If only the treemap (4a) ships initially, the code is ~24KB.

### HTML file size targets

| Project size | Code (KB) | Data (KB) | Total HTML |
|---|---|---|---|
| Small (100 files) | 24-43 | 20-50 | ~70-100KB |
| Medium (1000 files) | 24-43 | 200-500 | ~250-550KB |
| Large (5000 files) | 24-43 | 1-5MB | ~1-5MB |

All within acceptable browser loading times. `JSON.parse` for 5MB takes ~20-40ms on modern hardware — acceptable but should be benchmarked.

---

## Data model

### Two-layer payload

Split into compact (treemap) and detail (graph) to optimize initial load time for large projects. The treemap needs only per-file aggregates; the graph needs per-edge symbol data.

**Compact layer** (`window.__FALLOW_DATA__`): always present.

```rust
pub struct VizCompact {
    pub version: String,
    pub elapsed_ms: u64,
    pub project_root: String,
    pub files: Vec<VizFile>,
    pub issues: VizIssueSummary,          // Per-file issue counts (not full AnalysisResults)
    pub cycles: Vec<Vec<u32>>,            // Pre-computed cycle FileIds
    pub stats: VizStats,                  // Aggregate stats for summary banner
    pub baseline_delta: Option<VizDelta>, // Change since baseline, if --baseline provided
}

pub struct VizFile {
    pub id: u32,
    pub path: String,                     // Relative to project_root
    pub size_bytes: u64,
    pub is_entry_point: bool,
    pub is_reachable: bool,
    pub workspace: Option<String>,
    pub issue_count: u16,                 // Total issues in this file
    pub unused_export_count: u16,         // Unused exports specifically (for color)
    pub is_unused_file: bool,             // Entire file is dead
    pub complexity: Option<VizComplexity>, // Only with --include-complexity
}

pub struct VizComplexity {
    pub cyclomatic_sum: u32,
    pub cognitive_sum: u32,
    pub function_count: u16,
    pub density: f32,                     // cyclomatic / LOC
    pub maintainability: f32,             // 0-100 index
}

pub struct VizIssueSummary {
    pub per_file: Vec<VizFileIssues>,     // Indexed by FileId
}

pub struct VizFileIssues {
    pub unused_exports: Vec<String>,      // Export names
    pub unused_types: Vec<String>,
    pub unresolved_imports: Vec<String>,
    pub unused_members: Vec<String>,
    pub has_duplication: bool,
    pub in_cycle: bool,
}

pub struct VizStats {
    pub total_files: u32,
    pub unused_files: u32,
    pub unused_exports: u32,
    pub dead_code_pct: f32,
    pub duplication_pct: Option<f32>,     // Only with --include-dupes
    pub avg_maintainability: Option<f32>, // Only with --include-complexity
    pub hotspot_count: Option<u32>,       // Only with --include-complexity
}

pub struct VizDelta {
    pub dead_code_pct_change: f32,        // e.g., -3.0 means "down 3%"
    pub unused_files_change: i32,
    pub unused_exports_change: i32,
    pub baseline_date: String,
}
```

Estimated size for compact layer: ~100KB for 1000 files, ~500KB for 5000 files.

**Detail layer** (`window.__FALLOW_DETAIL__`): only included when graph view data is needed (4b).

```rust
pub struct VizDetail {
    pub edges: Vec<VizEdge>,
    pub exports: Vec<VizExport>,
    pub duplication: Option<VizDuplication>,
}

pub struct VizEdge {
    pub source: u32,                      // FileId
    pub target: u32,
    pub symbols: Vec<String>,
    pub is_type_only: bool,
    pub is_cross_workspace: bool,         // For visual de-emphasis
}

pub struct VizExport {
    pub file_id: u32,
    pub name: String,
    pub is_type_only: bool,
    pub reference_count: u16,
    pub is_unused: bool,
    pub line: u32,
}

pub struct VizDuplication {
    pub clone_families: Vec<VizCloneFamily>,
    pub stats: DuplicationStats,          // Re-use existing type
}

pub struct VizCloneFamily {
    pub files: Vec<u32>,                  // FileIds involved
    pub total_duplicated_lines: u32,
    pub instance_count: u16,
}
```

Estimated size for detail layer: ~100-400KB for 1000 files, ~1-4MB for 5000 files (dominated by edge symbol data).

**Frontend data access:** Index into the original parsed arrays directly. Do not transform into a new object graph — this creates GC pressure at scale. Use `files[edge.source]` lookups.

---

## Views

Delivered incrementally. Each sub-phase is independently shippable.

### Sub-phase 4a: Treemap + text formats (MVP)

**Scope:** Canvas treemap, DOT output, Mermaid cycle output, JSON topology format. This is the marketing material — the screenshot that goes viral.

**Treemap rendering (Canvas):**

Files sized by `size_bytes`, colored by status:

| Status | Color | Hex | Priority |
|--------|-------|-----|----------|
| Unused file | Red | `#EF4444` | 1 (highest) |
| Has unused exports | Amber | `#F59E0B` | 2 |
| Entry point | Green | `#10B981` | 3 |
| Clean (no issues) | Blue | `#3B82F6` | 4 (lowest) |

Priority determines color when a file has multiple statuses (e.g., an entry point with unused exports shows amber, not green).

Optional color modes (toolbar toggle):
- **Status** (default): the table above
- **Complexity**: gradient from green (low) to red (high density) — requires `--include-complexity`
- **Duplication**: gradient by duplication density — requires `--include-dupes`

Palette tested for protanopia/deuteranopia distinguishability. Color mode persisted in URL hash.

**Layout algorithm:** Strip variant of Squarify (adapts to container aspect ratio, not the fixed-ratio original paper). ~120 LOC.

**Canvas text rendering:** Multi-line truncation within rectangle bounds. Font size selected by rectangle dimensions (3 tiers: 14px for large rects, 11px for medium, 8px for small). Contrast-aware text color (white on dark rects, dark on light rects). Labels only rendered when rectangle exceeds minimum pixel threshold (60×30px). ~250 LOC.

**Interactions:**

| Action | Behavior |
|--------|----------|
| Click | Select file/directory → show details in sidebar |
| Double-click | Drill down into directory (re-root + re-layout with animation) |
| Hover | Tooltip: relative path, human-readable size, issue summary |
| Right-click | Context menu: "Copy path", "Open in terminal" (if applicable) |
| `/` | Search box with fuzzy matching → zoom-to-result in treemap |
| Arrow keys | Navigate siblings within current directory |
| Enter | Drill down into selected directory |
| Escape | Go up one level (breadcrumb back) |
| `P` | Export current view as PNG |
| `Tab` | Move focus between sidebar and treemap (accessibility) |

Click = select, double-click = drill-down. This matches file-explorer conventions and avoids accidental navigation.

**Drill-down animation:** Compute new layout at clicked rectangle's bounds. Store both old and new positions. Lerp between them over 250ms with cubic-bezier easing. Animate color transitions for directories that expand to reveal differently-colored children. Two-frame render: compute layout in frame 1, begin animation in frame 2 (prevents initial jank). ~150 LOC.

**Breadcrumb trail:** Rendered in Preact UI shell (not in Canvas — text measurement in Canvas is not worth the complexity). Shows clickable path segments: `root / src / components / forms`. Click any segment to navigate up.

**Summary stats banner:**

```
┌─────────────────────────────────────────────────────────────────────┐
│  1,247 files  ·  47 unused files (3.8%)  ·  128 unused exports     │
│  12.3% dead code  ↓3.0% since baseline  ·  analyzed in 340ms      │
└─────────────────────────────────────────────────────────────────────┘
```

The banner is the hook — "12.3% of your code is dead" is what makes someone share this. Baseline delta shown when `--baseline` is provided. Positioned above the treemap, always visible.

**Sidebar (Preact):**
- **Color legend** — always visible, showing what each color means. Not a tooltip, not a modal.
- **Selected file details** — on click: relative path, size, issue list (unused exports by name, unresolved imports, cycle membership), complexity scores if available
- **Onboarding overlay** — shown once on first load (stored in `localStorage`). Three steps: "This is a treemap of your codebase. Red = dead files. Amber = has unused exports. Click to explore." Dismissible, never shown again.

**Dark mode:** Match VS Code's default dark theme colors (`#1e1e1e` background, `#d4d4d4` text). Toggle via `prefers-color-scheme` + manual button. Developers spend hours in VS Code dark — if the viz uses different shades, it feels wrong.

**Export-to-PNG:** Button in toolbar. Captures current Canvas state via `canvas.toDataURL('image/png')`. Adds the summary stats banner and legend as overlays. ~20 LOC. Critical for shareability — developers paste screenshots in PRs, Slack, and decks.

**Shareable URL hash:** Encode current view state in the URL hash: `#view=treemap&focus=src/api&color=status&theme=dark`. When someone receives the HTML file + hash, it opens to the same view. Zero server infrastructure needed. ~60 LOC.

**DOT output** (~200 LOC):
- Directory clustering via `subgraph cluster_*`
- Node attributes: shape, color, label (filename)
- Edge weight hints for layout tools
- Respects `--cluster-by`, `--unused-only`, `--max-nodes`
- Piped to `dot -Tsvg` or `dot -Tpng` for static images

**Mermaid output** (~80 LOC):
- Cycle diagrams only by default (`--cycles-only` implied for Mermaid)
- GitHub renders Mermaid in PRs/issues/comments up to ~50 nodes
- Auto-truncation at `--max-nodes` (default: 50 for Mermaid)
- Useful for CI: `fallow viz --format mermaid >> $GITHUB_STEP_SUMMARY`

**JSON output:**
- Full `VizCompact` + `VizDetail` as a single JSON document
- Schema version for forward compatibility
- Enables custom tooling, internal dashboards, and integration with other viz tools

**Performance targets:**

| Project size | Layout (ms) | First render (ms) | Drill-down animation (ms) |
|---|---|---|---|
| 100 files | <1 | <5 | <16 (single frame) |
| 1000 files | <5 | <10 | <32 (2 frames) |
| 5000 files | <30 | <50 | <50 |

Benchmark on real monorepo before shipping. Don't discover performance cliffs in production.

### Sub-phase 4b: Dependency graph view

Second view tab in the same HTML report. Ships separately after treemap is validated.

**Graph rendering (Canvas 2D):**
- Nodes = files (circles), sized by export count or file size
- Edges = import relationships with directional arrows
- Color = same scheme as treemap (re-uses color mode toggle)
- Entry points = diamond marker
- Cross-workspace edges = dashed lines, lower opacity (visually de-emphasized)

**Layout — hierarchical as default:**

Hierarchical layout (simplified Sugiyama, ~300 LOC) as the default. Produces a clean top-to-bottom DAG that makes import direction immediately obvious — edges flow downward. This is critical for a tool about understanding dependency structure.

Force-directed layout available as a toggle in the toolbar. Runs in Web Worker (~250 LOC). Useful for exploring connection density, but hides directionality.

Layout switcher in toolbar: `Hierarchical | Force-directed`.

**Web Worker protocol (for force-directed):**

```
Main thread                          Worker
    │                                    │
    ├─── { type: 'init', nodes, edges } ─→
    │                                    │
    │ ←── { type: 'tick', positions[] } ─┤  (30fps)
    │                                    │
    │ ←── { type: 'converged' } ────────┤
    │                                    │
    ├─── { type: 'stop' } ──────────────→
```

Main thread interpolates between received positions and renders at 60fps. Worker sends position arrays (not individual node objects — avoids allocation overhead). Converged signal stops the animation loop.

**Directory clustering (critical for scale):**
- Default: show ONLY top-level directory clusters. Don't show individual files until the user explicitly drills down. This prevents the "hairball graph" problem.
- Click cluster to expand one level (progressive disclosure: package → directory → file)
- Edge count labels on collapsed inter-directory edges
- At any zoom level, visible node count stays manageable (<100 nodes)

**Focus mode:**
- Click node → "Focus" button → subgraph with configurable depth
- **Direction toggle:** "Dependencies" (downstream — what does this import?) vs. "Dependents" (upstream — what imports this?). Default: downstream.
- `--focus` and `--direction` CLI flags for pre-focused output
- Breadcrumb for focus path, "Clear focus" to return

**Hit testing:** Quadtree spatial index for circle-based nodes (~150 LOC). Rebuilt on layout change. O(log n) lookup on hover/click.

**Additional interactions:**
- Minimap: rendered as a static snapshot, updates on pan/zoom (not on every frame — avoids doubling render cost)
- Filter bar with toggle buttons showing counts: `Unused (47) | Cycles (3) | Entry Points (12)`. Zero-count filters grayed out.
- Edge highlighting on hover (show all imports to/from a file, dim all others)

### Sub-phase 4c: Cycle diagrams + duplication overlay

Ships only if user demand exists after 4a/4b.

**Cycle diagrams:**
- Each cycle as a separate small diagram (max ~10 nodes per cycle)
- Numbered edges showing import order
- Sorted by impact: number of files affected, not cycle length. A 3-node cycle through core utilities is more important than a 10-node cycle in test helpers.
- Cycle-breaking suggestion: highlight the edge with highest betweenness centrality within the cycle (computed per-cycle, bounded by cycle size)
- Sidebar lists all cycles with file names, click to focus

**Duplication overlay on treemap (not a separate view):**
- When `--include-dupes` is active, add "Duplication" color mode to treemap toggle
- Treemap colored by duplication density (duplicated lines / total lines): green → yellow → red gradient
- Click file → sidebar shows clone instances with file paths and line ranges
- Cross-reference badge in sidebar: "unused + duplicated" items sorted to top

**What was cut (and why):**

| Proposed | Why cut |
|---|---|
| Separate duplication heatmap view | Low incremental value over the treemap overlay. The treemap with duplication coloring serves the same purpose with less engineering cost. |
| Source code fragments in duplication view | Requires syntax highlighting library (~50KB) or shows unreadable plain text. Developers should open the file in their editor instead. Show file path + line range, not code. |
| Clone family network diagram | Niche visualization that few users would understand. Clone families are better represented as a sorted list in the sidebar. |

### ~~Sub-phase 4d: VS Code webview integration~~ — DEFERRED

**Status:** Deferred indefinitely. Revisit only if standalone viz adoption is high AND users explicitly request IDE integration.

**Rationale:**
1. The standalone HTML already works in VS Code's Simple Browser panel. It renders perfectly — just lacks bidirectional editor integration.
2. VS Code webview integration is always harder than expected: CSP restrictions prevent inline scripts (which is exactly how the standalone HTML works), message passing adds complexity, state sync with the editor is a rabbit hole.
3. UX research finding: IDE-integrated visualizations are used less than standalone ones. Developers open them once, then close them. The standalone HTML, which can be bookmarked or shared, gets revisited more.
4. Engineering cost (3+ weeks) is disproportionate to value for a feature rated Revenue 3/10.

If demand materializes, the implementation approach would be: serve the viz HTML in a webview panel with `postMessage` bridge for click-to-open-file. But don't build it speculatively.

---

## Competitive position

| Capability | knip | madge | dep-cruiser | Nx graph | **fallow viz** |
|------------|------|-------|-------------|----------|----------------|
| Dead code treemap | — | — | — | — | **4a** |
| Interactive HTML | — | — | interactive SVG (pan/zoom/filter) | full React app | **4a** (~24-43KB) |
| Dependency graph | — | static SVG | static SVG | interactive | **4b** |
| Focus mode (up/downstream) | — | — | `--focus` (static) | interactive | **4b** |
| Directory clustering | — | — | `ddot` (static) | group toggle | **4b** |
| Cycle visualization | — | static SVG | highlight in graph | — | **4c** |
| Duplication overlay | — | — | — | — | **4a** (overlay) |
| DOT/Mermaid output | — | DOT | DOT + Mermaid | DOT | **4a** |
| Complexity coloring | — | — | — | — | **4a** (optional) |
| Export to PNG | — | — | — | — | **4a** |
| Shareable URL state | — | — | — | yes (web app) | **4a** (hash) |
| Zero runtime deps | — | needs Graphviz | needs Graphviz | needs Node.js | **yes** |
| Self-contained file | — | — | yes (~200KB) | no (web app) | **yes** |
| IDE integration | — | — | — | Nx Console | ~~4d~~ deferred |

**Honest assessment of dep-cruiser:** The spec previously undersold dep-cruiser's HTML output as "basic." It actually has interactive SVG with pan/zoom and filtering. But dep-cruiser requires Graphviz as a peer dependency and generates multi-megabyte SVG files for large projects. Fallow wins on: zero deps, smaller output, treemap view (unique), complexity overlay (unique), and Canvas performance at scale.

**The unique angle:** The treemap dead code overlay (4a) is unique in the entire JS/TS ecosystem. No other tool answers "where is my dead code?" visually. This is the acquisition driver. The dependency graph (4b) is useful but competitive with existing tools — fallow only beats them on the "zero deps, single file" angle.

---

## Phasing & effort

| Phase | Scope | Effort | Priority |
|-------|-------|--------|----------|
| **4a** | Canvas treemap + summary stats + drill-down + search + dark mode + export-to-PNG + shareable URL hash + onboarding + DOT + Mermaid + JSON | 4 weeks | **Ship first** |
| **4b** | Dependency graph (hierarchical default, force-directed toggle) + directory clustering + focus mode (up/downstream) + minimap + filter bar | 5 weeks | **Ship second** |
| **4c** | Cycle diagrams + duplication overlay on treemap | 2 weeks | **Ship if demand exists** |
| ~~**4d**~~ | ~~VS Code webview integration~~ | ~~3+ weeks~~ | **Deferred indefinitely** |

**What to cut if capacity is tight:** Ship 4a only. The treemap + DOT + Mermaid is the complete marketing story. The graph view (4b) adds depth but isn't required for the acquisition narrative. 4c and 4d are stretch goals.

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Interaction complexity underestimated (text rendering, animation, pan/zoom) | High | Medium — delays 4a by 1-2 weeks | LOC estimates already revised up 40-60%. Budget 5 weeks instead of 4 for 4a. |
| Force-directed graph looks terrible at scale ("hairball problem") | High | Medium — users dismiss graph view | Default to hierarchical layout. Directory clustering limits visible nodes. Force-directed is opt-in toggle. |
| Large monorepo performance (>5000 files) | Medium | High — unusable for target audience | Two-layer data split reduces initial parse. Benchmark on real monorepo before shipping. Canvas handles 10K+ rects. |
| Pipeline refactoring to expose ModuleGraph + DiscoveredFile | Low | Medium — blocks viz development | Refactoring is additive (new fields on AnalysisOutput), not a redesign. Estimate: 1-2 days. |
| Bundle size exceeds budget | Low | Low — 43KB is still tiny vs. alternatives | Code-split treemap-only (24KB) vs. treemap+graph (43KB). Both are negligible compared to data payload. |
| Mermaid output exceeds GitHub rendering limits | Medium | Low — bad rendering in PRs | Auto-truncation with `--max-nodes` (default: 50). Warning when truncated. |
| "Opened once, never returned" viz abandonment | Medium | Medium — wasted engineering effort | Treemap answers a concrete question ("where is dead code?") which research shows drives re-engagement. Summary stats with baseline delta give reason to revisit. |

---

## Relationship to other roadmaps

- **[Health roadmap](roadmap-health.md):** Viz is Phase 4 in the health roadmap. The `--include-complexity` flag depends on health Phase 1a (file-level health scores — shipped). The `--baseline` flag depends on health Phase 2a (vital signs snapshots — open).
- **[VS Code roadmap](roadmap-vscode.md):** The VS Code roadmap deliberately does NOT include viz webview integration. The sidebar widget (VS Code Phase 5a) is the extension's health surface. If viz webview ever ships, it would be a separate command independent of the VS Code roadmap.
- **Cloud product:** Viz JSON output is the data format the cloud dashboard would render. Building viz first validates the data model before investing in cloud infrastructure.
