import { hasTextSelection, isEditingTarget } from '@/lib/textInteraction';
export { isEditingTarget } from '@/lib/textInteraction';

export type CanvasAction = "copy" | "cut" | "paste" | "duplicate" | "selectAll" |
  "undo" | "redo" | "delete" | "deselect" | "rename" | "fit" | "actualSize" |
  "zoomIn" | "zoomOut" | "left" | "right" | "up" | "down";

type KeyStroke = Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "altKey" | "shiftKey" | "isComposing">;
const modifiedKeys: Record<string, CanvasAction> = { c: "copy", x: "cut", v: "paste", d: "duplicate", a: "selectAll", y: "redo", "0": "actualSize" };
const plainKeys: Record<string, CanvasAction> = { delete: "delete", backspace: "delete", escape: "deselect", f2: "rename", home: "fit",
  arrowleft: "left", arrowright: "right", arrowup: "up", arrowdown: "down" };

/** Resolve only canvas commands; text fields and dialogs are guarded at the call site. */
export function canvasShortcut(e: KeyStroke): CanvasAction | null {
  if (e.isComposing || e.altKey) return null;
  const key = e.key.toLowerCase();
  if (e.ctrlKey || e.metaKey) {
    if (key === "insert" && !e.shiftKey) return "copy";
    if (key === "z") return e.shiftKey ? "redo" : "undo";
    if (key === "+" || key === "=") return "zoomIn";
    if (key === "-") return "zoomOut";
    if (e.shiftKey) return null;
    return modifiedKeys[key] ?? null;
  }
  if (e.shiftKey && key === 'insert') return 'paste';
  if (e.shiftKey && key === 'delete') return 'cut';
  return plainKeys[key] ?? null;
}

/** A canvas view also contains results, an AI conversation and inspectors.
 * Those must never lose native text shortcuts to the graph clipboard. */
export function canvasOwnsKeyboard(target: EventTarget | null): boolean {
  return target instanceof Element && !!target.closest('[data-canvas-commands]') &&
    !isEditingTarget(target) && !hasTextSelection(target.ownerDocument);
}

export const canvasOverlaySelector = '[role="dialog"], [aria-modal="true"], [data-canvas-shortcuts="blocked"]';
