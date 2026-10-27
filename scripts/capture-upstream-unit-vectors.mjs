// Maintenance-only pinned reference capture. Runtime uses static data + Rust.
import { execFileSync } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import vm from 'node:vm';
const root = resolve('.git/reference/EBCTFCodeBox');
const revision = execFileSync('git', ['-C', root, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
if (revision !== 'ea2bc36dae31c97060f7f2cddad8df5518afad7b') throw Error('Wrong reference revision');
const path = resolve(root, 'src/core/unitConv.js');
const source = await readFile(path, 'utf8');
const ref = await import(pathToFileURL(path));
const extract = name => vm.runInNewContext(source.match(new RegExp('const ' + name + ' = (\\[[\\s\\S]*?^\\]);', 'm'))[1], { rat: ref.rat, R1: ref.rat(1n) });
const aliases = vm.runInNewContext('(' + source.match(/function normUnit[\s\S]*?const map = (\{[\s\S]*?\n  \});/)[1] + ')');
const groups = [
  { id: 'dataSize', name: '数据量', units: ref.DATA_SIZE },
  { id: 'dataRate', name: '数据速率', units: ref.DATA_RATE },
  { id: 'time', name: '时间', units: ref.TIME_UNITS },
  { id: 'freq', name: '频率', units: ref.FREQ_UNITS },
  { id: 'length', name: '长度', units: extract('LEN_UNITS') },
  { id: 'speed', name: '速度', units: extract('SPEED_UNITS') },
];
const currencies = extract('CURRENCY_SETS');
const replacer = (_key, value) => typeof value === 'bigint' ? value.toString() : value;
await writeFile('crates/misclab-core/src/nodes/unit_definitions.json', JSON.stringify({ revision, groups, currencies, aliases }, replacer, 2) + '\n');
const vectors = [];
function capture(input, params = {}) {
  const v = { input, params };
  try {
    const report = ref.unitConvRun(input, params);
    v.rows = [...report.matchAll(/^  (\S+)\s+= (\S+)/gm)].map(m => ({ unit: m[1], value: m[2] }));
    if (!v.rows.length) {
      const qty = ref.parseRatDecimal(input.trim());
      const epoch = params.tsEpoch && params.tsEpoch !== 'auto' ? params.tsEpoch : ref.sniffEpoch(qty);
      const sec = ref.TS_EPOCHS[epoch].toSec(qty);
      v.timestamp = { epoch, seconds: sec };
      v.iso = report.match(/ISO8601\(UTC\) : (\S+)/)?.[1];
      v.dos = report.match(/DOS 日期时间 : (0x[\dA-F]+)/)?.[1] ?? null;
    }
  } catch (e) { v.error = e.message; }
  vectors.push(v);
}
const units = [...new Set([...groups, ...currencies].flatMap(g => g.units.map(u => u.u)).concat(ref.ANGLE_UNITS))];
for (const unit of units) for (const q of ['0', '1', '1.5', '-7', '1000000000000000', '0.000000000000000001']) capture(q + ' ' + unit);
for (const alias of Object.keys(aliases)) capture('1 ' + alias);
for (const tsEpoch of ['auto', 'unix_s', 'unix_ms', 'filetime', 'cocoa', 'chrome_us']) {
  for (const input of ['0', '-1', '1', '1.5', '-0.5', '1655526400', '1655526400000', '133000000000000000', '13300000000000000', '677219200', '99999999999', '100000000000', '99999999999999', '100000000000000', '99999999999999999', '100000000000000000']) capture(input, { tsEpoch });
}
for (const input of ['', '中文🙂', 'GB 1', '1 XX', '1.2.3 GB', '.5 GB', '1. GB', '1e3 B', '1,000 B', '1.0000000000000000001 B', '1'.repeat(4097)]) capture(input);
capture('1', { tsEpoch: 'invalid' });
await writeFile('crates/misclab-core/tests/fixtures/ebctf-unit-batch21.json', JSON.stringify({ revision, vectors }, replacer, 2) + '\n');
console.log(`Captured ${vectors.length} unit vectors, ${units.length} units and ${Object.keys(aliases).length} aliases.`);
