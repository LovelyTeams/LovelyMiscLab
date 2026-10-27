// Maintainer-only reference capture; the application runs Rust exclusively.
import { writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
const root = resolve('.git/reference/EBCTFCodeBox');
const revision = execFileSync('git', ['-C', root, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
if (revision !== 'ea2bc36dae31c97060f7f2cddad8df5518afad7b') throw Error('Wrong reference');
const url = name => pathToFileURL(resolve(root, 'src/core/' + name + '.js'));
for (const name of ['geo', 'hamming', 'huffman']) await import(url(name));
const { OPS } = await import(url('registry'));
const nodes = { geoDms: 'geo_dms', geoHash: 'geo_hash', geoPlusCode: 'geo_plus_code', geoMaidenhead: 'geo_maidenhead', geoUtm: 'geo_utm', hammingCode: 'hamming_code', huffmanCodec: 'huffman' };
const vectors = [];
async function cap(id, dir, text, override = {}) {
  const op = OPS.find(o => o.id === id);
  const params = { ...Object.fromEntries((op.params || []).map(p => [p.key, p.default])), ...override };
  const v = { operation: id, node: nodes[id] + '_' + dir, input: { [id === 'huffmanCodec' ? 'data' : 'text']: { type: 'text', value: text } }, params };
  try { v.text = await op[dir](text, params); } catch (e) { v.error = e.message; }
  vectors.push(v); return v;
}
for (const id of Object.keys(nodes).filter(id => id.startsWith('geo'))) {
  const opts = id === 'geoHash' ? Array.from({length:12},(_,i)=>({precision:String(i+1)})) : id === 'geoPlusCode' ? [2,4,6,8,10,11,12,13,14,15].map(codeLength=>({codeLength:String(codeLength)})) : id === 'geoMaidenhead' ? [2,3,4].map(pairs=>({pairs:String(pairs)})) : [{}];
  for (const params of opts) for (const s of ['0,0','57.649,10.407','39.9042，116.4074','-33.8688 151.2093','48.8582,2.2945','-79.999,-179.999','83.999,179.999','1.234567,-0.000001']) {
    const v = await cap(id,'encode',s,params);
    if (v.text !== undefined) await cap(id,'decode',v.text,params);
  }
  for (const s of ['', '中文🙂','91,0','0,181','1e2,3','NaN,0','1,2,3']) await cap(id,'encode',s);
}
for (const s of ['90,180','-90,-180','90,-180']) for (const id of ['geoDms','geoHash','geoPlusCode']) { const v=await cap(id,'encode',s); await cap(id,'decode',v.text); }
for (const s of ['', '中文', 'z', 'EZs42', 'u4pruydqqvj', 'aaaaaaaa','0','zzzzzzzzzzzz']) await cap('geoHash','decode',s);
for (const s of ['', '中文', '8FVC9G8F+6X','6FG22222+22','22222222+22','CFX3X2X2+X2','8FVC9G8F+6','9G8F+6X','8F000000+','849VCWC8+R9','849VCWC8+R9G','849VCWC8+R9GXX','849VCWC8+R9GXXZZ']) await cap('geoPlusCode','decode',s);
for (const s of ['', 'FN20','FN20XR','FN20XRAB','fn20xrab','AA00','RR99xx','ZZ99','FN20zz','FN2000','FN20XRA']) await cap('geoMaidenhead','decode',s);
for (const s of ['', '中文', '31U 448251 5411937','56H,334369,6250948','31 U 500000 0','0U 500000 0','61U 500000 0','31I 500000 0','31U -1 5']) await cap('geoUtm','decode',s);
for (const s of ['', '中文','48°51\'29.52"N, 2°17\'40.2"E','33 52 7.68 S,151 12 33.48 E','0N,0E','90N,180E','91N,0E','0,0','1N','1N,2E,3W']) await cap('geoDms','decode',s);
for (const k of [1,2,3,4,5,11,26,57,120,247,502,1013]) {
  for (const s of ['', '中文🙂', '0','1','1011','1100 0011\n0101','10101010101010101']) {
    const v=await cap('hammingCode','encode',s,{k}); await cap('hammingCode','decode',v.text,{k});
    if (v.text) await cap('hammingCode','decode',(v.text[0]==='1'?'0':'1')+v.text.slice(1),{k});
  }
}
for (const k of [0,-1,2.5,'invalid']) for (const dir of ['encode','decode']) await cap('hammingCode',dir,'1011',{k});
for (const s of ['101', '111111111', '10101中文001']) await cap('hammingCode','decode',s);
for (const s of ['', 'a','aaaaaa','ABBA','abcabcabc', 'Hello, 中文🙂!','\uFEFFhello','\0\x01\x02\xff','the quick brown fox jumps over the lazy dog']) {
  const v=await cap('huffmanCodec','encode',s);
  if (v.text) {
    const weights=[...new Set(new TextEncoder().encode(s))].map(b=>`\\x${b.toString(16).padStart(2,'0')} ${new TextEncoder().encode(s).filter(x=>x===b).length}`).join('\n');
    const bitLen=Number(v.text.match(/bitLen: (\d+)/)[1]); const hex=v.text.match(/hex: ([a-f0-9]+)/)[1];
    await cap('huffmanCodec','decode',hex,{weights,bitLen});
  }
}
for (const weights of ['a 5\nb 2\nc 1','a:5\nb,2\nc=1','a 0\nb 0\nc 0','a 0.1\nb 0.2\nc 0.3','a 5\na 1\nb 2\nc 3','a -1','a NaN','a 1','中文 1']) await cap('huffmanCodec','encode','abccba',{weights});
for (const table of ['a 0\nb 10\nc 11','a=0\nb=10\nc=11','a 0\nb 01','a 0\na 1','a 0\nb 0','a 2','a 0','', '#comment\n"a" 0\n0x62 10\n\\x63 11']) {
  const v=await cap('huffmanCodec','encode','abcabc',{mode:'table',table});
  if(v.text) await cap('huffmanCodec','decode',v.text.match(/hex: ([a-f0-9]+)/)[1],{mode:'table',table,bitLen:Number(v.text.match(/bitLen: (\d+)/)[1])});
}
for (const [s,p] of [['',{}],['0',{}],['ff',{}],['01',{weights:'a 1',bitLen:9}],['01',{weights:'a 1'}],['80',{mode:'table',table:'a 0\nb 10\nc 11',bitLen:1}],['c0',{mode:'table',table:'\\xff 0\n\\x00 1',bitLen:2}]]) await cap('huffmanCodec','decode',s,p);
await writeFile('crates/misclab-core/tests/fixtures/ebctf-geo-coding-batch15.json',JSON.stringify({revision,vectors},null,2)+'\n');
console.log(`Captured ${vectors.length} vectors for ${new Set(vectors.map(v=>v.operation)).size} operations.`);
