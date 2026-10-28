import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager';
import { inTauri } from './devMocks';

/** Buttons and menus share the OS clipboard on desktop, with a browser fallback. */
export async function writeClipboardText(text: string): Promise<void> {
  if (inTauri) {
    try { await writeText(text); return; } catch { /* Older development shells may not have the plugin yet. */ }
  }
  try { await navigator.clipboard.writeText(text); return; } catch { /* WebView permission / insecure context. */ }
  const active = document.activeElement;
  const field = active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement ? active : null;
  const start = field?.selectionStart, end = field?.selectionEnd;
  const selection = document.getSelection();
  const ranges = selection ? Array.from({ length: selection.rangeCount }, (_, i) => selection.getRangeAt(i).cloneRange()) : [];
  const area = document.createElement('textarea');
  area.value = text;
  area.style.cssText = 'position:fixed;left:-10000px;top:0;opacity:0';
  // A dialog may make the rest of the document inert.
  (active?.closest('[role="dialog"], dialog') ?? document.body).append(area);
  try {
    area.focus({ preventScroll: true }); area.select();
    if (!document.execCommand('copy')) throw new Error('复制失败，请选中文字后按 Ctrl+C');
  } finally {
    area.remove();
    if (active instanceof HTMLElement && active.isConnected) active.focus({ preventScroll: true });
    if (field && start != null && end != null) field.setSelectionRange(start, end);
    else if (selection) { selection.removeAllRanges(); for (const range of ranges) selection.addRange(range); }
  }
}

/** Only called by an explicit Paste action; never read clipboard on menu open. */
export async function readClipboardText(): Promise<string> {
  if (inTauri) {
    try { return await readText(); } catch { /* Browser fallback for an older development shell. */ }
  }
  try { return await navigator.clipboard.readText(); }
  catch { throw new Error('无法读取剪贴板，请在输入框中按 Ctrl+V 粘贴'); }
}
