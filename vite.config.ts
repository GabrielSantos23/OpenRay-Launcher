/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { readdirSync } from "node:fs";
import type { Plugin } from "vite";

function lucideNames(): Plugin {
  const id = "virtual:lucide-names";
  return {
    name: "lucide-names",
    resolveId: (source) => (source === id ? `\0${id}` : null),
    load: (source) =>
      source === `\0${id}`
        ? `export default ${JSON.stringify(readdirSync("node_modules/lucide-react/dist/esm/icons").filter((f) => f.endsWith(".js")).map((f) => f.slice(0, -3)))};`
        : null,
  };
}

export default defineConfig({
  plugins: [react(), tailwindcss(), lucideNames()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: "chrome120" },
  worker: { format: "es" },
  optimizeDeps: { include: ["@flowkey-cli/react-ui", "@flowkey-cli/native-sdk", "marked"] },
  test: { environment: "node" },
});
