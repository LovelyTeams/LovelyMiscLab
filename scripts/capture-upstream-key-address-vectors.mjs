// Maintenance oracle capture only; no JS or external tool is used by Rust nodes.
import { execFileSync } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash, createECDH } from 'node:crypto';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
const root=resolve('.git/reference/EBCTFCodeBox');
const revision=execFileSync('git',['-C',root,'rev-parse','HEAD'],{encoding:'utf8'}).trim();
if(revision!=='ea2bc36dae31c97060f7f2cddad8df5518afad7b')throw Error('Wrong reference revision');
const upstream=await import(pathToFileURL(resolve(root,'src/core/eccdetect.js')));
const vectors=[];
const text=value=>({type:'text',value});
function capture(id,input,params,fn,extra={}){const v={node:id,input,params,...extra};try{v.text=fn();}catch(e){v.error=String(e.message);}vectors.push(v);return v;}
const sshField=b=>{b=Buffer.from(b);const len=Buffer.alloc(4);len.writeUInt32BE(b.length);return Buffer.concat([len,b]);};
const mpint=n=>{let hex=n.toString(16);if(hex.length%2)hex='0'+hex;let b=Buffer.from(hex,'hex');if(b[0]&128)b=Buffer.concat([Buffer.from([0]),b]);return sshField(n===0n?Buffer.alloc(0):b);};
function ssh(type,parts){const blob=Buffer.concat([sshField(type),...parts]);const b64=blob.toString('base64');
  for(const input of [b64,`${type} ${b64}`,`${type} ${b64} 中文 demo user@example`,b64.replace(/=+$/,'').replaceAll('/','_').replaceAll('+','-')]){
    const v=capture('ssh_public_key_parse',{data:text(input)},{},()=>upstream.sshPubkeyParseReport(input),{bytes:[...blob]});
    if(v.error&&input===b64.replace(/=+$/,'').replaceAll('/','_').replaceAll('+','-')){
      // eccdetect's outer regex omits '-', even though its Base64 helper supports
      // URL-safe input. Preserve the upstream error and use the equivalent
      // standard Base64 blob as the independent expected parse result.
      v.upstreamError=v.error;delete v.error;v.text=upstream.sshPubkeyParseReport(b64);
      v.correction='Accept URL-safe Base64 dash consistently with the decoder';
    }
    v.fingerprint='SHA256:'+createHash('sha256').update(blob).digest('base64').replace(/=+$/,'');
  }
  capture('ssh_public_key_parse',{data:{type:'bytes',value:[...blob]}},{},()=>upstream.sshPubkeyParseReport(b64),{bytes:[...blob]});
}
for(const bits of [1,8,12,128,256,1024,2048,4096])ssh('ssh-rsa',[mpint(65537n),mpint((1n<<BigInt(bits))-1n)]);
for(const byte of [0,1,127,128,255])ssh('ssh-ed25519',[sshField(Buffer.alloc(32,byte))]);
ssh('ssh-dss',[23n,11n,2n,4n].map(mpint));
for(const [curve,native]of [['nistp256','prime256v1'],['nistp384','secp384r1'],['nistp521','secp521r1']]){
  const key=createECDH(native);key.setPrivateKey(Buffer.from([1]));
  for(const format of ['compressed','uncompressed'])ssh('ecdsa-sha2-'+curve,[sshField(curve),sshField(key.getPublicKey(null,format))]);
}
// The pinned text wrapper does not recognize '@' in a type prefix; raw blob does.
const sk=Buffer.concat([sshField('sk-ssh-ed25519@openssh.com'),sshField(Buffer.alloc(32,7)),sshField('ssh:demo')]);
capture('ssh_public_key_parse',{data:text(sk.toString('base64'))},{},()=>upstream.sshPubkeyParseReport(sk.toString('base64')),{bytes:[...sk]});
ssh('future-key',[Buffer.from([0,255,1,2,3])]);
for(const input of ['', 'abc', 'AAAA', 'ssh-rsa !!!', 'ssh-rsa AAAA'])capture('ssh_public_key_parse',{data:text(input)},{},()=>upstream.sshPubkeyParseReport(input));

const alphabet='123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
function base58(b){let n=BigInt('0x'+b.toString('hex'));let out='';while(n){out=alphabet[Number(n%58n)]+out;n/=58n;}let i=0;while(i<b.length&&b[i]===0){out='1'+out;i++;}return out;}
for(const version of [0,5,0x6f,0xc4])for(const byte of [0,1,127,128,255]){
  const payload=Buffer.concat([Buffer.from([version]),Buffer.alloc(20,byte)]);
  const sum=createHash('sha256').update(createHash('sha256').update(payload).digest()).digest().subarray(0,4);
  for(const corrupt of [false,true]){
    const checksum=Buffer.from(sum);if(corrupt)checksum[0]^=1;
    const address=base58(Buffer.concat([payload,checksum]));
    capture('bitcoin_address_inspect',{text:text(address)},{},()=>upstream.btcAddressIdentReport(address),{bytes:[...payload.subarray(1)],valid:!corrupt});
  }
}
for(const input of ['', '0OIl', '1', '中文', 'bc1abc', 'bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t5'])capture('bitcoin_address_inspect',{text:text(input)},{},()=>upstream.btcAddressIdentReport(input));
// Independently frame witness words (not the generic byte-payload Bech32 node).
function witness(hrp,version,program){
  let acc=0,bits=0;const words=[version];
  for(const b of program){acc=(acc<<8)|b;bits+=8;while(bits>=5){bits-=5;words.push((acc>>bits)&31);}}
  if(bits)words.push((acc<<(5-bits))&31);
  const input=[...[...hrp].map(c=>c.charCodeAt(0)>>5),0,...[...hrp].map(c=>c.charCodeAt(0)&31),...words,0,0,0,0,0,0];
  let check=1;for(const value of input){const top=check>>>25;check=((check&0x1ffffff)<<5)^value;for(const [i,g]of [0x3b6a57b2,0x26508e6d,0x1ea119fa,0x3d4233dd,0x2a1462b3].entries())if((top>>>i)&1)check^=g;}
  check^=version===0?1:0x2bc830a3;
  for(let i=0;i<6;i++)words.push((check>>>(5*(5-i)))&31);
  return hrp+'1'+words.map(w=>'qpzry9x8gf2tvdw0s3jn54khce6mua7l'[w]).join('');
}
for(const hrp of ['bc','tb','bcrt'])for(const version of [0,1,2,16])for(const len of version===0?[20,32]:[2,32,40]){
  const payload=Buffer.from(Array.from({length:len},(_,i)=>i&255));const address=witness(hrp,version,payload);const result=upstream.identifyBtcAddress(address);
  const script=Buffer.concat([Buffer.from([version===0?0:0x50+version,len]),payload]);
  capture('bitcoin_address_inspect',{text:text(address)},{},()=>result.lines,{bytes:[...payload],valid:true,network:result.net,scriptPubKey:script.toString('hex'),reportDifference:version>1||version===1&&len!==32?'Correct Bech32m and unknown-witness labeling':undefined});
}
const eth=[
  '0x52908400098527886E0F7030069857D2E4169EE7','0x8617E340B3D01FA5F11F306F4090FD50E238070D',
  '0xde709f2102306220921060314715629080e2fb77','0x27b1fdb04752bbc536007a920d24acb045561c26',
  '0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed','0xfB6916095ca1df60bB79Ce92cE3Ea74c37c5d359',
  '0xdbF03B407c01E7cD3CBea99509d93f8DDDC8C6FB','0xD1220A0cf47c7B9Be7A2E6BA89F429762e7b9aDb',
];
for(const expected of eth){for(const s of [expected,expected.toLowerCase(),expected.toUpperCase(),expected.slice(0,10).toLowerCase()+expected.slice(10)])capture('ethereum_address_inspect',{text:text(s)},{},()=>upstream.ethAddressIdentReport(s),{canonical:expected,bytes:[...Buffer.from(s.slice(2),'hex')]});}
for(let n=0;n<64;n++){const address='0x'+createHash('sha256').update('EBCTF fixture '+n).digest('hex').slice(0,40);const result=upstream.identifyEthAddress(address);capture('ethereum_address_inspect',{text:text(address)},{},()=>result.lines,{canonical:result.checksummed,bytes:[...Buffer.from(address.slice(2),'hex')]});}
for(const input of ['', '0x1', '0x'+'g'.repeat(40), '0x'+'a'.repeat(39), '0x'+'a'.repeat(41),'中文'])capture('ethereum_address_inspect',{text:text(input)},{},()=>upstream.ethAddressIdentReport(input));

// Published BIP-350 witness vectors, preserved independently from the upstream
// oracle because upstream fails to validate several invalid witness lengths.
const spec=await readFile('.git/reference/bip-0350.mediawiki','utf8');
const section=spec.slice(spec.indexOf('The following list gives valid segwit addresses'),spec.indexOf('==Appendix'));
const validSection=section.split('The following list gives invalid segwit addresses')[0];
const invalidSection=section.split('The following list gives invalid segwit addresses')[1];
const valid=[...validSection.matchAll(/\* <tt>([^<]+)<\/tt>: <tt>([0-9a-f]+)<\/tt>/g)].map(m=>({address:m[1],scriptPubKey:m[2]}));
const invalid=[...invalidSection.matchAll(/\* <tt>([^<]+)<\/tt>: ([^\r\n]+)/g)].map(m=>({address:m[1],reason:m[2]}));
if(valid.length!==8||invalid.length!==15)throw Error('BIP-350 vector section changed');
for(const v of valid){const r=upstream.identifyBtcAddress(v.address);const ver=Number.parseInt(v.scriptPubKey.slice(0,2),16);const prog=Buffer.from(v.scriptPubKey.slice(4),'hex');
  capture('bitcoin_address_inspect',{text:text(v.address)},{},()=>r.lines,{bytes:[...prog],valid:true,scriptPubKey:v.scriptPubKey,reportDifference:ver>0x51||ver===0x51&&prog.length!==32?'Correct Bech32m and unknown-witness labeling':undefined});
}
await writeFile('crates/misclab-core/tests/fixtures/bip350-segwit.json',JSON.stringify({source:'https://github.com/bitcoin/bips/blob/master/bip-0350.mediawiki',license:'BSD-2-Clause',retrievedAt:'2026-10-28',sourceSha256:createHash('sha256').update(spec).digest('hex'),valid,invalid},null,2)+'\n');
await writeFile('crates/misclab-core/tests/fixtures/ebctf-key-address-batch23.json',JSON.stringify({revision,vectors},null,2)+'\n');
console.log(JSON.stringify({vectors:vectors.length,bip350:valid.length+invalid.length,eth55:eth.length}));
