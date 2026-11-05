#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

if [[ ! -x target/debug/relay-server ]]; then
  cargo build -p relay-server
fi

exec target/debug/relay-server \
  --static-dir web/relay-landing \
  --bind "${RELAY_BIND:-0.0.0.0:3000}" \
  --max-players "${RELAY_MAX_PLAYERS:-4}"
