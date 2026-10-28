import { useCallback, useEffect, useState } from 'react';
import { ContextMenu, type MenuItem } from '@/flow/ContextMenu';
import { readClipboardText, writeClipboardText } from '@/lib/clipboard';
import { captureTextContext, replaceContextText, restoreTextContext, selectContextText, usesNativeContextMenu, type TextContext } from '@/lib/textInteraction';
import { useCommandPaletteStore } from '@/store/commandPalette';
import { useHelpStore } from '@/store/help';
import { useViewStore } from '@/store/view';

type MenuState = { x: number; y: number; context: TextContext };

export function GlobalContextMenu() {
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [notice, setNotice] = useState('');
  const close = useCallback(() => setMenu(null), []);
  const view = useViewStore((s) => s.view);
  useEffect(close, [close, view]);
  useEffect(() => {
    if (!notice) return;
    const timer = setTimeout(() => setNotice(''), 4000);
    return () => clearTimeout(timer);
  }, [notice]);
  useEffect(() => {
    const open = (e: MouseEvent) => {
      // Canvas, node pickers and menus that handled the event retain ownership.
      if (e.defaultPrevented || !(e.target instanceof Element)) return;
      if (usesNativeContextMenu(e.target)) return;
      e.preventDefault();
      const context = captureTextContext(e.target);
      const rect = context.target.getBoundingClientRect();
      setMenu({ x: e.clientX || Math.min(rect.left + 12, window.innerWidth - 24),
        y: e.clientY || Math.min(rect.bottom, window.innerHeight - 24), context });
    };
    const keyboard = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.isComposing || e.altKey || e.ctrlKey || e.metaKey) return;
      if (e.key !== 'ContextMenu' && !(e.shiftKey && e.key === 'F10')) return;
      if (!(e.target instanceof Element) || e.target.closest('[role="menu"]') || usesNativeContextMenu(e.target)) return;
      e.preventDefault();
      const rect = e.target.getBoundingClientRect();
      e.target.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true,
        clientX: Math.min(rect.left + 12, window.innerWidth - 24), clientY: Math.min(rect.bottom, window.innerHeight - 24) }));
    };
    document.addEventListener('contextmenu', open);
    window.addEventListener('keydown', keyboard);
    return () => { document.removeEventListener('contextmenu', open); window.removeEventListener('keydown', keyboard); };
  }, []);

  const perform = (operation: () => void | Promise<void>) => {
    setNotice('');
    void Promise.resolve().then(operation).catch((e: unknown) => setNotice(e instanceof Error ? e.message : String(e)));
  };
  const items: MenuItem[] = [];
  if (menu) {
    const ctx = menu.context;
    const editing = !!(ctx.field || ctx.editor);
    const copy = () => perform(async () => { restoreTextContext(ctx); await writeClipboardText(ctx.text); setNotice('已复制文字'); });
    if (editing) items.push(
      { label: '剪切', shortcut: 'Ctrl+X', disabled: !ctx.writable || !ctx.text || ctx.password,
        onClick: () => perform(async () => {
          restoreTextContext(ctx); await writeClipboardText(ctx.text);
          if (document.activeElement !== (ctx.field ?? ctx.editor)) throw new Error('输入焦点已变化，请重新剪切');
          replaceContextText(ctx, '');
        }) },
    );
    items.push({ label: '复制', shortcut: 'Ctrl+C', disabled: !ctx.text || ctx.password, onClick: copy });
    if (editing) items.push(
      { label: '粘贴', shortcut: 'Ctrl+V', disabled: !ctx.writable, onClick: () => perform(async () => {
        restoreTextContext(ctx);
        const text = await readClipboardText();
        if (document.activeElement !== (ctx.field ?? ctx.editor)) throw new Error('输入焦点已变化，请重新粘贴');
        if (text) replaceContextText(ctx, text);
      }) },
      { label: '删除选中文字', shortcut: 'Delete', disabled: !ctx.writable || !(ctx.text || ctx.password && ctx.start !== ctx.end),
        onClick: () => perform(() => replaceContextText(ctx, '')) },
    );
    if (ctx.region) items.push({ label: '复制此处文本', disabled: !ctx.region.textContent, onClick: () => perform(async () => {
      await writeClipboardText(ctx.region!.innerText); setNotice('已复制文字');
    }) });
    items.push({ label: '全选', shortcut: 'Ctrl+A', separatorBefore: true,
      disabled: ctx.field?.disabled, onClick: () => selectContextText(ctx) });
    if (!editing && !ctx.target.closest('[role="dialog"], [aria-modal="true"]')) items.push(
      { label: '搜索模块与命令…', shortcut: 'Ctrl+K', separatorBefore: true, onClick: () => useCommandPaletteStore.getState().setOpen(true) },
      { label: '帮助与快捷键', onClick: () => useHelpStore.getState().setOpen(true) },
    );
  }
  return <>
    {menu && <ContextMenu x={menu.x} y={menu.y} label={menu.context.field || menu.context.editor ? '文本编辑' : '常用操作'}
      preserveFocus={!!(menu.context.field || menu.context.editor)} items={items} onClose={close} />}
    {notice && <div role="status" className="pointer-events-none fixed bottom-12 left-1/2 z-[210] -translate-x-1/2 rounded-lg border border-border bg-popover px-4 py-2 text-xs text-popover-foreground shadow-lg">{notice}</div>}
  </>;
}
