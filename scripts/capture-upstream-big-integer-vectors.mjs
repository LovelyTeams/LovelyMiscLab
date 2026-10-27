// Maintenance only: capture the pinned upstream oracle, never a runtime dependency.
import { execFileSync } from 'node:child_process';
import { writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
const root = resolve('.git/reference/EBCTFCodeBox');
const revision = execFileSync('git', ['-C', root, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
if (revision !== 'ea2bc36dae31c97060f7f2cddad8df5518afad7b') throw Error('Wrong reference revision');
const url = name => pathToFileURL(resolve(root, 'src/core/' + name + '.js'));
await import(url('bigmath'));
const { OPS } = await import(url('registry'));
const operation = OPS.find(o => o.id === 'bigCalc');
const vectors = [];
async function capture(op, a, b = '', m = '') {
  const params = { op, a: String(a), b: String(b), m: String(m) };
  const v = { params };
  try { v.text = await operation.run('', params); } catch (e) { v.error = e.message; }
  vectors.push(v);
}
for (const op of ['add', 'sub', 'mul', 'div', 'mod', 'gcd', 'extgcd', 'lcm']) {
  for (const [a, b] of [[0, 0], [0, -7], [-7, 0], [-7, 3], [7, -3], [-7, -3], [240, 46], [-240, 46], [240, -46], [12, 18], ['123456789012345678901234567890', '98765432109876543210987654321'], ['\uFEFF-1,234_567　', '\n+ 89\t']]) await capture(op, a, b);
}
for (const a of [-12, -1, 0, 1, 2, 123456789]) for (const b of [0, 1, 2, 17]) await capture('pow', a, b);
for (const a of [-17, 0, 1, 2, '123456789012345678901234567890']) {
  for (const m of [1, 5, 101, '100000000000000000000000000003']) {
    await capture('modinv', a, '', m);
    for (const b of [0, 1, 19, '123456789012345678901']) await capture('modpow', a, b, m);
  }
}
const mersenne = (1n << 127n) - 1n;
for (const n of [-7, 0, 1, 2, 3, 4, 9, 97, 997, 1009, 561, 1105, 1729, 99991, '3215031751', '341550071728321', '3825123056546413051', '318665857834031151167461', '3317044064679887385961981', '2305843009213693951', mersenne, mersenne * 3n]) {
  await capture('isPrime', n);
  await capture('bitLen', n);
  if (BigInt(n) >= 0n) await capture('sqrtInt', n);
}
for (const n of [-7, 0, 1, 2, 3, 4, 10, 97, 1000, 65536, '999999999999999999', mersenne - 100n]) {
  await capture('nextPrime', n); await capture('prevPrime', n);
}
for (const n of [1, 2, 12, 65536, 99991, 100000, '1000036000099', '1000006000009', '1000073001431003663', '2305843009213693951']) await capture('factor', n);
for (const [op, a, b, m] of [
  ['pow', 2, -1], ['pow', 2, 10000001], ['pow', 2, 500001],
  ['modpow', 2, -1, 5], ['modpow', 2, 10, 0], ['modinv', 2, '', -1],
  ['sqrtInt', -1], ['factor', 0], ['factor', -1], ['other', 1],
]) await capture(op, a, b, m);
for (const a of ['', '中文', '１２３', '0x123', '1.2', '1e3', '--2', '+', '1\u00852']) await capture('add', a, 1);
await writeFile('crates/misclab-core/tests/fixtures/ebctf-big-integer-batch19.json', JSON.stringify({ revision, vectors }, null, 2) + '\n');
console.log(`Captured ${vectors.length} big integer vectors across all 17 operations.`);
