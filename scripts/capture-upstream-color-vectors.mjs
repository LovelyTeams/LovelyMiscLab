// Maintenance only: pinned reference data, never loaded as application code.
import { writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
const root=resolve('.git/reference/EBCTFCodeBox');
const revision=execFileSync('git',['-C',root,'rev-parse','HEAD'],{encoding:'utf8'}).trim();
if(revision!=='ea2bc36dae31c97060f7f2cddad8df5518afad7b') throw Error('Wrong reference');
const url=name=>pathToFileURL(resolve(root,'src/core/'+name+'.js'));
const {CSS_NAMED,FORMATS}=await import(url('color'));
const {OPS}=await import(url('registry'));
const vectors=[];
async function cap(id,dir,text,override={}) {
 const op=OPS.find(o=>o.id===id);
 const params={...Object.fromEntries((op.params||[]).map(p=>[p.key,p.default])),...override};
 const v={operation:id,node:id==='colorInfo'?'color_info':'color_convert_'+dir,input:{text:{type:'text',value:text}},params};
 try {v.text=await op[dir](text,params);}catch(e){v.error=e.message;}
 vectors.push(v);return v;
}
for (const [name,hex] of Object.entries(CSS_NAMED)) {
 await cap('colorInfo','run',name);
 for (const to of FORMATS) {const v=await cap('color','encode',name,{from:'cssname',to}); if(v.text) await cap('color','decode',v.text,{from:'cssname',to});}
 await cap('color','encode',hex,{from:'hex',to:'hex',upper:false});
}
const samples={
 auto:['red','#123','#ff0080','0xffa500','000001','255','1,1,1','1,2,3','0.2 0.3 0.4','255/128/0','hsl(300,100%,50%)','hsv(-60, 50%, 100%)','cmyk(0,1,1,0)','rgb(1,2,3)','rgb(100%,0%,0%)',' light blue '],
 hex:['#ABC','abcdef','0x123456','#000000','#ffffff'],
 rgb:['0 0 0','0.5 0.5 0.5','127,128,129','rgb(-1, 300, 20)','1,2,3'],
 hsl:['0,100,50','360,100,50','-120,1,0.5','hsl(33.3,66.6%,44.4%)','0,0,0','270,0,1','720,500,-1'],
 hsv:['0,100,100','120,1,1','240,1,1','-360,0,0','hsv(59.9,33.3%,66.6%)','800,400,-1'],
 cmyk:['0,0,0,0','0 0 0 100','0,100,100,0','cmyk(11.1%,22.2%,33.3%,44.4%)','2,0.5,1,0.1'],
 int:['0','1','255','16777215','0xff0080','ff00ff','#123456'],
 cssname:['RED',' Light Blue ','cyan','magenta','grey','darkgrey'],
};
for(const [from,list] of Object.entries(samples)) for(const s of list) for(const to of FORMATS) for(const upper of [true,false]) {
 const v=await cap('color','encode',s,{from,to,upper});if(v.text) await cap('color','decode',v.text,{from,to,upper});
}
for (const s of ['', '中文🙂','transparent','rebeccapurple','bad-color','#12','#abcd','#aabbccdd','16777216','-1','rgb(1,2)','cmyk(1,2,3)','hsl(1,2)','0,0','10 20 30 40 50']) {
 await cap('colorInfo','run',s);await cap('color','encode',s);
}
await writeFile('crates/misclab-core/src/nodes/css_named_colors.json',JSON.stringify(Object.entries(CSS_NAMED).map(([name,hex])=>[name,[...Buffer.from(hex.slice(1),'hex')]]),null,2)+'\n');
await writeFile('crates/misclab-core/tests/fixtures/ebctf-color-batch16.json',JSON.stringify({revision,namedColors:Object.keys(CSS_NAMED).length,vectors},null,2)+'\n');
console.log(`Captured ${vectors.length} vectors and ${Object.keys(CSS_NAMED).length} named colors.`);
