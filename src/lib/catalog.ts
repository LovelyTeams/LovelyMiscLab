import categories from "./categories.generated.json";
import type { NodeCategory, NodeDescriptor, ParamSpec } from "./types";

/** Generated from Rust. Desktop may replace this with the live catalog. */
export const nodeCategories: NodeCategory[] = categories;
export function categoryLabel(d: NodeDescriptor, catalog = nodeCategories) {
  const child = catalog.find((c) => c.id === d.subcategoryId);
  return child ? `${d.category} / ${child.name}` : d.category;
}
export function searchFields(d: NodeDescriptor): string[] {
  return [d.id, d.displayName, d.category, d.categoryId ?? "", d.subcategoryId ?? "", d.family ?? "", d.description ?? "",
    ...(d.aliases ?? []), ...(d.upstreamIds ?? []), ...(d.legacyCategories ?? []),
    ...nodeCategories.filter((c) => c.id === d.subcategoryId).map((c) => c.name),
    ...d.params.flatMap((p) => [p.name, p.label, p.description ?? "", ...(p.widget.kind === "select" ? p.widget.options : [])])];
}
const synonyms = [
  ["decode", "decoder", "decoding", "解码"], ["encode", "encoder", "encoding", "编码"],
  ["decrypt", "decryption", "解密"], ["encrypt", "encryption", "加密"],
  ["regex", "regexp", "regular expression", "正则表达式", "正则"],
  ["extract", "extraction", "提取", "抽取"], ["replace", "replacement", "替换"],
  ["decompress", "decompression", "unzip", "unpack", "解压缩", "解压"],
  ["qrcode", "qr code", "二维码"], ["lsb", "最低有效位", "最低位"],
  ["stego", "steganography", "隐写"],
  ["hex", "hexadecimal", "十六进制", "十六进制编码"], ["hash", "digest", "哈希", "散列", "摘要"],
];
const normalize = (s: string) => s.normalize("NFKC").toLowerCase().replace(/[\s\p{P}\p{S}_]+/gu, "");

// Algorithm tokens can start a name or a separated word, but AES must not
// match the middle of Caesar. Keep prefix searches such as SHA -> SHA256.
function containsTerm(field: string, term: string): boolean {
  for (let at = field.indexOf(term); at >= 0; at = field.indexOf(term, at + 1)) {
    if (!/^[a-z0-9]/.test(term) || at === 0 || !/[a-z0-9]/.test(field[at - 1])) return true;
  }
  return false;
}

/** Shared by the module UI and Agent: split mixed-language compound searches
 * and expand common terminology without dropping algorithm/mode constraints. */
export function searchTerms(query: string): string[][] {
  let q = query.normalize("NFKC").toLowerCase().replace(/(?:模块|节点|工具|功能|\bmodule|\bnode|\btool)\s*$/i, "");
  const phrases = synonyms.flatMap((group) => group.filter((word) => /[\u3400-\u9fff ]/.test(word)).map((word) => ({ word, canonical: group[0] })))
    .sort((a, b) => b.word.length - a.word.length);
  for (const { word, canonical } of phrases) q = q.split(word).join(` ${canonical} `);
  const tokens = [...new Set(q.match(/[a-z0-9]+|[\u3400-\u9fff]+/g) ?? [])].filter((token) => token.length > 1 && !["module", "node", "tool"].includes(token));
  return tokens.map((token) => synonyms.find((group) => group.includes(token)) ?? [token]);
}
export function searchRank(d: NodeDescriptor, query: string): number {
  const q = query.trim().toLowerCase();
  if (!q) return 3;
  const fields = searchFields(d).map((s) => s.normalize("NFKC").toLowerCase());
  const direct = Math.min(...fields.map((s) => {
    s = s.toLowerCase();
    return s === q ? 0 : s.startsWith(q) ? 1 : containsTerm(s, q) ? 2 : Infinity;
  }));
  if (Number.isFinite(direct)) return direct;
  const compact = normalize(q.replace(/(?:模块|节点|工具|功能)$/, "")), normalized = fields.map(normalize);
  if (!compact) return Infinity;
  const tolerant = Math.min(...normalized.map((s) => s === compact ? 3 : s.startsWith(compact) ? 4 : containsTerm(s, compact) ? 5 : Infinity));
  if (Number.isFinite(tolerant)) return tolerant;
  const terms = searchTerms(q);
  return terms.length && terms.every((group) => group.some((word) => fields.some((field) => containsTerm(field, word)))) ? 6 : Infinity;
}
export function searchNodes(list: NodeDescriptor[], query: string): NodeDescriptor[] {
  const ranked = list.map((d, index) => ({ d, index, rank: searchRank(d, query) })).filter((r) => Number.isFinite(r.rank));
  // Expand only when literal/normalized search misses, so an exact decode ID
  // does not get diluted with encoders that mention decoding in parameter help.
  const tier = Math.min(...ranked.map((r) => Math.floor(r.rank / 3)));
  return ranked.filter((r) => Math.floor(r.rank / 3) === tier)
    .sort((a, b) => a.rank - b.rank || a.index - b.index).map((r) => r.d);
}
export function matchesCategory(d: NodeDescriptor, category: string): boolean {
  return !category || [d.category, d.categoryId, d.subcategoryId, ...(d.legacyCategories ?? [])].includes(category)
    || nodeCategories.some((c) => c.id === d.subcategoryId && c.name === category);
}
export function inlineParams(d: NodeDescriptor): ParamSpec[] {
  if (!d.inputs.length) return d.params;
  const names = d.presentation?.inlineParams;
  return d.params.filter((p) => names ? names.includes(p.name) :
    p.widget.kind !== "file" && p.widget.kind !== "image" && (p.widget.kind !== "text" || !p.widget.multiline && String(p.default ?? "").length <= 80 && !/key|iv|pem|alphabet|password|secret/i.test(p.name))).slice(0, 3);
}
