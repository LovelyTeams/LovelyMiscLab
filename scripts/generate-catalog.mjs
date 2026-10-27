import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
const result = spawnSync("cargo", ["run", "--quiet", "-p", "misclab-core", "--example", "export_descriptors"], { encoding: "utf8", maxBuffer: 8 * 1024 * 1024 });
if (result.status !== 0) { console.error(result.stderr); process.exit(result.status || 1); }
const catalog = JSON.parse(result.stdout);
const check = process.argv.includes("--check");
for (const [name, data] of [["nodeCatalog", catalog.nodes], ["categories", catalog.categories]]) {
  const path = new URL(`../src/lib/${name}.generated.json`, import.meta.url);
  const content = JSON.stringify(data, null, 2) + "\n";
  // Git may check out generated JSON with CRLF on Windows. Compare canonical
  // line endings so the catalog check measures registry changes on every OS.
  if (check) { if (readFileSync(path, "utf8").replace(/\r\n/g, "\n") !== content) throw Error(`${name} differs from Rust; run pnpm generate:catalog`); }
  else writeFileSync(path, content);
}
console.log(`${check ? "Verified" : "Exported"} ${catalog.nodes.length} Rust node descriptors and ${catalog.categories.length} categories.`);
