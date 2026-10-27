// Fixed reference vectors; application algorithms are Rust only.
import{writeFile}from'node:fs/promises';
import{execFileSync}from'node:child_process';
import{resolve}from'node:path';
import{pathToFileURL}from'node:url';
process.env.TZ='Asia/Shanghai';
const root=resolve('.git/reference/EBCTFCodeBox');
const revision=execFileSync('git',['-C',root,'rev-parse','HEAD'],{encoding:'utf8'}).trim();
if(revision!=='ea2bc36dae31c97060f7f2cddad8df5518afad7b')throw Error('Wrong reference');
for(const f of ['radix','radixExt'])await import(pathToFileURL(resolve(root,'src/core/'+f+'.js')));
const{OPS}=await import(pathToFileURL(resolve(root,'src/core/registry.js')));
const ops=Object.fromEntries(OPS.map(o=>[o.id,o]));
const vectors=[];
async function capture(id,dir,input,params={}){
 let node,port='text',p={...params};
 if(id==='ieee754'){node='ieee754_'+dir;if(params.precision==='half')p.halfMode='upstream';}
 else if(id==='decimalToFloat')node='decimal_to_radix_float';
 else if(id==='floatToDecimal')node='radix_float_to_decimal';
 else if(id==='standardCode'){const s=input.trim().replace(/\s/g,'');const isHex=/^[0-9a-f]+$/i.test(s)&&s.length%2===0;node='charset_probe_'+(isHex?'decode':'encode');port=isHex?'data':'text';if(!isHex)p.legacyReport=true;}
 else{node=params.dir==='toTime'?'timestamp_to_time':params.dir==='toStamp'?'time_to_timestamp':'timestamp_report';p={timezone:'+08:00'};}
 const v={operation:id,node,input:{[port]:{type:'text',value:input}},params:p};let result;try{result=await ops[id][dir](input,params);v.text=String(result);}catch(e){v.error=e.message;}vectors.push(v);return result;
}
for(const precision of ['half','single','double'])for(const s of ['','0','-0','1','-1','1.1','0.1','1.00048828125','3.141592653589793','65504','65520','1e-10','1e-15','5.960464477539063e-8','1.1754943508222875e-38','1e-40','Infinity','-Infinity','1e308','NaN','中文']){
 const e=await capture('ieee754','encode',s,{precision});if(e!==undefined)await capture('ieee754','decode',e,{precision});
}
for(const [precision,hexes]of [['half',['0001','03FF','0400','8000','7C01','7E00','FC00','3c00',' 3 c 0 0 ','']],['single',['00000001','7FC00000','80000000','FF800000','3f8','']],['double',['0000000000000001','7FF8000000000000','8000000000000000','FFF0000000000000','3FF0000000000000','']]])for(const s of hexes)await capture('ieee754','decode',s,{precision});
for(const precision of [1,2,16,64])for(const s of ['','0','-0','10.625','-10.625','0.1','255','1e21','1e-10','0xFF','中文'])await capture('decimalToFloat','run',s,{precision});
for(const [base,inputs]of [[2,['1010.101','-0.1','0b11.01','.1','0.1','']],[8,['12.5','-0o12.5','0.1','8']],[10,['10.625','0.1','-0.0001','2.5','3.5','1.005','0x10']],[16,['a.a','-0xa.a','0BEEF','0.1','FF.FF','z']]])for(const precision of [0,1,2,16,64])for(const s of inputs)await capture('floatToDecimal','run',s,{base:String(base),precision});
for(const s of ['','Hello','中文🙂','中文','café','こんにちは','48656C6C6F','e4b8ade69687','EFBBBF41','FFFE4100','FEFF0041','ff00','800081829F','d6d0cec4','c4e3bac3','00','A','AB','🙂'])await capture('standardCode','run',s);
for(const dir of ['auto','toTime'])for(const s of ['0','1793059200','1793059200123','-1','1.25','-1000000000000'])await capture('timestamp','run',s,{dir});
for(const dir of ['auto','toStamp'])for(const s of ['1970-01-01','2026-10-27 08:00:00','2026-10-27T08:00:00','2026-10-27T08:00:00+08:00','2026-10-27T00:00:00Z','2026/10/27','2026-10-27T00:00:00.123Z','','wrong'])await capture('timestamp','run',s,{dir});
await writeFile('crates/misclab-core/tests/fixtures/ebctf-float-charset-batch8.json',JSON.stringify({revision,timezone:'Asia/Shanghai',vectors},null,2).replaceAll('\uFFFD','\\uFFFD')+'\n');
console.log('Captured '+vectors.length+' vectors across 5 operations.');
