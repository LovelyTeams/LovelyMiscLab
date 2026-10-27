// Exercise the real stores and runners with a fake Tauri transport, without
// requiring a WebView, network model or additional test-runtime dependency.
import { createRequire } from "node:module";
import { mkdir } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { resolve } from "node:path";

const require = createRequire(import.meta.url);
const viteRequire = createRequire(require.resolve("vite/package.json"));
const { build } = viteRequire("esbuild");
const outfile = resolve("node_modules/.cache/misclab/frontend-tests.mjs");
await mkdir(resolve(outfile, ".."), { recursive: true });
await build({
  entryPoints: ["tests/frontend/index.ts"], outfile,
  bundle: true, platform: "node", format: "esm", packages: "external",
  tsconfig: "tsconfig.json",
  plugins: [{ name: "desktop-transport", setup(builder) {
    builder.onResolve({ filter: /(?:@\/lib\/devMocks|\.\/devMocks)$/ }, () => ({ path: "desktop", namespace: "test" }));
    builder.onLoad({ filter: /.*/, namespace: "test" }, () => ({ contents: "export const inTauri = true;", loader: "js" }));
  } }],
});
const result = spawnSync(process.execPath, ["--test", outfile], { stdio: "inherit" });
process.exitCode = result.status ?? 1;
