"""Independent cross-domain CTF fixture for an opt-in real-model UI test.

Only specimen.dat and the task enter the model. cases.json is a held-out oracle.
Python/Node are test fixture generators; production algorithms remain Rust.
"""
import base64
import codecs
import hashlib
import json
from pathlib import Path
import random
import secrets
import struct
import subprocess
import sys
import zipfile
import zlib

root = Path(sys.argv[1] if len(sys.argv) > 1 else ".git/solver-fixtures/complex").resolve()
root.mkdir(parents=True, exist_ok=True)
answer = f"flag{{cross_domain_{secrets.token_hex(8)}}}"
key = secrets.token_hex(16)
iv = secrets.token_hex(16)
wrapped = base64.b64encode(codecs.encode(answer, "rot_13").encode().hex().encode()).decode()
crypto = subprocess.run(
    ["node", "-e", "const c=require('node:crypto'),fs=require('node:fs');const x=JSON.parse(fs.readFileSync(0,'utf8'));const e=c.createCipheriv('aes-128-cbc',Buffer.from(x.key,'hex'),Buffer.from(x.iv,'hex'));process.stdout.write(Buffer.concat([e.update(x.text,'utf8'),e.final()]).toString('base64'));"],
    input=json.dumps(dict(key=key, iv=iv, text=wrapped)),
    capture_output=True, text=True, check=True,
).stdout

def chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xffffffff)

payload = f"KEY_HEX={key}\n\0".encode()
bits = [(b >> k) & 1 for b in payload for k in range(7, -1, -1)]
rng = random.Random(271028)
w, h = 32, 32
rows = bytearray()
for y in range(h):
    rows.append(0)
    for x in range(w):
        i = y * w + x
        rows.extend((rng.randrange(256) & 254, (rng.randrange(256) & 254) | (bits[i] if i < len(bits) else 0), rng.randrange(256) & 254))
picture = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">2I5B", w, h, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b"")
brief = (
    "Recovered dispatch, ticket 271028. The file extension was lost.\n"
    "The courier kept a key in the photograph: green whispers at the lowest bit, "
    "eight whispers form a byte with the most significant first.\n"
    "The packet states its cipher settings. The note is still wrapped for transport "
    "after opening the envelope. Find the final flag; diagnose each layer from its actual contents.\n"
    "The telemetry file is unrelated measurement noise.\n"
)
packet = dict(version=1, cipher="AES-128-CBC", padding="PKCS7", encoding="Base64", iv_hex=iv, ciphertext=crypto)
specimen = root / "specimen.dat"
with zipfile.ZipFile(specimen, "w", compression=zipfile.ZIP_DEFLATED) as f:
    f.writestr("brief.txt", brief)
    f.writestr("dispatch/packet.json", json.dumps(packet, indent=2))
    f.writestr("assets/photograph.png", picture)
    f.writestr("telemetry/noise.csv", "time,value\n0,12\n1,94\n2,27\n")

case = dict(name="cross_domain", task="未知格式的综合取证题。请自主读取附件、判断真实线索，找出最终答案；记录每轮证据和推理，生成可以从附件重新执行的流程。", answer=answer, data="", files=[str(specimen)], maxRounds=8, maxMinutes=15,
    sha256=hashlib.sha256(specimen.read_bytes()).hexdigest(), referenceStages=["ZIP recognition", "archive entries", "green-channel bit-0 MSB LSB", "AES-128-CBC PKCS7", "Base64", "Hex", "ROT13"], keyHex=key, ivHex=iv)
(root / "cases.json").write_text(json.dumps(case, ensure_ascii=False, indent=2), encoding="utf-8")
print(json.dumps(dict(file=str(specimen), size=specimen.stat().st_size, sha256=case["sha256"], heldOut=str(root / "cases.json"))))
