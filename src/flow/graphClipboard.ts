import { useGraphStore, type Clipboard } from "@/store/graph";

// Shared by the keyboard and context menu. It never replaces the system's text clipboard.
let clipboard: Clipboard | null = null;
let pasteOffset = 0;

export function selectedSubgraph(): Clipboard | null {
  const g = useGraphStore.getState();
  const nodes = g.nodes.filter((n) => n.selected);
  if (!nodes.length) return null;
  const ids = new Set(nodes.map((n) => n.id));
  return {
    nodes: nodes.map((n) => ({
      oldId: n.id, descriptorId: n.data.descriptorId, label: n.data.label,
      color: n.data.color, params: structuredClone(n.data.params),
      inputParams: [...(n.data.inputParams ?? [])], disabled: n.data.disabled,
      position: { ...n.position },
    })),
    edges: g.edges.filter((e) => ids.has(e.source) && ids.has(e.target)).map((e) => ({
      source: e.source, sourceHandle: e.sourceHandle,
      target: e.target, targetHandle: e.targetHandle, type: e.type,
    })),
  };
}

export function copySelection(): boolean {
  const clip = selectedSubgraph();
  if (!clip) return false;
  clipboard = clip;
  pasteOffset = 0;
  return true;
}

export function cutSelection() {
  if (copySelection()) useGraphStore.getState().deleteSelection();
}

export function pasteSelection(position?: { x: number; y: number }) {
  if (!clipboard) return;
  pasteOffset += 32;
  const dx = position ? position.x - Math.min(...clipboard.nodes.map((n) => n.position.x)) : pasteOffset;
  const dy = position ? position.y - Math.min(...clipboard.nodes.map((n) => n.position.y)) : pasteOffset;
  useGraphStore.getState().paste(clipboard, dx, dy);
}

export function duplicateSelection() {
  const clip = selectedSubgraph();
  if (clip) useGraphStore.getState().paste(clip, 32, 32);
}

export const hasGraphClipboard = () => clipboard !== null;
