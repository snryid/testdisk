/**
 * Tauri 1.x / 2.x compatibility layer.
 * Detects the injected runtime and routes invoke/dialog calls accordingly.
 */

function isTauriV2() {
  if (typeof window === "undefined" || !window.__TAURI__) {
    return true;
  }
  return window.__TAURI__.core != null;
}

export async function invoke(cmd, args) {
  if (isTauriV2()) {
    const { invoke: invokeV2 } = await import("@tauri-apps/api/core");
    return invokeV2(cmd, args);
  }
  return window.__TAURI__.tauri.invoke(cmd, args);
}

export async function openDialog(options) {
  if (isTauriV2()) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    return open(options);
  }
  return window.__TAURI__.dialog.open(options);
}

export async function saveDialog(options) {
  if (isTauriV2()) {
    const { save } = await import("@tauri-apps/plugin-dialog");
    return save(options);
  }
  return window.__TAURI__.dialog.save(options);
}

export function tauriMajorVersion() {
  return isTauriV2() ? 2 : 1;
}
