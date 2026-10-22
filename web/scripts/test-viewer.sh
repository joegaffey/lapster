#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT/web"

bash scripts/setup-playwright-linux.sh

export LD_LIBRARY_PATH="$ROOT/web/.cache/playwright-libs/usr/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
exec npx playwright test --config=playwright.config.ts "$@"
