#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

VERSION="${LAPSTER_BUILD_VERSION:-$(git rev-parse --short HEAD 2>/dev/null || echo dev)-$(date +%s)}"
CHANNEL="${LAPSTER_BUILD_CHANNEL:-stable}"
case "$CHANNEL" in
  stable|dev) ;;
  *) echo "LAPSTER_BUILD_CHANNEL must be stable or dev" >&2; exit 1 ;;
esac
mkdir -p web/src
printf 'export const APP_VERSION = "%s";\nexport const BUILD_CHANNEL = "%s";\n' "$VERSION" "$CHANNEL" > web/src/version.generated.ts
printf '{"version":"%s"}\n' "$VERSION" > web/version.json

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

cargo build -p sim-worker --release --target wasm32-unknown-unknown
rm -rf web/pkg/worker
wasm-bindgen --target web --out-dir web/pkg/worker target/wasm32-unknown-unknown/release/sim_worker.wasm
"$WASM_OPT" -O3 \
  --enable-bulk-memory \
  --enable-bulk-memory-opt \
  --enable-nontrapping-float-to-int \
  --enable-mutable-globals \
  --enable-sign-ext \
  --enable-reference-types \
  --enable-multivalue \
  web/pkg/worker/sim_worker_bg.wasm -o web/pkg/worker/sim_worker_bg.wasm.opt
mv web/pkg/worker/sim_worker_bg.wasm.opt web/pkg/worker/sim_worker_bg.wasm

npx esbuild web/track-viewer/main.ts --bundle --format=esm --outfile=web/track-viewer/main.js --external:../pkg/*
npx esbuild web/garage/main.ts --bundle --format=esm --outfile=web/garage/main.js --external:../pkg/*
npx esbuild web/drive/main.ts --bundle --format=esm --outfile=web/drive/main.js --external:../pkg/*
npx esbuild web/worker/physics-worker.ts --bundle --format=esm --outfile=web/worker/physics-worker.js --external:../pkg/worker/*
mkdir -p web/audio
cp web/src/audio/engine-sim/combustion-worklet.js web/audio/combustion-worklet.js
