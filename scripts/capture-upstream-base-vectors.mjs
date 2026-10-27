// Maintainer-only reference capture; app and cargo tests run exclusively in Rust.
import { writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { execFileSync } from 'node:child_process';
const root = resolve('.git/reference/EBCTFCodeBox');
const revision = execFileSync('git', ['-C', root, 'rev-parse', 'HEAD'], {encoding:'utf8'}).trim();
if (revision !== 'ea2bc36dae31c97060f7f2cddad8df5518afad7b') throw Error('Wrong upstream revision');
await import(pathToFileURL(resolve(root,'src/core/base.js')));
const { OPS } = await import(pathToFileURL(resolve(root,'src/core/registry.js')));
const ids = ['base16','base32','base45','base58','base62','base64','base85','base92'];
const ops = Object.fromEntries(OPS.map(o => [o.id,o]));
const alphabets = {
 base16:'0123456789ABCDEF',base32:'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567',base45:'0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:',
 base58:'123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz',base62:'0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz',
 base64:'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/',base92:"!#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_abcdefghijklmnopqrstuvwxyz{|}",
};
const vectors = [];
function params(id, p) {
 const dict = p.dict || alphabets[id];
 switch(id) {
  case 'base16': return {alphabet:dict,upper:p.upper!==false,space:!!p.space,strip:true};
  case 'base32': return {variant:p.dict?'自定义':({rfc4648:'标准',hex:'Hex 扩展',crockford:'Crockford',zbase32:'z-base-32'}[p.variant||'rfc4648']),alphabet:dict,padding:['crockford','zbase32'].includes(p.variant)?false:p.padding!==false,strip:false};
  case 'base45': return {alphabet:dict,strip:false};
  case 'base58': return {variant:{bitcoin:'Bitcoin',ripple:'Ripple',flickr:'Flickr',custom:'自定义'}[p.alphabet||'bitcoin'],alphabet:dict,literalAlphabet:true,strip:false};
  case 'base62': return {alphabet:dict,literalAlphabet:true,preserveZeros:true,strip:false};
  case 'base64': return {variant:p.dict?'自定义':p.urlsafe?'URL安全':'标准',alphabet:dict,padding:p.padding!==false,urlsafe:!!p.urlsafe};
  case 'base85': return {variant:'标准',delim:!p.compat,strip:false};
  case 'base92': return {alphabet:dict,emptyMarker:true};
 }
}
async function capture(id, direction, input, p={}) {
 const node=(id==='base16'?'hex':id)+'_'+direction;
 const port=direction==='encode'&&!['base16','base64'].includes(id)?'data':'text';
 const v={upstreamId:id,direction,node,input:{[port]:input instanceof Uint8Array?{type:'bytes',value:[...input]}:{type:'text',value:input}},params:params(id,p)};
 let result;
 try {result = await ops[id][direction](input,p);} catch(e) {v.error=String(e.message);vectors.push(v);return null;}
 if (result && typeof result === 'object' && result.files?.[0]?.bytes) v.bytes=[...result.files[0].bytes];
 else v.text=String(result);
 vectors.push(v);return result;
}
async function pair(id,input,p={}) {const encoded=await capture(id,'encode',input,p); if(typeof encoded==='string') await capture(id,'decode',encoded,p);}
for(const id of ids) {
 for(const input of ['', 'Hello, CTF!', '中文🙂\n', ' \t\u0000A\u0000 ']) await pair(id,input);
 for(const length of [1,2,3,4,5,7,16,255,256]) await pair(id,Uint8Array.from({length},(_,i)=>i));
 await pair(id,Uint8Array.from([0,0,255,0,128,65,0]));
}
for(const upper of [false,true]) for(const space of [false,true]) await pair('base16','中文abc',{upper,space});
for(const variant of ['rfc4648','hex','crockford','zbase32']) for(const padding of [false,true]) await pair('base32','中文🙂a',{variant,padding});
await capture('base32','decode','mzxw6==='); await capture('base32','decode','CSQPY',{variant:'crockford'});
for(const alphabet of ['bitcoin','flickr','ripple','custom']) await pair('base58',Uint8Array.from([0,0,1,128,255]),{alphabet,dict:[...alphabets.base58].reverse().join('')});
for(const padding of [false,true]) for(const urlsafe of [false,true]) {
 await pair('base64',Uint8Array.from([251,255,0,128]),{padding,urlsafe});
 await pair('base64','中文',{dict:[...alphabets.base64].reverse().join(''),padding,urlsafe});
}
for(const id of ['base16','base32','base45','base62','base92']) await pair(id,'字典A',{dict:[...alphabets[id]].reverse().join('')});
for(const compat of [false,true]) for(const input of ['',Uint8Array.from([0]),Uint8Array.from([0,0,0]),Uint8Array.from([0,0,0,0]),'Hello']) await pair('base85',input,{compat});
await capture('base16','decode','0x48:65,6c 6c6f');
await capture('base85','decode','<~BOu!r\nD]j7BEbo7~>');
for(const [id,input] of [['base16','123'],['base32','!!!'],['base45','GGW'],['base45','::'],['base45','0'],['base58','0'],['base64','%%%'],['base85','~']]) await capture(id,'decode',input);
await writeFile('crates/misclab-core/tests/fixtures/ebctf-base-batch3.json',JSON.stringify({revision,vectors},null,2)+'\n');
console.log(`Captured ${vectors.length} reference vectors for ${ids.length} Base operations`);
