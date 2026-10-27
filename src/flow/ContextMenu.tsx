import { useEffect } from "react";

export interface MenuItem {
  label: string;
  onClick: () => void;
  danger?: boolean;
  disabled?: boolean;
  shortcut?: string;
}

interface Props {
  x: number;
  y: number;
  items: MenuItem[];
  onClose: () => void;
}

/** A cursor-positioned context menu with a full-screen backdrop to dismiss. */
export function ContextMenu({ x, y, items, onClose }: Props) {
  const left = Math.max(8, Math.min(x, window.innerWidth - 250));
  const top = Math.max(8, Math.min(y, window.innerHeight - (items.length * 32 + 16)));
  useEffect(() => {
    const close = (e: KeyboardEvent) => { if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); onClose(); } };
    window.addEventListener("keydown", close);
    return () => window.removeEventListener("keydown", close);
  }, [onClose]);

  return (
    <>
      <div
        className="fixed inset-0 z-40"
        onClick={onClose}
        onContextMenu={(e) => {
          e.preventDefault();
          onClose();
        }}
      />
      <div
        role="menu" aria-label="画布操作" data-canvas-shortcuts="blocked"
        className="fixed z-50 min-w-[230px] rounded-md border border-border bg-popover p-1 text-xs text-popover-foreground shadow-lg"
        style={{ left, top }}
      >
        {items.map((it, i) => (
          <button
            key={i}
            role="menuitem"
            disabled={it.disabled}
            onClick={() => {
              it.onClick();
              onClose();
            }}
            className={`flex w-full items-center justify-between gap-4 rounded px-2 py-1.5 text-left transition-colors hover:bg-accent disabled:opacity-40 ${
              it.danger ? "text-destructive hover:bg-destructive/10" : ""
            }`}
          >
            {it.label}
            {it.shortcut && <kbd className="text-[10px] text-muted-foreground">{it.shortcut}</kbd>}
          </button>
        ))}
      </div>
    </>
  );
}
