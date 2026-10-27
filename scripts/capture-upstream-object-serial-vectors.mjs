// Maintainer-only reference capture. No upstream runtime is shipped.
import{writeFile}from'node:fs/promises';import{execFileSync}from'node:child_process';import{resolve}from'node:path';import{pathToFileURL}from'node:url';
const root=resolve('.git/reference/EBCTFCodeBox'),revision=execFileSync('git',['-C',root,'rev-parse','HEAD'],{encoding:'utf8'}).trim();
if(revision!=='ea2bc36dae31c97060f7f2cddad8df5518afad7b')throw Error('Wrong reference');
await import(pathToFileURL(resolve(root,'src/core/serial.js')));const{OPS}=await import(pathToFileURL(resolve(root,'src/core/registry.js')));
const ids={bsonParse:'bson_parse',phpSerializeParse:'php_serialize_parse',javaSerializeIdent:'java_serialize_identify'},vectors=[];
async function capture(id,data,mode){
 const p={},v={operation:id,node:ids[id],params:{}};let s='';
 if(id==='phpSerializeParse'){s=data;v.input={data:{type:'text',value:s}};v.params={legacyReport:true};}
 else if(mode==='bytes'){p.rawBytes=data;v.input={data:{type:'bytes',value:[...data]}};}
 else{s=mode==='base64'?data.toString('base64'):data.toString('hex');p.inputEnc=mode;v.params={...p};v.input={data:{type:'text',value:s}};}
 if(id==='javaSerializeIdent')Object.assign(v.params,{legacyReport:true,stringEncoding:'utf8'});
 try{v.text=String(await OPS.find(o=>o.id===id).run(s,p));}catch(e){v.error=e.message;}vectors.push(v);
}
const int=(n,size=4)=>{const b=Buffer.alloc(size);if(size===8)b.writeBigInt64LE(BigInt(n));else b.writeInt32LE(n);return b;};
const cs=s=>Buffer.concat([Buffer.from(s),Buffer.from([0])]),str=s=>Buffer.concat([int(Buffer.byteLength(s)+1),cs(s)]);
const field=(t,name,b=Buffer.alloc(0))=>Buffer.concat([Buffer.from([t]),cs(name),b]);
const doc=(...fields)=>{const b=Buffer.concat([...fields,Buffer.from([0])]);return Buffer.concat([int(b.length+4),b]);};
const bsons=[Buffer.alloc(0),doc(),doc(field(16,'n',int(42))),doc(field(2,'中文',str('中文🙂'))),doc(field(2,'b',str('\ufeffA'))),doc(field(2,'empty',str(''))),doc(field(2,'nul',str('a\0b'))),doc(field(18,'large',int(9223372036854775807n,8))),doc(field(18,'negative',int(-9223372036854775808n,8))),doc(field(9,'date',int(1793059200000n,8))),doc(field(17,'ts',Buffer.concat([int(12),int(1793059200)]))),doc(field(7,'oid',Buffer.from('00112233445566778899aabb','hex'))),doc(field(8,'yes',Buffer.from([1])),field(8,'no',Buffer.from([0])),field(10,'empty')),doc(field(11,'regex',Buffer.concat([cs('a.*'),cs('im')]))),doc(field(255,'min'),field(127,'max')),doc(field(3,'child',doc(field(16,'n',int(1))))),doc(field(4,'array',doc(field(2,'0',str('A')),field(2,'1',str('B'))))),doc(field(16,'duplicate',int(1)),field(16,'duplicate',int(2)))];
for(const n of [0,1.5,-0,Infinity,NaN]){const b=Buffer.alloc(8);b.writeDoubleLE(n);bsons.push(doc(field(1,'float',b)));}
for(const sub of [0,1,2,3,4,5,6,7,128]){const raw=Buffer.from([0,255,128]),b=sub===2?Buffer.concat([int(raw.length),raw]):raw;bsons.push(doc(field(5,'data',Buffer.concat([int(b.length),Buffer.from([sub]),b]))));}
const phps=['','N;','b:0;','b:1;','i:0;','i:-1;','i:2147483648;','d:1.5;','d:-0.125;','d:1e20;','s:0:"";','s:5:"Hello";','s:3:"a\0b";','a:0:{}','a:2:{i:0;s:1:"A";i:1;s:1:"B";}','a:1:{s:3:"obj";O:3:"Foo":1:{s:1:"n";i:42;}}','O:3:"Foo":2:{s:1:"n";i:42;s:4:"text";s:2:"hi";}','C:3:"Foo":3:{abc}','a:1:{i:0;C:3:"Foo":3:{abc}}','a:1:{i:0;a:1:{i:1;N;}}','r:1;','R:42;','i:1;i:2;','  N;  ','a:1:{s:4:"\0*\0x";N;}'];
const javas=['','00','00000005','aced0005','aced0004','aced000570','aced000574000548656c6c6f','aced0005740006e4b8ade69687','aced0005740007efbbbf41f09f9880','aced00057c000000000000000548656c6c6f','aced0005770300ff80','aced00057a0000000300ff80','aced000571007e0000','aced000579787073','aced000575767b7d7e00','aced00057400014171007e00007974000142'];
for(const flags of [0,1,2,4,8,16,31])javas.push('aced0005720003466f6f0000000000000001'+flags.toString(16).padStart(2,'0')+'00007870');
javas.push('aced0005720003466f6f00000000000000010200024900016e4c0001737400124c6a6176612f6c616e672f537472696e673b7870');
for(const b of bsons)for(const m of ['auto','hex','base64','bytes'])await capture('bsonParse',b,m);
for(const s of phps)await capture('phpSerializeParse',s);
for(const h of javas)for(const m of ['auto','hex','base64','bytes'])await capture('javaSerializeIdent',Buffer.from(h,'hex'),m);
await writeFile('crates/misclab-core/tests/fixtures/ebctf-object-serial-batch10.json',JSON.stringify({revision,vectors},null,2).replaceAll('\uFFFD','\\uFFFD')+'\n');console.log(`Captured ${vectors.length} vectors.`);
