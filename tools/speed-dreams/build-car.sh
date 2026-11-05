#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SRC="${1:-}"
OUT="${2:-}"

if [[ -z "$SRC" || -z "$OUT" ]]; then
  echo "usage: tools/speed-dreams/build-car.sh <source-dir> <output.sima>" >&2
  exit 2
fi

BODY="$SRC/body/sc-deckard-conejo.acc"
XML="$SRC/body/sc-deckard-conejo.xml"
WHEEL="$SRC/wheel/wheel0.acc"
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
node "$ROOT/tools/speed-dreams/assemble.mjs" "$TMP/body.glb" "$TMP/wheel.glb" "$TMP/$NAME.glb"
mkdir -p "$(dirname "$OUT")"
cargo run -q -p asset-compiler -- bake-glb "$TMP/$NAME.glb" "$OUT" --textures --physics-xml "$XML"
