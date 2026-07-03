#!/usr/bin/env bash
# Run Tauri dev/build using the major version matching this host.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TAURI_MAJOR="$("$ROOT/scripts/detect-tauri-version.sh")"
CMD="${1:-dev}"
shift || true

run_cmd() {
  local runner=( "$@" )
  if [ "${RUN_UNDER_XVFB:-}" = "1" ]; then
    exec xvfb-run -a "${runner[@]}"
  else
    exec "${runner[@]}"
  fi
}

if [ "$CMD" = "dev" ]; then
  # shellcheck source=ensure-display.sh
  source "$ROOT/scripts/ensure-display.sh"
  ensure_display dev
fi

case "$TAURI_MAJOR" in
  1)
    echo "Using Tauri 1.x (WebKitGTK 4.0 — Ubuntu 20.04 compatible)"
    cd "$ROOT/src-tauri-v1"
    run_cmd npx --yes @tauri-apps/cli@1 "$CMD" "$@"
    ;;
  2)
    echo "Using Tauri 2.x (WebKitGTK 4.1+)"
    cd "$ROOT"
    run_cmd cargo tauri "$CMD" "$@"
    ;;
  *)
    echo "Unsupported Tauri major version: $TAURI_MAJOR" >&2
    exit 1
    ;;
esac
