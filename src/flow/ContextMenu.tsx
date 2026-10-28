import { useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

export interface MenuItem {
  label: string;
  onClick: () => void;
  danger?: boolean;
  disabled?: boolean;
  shortcut?: string;
  separatorBefore?: boolean;
}

interface Props {
  x: number;
  y: number;
  items: MenuItem[];
  onClose: () => void;
  label?: string;
  preserveFocus?: boolean;
}

/** Shared menu, above dialogs and outside transformed canvas containers. */
export function ContextMenu({ x, y, items, onClose, label = "画布操作", preserveFocus = false }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const itemsRef = useRef(items);
  itemsRef.current = items;
  const [active, setActive] = useState(0);
  const activeRef = useRef(0);
  const [position, setPosition] = useState({ left: x, top: y });
  useLayoutEffect(() => {
    const menu = ref.current!;
    const previousFocus = document.activeElement;
    const rect = menu.getBoundingClientRect();
    setPosition({ left: Math.max(8, Math.min(x, window.innerWidth - rect.width - 8)),
      top: Math.max(8, Math.min(y, window.innerHeight - rect.height - 8)) });
    const buttons = () => Array.from(menu.querySelectorAll<HTMLButtonElement>('button:not(:disabled)'));
    const activate = (index: number) => {
      activeRef.current = index; setActive(index);
      const button = menu.querySelector<HTMLButtonElement>(`[data-menu-index="${index}"]`);
      if (!preserveFocus) (button ?? menu).focus({ preventScroll: true });
      button?.scrollIntoView({ block: 'nearest' });
    };
    // Focusing a menu must not blur/unmount an inline rename field or commit a
    // half-edited number. Text menus retain editor focus and track navigation.
    activate(Math.max(0, itemsRef.current.findIndex((item) => !item.disabled)));
    const close = (e: KeyboardEvent) => {
      if (e.isComposing || ['Control', 'Meta', 'Shift', 'Alt'].includes(e.key)) return;
      const shortcut = e.ctrlKey || e.metaKey ? `Ctrl+${e.key.toUpperCase()}` : e.key;
      const item = itemsRef.current.find((it) => it.shortcut === shortcut && !it.disabled);
      if (!e.altKey && !e.shiftKey && item) {
        e.preventDefault(); e.stopImmediatePropagation();
        if (!e.repeat) { onClose(); item.onClick(); }
        return;
      }
      if (e.key === 'Escape' || e.key === 'Tab') {
        e.preventDefault(); e.stopImmediatePropagation(); onClose();
      } else if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(e.key)) {
        e.preventDefault(); e.stopImmediatePropagation();
        const enabled = buttons(), index = enabled.findIndex((button) => Number(button.dataset.menuIndex) === activeRef.current);
        const next = e.key === 'Home' ? 0 : e.key === 'End' ? enabled.length - 1 :
          (index + (e.key === 'ArrowDown' ? 1 : -1) + enabled.length) % enabled.length;
        if (enabled[next]) activate(Number(enabled[next].dataset.menuIndex));
      } else if (preserveFocus && (e.key === 'Enter' || e.key === ' ')) {
        e.preventDefault(); e.stopImmediatePropagation();
        const selected = itemsRef.current[activeRef.current];
        if (!e.repeat && selected && !selected.disabled) { onClose(); selected.onClick(); }
      } else if (preserveFocus) {
        onClose(); // Resume ordinary typing in the still-focused editor.
      }
    };
    const outside = (e: PointerEvent) => { if (!menu.contains(e.target as Node)) onClose(); };
    const scroll = (e: Event) => { if (!menu.contains(e.target as Node)) onClose(); };
    window.addEventListener('keydown', close, true);
    document.addEventListener('pointerdown', outside, true);
    document.addEventListener('scroll', scroll, true);
    window.addEventListener('resize', onClose);
    window.addEventListener('blur', onClose);
    return () => {
      window.removeEventListener('keydown', close, true);
      document.removeEventListener('pointerdown', outside, true);
      document.removeEventListener('scroll', scroll, true);
      window.removeEventListener('resize', onClose);
      window.removeEventListener('blur', onClose);
      if ((menu.contains(document.activeElement) || document.activeElement === document.body) &&
        previousFocus instanceof HTMLElement && previousFocus.isConnected) previousFocus.focus({ preventScroll: true });
    };
  }, [x, y, onClose, preserveFocus]);

  return createPortal(
      <div
        ref={ref} role="menu" aria-label={label} tabIndex={-1} data-canvas-shortcuts="blocked"
        className="fixed z-[200] min-w-[230px] max-w-[calc(100vw-16px)] overflow-y-auto rounded-md border border-border bg-popover p-1 text-xs text-popover-foreground shadow-lg outline-none"
        style={{ ...position, maxHeight: 'calc(100vh - 16px)' }}
        onContextMenu={(e) => { e.preventDefault(); e.stopPropagation(); }}
        onPointerDown={(e) => e.stopPropagation()}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="px-2 py-1 text-[10px] text-muted-foreground">{label}</div>
        {items.map((it, index) => (
          <div key={it.label}>
          {it.separatorBefore && <div role="separator" className="my-1 border-t border-border" />}
          <button
            type="button" tabIndex={-1}
            role="menuitem"
            aria-label={it.label}
            data-menu-index={index}
            data-active={active === index || undefined}
            disabled={it.disabled}
            onMouseEnter={() => { if (!it.disabled) { activeRef.current = index; setActive(index); } }}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => {
              onClose();
              it.onClick();
            }}
            className={`flex w-full items-center justify-between gap-4 rounded px-2 py-1.5 text-left outline-none transition-colors hover:bg-accent focus:bg-accent disabled:opacity-40 ${active === index ? 'bg-accent' : ''} ${
              it.danger ? "text-destructive hover:bg-destructive/10" : ""
            }`}
          >
            {it.label}
            {it.shortcut && <kbd className="text-[10px] text-muted-foreground">{it.shortcut}</kbd>}
          </button>
          </div>
        ))}
      </div>, document.body
  );
}
