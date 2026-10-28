/** Shared focus/selection rules for text menus and canvas keyboard commands. */
export function isEditingTarget(target: EventTarget | null): boolean {
  return target instanceof Element && !!target.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="textbox"]');
}

export function hasTextSelection(doc: Document = document): boolean {
  const selection = doc.getSelection();
  return !!selection && !selection.isCollapsed && selection.toString().length > 0;
}

export function isTextContextTarget(target: EventTarget | null): boolean {
  return isEditingTarget(target) || target instanceof Element && !!target.closest('[data-text-region], pre, code, .select-text');
}

/** Specialized inputs do not expose a restorable text selection (e.g. number,
 * date, file). Keep their native picker/edit menu and native keyboard behavior. */
export function usesNativeContextMenu(target: Element): boolean {
  if (target.closest('select, option, audio, video')) return true;
  const input = target.closest('input');
  return input instanceof HTMLInputElement && input.selectionStart === null;
}

type TextField = HTMLInputElement | HTMLTextAreaElement;
export interface TextContext {
  target: HTMLElement;
  field: TextField | null;
  editor: HTMLElement | null;
  region: HTMLElement | null;
  text: string;
  writable: boolean;
  password: boolean;
  value: string;
  start: number | null;
  end: number | null;
  range: Range | null;
}

/** Take the selection before a menu receives focus, including input selections. */
export function captureTextContext(target: Element): TextContext {
  const doc = target.ownerDocument;
  const candidate = target.closest('input, textarea');
  const field = candidate instanceof HTMLTextAreaElement || candidate instanceof HTMLInputElement &&
    ['text', 'search', 'url', 'tel', 'email', 'password', 'number'].includes(candidate.type) ? candidate : null;
  const editable = target.closest<HTMLElement>('[contenteditable]');
  const editor = editable?.isContentEditable ? editable : null;
  const selection = doc.getSelection();
  const range = selection?.rangeCount ? selection.getRangeAt(0).cloneRange() : null;
  const password = field instanceof HTMLInputElement && field.type === 'password';
  const start = field?.selectionStart ?? null;
  const end = field?.selectionEnd ?? null;
  const text = password ? '' : field ? start !== null && end !== null ? field.value.slice(start, end) :
    doc.activeElement === field ? selection?.toString() ?? '' : '' : selection?.toString() ?? '';
  return {
    target: (target instanceof HTMLElement ? target : target.parentElement) ?? doc.body,
    field, editor, password, text, start, end, range,
    value: field?.value ?? editor?.innerHTML ?? '',
    writable: field ? !field.readOnly && !field.disabled : !!editor,
    region: field || editor ? null : target.closest<HTMLElement>('[data-text-region], pre, p, td, th, code'),
  };
}

export function restoreTextContext(ctx: TextContext) {
  const { field, editor, target, start, end, range } = ctx;
  if (!target.isConnected) return;
  const doc = target.ownerDocument;
  (field ?? editor)?.focus({ preventScroll: true });
  if (field && start !== null && end !== null) {
    field.setSelectionRange(start, end);
  } else if (range?.startContainer.isConnected && range.endContainer.isConnected) {
    const selection = doc.getSelection();
    selection?.removeAllRanges();
    selection?.addRange(range);
  }
}

export function selectContextText(ctx: TextContext) {
  if (!ctx.target.isConnected) return;
  if (ctx.field) {
    ctx.field.focus({ preventScroll: true });
    ctx.field.select();
    return;
  }
  const doc = ctx.target.ownerDocument;
  const region = ctx.editor ?? ctx.region ?? ctx.target.closest('[role="dialog"], main') ?? doc.getElementById('root') ?? doc.body;
  ctx.editor?.focus({ preventScroll: true });
  const range = doc.createRange();
  range.selectNodeContents(region);
  doc.getSelection()?.removeAllRanges();
  doc.getSelection()?.addRange(range);
}

/** Preserve native edit undo and React onChange. Never paste a late response into a changed field. */
export function replaceContextText(ctx: TextContext, text: string) {
  const { field, editor } = ctx;
  if (!ctx.writable || !ctx.target.isConnected || (field && (field.readOnly || field.disabled)) || (editor && !editor.isContentEditable) ||
    (field ? field.value : editor?.innerHTML) !== ctx.value ||
    (field && (field.selectionStart !== ctx.start || field.selectionEnd !== ctx.end))) throw new Error('文字或选区已变化，请重新选择后操作');
  restoreTextContext(ctx);
  const doc = ctx.target.ownerDocument;
  // insertText/delete keep the WebView editing undo stack and send a real input event.
  if (doc.execCommand(text ? 'insertText' : 'delete', false, text)) return;
  // Some WebViews reject editing commands. Apply through the native value setter
  // so React's value tracker still observes the input event.
  if (field && ctx.start !== null && ctx.end !== null) {
    const inserted = field.maxLength >= 0 ? text.slice(0, Math.max(0, field.maxLength - field.value.length + ctx.end - ctx.start)) : text;
    const value = field.value.slice(0, ctx.start) + inserted + field.value.slice(ctx.end);
    const prototype = field instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
    Object.getOwnPropertyDescriptor(prototype, 'value')!.set!.call(field, value);
    field.setSelectionRange(ctx.start + inserted.length, ctx.start + inserted.length);
    field.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: text ? 'insertText' : 'deleteContentBackward', data: inserted }));
    return;
  }
  throw new Error('此控件不支持菜单编辑，请使用键盘快捷键');
}
