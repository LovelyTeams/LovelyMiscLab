import { useEffect } from "react";
import { useReactFlow } from "@xyflow/react";

import { newFlow, openFlow, saveFlow } from "@/lib/project";
import { useCommandPaletteStore } from "@/store/commandPalette";
import { useViewStore } from "@/store/view";
import { useGraphStore } from "@/store/graph";
import { useAgentStore } from "@/store/agent";
import { canvasOverlaySelector, canvasShortcut, isEditingTarget } from "@/flow/canvasShortcuts";
import { copySelection, cutSelection, duplicateSelection, pasteSelection } from "@/flow/graphClipboard";

export function KeyboardShortcuts() {
  const rf = useReactFlow();
  useEffect(() => {
    const endMove = () => useGraphStore.getState().endHistoryGroup("moveSelection");
    const keyup = (e: KeyboardEvent) => { if (e.key.startsWith("Arrow")) endMove(); };
    const handler = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.isComposing || e.altKey) return;
      // A modal or floating picker owns its keyboard, even when a button has focus.
      if (document.querySelector(canvasOverlaySelector)) return;
      const mod = e.ctrlKey || e.metaKey;
      const key = e.key.toLowerCase();
      if (mod && ["s", "o", "n", "k"].includes(key)) {
        e.preventDefault(); e.stopPropagation();
        if (e.repeat) return;
        if (key === "k") useCommandPaletteStore.getState().toggle();
        else if (key === "s") void saveFlow(e.shiftKey);
        else if (key === "o") void openFlow();
        else newFlow();
        return;
      }
      if (isEditingTarget(e.target) || useViewStore.getState().view !== "canvas") return;
      const action = canvasShortcut(e);
      if (!action) return;
      e.preventDefault(); e.stopPropagation();
      const g = useGraphStore.getState();
      const move = ({ left: [-1, 0], right: [1, 0], up: [0, -1], down: [0, 1] } as Record<string, number[]>)[action];
      if (move) {
        if (g.nodes.some((node) => node.selected)) useAgentStore.getState().focusNodes([]);
        g.beginHistoryGroup("moveSelection");
        const step = e.shiftKey ? 10 : 1;
        g.moveSelection(move[0] * step, move[1] * step);
        return;
      }
      endMove();
      if (e.repeat) return;
      switch (action) {
        case "copy": copySelection(); break;
        case "cut": cutSelection(); break;
        case "paste": pasteSelection(); break;
        case "duplicate": duplicateSelection(); break;
        case "selectAll": g.selectAll(); break;
        case "undo": g.undo(); break;
        case "redo": g.redo(); break;
        case "delete": g.deleteSelection(); break;
        case "deselect": g.deselectAll(); break;
        case "rename": {
          const selected = g.nodes.filter((n) => n.selected);
          if (selected.length !== 1) break;
          g.setSelected(selected[0].id);
          requestAnimationFrame(() => {
            const field = document.querySelector<HTMLInputElement>("[data-node-name]");
            field?.focus(); field?.select();
          });
          break;
        }
        case "fit": void rf.fitView({ duration: 200, padding: 0.15 }); break;
        case "actualSize": void rf.zoomTo(1, { duration: 200 }); break;
        case "zoomIn": void rf.zoomIn({ duration: 150 }); break;
        case "zoomOut": void rf.zoomOut({ duration: 150 }); break;
      }
    };
    // Capture prevents React Flow's node keyboard handler from moving a node twice.
    window.addEventListener("keydown", handler, true);
    window.addEventListener("keyup", keyup);
    window.addEventListener("blur", endMove);
    return () => {
      window.removeEventListener("keydown", handler, true);
      window.removeEventListener("keyup", keyup);
      window.removeEventListener("blur", endMove);
      endMove();
    };
  }, [rf]);
  return null;
}
