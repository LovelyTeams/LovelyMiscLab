// Maintenance oracle only. Runtime MIME/BinHex implementations are pure Rust.
import { execFileSync } from 'node:child_process';
import { writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
const root = resolve('.git/reference/EBCTFCodeBox');
const revision = execFileSync('git', ['-C', root, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
if (revision !== 'ea2bc36dae31c97060f7f2cddad8df5518afad7b') throw Error('Wrong reference revision');
const url = name => pathToFileURL(resolve(root, 'src/core/' + name + '.js'));
const mime = await import(url('mimeMultipart'));
const bin = await import(url('binhex'));
const vectors = [];
const text = value => ({ type: 'text', value });
const bytes = value => ({ type: 'bytes', value: [...value] });
function capture(node, input, params, calculate, extra = {}) {
  const v = { node, input, params, ...extra };
  try { v.text = calculate(); } catch (e) { v.error = String(e.message); }
  vectors.push(v);
  return v;
}
for (const filename of ['file.txt', 'a', ' x.bin ', '题目.txt', 'a'.repeat(63)]) {
  for (const input of ['', 'a', 'ab', 'abc', 'aaaa', 'a'.repeat(255), 'a'.repeat(256), 'a'.repeat(512), 'Hello\r\nworld\t!', 'café', '\x80\x90\xFF', '中文😀', '\0\x01\xFF', '123456789']) {
    const v = capture('binhex_encode', { data: text(input) }, { filename }, () => bin.binhexEncode(input, { filename }));
    const b = [...input].every(c => c.codePointAt(0) <= 255) ? Buffer.from(input, 'latin1') : Buffer.from(input, 'utf8');
    capture('binhex_decode', { text: text(v.text) }, {}, () => bin.binhexDecode(v.text), { bytes: [...b], resource: [] });
  }
}
for (const len of [0, 1, 2, 3, 90, 255, 256, 512, 2048]) {
  const b = Uint8Array.from({ length: len }, (_, i) => i & 255);
  const v = capture('binhex_encode', { data: bytes(b) }, {}, () => bin.binhexEncode('', { rawBytes: b }));
  capture('binhex_decode', { text: text(v.text) }, {}, () => bin.binhexDecode(v.text), { bytes: [...b], resource: [] });
}
// Different fork sizes, FourCC, flags and independent CRC corruption in each section.
for (const len of [0, 1, 90, 256]) {
  const data = Buffer.from('123456789');
  const resource = Buffer.from(Array.from({ length: len }, (_, i) => (i * 17) & 255));
  const head = Buffer.concat([Buffer.from([1, 97, 0]), Buffer.from('BINA????'), Buffer.from([0xA0, 0]), Buffer.alloc(8)]);
  head.writeUInt32BE(data.length, head.length - 8); head.writeUInt32BE(resource.length, head.length - 4);
  const u16 = n => Buffer.from([n >> 8, n & 255]);
  const raw = Buffer.concat([head, u16(bin.crcHqx(head)), data, u16(bin.crcHqx(data)), resource, u16(bin.crcHqx(resource))]);
  for (const corrupt of [-1, head.length, head.length + 2 + data.length, raw.length - 2]) {
    const damaged = Buffer.from(raw); if (corrupt >= 0) damaged[corrupt] ^= 1;
    const encoded = ':' + bin.sixbitEncode(bin.rle90Encode(damaged)) + ':';
    capture('binhex_decode', { text: text(encoded) }, {}, () => bin.binhexDecode(encoded), { bytes: [...data], resource: [...resource], validCrc: corrupt < 0 });
  }
}
for (const input of ['', 'x', ':!!:', '(This file must be converted with BinHex 4.0)\n:!!!']) {
  capture('binhex_decode', { text: text(input) }, {}, () => bin.binhexDecode(input));
}
for (const boundary of ['', 'foo', "a:b/c?d=e", 'a'.repeat(70)]) {
  for (const input of ['A', ' A | B ', '中文 | 😀 | café', '|a||b|', 'hello\nworld | x\r\ny', 'a'.repeat(501), '\uFEFFa　|　b\uFEFF', '', '| |']) {
    const v = capture('mime_multipart_encode', { data: text(input) }, { boundary }, () => mime.mimeMultipartEncode(input, { boundary }));
    if (!v.error) capture('mime_multipart_decode', { data: text(v.text) }, {}, () => mime.mimeMultipartOp(v.text));
  }
}
for (const ending of ['\n', '\r\n']) {
  for (const encoding of ['', 'base64', 'quoted-printable', 'quoted_printable', '7bit', '8bit', 'binary', 'BASE64']) {
    for (const body of ['Hello', '中文\nnext', 'a'.repeat(501)]) {
      const payload = encoding.toLowerCase() === 'base64' ? Buffer.from(body).toString('base64') : encoding.startsWith('quoted') ? [...Buffer.from(body)].map(b => '=' + b.toString(16).padStart(2, '0')).join('') : body;
      const wire = ['Content-Type: multipart/mixed; boundary=xx', '', '--xx', 'Content-Type: text/plain; name="a.txt"', 'Content-Disposition: attachment; filename="b.txt"', ...(encoding ? ['Content-Transfer-Encoding: ' + encoding] : []), '', payload, '--xx--', 'ignored epilogue'].join(ending);
      capture('mime_multipart_decode', { data: text(wire) }, {}, () => mime.mimeMultipartOp(wire));
    }
  }
}
const missingType = 'Content-Type: multipart/mixed; boundary=x\n\n--x\n\nhello\n--x--';
capture('mime_multipart_decode', { data: text(missingType) }, {}, () => mime.mimeMultipartOp(missingType));
for (const input of ['', 'no boundary', 'Content-Type: multipart/mixed; boundary=x\n\nno marker']) {
  capture('mime_multipart_decode', { data: text(input) }, {}, () => mime.mimeMultipartOp(input));
}
// Escape replacement characters in fixtures so the repository UTF-8 guard can
// distinguish intentional lossy previews from accidentally damaged source text.
const jsonText = JSON.stringify({ revision, vectors }, null, 2).replaceAll('\uFFFD', '\\ufffd');
await writeFile('crates/misclab-core/tests/fixtures/ebctf-mail-batch22.json', jsonText + '\n');
console.log(JSON.stringify({ vectors: vectors.length, example: bin.binhexEncode('Hello') }));
