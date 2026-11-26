#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SRC="${1:-}"
OUT="${2:-}"

if [[ -z "$SRC" || -z "$OUT" ]]; then
  echo "usage: tools/speed-dreams/build-car.sh <source-dir> <output.sima>" >&2
  exit 2
fi
if [[ ! -d "$SRC" ]]; then
  echo "missing source directory: $SRC" >&2
  exit 1
fi
SRC="$(cd "$SRC" && pwd)"

CONFIG="$SRC/import.json"
if [[ ! -f "$CONFIG" ]]; then
  echo "missing import config: $CONFIG" >&2
  exit 1
fi

read_config() {
  node -e '
    const fs = require("fs");
    const config = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
    const value = process.argv[2].split(".").reduce((obj, key) => obj?.[key], config);
    if (value === undefined) process.exit(1);
    process.stdout.write(String(value));
  ' "$CONFIG" "$1"
}

BODY="$SRC/$(read_config body)"
XML="$SRC/$(read_config xml)"
WHEEL="$SRC/$(read_config wheel)"
CATEGORY="$(read_config category || true)"

if [[ "$CATEGORY" != "road" && "$CATEGORY" != "gp67" ]]; then
  echo "import.json must define category as road or gp67" >&2
  exit 1
fi

for file in "$BODY" "$XML" "$WHEEL"; do
  if [[ ! -f "$file" ]]; then
    echo "missing source file: $file" >&2
    exit 1
  fi
done

if ! command -v assimp >/dev/null 2>&1; then
  echo "assimp is required; see tools/speed-dreams/README.md" >&2
  exit 1
fi
if [[ ! -d "$ROOT/tools/speed-dreams/node_modules/@gltf-transform/core" ]]; then
  echo "install tool dependencies first: npm install --prefix tools/speed-dreams" >&2
  exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

assimp export "$BODY" "$TMP/body.glb" -f glb2 -embtex >/dev/null
assimp export "$WHEEL" "$TMP/wheel.glb" -f glb2 -embtex >/dev/null
NAME="$(basename "$OUT")"
NAME="${NAME%.sima}"
node "$ROOT/tools/speed-dreams/assemble.mjs" "$CONFIG" "$TMP/body.glb" "$TMP/wheel.glb" "$TMP/$NAME.glb"
mkdir -p "$(dirname "$OUT")"
cargo run -q -p asset-compiler -- bake-glb "$TMP/$NAME.glb" "$OUT" --category "$CATEGORY" --textures --physics-xml "$XML"
