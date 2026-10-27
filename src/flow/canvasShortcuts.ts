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
    if (key === "z") return e.shiftKey ? "redo" : "undo";
    if (key === "+" || key === "=") return "zoomIn";
    if (key === "-") return "zoomOut";
    if (e.shiftKey) return null;
    return modifiedKeys[key] ?? null;
  }
  return plainKeys[key] ?? null;
}

export function isEditingTarget(target: EventTarget | null): boolean {
  return target instanceof Element && !!target.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="textbox"]');
}

export const canvasOverlaySelector = '[role="dialog"], [aria-modal="true"], [data-canvas-shortcuts="blocked"]';
