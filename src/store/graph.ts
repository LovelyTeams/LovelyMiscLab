import {
  addEdge,
  applyEdgeChanges,
  applyNodeChanges,
  type Connection,
  type Edge,
  type EdgeChange,
  type Node,
  type NodeChange,
} from "@xyflow/react";
import { create } from "zustand";

import type { NodeDescriptor, PortValue } from "@/lib/types";
import type { SavedEdge, SavedNode } from "@/lib/project";

import { measuredLayeredLayout, packedLayout } from "@/flow/layout";
import { connectionError } from "@/flow/graphValidation";
import { useDescriptorStore } from "@/store/descriptors";

export type NodeStatus = "idle" | "running" | "done" | "error";

export interface NodeLog {
  time: string;
  level: string;
  message: string;
}

export interface FlowNodeData {
  descriptorId: string;
  label: string;
  color: string;
  params: Record<string, unknown>;
  status: NodeStatus;
  progress: number;
  outputs?: Record<string, PortValue>;
  error?: string;
  disabled?: boolean;
  logs?: NodeLog[];
  /** Param names promoted to input ports (driven by upstream connections). */
  inputParams?: string[];
  // Index signature required by React Flow's Node<Data> constraint.
  [key: string]: unknown;
}

export type FlowNode = Node<FlowNodeData>;

let counter = 0;
const nextId = (prefix: string) => `${prefix}_${counter++}`;

/** React Flow node component to use for a descriptor (most use the generic one). */
const flowType = (descriptorId: string) =>
  descriptorId === "selector" ? "selector" : "generic";

export interface ClipboardNode {
  oldId: string;
  descriptorId: string;
  label: string;
  color: string;
  params: Record<string, unknown>;
  inputParams: string[];
  disabled?: boolean;
  position: { x: number; y: number };
}
export interface Clipboard {
  nodes: ClipboardNode[];
  edges: {
    source: string;
    sourceHandle?: string | null;
    target: string;
    targetHandle?: string | null;
    type?: string;
  }[];
}

interface GraphSnapshot {
  nodes: FlowNode[];
  edges: Edge[];
}

const MAX_HISTORY = 80;

function invalidateFrom(nodes: FlowNode[], edges: Edge[], roots: string[]): FlowNode[] {
  const affected = new Set<string>();
  const pending = [...roots];
  while (pending.length) {
    const id = pending.pop()!;
    if (affected.has(id)) continue;
    affected.add(id);
    for (const edge of edges) if (edge.source === id) pending.push(edge.target);
  }
  return nodes.map((n) => affected.has(n.id) ? { ...n, data: {
    ...n.data, outputs: undefined, error: undefined, status: "idle", progress: 0,
  } } : n);
}

function cloneNode(n: FlowNode): FlowNode {
  return {
    ...n,
    position: { ...n.position },
    data: {
      ...n.data,
      params: structuredClone(n.data.params),
      outputs: n.data.outputs ? { ...n.data.outputs } : undefined,
      logs: n.data.logs ? [...n.data.logs] : undefined,
      inputParams: n.data.inputParams ? [...n.data.inputParams] : undefined,
    },
  };
}

function cloneEdge(e: Edge): Edge {
  return { ...e };
}

function snapshot(nodes: FlowNode[], edges: Edge[]): GraphSnapshot {
  return {
    nodes: nodes.map(cloneNode),
    edges: edges.map(cloneEdge),
  };
}

function pushPast(past: GraphSnapshot[], snap: GraphSnapshot): GraphSnapshot[] {
  return [...past, snap].slice(-MAX_HISTORY);
}

/** History patch for a mutating action: a fresh past entry (clearing redo),
 * unless recording is suppressed (agent batch) — then `{}` keeps undo collapsed
 * to the single pre-build snapshot. */
function histFields(state: {
  suppressHistory: boolean;
  past: GraphSnapshot[];
  nodes: FlowNode[];
  edges: Edge[];
  historyGroup: { key: string; recorded: boolean } | null;
}, key?: string): Partial<Pick<GraphState, "past" | "future" | "historyGroup">> {
  if (state.suppressHistory) return {};
  const group = key && state.historyGroup?.key === key ? state.historyGroup : null;
  if (group?.recorded) return { future: [] };
  return {
    past: pushPast(state.past, snapshot(state.nodes, state.edges)), future: [],
    historyGroup: group ? { ...group, recorded: true } : null,
  };
}

function shouldRecordNodeChanges(changes: NodeChange<FlowNode>[]) {
  return changes.some((c) => c.type !== "select" && c.type !== "dimensions");
}

function shouldRecordEdgeChanges(changes: EdgeChange[]) {
  return changes.some((c) => c.type !== "select");
}

interface GraphState {
  nodes: FlowNode[];
  edges: Edge[];
  selectedId: string | null;
  past: GraphSnapshot[];
  future: GraphSnapshot[];
  runRevision: number;
  /** When true, mutations skip history recording — used to collapse an agent's
   * many-step build into a single undo entry (the pre-build snapshot). */
  suppressHistory: boolean;
  historyGroup: { key: string; recorded: boolean } | null;
  beginHistoryGroup: (key: string) => void;
  endHistoryGroup: (key?: string) => void;

  onNodesChange: (changes: NodeChange<FlowNode>[]) => void;
  onEdgesChange: (changes: EdgeChange[]) => void;
  onConnect: (conn: Connection) => void;
  reconnectEdge: (id: string, conn: Connection) => void;

  addNode: (descriptor: NodeDescriptor, position: { x: number; y: number }) => string;
  setParam: (nodeId: string, name: string, value: unknown) => void;
  setSelected: (id: string | null) => void;
  updateRuntime: (nodeId: string, patch: Partial<FlowNodeData>) => void;
  resetRuntime: (ids?: ReadonlySet<string>) => void;
  clear: () => void;
  deleteNode: (id: string) => void;
  deleteEdge: (id: string) => void;
  deleteSelection: () => void;
  duplicateNode: (id: string) => void;
  /** Pack all nodes to fit the current view (vertical-first, wrap horizontally).
   * `aspect` = viewport width/height, so the block matches the screen. */
  arrangeNodes: (aspect: number) => void;
  /** Refine an AI build without adding undo steps or invalidating its results. */
  layoutAgentNodes: (ids: ReadonlySet<string>) => boolean;
  setSuppressHistory: (v: boolean) => void;
  selectAll: () => void;
  selectNode: (id: string) => void;
  selectEdge: (id: string) => void;
  addToSelection: (nodeIds: Iterable<string>, edgeIds: Iterable<string>) => void;
  moveSelection: (dx: number, dy: number) => void;
  setDisabled: (id: string, disabled: boolean) => void;
  appendLog: (id: string, log: NodeLog) => void;
  toggleParamInput: (nodeId: string, name: string) => void;
  renameNode: (id: string, label: string) => void;
  deselectAll: () => void;
  paste: (clip: Clipboard, dx: number, dy: number) => void;
  loadFlow: (nodes: SavedNode[], edges: SavedEdge[]) => void;
  undo: () => void;
  redo: () => void;
  canUndo: () => boolean;
  canRedo: () => boolean;
}

export const useGraphStore = create<GraphState>((set, get) => ({
  nodes: [],
  edges: [],
  selectedId: null,
  past: [],
  future: [],
  runRevision: 0,
  suppressHistory: false,
  historyGroup: null,
  beginHistoryGroup: (key) => set((state) => state.historyGroup?.key === key ? {} : { historyGroup: { key, recorded: false } }),
  endHistoryGroup: (key) => set((state) => !key || state.historyGroup?.key === key ? { historyGroup: null } : {}),

  onNodesChange: (changes) =>
    set((state) => {
      const nodes = applyNodeChanges(changes, state.nodes);
      const meaningful = changes.filter((c) => c.type !== "position" || (c.position && state.nodes.some((n) =>
        n.id === c.id && (n.position.x !== c.position!.x || n.position.y !== c.position!.y))));
      if (!shouldRecordNodeChanges(meaningful)) return { nodes };
      const removed = new Set(changes.filter((c) => c.type === "remove").map((c) => c.id));
      return {
        nodes: invalidateFrom(nodes, state.edges, [...removed]),
        edges: state.edges.filter((e) => !removed.has(e.source) && !removed.has(e.target)),
        selectedId: removed.has(state.selectedId ?? "") ? null : state.selectedId,
        ...histFields(state),
        runRevision: state.runRevision + (removed.size ? 1 : 0),
      };
    }),

  reconnectEdge: (id, conn) => set((state) => {
    const old = state.edges.find((e) => e.id === id);
    if (!old || (old.source === conn.source && old.target === conn.target &&
      old.sourceHandle === conn.sourceHandle && old.targetHandle === conn.targetHandle)) return {};
    const remaining = state.edges.filter((e) => e.id !== id);
    if (connectionError(conn, state.nodes, remaining, useDescriptorStore.getState().byId)) return {};
    return {
      edges: [...remaining.filter((e) => !(e.target === conn.target && e.targetHandle === conn.targetHandle)), { ...old, ...conn }],
      nodes: invalidateFrom(state.nodes, state.edges, [old.target, conn.target]),
      ...histFields(state), runRevision: state.runRevision + 1,
    };
  }),
  onEdgesChange: (changes) =>
    set((state) => {
      const edges = applyEdgeChanges(changes, state.edges);
      if (!shouldRecordEdgeChanges(changes)) return { edges };
      return {
        edges,
        nodes: invalidateFrom(state.nodes, state.edges, state.edges.filter((e) => !edges.some((next) => next.id === e.id)).map((e) => e.target)),
        ...histFields(state),
        runRevision: state.runRevision + 1,
      };
    }),
  onConnect: (conn) =>
    set((state) => {
      if (connectionError(conn, state.nodes, state.edges, useDescriptorStore.getState().byId)) return {};
      if (state.edges.some((e) => e.source === conn.source && e.sourceHandle === conn.sourceHandle &&
        e.target === conn.target && e.targetHandle === conn.targetHandle)) return {};
      return {
        edges: addEdge(conn, state.edges.filter((e) =>
          !(e.target === conn.target && e.targetHandle === conn.targetHandle))),
        nodes: invalidateFrom(state.nodes, state.edges, [conn.target]),
        ...histFields(state),
        runRevision: state.runRevision + 1,
      };
    }),

  addNode: (descriptor, position) => {
    const params: Record<string, unknown> = {};
    for (const p of descriptor.params) params[p.name] = p.default;
    const node: FlowNode = {
      id: nextId(descriptor.id),
      type: flowType(descriptor.id),
      selected: true,
      position,
      data: {
        descriptorId: descriptor.id,
        label: descriptor.displayName,
        color: descriptor.color,
        params,
        status: "idle",
        progress: 0,
        disabled: false,
        logs: [],
        inputParams: [],
      },
    };
    set((state) => ({
      nodes: [...state.nodes.map((n) => ({ ...n, selected: false })), node],
      edges: state.edges.map((e) => ({ ...e, selected: false })),
      selectedId: node.id,
      ...histFields(state),
      runRevision: state.runRevision + 1,
    }));
    return node.id;
  },

  setParam: (nodeId, name, value) =>
    set((state) => {
      const node = state.nodes.find((n) => n.id === nodeId);
      if (!node || Object.is(node.data.params[name], value) || (typeof value === "number" && !Number.isFinite(value))) return {};
      return {
      nodes: invalidateFrom(state.nodes, state.edges, [nodeId]).map((n) =>
        n.id === nodeId
          ? {
              ...n,
              data: { ...n.data, params: { ...n.data.params, [name]: value } },
            }
          : n
      ),
      ...histFields(state, `param:${nodeId}:${name}`),
      runRevision: state.runRevision + 1,
    }; }),

  setSelected: (id) => set({ selectedId: id }),

  updateRuntime: (nodeId, patch) =>
    set({
      nodes: get().nodes.map((n) =>
        n.id === nodeId ? { ...n, data: { ...n.data, ...patch } } : n
      ),
    }),

  layoutAgentNodes: (ids) => {
    const state = get();
    const positions = measuredLayeredLayout(state.nodes.filter((n) => ids.has(n.id)), state.edges);
    let changed = false;
    const nodes = state.nodes.map((node) => {
      const position = positions[node.id];
      if (!position || (position.x === node.position.x && position.y === node.position.y)) return node;
      changed = true;
      return { ...node, position };
    });
    if (changed) set({ nodes });
    return changed;
  },

  resetRuntime: (ids) =>
    set({
      nodes: get().nodes.map((n) => ids && !ids.has(n.id) ? n : ({
        ...n,
        data: {
          ...n.data,
          status: "idle",
          progress: 0,
          error: undefined,
          outputs: undefined,
          logs: [],
        },
      })),
    }),

  clear: () =>
    set((state) => ({
      nodes: [],
      edges: [],
      selectedId: null,
      ...histFields(state),
      runRevision: state.runRevision + 1,
    })),

  deleteNode: (id) =>
    set((state) => ({
      nodes: invalidateFrom(state.nodes, state.edges, [id]).filter((n) => n.id !== id),
      edges: state.edges.filter((e) => e.source !== id && e.target !== id),
      selectedId: state.selectedId === id ? null : state.selectedId,
      ...histFields(state),
      runRevision: state.runRevision + 1,
    })),

  deleteEdge: (id) =>
    set((state) => ({
      nodes: invalidateFrom(state.nodes, state.edges, state.edges.filter((e) => e.id === id).map((e) => e.target)),
      edges: state.edges.filter((e) => e.id !== id),
      ...histFields(state),
      runRevision: state.runRevision + 1,
    })),

  deleteSelection: () => set((state) => {
    const ids = new Set(state.nodes.filter((n) => n.selected).map((n) => n.id));
    if (!ids.size && !state.edges.some((e) => e.selected)) return {};
    return {
      nodes: invalidateFrom(state.nodes, state.edges, [...ids, ...state.edges.filter((e) => e.selected).map((e) => e.target)]).filter((n) => !ids.has(n.id)),
      edges: state.edges.filter((e) => !e.selected && !ids.has(e.source) && !ids.has(e.target)),
      selectedId: ids.has(state.selectedId ?? "") ? null : state.selectedId,
      ...histFields(state),
      runRevision: state.runRevision + 1,
    };
  }),

  duplicateNode: (id) => {
    const n = get().nodes.find((x) => x.id === id);
    if (!n) return;
    const copy: FlowNode = {
      ...n,
      id: nextId(n.data.descriptorId),
      position: { x: n.position.x + 32, y: n.position.y + 32 },
      selected: true,
      data: {
        ...n.data,
        params: structuredClone(n.data.params),
        inputParams: [...(n.data.inputParams ?? [])],
        status: "idle",
        progress: 0,
        outputs: undefined,
        error: undefined,
        logs: [],
      },
    };
    set((state) => ({
      nodes: [...state.nodes.map((n) => ({ ...n, selected: false })), copy],
      edges: state.edges.map((e) => ({ ...e, selected: false })),
      selectedId: copy.id,
      ...histFields(state),
      runRevision: state.runRevision + 1,
    }));
  },

  arrangeNodes: (aspect) =>
    set((state) => {
      if (state.nodes.length === 0) return {};
      const pos = packedLayout(state.nodes, state.edges, aspect);
      return {
        nodes: state.nodes.map((n) =>
          pos[n.id] ? { ...n, position: pos[n.id] } : n
        ),
        ...histFields(state),
      };
    }),

  setSuppressHistory: (v) => set((state) => ({
    ...(v && !state.suppressHistory ? histFields(state) : {}),
    suppressHistory: v,
  })),

  selectAll: () => set((state) => ({
    nodes: state.nodes.map((n) => ({ ...n, selected: true })),
    edges: state.edges.map((e) => ({ ...e, selected: true })),
    selectedId: state.nodes[0]?.id ?? null,
  })),
  selectNode: (id) => set((state) => ({
    nodes: state.nodes.map((n) => ({ ...n, selected: n.id === id })),
    edges: state.edges.map((e) => ({ ...e, selected: false })), selectedId: id,
  })),
  selectEdge: (id) => set((state) => ({
    nodes: state.nodes.map((n) => ({ ...n, selected: false })),
    edges: state.edges.map((e) => ({ ...e, selected: e.id === id })), selectedId: null,
  })),
  addToSelection: (nodeIds, edgeIds) => set((state) => {
    const nodeSet = new Set(nodeIds), edgeSet = new Set(edgeIds);
    const nodes = state.nodes.map((n) => nodeSet.has(n.id) ? { ...n, selected: true } : n);
    return { nodes, edges: state.edges.map((e) => edgeSet.has(e.id) ? { ...e, selected: true } : e),
      selectedId: nodes.find((n) => n.selected)?.id ?? null };
  }),
  moveSelection: (dx, dy) => set((state) => {
    if ((!dx && !dy) || !state.nodes.some((n) => n.selected)) return {};
    return {
      nodes: state.nodes.map((n) => n.selected ? { ...n, position: { x: n.position.x + dx, y: n.position.y + dy } } : n),
      ...histFields(state, "moveSelection"),
    };
  }),

  setDisabled: (id, disabled) =>
    set((state) => ({
      nodes: invalidateFrom(state.nodes, state.edges, [id]).map((n) =>
        n.id === id ? { ...n, data: { ...n.data, disabled } } : n
      ),
      ...histFields(state),
      runRevision: state.runRevision + 1,
    })),

  appendLog: (id, log) =>
    set({
      nodes: get().nodes.map((n) =>
        n.id === id
          ? { ...n, data: { ...n.data, logs: [...(n.data.logs ?? []), { ...log, message: log.message.slice(0, 4000) }].slice(-300) } }
          : n
      ),
    }),

  toggleParamInput: (nodeId, name) => {
    const node = get().nodes.find((n) => n.id === nodeId);
    const removing = node?.data.inputParams?.includes(name) ?? false;
    set((state) => ({
      nodes: invalidateFrom(state.nodes, state.edges, [nodeId]).map((n) =>
        n.id === nodeId
          ? {
              ...n,
              data: {
                ...n.data,
                inputParams: removing
                  ? (n.data.inputParams ?? []).filter((x) => x !== name)
                  : [...(n.data.inputParams ?? []), name],
              },
            }
          : n
      ),
      // Un-promoting drops any edge that was feeding this param.
      edges: removing
        ? state.edges.filter((e) => !(e.target === nodeId && e.targetHandle === name))
        : state.edges,
      ...histFields(state),
      runRevision: state.runRevision + 1,
    }));
  },

  renameNode: (id, label) =>
    set((state) => {
      if (!state.nodes.some((n) => n.id === id && n.data.label !== label)) return {};
      return {
      nodes: state.nodes.map((n) =>
        n.id === id ? { ...n, data: { ...n.data, label } } : n
      ),
      ...histFields(state, `rename:${id}`),
    }; }),

  deselectAll: () =>
    set({
      nodes: get().nodes.map((n) => (n.selected ? { ...n, selected: false } : n)),
      edges: get().edges.map((e) => e.selected ? { ...e, selected: false } : e),
      selectedId: null,
    }),

  paste: (clip, dx, dy) => {
    if (!clip.nodes.length) return;
    const idMap: Record<string, string> = {};
    const newNodes: FlowNode[] = clip.nodes.map((cn) => {
      const id = nextId(cn.descriptorId);
      idMap[cn.oldId] = id;
      return {
        id,
        type: flowType(cn.descriptorId),
        position: { x: cn.position.x + dx, y: cn.position.y + dy },
        selected: true,
        data: {
          descriptorId: cn.descriptorId,
          label: cn.label,
          color: cn.color,
          params: structuredClone(cn.params),
          status: "idle",
          progress: 0,
          disabled: cn.disabled ?? false,
          logs: [],
          inputParams: [...cn.inputParams],
        },
      };
    });
    let edges = get().edges.map((e) => ({ ...e, selected: false }));
    for (const e of clip.edges) {
      const source = idMap[e.source];
      const target = idMap[e.target];
      if (source && target) {
        edges = addEdge(
          {
            source,
            sourceHandle: e.sourceHandle ?? null,
            target,
            targetHandle: e.targetHandle ?? null,
            type: e.type,
          },
          edges
        );
      }
    }
    set((state) => ({
      nodes: [...state.nodes.map((n) => ({ ...n, selected: false })), ...newNodes],
      edges,
      selectedId: newNodes[0]?.id ?? get().selectedId,
      ...histFields(state),
      runRevision: state.runRevision + 1,
    }));
  },

  loadFlow: (savedNodes, savedEdges) => {
    const flowNodes: FlowNode[] = savedNodes.map((n) => ({
      id: n.id,
      type: flowType(n.descriptorId),
      position: { x: n.position.x, y: n.position.y },
      data: {
        descriptorId: n.descriptorId,
        label: n.label,
        color: n.color,
        params: { ...n.params },
        status: "idle",
        progress: 0,
        disabled: n.disabled ?? false,
        logs: [],
        inputParams: [...(n.inputParams ?? [])],
      },
    }));
    const flowEdges: Edge[] = savedEdges.map((e) => ({
      id: e.id,
      source: e.source,
      sourceHandle: e.sourceHandle ?? null,
      target: e.target,
      targetHandle: e.targetHandle ?? null,
      ...(e.type ? { type: e.type } : {}),
    }));
    // Bump the id counter past loaded numeric suffixes to avoid future collisions.
    let max = counter;
    for (const n of savedNodes) {
      const m = /_(\d+)$/.exec(n.id);
      if (m) max = Math.max(max, Number(m[1]) + 1);
    }
    counter = max;
    set({
      nodes: flowNodes,
      edges: flowEdges,
      selectedId: null,
      past: [],
      future: [],
      historyGroup: null,
      suppressHistory: false,
      runRevision: get().runRevision + 1,
    });
  },

  undo: () =>
    set((state) => {
      const previous = state.past[state.past.length - 1];
      if (!previous) return {};
      return {
        nodes: invalidateFrom(previous.nodes.map(cloneNode), previous.edges, previous.nodes.map((n) => n.id)),
        edges: previous.edges.map(cloneEdge),
        selectedId: previous.nodes.find((n) => n.selected)?.id ?? null,
        historyGroup: null,
        past: state.past.slice(0, -1),
        future: [snapshot(state.nodes, state.edges), ...state.future].slice(0, MAX_HISTORY),
        runRevision: state.runRevision + 1,
      };
    }),

  redo: () =>
    set((state) => {
      const next = state.future[0];
      if (!next) return {};
      return {
        nodes: invalidateFrom(next.nodes.map(cloneNode), next.edges, next.nodes.map((n) => n.id)),
        edges: next.edges.map(cloneEdge),
        selectedId: next.nodes.find((n) => n.selected)?.id ?? null,
        historyGroup: null,
        past: pushPast(state.past, snapshot(state.nodes, state.edges)),
        future: state.future.slice(1),
        runRevision: state.runRevision + 1,
      };
    }),

  canUndo: () => get().past.length > 0,
  canRedo: () => get().future.length > 0,
}));
