import { searchNodes, searchTerms } from "@/lib/catalog";
import type { NodeDescriptor } from "@/lib/types";
import type { SolvePlanning } from "./agentSolve";

export function findSolverModules(list: NodeDescriptor[], query: string): SolvePlanning["searches"][number] {
  const brief = (d: NodeDescriptor) => ({ id: d.id, name: d.displayName });
  const matches = searchNodes(list, query).slice(0, 8).map(brief);
  if (matches.length) return { query, matches };
  // Partial matches are suggestions, never evidence that the complete requested
  // capability exists. The model must select a real ID on its next decision.
  const terms = searchTerms(query).filter((group) => group.length === 1 && group[0].length >= 3);
  const suggestions = list.map((d, index) => {
    const names = [d.id, d.displayName, d.family ?? "", ...(d.aliases ?? [])].map((s) => s.normalize("NFKC").toLowerCase());
    return { d, index, score: terms.filter(([term]) => names.some((name) => name.includes(term))).length };
  }).filter((entry) => entry.score > 0).sort((a, b) => b.score - a.score || a.index - b.index).slice(0, 8).map(({ d }) => brief(d));
  return { query, matches, ...(suggestions.length ? { suggestions } : {}) };
}

/** Names and IDs only, without task inputs or parameter values. The decision
 * model can browse all allowed capabilities instead of guessing module names. */
export function solverModuleIndex(list: NodeDescriptor[]): string {
  const groups = new Map<string, string[]>();
  for (const d of list) {
    const group = groups.get(d.category) ?? [];
    group.push(`${d.id}=${d.displayName}`); groups.set(d.category, group);
  }
  return [...groups].map(([category, entries]) => `${category}: ${entries.join("；")}`).join("\n");
}
