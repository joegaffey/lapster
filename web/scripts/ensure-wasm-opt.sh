#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CACHE="$ROOT/web/.cache/binaryen"
BIN="$CACHE/bin/wasm-opt"
VERSION="version_123"

if command -v wasm-opt >/dev/null 2>&1; then
  command -v wasm-opt
  exit 0
fi

if [[ -x "$BIN" ]]; then
  echo "$BIN"
  exit 0
fi

mkdir -p "$CACHE"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
curl -L -sS -o "$WORK/binaryen.tar.gz" \
  "https://github.com/WebAssembly/binaryen/releases/download/${VERSION}/binaryen-${VERSION}-x86_64-linux.tar.gz"
tar -xzf "$WORK/binaryen.tar.gz" -C "$CACHE" --strip-components=1
echo "$BIN"
