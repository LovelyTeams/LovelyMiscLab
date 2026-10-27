// Static inventory only: never execute upstream algorithms or bundle its JS.
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";

const upstream = resolve(process.argv[2] ?? ".git/reference/EBCTFCodeBox");
const source = await readFile(resolve(upstream, "src/core/lazyTables.generated.js"), "utf8");
const table = (name) => JSON.parse(source.match(new RegExp(`export const ${name} = (.*);`))[1]);
const stubs = table("CORE_OP_STUBS"), files = table("OP_FILE_MAP");
const revision = execFileSync("git", ["-C", upstream, "rev-parse", "HEAD"], { encoding: "utf8" }).trim();
const catalog = JSON.parse(await readFile("src/lib/nodeCatalog.generated.json", "utf8"));
const normalize = (s) => s.toLowerCase().replace(/[^a-z0-9]/g, "");
const stem = (s) => normalize(s.replace(/_(encode|decode|encrypt|decrypt)$/, ""));
// Human-reviewed source of truth. This script NEVER writes the acceptance file.
const acceptance = JSON.parse(await readFile("docs/ebctf-acceptance.json", "utf8"));
if (acceptance.revision !== revision || revision !== "ea2bc36dae31c97060f7f2cddad8df5518afad7b") throw new Error("Upstream revision differs from the accepted baseline");
const records = acceptance.operations;
const aliases = { base16: "hex", urlEncode: "url", railFence: "railfence", quotedPrintable: "quoted_printable", caesarCipher: "caesar" };
const operations = stubs.map((op) => {
  const sourceFile = `src/core/${files[op.id]?.replace(/^\.\//, "")}`;
  const record = records[op.id];
  const targetNodes = record?.targetNodes ?? catalog.filter((d) => stem(d.id) === normalize(aliases[op.id] ?? op.id)).map((d) => d.id);
  return {
    ...op, sourceFile,
    sourceUrl: `https://github.com/Henglie/EBCTFCodeBox/blob/${revision}/${sourceFile}`,
    status: targetNodes.length ? "existing-review" : "pending",
    targetNodes, batch: null, validation: [],
    note: targetNodes.length ? "名称匹配候选；参数、变体与行为尚未逐项验收，不能视为等价完成。" : "待用 Rust 实现或核验现有节点组合。",
    ...record,
    categories: [...new Set(targetNodes.map(id => catalog.find(d => d.id === id)?.subcategoryId).filter(Boolean))],
  };
});
if (stubs.length !== 820 || new Set(stubs.map((s) => s.id)).size !== 820) throw new Error("Upstream inventory changed: review counts and mappings");
for (const [id, record] of Object.entries(records)) {
  if (!stubs.some((s) => s.id === id)) throw new Error(`Missing upstream operation ${id}`);
  if (!["migrated", "partial", "blocked", "existing-review", "pending"].includes(record.status)) throw new Error(`Invalid status ${id}`);
  for (const target of record.targetNodes ?? []) if (!catalog.some((d) => d.id === target)) throw new Error(`Missing Rust node ${target}`);
  if (record.status === "migrated" && (!record.targetNodes?.length || !record.validation?.length || !record.directions?.length || !record.parameterMapping)) throw new Error(`Incomplete acceptance evidence ${id}`);
  for (const test of record.validation ?? []) await readFile(test.split('#')[0]);
  if (record.status === "migrated") {
    if (!record.nodeMappings?.length || record.targetNodes.some(id => !record.nodeMappings.some(m => m.nodeId === id))) throw Error(`Missing explicit node mapping for ${id}`);
    for (const mapping of record.nodeMappings) {
      const d = catalog.find(d => d.id === mapping.nodeId);
      if (!d || !record.targetNodes.includes(mapping.nodeId) || !record.directions.includes(mapping.direction)) throw Error(`Invalid node/direction mapping for ${id}`);
      if (mapping.categoryId !== d.categoryId || mapping.subcategoryId !== d.subcategoryId) throw Error(`Category acceptance stale for ${id}/${d.id}`);
      if (JSON.stringify(mapping.params) !== JSON.stringify(d.params.map(p => ({name:p.name,default:p.default})))) throw Error(`Parameter acceptance stale for ${id}/${d.id}`);
    }
  }
}
for (const batch of new Set(Object.values(records).map(r => r.batch).filter(Number.isInteger))) {
  if (Object.values(records).filter(r => r.batch === batch).length > 20) throw Error(`Batch ${batch} exceeds 20 operations`);
}
if (operations.some((op) => op.sourceFile.endsWith("undefined"))) throw new Error("Incomplete source mapping");
const counts = Object.fromEntries(["migrated", "partial", "blocked", "existing-review", "pending"].map((status) => [status, operations.filter((o) => o.status === status).length]));
const inventory = { source: "https://github.com/Henglie/EBCTFCodeBox", revision, license: "Apache-2.0", total: stubs.length, counts, operations };
await mkdir("docs", { recursive: true });
await writeFile("docs/ebctf-migration.json", JSON.stringify(inventory, null, 2) + "\n");
const rows = operations.map((o) => `| \`${o.id}\` | ${o.name.replaceAll("|", "\\|")} | ${o.cat} | ${o.status} | ${o.targetNodes.map((s) => `\`${s}\``).join(", ") || "—"} | [源码](${o.sourceUrl}) |`);
await writeFile("docs/ebctf-operations.md", `# EBCTFCodeBox 完整功能清单\n\n由 \`pnpm generate:migration\` 生成。来源提交：\`${revision}\`。人工验收源：[ebctf-acceptance.json](ebctf-acceptance.json)，生成器不会覆盖它。\n\n总计 ${stubs.length} 项：已迁入 ${counts.migrated}，部分完成 ${counts.partial}，待解决 ${counts.blocked}，现有候选待核验 ${counts["existing-review"]}，待迁入 ${counts.pending}。\n\n\`existing-review\` 仅表示名称匹配，不表示算法、参数或文件处理能力等价。验收范围和批次见 [迁移说明](ebctf-migration.md)；机器可读清单见 [JSON](ebctf-migration.json)。\n\n| 上游 ID | 功能 | 分类 | 状态 | Rust 节点 / 待核验候选 | 来源 |\n|---|---|---|---|---|---|\n${rows.join("\n")}\n`);
console.log(JSON.stringify({ revision, total: stubs.length, counts }, null, 2));
