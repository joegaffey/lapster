#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SRC="$ROOT/web/track-ed-src"
OUT="$ROOT/web/track-ed"

if [ ! -d "$SRC/node_modules" ]; then
  echo "error: $SRC/node_modules is missing; run 'npm ci' in web/track-ed-src" >&2
  exit 1
fi

(cd "$SRC" && VITE_LAPSTER_HOST=1 npm run build)

rm -rf "$OUT"
mkdir -p "$OUT"
cp -R "$SRC/dist/." "$OUT/"

for file in THIRD_PARTY_NOTICES.md LICENSE LICENSE-MIT LICENSE-APACHE; do
  cp "$SRC/$file" "$OUT/$file"
done

echo "Built hosted track-ed in web/track-ed"
