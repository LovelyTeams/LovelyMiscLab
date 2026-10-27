// Maintenance oracle capture only; the installed node runs entirely in Rust.
import { execFileSync } from 'node:child_process';
import { writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
const root = resolve('.git/reference/EBCTFCodeBox');
const revision = execFileSync('git', ['-C', root, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
if (revision !== 'ea2bc36dae31c97060f7f2cddad8df5518afad7b') throw Error('Wrong reference revision');
const { evalProgExpr, progCalcRun } = await import(pathToFileURL(resolve(root, 'src/core/progCalc.js')));
const vectors = [];
function capture(input, params) {
  const v = { input, params };
  try {
    v.text = progCalcRun(input, params);
    const { value, ctx } = evalProgExpr(input, params);
    v.unsigned = value.toString(); v.signed = ctx.toSigned(value).toString();
  // The JS lexer may quote half of an emoji in diagnostics. JSON strings for
  // Rust's UTF-8 reader must contain scalar values, not lone UTF-16 surrogates.
  // Keep such diagnostics as escaped JSON text without altering the input.
  } catch (e) { v.error = e.message.isWellFormed() ? e.message : JSON.stringify(e.message); }
  vectors.push(v);
}
const expressions = [
  '0', '1', '-1', '+0xFF', '~0', '~~0xC0DE', '0b1010 | 0o17', '00017 + 0XFF',
  '2 + 3 * 4', '(2+3)*4', '1 | 2 ^ 3 & 5', '1 << 4 + 2', '0xFF & 0x0F',
  '-7 / 2', '-7 % 2', '7 / -2', '-7 % -2', '1 << 31', '1 << 63', '1 << 33',
  '1 << -1', '1 << (0-1)', '-8 >> 1', '-8 >>> 1', '0xFFFFFFFFFFFFFFFF >> 4',
  '0xFFFFFFFFFFFFFFFF >>> 4', '0xFFFFFFFFFFFFFFFF + 1', '0xFFFFFFFFFFFFFFFF * 2',
  'rotl(0x80000000, 1)', 'rotr(1, 1)', 'rotl(1, -1)', 'rotr(1, -1)',
  'rotl(rotr(0x1234,3),3)', 'rotl(0xF1, 8)', 'rotr(0xF1, 0)',
  '2 ** 3 ** 2', '-2 ** 2', '2 ** 64', '0 ** 0', '2 ** -1',
  '3 ** 0x7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF', '\uFEFF1　+\t2\n',
];
for (const width of [1, 8, 12, 16, 24, 32, 64, 127, 256, 512]) {
  for (const signed of [true, false]) for (const input of expressions) capture(input, { width, signed });
}
for (const input of ['', '中文', '1😀', '1 +', '1 +* 2', '((1)', '1)', '1,2', '0x', '0b2', '0o9', '123abc', 'eval(1)', 'process.exit()', 'ROTL(1,2)', 'rotl(1)', 'rotl(1,2,3)', '1/0', '1%0', '1/256', '('.repeat(33) + '1' + ')'.repeat(33), '1'.repeat(1025)]) capture(input, { width: 8, signed: true });
for (const width of [-1, 513, 2.5]) capture('1', { width });
capture('('.repeat(32) + '1' + ')'.repeat(32), { width: 32 });
capture(Array(512).fill('1').join('+'), { width: 32 });
await writeFile('crates/misclab-core/tests/fixtures/ebctf-programmer-batch20.json', JSON.stringify({ revision, vectors }, null, 2) + '\n');
console.log(`Captured ${vectors.length} programmer-calculator vectors.`);
