import { build } from "esbuild";
import { fileURLToPath } from "node:url";

const here = (path) => fileURLToPath(new URL(path, import.meta.url));

await build({
  entryPoints: [here("./host.js")],
  outfile: here("../assets/flowkey-host.js"),
  bundle: true,
  format: "esm",
  platform: "neutral",
  target: "es2023",
  mainFields: ["module", "main"],
  conditions: ["production"],
  define: { "process.env.NODE_ENV": '"production"' },
  tsconfigRaw: "{}",
  minify: true,
  legalComments: "eof",
  logLevel: "info",
});
