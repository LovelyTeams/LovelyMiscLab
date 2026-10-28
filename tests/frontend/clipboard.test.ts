import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readClipboardText, writeClipboardText } from '@/lib/clipboard';

// Exercise the installed Tauri JS transport, without touching the user's clipboard.
async function withClipboard(invoke, clipboard, run) {
  const previous = ['window', 'navigator'].map((name) => [name, Object.getOwnPropertyDescriptor(globalThis, name)]);
  Object.defineProperty(globalThis, 'window', { configurable: true, value: { __TAURI_INTERNALS__: { invoke } } });
  Object.defineProperty(globalThis, 'navigator', { configurable: true, value: { clipboard } });
  try { await run(); }
  finally { for (const [name, descriptor] of previous) {
    if (descriptor) Object.defineProperty(globalThis, name, descriptor); else delete globalThis[name];
  } }
}

test('desktop clipboard uses native text commands and preserves Unicode and line breaks', async () => {
  const calls = [], text = '中文 😀\r\nsecond line\tαβ';
  await withClipboard(async (command, args) => { calls.push([command, args]); return text; }, {
    readText: () => { throw new Error('browser path must not run'); },
    writeText: () => { throw new Error('browser path must not run'); },
  }, async () => {
    await writeClipboardText(text);
    assert.equal(await readClipboardText(), text);
    assert.deepEqual(calls, [['plugin:clipboard-manager|write_text', { text, label: undefined }], ['plugin:clipboard-manager|read_text', {}]]);
  });
});

test('older desktop shells fall back to browser clipboard without losing the payload', async () => {
  const values = [];
  await withClipboard(async () => { throw new Error('plugin missing'); }, {
    writeText: async (value) => { values.push(value); }, readText: async () => '',
  }, async () => {
    await writeClipboardText('fallback 中文');
    assert.deepEqual(values, ['fallback 中文']);
    assert.equal(await readClipboardText(), '');
  });
});

test('clipboard read rejection is actionable and does not expose native error details', async () => {
  await withClipboard(async () => { throw new Error('native details'); }, {
    readText: async () => { throw new Error('permission details'); },
  }, async () => { await assert.rejects(readClipboardText(), /^Error: 无法读取剪贴板，请在输入框中按 Ctrl\+V 粘贴$/); });
});
