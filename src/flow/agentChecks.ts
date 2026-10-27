import type { GeneratedGraph } from "@/lib/bindings";
import type { PortValue } from "@/lib/types";
import type { AgentStep } from "@/store/agent";

export function checkAgentResults(plan: GeneratedGraph, results: Record<string, {
  status: string; outputs?: Record<string, PortValue>; error?: string;
}>): AgentStep[] {
  const steps: AgentStep[] = plan.nodes.map((node) => {
    const result = results[node.key];
    const ok = result?.status === "done" && result.outputs !== undefined;
    return { kind: "result", ok, text: `${node.key}：${ok ? "执行通过" : "未完成"}`,
      detail: ok ? `${Object.keys(result.outputs!).length} 项输出已返回` : result?.error || "未收到完整执行结果" };
  });
  for (const check of plan.checks ?? []) {
    const value = results[check.node]?.outputs?.[check.port];
    const text = value?.type === "text" ? value.value : undefined;
    const ok = text !== undefined && (check.kind === "equals" ? text === check.expected :
      check.kind === "contains" ? text.includes(check.expected!) : text.length > 0);
    const label = check.kind === "equals" ? "精确匹配" : check.kind === "contains" ? "包含检查" : "非空检查";
    steps.push({ kind: "result", ok, text: `${check.node}.${check.port} ${label}：${ok ? "通过" : "未通过"}`,
      detail: text === undefined ? "未返回文本结果" : `实际：${JSON.stringify(text.slice(0, 160))}${text.length > 160 ? "…" : ""}` });
  }
  return steps;
}
