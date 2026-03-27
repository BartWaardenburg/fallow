#!/usr/bin/env bash
set -eo pipefail

# Post review comments with rich markdown formatting
# Required env: GH_TOKEN, PR_NUMBER, GH_REPO, FALLOW_COMMAND, FALLOW_ROOT,
#   MAX_COMMENTS, ACTION_JQ_DIR

MAX="${MAX_COMMENTS:-50}"

# Dismiss previous fallow review to avoid stacking
PREV_REVIEW_ID=$(gh api \
  "repos/${GH_REPO}/pulls/${PR_NUMBER}/reviews" \
  --jq '.[] | select(.user.login == "github-actions[bot]" and (.body | contains("<!-- fallow-review -->"))) | .id' \
  2>/dev/null | head -1)
if [ -n "$PREV_REVIEW_ID" ]; then
  gh api "repos/${GH_REPO}/pulls/${PR_NUMBER}/reviews/${PREV_REVIEW_ID}" \
    --method PUT --field event=DISMISS \
    --field message="Superseded by new analysis" > /dev/null 2>&1 || true
fi

# Prefix for paths: if root is not ".", prepend it
PREFIX=""
if [ "$FALLOW_ROOT" != "." ]; then
  PREFIX="${FALLOW_ROOT}/"
fi

# Export env vars for jq access
export PREFIX MAX FALLOW_ROOT

# Collect all review comments from the results
COMMENTS="[]"
case "$FALLOW_COMMAND" in
  dead-code|check)
    COMMENTS=$(jq -f "${ACTION_JQ_DIR}/review-comments-check.jq" fallow-results.json 2>&1) || { echo "jq check error: $COMMENTS"; COMMENTS="[]"; } ;;
  dupes)
    COMMENTS=$(jq -f "${ACTION_JQ_DIR}/review-comments-dupes.jq" fallow-results.json 2>&1) || { echo "jq dupes error: $COMMENTS"; COMMENTS="[]"; } ;;
  health)
    COMMENTS=$(jq -f "${ACTION_JQ_DIR}/review-comments-health.jq" fallow-results.json 2>&1) || { echo "jq health error: $COMMENTS"; COMMENTS="[]"; } ;;
  "")
    # Combined: extract each section and run through its jq script
    TMPDIR=$(mktemp -d)
    jq '.check // {}' fallow-results.json > "$TMPDIR/check.json" 2>/dev/null
    jq '.dupes // {}' fallow-results.json > "$TMPDIR/dupes.json" 2>/dev/null
    jq '.health // {}' fallow-results.json > "$TMPDIR/health.json" 2>/dev/null
    CHECK=$(jq -f "${ACTION_JQ_DIR}/review-comments-check.jq" "$TMPDIR/check.json" 2>/dev/null || echo "[]")
    DUPES=$(jq -f "${ACTION_JQ_DIR}/review-comments-dupes.jq" "$TMPDIR/dupes.json" 2>/dev/null || echo "[]")
    HEALTH=$(jq -f "${ACTION_JQ_DIR}/review-comments-health.jq" "$TMPDIR/health.json" 2>/dev/null || echo "[]")
    COMMENTS=$(echo "$CHECK" "$DUPES" "$HEALTH" | jq -s 'add | .[:'"$MAX"']')
    rm -rf "$TMPDIR" ;;
esac

TOTAL=$(echo "$COMMENTS" | jq 'length')
if [ "$TOTAL" -eq 0 ]; then
  echo "No review comments to post"
  exit 0
fi

echo "Posting $TOTAL review comments..."

# Build the review payload
REVIEW_BODY=$'**Fallow** found issues in this PR \u2014 see inline comments below.\n\n<!-- fallow-review -->'
PAYLOAD=$(echo "$COMMENTS" | jq --arg body "$REVIEW_BODY" '{
  event: "COMMENT",
  body: $body,
  comments: [.[] | {path: .path, line: .line, body: .body}]
}')

# Post the review
if ! echo "$PAYLOAD" | gh api \
  "repos/${GH_REPO}/pulls/${PR_NUMBER}/reviews" \
  --method POST \
  --input - > /dev/null 2>&1; then
  echo "::warning::Failed to post review comments. Some findings may be on lines not in the PR diff."

  # Fallback: post comments one by one, skipping failures
  POSTED=0
  for i in $(seq 0 $((TOTAL - 1))); do
    SINGLE=$(echo "$COMMENTS" | jq --arg body "$REVIEW_BODY" '{
      event: "COMMENT",
      body: (if '"$i"' == 0 then $body else "" end),
      comments: [.['"$i"']]
    }')
    if echo "$SINGLE" | gh api \
      "repos/${GH_REPO}/pulls/${PR_NUMBER}/reviews" \
      --method POST \
      --input - > /dev/null 2>&1; then
      POSTED=$((POSTED + 1))
    fi
  done
  echo "Posted $POSTED of $TOTAL comments individually"
else
  echo "Posted review with $TOTAL inline comments"
fi
