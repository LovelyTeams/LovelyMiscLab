// Upstream reference capture only; production and tests execute Rust.
import{writeFile}from'node:fs/promises';
import{execFileSync}from'node:child_process';
import{resolve}from'node:path';
import{pathToFileURL}from'node:url';
const root=resolve('.git/reference/EBCTFCodeBox');
const revision=execFileSync('git',['-C',root,'rev-parse','HEAD'],{encoding:'utf8'}).trim();
if(revision!=='ea2bc36dae31c97060f7f2cddad8df5518afad7b')throw Error('Wrong reference');
await import(pathToFileURL(resolve(root,'src/core/serial.js')));
const{OPS}=await import(pathToFileURL(resolve(root,'src/core/registry.js')));
const names={protobufParse:'protobuf_wire_parse',msgpackParse:'msgpack_parse',cborParse:'cbor_parse'};
const vectors=[];
async function capture(id,bytes,mode){
 const buf=Buffer.from(bytes),p={},v={operation:id,node:names[id],params:{}};let input='';
 if(mode==='bytes'){p.rawBytes=buf;v.input={data:{type:'bytes',value:[...buf]}};}
 else{input=mode==='base64'?buf.toString('base64'):mode==='utf8'?buf.toString('utf8'):buf.toString('hex');p.inputEnc=mode;v.params={...p};v.input={data:{type:'text',value:input}};}
 try{v.text=String(await OPS.find(o=>o.id===id).run(input,p));}catch(e){v.error=e.message;}
 vectors.push(v);
}
const fixtures={
 protobufParse:['','0800','089601','08ffffffffffffffffff01','09ffffffffffffffff','0d0000803f','120548656c6c6f','1206e4b8ade69687','1207efbbbf41f09f9880','12020801','12021200','1200','1203ff00ff','1b08011c','080112030801ff','0801120208ff','08011500','0001','0e','08','80','120aff'],
 msgpackParse:['','00','7f','e0','ff','c0','c2','c3','ccff','cd1234','ceffffffff','cfffffffffffffffff','d080','d18000','d280000000','d38000000000000000','ca3fc00000','cb400921fb54442d18','ca7f800000','cb7ff8000000000000','a0','a548656c6c6f','a6e4b8ade69687','a4efbbbf41','a2ff00','d903616263','da0003616263','db00000003616263','c40300ff80','c5000300ff80','c60000000300ff80','c703ff00ff80','c800030100ff80','c9000000030200ff80','d4ff41','d5014142','d60241424344','d7030001020304050607','d804000102030405060708090a0b0c0d0e0f','90','9301a161c3','dc00020102','dd000000020102','80','82a16101a162c3','de000101a161','df00000001c001','81c2c3','9181a16e2a','0102'],
 cborParse:['','00','17','1818','190100','1a00010000','1bffffffffffffffff','20','37','3818','39ffff','3bffffffffffffffff','40','4300ff80','580300ff80','59000300ff80','6000','63616263','66e4b8ade69687','64efbbbf41','62ff00','78186162636465666768696a6b6c6d6e6f707172737475767778','790003616263','5f4201024103ff','7f61616162ff','80','83016161f5','98020102','9900020102','9f018202039f0405ffff','a0','a26161016162f5','b801016161','bf6161016162f6ff','c074323031332d30332d32315432303a30343a30305a','d86401','dbffffffffffffffff01','f4','f5','f6','f7','f0','f93c00','f90001','f98000','f97c00','f97e00','fa3fc00000','fb400921fb54442d18','0102'],
};
for(const[id,hexes]of Object.entries(fixtures))for(const hex of hexes)for(const mode of ['auto','hex','base64','bytes'])await capture(id,Buffer.from(hex,'hex'),mode);
for(const id of Object.keys(names))for(const text of ['','A','中文','🙂'])await capture(id,Buffer.from(text),'utf8');
await writeFile('crates/misclab-core/tests/fixtures/ebctf-serial-batch9.json',JSON.stringify({revision,vectors},null,2).replaceAll('\uFFFD','\\uFFFD')+'\n');
console.log(`Captured ${vectors.length} vectors.`);
