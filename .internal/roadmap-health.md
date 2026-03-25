# Fallow — Internal Roadmap & Business Strategy

> Last updated: 2026-03-25

---

## The thesis

Fallow's module graph is expensive to build and cheap to query. Every new feature — hotspots, trends, dependency risk, visualization — is a cheap query on data we already compute. This makes fallow the natural aggregation point for JS/TS codebase intelligence.

The business model: **free CLI builds the user base, paid cloud aggregates insights across teams and time.** The CLI must be standalone-valuable — the cloud layer adds the time dimension (trends, regressions, team comparison) and the collaboration layer (dashboards, alerts, PR annotations) that justify a subscription.

**Strategic position:** Not a linter (oxlint is free), not SAST (needs type info we don't have), not SonarQube-for-JS (compliance-driven, crowded). Instead: the only tool that combines dead code + complexity + change history + dependency risk into a single fast picture of JS/TS codebase health.

---

## Business model

### Revenue tiers


| Tier           | What                                                                                          | Price model                   | Target                                             |
| -------------- | --------------------------------------------------------------------------------------------- | ----------------------------- | -------------------------------------------------- |
| **Free**       | CLI: check, dupes, health, viz, fix. Full-featured, no limits.                                | $0                            | Individual devs, open source projects              |
| **Team**       | GitHub App: PR annotations, trend dashboards, Slack alerts, team comparison, snapshot history | Per-repo/month (~$20-50/repo) | Tech leads, small teams (5-20 devs)                |
| **Enterprise** | Audit logs, custom policies, dependency risk reports, org-wide dashboards, SLA, SSO/SAML included | Per-seat/year (~$15-25/seat)  | Engineering directors, compliance teams (50+ devs) |

> **SSO note:** SSO/SAML should be included in all paid tiers, not gated at Enterprise. The SSO tax backlash is real (sso.tax, CISA "Secure by Design" guidance). Gating SSO hurts bottom-up adoption — the exact motion fallow depends on. Use "SSO included at every tier" as a competitive differentiator.


### Why per-repo for Team tier

Per-seat punishes the champion who adopts it (they pay, teammates benefit for free). Per-repo aligns with how teams think ("add fallow to this project") and is easy to start with. Per-seat only makes sense at enterprise scale where procurement wants headcount-based contracts.

### First dollar (before any cloud product)

1. **GitHub Sponsors / Open Collective** — immediate, low effort, depends on community goodwill
2. **Consulting** — monorepo migration audits, dead code cleanup for large codebases. $2-5K per engagement. Validates the market and builds relationships.
3. **Sponsored plugins** — framework teams (Nuxt, NestJS, etc.) pay to prioritize their plugin quality. Unlikely but worth exploring.

### Conversion funnel

```
npx fallow (free, zero-config)
  → User sees value, adds to CI
    → Team notices, enables rules system
      → Tech lead wants trends → needs cloud (Team tier)
        → Director wants org-wide view → Enterprise
```

**The critical gap:** The jump from "free CI tool" to "paying cloud customer" is the hardest conversion in developer tools. The cloud product must offer something the CLI fundamentally cannot: **aggregation across time and teams.** A dashboard that just re-renders JSON output will not convert.

### What the cloud must do that the CLI cannot

- **Historical trends** — the CLI stores snapshots locally but CI ephemeral runners don't persist `.fallow/`. The cloud stores every run automatically.
- **Cross-team comparison** — "the payments team's dead code ratio is rising while the auth team's is falling." Requires aggregation across repos/workspaces.
- **PR annotations** — inline comments on GitHub PRs showing new dead code, new hotspots, regressions. Note: bot fatigue is real in the PR comment space (GitHub added comment minimization specifically for this). Prefer pass/fail check status with file-level annotations over verbose PR comments. Keep it minimal.
- **Alerts** — "dead code percentage increased by 3% this sprint" via Slack/email.
- **Shareable reports** — a URL you send to your manager showing codebase health grade.

---

## Competitive landscape

> Last surveyed: 2026-03-24.

### Direct competitors


| Tool            | What they do                        | Fallow's edge                                                                                | Their edge                                                                                      |
| --------------- | ----------------------------------- | -------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| **Knip**        | Dead code detection (JS)            | 3-18x faster, duplication detection, health metrics, viz, 84 plugins with AST config parsing | Larger community, longer track record, JS-native (easier to contribute)                         |
| **Biome**       | Linting + formatting + module graph | 13 issue types vs their 1, duplication, framework plugins, health track                      | Bundled toolchain narrative, larger team, backed by investment                                  |
| **SonarQube**   | Enterprise code quality (30+ langs) | JS/TS depth, speed, framework awareness, dead code precision                                 | 15yr enterprise trust, compliance certifications, 30+ languages                                 |
| **CodeScene**   | Hotspot analysis, code health       | JS/TS specialization, open-source CLI, speed, dead code integration                          | Proven enterprise sales motion, organizational metrics, deeper git analysis (temporal coupling) |
| **CodeClimate** | Maintainability scores, trends      | Speed, framework awareness, active development                                               | Brand recognition (though stagnant)                                                             |


### Adjacent tools (not direct competitors)

- **oxlint** — per-file lint rules. Complementary. Same Oxc foundation.
- **Snyk / npm audit** — dependency security. Fallow doesn't compete but can leverage: unused dep + CVE = unique insight.
- **dependency-cruiser** — boundary rules. JS-only, slow. Fallow can absorb this use case.

### Honest threat assessment

- **Biome shipping unused exports** — would commoditize fallow's core feature. Timeline unknown but module graph infra exists. Mitigation: move up the value chain (health, hotspots, trends) before this happens.
- **AI code cleanup** — "just ask Copilot to remove dead code" may become good enough for small projects. Mitigation: fallow serves large monorepos where AI context windows can't hold the full graph.
- **SonarQube improving JS/TS** — they have resources. Mitigation: framework-aware precision they can't match without JS ecosystem expertise.

---

## Phase ordering — revenue-informed

The previous version ordered phases by engineering elegance. This version orders by: **what builds the user base fastest, what creates the most conversion pressure, and what unlocks paid tiers.**

### Phase 0: User base growth (ongoing, no new features needed)

The free CLI is the funnel entry. Priority actions:

- Ship 1.0 stable release
- Write the comparison blog post (fallow vs knip vs Biome)
- Add `fallow` to awesome-lists, Rust tooling directories
- Publish framework-specific guides ("Dead code cleanup for Next.js monorepos")
- VS Code extension marketplace screenshots and description polish

**Metric:** npm weekly downloads, GitHub stars, Discord/community size.

This is not engineering work — it's marketing work. But it's the prerequisite for everything else.

### Phase 1: Health intelligence — the differentiation layer

**Goal:** Transform fallow from "dead code finder" (commodity) into "codebase intelligence tool" (differentiated). This is what makes the cloud product worth building.

**Ship order within Phase 1:**

**1a. File-level health scores** (effort: S, days)

Extend the shipped complexity metrics with per-file scores. Low effort, high leverage — immediately gives tech leads a reason to look at fallow output beyond "fix these unused exports."

- Fan-in/fan-out from graph edges
- Dead code ratio per file
- Complexity density (total cyclomatic / LOC)
- Maintainability index (weighted composite, 0-100)

Implementation: single new function that takes `AnalysisResults` + `ModuleGraph` → `Vec<FileHealthScore>`. No new AST traversal.

The maintainability formula starts simple and documented: `100 - (complexity_density × 30) - (dead_code_ratio × 20) - (fan_out × 0.5)` capped at 0-100. Published in docs so users can understand and critique it. Iterate based on feedback. Do not ship a black-box score.

**1b. Hotspot analysis** (effort: M, weeks)

The killer feature. Combine health scores with git history.

- Shell out to `git log --numstat`, parse in Rust
- Churn score = commits × lines_changed, weighted by recency
- Hotspot score = churn × complexity_density × log(fan_in + 1)
- Churn trend: split period in halves, compare

```bash
fallow health --hotspots --since 3months
```

**Graceful degradation:** If not in a git repo (tarball, CI shallow clone), skip hotspot analysis and print a clear message. `--hotspots` without git = error with helpful message. All other health features work without git.

**1c. Refactoring targets** (effort: S, days — builds on 1b) — **SHIPPED (v1.9.0)**

Ranked list with one-line actionable recommendations per file. Seven rules evaluated in priority order. Priority formula avoids MI to prevent double-counting. Shipped with:
- Effort estimation (low/medium/high) based on file size, function count, fan-in
- Evidence linking (unused export names, complex function names+lines, cycle paths) for AI agent consumption
- Baseline support for targets (`--save-baseline` / `--baseline` with `path:category` keys)
- All 5 output formats, MCP integration, explain metadata

**Expert panel follow-ups for Phase 1d (targets iteration):**

| Feature | Value | Effort | Source |
|---------|-------|--------|--------|
| Effort-weighted priority (`priority / effort` as default sort) | High — surfaces quick wins first | S | EM panel |
| `--group-by directory` | High for orgs >50 — enables per-team sprint planning | M | EM panel |
| Confidence scores per recommendation (High/Medium/Low based on data source reliability) | Medium — calibrates expectations | S | Static analysis expert |
| Percentile-based thresholds (fan_in top 5% vs ≥20) | Medium — adapts to codebase size | S | Senior dev + analysis expert |
| Cluster IDs for related targets (circular dep chains, co-located files) | Medium — prevents local optimizations that miss global picture | M | AI agent expert |
| Evidence for split_high_impact (top importers) and extract_dependencies (import list) | Low — fan_in/out counts are sufficient for now | S | Dropped by pragmatic assessment |

**Why Phase 1 is first:** Hotspots are the feature that makes engineering managers care. Engineering managers are the ones who approve tool purchases. Without this, fallow is a developer utility that never reaches the buying decision-maker.

### Phase 2: Trend tracking — the conversion trigger

**Goal:** Create the data that makes a cloud product valuable. Without trends, the cloud is just a dashboard wrapper. With trends, it becomes the system of record for codebase health.

**2a. Vital signs snapshot** (effort: S, days)

Compute a fixed set of codebase metrics per run and serialize to JSON.

```bash
fallow health --save-snapshot       # stores to .fallow/snapshots/
fallow health --vital-signs         # prints current values
```

Vital signs: dead_file_pct, dead_export_pct, avg_cyclomatic, p90_cyclomatic, duplication_pct, hotspot_count, maintainability_avg, dep_count.

**Storage design:** `.fallow/snapshots/YYYY-MM-DDTHH-MM-SS.json`. Committed to git or not — user's choice. Lightweight (~2KB per snapshot). CI runners that persist `.fallow/` via caching get trends for free.

**2b. Trend reporting** (effort: S-M, days-week)

```bash
fallow health --trend --since 30days
```

Load snapshots, compute deltas, format as table with directional indicators.

**2c. Regression detection** (effort: S, days)

```bash
fallow health --fail-on-regression --tolerance 2%
```

Compare current vital signs against last snapshot. Exit code 1 if degradation exceeds tolerance. This is the CI gate that creates the "we need a proper dashboard" conversation.

**Design note:** Use a diff-only approach — only check new/changed code against the baseline, not the entire project. This is the least-gamed quality gate pattern (Codecov's "no coverage regression" works this way). A whole-project threshold invites gaming; a "don't make it worse" gate aligns with how teams actually work.

**Why Phase 2 is second:** Trends are meaningless without health scores and hotspots to trend. But once they exist, they're the data layer that the cloud product renders. Without a cloud product, the CLI trend command is still useful — but it's deliberately limited (local snapshots only) to create conversion pressure.

### Phase 3: Dependency risk — the enterprise wedge

**Goal:** Bridge from "code hygiene" to "security" — the budget category where procurement actually happens.

**3a. Unused deps × CVEs** (effort: S-M, days-week)

Shell out to `npm audit --json`, cross-reference with fallow's unused dependency list.

The pitch: "Remove these 5 unused dependencies and eliminate 3 known vulnerabilities with zero code changes."

This is the sentence that gets security teams to approve adoption. Compliance teams understand CVEs; they don't understand "unused exports."

**~~3b. Usage-aware SBOM~~** — DROPPED (March 2026 review)

After critical analysis, this feature doesn't hold up:
- **Security tools already do reachability analysis better.** Snyk, Endor Labs, and Semgrep match vulnerable *functions* to call paths — deeper than import-level detection. Fallow can only say "package X is imported," not "vulnerable function Y is called."
- **Compliance wants the full list, not the used subset.** Auditors want maximum coverage. An installed dep may be subject to its license terms regardless of import status.
- **Bundled apps already exclude unused deps.** For webpack/vite apps, the bundler is the source of truth for what's in production.
- **Standard SBOM tools are mature.** Syft, cdxgen, and built-in GitHub/GitLab SBOM export cover this space adequately.

Fallow's real SBOM contribution is indirect: unused dep cleanup via `fallow` reduces the SBOM surface area at the source. That's the existing `check` command, not a new feature.

**3c. Transitive risk mapping** (effort: M, weeks)

Not just "express has a CVE" but "these 14 files import express." Graph traversal from vulnerable deps to consuming files.

**Why Phase 3 is third:** Security features need the health intelligence foundation to be compelling — "this vulnerable dependency is unused AND in a hotspot file" is more actionable than just "this dep has a CVE." Also: security is the enterprise procurement trigger, and enterprise sales require the product to be mature enough to survive evaluation.

### Phase 4: Visualization — the demo and sales tool

**Goal:** Make codebase health visible and shareable. Viz is a marketing/sales tool, not a revenue feature.

Previously Phase 1, moved here. Rationale: viz is high-effort (custom Canvas rendering, Preact UI, force-directed layout — weeks of frontend work), has no direct revenue potential, and is independent of the health intelligence track. It's valuable for demos, blog posts, and the "wow factor" that drives word-of-mouth — but it doesn't create conversion pressure for a paid tier.

**4a. Treemap with dead code overlay** (effort: M) — Self-contained HTML report. Canvas-based, ~25-35KB embedded in binary. Files sized by bytes, colored by health status. No external dependencies.

**4b. Dependency graph + focus mode** (effort: M) — Canvas 2D, force-directed layout, directory clustering. Builds on 4a infrastructure.

**4c. DOT + Mermaid output** (effort: S) — Simple string formatting. Useful for PR comments and CI artifacts. Ship alongside or before 4a — much less effort.

**4d. VS Code webview integration** (effort: S-M) — Embed 4a/4b in a VS Code webview. Click node → open file.

Full architecture spec: [viz-spec.md](viz-spec.md) — data model, Rust/frontend architecture, interaction design, bundle size budget, competitive position table.

### Phase 5: Architecture & quality

**Goal:** Additional CLI features that increase daily usage and CI integration.

**5a. Architecture boundaries** (effort: M, weeks)

Directory-based import rules validated against the module graph. dependency-cruiser's mental model at Rust speed.

```toml
[[boundary]]
from = "src/ui/**"
deny = ["src/db/**"]
message = "UI layer cannot import database code"
```

**5b. Static test coverage gaps** (effort: M, weeks)

Which exports have zero test file dependency? Uses existing plugin knowledge to classify test files + graph reachability.

Feeds into hotspot analysis: complex + high-churn + untested = highest risk.

**5c. Treeshakeability lint** (effort: M, weeks)

Detect module-scope side effects, CJS in ESM context, barrel file impact quantification, missing `sideEffects` field.

### Phase 6: Library & monorepo tools

**Goal:** Features for library authors and large monorepo teams.

**6a. API surface diff** (effort: L, months)

`fallow diff main..HEAD` — compare exported API signatures between git refs, classify changes by semver impact. Syntactic only (catches ~70% of breaking changes).

**6b. Monorepo health** (effort: M, weeks)

Version drift detection, circular workspace deps, unused cross-workspace dependencies.

---

## Cloud product MVP

When to build: **after Phase 2 (trends) is shipped and at least 50 teams are using `--save-snapshot` in CI.** Not before. Building cloud infrastructure without validated demand is the most common developer-tool startup mistake.

### MVP scope

1. **GitHub Action** (not App) — thin wrapper around CLI, runs in user's CI. Pass/fail check status + file-level annotations via `::error file=...` workflow commands. Listed on Actions Marketplace for distribution. No server infrastructure needed, zero marginal cost per user.
2. **Dashboard** — vital signs trends per repo, health grade. Keep it minimal: one page with a trend line and current scores. 70% of dashboard development is wasted (shelfware problem), and enterprises use only ~47% of licensed SaaS seats. Build the smallest useful thing.
3. **Alerts** — Slack webhook when a vital sign degrades. Only if customers ask.

### MVP tech stack

- Snapshots uploaded from CI via `fallow health --upload` (authenticated, API key)
- Backend: simple API storing JSON snapshots in S3/R2 + SQLite/Postgres for metadata
- Frontend: lightweight dashboard (could be the same Preact used for viz)
- GitHub Action: workflow commands for annotations, no webhook infrastructure needed

### What the MVP is NOT

- Not a SonarQube clone with code viewers and inline highlighting
- Not a platform with user management for 50 features
- Not multi-language — JS/TS only, forever (that's the focus)

---

## Scoring matrix

Scored on **Revenue potential** (does this drive paid conversions?), **User acquisition** (does this grow the free user base?), **Uniqueness** (differentiation from competitors), and **Feasibility** (leverage of existing infrastructure). Each 1-10.


| #   | Feature                            | Revenue | Acquisition | Uniqueness | Feasibility | Phase | Effort | Status      |
| --- | ---------------------------------- | ------- | ----------- | ---------- | ----------- | ----- | ------ | ----------- |
| 1   | Circular deps                      | —       | —           | 3          | 10          | —     | S      | **SHIPPED** |
| 2   | Complexity metrics                 | —       | —           | 5          | 9           | —     | M      | **SHIPPED** |
| 3   | File-level health scores           | —       | —           | 8          | 9           | 1a    | S      | **SHIPPED** |
| 4   | **Hotspot analysis**               | **—**   | **—**       | **10**     | **8**       | 1b    | M      | **SHIPPED** |
| 5   | Refactoring targets                | 7       | 6           | 9          | 9           | 1c    | S      | **SHIPPED** |
| 6   | Vital signs snapshots              | 8       | 5           | 7          | 9           | 2a    | S      | Open        |
| 7   | Trend reporting                    | 9       | 6           | 8          | 8           | 2b    | S-M    | Open        |
| 8   | **Regression detection (CI gate)** | **8**   | **8**       | **7**      | **9**       | 2c    | S      | Open        |
| 9   | **Unused deps × CVEs**             | **9**   | **7**       | **9**      | **8**       | 3a    | S-M    | Open        |
| 10  | ~~Usage-aware SBOM~~               | —       | —           | —          | —           | ~~3b~~| —      | **DROPPED** |
| 11  | Transitive risk mapping            | 6       | 3           | 8          | 7           | 3c    | M      | Open        |
| 12  | Viz: treemap + dead code           | 3       | 9           | 10         | 6           | 4a    | M      | Open        |
| 13  | Viz: dependency graph              | 2       | 6           | 8          | 6           | 4b    | M      | Open        |
| 14  | Viz: DOT/Mermaid                   | 1       | 4           | 5          | 10          | 4c    | S      | Open        |
| 15  | Architecture boundaries            | 5       | 8           | 8          | 9           | 5a    | M      | Open        |
| 16  | Static test coverage gaps          | 6       | 7           | 10         | 8           | 5b    | M      | Open        |
| 17  | Treeshakeability lint              | 4       | 6           | 10         | 7           | 5c    | M      | Open        |
| 18  | API surface diff                   | 5       | 5           | 9          | 6           | 6a    | L      | Open        |
| 19  | Monorepo health                    | 4       | 5           | 8          | 9           | 6b    | M      | Open        |
| 20  | **Human output polish**            | **3**   | **7**       | **6**      | **9**       | —     | M      | **SHIPPED** |
| 21  | Metric explainability (docs + JSON)| 4       | 6           | 7          | 8           | —     | S-M    | **SHIPPED** |


Effort: **S** = days, **M** = weeks, **L** = months.

**Reading the matrix:** Features that score high on Revenue AND Acquisition are the priority: hotspots (#4), regression detection (#8), CVE cross-ref (#9), trend reporting (#7). Features that score high on Acquisition but low on Revenue (viz, boundaries) are marketing tools — valuable for growth but won't pay the bills. ~~Features that score high on Revenue but low on Acquisition (SBOM, transitive risk) are enterprise upsells — build them when you have enterprise customers asking.~~ SBOM dropped — transitive risk mapping is the remaining enterprise upsell.

---

## Dependency graph

```
Phase 0 (marketing) ──────────────────── ongoing, parallel with everything

Phase 1 (health intel) ──→ Phase 2 (trends) ──→ Cloud MVP
                       ╲
Phase 3 (dep risk) ─────── can parallel Phase 1/2

Phase 4 (viz) ─────────── independent, parallel with anything

Phase 5 (quality) ─────── Phase 1 needed for test gap integration

Phase 6 (library/mono) ── independent, lowest priority
```

**What to cut if capacity is tight:** Phase 6 entirely (library/monorepo tools are nice-to-have). Phase 4 viz sub-phases 4b/4c/4d (ship treemap only, skip graph/cycles/VS Code). Phase 5c treeshakeability (novel but niche).

**What to never cut:** Hotspots (Phase 1b), trend tracking (Phase 2), CVE cross-ref (Phase 3a). These three features are the business thesis.

---

## Risks


| Risk                                                    | Likelihood                   | Impact                                           | Mitigation                                                                                                                                                             |
| ------------------------------------------------------- | ---------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Biome ships unused export detection                     | Medium (they have the infra) | High — commoditizes fallow's core feature        | Move up the value chain fast (health, hotspots, trends). Biome won't build codebase intelligence.                                                                      |
| VoidZero/Oxlint expands to project-level analysis       | Medium (Rolldown needs module graph) | Critical — fallow depends on Oxc and would compete with its maintainer | Differentiate on features Oxlint's per-file model can't do (whole-project graph analysis, duplication, hotspots). Architectural gap buys time but isn't permanent. |
| "AI will just fix dead code" narrative                  | Medium                       | Medium — reduces urgency for automated detection | Fallow is complementary: AI generates, fallow audits. Also: AI makes the problem worse (more generated dead code), not better.                                         |
| Cloud conversion rate below 2%                          | High                         | High — business model fails                      | Keep CLI standalone-valuable. Explore alternative revenue (consulting, enterprise support, sponsored development). Don't over-invest in cloud infra before validating. |
| Solo developer burnout                                  | Medium-High                  | Critical — project dies                          | Scope ruthlessly. Ship Phase 1-2, validate demand, then decide if cloud is worth building. Open-source contributions for Phase 5-6 features.                           |
| SonarQube / Snyk adds JS/TS dead code                   | Low                          | Medium                                           | Their architecture makes framework-aware analysis hard. By the time they ship it, fallow should be differentiated on health intelligence, not just dead code.          |
| CodeScene adds JS/TS specialization                     | Low                          | Medium                                           | Their business model is enterprise consulting. Fallow's advantage is open-source developer adoption funnel. Different markets.                                         |
| `.fallow/snapshots/` creates state management headaches | Medium                       | Low                                              | Document clearly: commit it or cache it in CI, don't rely on ephemeral storage. Cloud product eliminates this friction entirely (that's a selling point).              |


---

## What this enables

After Phase 1-3 + Cloud MVP, fallow answers:


| Question                         | Command / Feature                     | Who asks     | Tier                  |
| -------------------------------- | ------------------------------------- | ------------ | --------------------- |
| What code is dead?               | `fallow`                        | Developer    | Free                  |
| What code is duplicated?         | `fallow dupes`                        | Developer    | Free                  |
| What's too complex?              | `fallow health`                       | Developer    | Free                  |
| Which files are riskiest?        | `fallow health --hotspots`            | Tech lead    | Free                  |
| Where should we refactor?        | `fallow health --targets`             | Eng manager  | Free                  |
| Is our codebase improving?       | `fallow health --trend`               | Eng manager  | Free (limited) / Team |
| Did this PR make things worse?   | PR annotation                         | Tech lead    | Team                  |
| How do our teams compare?        | Dashboard                             | Eng director | Team / Enterprise     |
| Which unused deps are dangerous? | `fallow health --dependency-risk`     | Security     | Free                  |
| ~~Show me an SBOM~~              | ~~`fallow health --sbom`~~            | ~~Compliance~~ | ~~DROPPED~~         |


The free tier is generous by design — that's the acquisition engine. The cloud tier adds the time dimension and collaboration layer. Enterprise adds compliance and organizational views.

**The honest ceiling:** This is likely a $3-10M ARR business at maturity. Enough for a small team, not VC-scale. The most likely "big outcome" is acquisition by a platform (Vercel, GitHub, JetBrains) that wants fallow's analysis engine as a feature. Build the product to be independently sustainable, but keep the architecture modular enough to integrate.