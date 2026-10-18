import { invoke } from "@tauri-apps/api/core";
import { useSyncExternalStore } from "react";

export type ToastVariant = "success" | "failure" | "warning" | "pending";

export interface Toast {
  variant: ToastVariant;
  title: string;
  message?: string;
  icon?: string;
}

let current: Toast | null = null;
let launcherVisible = false;
const listeners = new Set<() => void>();

function set(toast: Toast | null) {
  current = toast;
  listeners.forEach((listener) => listener());
}

export function showHUD(toast: Toast) {
  invoke("show_hud", { toast }).catch((e) => console.error("show_hud failed", e));
}

export function showToast(toast: Toast) {
  if (launcherVisible) set(toast);
  else showHUD(toast);
}

export function hideToast() {
  set(null);
}

export function setLauncherVisible(visible: boolean) {
  launcherVisible = visible;
  if (!visible) set(null);
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function useToast(): Toast | null {
  return useSyncExternalStore(subscribe, () => current);
}
