// Optional maintainer tool: captures golden vectors from the pinned reference
// checkout only. Upstream JS is NEVER part of app/runtime/test execution.
// Normal cargo test reads the committed JSON fixture, without Node or network.
import { writeFile, mkdir } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
const root = resolve(".git/reference/EBCTFCodeBox");
const revision = execFileSync("git", ["-C", root, "rev-parse", "HEAD"], { encoding: "utf8" }).trim();
if (revision !== "ea2bc36dae31c97060f7f2cddad8df5518afad7b") throw Error("Wrong reference revision");
for (const file of ["baseExt", "textExt", "radixExt", "bitops", "netcodecExt", "token", "byteTools"]) await import(pathToFileURL(resolve(root, "src/core/" + file + ".js")));
const { OPS } = await import(pathToFileURL(resolve(root, "src/core/registry.js")));
const { decodeDataUri } = await import(pathToFileURL(resolve(root, "src/core/dataUri.js")));
const byId = Object.fromEntries(OPS.map(o => [o.id, o]));
const vectors = [];
const mapping = {
  z85: "z85", base85ipv6: "base85ipv6", base64dict: "base64_dict", multilineBase64: "multiline_base64",
  base64decompress: "base64_zlib", netbios: "netbios", caretMdecode: "caret_m", httpBasicAuth: "http_basic_auth",
  bitReverse: "bit_reverse", bitRotate: "bit_rotate", byteSwap: "byte_swap", byteReverse: "byte_reverse",
  hexReverse: "hex_reverse", varint: "varint", b64urlJson: "b64url_json", dataUriParse: "data_uri",
  urlQueryParse: "url_query_parse", cookieParse: "cookie_parse", asciiControl: "ascii_control", splitHex: "split_hex",
};
async function capture(id, direction, text, params = {}, rawBytes) {
  const op = byId[id];
  const node = mapping[id] + (direction === "run" ? "" : "_" + direction);
  const byteInput = ["z85", "base85ipv6", "base64dict", "multilineBase64", "base64decompress", "netbios", "caretMdecode", "httpBasicAuth", "bitReverse", "bitRotate", "byteSwap", "byteReverse", "dataUriParse"].includes(id) && direction === "encode";
  const input = rawBytes ? { type: "bytes", value: rawBytes } : { type: "text", value: text };
  const v = { node, input: { [byteInput ? "data" : "text"]: input }, params };
  let result;
  try { result = await op[direction](text, { ...params, ...(rawBytes ? { rawBytes: Uint8Array.from(rawBytes) } : {}) }); }
  catch (e) { v.error = String(e.message ?? e); vectors.push(v); return null; }
  if (id === "base64decompress" && direction === "encode") v.zlibRoundtrip = rawBytes ?? [...Buffer.from(text)];
  else if (id === "dataUriParse" && direction === "decode") {
    const d = decodeDataUri(text); v.bytes = [...d.bytes]; v.metadata = { mime: d.mime, base64: d.base64, attrs: d.attrs };
  } else if (result && typeof result === "object" && result.files?.[0]?.bytes) v.bytes = [...result.files[0].bytes];
  else v.text = String(result);
  if (id === "caretMdecode" && direction === "decode") v.bytes = [...Buffer.from(result, "latin1")];
  vectors.push(v);
  return result;
}
for (const id of ["z85", "base64dict", "multilineBase64", "base64decompress", "netbios", "httpBasicAuth", "bitReverse", "bitRotate", "byteSwap", "byteReverse"]) {
  for (const text of ["Hello, CTF!", "中文🙂\n", ""]) {
    if (id === "netbios" && text === "") continue; // Upstream rejects empty decoding; Rust accepts empty.
    const encoded = await capture(id, "encode", text);
    await capture(id, "decode", encoded);
  }
}
for (const id of ["z85", "base64dict", "multilineBase64", "base64decompress", "netbios"]) {
  const bytes = [0, 255, 1, 128, 65, 0];
  const encoded = await capture(id, "encode", "", {}, bytes);
  await capture(id, "decode", encoded);
}
for (const bytes of [Array(16).fill(0), [...Buffer.from("108000000000000000080800200c417a", "hex")], Array(16).fill(255)]) {
  const encoded = await capture("base85ipv6", "encode", "", {}, bytes);
  await capture("base85ipv6", "decode", encoded);
}
for (const p of [{ lineLen: 1 }, { lineLen: 13 }, { lineLen: 76 }]) {
  const encoded = await capture("multilineBase64", "encode", "中文🙂跨行测试", p);
  await capture("multilineBase64", "decode", encoded, p);
}
await capture("multilineBase64", "decode", "SGk=\n!!\n5Lit\n5paH");
const key = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".split("").reverse().join("");
const encodedDict = await capture("base64dict", "encode", "字典🙂", { key });
await capture("base64dict", "decode", "!" + encodedDict + " ", { key });
for (const dir of ["left", "right"]) for (const bits of [1, 3, 7]) {
  const p = { dir, bits }, encoded = await capture("bitRotate", "encode", "ABC中文", p);
  await capture("bitRotate", "decode", encoded, p);
}
for (const group of [2, 4, 8]) for (const mode of ["text", "hex"]) {
  const p = { group: String(group), mode };
  const encoded = await capture("byteSwap", "encode", mode === "hex" ? "0011223344556677" : "abcde", p);
  await capture("byteSwap", "decode", encoded, p);
}
await capture("byteReverse", "encode", "0x00 ff 80 42", { mode: "hex" });
await capture("byteReverse", "decode", "4280FF00", { mode: "hex" });
await capture("caretMdecode", "encode", "", {}, [0, 13, 10, 127, 128, 255, 65]);
await capture("caretMdecode", "decode", "^@^M^J^?M-^@M-^?A");
await capture("caretMdecode", "encode", "ÿ中文");
await capture("caretMdecode", "decode", "^[^\\^]^^^_");
await capture("hexReverse", "encode", "0x1a 2B");
await capture("hexReverse", "decode", "a1B2");
for (const signed of [false, true]) for (const number of ["0", "127", "128", "300", "18446744073709551616", ...(signed ? ["-1", "-9223372036854775808"] : [])]) {
  const encoded = await capture("varint", "encode", number, { signed });
  await capture("varint", "decode", encoded + "ff", { signed });
}
for (const text of ['{ "z": 1, "a": [1.0, 1000] }', "非 JSON", "", '{"2":"b","1":"a","x":1,"01":0,"4294967295":1,"0":2}', '[9007199254740993,1e21,1e-7,1e-6,-0,1e400,1e-400]', '{"nested":[{"4":4,"2":2},[],{}]}']) {
  const encoded = await capture("b64urlJson", "encode", text);
  await capture("b64urlJson", "decode", encoded);
}
for (const s of ["https://x.test/p?a=1&a=2&k=%E4%B8%AD+%E6%96%87#hash", "?a=%FF&b=%XX&empty&eq=x=y", "?"]) await capture("urlQueryParse", "run", s);
for (const s of ["Cookie: a=1; a=2; token=a=b", "Set-Cookie: sid=abc; Expires=Wed, 21 Oct 2030 07:28:00 GMT; Path=/; HttpOnly", " ; "]) await capture("cookieParse", "run", s);
for (const p of [{ mode: "name" }, { mode: "char" }]) for (const s of ["NUL, LF DEL SP 中", "A\u0000B\tC\nD", ""]) await capture("asciiControl", "run", s, p);
for (const s of ["12 3456789abcdef", "", "a"]) await capture("splitHex", "run", s);
for (const base64 of [false, true]) {
  const p = { mime: "text/plain;charset=utf-8", base64 };
  const encoded = await capture("dataUriParse", "encode", "Hello, 世界🙂", p);
  await capture("dataUriParse", "decode", encoded);
  const binary = await capture("dataUriParse", "encode", "", { mime: "application/octet-stream", base64 }, [0, 255, 128, 65]);
  await capture("dataUriParse", "decode", binary);
}
for (const uri of ["data:,%FF%00%41", "DATA:;charset=utf-8;base64,SGk%3D", "data:text/plain;base64,iVBORw0KGgo="]) await capture("dataUriParse", "decode", uri);
const out = "crates/misclab-core/tests/fixtures/ebctf-batch2.json";
await mkdir(resolve(out, ".."), { recursive: true });
await writeFile(out, JSON.stringify({ revision, source: "EBCTFCodeBox", vectors }, null, 2) + "\n");
console.log("Captured " + vectors.length + " upstream vectors for " + Object.keys(mapping).length + " operations.");
