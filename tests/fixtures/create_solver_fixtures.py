"""Generate independent, random-answer fixtures for opt-in real-model tests.

cases.json is the held-out oracle. Only task/data/file paths enter the AI UI.
Uses Python's standard library; application algorithms remain Rust.
"""
import base64
import codecs
import json
from pathlib import Path
import random
import secrets
import struct
import sys
import wave
import zipfile
import zlib

root = Path(sys.argv[1] if len(sys.argv) > 1 else ".git/solver-fixtures").resolve()
root.mkdir(parents=True, exist_ok=True)
cases = []


def flag(kind):
    return f"flag{{{kind}_{secrets.token_hex(5)}}}"


def layers(text):
    return base64.b64encode(codecs.encode(text, "rot_13").encode().hex().encode()).decode()


def case(name, task, answer, data="", file=None):
    cases.append(dict(name=name, task=task, answer=answer, data=data, files=[str(file)] if file else []))


answer = flag("text")
case("text", "文本题，未知编码方式和层数，请根据数据特征自主解题。", answer, layers("记录\n" + answer))
answer = flag("the_hidden_caesar_message")
cipher = "".join(chr((ord(c) - ord('a') + 11) % 26 + ord('a')) if 'a' <= c <= 'z' else c for c in answer)
case("cipher", "字符替换类题目，未知算法和密钥，请尝试找出答案。", answer, cipher)
answer = flag("archive")
archive = root / "specimen.dat"
with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as f:
    f.writestr("readme.txt", "Evidence is inside records/payload.txt. It is not plaintext.")
    f.writestr("records/noise.txt", "This is unrelated background noise.")
    f.writestr("records/payload.txt", layers(answer))
case("archive", "未知格式的取证文件。先识别结构与内容，再根据实际发现继续分析，找出答案。", answer, file=archive)

answer = flag("image")
payload = (answer + "\0").encode()
bits = [(b >> k) & 1 for b in payload for k in range(7, -1, -1)]
rng = random.Random(83197)
w, h = 64, 64
rows = bytearray()
for y in range(h):
    rows.append(0)
    for x in range(w):
        i = y * w + x
        rows.extend(((rng.randrange(256) & 254) | (bits[i] if i < len(bits) else 0), rng.randrange(256) & 254, rng.randrange(256) & 254))


def chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xffffffff)


image = root / "picture.png"
image.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">2I5B", w, h, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b""))
case("image", "图片隐写题，题目提示是最低有效位；未知通道与位序，请自主探测并提取答案。", answer, file=image)

answer = flag("audio")
bits = [(b >> k) & 1 for b in (answer + "\0").encode() for k in range(7, -1, -1)]
audio = root / "recording.wav"
with wave.open(str(audio), "wb") as f:
    f.setnchannels(1)
    f.setsampwidth(2)
    f.setframerate(8000)
    f.writeframes(b"".join(struct.pack("<h", ((i * 134 % 10000) & ~1) | (bits[i] if i < len(bits) else 0)) for i in range(8000)))
case("audio", "音频隐写题，题目提示是采样的最低有效位；未知声道及位序，请自主提取答案。", answer, file=audio)
(root / "cases.json").write_text(json.dumps(cases, ensure_ascii=False, indent=2), encoding="utf-8")
print(f"Created {len(cases)} fixtures; held-out answers: {root / 'cases.json'}")
