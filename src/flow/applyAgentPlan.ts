import type { GeneratedGraph } from "@/lib/bindings";
import { useAgentStore } from "@/store/agent";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";

/** One reviewed graph, one undo step. Previously drawn attempts stay replayable. */
export function applyAgentPlan(plan: GeneratedGraph, replace: boolean): Map<string, string> {
  const g = useGraphStore.getState(), agent = useAgentStore.getState();
  const descriptors = useDescriptorStore.getState().byId;
  const offset = replace || !g.nodes.length ? 0 :
    Math.max(...g.nodes.map((n) => n.position.x + (n.measured?.width ?? 220))) + 100;
  const ids = new Map<string, string>();
  g.setSuppressHistory(true);
  try {
    if (replace) g.clear();
    for (const node of plan.nodes) {
      const descriptor = descriptors[node.descriptorId];
      const id = g.addNode(descriptor, { x: node.position.x + offset, y: node.position.y });
      ids.set(node.key, id);
      for (const [key, value] of Object.entries(node.params)) g.setParam(id, key, value);
      agent.pushStep({ kind: "add", text: descriptor.displayName, detail: node.key });
    }
    for (const edge of plan.edges) {
      const target = ids.get(edge.to.node)!;
      const node = useGraphStore.getState().nodes.find((n) => n.id === target)!;
      const descriptor = descriptors[node.data.descriptorId];
      if (!descriptor.inputs.some((p) => p.name === edge.to.port) && !node.data.inputParams?.includes(edge.to.port))
        g.toggleParamInput(target, edge.to.port);
      g.onConnect({ source: ids.get(edge.from.node)!, sourceHandle: edge.from.port, target, targetHandle: edge.to.port });
    }
    agent.pushStep({ kind: "connect", text: `已配置 ${plan.nodes.length} 个节点、连接 ${plan.edges.length} 条线` });
  } finally { g.setSuppressHistory(false); }
  g.deselectAll();
  agent.focusNodes([...ids.values()]);
  return ids;
}

/** Reconcile only this solver's graph, retaining canvas IDs for stable keys.
 * Each revision is one undo step; unrelated pre-existing canvas nodes survive. */
export function applyAgentRevision(plan: GeneratedGraph, previous: GeneratedGraph, oldIds: Map<string, string>): Map<string, string> {
  const g = useGraphStore.getState(), descriptors = useDescriptorStore.getState().byId;
  const owned = new Set(oldIds.values()), ids = new Map<string, string>();
  const origin = Math.min(...g.nodes.filter((n) => owned.has(n.id)).map((n) => n.position.x));
  for (const node of plan.nodes) {
    const old = previous.nodes.find((n) => n.key === node.key && n.descriptorId === node.descriptorId);
    if (old && oldIds.has(old.key)) ids.set(node.key, oldIds.get(old.key)!);
  }
  const retained = new Set(ids.values());
  g.setSuppressHistory(true);
  try {
    g.onEdgesChange(g.edges.filter((e) => owned.has(e.source) || owned.has(e.target)).map((e) => ({ id: e.id, type: "remove" })));
    g.onNodesChange([...owned].filter((id) => !retained.has(id)).map((id) => ({ id, type: "remove" })));
    for (const node of plan.nodes) {
      let id = ids.get(node.key);
      if (!id) { id = g.addNode(descriptors[node.descriptorId], { x: (Number.isFinite(origin) ? origin : 0) + node.position.x, y: node.position.y }); ids.set(node.key, id); }
      for (const [name, value] of Object.entries(node.params)) g.setParam(id, name, value);
      const current = useGraphStore.getState().nodes.find((n) => n.id === id)!;
      for (const param of current.data.inputParams ?? []) if (!plan.edges.some((e) => e.to.node === node.key && e.to.port === param)) g.toggleParamInput(id, param);
    }
    for (const edge of plan.edges) {
      const target = ids.get(edge.to.node)!;
      const node = useGraphStore.getState().nodes.find((n) => n.id === target)!;
      if (!descriptors[node.data.descriptorId].inputs.some((p) => p.name === edge.to.port) && !node.data.inputParams?.includes(edge.to.port)) g.toggleParamInput(target, edge.to.port);
      g.onConnect({ source: ids.get(edge.from.node)!, sourceHandle: edge.from.port, target, targetHandle: edge.to.port });
    }
  } finally { g.setSuppressHistory(false); }
  g.deselectAll();
  useAgentStore.getState().focusNodes([...ids.values()]);
  useAgentStore.getState().pushStep({ kind: "connect", text: `已在当前流程修改并拼接：${plan.nodes.length} 个节点、${plan.edges.length} 条连线` });
  return ids;
}
