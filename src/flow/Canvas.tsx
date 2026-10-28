import { useCallback, useEffect, useRef, useState } from "react";
import {
  Background,
  BackgroundVariant,
  Controls,
  MarkerType,
  MiniMap,
  ReactFlow,
  SelectionMode,
  useReactFlow,
  useStoreApi,
} from "@xyflow/react";
import { Hand, MousePointer2, Trash2 } from "lucide-react";

import type { NodeDescriptor } from "@/lib/types";
import { hasTextSelection, isTextContextTarget } from "@/lib/textInteraction";
import { AgentPanel } from "@/app/AgentPanel";
import { useAgentStore } from "@/store/agent";
import { useGraphStore } from "@/store/graph";
import { usePaletteDrag } from "@/store/paletteDrag";
import { usePortSuggest } from "@/store/portSuggest";
import { useThemeStore } from "@/store/theme";

import { runAgent } from "./agentRunner";
import { ContextMenu, type MenuItem } from "./ContextMenu";
import { viewportAspect } from "./layout";
import { GenericNode } from "./GenericNode";
import { LabeledEdge } from "./LabeledEdge";
import { NodeSearchMenu } from "./NodeSearchMenu";
import { connectionError } from "./graphValidation";
import { useDescriptorStore } from "@/store/descriptors";
import { PortSuggest } from "./PortSuggest";
import { executeGraph } from "./runner";
import { SelectorNode } from "./SelectorNode";
import { copySelection, cutSelection, duplicateSelection, hasGraphClipboard, pasteSelection } from "./graphClipboard";

const nodeTypes = { generic: GenericNode, selector: SelectorNode };
const edgeTypes = { labeled: LabeledEdge };
const multiSelectionKeys = ["Control", "Meta", "Shift"];
const middleButton = [1];
const leftAndMiddleButtons = [0, 1];

type Menu = {
  x: number;
  y: number;
  kind: "pane" | "node" | "edge";
  id?: string;
  flow?: { x: number; y: number };
};

type Search = { x: number; y: number; flow: { x: number; y: number } };

export function Canvas() {
  const nodes = useGraphStore((s) => s.nodes);
  const edges = useGraphStore((s) => s.edges);
  const onNodesChange = useGraphStore((s) => s.onNodesChange);
  const onEdgesChange = useGraphStore((s) => s.onEdgesChange);
  const onConnect = useGraphStore((s) => s.onConnect);
  const addNode = useGraphStore((s) => s.addNode);
  const setSelected = useGraphStore((s) => s.setSelected);
  const setDrop = usePaletteDrag((s) => s.setDrop);
  const theme = useThemeStore((s) => s.theme);
  const rf = useReactFlow();
  const flowStore = useStoreApi();
  const selectionBeforePointer = useRef<{ nodes: Set<string>; edges: Set<string> }>({ nodes: new Set(), edges: new Set() });
  const appendBoxSelection = useRef(false);
  const suggestCtx = usePortSuggest((s) => s.ctx);
  const pendingPlan = useAgentStore((s) => s.pendingPlan);
  const focusNodeIds = useAgentStore((s) => s.focusNodeIds);
  const agentRunning = useAgentStore((s) => s.running);
  const [menu, setMenu] = useState<Menu | null>(null);
  const [search, setSearch] = useState<Search | null>(null);
  const [tool, setTool] = useState<"select" | "pan">("select");
  const selectedCount = nodes.filter((n) => n.selected).length;
  const selectedEdges = edges.filter((e) => e.selected).length;

  // Consume a reviewed plan or a one-click solve request exactly once.
  useEffect(() => {
    if (!pendingPlan || useAgentStore.getState().pendingPlan !== pendingPlan) return;
    useAgentStore.getState().clearPending();
    void runAgent(pendingPlan);
  }, [pendingPlan]);

  useEffect(() => {
    if (!focusNodeIds.length) return;
    const targets = nodes.filter((n) => focusNodeIds.includes(n.id));
    if (targets.length !== focusNodeIds.length) {
      useAgentStore.getState().focusNodes([]);
      return;
    }
    if (targets.some((n) => !n.measured?.width || !n.measured?.height || n.dragging)) return;
    if (useGraphStore.getState().layoutAgentNodes(new Set(focusNodeIds))) return;
    // ResizeObserver reports the final output previews after execution finishes.
    // Coalesce those measurements before fitting, without recording another undo.
    const timer = window.setTimeout(() => {
      void rf.fitView({ nodes: focusNodeIds.map((id) => ({ id })), duration: 250, padding: 0.2, minZoom: 0.1, maxZoom: 1 });
      if (!agentRunning) useAgentStore.getState().focusNodes([]);
    }, 120);
    return () => window.clearTimeout(timer);
  }, [focusNodeIds, agentRunning, nodes, edges, rf]);

  // Resolve a palette drop: add at the cursor if over the canvas, else (a plain
  // click) add near the canvas center.
  useEffect(() => {
    setDrop((d, x, y, moved) => {
      const el = document.elementFromPoint(x, y);
      const overCanvas = !!(el && el.closest(".react-flow"));
      if (overCanvas) {
        addNode(d, rf.screenToFlowPosition({ x, y }));
      } else if (!moved) {
        addNode(
          d,
          rf.screenToFlowPosition({
            x: window.innerWidth / 2 - 90,
            y: window.innerHeight / 2 - 40,
          })
        );
      }
    });
  }, [setDrop, addNode, rf]);

  const isValidConnection = useCallback(
    (c: {
      source?: string | null;
      target?: string | null;
      sourceHandle?: string | null;
      targetHandle?: string | null;
    }) => {
      if (!c.source || !c.target || !c.sourceHandle || !c.targetHandle) return false;
      const g = useGraphStore.getState();
      return !connectionError({ source: c.source, target: c.target,
        sourceHandle: c.sourceHandle, targetHandle: c.targetHandle },
        g.nodes, g.edges, useDescriptorStore.getState().byId);
    },
    []
  );

  const closeMenu = useCallback(() => setMenu(null), []);

  // Double-click empty canvas → open the node search picker at the cursor.
  const onDoubleClick = (e: React.MouseEvent) => {
    const el = e.target as HTMLElement;
    if (!el.classList.contains("react-flow__pane")) return;
    setSearch({
      x: e.clientX,
      y: e.clientY,
      flow: rf.screenToFlowPosition({ x: e.clientX, y: e.clientY }),
    });
  };

  const onPick = (d: NodeDescriptor) => {
    if (search) addNode(d, search.flow);
    setSearch(null);
  };

  const menuItems = (): MenuItem[] => {
    if (!menu) return [];
    const g = useGraphStore.getState();
    if (menu.kind === "node") {
      return [
        { label: "复制", shortcut: "Ctrl+C", onClick: copySelection },
        { label: "剪切", shortcut: "Ctrl+X", onClick: cutSelection },
        { label: "创建副本", shortcut: "Ctrl+D", onClick: duplicateSelection },
        { label: selectedCount > 1 ? `删除选中的 ${selectedCount} 个节点` : "删除节点", shortcut: "Delete", danger: true, onClick: () => g.deleteSelection() },
      ];
    }
    if (menu.kind === "edge") {
      return [{ label: "删除选中连线", shortcut: "Delete", danger: true, onClick: () => g.deleteSelection() }];
    }
    const { x, y, flow } = menu;
    return [
      { label: "添加节点…", onClick: () => flow && setSearch({ x, y, flow }) },
      { label: "粘贴", shortcut: "Ctrl+V", disabled: !hasGraphClipboard(), onClick: () => pasteSelection(flow) },
      { label: "撤销", shortcut: "Ctrl+Z", disabled: !g.canUndo(), onClick: () => g.undo() },
      { label: "重做", shortcut: "Ctrl+Y", disabled: !g.canRedo(), onClick: () => g.redo() },
      { label: "运行整图", onClick: () => void executeGraph() },
      {
        label: "整理节点",
        onClick: () => {
          useAgentStore.getState().focusNodes([]);
          g.arrangeNodes(viewportAspect());
          // Let React Flow apply the new positions before fitView measures them.
          requestAnimationFrame(() => rf.fitView({ duration: 250, padding: 0.15 }));
        },
      },
      { label: "适应视图", shortcut: "Home", onClick: () => rf.fitView({ duration: 200 }) },
      { label: "全选", shortcut: "Ctrl+A", onClick: () => g.selectAll() },
      { label: "清空画布", danger: true, onClick: () => g.clear() },
    ];
  };

  return (
    <div className="relative flex h-full w-full outline-none" tabIndex={0} data-testid="canvas-surface"
      onPointerDownCapture={(e) => {
        if (e.button !== 0 || isTextContextTarget(e.target)) return;
        const graph = useGraphStore.getState();
        selectionBeforePointer.current = {
          nodes: new Set(graph.nodes.filter((n) => n.selected).map((n) => n.id)),
          edges: new Set(graph.edges.filter((edge) => edge.selected).map((edge) => edge.id)),
        };
        // Read the actual pointer modifiers too: a quick Ctrl-click can arrive
        // before React Flow's keypress effect has committed multi-selection.
        flowStore.setState({ multiSelectionActive: e.ctrlKey || e.metaKey || e.shiftKey });
        appendBoxSelection.current = tool === "select" && (e.ctrlKey || e.metaKey || e.shiftKey) &&
          (e.target as HTMLElement).classList.contains("react-flow__pane");
        if ((e.target as HTMLElement).classList.contains("react-flow__pane")) {
          document.getSelection()?.removeAllRanges();
          (e.target as Element).closest<HTMLElement>('[data-canvas-commands]')?.focus({ preventScroll: true });
        }
      }}
      onDoubleClick={onDoubleClick}>
      <div className="relative min-h-0 min-w-0 flex-1 outline-none" tabIndex={0} data-canvas-commands
        onContextMenu={(e) => {
          if (e.target !== e.currentTarget || hasTextSelection()) return;
          e.preventDefault();
          setMenu({ x: e.clientX, y: e.clientY, kind: 'pane', flow: rf.screenToFlowPosition({ x: e.clientX, y: e.clientY }) });
        }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        defaultEdgeOptions={{
          type: "labeled",
          markerEnd: { type: MarkerType.ArrowClosed, width: 16, height: 16 },
        }}
        onNodesChange={onNodesChange}
        onNodeDragStart={() => { useAgentStore.getState().focusNodes([]); useGraphStore.getState().setSuppressHistory(true); }}
        onNodeDragStop={() => useGraphStore.getState().setSuppressHistory(false)}
        onSelectionDragStart={() => { useAgentStore.getState().focusNodes([]); useGraphStore.getState().setSuppressHistory(true); }}
        onSelectionDragStop={() => useGraphStore.getState().setSuppressHistory(false)}
        onSelectionEnd={() => {
          if (appendBoxSelection.current) {
            const before = selectionBeforePointer.current;
            useGraphStore.getState().addToSelection(before.nodes, before.edges);
          }
          appendBoxSelection.current = false;
        }}
        onSelectionContextMenu={(e, selected) => {
          if (isTextContextTarget(e.target) || hasTextSelection()) return;
          e.preventDefault();
          setMenu({ x: e.clientX, y: e.clientY, kind: "node", id: selected[0]?.id });
        }}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onReconnect={(edge, connection) => useGraphStore.getState().reconnectEdge(edge.id, connection)}
        isValidConnection={isValidConnection as never}
        onSelectionChange={({ nodes }) => setSelected(nodes[0]?.id ?? null)}
        onNodeClick={(e, node) => {
          if (!(e.ctrlKey || e.metaKey || e.shiftKey) || (e.target as Element).closest(".nodrag")) return;
          const selected = selectionBeforePointer.current.nodes;
          const g = useGraphStore.getState();
          g.onNodesChange(g.nodes.map((n) => ({ type: "select", id: n.id, selected: n.id === node.id ? !selected.has(n.id) : selected.has(n.id) })));
        }}
        onEdgeClick={(e, edge) => {
          if (!(e.ctrlKey || e.metaKey || e.shiftKey)) return;
          const selected = selectionBeforePointer.current.edges;
          const g = useGraphStore.getState();
          g.onEdgesChange(g.edges.map((item) => ({ type: "select", id: item.id, selected: item.id === edge.id ? !selected.has(item.id) : selected.has(item.id) })));
        }}
        onPaneClick={closeMenu}
        onMoveStart={closeMenu}
        onPaneContextMenu={(e) => {
          if (isTextContextTarget(e.target) || hasTextSelection()) return;
          e.preventDefault();
          setMenu({
            x: e.clientX,
            y: e.clientY,
            kind: "pane",
            flow: rf.screenToFlowPosition({ x: e.clientX, y: e.clientY }),
          });
        }}
        onNodeContextMenu={(e, node) => {
          if (isTextContextTarget(e.target) || hasTextSelection()) return;
          e.preventDefault();
          if (!node.selected) useGraphStore.getState().selectNode(node.id);
          setMenu({ x: e.clientX, y: e.clientY, kind: "node", id: node.id });
        }}
        onEdgeContextMenu={(e, edge) => {
          if (isTextContextTarget(e.target) || hasTextSelection()) return;
          e.preventDefault();
          if (!edge.selected) useGraphStore.getState().selectEdge(edge.id);
          setMenu({ x: e.clientX, y: e.clientY, kind: "edge", id: edge.id });
        }}
        onlyRenderVisibleElements={focusNodeIds.length === 0}
        deleteKeyCode={null}
        selectionOnDrag={tool === "select"}
        selectionMode={SelectionMode.Partial}
        selectionKeyCode={null}
        multiSelectionKeyCode={multiSelectionKeys}
        panOnDrag={tool === "pan" ? leftAndMiddleButtons : middleButton}
        panActivationKeyCode="Space"
        fitView
        minZoom={0.1}
        zoomOnDoubleClick={false}
        colorMode={theme}
        proOptions={{ hideAttribution: true }}
      >
        <Background
          variant={BackgroundVariant.Dots}
          gap={16}
          size={1}
          color={theme === "dark" ? "#2a3340" : "#cbd5e1"}
        />
        <Controls />
        <MiniMap
          pannable
          zoomable
          nodeColor={(n) => (n.data?.color as string) ?? "#64748b"}
          maskColor={theme === "dark" ? "#0d101799" : "#f1f5f999"}
          style={{ background: theme === "dark" ? "#151a22" : "#e2e8f0" }}
        />
      </ReactFlow>

      <div role="toolbar" aria-label="画布工具" className="absolute left-3 top-3 z-10 flex items-center gap-1 rounded-lg border border-border bg-card/95 p-1 shadow-sm">
        <button aria-pressed={tool === "select"} title="选择工具：左键拖动框选，Ctrl / Shift 点击多选"
          onClick={() => setTool("select")} className={`flex items-center gap-1 rounded px-2 py-1 text-xs ${tool === "select" ? "bg-primary/10 text-primary" : "text-muted-foreground hover:bg-accent"}`}>
          <MousePointer2 className="h-3.5 w-3.5" />框选
        </button>
        <button aria-pressed={tool === "pan"} title="平移工具：拖动画布；也可按住空格或使用中键"
          onClick={() => setTool("pan")} className={`flex items-center gap-1 rounded px-2 py-1 text-xs ${tool === "pan" ? "bg-primary/10 text-primary" : "text-muted-foreground hover:bg-accent"}`}>
          <Hand className="h-3.5 w-3.5" />平移
        </button>
        {(selectedCount > 0 || selectedEdges > 0) && <>
          <span className="border-l border-border px-2 text-[11px] text-muted-foreground" aria-live="polite">已选 {selectedCount} 个节点 · {selectedEdges} 条连线</span>
          <button title="删除选中内容 (Delete)" onClick={() => useGraphStore.getState().deleteSelection()} className="rounded p-1 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"><Trash2 className="h-3.5 w-3.5" /></button>
        </>}
      </div>
      <div className="pointer-events-none absolute bottom-3 left-12 rounded bg-card/90 px-2 py-1 text-[10px] text-muted-foreground">
        {tool === "select" ? "左键框选" : "左键平移"} · Ctrl / Shift 多选 · 空格 / 中键平移
      </div>

      {menu && (
        <ContextMenu x={menu.x} y={menu.y} items={menuItems()} onClose={closeMenu} />
      )}
      {search && (
        <NodeSearchMenu
          x={search.x}
          y={search.y}
          onPick={onPick}
          onClose={() => setSearch(null)}
        />
      )}
      {suggestCtx && <PortSuggest />}
      </div>
      <AgentPanel />
    </div>
  );
}
