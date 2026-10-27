import type { GeneratedGraph } from "@/lib/bindings";
import type { NodeDescriptor } from "@/lib/types";
import { connectionError } from "./graphValidation";
import type { SolveOptions, SolvePlanning } from "./agentSolve";

export interface ReviewedPlan {
  graph: GeneratedGraph;
  replace: boolean;
  revision: number;
  solve?: SolveOptions;
  solvePlanning?: SolvePlanning;
}

/** Prepare the exact values shown for review and later used for execution. */
export function preparePlan(generated: GeneratedGraph, descriptors: Record<string, NodeDescriptor>, data = ""): GeneratedGraph {
  const plan = structuredClone(generated);
  if (!plan.nodes.length || plan.nodes.length > 80) throw new Error("计划需要包含 1–80 个节点");
  const keys = new Set<string>();
  for (const node of plan.nodes) {
    const descriptor = descriptors[node.descriptorId];
    if (!descriptor) throw new Error(`未知节点：${node.descriptorId}`);
    if (!node.key.trim() || node.key.includes(".") || keys.has(node.key)) throw new Error(`重复或无效节点标识：${node.key}`);
    if (node.params != null && (typeof node.params !== "object" || Array.isArray(node.params)))
      throw new Error(`${node.key} 的参数必须是对象`);
    keys.add(node.key);
    const defaults = Object.fromEntries(descriptor.params.map((p) => [p.name, p.default]));
    for (const [key, value] of Object.entries(node.params ?? {})) {
      const spec = descriptor.params.find((p) => p.name === key);
      if (!spec) throw new Error(`${descriptor.displayName} 没有参数 ${key}`);
      const widget = spec.widget;
      if (widget.kind === "select" && !widget.options.includes(String(value)))
        throw new Error(`${descriptor.displayName} 的 ${spec.label} 选项无效`);
      if ((widget.kind === "number" || widget.kind === "slider") &&
        (typeof value !== "number" || !Number.isFinite(value) || value < widget.min || value > widget.max))
        throw new Error(`${descriptor.displayName} 的 ${spec.label} 数值无效`);
      if (widget.kind === "toggle" && typeof value !== "boolean") throw new Error(`${spec.label} 应为布尔值`);
      if (["text", "select", "file", "image"].includes(widget.kind) && typeof value !== "string")
        throw new Error(`${spec.label} 应为文本`);
    }
    node.params = { ...defaults, ...node.params };
    if (!Number.isFinite(node.position?.x) || !Number.isFinite(node.position?.y))
      throw new Error(`${descriptor.displayName} 的坐标无效`);
  }
  if (data.length) {
    const source = plan.nodes.find((n) => n.descriptorId === "text_input");
    if (!source) throw new Error("计划缺少文本输入节点，无法放入待处理数据，请重新规划");
    source.params.text = data;
  }
  const nodes = plan.nodes.map((n) => ({ id: n.key, data: { descriptorId: n.descriptorId } }));
  const edges: import("@xyflow/react").Edge[] = [];
  for (const edge of plan.edges) {
    const connection = { source: edge.from.node, sourceHandle: edge.from.port, target: edge.to.node, targetHandle: edge.to.port };
    if (edges.some((e) => e.target === connection.target && e.targetHandle === connection.targetHandle))
      throw new Error(`输入 ${edge.to.node}.${edge.to.port} 存在多条连线`);
    const error = connectionError(connection, nodes, edges, descriptors);
    if (error) throw new Error(`${edge.from.node} → ${edge.to.node}：${error}`);
    edges.push({ id: String(edges.length), ...connection });
  }
  for (const node of plan.nodes) {
    for (const input of descriptors[node.descriptorId].inputs) {
      if (input.required && !edges.some((e) => e.target === node.key && e.targetHandle === input.name))
        throw new Error(`${node.key} 的必需输入「${input.label}」未连接`);
    }
  }
  if (plan.checks && (!Array.isArray(plan.checks) || plan.checks.length > 20))
    throw new Error("测试条件最多 20 项");
  for (const check of plan.checks ?? []) {
    const node = plan.nodes.find((n) => n.key === check.node);
    const descriptor = node && descriptors[node.descriptorId];
    const port = descriptor?.outputs.find((p) => p.name === check.port);
    if (!(descriptor?.id === "text_output" && check.port === "value") &&
      (!port || !["text", "any"].includes(port.type))) throw new Error(`测试目标无效：${check.node}.${check.port}`);
    if (!["equals", "contains", "nonEmpty"].includes(check.kind) ||
      (check.kind !== "nonEmpty" && (typeof check.expected !== "string" || check.expected.length > 4096)))
      throw new Error("测试条件无效");
  }
  return plan;
}

/** A user-provided exact expectation is attached to a visible text result. */
export function withExpectedText(plan: GeneratedGraph, expected: string, descriptors: Record<string, NodeDescriptor>): GeneratedGraph {
  if (!expected.length) return plan;
  const sink = [...plan.nodes].reverse().find((n) => n.descriptorId === "text_output");
  const node = sink ?? [...plan.nodes].reverse().find((n) => descriptors[n.descriptorId]?.outputs.some((p) => p.type === "text"));
  if (!node) throw new Error("计划没有文本输出，无法设置预期文本");
  const port = sink ? "value" : descriptors[node.descriptorId].outputs.find((p) => p.type === "text")!.name;
  return preparePlan({ ...plan, checks: [...(plan.checks ?? []).filter((c) => c.node !== node.key || c.port !== port),
    { node: node.key, port, kind: "equals", expected }] }, descriptors);
}
