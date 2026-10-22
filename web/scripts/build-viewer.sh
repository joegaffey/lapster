#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

export PATH="$HOME/.cargo/bin:$PATH"

cargo build -p sim-render --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir web/pkg target/wasm32-unknown-unknown/release/sim_render.wasm
WASM_OPT="$(bash web/scripts/ensure-wasm-opt.sh)"
"$WASM_OPT" -O3 \
  --enable-bulk-memory \
  --enable-bulk-memory-opt \
  --enable-nontrapping-float-to-int \
  --enable-mutable-globals \
  --enable-sign-ext \
  --enable-reference-types \
  --enable-multivalue \
  web/pkg/sim_render_bg.wasm -o web/pkg/sim_render_bg.wasm.opt
mv web/pkg/sim_render_bg.wasm.opt web/pkg/sim_render_bg.wasm
npx esbuild web/track-viewer/main.ts --bundle --format=esm --outfile=web/track-viewer/main.js --external:../pkg/*
npx esbuild web/garage/main.ts --bundle --format=esm --outfile=web/garage/main.js --external:../pkg/*
npx esbuild web/drive/main.ts --bundle --format=esm --outfile=web/drive/main.js --external:../pkg/*
