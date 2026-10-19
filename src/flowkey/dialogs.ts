import { useSyncExternalStore } from "react";

export interface DialogRequest {
  title: string;
  message?: string;
  items?: string[];
  confirmTitle: string;
  cancelTitle: string;
  destructive: boolean;
  resolve: (ok: boolean) => void;
}

const queue: DialogRequest[] = [];
const listeners = new Set<() => void>();
let head: DialogRequest | null = null;

function notify() {
  head = queue[0] ?? null;
  listeners.forEach((listener) => listener());
}

export function ask(request: Omit<DialogRequest, "resolve">, signal?: AbortSignal): Promise<boolean> {
  return new Promise((resolve) => {
    const entry: DialogRequest = {
      ...request,
      resolve: (ok) => {
        const index = queue.indexOf(entry);
        if (index < 0) return;
        queue.splice(index, 1);
        notify();
        resolve(ok);
      },
    };
    signal?.addEventListener("abort", () => entry.resolve(false), { once: true });
    queue.push(entry);
    notify();
  });
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function useDialog(): DialogRequest | null {
  return useSyncExternalStore(subscribe, () => head);
}
