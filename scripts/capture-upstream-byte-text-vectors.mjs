// Fixed upstream byte/text conversion vectors; maintainer tool only.
import{readFile,writeFile}from'node:fs/promises';
import{execFileSync}from'node:child_process';
import{resolve}from'node:path';
import{pathToFileURL}from'node:url';
const root=resolve('.git/reference/EBCTFCodeBox'),revision=execFileSync('git',['-C',root,'rev-parse','HEAD'],{encoding:'utf8'}).trim();
if(revision!=='ea2bc36dae31c97060f7f2cddad8df5518afad7b')throw Error('Wrong reference');
for(const name of ['jsEscape','fullwidth','bubblebabble','yenc','bech32','engEncoding'])await import(pathToFileURL(resolve(root,'src/core/'+name+'.js')));
const{OPS}=await import(pathToFileURL(resolve(root,'src/core/registry.js')));
const vectors=[];
async function cap(id,dir,s,p={},raw){
 const op=OPS.find(o=>o.id===id),params=Object.fromEntries((op.params||[]).map(p=>[p.key,p.default]));Object.assign(params,p);
 const stem={jsEscape:'js_escape',fullwidth:'fullwidth',bubblebabble:'bubblebabble',yenc:'yenc',bech32:'bech32'}[id];
 const node=id==='hexdump'?(dir==='encode'?'to_hexdump':'from_hexdump'):stem+'_'+dir;
 const port=(dir==='encode'&&['bubblebabble','yenc','bech32','hexdump'].includes(id))?'data':'text';
 const v={operation:id,node,input:{[port]:raw?{type:'bytes',value:raw}:{type:'text',value:s}},params:{...params,...(id==='hexdump'?{style:'xxd'}:{}),...(['jsEscape','yenc','hexdump'].includes(id)?{strict:false}:{})}};
 try {
  const result=await op[dir](s,{...params,...(raw?{rawBytes:Uint8Array.from(raw)}:{})});
  if(typeof result==='string'){
   if(id==='jsEscape'){v.utf16le=Array.from(Buffer.from(result,'utf16le'));v.text=result.isWellFormed()?result:null;}
   else v.text=result;
  }else {v.text=result.text;v.bytes=Array.from(result.rawBytes||result.bytes||result.files?.[0]?.bytes||[]);}
 }catch(e){v.error=e.message;}
 vectors.push(v);return v;
}
for(const id of ['jsEscape','fullwidth','bubblebabble'])for(const s of ['', 'Hello, World!', '中文🙂\ufeff', '\0\t\r\n', 'café', '1234567890', 'Pineapple', Array.from({length:128},(_,i)=>String.fromCharCode(i)).join('')]){
 const enc=await cap(id,'encode',s);if(enc.text!==undefined)await cap(id,'decode',enc.text);
}
for(const s of ['%u4E2D%u6587','%uD83D%uDE42','%uD800','%uDC00','%uD800A','%','%u','%uZZZZ','%U0041','%G0','%00%E9','%uFEFF'])await cap('jsEscape','decode',s);
for(const s of ['ａｂｃ　中文🙂','ＡＢＣ１２３','ｶﾀｶﾅ','一二三','ＡA'])await cap('fullwidth','decode',s);
const all=Array.from({length:256},(_,i)=>i);
for(const raw of [all,[0],[0,0],[255,255,255],[0xef,0xbb,0xbf,65],[]]){
 const enc=await cap('bubblebabble','encode','',{},raw);await cap('bubblebabble','decode',enc.text);
}
for(const s of ['xexax',' \nxexax\t','xexex','abc','x','xx','xixax','xexabx','xexab-bexax','xexax-bexax'])await cap('bubblebabble','decode',s);
for(const width of [1,2,3,16,128,256])for(const s of ['','Hello, 中文🙂!', '\0\r\n=', 'café']){
 const enc=await cap('yenc','encode',s,{width});await cap('yenc','decode',enc.text);
}
for(const width of [1,2,128])for(const raw of [all,[214,224,227,19,223,246,4],[]]){
 const enc=await cap('yenc','encode','',{width},raw);await cap('yenc','decode',enc.text);
}
for(const s of ['=','=A=','=ybegin line=128 size=1 name=x\nk\n=yend size=1','k\r','k\r\n','==','🙂'])await cap('yenc','decode',s);
for(const width of [0,-1,'bad'])await cap('yenc','encode','A',{width});
for(const hrp of ['bc','tb','a','test1','!'])for(const s of ['','00','ff','00010203','48656c6c6f','00112233445566778899aabbccddeeff']){
 const enc=await cap('bech32','encode',s,{hrp});if(enc.text!==undefined){await cap('bech32','decode',enc.text);await cap('bech32','decode',enc.text.toUpperCase());}
}
for(const s of ['A12UEL5L','a12uel5l','A1G7SGD8','bc1qqqqqq','abc','a1lqfn3a','a1gmk9yu','A12uEL5L','a1qqqqqq','a12uel5l '])await cap('bech32','decode',s);
for(const width of [1,2,3,16,17,32,512])for(const upperCase of [false,true])for(const s of ['','Hello, World!','中文🙂\0','ABCD'.repeat(11)]){
 const enc=await cap('hexdump','encode',s,{width,upperCase});await cap('hexdump','decode',enc.text);
}
const allDump=await cap('hexdump','encode','',{},all);await cap('hexdump','decode',allDump.text);
for(const s of ['00000000: 4142  AB\n*\n00000008: 4344  CD','0040h: 41-42-43  ABC','00000000  41 42 43  |ABC|','41 42 43','not a dump','','00000000: 414  junk','00000000: 4142  AB\n*\n00000008'])await cap('hexdump','decode',s);
for(const width of [0,513,-1,'bad'])await cap('hexdump','encode','A',{width});
await writeFile('crates/misclab-core/tests/fixtures/ebctf-byte-text-batch13.json',JSON.stringify({revision,vectors},null,2).replaceAll('\uFFFD','\\uFFFD')+'\n');
console.log('Captured '+vectors.length+' vectors across '+new Set(vectors.map(v=>v.operation)).size+' operations.');
