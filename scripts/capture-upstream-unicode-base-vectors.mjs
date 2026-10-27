// Optional reference capture; no upstream JS is used by application algorithms.
import{readFile,writeFile}from'node:fs/promises';
import{execFileSync}from'node:child_process';
import{resolve}from'node:path';
import{pathToFileURL}from'node:url';
const root=resolve('.git/reference/EBCTFCodeBox');
const revision=execFileSync('git',['-C',root,'rev-parse','HEAD'],{encoding:'utf8'}).trim();
if(revision!=='ea2bc36dae31c97060f7f2cddad8df5518afad7b')throw Error('Wrong reference');
for(const f of ['base','baseExt'])await import(pathToFileURL(resolve(root,'src/core/'+f+'.js')));
const{OPS}=await import(pathToFileURL(resolve(root,'src/core/registry.js')));
const ops=Object.fromEntries(OPS.map(o=>[o.id,o]));
const mapping={radixN:'radix_n',base69:'base69',base2048:'base2048',base65536:'base65536',ecoji:'ecoji',base32steg:'base32_padding_stego',base64steg:'base64_padding_stego'};
const vectors=[];
async function capture(id,dir,s,p={},raw){const v={operation:id,node:mapping[id]+'_'+dir,input:{[dir==='encode'?'data':'text']:raw?{type:'bytes',value:raw}:{type:'text',value:s}},params:p};let result;try{result=await ops[id][dir](s,{...p,...(raw?{rawBytes:Uint8Array.from(raw)}:{})});}catch(e){v.error=e.message;vectors.push(v);return;}if(result&&typeof result==='object'&&result.files?.[0]?.bytes)v.bytes=[...result.files[0].bytes];else v.text=String(result);vectors.push(v);return result;}
async function pair(id,s,p={},raw){const e=await capture(id,'encode',s,p,raw);if(typeof e==='string')await capture(id,'decode',e,p);}
for(const id of Object.keys(mapping))for(const s of ['','A','Hi','abc','abcd','abcde','abcdef','abcdefg','abcdefgh','Hello, 中文🙂','\uFEFFtest','\0\0Hello'])await pair(id,s);
for(const id of ['radixN','base69','base2048','base65536','ecoji'])for(const b of [[0],[0,0],[255],[0,255,128,1,65,0],Array.from({length:256},(_,i)=>i)])await pair(id,'',{},b);
for(const radix of [2,8,16,36,62,94])await pair('radixN','Hi中文',{radix});
await pair('radixN','ABC',{radix:4,dict:'abcd'});
await pair('radixN','ABC',{radix:95,dict:Array.from({length:95},(_,i)=>String.fromCharCode(33+i)).join('')});
const custom='ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/-*<>|'.split('').reverse().join('');
for(let n=1;n<=14;n++)await pair('base69','x'.repeat(n),{dict:custom});
const old=await capture('base69','encode','1234567');await capture('base69','decode',old+'AAAAAAAAAAAA7=');
await capture('base2048','decode','not-a-valid-code🙂',{strict:false});
await capture('base65536','decode','ABC🙂',{strict:false});
await capture('ecoji','decode','ABCDx',{strict:false});
for(const id of ['base32steg','base64steg']){const e=await capture(id,'encode','Secret中文');await capture(id,'decode',e.split('\n').map(s=>' '+s+' ').join('\r\n'));await capture(id,'decode','plain line without padding');}
for(const[id,s,p]of [['radixN','Hello',{radix:95}],['base69','AA',{}],['base69','AAAAAAAAAAAA7=',{}]])await capture(id,id==='radixN'?'encode':'decode',s,p);
const source=await readFile(resolve(root,'src/core/baseExt.js'),'utf8');
const tables=JSON.parse(await readFile('crates/misclab-core/src/nodes/unicode_base_tables.json','utf8'));
for(const key of ['B2048_11','B2048_3','B65_START','ECO_MAP','ECO_PAD4']){const expected=JSON.parse(source.match(new RegExp('const '+key+' = ([^;]+);'))[1]);if(JSON.stringify(tables[key])!==JSON.stringify(expected))throw Error('Table differs: '+key);}
await writeFile('crates/misclab-core/tests/fixtures/ebctf-unicode-base-batch7.json',JSON.stringify({revision,vectors},null,2).replaceAll('\uFFFD','\\uFFFD')+'\n');
console.log('Captured '+vectors.length+' vectors across 7 operations; all Unicode tables match reference.');
