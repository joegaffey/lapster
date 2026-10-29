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
mkdir -p "$DIST/web/pkg/$VERSION"
cp -R "$ROOT/web/pkg/." "$DIST/web/pkg/$VERSION/"
cp "$ROOT/web/version.json" "$DIST/web/version.json"
cp "$ROOT/web/sw.js" "$DIST/web/sw.js"
cp "$ROOT/web/manifest.webmanifest" "$DIST/web/manifest.webmanifest"
sed "s#__VERSION__#${VERSION}#g" "$ROOT/web/index.html" > "$DIST/web/index.html"
cp -R "$ROOT/web/icons" "$DIST/web/icons"
cp -R "$ROOT/web/assets" "$DIST/web/assets"
mkdir -p "$DIST/web/worker"
cp "$ROOT/web/worker/physics-worker.js" "$DIST/web/worker/physics-worker.js"
mkdir -p "$DIST/web/audio"
cp "$ROOT/web/audio/combustion-worklet.js" "$DIST/web/audio/combustion-worklet.js"

for viewer in drive garage track-viewer debug-viewer; do
  mkdir -p "$DIST/web/$viewer"
  sed \
    -e "s#\./main\.js#./main.js?v=${VERSION}#g" \
    -e "s#\./viewer\.js#./viewer.js?v=${VERSION}#g" \
    "$ROOT/web/$viewer/index.html" > "$DIST/web/$viewer/index.html"
  for file in main.js viewer.js; do
    if [[ -f "$ROOT/web/$viewer/$file" ]]; then
      cp "$ROOT/web/$viewer/$file" "$DIST/web/$viewer/$file"
      python3 - "$DIST/web/$viewer/$file" "$VERSION" <<'PY'
from pathlib import Path
import sys
path = Path(sys.argv[1])
version = sys.argv[2]
source = path.read_text()
source = source.replace('"../pkg/', f'"../pkg/{version}/')
path.write_text(source)
PY
    fi
  done
done

if [[ -f "$DIST/web/worker/physics-worker.js" ]]; then
  python3 - "$DIST/web/worker/physics-worker.js" "$VERSION" <<'PY'
from pathlib import Path
import sys
path = Path(sys.argv[1])
version = sys.argv[2]
source = path.read_text()
source = source.replace('"../pkg/', f'"../pkg/{version}/')
path.write_text(source)
PY
fi

cp -R "$ROOT/fixtures/cars" "$DIST/fixtures/cars"
cp -R "$ROOT/fixtures/tracks" "$DIST/fixtures/tracks"
cp "$ROOT/fixtures/README.md" "$DIST/fixtures/README.md"
cp "$ROOT/THIRD_PARTY_NOTICES.md" "$DIST/THIRD_PARTY_NOTICES.md"
touch "$DIST/.nojekyll"

echo "Built Pages site in $DIST"
