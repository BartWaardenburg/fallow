import { defineConfig } from "rolldown";

const watch = process.argv.includes("--watch");

export default defineConfig({
  input: "src/extension.ts",
  external: ["vscode"],
  platform: "node",
  output: {
    file: "dist/extension.js",
    format: "cjs",
    sourcemap: true,
    minify: !watch,
  },
});
