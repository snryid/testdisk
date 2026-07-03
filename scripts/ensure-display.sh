#!/usr/bin/env bash
# Ensure a GUI display is available before `tauri dev` on Linux.
# When sourced, may export DISPLAY or set RUN_UNDER_XVFB=1.
# Exit 1 with remediation hints when display is required but unavailable.
set -euo pipefail

ensure_display() {
  local mode="${1:-dev}"

  if [ "$mode" != "dev" ]; then
    return 0
  fi

  if [ "$(uname -s 2>/dev/null || echo Unknown)" != "Linux" ]; then
    return 0
  fi

  if [ -n "${SKIP_DISPLAY_CHECK:-}" ]; then
    return 0
  fi

  if display_usable; then
    return 0
  fi

  # Local desktop: attach to the first available X11 socket.
  local d num
  for d in :0 :1 :2; do
    num="${d#:}"
    if [ -S "/tmp/.X11-unix/X${num}" ]; then
      export DISPLAY="$d"
      if display_usable; then
        echo "Auto-set DISPLAY=$DISPLAY"
        return 0
      fi
      unset DISPLAY
    fi
  done

  # systemd user graphical session (GDM on local console).
  if command -v loginctl >/dev/null 2>&1; then
    local disp
    disp="$(loginctl show-user "$(id -u)" -p Display --value 2>/dev/null || true)"
    if [[ "$disp" =~ ^: ]]; then
      export DISPLAY="$disp"
      if display_usable; then
        echo "Auto-set DISPLAY=$DISPLAY (from loginctl)"
        return 0
      fi
      unset DISPLAY
    fi
  fi

  if [ -n "${USE_XVFB:-}" ]; then
    if command -v xvfb-run >/dev/null 2>&1; then
      export RUN_UNDER_XVFB=1
      echo "No display — using xvfb-run (virtual framebuffer, window not visible on SSH)"
      return 0
    fi
    echo "Error: USE_XVFB=1 but xvfb-run not found. Install: sudo apt install -y xvfb" >&2
    return 1
  fi

  print_display_help >&2
  return 1
}

display_usable() {
  if [ -n "${WAYLAND_DISPLAY:-}" ]; then
    return 0
  fi
  if [ -z "${DISPLAY:-}" ]; then
    return 1
  fi
  if command -v xdpyinfo >/dev/null 2>&1; then
    xdpyinfo >/dev/null 2>&1
    return $?
  fi
  if command -v xset >/dev/null 2>&1; then
    xset q >/dev/null 2>&1
    return $?
  fi
  # DISPLAY is set (e.g. ssh -X → localhost:10.0); trust it when probe tools are absent.
  return 0
}

print_display_help() {
  cat <<'EOF'
Error: 无法启动 GUI — 未检测到可用的图形显示环境。

常见原因：通过 SSH 登录且未启用 X11 转发，或服务器未运行桌面环境。

解决方法（任选其一）：

  1. 使用 X11 转发重新 SSH 登录（客户端需 X 服务器：Windows 用 VcXsrv/MobaXterm，macOS 用 XQuartz）：
     ssh -X user@this-host
     make dev

  2. 在本机桌面环境的终端中直接运行：
     make dev

  3. 无界面冒烟测试（仅验证能否启动，窗口不可见）：
     sudo apt install -y xvfb
     make dev-xvfb

  4. 跳过检查（不推荐，仍会启动失败）：
     SKIP_DISPLAY_CHECK=1 make dev
EOF
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  ensure_display "${1:-dev}"
fi
