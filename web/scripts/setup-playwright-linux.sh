#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  exit 0
fi

if ldconfig -p 2>/dev/null | grep -q 'libnspr4.so'; then
  exit 0
fi

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TARGET="$ROOT/web/.cache/playwright-libs/usr/lib/x86_64-linux-gnu"
if [[ -e "$TARGET/libnspr4.so" ]]; then
  exit 0
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cd "$WORK"
apt-get download libnspr4 libnss3 >/dev/null
mkdir -p extract
for package in ./*.deb; do
  dpkg-deb -x "$package" extract
done
mkdir -p "$TARGET"
cp -a extract/usr/lib/x86_64-linux-gnu/. "$TARGET/"
