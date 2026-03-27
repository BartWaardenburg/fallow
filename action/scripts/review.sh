#!/usr/bin/env bash
set -eo pipefail

# Post review comments with rich markdown formatting
# Required env: GH_TOKEN, PR_NUMBER, GH_REPO, FALLOW_COMMAND, FALLOW_ROOT,
#   MAX_COMMENTS, ACTION_JQ_DIR

MAX="${MAX_COMMENTS:-50}"
if ! [[ "$MAX" =~ ^[0-9]+$ ]]; then
  echo "::warning::max-annotations must be a positive integer, got: ${MAX_COMMENTS}. Using default: 50"
  MAX=50
fi

# Reject path traversal in root
if [[ "${FALLOW_ROOT:-}" =~ \.\. ]]; then
  echo "::error::root input contains path traversal sequence"
  exit 2
fi

# Clean up ALL previous review comments from github-actions[bot]
while read -r CID; do
  gh api "repos/${GH_REPO}/pulls/comments/${CID}" --method DELETE > /dev/null 2>&1 || true
done < <(gh api "repos/${GH_REPO}/pulls/${PR_NUMBER}/comments" --paginate \
  --jq '.[] | select(.user.login == "github-actions[bot]") | .id' 2>/dev/null)

# Dismiss previous fallow reviews
gh api "repos/${GH_REPO}/pulls/${PR_NUMBER}/reviews" --paginate \
  --jq '.[] | select(.user.login == "github-actions[bot]" and .state != "DISMISSED") | .id' 2>/dev/null | while read -r RID; do
  gh api "repos/${GH_REPO}/pulls/${PR_NUMBER}/reviews/${RID}" \
    --method PUT --field event=DISMISS \
    --field message="Superseded by new analysis" > /dev/null 2>&1 || true
done

# Prefix for paths: if root is not ".", prepend it
PREFIX=""
if [ "$FALLOW_ROOT" != "." ]; then
  PREFIX="${FALLOW_ROOT}/"
fi

# Export env vars for jq access
export PREFIX MAX FALLOW_ROOT GH_REPO PR_NUMBER PR_HEAD_SHA

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
    WORK_DIR=$(mktemp -d)
    jq '.check // {}' fallow-results.json > "$WORK_DIR/check.json" 2>/dev/null
    jq '.dupes // {}' fallow-results.json > "$WORK_DIR/dupes.json" 2>/dev/null
    jq '.health // {}' fallow-results.json > "$WORK_DIR/health.json" 2>/dev/null
    CHECK=$(jq -f "${ACTION_JQ_DIR}/review-comments-check.jq" "$WORK_DIR/check.json" 2>/dev/null || echo "[]")
    DUPES=$(jq -f "${ACTION_JQ_DIR}/review-comments-dupes.jq" "$WORK_DIR/dupes.json" 2>/dev/null || echo "[]")
    HEALTH=$(jq -f "${ACTION_JQ_DIR}/review-comments-health.jq" "$WORK_DIR/health.json" 2>/dev/null || echo "[]")
    COMMENTS=$(echo "$CHECK" "$DUPES" "$HEALTH" | jq -s 'add | .[:'"$MAX"']')
    rm -rf "$WORK_DIR" ;;
esac

# Post-process: group unused exports, dedup clones, drop refactoring targets, merge same-line
MERGED=$(echo "$COMMENTS" | jq --argjson max "$MAX" -f "${ACTION_JQ_DIR}/merge-comments.jq" 2>&1) && COMMENTS="$MERGED" || echo "Merge warning: $MERGED"

TOTAL=$(echo "$COMMENTS" | jq 'length')
if [ "$TOTAL" -eq 0 ]; then
  echo "No review comments to post"
  exit 0
fi

echo "Posting $TOTAL review comments (after merging)..."

# Generate rich review body from the analysis results
REVIEW_BODY=""
if [ -f "${ACTION_JQ_DIR}/review-body.jq" ]; then
  REVIEW_BODY=$(jq -r -f "${ACTION_JQ_DIR}/review-body.jq" fallow-results.json 2>&1) || true
fi
# Fallback if jq failed or produced empty output
if [ -z "$REVIEW_BODY" ] || echo "$REVIEW_BODY" | /usr/bin/grep -q "^jq:"; then
  REVIEW_BODY=$'## \xf0\x9f\x8c\xbf Fallow Review\n\nFound **'"$TOTAL"$'** issues \xe2\x80\x94 see inline comments below.\n\n<!-- fallow-review -->'
fi

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
      comments: [.['"$i"'] | {path, line, body}]
    }')
    RESULT=$(echo "$SINGLE" | gh api \
      "repos/${GH_REPO}/pulls/${PR_NUMBER}/reviews" \
      --method POST \
      --input - 2>&1) && POSTED=$((POSTED + 1)) || \
      echo "  Skip: $(echo "$COMMENTS" | jq -r ".[${i}].path"):$(echo "$COMMENTS" | jq -r ".[${i}].line")"
  done
  echo "Posted $POSTED of $TOTAL comments individually"
else
  echo "Posted review with $TOTAL inline comments"
fi
