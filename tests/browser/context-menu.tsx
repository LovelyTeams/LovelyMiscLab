import { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { ReactFlowProvider } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import '@/index.css';
import { Canvas } from '@/flow/Canvas';
import { KeyboardShortcuts } from '@/app/KeyboardShortcuts';
import { GlobalContextMenu } from '@/app/GlobalContextMenu';
import { OutputValue } from '@/flow/portValue';
import { captureTextContext, replaceContextText, selectContextText } from '@/lib/textInteraction';
import { writeClipboardText, readClipboardText } from '@/lib/clipboard';
import { useGraphStore as G } from '@/store/graph';
import { useDescriptorStore as D } from '@/store/descriptors';
import catalog from '@/lib/nodeCatalog.generated.json';

// Isolated fixture: no model calls, project persistence or real settings access.
D.getState().setDescriptors(catalog as never);
const source = G.getState().addNode(D.getState().byId.text_input, { x: 30, y: 30 });
const output = G.getState().addNode(D.getState().byId.text_output, { x: 320, y: 30 });
G.getState().setParam(source, 'text', 'canvas 文本 αβ');
G.getState().onConnect({ source, sourceHandle: 'text', target: output, targetHandle: 'text' });
G.getState().updateRuntime(output, { status: 'done', outputs: { value: { type: 'text', value: 'flag{copy_中文_αβ}' } } });
G.getState().selectNode(output);
const tick = () => new Promise<void>((resolve) => setTimeout(resolve, 20));
const key = (target: Element, value: string, extra: KeyboardEventInit = {}) => {
  const event = new KeyboardEvent('keydown', { key: value, bubbles: true, cancelable: true, ...extra });
  target.dispatchEvent(event); return event;
};
const context = async (target: Element, x = 80, y = 100) => {
  target.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: x, clientY: y }));
  await tick();
};
const dismiss = async () => { key(document.activeElement!, 'Escape'); await tick(); };
const item = (label: string) => document.querySelector<HTMLButtonElement>(`[role="menuitem"][aria-label="${label}"]`)!;

function Harness() {
  const [text, setText] = useState('剪贴板测试 hello 中文 😀');
  const [report, setReport] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [modal, setModal] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const nodes = G((s) => s.nodes.length);
  const suite = async () => {
    setBusy(true); setReport([]);
    const results: string[] = [];
    const check = (ok: unknown, name: string) => { if (!ok) throw new Error(name); results.push('通过：' + name); };
    try {
      G.getState().updateRuntime(output, { status: 'done', outputs: { value: { type: 'text', value: 'flag{copy_中文_αβ}' } } });
      await tick();
      const field = document.querySelector<HTMLTextAreaElement>('#fixture-editor')!;
      const canvas = document.querySelector<HTMLElement>('[data-canvas-commands]')!;
      const node = document.querySelector<HTMLElement>(`[data-id="${output}"]`)!;
      const preview = node.querySelector('pre')!;
      const sidebar = document.querySelector<HTMLElement>('#fixture-sidebar')!;
      const before = G.getState().nodes.length;
      field.focus(); field.select();
      for (const command of ['c', 'x', 'v', 'a', 'z', 'y']) check(!key(field, command, { ctrlKey: true }).defaultPrevented, '输入框保留 Ctrl+' + command.toUpperCase());
      field.blur(); canvas.focus();
      const range = document.createRange(); range.selectNodeContents(preview);
      document.getSelection()!.removeAllRanges(); document.getSelection()!.addRange(range);
      check(!key(canvas, 'c', { ctrlKey: true }).defaultPrevented, '结果选中文字不被节点复制拦截');
      key(canvas, 'Delete'); check(G.getState().nodes.length === before, '选中文字时 Delete 不删除节点');
      const position = JSON.stringify(G.getState().nodes.map((n) => n.position));
      key(node, 'ArrowRight'); check(position === JSON.stringify(G.getState().nodes.map((n) => n.position)), '选中文字时方向键不移动节点');
      await context(preview); check(!!item('复制此处文本'), '节点结果使用文本菜单'); await dismiss();
      document.getSelection()!.removeAllRanges();
      sidebar.focus(); check(!key(sidebar, 'a', { ctrlKey: true }).defaultPrevented, '侧栏 Ctrl+A 不全选节点');
      canvas.focus(); G.getState().selectNode(output);
      check(key(canvas, 'c', { ctrlKey: true }).defaultPrevented, '画布 Ctrl+C 仍复制节点');
      key(canvas, 'v', { ctrlKey: true }); await tick();
      check(G.getState().nodes.length === before + 1, '画布 Ctrl+V 粘贴节点');
      key(canvas, 'z', { ctrlKey: true }); await tick(); check(G.getState().nodes.length === before, '画布撤销粘贴');
      const nodeField = document.querySelector<HTMLTextAreaElement>(`[data-id="${source}"] textarea`)!;
      nodeField.focus(); nodeField.setSelectionRange(0, 6);
      await context(nodeField); check(!!document.querySelector('[role="menu"][aria-label="文本编辑"]'), '节点参数右键没有被节点菜单覆盖');
      check(!item('复制').disabled, '菜单保留输入框原选择'); await dismiss();
      const readOnly = document.querySelector<HTMLTextAreaElement>('#fixture-readonly')!;
      readOnly.focus(); readOnly.select(); await context(readOnly);
      check(!item('复制').disabled && item('剪切').disabled && item('粘贴').disabled, '只读文本允许复制并禁用修改'); await dismiss();
      const password = document.querySelector<HTMLInputElement>('#fixture-password')!;
      password.focus(); password.select(); await context(password);
      check(item('复制').disabled && item('剪切').disabled && !item('粘贴').disabled, '密码框保留原生复制限制并允许粘贴'); await dismiss();
      const numeric = document.querySelector<HTMLInputElement>('#fixture-number')!;
      const nativeMenu = new MouseEvent('contextmenu', { bubbles: true, cancelable: true });
      numeric.dispatchEvent(nativeMenu); await tick();
      check(!nativeMenu.defaultPrevented && !document.querySelector('[role="menu"]'), '数值等特殊控件保留原生菜单');
      field.focus(); field.setSelectionRange(0, 3); await context(field, window.innerWidth - 1, window.innerHeight - 1);
      const rect = document.querySelector('[role="menu"]')!.getBoundingClientRect();
      check(rect.right <= window.innerWidth && rect.bottom <= window.innerHeight && rect.left >= 0 && rect.top >= 0, '菜单在窗口边界内');
      check(document.activeElement === field, '文本菜单不触发输入框失焦提交');
      key(document.activeElement!, 'End'); await tick(); check(item('全选').dataset.active === 'true', 'End 定位末项');
      key(document.activeElement!, 'Home'); await tick(); check(item('剪切').dataset.active === 'true', 'Home 定位首项');
      await dismiss(); check(document.activeElement === field, 'Esc 关闭菜单并恢复输入焦点');
      key(field, 'F10', { shiftKey: true }); await tick(); check(!!document.querySelector('[role="menu"]'), 'Shift+F10 打开菜单');
      item('全选').click(); await tick(); check(field.selectionStart === 0 && field.selectionEnd === field.value.length, '菜单全选恢复输入框选择');
      await context(field); key(document.activeElement!, 'Control'); await tick();
      check(!!document.querySelector('[role="menu"]'), '按修饰键不会提前关闭菜单');
      key(document.activeElement!, 'a', { ctrlKey: true }); await tick();
      check(!document.querySelector('[role="menu"]') && field.selectionEnd === field.value.length, '菜单加速键全选文字');
      field.setSelectionRange(0, 3); const captured = captureTextContext(field);
      replaceContextText(captured, 'REPLACE'); await tick();
      check(field.value.startsWith('REPLACE') && document.querySelector('#fixture-value')!.textContent === field.value, '菜单编辑同步 React 状态');
      let staleBlocked = false;
      try { replaceContextText(captured, 'late'); } catch { staleBlocked = true; }
      check(staleBlocked, '迟到的剪切或粘贴不覆盖已修改文字');
      const movedSelection = captureTextContext(field);
      field.setSelectionRange(0, 0); let movedBlocked = false;
      try { replaceContextText(movedSelection, 'late'); } catch { movedBlocked = true; }
      check(movedBlocked, '剪贴板读取期间移动光标后不覆盖原选区');
      document.querySelector<HTMLButtonElement>('#fixture-rename-button')!.click(); await tick();
      const rename = document.querySelector<HTMLInputElement>('#fixture-rename')!;
      rename.focus(); rename.select(); await context(rename);
      check(rename.isConnected && document.activeElement === rename, '右键不会结束行内重命名');
      key(rename, 'ArrowDown'); key(rename, 'Escape'); await tick();
      const sidebarPreview = sidebar.querySelector('pre')!;
      selectContextText(captureTextContext(sidebarPreview));
      check(document.getSelection()!.toString() === sidebarPreview.textContent, '结果全选仅选择当前文本区域');
      document.getSelection()!.removeAllRanges();
      G.getState().updateRuntime(output, { status: 'done', outputs: { value: { type: 'text', value: 'flag{copy_中文_αβ}' } } });
      results.push('交互回归全部通过；系统剪贴板请继续手动验证。');
    } catch (error) { results.push('失败：' + (error instanceof Error ? error.message : String(error))); }
    finally { setReport(results); setBusy(false); }
  };
  const clipboardFailures = async () => {
    setBusy(true); setReport([]);
    const original = Object.getOwnPropertyDescriptor(navigator, 'clipboard');
    const command = document.execCommand;
    const results: string[] = [];
    const check = (ok: unknown, name: string) => { if (!ok) throw new Error(name); results.push('通过：' + name); };
    try {
      const field = document.querySelector<HTMLTextAreaElement>('#fixture-editor')!;
      field.focus(); field.setSelectionRange(1, 4);
      Object.defineProperty(navigator, 'clipboard', { configurable: true, value: {
        writeText: async () => { throw new Error('fixture permission denied'); },
        readText: async () => { throw new Error('fixture permission denied'); },
      } });
      let copied = '';
      document.execCommand = (name, show, value) => {
        if (name === 'copy') { copied = (document.activeElement as HTMLTextAreaElement).value; return true; }
        return command.call(document, name, show, value);
      };
      await writeClipboardText('legacy 中文😀\nline 2');
      check(copied === 'legacy 中文😀\nline 2', '浏览器剪贴板拒绝时使用兼容复制');
      check(document.activeElement === field && field.selectionStart === 1 && field.selectionEnd === 4, '兼容复制恢复输入焦点与选区');
      let rejected = false;
      try { await readClipboardText(); } catch (error) { rejected = String(error).includes('Ctrl+V'); }
      check(rejected, '读取失败给出可操作的快捷键提示');
      const before = field.value;
      await context(field);
      document.execCommand = () => false;
      item('剪切').click(); await tick();
      check(field.value === before, '复制失败时剪切不删除原文');
      check(document.querySelector('[role="status"]')?.textContent?.includes('复制失败'), '复制失败在页面显示');
      field.focus(); field.setSelectionRange(0, 1); const snapshot = captureTextContext(field);
      replaceContextText(snapshot, 'fallback'); await tick();
      check(field.value.startsWith('fallback') && document.querySelector('#fixture-value')!.textContent === field.value, '原生编辑命令不可用时仍同步 React 参数');
      results.push('剪贴板异常回归全部通过。');
    } catch (error) { results.push('失败：' + (error instanceof Error ? error.message : String(error))); }
    finally {
      document.execCommand = command;
      if (original) Object.defineProperty(navigator, 'clipboard', original); else delete (navigator as { clipboard?: unknown }).clipboard;
      setReport(results); setBusy(false);
    }
  };
  return <ReactFlowProvider><main className="flex h-screen flex-col bg-background text-foreground">
    <header className="flex items-center gap-4 border-b p-3 text-sm">
      <strong>右键与剪贴板回归</strong><button disabled={busy} onClick={() => void suite()}>{busy ? '测试中' : '运行交互回归'}</button>
      <button disabled={busy} onClick={() => void clipboardFailures()}>运行剪贴板异常回归</button>
      <button onClick={() => setModal(true)}>打开测试弹窗</button><span>节点数：{nodes}</span>
    </header>
    <div className="flex min-h-0 flex-1">
      <aside id="fixture-sidebar" tabIndex={0} className="w-96 space-y-3 overflow-auto border-r p-3 text-xs">
        <label className="block">编辑文字<textarea id="fixture-editor" aria-label="编辑文字" className="block h-20 w-full border p-2" value={text} onChange={(e) => setText(e.target.value)} /></label>
        <div>受控值：<span id="fixture-value">{text}</span></div>
        <label className="block">只读文字<textarea id="fixture-readonly" aria-label="只读文字" className="block w-full border p-2" readOnly value="只读内容 keep unchanged" /></label>
        <label className="block">测试密码<input id="fixture-password" aria-label="测试密码" className="block w-full border p-2" type="password" defaultValue="fixture-only" /></label>
        <label className="block">测试数值<input id="fixture-number" aria-label="测试数值" className="block w-full border p-2" type="number" defaultValue="42" /></label>
        {renaming ? <input id="fixture-rename" aria-label="临时名称" defaultValue="临时名称" onBlur={() => setRenaming(false)} className="border p-2" /> : <button id="fixture-rename-button" onClick={() => setRenaming(true)}>编辑临时名称</button>}
        <div role="textbox" aria-label="富文本" contentEditable suppressContentEditableWarning className="border p-2">富文本 editable 中文</div>
        <OutputValue value={{ type: 'text', value: '结果文字 copy_result_中文😀' }} />
        <div role="log" aria-label="回归报告" className="space-y-1">{report.map((line, i) => <p key={i}>{line}</p>)}</div>
      </aside>
      <div className="min-w-0 flex-1"><Canvas /></div>
    </div>
    {modal && <div className="fixed inset-0 z-[90] flex items-center justify-center bg-black/30"><div role="dialog" aria-modal="true" aria-label="测试弹窗" className="space-y-4 rounded bg-card p-8">
      <textarea aria-label="弹窗输入" defaultValue="弹窗内文本" className="border p-2" /><button onClick={() => setModal(false)}>关闭测试弹窗</button>
    </div></div>}
    <KeyboardShortcuts /><GlobalContextMenu />
  </main></ReactFlowProvider>;
}
createRoot(document.getElementById('root')!).render(<Harness />);
