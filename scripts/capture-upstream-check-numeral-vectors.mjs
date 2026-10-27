// Fixed-reference fixture capture; no upstream algorithms are used at runtime.
import{writeFile,readFile}from'node:fs/promises';import{execFileSync}from'node:child_process';import{resolve}from'node:path';import{pathToFileURL}from'node:url';
const root=resolve('.git/reference/EBCTFCodeBox'),revision=execFileSync('git',['-C',root,'rev-parse','HEAD'],{encoding:'utf8'}).trim();if(revision!=='ea2bc36dae31c97060f7f2cddad8df5518afad7b')throw Error('Wrong reference');
for(const file of ['checkdigit','iban','mayaNumerals','babylonianNumerals','egyptianNumerals','pipNumerals'])await import(pathToFileURL(resolve(root,'src/core/'+file+'.js')));
const{OPS}=await import(pathToFileURL(resolve(root,'src/core/registry.js'))),{IBAN_LENGTH}=await import(pathToFileURL(resolve(root,'src/core/iban.js')));
const table=JSON.parse(await readFile('crates/misclab-core/src/nodes/checkdigit_tables.json','utf8'));if(JSON.stringify(IBAN_LENGTH)!==JSON.stringify(table.iban))throw Error('IBAN snapshot drift');
const stem={luhn:'luhn',isbn:'isbn',ean13:'ean13',cnidCheck:'cnid_check',upc:'upc_a',iban:'iban',mayaNumerals:'maya_numerals',babylonianNumerals:'babylonian_numerals',egyptianNumerals:'egyptian_numerals',pipNumerals:'pip_numerals'};
const vectors=[];
async function cap(id,dir,s,p={}){const numeric=['luhn','isbn','ean13','cnidCheck','upc','iban'].includes(id);const node=id==='bankBin'?'bank_bin_identify':stem[id]+'_'+(numeric?(dir==='encode'?'calculate':'validate'):dir);const v={operation:id,node,input:{text:{type:'text',value:s}},params:p};let result;try{result=await OPS.find(o=>o.id===id)[dir](s,p);v.text=String(result);}catch(e){v.error=e.message;}vectors.push(v);return result;}
for(const[id,bodies]of Object.entries({luhn:['0','1','7992739871','123456789','00','中文 799-273.9871'],isbn:['030640615','097522980','978030640615','0-306-40615','000000000'],ean13:['400638133393','000000000000','123456789012'],cnidCheck:['00000000000000000','11111111111111111','12345678901234567'],upc:['03600029145','00000000000','12345678901']})){
 for(const s of bodies){const c=await cap(id,'encode',s);if(c!==undefined){const clean=s.replace(/[^0-9]/g,'');await cap(id,'decode',clean+c);await cap(id,'decode',clean+(c==='0'?'1':'0'));}}
 for(const s of ['','中文','🙂','1','１２３','X','12x','0/1-2.3'])for(const dir of ['encode','decode'])await cap(id,dir,s);
}
for(const cc of Object.keys(IBAN_LENGTH)){const body=cc+'0'.repeat(IBAN_LENGTH[cc]-4),full=await cap('iban','encode',body);await cap('iban','decode',full);await cap('iban','decode',cc+'00'+full.slice(4));}
for(const s of ['GBWEST12345698765432','GB WEST 1234 5698 7654 32','gbwest12345698765432','ZZ123456','GBW?ST12345698765432','','中文','🙂'])await cap('iban','encode',s);
for(const s of ['GB82 WEST 1234 5698 7654 32','GB-82-WEST12345698765432','XX000000','GBX2WEST12345698765432','GB83WEST12345698765432','','中文'])await cap('iban','decode',s);
for(const s of [...table.bank.map(x=>x.prefix.padEnd(16,'0')),'4111','621700','1234567890','999999','12','中文','4111111111111111'])await cap('bankBin','run',s);
for(const mode of ['vigesimal','longcount'])for(const form of ['unicode','dotbar'])for(const s of ['0','1','5','14','17','19','20','26','33','360','400','406','429','7200','144000','123456789012345678901234567890','','01','-1','1.2','中文']){const p={mode,form};const out=await cap('mayaNumerals','encode',s,p);if(out!==undefined)await cap('mayaNumerals','decode',out,p);}
for(const form of ['unicode','ascii'])for(const s of ['0','1','10','23','59','60','61','100','3842','8583','123456789012345678901234567890','','01','-1','1.2','中文']){const p={form},out=await cap('babylonianNumerals','encode',s,p);if(out!==undefined)await cap('babylonianNumerals','decode',out,p);}
for(const s of ['1','9','10','123','203','513','2001','1000000','12345678','9999999999','0','01','10000000000','-1','','中文']){const out=await cap('egyptianNumerals','encode',s);if(out!==undefined)await cap('egyptianNumerals','decode',out);}
for(const s of ['123456','1 2\n3\t4','0','7','9','中文','🙂','🎲','',' ']){const out=await cap('pipNumerals','encode',s);if(out!==undefined)await cap('pipNumerals','decode',out);}
for(const orientation of ['horizontal','vertical'])for(const s of ['00','66','01 23/45|60,(12)','00010203040506','123','77','中文','',' ']){const p={system:'domino',orientation},out=await cap('pipNumerals','encode',s,p);if(out!==undefined)await cap('pipNumerals','decode',out,p);}
for(const[id,p,inputs]of[
 ['mayaNumerals',{form:'dotbar'},['.....','----','-.','0 .','...-- ,; .','中']],['mayaNumerals',{mode:'longcount',form:'dotbar'},['...--- 0','0 0','. 0 0']],['mayaNumerals',{},['𝋠𝋡','𝋠 𝋠','中','🙂']],
 ['babylonianNumerals',{form:'ascii'},['|<','||||||||||','0 |','<<<<<<','中']],['babylonianNumerals',{},['𒁹𒁹','0 𒐕','𒐕,𒐕','🙂']],['egyptianNumerals',{},['𓏺𓍢𓎆','𓏺 ,;𓏺','0','𓏨','🙂']],['pipNumerals',{},['⚀⚁⚂ ⚃⚄⚅','🎲','0','🙂']],['pipNumerals',{system:'domino',orientation:'horizontal'},['🀱','🁣','🀰','🁢','🙂']],
])for(const s of inputs)await cap(id,'decode',s,p);
await writeFile('crates/misclab-core/tests/fixtures/ebctf-check-numerals-batch11.json',JSON.stringify({revision,vectors},null,2)+'\n');console.log(`Captured ${vectors.length} vectors across ${new Set(vectors.map(v=>v.operation)).size} operations.`);
