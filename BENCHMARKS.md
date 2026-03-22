# Benchmark Methodology

This document describes how fallow's performance benchmarks are structured, how to reproduce them, and how to interpret results.

## Overview

Fallow uses two benchmark layers:

1. **Criterion (Rust)** — Microbenchmarks for regression detection in CI. Measures individual pipeline stages and full end-to-end analysis at various project sizes (10, 100, 1000, 5000 files).
2. **Comparative (Node.js)** — Wall-clock comparisons against knip (dead code), jscpd (duplication), and madge/dpdm (circular dependencies) on synthetic and real-world projects.

## Project Sizes

| Size    | Files | Purpose                          |
|---------|------:|----------------------------------|
| tiny    |    10 | Baseline / startup overhead      |
| small   |    50 | Small library                    |
| medium  |   200 | Typical module                   |
| large   | 1,000 | Monorepo package / mid-size app  |
| xlarge  | 5,000 | Large monorepo / enterprise app  |

Synthetic projects use deterministic seeding (Mulberry32, seed `42 + fileCount`) for reproducibility across runs and machines. Each project includes a realistic mix of TypeScript constructs: interfaces, types, functions, constants, and import graphs with ~80% used / ~20% dead code.

## What Is Measured

### Check (dead code analysis)

Full pipeline: file discovery → parallel Oxc parsing → import resolution → module graph construction → re-export chain propagation → dead code detection.

### Dupes (code duplication)

Full pipeline: file discovery → tokenization → normalization → suffix array construction → LCP computation → clone extraction → family grouping.

### Circular (circular dependency detection)

Full pipeline: file discovery → parallel Oxc parsing → import resolution → module graph construction → Tarjan's SCC algorithm.

### Cache Modes

- **Cold cache** (`--no-cache`): No cache read or write. Measures raw analysis speed.
- **Warm cache**: Cache populated by a prior run. Measures incremental analysis speed where file content hashes match cached results, skipping re-parsing.

## Metrics Collected

| Metric | Source | Description |
|--------|--------|-------------|
| Wall time | `performance.now()` / Criterion | End-to-end elapsed time |
| Peak RSS | `/usr/bin/time -l` (macOS) or `-v` (Linux) | Maximum resident set size |
| Issue count | JSON output parsing | Correctness cross-check |
| Min/Max/Mean/Median | Statistical aggregation | Distribution characterization |

## Reproducing Benchmarks

### Prerequisites

```bash
# Rust toolchain (stable)
rustup update stable

# Node.js (for comparative benchmarks)
cd benchmarks && npm install

# Optional: install knip v6 for three-way comparison
cd benchmarks/knip6 && npm install
```

### Criterion Benchmarks

```bash
# All benchmarks (both standard and large-scale)
cargo bench

# Only standard benchmarks (fast)
cargo bench --bench analysis

# Only large-scale benchmarks (1000+ files, slower)
cargo bench --bench large_analysis
```

Large-scale benchmarks use `sample_size(10)` and `measurement_time(60s)` to accommodate longer iteration times.

### Comparative Benchmarks

```bash
cd benchmarks

# Generate synthetic fixtures (required once)
npm run generate           # check fixtures (tiny → xlarge)
npm run generate:dupes     # dupes fixtures (tiny → xlarge)
npm run generate:circular  # circular dep fixtures (tiny → xlarge)

# Download real-world projects (required once)
npm run download-fixtures  # preact, fastify, zod, vue-core, svelte, query, vite, next.js

# Run benchmarks (includes knip v6 if installed in benchmarks/knip6/)
npm run bench              # fallow vs knip v5 + v6 (all fixtures)
npm run bench:synthetic    # synthetic only
npm run bench:real-world   # real-world only
npm run bench:dupes        # fallow dupes vs jscpd (all fixtures)
npm run bench:circular     # fallow vs madge + dpdm (all fixtures)

# Customize runs
npm run bench -- --runs=10 --warmup=3
```

### Output

Benchmark scripts print:
1. **Environment info**: CPU model, core count, RAM, OS, Node/Rust versions
2. **Per-project tables**: cold cache, warm cache, and competitor timings with memory usage
3. **Summary table**: all projects with speedup ratios and peak RSS

## Interpreting Results

- **Median** is the primary comparison metric (robust to outliers).
- **Min** indicates best-case (OS caches warm, no contention).
- **Max** indicates worst-case (GC pauses for JS tools, cold OS caches).
- **Cache speedup** shows the ratio of cold-to-warm median times. Values > 1.5x indicate significant parsing savings from caching.
- **Peak RSS** measures maximum memory usage. Lower is better for CI environments with constrained memory.
- **Speedup** is `competitor_median / fallow_median`. Values > 1.0x mean fallow is faster.

## Hardware Considerations

Benchmark results vary with hardware. Key factors:

- **CPU core count**: fallow uses rayon for parallel parsing. More cores = faster cold cache analysis. Single-threaded tools (knip) don't benefit.
- **Disk speed**: SSD vs HDD significantly affects file discovery and first-read performance.
- **Available RAM**: Large projects (5000+ files) with duplication detection can use several hundred MB.

When publishing results, always include the environment info printed by the benchmark scripts.

## Reference Results (2026-03-22)

Environment: Apple M5 (10 cores), 32 GB RAM, macOS 25.3.0, Node v22.21.1, rustc 1.93.0. fallow 1.2.0, knip 5.87.0, knip 6.0.0, jscpd 4.0.8, madge 8.0.0. Median of 5 runs, 2 warmup.

### Dead code: fallow check vs knip

| Project | Files | fallow | knip v5 | knip v6 | vs v5 | vs v6 | fallow RSS | knip v5 RSS | knip v6 RSS |
|:--------|------:|-------:|--------:|--------:|------:|------:|-----------:|------------:|------------:|
| zod | 174 | 19ms | 639ms | 334ms | 34.4x | 18.0x | 21 MB | 250 MB | 161 MB |
| preact | 244 | 20ms | 819ms | —* | 40.6x | — | 22 MB | 235 MB | — |
| fastify | 286 | 24ms | 1.13s | 289ms | 46.3x | 11.9x | 27 MB | 289 MB | 107 MB |
| vue/core | 522 | 63ms | 702ms | 299ms | 11.2x | 4.8x | 34 MB | 271 MB | 119 MB |
| TanStack/query | 901 | 148ms | 2.75s | 1.41s | 18.6x | 9.6x | 59 MB | 673 MB | 354 MB |
| vite | 1,420 | 596ms | —† | —† | — | — | 52 MB | — | — |
| svelte | 3,337 | 325ms | 1.93s | 860ms | 5.9x | 2.6x | 67 MB | 460 MB | 243 MB |
| next.js | 20,416 | 1.48s | —† | —† | — | — | 194 MB | — | — |

\* knip v6 excluded for preact due to a v6 regression.
† knip errors out on vite and next.js (exits without producing valid results).

### Duplication: fallow dupes vs jscpd

| Project | Files | fallow | jscpd | Speedup | fallow RSS | jscpd RSS |
|:--------|------:|-------:|------:|--------:|-----------:|----------:|
| zod | 174 | 46ms | 909ms | 19.7x | 54 MB | 188 MB |
| preact | 244 | 44ms | 1.33s | 30.4x | 57 MB | 262 MB |
| fastify | 286 | 84ms | 2.83s | 33.6x | 97 MB | 315 MB |
| vue/core | 522 | 120ms | 3.13s | 26.1x | 155 MB | 430 MB |
| TanStack/query | 901 | 120ms | 1.19s | 9.9x | 132 MB | 226 MB |
| vite | 1,420 | 113ms | 1.82s | 16.0x | 92 MB | 292 MB |
| svelte | 3,337 | 400ms | 3.63s | 9.1x | 155 MB | 470 MB |
| next.js | 20,416 | 3.16s | 24.64s | 7.8x | 834 MB | 1.52 GB |

### Circular dependencies: fallow check --circular-deps vs madge/dpdm

| Project | Files | fallow | madge | dpdm | vs madge | vs dpdm | fallow RSS |
|:--------|------:|-------:|------:|-----:|---------:|--------:|-----------:|
| zod | 174 | 17ms | 540ms | 190ms | 31.5x | 11.1x | 21 MB |
| preact | 244 | 19ms | 298ms | 132ms | 15.5x | 6.9x | 22 MB |
| fastify | 286 | 20ms | 165ms | 132ms | 8.2x | 6.6x | 27 MB |
| vue/core | 522 | 59ms | 175ms | 143ms | 3.0x | 2.4x | 36 MB |
| TanStack/query | 901 | 134ms | 168ms | 137ms | 1.3x | 1.0x | 60 MB |
| svelte | 3,337 | 310ms | 165ms | 132ms | 0.5x | 0.4x | 67 MB |
| vite | 1,420 | 564ms | 164ms | 133ms | 0.3x | 0.2x | 52 MB |
| next.js | 20,416 | 1.21s | 472ms | 427ms | 0.4x | 0.4x | 193 MB |

Note: fallow runs a full analysis pipeline (discovery, parsing, graph building, SCC detection) while madge/dpdm only build a dependency graph from imports. On large monorepos the pipeline overhead dominates. On small-to-medium projects fallow's native speed wins. Madge and dpdm report `?` for cycle counts on many projects, suggesting incomplete detection.

### Summary ranges

| Comparison | Speed | Memory |
|:-----------|:------|:-------|
| fallow vs knip v5 | 6-46x faster | 4-11x less |
| fallow vs knip v6 | 3-18x faster | 3-6x less |
| fallow vs jscpd | 8-34x faster | 2-4x less |
| fallow vs madge | 1-32x faster (small-medium), 0.3-0.5x on large monorepos | 4-13x less |

## CI Integration

The `.github/workflows/bench.yml` workflow runs Criterion benchmarks on PRs and pushes to main (when Rust source files change):

- Results stored on `gh-pages` branch
- 10% regression threshold triggers alerts
- PR comments show benchmark comparisons
- Only measures the Criterion (Rust) benchmarks, not comparative benchmarks
