#!/usr/bin/env bash
set -o pipefail

# Run fallow analysis with CLI argument construction (deduped)
# Required env: INPUT_COMMAND, INPUT_ROOT, INPUT_CONFIG, INPUT_FORMAT, INPUT_PRODUCTION,
#   INPUT_CHANGED_SINCE, INPUT_AUTO_CHANGED_SINCE, PR_BASE_SHA, EVENT_NAME,
#   INPUT_BASELINE, INPUT_SAVE_BASELINE, INPUT_ARGS, INPUT_DUPES_MODE,
#   INPUT_MIN_TOKENS, INPUT_MIN_LINES, INPUT_THRESHOLD, INPUT_SKIP_LOCAL,
#   INPUT_CROSS_LANGUAGE, INPUT_DRY_RUN, INPUT_WORKSPACE, INPUT_MAX_CYCLOMATIC,
#   INPUT_MAX_COGNITIVE, INPUT_TOP, INPUT_SORT, INPUT_FILE_SCORES, INPUT_HOTSPOTS,
#   INPUT_TARGETS, INPUT_COMPLEXITY, INPUT_SINCE, INPUT_MIN_COMMITS,
#   INPUT_SAVE_SNAPSHOT, INPUT_ISSUE_TYPES, INPUT_NO_CACHE, INPUT_THREADS,
#   INPUT_ONLY, INPUT_SKIP

# --- Shared argument building functions ---
# Uses global ARGS array (avoids bash nameref compatibility issues)

build_common_args() {
  local format=${1:-json}

  ARGS=(--root "$INPUT_ROOT" --quiet --format "$format")
  [ -n "$INPUT_COMMAND" ] && ARGS=("$INPUT_COMMAND" "${ARGS[@]}")

  [ -n "${INPUT_CONFIG:-}" ] && ARGS+=(--config "$INPUT_CONFIG")
  [ "${INPUT_PRODUCTION:-}" = "true" ] && ARGS+=(--production)
  [ -n "${INPUT_CHANGED_SINCE:-}" ] && ARGS+=(--changed-since "$INPUT_CHANGED_SINCE")
  [ -n "${INPUT_BASELINE:-}" ] && ARGS+=(--baseline "$INPUT_BASELINE")
  [ -n "${INPUT_SAVE_BASELINE:-}" ] && ARGS+=(--save-baseline "$INPUT_SAVE_BASELINE")
  [ -n "${INPUT_WORKSPACE:-}" ] && ARGS+=(--workspace "$INPUT_WORKSPACE")
  [ "${INPUT_NO_CACHE:-}" = "true" ] && ARGS+=(--no-cache)
  [ -n "${INPUT_THREADS:-}" ] && ARGS+=(--threads "$INPUT_THREADS")

  if [ -z "$INPUT_COMMAND" ]; then
    [ -n "${INPUT_ONLY:-}" ] && ARGS+=(--only "$INPUT_ONLY")
    [ -n "${INPUT_SKIP:-}" ] && ARGS+=(--skip "$INPUT_SKIP")
  fi
}

build_command_args() {
  local include_top=${1:-true}

  case "$INPUT_COMMAND" in
    dead-code|check)
      if [ "${INPUT_FORMAT:-}" = "sarif" ] && [ "${HAS_SARIF_FILE:-false}" = "true" ]; then
        ARGS+=(--sarif-file fallow-results.sarif)
      fi
      if [ -n "${INPUT_ISSUE_TYPES:-}" ]; then
        IFS=',' read -ra TYPES <<< "$INPUT_ISSUE_TYPES"
        for t in "${TYPES[@]}"; do
          t="$(echo "$t" | xargs)"
          ARGS+=("--${t}")
        done
      fi
      ;;
    dupes)
      ARGS+=(--mode "${INPUT_DUPES_MODE:-mild}")
      [ -n "${INPUT_MIN_TOKENS:-}" ] && ARGS+=(--min-tokens "$INPUT_MIN_TOKENS")
      [ -n "${INPUT_MIN_LINES:-}" ] && ARGS+=(--min-lines "$INPUT_MIN_LINES")
      [ -n "${INPUT_THRESHOLD:-}" ] && ARGS+=(--threshold "$INPUT_THRESHOLD")
      [ "${INPUT_SKIP_LOCAL:-}" = "true" ] && ARGS+=(--skip-local)
      [ "${INPUT_CROSS_LANGUAGE:-}" = "true" ] && ARGS+=(--cross-language)
      [ "$include_top" = "true" ] && [ -n "${INPUT_TOP:-}" ] && ARGS+=(--top "$INPUT_TOP")
      ;;
    health)
      [ -n "${INPUT_MAX_CYCLOMATIC:-}" ] && ARGS+=(--max-cyclomatic "$INPUT_MAX_CYCLOMATIC")
      [ -n "${INPUT_MAX_COGNITIVE:-}" ] && ARGS+=(--max-cognitive "$INPUT_MAX_COGNITIVE")
      [ "$include_top" = "true" ] && [ -n "${INPUT_TOP:-}" ] && ARGS+=(--top "$INPUT_TOP")
      [ -n "${INPUT_SORT:-}" ] && ARGS+=(--sort "$INPUT_SORT")
      [ "${INPUT_FILE_SCORES:-}" = "true" ] && ARGS+=(--file-scores)
      [ "${INPUT_HOTSPOTS:-}" = "true" ] && ARGS+=(--hotspots)
      [ "${INPUT_TARGETS:-}" = "true" ] && ARGS+=(--targets)
      [ "${INPUT_COMPLEXITY:-}" = "true" ] && ARGS+=(--complexity)
      [ -n "${INPUT_SINCE:-}" ] && ARGS+=(--since "$INPUT_SINCE")
      [ -n "${INPUT_MIN_COMMITS:-}" ] && ARGS+=(--min-commits "$INPUT_MIN_COMMITS")
      if [ -n "${INPUT_SAVE_SNAPSHOT:-}" ]; then
        if [ "$INPUT_SAVE_SNAPSHOT" = "true" ]; then
          ARGS+=(--save-snapshot)
        else
          ARGS+=(--save-snapshot "$INPUT_SAVE_SNAPSHOT")
        fi
      fi
      ;;
    fix)
      if [ "${INPUT_DRY_RUN:-}" = "true" ]; then
        ARGS+=(--dry-run)
      else
        ARGS+=(--yes)
      fi
      ;;
    "")
      if [ "${INPUT_FORMAT:-}" = "sarif" ] && [ "${HAS_SARIF_FILE:-false}" = "true" ]; then
        ARGS+=(--sarif-file fallow-results.sarif)
      fi
      ;;
  esac
}

# --- Validation ---

case "$INPUT_COMMAND" in
  ""|dead-code|check|dupes|health|fix) ;;
  *) echo "::error::Invalid command: ${INPUT_COMMAND}. Must be dead-code, dupes, health, fix, or empty (runs all)."; exit 2 ;;
esac

for name_val in "min-tokens:${INPUT_MIN_TOKENS:-}" "min-lines:${INPUT_MIN_LINES:-}" \
               "max-cyclomatic:${INPUT_MAX_CYCLOMATIC:-}" "max-cognitive:${INPUT_MAX_COGNITIVE:-}" \
               "top:${INPUT_TOP:-}" "min-commits:${INPUT_MIN_COMMITS:-}" "threads:${INPUT_THREADS:-}"; do
  name="${name_val%%:*}"; val="${name_val#*:}"
  if [ -n "$val" ] && ! [[ "$val" =~ ^[0-9]+$ ]]; then
    echo "::error::${name} must be a positive integer, got: ${val}"; exit 2
  fi
done
if [ -n "${INPUT_THRESHOLD:-}" ] && ! [[ "$INPUT_THRESHOLD" =~ ^[0-9]+\.?[0-9]*$ ]]; then
  echo "::error::threshold must be a number, got: ${INPUT_THRESHOLD}"; exit 2
fi

# --- Check for --sarif-file support ---

HAS_SARIF_FILE=false
if { [ "$INPUT_COMMAND" = "dead-code" ] || [ "$INPUT_COMMAND" = "check" ] || [ -z "$INPUT_COMMAND" ]; }; then
  HELP_TMP=$(mktemp)
  fallow dead-code --help > "$HELP_TMP" 2>/dev/null || true
  if /usr/bin/grep -q -- '--sarif-file' "$HELP_TMP"; then
    HAS_SARIF_FILE=true
  fi
  rm -f "$HELP_TMP"
fi

# --- Auto-detect changed-since in PR context ---

if [ -z "${INPUT_CHANGED_SINCE:-}" ] && [ "${INPUT_AUTO_CHANGED_SINCE:-}" = "true" ] && \
   { [ "${EVENT_NAME:-}" = "pull_request" ] || [ "${EVENT_NAME:-}" = "pull_request_target" ]; } && \
   [ -n "${PR_BASE_SHA:-}" ]; then
  INPUT_CHANGED_SINCE="$PR_BASE_SHA"
  echo "::notice::Auto-scoping analysis to files changed since PR base (${PR_BASE_SHA:0:7})"
fi

# --- Build and run main analysis ---

ARGS=()
build_common_args json
build_command_args true

# Parse extra arguments safely
EXTRA_ARGS=()
if [ -n "${INPUT_ARGS:-}" ]; then
  read -ra EXTRA_ARGS <<< "$INPUT_ARGS"
fi

# Run analysis — no --fail-on-issues so subsequent steps always run
if ! fallow "${ARGS[@]}" "${EXTRA_ARGS[@]}" > fallow-results.json 2> fallow-stderr.log; then
  if [ ! -s fallow-results.json ] || ! jq -e '.' fallow-results.json > /dev/null 2>&1; then
    echo "::error::Fallow failed to run"
    [ -s fallow-stderr.log ] && cat fallow-stderr.log
    [ -s fallow-results.json ] && cat fallow-results.json
    exit 2
  fi
fi

# --- Fallback SARIF generation ---

if [ "${INPUT_FORMAT:-}" = "sarif" ] && [ "$INPUT_COMMAND" != "fix" ] && \
   { [ ! -f fallow-results.sarif ] || ! jq -e '.' fallow-results.sarif > /dev/null 2>&1; }; then
  ARGS=()
  build_common_args sarif
  build_command_args false  # omit --top for SARIF

  if ! fallow "${ARGS[@]}" "${EXTRA_ARGS[@]}" > fallow-results.sarif 2>/dev/null; then
    echo "::warning::SARIF generation failed"
  fi
fi

# --- Surface warnings from stderr ---

if [ -s fallow-stderr.log ]; then
  while IFS= read -r line; do
    echo "::debug::${line}"
  done < fallow-stderr.log
fi

# --- Extract issue count ---

case "$INPUT_COMMAND" in
  dead-code|check) ISSUES=$(jq -r '.total_issues' fallow-results.json) ;;
  dupes)           ISSUES=$(jq -r '.stats.clone_groups' fallow-results.json) ;;
  health)          ISSUES=$(jq -r '.summary.functions_above_threshold' fallow-results.json) ;;
  fix)             ISSUES=$(jq -r '(.fixes | length)' fallow-results.json) ;;
  "")              ISSUES=$(jq -r '((.check.total_issues // 0) + (.dupes.stats.clone_groups // 0) + (.health.summary.functions_above_threshold // 0))' fallow-results.json) ;;
esac

if ! [[ "$ISSUES" =~ ^[0-9]+$ ]]; then
  echo "::error::Unexpected issue count: ${ISSUES}"
  exit 2
fi

echo "issues=${ISSUES}" >> "$GITHUB_OUTPUT"
echo "results=fallow-results.json" >> "$GITHUB_OUTPUT"
echo "command=${INPUT_COMMAND}" >> "$GITHUB_OUTPUT"

if [ -f fallow-results.sarif ]; then
  echo "sarif=fallow-results.sarif" >> "$GITHUB_OUTPUT"
fi

if [ "$ISSUES" -gt 0 ]; then
  case "$INPUT_COMMAND" in
    dead-code|check) echo "::warning::Fallow found ${ISSUES} unused code issues" ;;
    dupes)           echo "::warning::Fallow found ${ISSUES} clone groups" ;;
    health)          echo "::warning::Fallow found ${ISSUES} high complexity functions" ;;
    fix)             echo "::warning::Fallow proposed ${ISSUES} fixes" ;;
    "")              echo "::warning::Fallow found ${ISSUES} issues" ;;
  esac
fi
