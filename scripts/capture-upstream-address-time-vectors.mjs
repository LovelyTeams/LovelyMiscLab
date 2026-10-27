// Maintainer reference capture only; never loaded by the app or cargo tests.
import{writeFile}from'node:fs/promises';
import{execFileSync}from'node:child_process';
import{resolve}from'node:path';
import{pathToFileURL}from'node:url';
const root=resolve('.git/reference/EBCTFCodeBox');
const revision=execFileSync('git',['-C',root,'rev-parse','HEAD'],{encoding:'utf8'}).trim();
if(revision!=='ea2bc36dae31c97060f7f2cddad8df5518afad7b')throw Error('Wrong reference');
for(const f of ['netcodec','netcodecExt','timecodec','timecodecExt','radixExt','radixAll'])await import(pathToFileURL(resolve(root,'src/core/'+f+'.js')));
const{OPS}=await import(pathToFileURL(resolve(root,'src/core/registry.js')));
const ops=Object.fromEntries(OPS.map(o=>[o.id,o]));
const map={ipv4Int:['ipv4_integer_encode','ipv4_integer_decode'],ipv6Format:['ipv6_compress','ipv6_expand'],macFormat:['mac_format_encode','mac_format_decode'],cidrCalc:['cidr_calculate'],userAgentParse:['user_agent_parse'],magnetParse:['magnet_parse'],unixTime:['unix_time_encode','unix_time_decode'],filetime:['windows_filetime_encode','windows_filetime_decode'],hfsTime:['hfs_time_encode','hfs_time_decode'],cocoaTime:['cocoa_time_encode','cocoa_time_decode'],dosDateTime:['dos_datetime_encode','dos_datetime_decode'],chineseDate:['chinese_date_encode','chinese_date_decode'],tzConvert:['timezone_convert'],julianDate:['julian_date_encode','julian_date_decode'],excelDate:['excel_date_encode','excel_date_decode'],chromeTime:['chrome_time_encode','chrome_time_decode'],snowflakeId:['snowflake_parse'],primeFactor:['prime_factorization'],collatz:['collatz_sequence'],radixAll:['radix_all']};
const vectors=[];
async function capture(id,direction,input,params={}){const v={operation:id,node:map[id][direction==='decode'?1:0],input:{text:{type:'text',value:input}},params};let result;try{result=await ops[id][direction](input,params);v.text=String(result);}catch(e){v.error=e.message;}vectors.push(v);return result;}
async function pair(id,input,p={}){const e=await capture(id,'encode',input,p);if(e!==undefined)await capture(id,'decode',String(e),p);}
for(const format of ['dec','hex','oct','bin'])for(const compat of [false,true])for(const s of ['0.0.0.0','192.168.1.1','255.255.255.255','127.1','10.256','10.0.65535','0x7f.01.0b1.1','2130706433'])await pair('ipv4Int',s,{format,compat});
for(const s of ['::','::1','2001:db8::1','2001:db8:0:1:0:0:0:1','1:0:0:2:0:0:3:4','1:0:2:3:4:5:6:7','::ffff:192.168.1.1','FE80::1234','::ffff:127.1'])await pair('ipv6Format',s);
for(const format of ['colon','hyphen','dot','int'])for(const s of ['aa:bb:cc:dd:ee:ff','00:00:00:00:00:00','ff-ff-ff-ff-ff-ff','0011.2233.4455','187723572702975','0xabcdef123456'])await pair('macFormat',s,{format});
for(const s of ['0.0.0.0/0','192.168.1.99/24','10.0.0.1/31','10.0.0.1/32','172.16.0.1/16','172.32.0.1/16','224.1.1.1/4','240.0.0.1/4','127.1/8'])await capture('cidrCalc','run',s);
for(const s of [
'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120.0 Safari/537.36 Edg/120.0',
'Mozilla/5.0 (Windows NT 6.1; WOW64; Trident/7.0; rv:11.0) like Gecko',
'Mozilla/5.0 (Windows NT 5.1) MSIE 8.0','Mozilla/5.0 (Linux; Android 13; Pixel 7) AppleWebKit/537.36 Chrome/120.0 Mobile Safari/537.36',
'Mozilla/5.0 (iPhone; CPU iPhone OS 17_1 like Mac OS X) Version/17.0 Mobile Safari/605.1',
'Mozilla/5.0 (iPad; CPU OS 16_0 like Mac OS X) Version/16.0 Safari/605.1',
'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1 Version/17.0 Safari/605.1',
'Mozilla/5.0 (Linux; rv:123.0) Gecko/20100101 Firefox/123.0','CrOS Chrome/99.0 OPR/88.0','Linux','未知🙂',''
])await capture('userAgentParse','run',s);
for(const s of ['magnet:?xt=urn:btih:abcdef0123&dn=a+b&tr=udp%3A%2F%2Fexample&tr.1=second','MAGNET:?xt.1=urn:btih:ABCD&xt.2=urn:sha1:000&xl=100&ws=http://example&as=foo&xs=bar&kt=k&mt=m&x=v','magnet:?xt=urn:btih:abc&dn=%E4%B8%AD%F0%9F%99%82','magnet:?xt=unknown&dn=%ZZ','magnet:?xt=unknown&dn=%FF','magnet:?xt=urn:btih:abc','magnet:?dn=only','magnet:?xt=',''])await capture('magnetParse','run',s);
// Full ISO time avoids V8's legacy Date.parse("0001-01-01Z") -> 2001 quirk.
for(const unit of ['auto','sec','ms','us'])for(const s of ['1970-01-01T00:00:00Z','1969-12-31T23:59:59.123Z','2026-10-27T12:34:56.789Z','2000-02-29T12:34:56+08:00','0001-01-01T00:00:00Z'])await pair('unixTime',s,{unit});
for(const s of ['-1000','0','1793059200','1793059200123','1793059200123456','-1793059200123456','1.25'])await capture('unixTime','decode',s,{unit:'auto'});
for(const id of ['filetime','hfsTime','cocoaTime','chromeTime'])for(const s of ['1601-01-01','1904-01-01','1970-01-01','2001-01-01','2026-10-27T12:34:56.789Z','2040-02-06T06:28:15Z'])await pair(id,s);
for(const [id,s]of [['filetime','18446744073709551615'],['filetime','0x19DB1DED53E8000'],['hfsTime','4294967295'],['cocoaTime','-1.5'],['chromeTime','-1001']])await capture(id,'decode',s);
for(const s of ['1980-01-01','2026-10-27T12:34:57Z','2000-02-29T00:00:00Z','2107-12-31T23:59:58Z'])await pair('dosDateTime',s);
for(const s of ['0000-01-01T00:00:00Z','0099-02-28T00:00:00Z','2000-02-29','2026-10-27','2026-12-31','2026-01-10','2026-11-20','2026-03-30'])await pair('chineseDate',s);
for(const fromTz of ['UTC','+8','-05:30'])for(const toTz of ['UTC','+08:00','-0430'])for(const s of ['2026-10-27T01:00:00','2026-10-27T01:00:00+02:00'])await capture('tzConvert','run',s,{fromTz,toTz});
for(const s of ['2000-01-01T12:00:00Z','1970-01-01','2026-10-27T12:34:56Z','1582-10-15','1500-01-01'])await pair('julianDate',s);
for(const system of ['1900','1904'])for(const s of ['1899-12-31','1900-01-01','1900-02-28','1900-03-01','1904-01-01','1970-01-01T12:00:00Z','2026-10-27T12:34:56Z'])await pair('excelDate',s,{system});
for(const epoch of ['twitter','discord','custom'])for(const s of ['0','175928847299117063','9223372036854775807',''])await capture('snowflakeId','run',s,{epoch,customEpoch:'0'});
for(const s of ['0','1','2','360','65537','1000036000099','000360','中文','-1',''])await capture('primeFactor','run',s);
for(const s of ['1','2','27','123456789','0','-1',''])await capture('collatz','run',s);
for(const s of ['0','00','255','1010','-1010','1_000_000','1,000,000','0x1F600','0xd800','0x110000','-0xff','-1','0b1010','0o777','0x1'+'0'.repeat(64),'','ff'])await capture('radixAll','run',s);
for(const [id,dir,s,p]of[
 ['ipv4Int','encode','256.0.0.1',{}],['ipv4Int','encode','1.16777216',{}],['ipv4Int','decode','4294967296',{}],['ipv6Format','encode','1::2::3',{}],['ipv6Format','encode','1:2:3',{}],['macFormat','encode','aa:bb',{}],['macFormat','encode','281474976710656',{}],['cidrCalc','run','1.2.3.4/33',{}],
 ['unixTime','decode','',{}],['filetime','decode','-1',{}],['filetime','decode','18446744073709551616',{}],['hfsTime','decode','-1',{}],['hfsTime','decode','4294967296',{}],['cocoaTime','decode','',{}],['dosDateTime','encode','1979-12-31',{}],['chineseDate','decode','2026年一月一日',{}],['tzConvert','run','x',{}],['excelDate','decode','60',{}],['snowflakeId','run','9223372036854775808',{}],['snowflakeId','run','-1',{}],
])await capture(id,dir,s,p);
await writeFile('crates/misclab-core/tests/fixtures/ebctf-address-time-batch6.json',JSON.stringify({revision,vectors},null,2).replaceAll('\uFFFD','\\uFFFD')+'\n');
console.log('Captured '+vectors.length+' vectors across '+Object.keys(map).length+' operations.');
