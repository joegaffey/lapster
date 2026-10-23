#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DIST="$ROOT/dist"

if [[ ! -f "$ROOT/web/pkg/sim_render_bg.wasm" ]]; then
  echo "web/pkg/sim_render_bg.wasm is missing; run npm run build:viewer first" >&2
  exit 1
fi

if [[ ! -f "$ROOT/web/version.json" ]]; then
  echo "web/version.json is missing; run npm run build:viewer first" >&2
  exit 1
fi

VERSION="$(python3 -c 'import json; print(json.load(open("web/version.json"))["version"])')"

rm -rf "$DIST"
mkdir -p "$DIST/web" "$DIST/fixtures"

cp "$ROOT/index.html" "$DIST/index.html"
cp -R "$ROOT/web/pkg" "$DIST/web/pkg"
cp "$ROOT/web/version.json" "$DIST/web/version.json"
cp "$ROOT/web/sw.js" "$DIST/web/sw.js"
cp "$ROOT/web/manifest.webmanifest" "$DIST/web/manifest.webmanifest"
cp "$ROOT/web/index.html" "$DIST/web/index.html"
cp -R "$ROOT/web/icons" "$DIST/web/icons"

for viewer in drive garage track-viewer debug-viewer; do
  mkdir -p "$DIST/web/$viewer"
  sed \
    -e "s#\./main\.js#./main.js?v=${VERSION}#g" \
    -e "s#\./viewer\.js#./viewer.js?v=${VERSION}#g" \
    "$ROOT/web/$viewer/index.html" > "$DIST/web/$viewer/index.html"
  for file in main.js viewer.js; do
    if [[ -f "$ROOT/web/$viewer/$file" ]]; then
      cp "$ROOT/web/$viewer/$file" "$DIST/web/$viewer/$file"
    fi
  done
done

cp -R "$ROOT/fixtures/cars" "$DIST/fixtures/cars"
cp -R "$ROOT/fixtures/tracks" "$DIST/fixtures/tracks"
cp "$ROOT/fixtures/README.md" "$DIST/fixtures/README.md"
touch "$DIST/.nojekyll"

echo "Built Pages site in $DIST"
