// Maintainer-only reference capture. No JS codecs ship in the Rust runtime.
import {readFile,writeFile} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
const root=resolve('.git/reference/EBCTFCodeBox');
const revision=execFileSync('git',['-C',root,'rev-parse','HEAD'],{encoding:'utf8'}).trim();
if(revision!=='ea2bc36dae31c97060f7f2cddad8df5518afad7b')throw Error('Wrong reference');
// Node 24 ICU differs from the browser's WHATWG code pages. Use a separate
// specification implementation, pinned by content hash, solely for capture.
// Obtain with npm pack text-encoding@0.7.0 and extract under this directory.
const decoderRoot=resolve('.git/reference/text-encoding/package');
const decoderHashes={
 'encoding.js':'13d6f5d8d7849ab5525c7b3defc4666d1d53b2a2fbf9777eb47039956ce2d0c7',
 'encoding-indexes.js':'d0fd4fed32adb04570634ef4c50e2fd92b3c7f240b68e4403db1cf5df609663f',
};
for(const[file,hash]of Object.entries(decoderHashes)){
 if(createHash('sha256').update(await readFile(resolve(decoderRoot,'lib',file))).digest('hex')!==hash)throw Error('Reference decoder drift: '+file);
}
globalThis.TextDecoder=createRequire(import.meta.url)(resolve(decoderRoot,'lib/encoding.js')).TextDecoder;
const src=await import(pathToFileURL(resolve(root,'src/core/charset.js')));
const {OPS}=await import(pathToFileURL(resolve(root,'src/core/registry.js')));
const tablePath='crates/misclab-core/src/nodes/charset_tables.json';
const tables={revision,source:'src/core/charset.js',ebcdic:src.EBCDIC_TABLES,presets:src.MOJIBAKE_PRESETS};
if(process.argv.includes('--tables'))await writeFile(tablePath,JSON.stringify(tables,null,2)+'\n');
if(JSON.stringify(JSON.parse(await readFile(tablePath,'utf8')))!==JSON.stringify(tables))throw Error('Charset snapshot drift');
const stems={gbCharset:'gb_charset',gb2312QuWei:'gb2312_quwei',big5:'big5',shiftJis:'shift_jis',eucKr:'euc_kr',latinCharset:'latin_charset',ebcdic:'ebcdic',utf16:'utf16',mojibakeFix:'mojibake'};
const vectors=[];
async function cap(id,dir,s,p={},raw){
 const op=OPS.find(o=>o.id===id);
 const params=Object.fromEntries(op.params.map(p=>[p.key,p.default]));
 Object.assign(params,p);
 const v={operation:id,node:stems[id]+'_'+dir,input:{data:raw?{type:'bytes',value:raw}:{type:'text',value:s}},params:{...params,codecMode:'upstream',strict:false}};
 try{v.text=String(await op[dir](s,{...params,...(raw?{rawBytes:Uint8Array.from(raw)}:{})}));}catch(e){v.error=e.message;}
 vectors.push(v);return v.text;
}
const samples={gbCharset:['Hello\0中文','你好世界，测试123','🙂𠀀€'],big5:['中文繁體','測試Hello','🙂€'],shiftJis:['日本語かなカナ','ｶﾀｶﾅ\\¥‾','🙂€'],eucKr:['한국어 한글','Hello 가힣','🙂€'],latinCharset:['Hello,\0World!','café Noël ÿ','€ŒŠ—','中文🙂']};
for(const id of ['gbCharset','big5','shiftJis','eucKr','latinCharset']){
 const labels=id==='gbCharset'?['gbk','gb2312','gb18030']:id==='latinCharset'?OPS.find(o=>o.id===id).params[0].options.map(x=>x.value):[undefined];
 for(const charset of labels)for(const format of ['hex','latin1','auto'])for(const s of ['',...samples[id]]){
  const p={format,...(charset?{charset}:{})},out=await cap(id,'encode',s,p);
  if(out!==undefined)await cap(id,'decode',out,p);
 }
 for(const s of ['00','41 42,43;44:45','0x4142','0','xy','中文🙂'])await cap(id,'decode',s);
 for(const raw of [[0,65,66],[0xff],[0xef,0xbb,0xbf,65],[0xff,0xfe,65,0]])await cap(id,'decode','ignored',{},raw);
 await cap(id,'encode','ignored',{},[0xff]);
 await cap(id,'encode','ignored',{},[0xef,0xbb,0xbf,65]);
}
for(const sep of ['none','space','newline'])for(const s of ['','中文','Hello 中文!','123456','中1中','5中','A1234B中56国','中\n文','中 文','🙂']){
 const p={sep},out=await cap('gb2312QuWei','encode',s,p);
 if(out!==undefined)await cap('gb2312QuWei','decode',out,p);
}
for(const s of ['0000','9595','0101','5448 4644','中5448🙂','1\r\n2\n3\n4','12345','99999'])await cap('gb2312QuWei','decode',s);
for(const codepage of ['cp037','cp1047']){
 const chars=Object.keys(src.EBCDIC_TABLES[codepage]).join('');
 for(const format of ['hex','latin1','auto'])for(const s of ['',chars,'Hello, World!','[]{}^~','\n','中文🙂']){
  const p={codepage,format},out=await cap('ebcdic','encode',s,p);
  if(out!==undefined)await cap('ebcdic','decode',out,p);
 }
 await cap('ebcdic','decode','',{codepage},Array.from({length:256},(_,i)=>i));
}
for(const endian of ['BE','LE','auto'])for(const bom of [true,false])for(const format of ['hex','latin1','auto'])for(const s of ['','Hello\0中文🙂','\ufeffA']){
 const p={endian,bom,format},out=await cap('utf16','encode',s,p);
 if(out!==undefined)await cap('utf16','decode',out,p);
}
for(const s of ['d800','dc00','d8000041','00','fffe4100','feff0041','fffefffe4100','fefffeff0041','00gg'])await cap('utf16','decode',s);
for(const preset of Object.keys(src.MOJIBAKE_PRESETS))for(const s of ['','Hello!','中文','日本語','한글','café','🙂']){
 const p={preset},out=await cap('mojibakeFix','encode',s,p);
 if(out!==undefined)await cap('mojibakeFix','decode',out,p);
}
for(const [wrong,right] of [['utf-16le','utf-8'],['windows-1251','utf-8'],['gb18030','utf-8'],['euc-jp','utf-8'],['invalid','utf-8']]){
 const p={preset:'custom',wrong,right},out=await cap('mojibakeFix','encode','中文A',p);
 if(out!==undefined)await cap('mojibakeFix','decode',out,p);
}
await writeFile('crates/misclab-core/tests/fixtures/ebctf-charsets-batch12.json',JSON.stringify({revision,referenceDecoder:{package:'text-encoding@0.7.0',source:'https://github.com/inexorabletash/text-encoding',hashes:decoderHashes,purpose:'WHATWG browser mappings; host Node ICU is not the browser compatibility baseline'},vectors},null,2).replaceAll('\uFFFD','\\uFFFD')+'\n');
console.log('Captured '+vectors.length+' vectors across '+new Set(vectors.map(v=>v.operation)).size+' operations.');
