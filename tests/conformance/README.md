# Conformance Test Suite

Compares fallow's dead code detection results against [knip](https://github.com/webpro-nl/knip) on the same projects, producing a structured report of agreements and disagreements.

This suite is **informational** -- it does not fail on disagreements. Differences between the tools are expected due to different analysis strategies and heuristics.

## Prerequisites

- **fallow** binary (built from this repo or in PATH)
- **Node.js** (v18+) with `npx` available
- **Python 3** (for the comparison logic)
- knip is installed automatically via `npx` on first run

## Running

```bash
# From the repo root, using the built binary
cargo build --release
./tests/conformance/run.sh

# With a specific fallow binary
./tests/conformance/run.sh --fallow-bin ./target/debug/fallow

# Against a custom project
./tests/conformance/run.sh /path/to/your/project

# Both options
./tests/conformance/run.sh /path/to/project --fallow-bin ./target/release/fallow
```

The script outputs:
- **stderr**: human-readable summary with issue counts and disagreement details
- **stdout**: structured JSON report (pipe to `jq` or save to file)

```bash
# Save JSON report
./tests/conformance/run.sh > report.json

# Pretty-print with jq
./tests/conformance/run.sh 2>/dev/null | jq .
```

## Interpreting Results

The report breaks down findings into three categories:

- **Agreed**: Issues found by both fallow and knip. High confidence these are real issues.
- **Fallow-only**: Issues found by fallow but not knip. Could be:
  - True positives that knip misses (fallow wins)
  - False positives in fallow (needs investigation)
- **Knip-only**: Issues found by knip but not fallow. Could be:
  - True positives that fallow misses (needs implementation)
  - False positives in knip (fallow correctly ignores them)

The agreement percentage is calculated as `agreed / total_unique_issues * 100`.

## Issue Type Mapping

| Fallow                    | Knip            |
|---------------------------|-----------------|
| `unused_files`            | `files`         |
| `unused_exports`          | `exports`       |
| `unused_types`            | `types`         |
| `unused_dependencies`     | `dependencies`  |
| `unused_dev_dependencies` | `devDependencies`|
| `unresolved_imports`      | `unresolved`    |
| `unlisted_dependencies`   | `unlisted`      |
| `duplicate_exports`       | `duplicates`    |
| `unused_enum_members`     | `enumMembers`   |
| `unused_class_members`    | `classMembers`  |

## Adding Test Projects

1. Create a new directory under `fixtures/`:
   ```
   fixtures/my-project/
     package.json    # must exist, list deps to test
     tsconfig.json   # optional but recommended
     src/            # source files
   ```

2. Run the suite against it:
   ```bash
   ./tests/conformance/run.sh fixtures/my-project
   ```

3. The project should have realistic dead code patterns. Good candidates:
   - Small open-source projects with known dead code
   - Synthetic projects targeting specific edge cases
   - Projects that exercise specific framework plugins

## JSON Report Schema

```json
{
  "summary": {
    "fallow_total": 5,
    "knip_total": 4,
    "agreed": 3,
    "fallow_only": 2,
    "knip_only": 1,
    "agreement_pct": 60.0
  },
  "by_type": {
    "unused_exports": {
      "fallow_count": 2,
      "knip_count": 2,
      "agreed": 2,
      "fallow_only": 0,
      "knip_only": 0,
      "agreement_pct": 100.0
    }
  },
  "details": {
    "agreed": [{"file": "src/utils.ts", "name": "unusedFn", "type": "unused_exports"}],
    "fallow_only": [],
    "knip_only": []
  }
}
```

## CI

The conformance suite runs weekly via `.github/workflows/conformance.yml` and can be triggered manually. Results are posted to the GitHub Actions step summary. It never fails the CI pipeline -- it is purely informational.
