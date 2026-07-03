#!/usr/bin/env bash
# Detect Tauri major version for the current host.
# Prints "1" (WebKitGTK 4.0 / Ubuntu 20.04) or "2" (WebKitGTK 4.1+).
set -euo pipefail

if [ -n "${TAURI_MAJOR:-}" ]; then
  case "$TAURI_MAJOR" in
    1|2) echo "$TAURI_MAJOR"; exit 0 ;;
    *) echo "Invalid TAURI_MAJOR=$TAURI_MAJOR (expected 1 or 2)" >&2; exit 1 ;;
  esac
fi

case "$(uname -s 2>/dev/null || echo Unknown)" in
  Darwin|MINGW*|MSYS*|CYGWIN*)
    echo 2
    exit 0
    ;;
esac

if command -v pkg-config >/dev/null 2>&1; then
  if pkg-config --exists 'webkit2gtk-4.1' 2>/dev/null; then
    echo 2
    exit 0
  fi
  if pkg-config --exists 'webkit2gtk-4.0' 2>/dev/null; then
    echo 1
    exit 0
  fi
fi

if [ -f /etc/os-release ]; then
  # shellcheck disable=SC1091
  . /etc/os-release
  case "${ID:-}" in
    ubuntu|pop|linuxmint|debian)
      version_id="${VERSION_ID%%.*}"
      if [ "${version_id:-0}" -ge 22 ] 2>/dev/null; then
        echo 2
      else
        echo 1
      fi
      exit 0
      ;;
  esac
fi

echo 1
