// Maintainer-only fixture capture; app/cargo tests never execute upstream JS.
import {writeFile} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
const root=resolve('.git/reference/EBCTFCodeBox');
const revision=execFileSync('git',['-C',root,'rev-parse','HEAD'],{encoding:'utf8'}).trim();
if(revision!=='ea2bc36dae31c97060f7f2cddad8df5518afad7b')throw Error('Wrong reference revision');
for(const file of ['radix','radixExt'])await import(pathToFileURL(resolve(root,'src/core/'+file+'.js')));
const {OPS}=await import(pathToFileURL(resolve(root,'src/core/registry.js')));
const ops=Object.fromEntries(OPS.map(o=>[o.id,o]));
const map={radixConvert:'radix_transform',asciiRadix:'ascii_radix',bcd:'bcd',binPad:'binary_pad',hybridCode:'hybrid_code',completion:'zero_align',negabase:'negative_base',balancedTernary:'balanced_ternary',factorialBase:'factorial_base',zeckendorf:'zeckendorf',roman:'roman',chineseNum:'chinese_number',continuedFraction:'continued_fraction',sternBrocot:'stern_brocot',separationAscii:'separate_ascii',asciiOffset:'ascii_offset',binaryComplement:'binary_complement',complementToDec:'complement_to_decimal',gcd:'gcd_lcm',fibonacci:'fibonacci_decode'};
const vectors=[];
async function capture(id,direction,input,params={}){
 const node=map[id]+(direction==='run'?'':'_'+direction);
 const port=id==='asciiRadix'&&direction==='encode'?'data':'text';
 const v={operation:id,node,input:{[port]:{type:'text',value:input}},params:{...params}};
 if(id==='factorialBase'&&direction==='decode')v.params.strict=false;
 let result;try{result=await ops[id][direction](input,params);}catch(e){v.error=String(e.message);vectors.push(v);return null;}
 v.text=Buffer.from(String(result),'utf8').toString('utf8');
 if(id==='asciiOffset'&&params.offset)v.bytes=[...Buffer.from(result,'utf16le')];
 vectors.push(v);return result;
}
async function pair(id,input,p={}){const encoded=await capture(id,'encode',input,p);if(encoded!==null)await capture(id,'decode',encoded,p);}
for(const [fromBase,toBase,input]of[[16,2,'FF'],[10,36,'-123456789012345678901234567890'],[2,16,'101101'],[36,10,'zzzz'],[10,2,'0'],[8,16,'+77']])await pair('radixConvert',input,{fromBase,toBase});
for(const base of [2,8,10,16,3,36])for(const input of ['','Hello','中文🙂'])await pair('asciiRadix',input,{base});
for(const bits of [7,8])for(const invert of [false,true])for(const bitReverse of [false,true])await pair('asciiRadix','Hello! 123',{base:2,bits,invert,bitReverse});
for(const base of [8,10,16])for(const compat of [false,true])await pair('asciiRadix','A B',{base,compat});
for(const [base,input]of[[2,'0100100001101001'],[16,'e4b8ad'],[10,'072105'],[8,'110151']])await capture('asciiRadix','decode',input,{base});
for(const compat of [false,true])for(const input of ['0','001234567890','2026-10-27',''])await pair('bcd',input,{compat});
for(const bits of [1,8,16,128])for(const input of ['0','1'])await pair('binPad',input,{bits});
await pair('binPad','18446744073709551616',{bits:128});
for(const mode of ['hex','dec','oct','bin','auto'])for(const sep of [' ',String.fromCharCode(92),','])await pair('hybridCode','Hi中🙂',{mode,sep});
await pair('hybridCode','');
for(const bits of [0,8,16])for(const input of ['0b1 0b101 0b11','0x1 0xff 0xa','1 02 333','中 🙂'])await pair('completion',input,{bits});
for(const base of [-2,-3,-10])for(const input of ['0','1','-1','13','-42','12345678901234567890'])await pair('negabase',input,{base});
for(const id of ['balancedTernary','chineseNum'])for(const input of ['0','1','-1','13','-42','12345678901234567890'])await pair(id,input);
for(const id of ['factorialBase','zeckendorf'])for(const input of ['0','1','2','10','100','463','123456789012345678901234567890'])await pair(id,input);
await capture('factorialBase','decode','1, 2 0');
await capture('zeckendorf','decode','1 1,0 1');
for(const input of ['1','4','9','49','1994','2026','3999'])await pair('roman',input);
await capture('roman','decode','mcmxciv');
for(const input of ['0','1','355/113','-22/7','22/-7','-22/-7','6/8','12345678901234567890/1234567'])await pair('continuedFraction',input);
await capture('continuedFraction','decode','[3; 7, 16]');
for(const input of ['1/1','5/3','1/42','42/1','6/8','123/456'])await pair('sternBrocot',input);
for(const input of ['', '(root)','l,r L'])await capture('sternBrocot','decode',input);
for(const input of ['','72101108108111','48656c6c6f','0100100001101001','310939249775','0','00072','not numbers'])await capture('separationAscii','run',input);
for(const offset of [-3,0,3,256])for(const input of ['','ABCxyz','中文','🙂']){await capture('asciiOffset','encode',input,{offset});await capture('asciiOffset','decode',input,{offset});}
for(const input of ['0','1','-1','127','-127','128','-129','32767','-32767','32768','-32769','2147483647'])await capture('binaryComplement','run',input);
for(const bits of [0,8,16,64])for(const input of ['0','1','10000000','11111111'])await capture('complementToDec','run',input,{bits});
for(const input of ['12,18,30','-12 18','0 5','42 42 42','1 2 3'])await capture('gcd','run',input);
for(const input of ['','3524578','flag 3524578/5702887','12中文'])await capture('fibonacci','run',input);
for(const[id,direction,input,params]of[
 ['radixConvert','encode','2',{fromBase:2,toBase:10}],['asciiRadix','encode','中',{base:2,bits:7}],['asciiRadix','decode','1',{base:2}],['asciiRadix','decode','999',{base:10}],
 ['bcd','decode','A',{}],['bcd','decode','1010',{compat:true}],['binPad','encode','256',{bits:8}],['binPad','encode','-1',{}],['binPad','decode','',{}],
 ['hybridCode','decode','z41',{}],['hybridCode','decode','x110000',{}],['completion','encode','',{}],['negabase','decode','2',{base:-2}],['balancedTernary','decode','2',{}],
 ['factorialBase','encode','-1',{}],['zeckendorf','encode','-1',{}],['roman','decode','IIII',{}],['roman','encode','4000',{}],['continuedFraction','encode','1/0',{}],
 ['sternBrocot','encode','0',{}],['sternBrocot','decode','X',{}],['complementToDec','run','001',{bits:2}],['gcd','run','1',{}]
])await capture(id,direction,input,params);
await writeFile('crates/misclab-core/tests/fixtures/ebctf-numeric-batch5.json',JSON.stringify({revision,vectors},null,2).replaceAll('\uFFFD','\\uFFFD')+'\n');
console.log(`Captured ${vectors.length} vectors for ${Object.keys(map).length} operations.`);
