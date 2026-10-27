import type { GeneratedGraph } from "@/lib/bindings";
import type { NodeDescriptor } from "@/lib/types";
import type { SolveAttempt, SolvePlanning, SolveResults } from "./agentSolve";

const short = (value: unknown) => {
  const text = JSON.stringify(value) ?? "未设置";
  return text.length > 100 ? `${text.slice(0, 100)}…` : text;
};

/** Describe actual graph changes rather than trusting model claims of a fix. */
export function solveChanges(previous: GeneratedGraph | undefined, next: GeneratedGraph, descriptors: Record<string, NodeDescriptor>): string[] {
  if (!previous) return [];
  const changes: string[] = [];
  for (const node of next.nodes) {
    const d = descriptors[node.descriptorId], old = previous.nodes.find((n) => n.key === node.key);
    const label = `${d.displayName} (${node.key})`;
    if (!old) { changes.push(`新增 ${label}`); continue; }
    if (old.descriptorId !== node.descriptorId) { changes.push(`替换 ${node.key}：${descriptors[old.descriptorId].displayName} → ${d.displayName}`); continue; }
    for (const p of d.params) if (JSON.stringify(old.params[p.name]) !== JSON.stringify(node.params[p.name]))
      changes.push(`修改 ${label}.${p.label}：${short(old.params[p.name])} → ${short(node.params[p.name])}`);
  }
  for (const node of previous.nodes) if (!next.nodes.some((n) => n.key === node.key))
    changes.push(`移除 ${descriptors[node.descriptorId].displayName} (${node.key})`);
  const edgeKey = (e: GeneratedGraph["edges"][number]) => `${e.from.node}.${e.from.port} → ${e.to.node}.${e.to.port}`;
  const before = new Set(previous.edges.map(edgeKey)), after = new Set(next.edges.map(edgeKey));
  for (const e of after) if (!before.has(e)) changes.push(`连接 ${e}`);
  for (const e of before) if (!after.has(e)) changes.push(`断开 ${e}`);
  return changes;
}

export function solveRepairChecks(planning: SolvePlanning | undefined, plan: GeneratedGraph, results: SolveResults): NonNullable<SolveAttempt["repairChecks"]> {
  return (planning?.decision.repairs ?? []).map((repair) => {
    if (!plan.nodes.some((n) => n.key === repair.node)) return { node: repair.node, status: "removed", detail: "原节点已移除；不能据此判定修复通过，需检查替代分支和目标结果。" };
    const result = results[repair.node];
    if (result?.status !== "done" || !result.outputs) return { node: repair.node, status: "failed", detail: result?.error || "未返回完整执行结果" };
    const checks = (plan.checks ?? []).filter((c) => c.node === repair.node);
    if (!checks.length) return { node: repair.node, status: "executed", detail: "节点已重新执行；没有针对该节点的结果断言，尚不能确认修复目标。" };
    const passed = checks.every((c) => {
      const value = result.outputs?.[c.port];
      return value?.type === "text" && (c.kind === "nonEmpty" ? value.value.length > 0 : c.kind === "equals" ? value.value === c.expected : value.value.includes(c.expected!));
    });
    return { node: repair.node, status: passed ? "passed" : "failed", detail: `${checks.length} 项实际输出断言${passed ? "通过" : "未通过"}；解题目标另行核验。` };
  });
}
