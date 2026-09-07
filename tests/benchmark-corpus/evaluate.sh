#!/usr/bin/env bash
set -euo pipefail

# Duplication Accuracy Baseline -- evaluate fallow dupes against ground truth
#
# Runs fallow dupes in all 4 modes + defaults, captures JSON output, and scores
# it with evaluate-results.py against the committed floor. Exits non-zero when a
# mode drops below the floor.
#
# Usage:
#   ./evaluate.sh [--fallow-bin PATH] [--results-dir DIR] [--update-floor]
#
# Results land in a scratch directory unless --results-dir is given, so the
# committed results/ tree is never written by a routine run.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CORPUS_DIR="$SCRIPT_DIR"
RESULTS_DIR=""
UPDATE_FLOOR=""

if [[ -n "${FALLOW_BIN:-}" ]]; then
    read -r -a FALLOW_CMD <<< "${FALLOW_BIN}"
else
    FALLOW_CMD=(cargo run --quiet --bin fallow --)
fi

while [[ $# -gt 0 ]]; do
    case "$1" in
        --fallow-bin)     FALLOW_CMD=("$2"); shift 2 ;;
        --fallow-bin=*)   FALLOW_CMD=("${1#*=}"); shift ;;
        --results-dir)    RESULTS_DIR="$2"; shift 2 ;;
        --results-dir=*)  RESULTS_DIR="${1#*=}"; shift ;;
        --update-floor)   UPDATE_FLOOR="--update-floor"; shift ;;
        *) echo "Unknown argument: $1" >&2; exit 2 ;;
    esac
done

if [[ -z "${RESULTS_DIR}" ]]; then
    RESULTS_DIR="$(mktemp -d)"
    trap 'rm -rf "${RESULTS_DIR}"' EXIT
fi
mkdir -p "${RESULTS_DIR}"

echo "=== Fallow Duplication Accuracy Baseline ==="
echo "Corpus:  ${CORPUS_DIR}"
echo "Results: ${RESULTS_DIR}"
echo ""

run_mode() {
    local mode="$1"
    local min_tokens="${2:-30}"
    local min_lines="${3:-3}"
    local label="${4:-$mode}"
    local output_file="${RESULTS_DIR}/dupes-${label}.json"

    echo "--- Running mode: ${label} ---"

    # Exit 1 means findings were reported, which is the normal case here. Only
    # exit 2 and above are execution errors worth aborting on.
    local exit_code=0
    "${FALLOW_CMD[@]}" dupes \
        --mode "${mode}" \
        --min-tokens "${min_tokens}" \
        --min-lines "${min_lines}" \
        --format json \
        --quiet \
        --no-cache \
        --root "${CORPUS_DIR}" \
        > "${output_file}" 2>/dev/null || exit_code=$?

    if [[ ${exit_code} -ge 2 ]]; then
        echo "Error: fallow dupes --mode ${mode} exited ${exit_code}" >&2
        exit "${exit_code}"
    fi

    if ! python3 -c "import json,sys; json.load(open(sys.argv[1]))" "${output_file}"; then
        echo "Error: fallow dupes --mode ${mode} did not produce valid JSON" >&2
        exit 2
    fi

    local groups
    groups=$(python3 -c "import json,sys; print(len(json.load(open(sys.argv[1]))['clone_groups']))" "${output_file}")
    echo "  Clone groups found: ${groups}"
    echo ""
}

run_mode strict 30 3
run_mode mild 30 3
run_mode weak 30 3
run_mode semantic 30 3

# Also run with default settings for comparison
run_mode mild 50 5 defaults

python3 "${SCRIPT_DIR}/evaluate-results.py" --results-dir "${RESULTS_DIR}" ${UPDATE_FLOOR:+"${UPDATE_FLOOR}"}
