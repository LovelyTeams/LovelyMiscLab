// Maintainer-only reference capture. No upstream JS is included in the app.
import { readFile, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
const root = resolve('.git/reference/EBCTFCodeBox');
const revision = execFileSync('git', ['-C', root, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
if (revision !== 'ea2bc36dae31c97060f7f2cddad8df5518afad7b') throw Error('Wrong reference');
const moduleUrl = name => pathToFileURL(resolve(root, 'src/core/' + name + '.js'));
for (const name of ['fengCodec', 'engEncoding', 'byteTools', 'objectIdTime', 'hexview', 'bitops']) await import(moduleUrl(name));
const { OPS } = await import(moduleUrl('registry'));
if (process.argv[2] === '--verify-rust') {
  const frames=JSON.parse(await readFile(process.argv[3],'utf8'));
  const { inflateRawSync }=await import('node:zlib');
  const dx=OPS.find(o=>o.id==='dxBase64');
  for (const {encoded,bytes} of frames) {
    const expected=new TextDecoder().decode(Uint8Array.from(bytes));
    if (await dx.decode(encoded)!==expected) throw Error('Rust encoder differs from upstream decoder');
    const frame=Buffer.from(encoded,'base64');
    const raw=inflateRawSync(frame.subarray(4).map((b,i)=>b^frame[2+i%2]));
    if (!raw.equals(Buffer.from(bytes))) throw Error('Rust encoder loses binary bytes');
  }
  console.log(`Verified ${frames.length} real Rust frames with pinned upstream + independent zlib.`);
  process.exit(0);
}
const vectors = [];
const nodes = { rison: 'rison', unixPerms: 'unix_permissions', uuidParse: 'uuid_parse', objectIdTime: 'object_id_time', hexRange: 'hex_range', hexStats: 'hex_stats', bitPlaneExtract: 'bit_plane_extract', dxBase64: 'dx_base64' };
async function cap(id, dir, text, override = {}, raw) {
  const op = OPS.find(o => o.id === id);
  const params = { ...Object.fromEntries((op.params || []).map(p => [p.key, p.default])), ...override };
  const port = ['hexRange', 'hexStats', 'bitPlaneExtract'].includes(id) ? 'data' : 'text';
  const v = { operation: id, node: nodes[id] + (dir === 'run' ? '' : '_' + dir), input: { [port]: raw !== undefined ? { type: 'bytes', value: raw } : { type: 'text', value: text } }, params };
  try { v.text = await op[dir](text, { ...params, ...(raw !== undefined ? { rawBytes: Uint8Array.from(raw) } : {}) }); }
  catch (e) { v.error = e.message; }
  vectors.push(v); return v;
}
const samples = ['', 'null', 'true', 'false', '0', '-0', '1e21', '1e-7', '9007199254740993', '1e400', '[]', '{}', '""', '"Hello, 中文🙂!"', '"a\'b! c+%"', '{"b":2,"a":1,"values":[true,false,null,"中文",-3.5]}', '{"10":"ten","2":"two","😀":1,"\uE000":2}', '[1,"x",{},[],false]', '"-1"'];
for (const mode of ['value', 'object', 'array', 'uri']) for (const s of samples) {
  const encoded = await cap('rison', 'encode', s, { mode });
  if (encoded.text !== undefined) await cap('rison', 'decode', encoded.text, { mode });
}
for (const s of ["a", "(a:1,a:2)", "(1:hello,!t:yes)", "(!(1,!n,!t):hello)", "(x:1.)", "-0", "-.5", "01", "1e-3", "!1", "!", "!(", "!(,a)", "!(a,)", "(,a:1)", "(a:)", "(a:1,)", "'x!x'", "'unclosed", "-", "1e", "1e+3", "1e400", "a b", "()extra", "!t!f"]) await cap('rison', 'decode', s);
for (const s of ["(a:%E4%B8%AD)","(a:%FF)","(a:%C3%28)","(a:'a+b')", "(a:%ZZ)"]) await cap('rison','decode',s,{mode:'uri'});
for (const s of ['', 'x', '888', '07555', '12', '0x755', 'rwxrwxrws', 'drwxrwxrwx', 'D---------', 'prw-------', 'lrwxrwxrwx', 'crw-rw-rw-', 'brw-rw-rw-', 'srwxr-xr-x', '-rwSr--r--', 'rwxr-xr-T', '  7 5 5  ', '中文']) await cap('unixPerms','run',s);
for (const sp of [0,1,2,3,4,5,6,7]) for (const regular of ['000','111','644','755','777']) await cap('unixPerms','run',sp + regular);
for (const s of ['', '中文', 'not-a-uuid', 'f81d4fae-7dec-11d0-a765-00a0c91e6bf6', '01890f47-6d48-7e1b-bd04-d233f911f5a8', 'FFFFFFFF-FFFF-7FFF-BFFF-FFFFFFFFFFFF', '{550E8400-E29B-41D4-A716-446655440000}']) await cap('uuidParse','run',s);
for (let v=0;v<16;v++) for (const variant of ['0','8','c','e']) await cap('uuidParse','run',`00000000-0000-${v.toString(16)}000-${variant}000-000000000000`);
for (const s of ['', '507c7f79bcf86cd7994f6c0e', '000000000000000000000000', 'ffffffffffffffffffffffff', '  507C7F79BCF86CD7994F6C0E  ', 'g'.repeat(24), '0'.repeat(23), '0'.repeat(25), '中文']) await cap('objectIdTime','run',s);
for (const plane of ['all','0','1','2','3','4','5','6','7','8','-1']) for (const s of ['', 'Hello', '中文🙂', '\0\x7f\n', 'ÿ']) await cap('bitPlaneExtract','run',s,{plane});
const all = Array.from({length:256},(_,i)=>i);
for (const inputEnc of ['auto','utf8','hex','base64']) for (const s of ['', 'Hello, 中文!', '00 01 ff 41', 'SGVsbG8=', 'ABCD', '%%']) {
  await cap('hexRange','run',s,{inputEnc});
  await cap('hexStats','run',s,{inputEnc});
}
for (const raw of [[],[0],[0,0,1],[0xef,0xbb,0xbf,65],all]) {
  for (const [start,end] of [['0',''],['-4','2'],['0x2','0x8'],['99','1'],['0','999'],['2','-1']]) await cap('hexRange','run','',{start,end},raw);
  for (const window of [1,16,17,256,65537,0,-1]) for (const [showCurve,showHist] of [[true,true],[true,false],[false,true],[false,false]]) await cap('hexStats','run','',{window,showCurve,showHist},raw);
}
// Fixed upstream random-source mode makes capture reproducible. The app keeps
// random salt by default; verify Rust-produced frames with upstream decode above.
const { setRandConfig } = await import(moduleUrl('randSource'));
setRandConfig({mode:'custom',seed:'LovelyMiscLab migration batch 14'});
const dx = OPS.find(o=>o.id==='dxBase64');
for (const raw of [[],[0],[255],all,[0xef,0xbb,0xbf,65],Array.from(new TextEncoder().encode('Hello, 中文🙂!'))]) {
  const encoded = await dx.encode('',{rawBytes:Uint8Array.from(raw)});
  const v=await cap('dxBase64','decode',encoded);v.bytes=raw;
}
for (const s of ['', ' ', 'a', 'AAAA', 'AAAAAA==', '%%%%', 'Bl23oUTpemh+dubZBcJaEt5MqEd4zeWltw==']) await cap('dxBase64','decode',s);
const overlimitFrame=await dx.encode('',{rawBytes:new Uint8Array(8*1024*1024+1)});
await writeFile('crates/misclab-core/tests/fixtures/ebctf-foundation-batch14.json', JSON.stringify({revision,vectors,overlimitFrame},null,2).replaceAll('\uFFFD','\\uFFFD')+'\n');
console.log(`Captured ${vectors.length} vectors across ${new Set(vectors.map(v=>v.operation)).size} operations.`);
