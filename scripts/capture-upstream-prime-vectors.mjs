// Maintenance-only reference capture. All application arithmetic runs in Rust.
import {readFile,writeFile} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
const root=resolve('.git/reference/EBCTFCodeBox');
const revision=execFileSync('git',['-C',root,'rev-parse','HEAD'],{encoding:'utf8'}).trim();
if(revision!=='ea2bc36dae31c97060f7f2cddad8df5518afad7b')throw Error('Wrong reference');
const url=name=>pathToFileURL(resolve(root,'src/core/'+name+'.js'));
const {isProbablePrime}=await import(url('primeGen'));
const {OPS}=await import(url('registry'));
if(process.argv[2]==='--verify-rust') {
 const samples=JSON.parse(await readFile(process.argv[3],'utf8'));
 for(const {bits,primes} of samples) for(const s of primes) {const n=BigInt(s);if(n.toString(2).length!==bits || !isProbablePrime(n)) throw Error('Rust candidate fails reference');}
 console.log(`Verified ${samples.reduce((n,s)=>n+s.primes.length,0)} Rust candidates with pinned upstream.`);process.exit(0);
}
const vectors=[];
async function cap(id,p) {
 const op=OPS.find(o=>o.id===id);const params={...Object.fromEntries(op.params.map(p=>[p.key,p.default])),...p};
 const v={operation:id,params};
 try {v.text=await op.run('',params);}catch(e){v.error=e.message;}
 vectors.push(v);
}
for(const n of ['','中文🙂','0x17','1.5','2e3','--2','-17','-1','0','1','2','3','4','5','9','25','97','561','1105','1729','2047','3215031751','341550071728321','3825123056546413051','2305843009213693951','18446744073709551557','18446744073709551615','318665857834031151167461','3317044064679887385961980','3317044064679887385961981','3317044064679887385961982',(2n**127n-1n).toString(),(2n**521n-1n).toString(),(2n**128n+1n).toString(),'+2,305_843 009213693951']) for(const rounds of [1,13,24,64]) await cap('primeTest',{n,rounds});
for(let rounds=1;rounds<=64;rounds++) await cap('primeTest',{n:'3317044064679887385961981',rounds});
for(const count of [1,2,100]) await cap('primeGen',{bits:2,count});
const {setRandConfig}=await import(url('randSource'));setRandConfig({mode:'custom',seed:'LovelyMiscLab prime reference'});
const generated=[];
for(const bits of [3,8,16,32,64,128,256,512,1024]) {const s=await OPS.find(o=>o.id==='primeGen').run('',{bits,count:1});generated.push({bits,n:s});}
await writeFile('crates/misclab-core/tests/fixtures/ebctf-prime-batch17.json',JSON.stringify({revision,vectors,generated},null,2)+'\n');
console.log(`Captured ${vectors.length} cases and ${generated.length} generated candidates.`);
