#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

if [[ ! -f dist/web/version.json ]]; then
  (cd web && npm run build:viewer)
  bash web/scripts/build-pages.sh
fi

port_free() {
  python3 - "$1" "$2" <<'PY'
import socket
import sys

host = sys.argv[1]
port = int(sys.argv[2])
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
try:
    sock.bind((host, port))
except OSError:
    sys.exit(1)
finally:
    sock.close()
PY
}

if [[ -n "${RELAY_BIND:-}" ]]; then
  BIND="$RELAY_BIND"
  HOST="${BIND%:*}"
  PORT="${BIND##*:}"
  if ! port_free "$HOST" "$PORT"; then
    echo "Relay port $PORT is already in use; choose RELAY_BIND=host:port." >&2
    exit 1
  fi
elif [[ -n "${CODESPACES:-}" ]]; then
  BIND="0.0.0.0:3000"
  if ! port_free "0.0.0.0" "3000"; then
    echo "Relay port 3000 is already in use; stop that process or set RELAY_BIND." >&2
    exit 1
  fi
else
  BIND=""
  for candidate in 3000 3001 3002 3003 3004 3005 3006 3007 3008 3009 3010; do
    if port_free "127.0.0.1" "$candidate"; then
      BIND="127.0.0.1:${candidate}"
      break
    fi
  done
  if [[ -z "$BIND" ]]; then
    echo "No free relay port found in 3000–3010." >&2
    exit 1
  fi
fi

PORT="${BIND##*:}"
echo "Open http://127.0.0.1:${PORT}/web/multiplayer/"
exec cargo run -p relay-server -- \
  --static-dir dist \
  --bind "$BIND" \
  --max-players "${RELAY_MAX_PLAYERS:-4}"
