import type { Connection, Edge } from "@xyflow/react";
import type { NodeDescriptor } from "@/lib/types";
import { canConnect, paramPortType } from "./portColors";

type NodeRef = { id: string; data: { descriptorId: string } };

/** Shared by every connection entry point, including AI and the canvas. */
export function connectionError(
  connection: Connection,
  nodes: NodeRef[],
  edges: Edge[],
  descriptors: Record<string, NodeDescriptor>
): string | null {
  const { source, target, sourceHandle, targetHandle } = connection;
  if (!sourceHandle || !targetHandle) return "请选择有效端口";
  if (source === target) return "节点不能连接自身";
  const from = descriptors[nodes.find((n) => n.id === source)?.data.descriptorId ?? ""];
  const to = descriptors[nodes.find((n) => n.id === target)?.data.descriptorId ?? ""];
  const output = from?.outputs.find((p) => p.name === sourceHandle)?.type;
  const parameter = to?.params.find((p) => p.name === targetHandle);
  const input = to?.inputs.find((p) => p.name === targetHandle)?.type ??
    (parameter && paramPortType(parameter.widget));
  if (!output || !input) return "连线引用了不存在的节点或端口";
  if (!canConnect(output, input)) return "端口类型不兼容";

  // Reconnecting an occupied input replaces its old edge. Exclude that edge
  // when checking whether the proposed connection closes a cycle.
  const remaining = edges.filter((e) => !(e.target === target && e.targetHandle === targetHandle));
  const seen = new Set<string>();
  const pending = [target];
  while (pending.length) {
    const id = pending.pop()!;
    if (id === source) return "连线会形成循环";
    if (seen.has(id)) continue;
    seen.add(id);
    for (const edge of remaining) if (edge.source === id) pending.push(edge.target);
  }
  return null;
}
