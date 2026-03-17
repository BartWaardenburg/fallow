#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="$ROOT_DIR/assets/screenshots"
OUTPUT_FILE="$OUTPUT_DIR/fallow-check-output.png"
FIXTURE_ROOT="$ROOT_DIR/tests/fixtures/cjs-project"

if ! command -v freeze >/dev/null 2>&1; then
  echo "freeze is required. Install with: brew install charmbracelet/tap/freeze" >&2
  exit 1
fi

mkdir -p "$OUTPUT_DIR"

tmp_script="$(mktemp)"
trap 'rm -f "$tmp_script"' EXIT

cat >"$tmp_script" <<EOF
#!/usr/bin/env bash
set -euo pipefail

cd "$ROOT_DIR"

if [[ -x "$ROOT_DIR/target/debug/fallow" ]]; then
  FALLOW_CMD=("$ROOT_DIR/target/debug/fallow" check)
else
  FALLOW_CMD=(cargo run -q -p fallow-cli -- check)
fi

printf '\$ fallow check\n\n'
CLICOLOR_FORCE=1 "\${FALLOW_CMD[@]}" -r "$FIXTURE_ROOT" 2>&1 | sed '/fallow_analyze:/d'
EOF
chmod +x "$tmp_script"

freeze --execute "bash \"$tmp_script\"" \
  -o "$OUTPUT_FILE" \
  --window \
  --padding 20 \
  --shadow.blur 10 \
  --shadow.x 0 \
  --shadow.y 8

echo "Wrote $OUTPUT_FILE"
