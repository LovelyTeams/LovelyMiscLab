// Optional maintainer tool. Capture pinned reference outputs into offline fixtures;
// neither upstream JS nor Node is used by Rust algorithms or cargo test.
import { writeFile } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
const root = resolve(".git/reference/EBCTFCodeBox");
const revision = execFileSync("git", ["-C", root, "rev-parse", "HEAD"], { encoding: "utf8" }).trim();
if (revision !== "ea2bc36dae31c97060f7f2cddad8df5518afad7b") throw Error("Wrong reference revision");
const { HTML_NAMED } = await import(pathToFileURL(resolve(root, "src/core/text.js")));
await import(pathToFileURL(resolve(root, "src/core/textExt.js")));
const { OPS } = await import(pathToFileURL(resolve(root, "src/core/registry.js")));
const ops = Object.fromEntries(OPS.map(o => [o.id, o]));
const mapping = { url: "url", htmlEntity: "html_entity", unicodeEscape: "unicode", quotedPrintable: "quoted_printable", utf7: "utf7", punycode: "punycode", jsHex: "js_hex", mixHexOctBin: "mixed_radix_decode", leetSpeak: "leet_speak", natoAlphabet: "nato_alphabet" };
const vectors = [];
async function capture(id, direction, text, params = {}, rawBytes) {
  const node = id === "unicodeEscape" ? (direction === "encode" ? "unicode_escape" : "unicode_unescape") : mapping[id] + (direction === "run" ? "" : "_" + direction);
  let mapped = { ...params };
  if (id === "url") mapped = direction === "encode" ? { mode: "standard", ...params } : { plus: true, trim: true, strict: true };
  if (id === "htmlEntity" && direction === "encode") mapped = { namedApostrophe: true };
  if (id === "unicodeEscape" && direction === "encode") mapped = { mode: "全部", upper: true, ...params };
  if (id === "quotedPrintable" && direction === "encode") mapped = { mode: "原始字节" };
  if (id === "mixHexOctBin") mapped = { strict: false };
  const port = ["jsHex", "quotedPrintable"].includes(id) && direction === "encode" ? "data" : "text";
  const v = { operation: id, node, input: { [port]: rawBytes ? { type: "bytes", value: rawBytes } : { type: "text", value: text } }, params: mapped };
  let result;
  try { result = await ops[id][direction](text, { ...params, ...(rawBytes ? { rawBytes: Uint8Array.from(rawBytes) } : {}) }); }
  catch (e) { v.error = String(e.message ?? e); vectors.push(v); return null; }
  if (result && typeof result === "object" && result.files?.[0]?.bytes) v.bytes = [...result.files[0].bytes];
  else v.text = String(result);
  vectors.push(v);
  return result;
}
for (const id of ["url", "htmlEntity", "unicodeEscape", "quotedPrintable", "utf7", "jsHex", "leetSpeak", "natoAlphabet"]) {
  for (const text of ["", "Hello, CTF!", "中文🙂\n", "A\u0000B\r\n\t", "&< >\"' +%\\x41", " ABC xyz 019 ", "\ufeffBOM"]) {
    const encoded = await capture(id, "encode", text);
    await capture(id, "decode", encoded);
  }
}
for (const mode of ["standard", "full", "plus"]) for (const text of ["!*'()~ +-_/", " a b 中文 "]) {
  const encoded = await capture("url", "encode", text, { mode });
  await capture("url", "decode", " " + encoded + " ");
}
for (const input of ["%", "%0", "%ZZ", "%ff", "%E4%B8", "a+b%2Bc"]) await capture("url", "decode", input);
for (const input of [Object.keys(HTML_NAMED).map(k => "&" + k + ";").join(" "), "&#65; &#x1F642; &#x0000; &#1114111; &amp;lt; &unknown;", "&#1114112; &#x110000; ", "& &no_semicolon"]) await capture("htmlEntity", "decode", input);
for (const fmt of ["uXXXX", "U+", "hex"]) for (const text of ["A中🙂Z", "\u0000\uffff\u{10ffff}"]) {
  const encoded = await capture("unicodeEscape", "encode", text, { fmt });
  await capture("unicodeEscape", "decode", encoded);
}
for (const input of ["\\u0041\\uD83D\\uDE42\\u0042", "\\u4e2d U+6587 &#x1F642;", "U+110000 &#x110000; \\uZZZZ", "\\u0055+0041"]) await capture("unicodeEscape", "decode", input);
for (const input of ["=48=69=21=\r\n!", "=00=FF=80=0D=0A", "a=\nb=\r\nc"]) await capture("quotedPrintable", "decode", input);
for (const input of ["+ImIDkQ. B-C", "+ZeVnLIqe-", "+2D3eQg-", "Hi +- +Ti0!", "+Ti0", "+-", "["] ) await capture("utf7", "decode", input);
for (const text of ["", "example.com", "中文.例子", "bücher.example.", "mañana.com", "Bücher.Example", "☃-⌘.emoji🙂", "日本語", "موقع.اختبار", ".a..中.", "他们为什么不说中文"]) {
  const encoded = await capture("punycode", "encode", text);
  await capture("punycode", "decode", encoded);
}
for (const input of ["XN--BCHER-KVA.Example", "xn--", "xn--abc-", "xn--z", "xn--!"]) await capture("punycode", "decode", input);
for (const mode of ["all", "nonascii"]) {
  const encoded = await capture("jsHex", "encode", "\\x41 中文🙂\t", { mode });
  await capture("jsHex", "decode", encoded);
}
for (const input of ["中文🙂", "\\x00\\xff\\x80", "\\x4\\xZZ", "\\x41\\x42中"]) await capture("jsHex", "decode", input);
for (const id of ["url", "jsHex", "quotedPrintable"]) {
  const raw = Array.from({ length: 256 }, (_, i) => i);
  const encoded = await capture(id, "encode", "", {}, raw);
  if (id !== "url") await capture(id, "decode", encoded);
}
for (const id of ["leetSpeak", "natoAlphabet"]) {
  const encoded = await capture(id, "encode", "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz 0123456789中🙂!?\\");
  await capture(id, "decode", encoded);
}
for (const input of ["alpha,BRAVO;Juliet x-ray (space) Niner ?未知", "unknown;unknown"]) await capture("natoAlphabet", "decode", input);
for (const input of ["", "0x480o151;0b100001", "0x680o1450b11011000x6c0d111", "0X4E2D,0d128578;33", "65 066 0x00", "bad 0b2 0x41zz", "0x0d 0x0b", "0x410x420x43"]) await capture("mixHexOctBin", "run", input);
await writeFile("crates/misclab-core/tests/fixtures/ebctf-text-batch4.json", JSON.stringify({ revision, source: "EBCTFCodeBox", vectors }, null, 2) + "\n");
await writeFile("crates/misclab-core/src/nodes/html_named.json", JSON.stringify(HTML_NAMED, null, 2) + "\n");
console.log(`Captured ${vectors.length} vectors for ${Object.keys(mapping).length} operations; ${Object.keys(HTML_NAMED).length} named entities.`);
