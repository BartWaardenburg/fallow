# Benchmark Methodology

This document describes how fallow's performance benchmarks are structured, how to reproduce them, and how to interpret results.

## Overview

Fallow uses two benchmark layers:

1. **Criterion (Rust)** — Microbenchmarks for regression detection in CI. Measures individual pipeline stages and full end-to-end analysis at various project sizes (10, 100, 1000, 5000 files).
2. **Comparative (Node.js)** — Wall-clock comparisons against knip (dead code) and jscpd (duplication) on synthetic and real-world projects.

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
# All benchmarks (including large-scale)
cargo bench --bench analysis

# Only the standard group (fast)
cargo bench --bench analysis -- benches/

# Only large-scale benchmarks (1000+ files, slower)
cargo bench --bench analysis -- large_scale_benches/
```

Large-scale benchmarks use `sample_size(10)` and `measurement_time(60s)` to accommodate longer iteration times.

### Comparative Benchmarks

```bash
cd benchmarks

# Generate synthetic fixtures (required once)
npm run generate           # check fixtures (tiny → xlarge)
npm run generate:dupes     # dupes fixtures (tiny → xlarge)

# Download real-world projects (required once)
npm run download-fixtures  # preact, fastify, zod

# Run benchmarks (includes knip v6 if installed in benchmarks/knip6/)
npm run bench              # fallow vs knip v5 + v6 (all fixtures)
npm run bench:synthetic    # synthetic only
npm run bench:real-world   # real-world only
npm run bench:dupes        # fallow dupes vs jscpd (all fixtures)

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

## CI Integration

The `.github/workflows/bench.yml` workflow runs Criterion benchmarks on every PR and push to main:

- Results stored on `gh-pages` branch
- 10% regression threshold triggers alerts
- PR comments show benchmark comparisons
- Only measures the Criterion (Rust) benchmarks, not comparative benchmarks
