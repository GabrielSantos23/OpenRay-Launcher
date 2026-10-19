/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: "chrome120" },
  worker: { format: "es" },
  optimizeDeps: { include: ["@flowkey-cli/react-ui", "@flowkey-cli/native-sdk", "marked"] },
  test: { environment: "node" },
});
