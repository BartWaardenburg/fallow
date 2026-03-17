#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="$ROOT_DIR/assets/screenshots"
OUTPUT_FILE="$OUTPUT_DIR/fallow-check-output.png"

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

printf '\$ fallow check\n\n'

printf '\033[33m●\033[0m \033[1;33mUnused files (3)\033[0m\n'
printf '  src/legacy/oldUtils.ts\n'
printf '  src/components/DeprecatedBanner.tsx\n'
printf '  src/lib/unused-helper.ts\n\n'

printf '\033[36m●\033[0m \033[1;36mUnused exports (3)\033[0m\n'
printf '  \033[2msrc/utils/format.ts\033[0m\n'
printf '    \033[2m:12\033[0m \033[1mformatCurrency\033[0m\n'
printf '    \033[2m:28\033[0m \033[1mformatPercentage\033[0m\n'
printf '  \033[2msrc/hooks/useAuth.ts\033[0m\n'
printf '    \033[2m:9\033[0m \033[1mAuthContext\033[0m\n'
printf '\n'

printf '\033[36m●\033[0m \033[1;36mUnused type exports (1)\033[0m\n'
printf '  \033[2msrc/types/ledger.ts\033[0m\n'
printf '    \033[2m:6\033[0m \033[1mLegacyLedgerRow\033[0m\n\n'

printf '\033[33m●\033[0m \033[1;33mUnused dependencies (2)\033[0m\n'
printf '  \033[1mmoment\033[0m\n'
printf '  \033[1mlodash\033[0m\n\n'

printf '\033[35m●\033[0m \033[1;35mUnlisted dependencies (1)\033[0m\n'
printf '  \033[1mchalk\033[0m\n\n'

printf '\033[31m●\033[0m \033[1;31mUnresolved imports (1)\033[0m\n'
printf '  \033[2msrc/index.ts\033[0m\n'
printf '    \033[2m:3\033[0m \033[1m@/missing/module\033[0m\n\n'

printf '\033[1;31m✗ Found 11 issues (0.04s)\033[0m\n'
EOF
chmod +x "$tmp_script"

freeze --execute "bash \"$tmp_script\"" \
  -o "$OUTPUT_FILE" \
  --width 980 \
  --window \
  --padding 20 \
  --font.size 18 \
  --shadow.blur 10 \
  --shadow.x 0 \
  --shadow.y 8

echo "Wrote $OUTPUT_FILE"
