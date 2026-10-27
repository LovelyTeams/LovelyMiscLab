// Maintenance-only capture from the pinned upstream; no application JS runtime.
import {writeFile} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
const root=resolve('.git/reference/EBCTFCodeBox');
const revision=execFileSync('git',['-C',root,'rev-parse','HEAD'],{encoding:'utf8'}).trim();
if(revision!=='ea2bc36dae31c97060f7f2cddad8df5518afad7b')throw Error('Wrong reference');
const url=name=>pathToFileURL(resolve(root,'src/core/'+name+'.js'));
await import(url('difftool'));const {OPS}=await import(url('registry'));const op=OPS.find(o=>o.id==='diffTool');
const vectors=[];
for(const mode of ['byte','line']) for(const context of [0,1,3,20]) for(const [a,b] of [
 ['',''],['','hello'],['hello',''],['ABC','ABC'],['ABCDE','ABXDE'],['ABC','BCDA'],['kitten','sitting'],['中文🙂\n你好','中文🙃\n世界'],['a\r\nb\r\n','a\nb\n'],['a\rb','a\nb'],['a\nb\nc','a\nc'],['a\nb','x\na\nb\ny'],['0\0\xff','0\0\xfe'],['abababa','babab'],['ab '.repeat(80),'ac '.repeat(80)]
 ]) {
 const params={textB:b,mode,inputEnc:'utf8',context};const v={input:a,params};try{v.text=await op.run(a,params);}catch(e){v.error=e.message;}vectors.push(v);
}
for(const inputEnc of ['auto','hex','base64','utf8']) for(const [a,b] of [['',''],['00ff41','00ff42'],['00 ff 41','00 ff 41 42'],['SGVsbG8=','SGVsbG8h'],['YWJj','YWJk'],['YWJj','中文'],['Hello','World'],['abc','ab']]) {
 const params={textB:b,mode:'byte',inputEnc,context:1};const v={input:a,params};
 // Explicit invalid Hex silently zero-fills in the reference; Rust rejects it.
 if(inputEnc==='hex'&&[a,b].some(s=>/[^\da-f\s]/i.test(s)))continue;
 try{v.text=await op.run(a,params);}catch(e){v.error=e.message;}vectors.push(v);
}
for(const [a,b] of [['prefix'+'a'.repeat(1500)+'suffix','prefix'+'b'.repeat(1501)+'suffix'],['same\n'+'a\n'.repeat(1500)+'tail','same\n'+'b\n'.repeat(1501)+'tail']]) {
 const params={textB:b,mode:a.includes('\n')?'line':'byte',inputEnc:'utf8',context:0};vectors.push({input:a,params,text:await op.run(a,params)});
}
await writeFile('crates/misclab-core/tests/fixtures/ebctf-diff-batch18.json',JSON.stringify({revision,vectors},null,2)+'\n');
console.log(`Captured ${vectors.length} diff vectors.`);
