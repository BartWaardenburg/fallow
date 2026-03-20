# Fallow — Positioning & Copy Guide

This document is the single source of truth for fallow's positioning, taglines, and copy across all surfaces. All repos (fallow, fallow-skills, fallow-docs) must stay consistent with this guide.

## North Star

**Who:** JavaScript and TypeScript teams of any size, from solo developers to large monorepos.

**What:** A comprehensive codebase analyzer that identifies unused code, circular dependencies, code duplication, and complexity hotspots through fast, deterministic static analysis.

**Why now:** The JavaScript ecosystem has converged on Rust-native tooling for linting (oxlint), formatting (oxfmt/Biome), and testing (Vitest) — but codebase analysis has remained either slow (knip) or fragmented across single-purpose tools (madge, jscpd, complexity reporters). Fallow unifies these in a single tool that matches the performance expectations set by the new Rust-native stack. Codebases are also growing faster than ever — through AI-assisted development, micro-package architectures, and rapid team scaling — making automated codebase analysis a necessity rather than a nice-to-have.

**Why fallow:** The only tool that combines dead code detection, circular dependency analysis, clone detection, and complexity analysis in a single Rust-native binary with sub-second performance. Zero configuration, 84 framework plugins, and fast enough to shift codebase analysis from a periodic audit to a continuous check.

## Tagline

**The codebase analyzer for JavaScript**

Usage: README hero, npm package name context, GitHub repo header. "JavaScript" is understood to include TypeScript in 2026 tooling discourse.

## Subtitle

**Unused code, circular dependencies, code duplication, and complexity hotspots — found in seconds, not minutes.**

Usage: directly below the tagline in README heroes. Lists what fallow finds with a speed claim.

## Stack Positioning

**Linters enforce style. Formatters enforce consistency. Fallow enforces relevance.**

Usage: docs landing page, blog posts, conference talks, README explainer sections. Positions fallow as the third pillar alongside oxlint/Biome and oxfmt/Prettier.

## One-Liners (per surface)

| Surface | Copy |
|---------|------|
| npm description | Fast codebase analysis for JS/TS — dead code, circular deps, duplication, and complexity |
| GitHub repo description | The codebase analyzer for JavaScript and TypeScript. Finds unused code, circular dependencies, code duplication, and complexity hotspots. Rust-native, sub-second, 84 framework plugins. |
| fallow-skills GitHub description | Agent skills for the JavaScript codebase analyzer. Teaches AI agents how to find unused code, circular deps, duplication, and complexity with fallow. |

## Elevator Pitch

> Fallow is a Rust-native codebase analyzer for JavaScript and TypeScript. It finds unused files, exports, types, and dependencies. It detects circular dependencies, duplicated code, and complexity hotspots. It ships with 84 framework plugins, requires zero configuration, and typically finishes in under a second — fast enough to run on every commit, not just in weekend CI jobs. Where linters enforce how you write code and formatters enforce how it looks, fallow tells you what shouldn't be there at all.

## AI Angle (narrative layer, NOT tagline)

The line "AI writes code. Nobody deletes it." is reserved for:

- Launch blog post headlines
- Conference talk titles
- Social media campaigns
- The "Why fallow?" section in docs (below the fold, after the tool has explained what it does)

**Do not use in:** README hero, npm description, GitHub description, taglines. The permanent positioning must be timeless and problem-focused. The AI narrative is a marketing layer that can be swapped out when discourse shifts.

## Words to Use

- "codebase analyzer" (our category)
- "enforces relevance" (what we do, in stack context)
- "unused code" (broader than "dead code", encompasses files, exports, types, deps)
- "structural issues" (umbrella for circular deps, complexity)
- "found in seconds" (speed without specific benchmarks in tagline)
- "Rust-native" (signals performance)

## Words to Avoid

- "static analysis" (too generic — every linter is static analysis)
- "dead code analyzer" (too narrow — fallow does much more now)
- "code quality" (too broad — overlaps with linters, formatters, everything)
- "codebase hygiene" (clinical, sounds like dental hygiene)
- "AI-powered" / "for the AI era" (fallow is deterministic, not AI-based)
- "lean" / "clean" as standalone adjectives (fail the substitution test)

## Benchmark Claims (sourced, usable)

| Claim | Source | Where to use |
|-------|--------|-------------|
| 3-36x faster than knip v5 | BENCHMARKS.md | README, docs, blog |
| 2-14x faster than knip v6 | BENCHMARKS.md | README, docs, blog |
| 20-33x faster than jscpd | BENCHMARKS.md | README, docs, blog |
| Sub-second on most projects | BENCHMARKS.md | Tagline subtitle, everywhere |
| 84 framework plugins | crates/core/src/plugins/ | Everywhere |
