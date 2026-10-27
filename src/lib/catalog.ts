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
    ...nodeCategories.filter((c) => c.id === d.subcategoryId).map((c) => c.name)];
}
export function searchRank(d: NodeDescriptor, query: string): number {
  const q = query.trim().toLowerCase();
  if (!q) return 3;
  return Math.min(...searchFields(d).map((s) => {
    s = s.toLowerCase();
    return s === q ? 0 : s.startsWith(q) ? 1 : s.includes(q) ? 2 : Infinity;
  }));
}
export function searchNodes(list: NodeDescriptor[], query: string): NodeDescriptor[] {
  return list.map((d, index) => ({ d, index, rank: searchRank(d, query) })).filter((r) => Number.isFinite(r.rank))
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
