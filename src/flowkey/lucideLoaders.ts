import type { LucideIcon } from "lucide-react";

const files = import.meta.glob<{ default: LucideIcon }>("/node_modules/lucide-react/dist/esm/icons/*.js");

export function loadIcon(key: string) {
  const load = files[`/node_modules/lucide-react/dist/esm/icons/${key}.js`];
  if (!load) throw new Error(`Unknown icon ${key}`);
  return load();
}
