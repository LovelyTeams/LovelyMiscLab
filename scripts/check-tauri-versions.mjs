import { readFileSync } from "node:fs";
import { resolve } from "node:path";

// Cargo tests do not check the JS/native pairing required by `tauri build`.
// Run this before the frontend build so local checks catch the same mismatch.
const manifest = JSON.parse(readFileSync("package.json", "utf8"));
const lockfile = readFileSync("Cargo.lock", "utf8");
const locked = new Map([...lockfile.matchAll(/\[\[package\]\]\s+name = "([^"]+)"\s+version = "([^"]+)"/g)]
  .map((match) => [match[1], match[2]]));
const minor = (version) => version.match(/^\d+\.\d+/)?.[0];
const errors = [];
let checked = 0;
for (const name of Object.keys(manifest.dependencies ?? {})) {
  if (name !== "@tauri-apps/api" && !name.startsWith("@tauri-apps/plugin-")) continue;
  const crate = name === "@tauri-apps/api" ? "tauri" : name.replace("@tauri-apps/", "tauri-");
  const native = locked.get(crate);
  const installed = JSON.parse(readFileSync(resolve("node_modules", name, "package.json"), "utf8")).version;
  if (!native || !minor(installed) || minor(native) !== minor(installed)) {
    errors.push(`${crate} (${native ?? "missing from Cargo.lock"}) / ${name} (${installed})`);
  }
  checked++;
}
if (errors.length) {
  console.error(`Tauri JS/native packages must use the same major/minor versions:\n${errors.join("\n")}`);
  process.exit(1);
}
console.log(`Verified ${checked} Tauri JS/native package pairs.`);
