# EBCTFCodeBox 完整功能清单

由 `pnpm generate:migration` 生成。来源提交：`ea2bc36dae31c97060f7f2cddad8df5518afad7b`。人工验收源：[ebctf-acceptance.json](ebctf-acceptance.json)，生成器不会覆盖它。

总计 820 项：已迁入 163，部分完成 0，待解决 0，现有候选待核验 49，待迁入 608。

`existing-review` 仅表示名称匹配，不表示算法、参数或文件处理能力等价。验收范围和批次见 [迁移说明](ebctf-migration.md)；机器可读清单见 [JSON](ebctf-migration.json)。

| 上游 ID | 功能 | 分类 | 状态 | Rust 节点 / 待核验候选 | 来源 |
|---|---|---|---|---|---|
| `office2john` | Office 哈希提取（office2john） | crack | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/oleExtract.js) |
| `oleExtract` | OLE/CFB 容器静态提取 | forensic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/oleExtract.js) |
| `pcapParse` | pcap/pcapng 结构解析 | forensic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoQuickScan.js) |
| `pcapTcpReassemble` | TCP 流重组 | forensic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoQuickScan.js) |
| `pcapHttpExtract` | HTTP 对象提取 | forensic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoQuickScan.js) |
| `pcapDnsTunnel` | DNS 隧道检测 | forensic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoQuickScan.js) |
| `pcapIcmpPayload` | ICMP 载荷提取 | forensic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoQuickScan.js) |
| `pcapFieldExtract` | pcap 字段提取/过滤 | forensic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoQuickScan.js) |
| `gzipCodec` | Gzip 解压 / 压缩 | archive | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoQuickScan.js) |
| `zlibCodec` | Zlib 解压 / 压缩 | archive | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoQuickScan.js) |
| `deflateRawCodec` | Raw Deflate 解压 / 压缩 | archive | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoQuickScan.js) |
| `b64CompressedProbe` | Base64 内嵌压缩流探测 | archive | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoQuickScan.js) |
| `mcLevelDat` | Minecraft level.dat 解析 | forensic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoQuickScan.js) |
| `mcMapRender` | Minecraft 地图渲染 | forensic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoQuickScan.js) |
| `pngChunkList` | PNG 全块解析 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoQuickScan.js) |
| `jpegAppList` | JPEG APPn 段列举 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoQuickScan.js) |
| `gifComment` | GIF 注释扩展 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoQuickScan.js) |
| `gifFrames` | GIF 多帧提取 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoQuickScan.js) |
| `iccStrip` | ICC 剥离 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoQuickScan.js) |
| `base16` | Base16 / Hex | base | migrated | `hex_encode`, `hex_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/base.js) |
| `base32` | Base32 | base | migrated | `base32_encode`, `base32_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/base.js) |
| `base36` | Base36 | base | migrated | `base36_encode`, `base36_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/base.js) |
| `base45` | Base45 | base | migrated | `base45_encode`, `base45_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/base.js) |
| `base58` | Base58 | base | migrated | `base58_encode`, `base58_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/base.js) |
| `base62` | Base62 | base | migrated | `base62_encode`, `base62_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/base.js) |
| `base64` | Base64 | base | migrated | `base64_encode`, `base64_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/base.js) |
| `base85` | Base85 / Ascii85 | base | migrated | `base85_encode`, `base85_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/base.js) |
| `base91` | Base91 | base | migrated | `base91_encode`, `base91_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/base.js) |
| `base92` | Base92 | base | migrated | `base92_encode`, `base92_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/base.js) |
| `base100` | Base100 | base | migrated | `base100_encode`, `base100_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/base.js) |
| `radixN` | 任意进制 | base | migrated | `radix_n_encode`, `radix_n_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/base.js) |
| `baseCustom` | 自定义字母表 Base | base | migrated | `base_custom_encode`, `base_custom_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/base.js) |
| `base58check` | Base58Check | base | migrated | `base58check_encode`, `base58check_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/baseExt.js) |
| `radix64` | Radix64 (crypt) | base | migrated | `radix64_encode`, `radix64_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/baseExt.js) |
| `base69` | Base69 | base | migrated | `base69_encode`, `base69_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/baseExt.js) |
| `z85` | Z85 (ZeroMQ) | base | migrated | `z85_encode`, `z85_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/baseExt.js) |
| `base85ipv6` | Base85 IPv6 | base | migrated | `base85ipv6_encode`, `base85ipv6_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/baseExt.js) |
| `base2048` | Base2048 | base | migrated | `base2048_encode`, `base2048_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/baseExt.js) |
| `base65536` | Base65536 | base | migrated | `base65536_encode`, `base65536_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/baseExt.js) |
| `ecoji` | Ecoji | base | migrated | `ecoji_encode`, `ecoji_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/baseExt.js) |
| `base64steg` | Base64 隐写 | base | migrated | `base64_padding_stego_encode`, `base64_padding_stego_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/baseExt.js) |
| `base32steg` | Base32 隐写 | base | migrated | `base32_padding_stego_encode`, `base32_padding_stego_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/baseExt.js) |
| `base64dict` | 凯撒自定义字典 Base64 | base | migrated | `base64_dict_encode`, `base64_dict_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/baseExt.js) |
| `multilineBase64` | 多行 Base64 | base | migrated | `multiline_base64_encode`, `multiline_base64_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/baseExt.js) |
| `base64decompress` | Base64 + Zlib | base | migrated | `base64_zlib_encode`, `base64_zlib_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/baseExt.js) |
| `url` | URL 编码 | text | migrated | `url_encode`, `url_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/text.js) |
| `htmlEntity` | HTML 实体 | text | migrated | `html_entity_encode`, `html_entity_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/text.js) |
| `unicodeEscape` | Unicode 转义 | text | migrated | `unicode_escape`, `unicode_unescape` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/text.js) |
| `quotedPrintable` | Quoted-Printable | text | migrated | `quoted_printable_encode`, `quoted_printable_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/text.js) |
| `uuencode` | UUencode | text | migrated | `uuencode_encode`, `uuencode_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/text.js) |
| `xxencode` | XXencode | text | migrated | `xxencode_encode`, `xxencode_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/text.js) |
| `jsfuck` | JSFuck | text | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/jsfuck.js) |
| `utf7` | UTF-7 编码 | text | migrated | `utf7_encode`, `utf7_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/textExt.js) |
| `punycode` | Punycode (IDN) | text | migrated | `punycode_encode`, `punycode_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/textExt.js) |
| `jsHex` | JS Hex 转义 | text | migrated | `js_hex_encode`, `js_hex_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/textExt.js) |
| `mixHexOctBin` | 混排进制解码 | text | migrated | `mixed_radix_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/textExt.js) |
| `hexReverse` | Hex 字节内反转 | text | migrated | `hex_reverse_encode`, `hex_reverse_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/textExt.js) |
| `leetSpeak` | Leet Speak (1337) | text | migrated | `leet_speak_encode`, `leet_speak_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/textExt.js) |
| `netbios` | NetBIOS 编码 | text | migrated | `netbios_encode`, `netbios_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/textExt.js) |
| `caretMdecode` | Caret/M 控制字符 | text | migrated | `caret_m_encode`, `caret_m_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/textExt.js) |
| `natoAlphabet` | NATO 音标字母 | text | migrated | `nato_alphabet_encode`, `nato_alphabet_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/textExt.js) |
| `asciiControl` | ASCII 控制字符 | text | migrated | `ascii_control` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/textExt.js) |
| `scoutStick` | 童子军木棍密码 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy.js) |
| `morse` | 摩斯电码 | fancy | existing-review | `morse_encode`, `morse_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy.js) |
| `bacon` | 培根密码 | fancy | existing-review | `bacon_encode`, `bacon_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy.js) |
| `railFence` | 栅栏密码 | classic | existing-review | `rail_fence_encode`, `rail_fence_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy.js) |
| `caesar` | 凯撒密码 | classic | existing-review | `caesar` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy.js) |
| `rot13` | ROT13 | fancy | existing-review | `rot13` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy.js) |
| `rot5` | ROT5 | fancy | migrated | `rot5` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy.js) |
| `rot18` | ROT18 | fancy | migrated | `rot18` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy.js) |
| `rot47` | ROT47 | fancy | existing-review | `rot47` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy.js) |
| `atbash` | Atbash | fancy | existing-review | `atbash` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy.js) |
| `a1z26` | A1Z26 | fancy | existing-review | `a1z26_encode`, `a1z26_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy.js) |
| `dna` | DNA 编码 | fancy | existing-review | `dna_encode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy.js) |
| `keyboard` | 键盘坐标 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy.js) |
| `brainfuck` | BrainFuck | esolang | existing-review | `brainfuck` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy2.js) |
| `ook` | Ook! | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy2.js) |
| `cetacean` | 鲸语 Cetacean | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy2.js) |
| `yygq` | 兽音译者 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy2.js) |
| `braille` | 盲文 Braille | fancy | existing-review | `braille_encode`, `braille_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy2.js) |
| `eightdiagram` | 六十四卦 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy2.js) |
| `pawnshop` | 当铺密码 | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy2.js) |
| `aaencode` | 颜文字 aaencode | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancyExt.js) |
| `baudot` | 博多码 Baudot | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancyExt.js) |
| `type7` | Cisco Type7 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancyExt.js) |
| `decabit` | Decabit 脉冲码 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancyExt.js) |
| `scytale` | Scytale 密码棒 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancyExt.js) |
| `aes` | AES | block | existing-review | `aes` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `des` | DES | block | existing-review | `des` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `des3` | 3DES / TripleDES | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `rc4` | RC4 | stream | existing-review | `rc4` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `xor` | XOR | stream | existing-review | `xor` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `fernet` | Fernet | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `rsa` | RSA | asym | existing-review | `rsa_decrypt` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `md5` | MD5 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `md4` | MD4 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `sha1` | SHA-1 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `sha256` | SHA-256 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `sha384` | SHA-384 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `sha512` | SHA-512 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `hmac` | HMAC | hash | existing-review | `hmac` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `crc32` | CRC32 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `crc16` | CRC16 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `ntlm` | NTLM | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `sha3` | SHA-3 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `keccak256` | Keccak-256 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `shake128` | SHAKE128 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `shake256` | SHAKE256 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `txtmoji` | txtmoji emoji 加密 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/txtmoji.js) |
| `keyCode` | JS keyCode 表 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/keyboard.js) |
| `shiftKey` | 上档键符号 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/keyboard.js) |
| `keyword9` | T9 九宫格 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/keyboard.js) |
| `keyboardSurround` | 键盘包围键 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/keyboard.js) |
| `qweAbc` | QWERTY→ABC | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/keyboard.js) |
| `vigenere` | 维吉尼亚 | classic | existing-review | `vigenere` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classic.js) |
| `gronsfeld` | Gronsfeld | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classic.js) |
| `beaufort` | Beaufort | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classic.js) |
| `autokey` | AutoKey 自动密钥 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classic.js) |
| `porta` | Porta | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classic.js) |
| `playfair` | Playfair | classic | existing-review | `playfair_encode`, `playfair_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classic.js) |
| `nihilist` | Nihilist 虚无党 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classic.js) |
| `columnar` | 列移位 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classic.js) |
| `hill` | Hill 希尔 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classic.js) |
| `affine` | 仿射 | classic | existing-review | `affine` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classic.js) |
| `bifid` | Bifid 双分 | classic | existing-review | `bifid_encode`, `bifid_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classic.js) |
| `trifid` | Trifid 三分 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classic.js) |
| `polybius` | Polybius 方阵 | classic | existing-review | `polybius` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classic.js) |
| `adfgx` | ADFGX | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classic.js) |
| `adfgvx` | ADFGVX | classic | existing-review | `adfgvx` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classic.js) |
| `foursquare` | FourSquare 四方 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classic.js) |
| `graycode` | 格雷码 GrayCode | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classic.js) |
| `trithemius` | Trithemius 渐进移位 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classicExt2.js) |
| `jefferson` | 杰斐逊轮盘 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/jefferson.js) |
| `amsco` | AMSCO 密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/amsco.js) |
| `ragbaby` | Ragbaby 密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ragbaby.js) |
| `trilitere` | Trilitère 三元密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/trilitere.js) |
| `skipCipher` | Skip 跳读密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/skipCipher.js) |
| `vic` | VIC 密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/vic.js) |
| `phillips` | Phillips 密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/phillips.js) |
| `bellaso` | Bellaso 密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bellaso.js) |
| `slidefair` | Slidefair 密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/slidefair.js) |
| `collon` | Collon 密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/collon.js) |
| `dancingMen` | 跳舞小人 Dancing Men | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/dancingMen.js) |
| `zodiac` | 黄道十二宫 Z408 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/zodiac.js) |
| `threeSquare` | 三方密码 Three-square | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/threeSquare.js) |
| `monomeBinome` | Monome-Binome 单子双子 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/monomeBinome.js) |
| `kuznyechik` | Kuznyechik（GOST R 34.12-2015） | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/kuznyechik.js) |
| `iban` | IBAN 校验位（mod-97） | radix | migrated | `iban_calculate`, `iban_validate` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/iban.js) |
| `mirrorLetters` | 镜像字母 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/mirrorLetters.js) |
| `mayaNumerals` | 玛雅数字 | radix | migrated | `maya_numerals_encode`, `maya_numerals_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/mayaNumerals.js) |
| `babylonianNumerals` | 巴比伦数字 | radix | migrated | `babylonian_numerals_encode`, `babylonian_numerals_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/babylonianNumerals.js) |
| `egyptianNumerals` | 埃及数字 | radix | migrated | `egyptian_numerals_encode`, `egyptian_numerals_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/egyptianNumerals.js) |
| `hieroglyphs` | 圣书体字母 MdC | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hieroglyphs.js) |
| `sga` | 银河标准字母 SGA | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/sga.js) |
| `occult` | 神秘学字母四件 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/occult.js) |
| `marineFlags` | 国际信号旗文本码 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/marineFlags.js) |
| `ssdeep` | SSDEEP 模糊哈希 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ssdeep.js) |
| `lznt1` | LZNT1 解压 | archive | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/lznt1.js) |
| `htmlCommentExtract` | HTML 注释提取 | filefmt | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/commentExtract.js) |
| `zipCommentExtract` | ZIP 注释提取 | filefmt | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/commentExtract.js) |
| `radixConvert` | 进制互转 | radix | migrated | `radix_transform_encode`, `radix_transform_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radix.js) |
| `asciiRadix` | 字符↔进制ASCII | radix | migrated | `ascii_radix_encode`, `ascii_radix_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radix.js) |
| `ieee754` | IEEE754 浮点 | radix | migrated | `ieee754_encode`, `ieee754_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radix.js) |
| `bcd` | BCD 码 | radix | migrated | `bcd_encode`, `bcd_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radix.js) |
| `binPad` | 二进制补零对齐 | radix | migrated | `binary_pad_encode`, `binary_pad_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radix.js) |
| `hybridCode` | 混合进制解码 | radix | migrated | `hybrid_code_encode`, `hybrid_code_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `separationAscii` | 数字串分割 ASCII | radix | migrated | `separate_ascii` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `asciiOffset` | ASCII 偏移 | radix | migrated | `ascii_offset_encode`, `ascii_offset_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `decimalToFloat` | 十进制转任意进制浮点 | radix | migrated | `decimal_to_radix_float` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `binaryComplement` | 原码反码补码 | radix | migrated | `binary_complement` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `floatToDecimal` | 任意进制浮点转十进制 | radix | migrated | `radix_float_to_decimal` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `complementToDec` | 原反补码转十进制 | radix | migrated | `complement_to_decimal` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `completion` | 补零对齐 | radix | migrated | `zero_align_encode`, `zero_align_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `splitHex` | Hex N 位分割 | radix | migrated | `split_hex` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `standardCode` | 字符集互转 | radix | migrated | `charset_probe_encode`, `charset_probe_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `timestamp` | 时间戳 ↔ 时间 | radix | migrated | `timestamp_report`, `timestamp_to_time`, `time_to_timestamp` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `gcd` | 最大公约数 | radix | migrated | `gcd_lcm` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `primeFactor` | 素数分解 | radix | migrated | `prime_factorization` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `fibonacci` | 斐波那契解码 | radix | migrated | `fibonacci_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `negabase` | 负进制 | radix | migrated | `negative_base_encode`, `negative_base_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `balancedTernary` | 平衡三进制 | radix | migrated | `balanced_ternary_encode`, `balanced_ternary_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `factorialBase` | 阶乘进制 | radix | migrated | `factorial_base_encode`, `factorial_base_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `zeckendorf` | Zeckendorf 表示 | radix | migrated | `zeckendorf_encode`, `zeckendorf_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `roman` | 罗马数字 | radix | migrated | `roman_encode`, `roman_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `chineseNum` | 中文数字 | radix | migrated | `chinese_number_encode`, `chinese_number_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `continuedFraction` | 连分数 | radix | migrated | `continued_fraction_encode`, `continued_fraction_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `sternBrocot` | Stern-Brocot 路径 | radix | migrated | `stern_brocot_encode`, `stern_brocot_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `collatz` | Collatz 序列 | radix | migrated | `collatz_sequence` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixExt.js) |
| `sm3` | SM3 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashExt.js) |
| `ripemd160` | RIPEMD-160 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashExt.js) |
| `blake2b` | BLAKE2b | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashExt.js) |
| `blake2s` | BLAKE2s | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashExt.js) |
| `adler32` | Adler-32 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashExt.js) |
| `crc8` | CRC-8 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashExt.js) |
| `crc8_maxim` | CRC-8/MAXIM | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashExt.js) |
| `crc64` | CRC-64 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashExt.js) |
| `crc32c` | CRC-32C | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashExt.js) |
| `fnv1a` | FNV-1a | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashExt.js) |
| `murmur3_32` | MurmurHash3-32 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashExt.js) |
| `md6` | MD6 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashMore.js) |
| `snefru` | Snefru | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashMore.js) |
| `sha0` | SHA-0 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashMore.js) |
| `has160` | HAS-160 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashMore.js) |
| `gostHash` | GOST R 34.11-94 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashMore.js) |
| `stemBranch` | 天干地支 | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cn.js) |
| `baiJiaXing` | 百家姓 | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cn.js) |
| `element` | 元素周期表 | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cn.js) |
| `foyu` | 佛曰 | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cn.js) |
| `tea` | TEA | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/modernExt.js) |
| `xtea` | XTEA | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/modernExt.js) |
| `xxtea` | XXTEA | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/modernExt.js) |
| `sm4` | SM4 | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/modernExt.js) |
| `salsa20` | Salsa20 | stream | existing-review | `salsa20` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/modernExt.js) |
| `chacha20` | ChaCha20 | stream | existing-review | `chacha20` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/modernExt.js) |
| `xorStrings` | XOR 循环补齐 | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/modernExt.js) |
| `noekeon` | Noekeon | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/blockMore.js) |
| `shacal2` | SHACAL-2 | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/blockMore.js) |
| `cast6` | CAST-256 | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/blockMore.js) |
| `rc5` | RC5 | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/modernExt2.js) |
| `idea` | IDEA | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/modernExt2.js) |
| `blowfish` | Blowfish | block | existing-review | `blowfish` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/modernExt2.js) |
| `rc6` | RC6 | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/modernExt2.js) |
| `cast5` | CAST-128 | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/modernExt2.js) |
| `twofish` | Twofish | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/modernExt2.js) |
| `xorBrute` | XOR 单字节爆破 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/analysis.js) |
| `freqDist` | 字符频率分布 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/analysis.js) |
| `entropy` | 香农熵 | analysis | existing-review | `entropy` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/analysis.js) |
| `wordFreq` | 词频统计 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/analysis.js) |
| `hammingDistance` | 汉明距离 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/analysis.js) |
| `levenshtein` | 编辑距离 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/analysis.js) |
| `strContrast` | 等长 ASCII 对比 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/analysis.js) |
| `debruijn` | De Bruijn 序列 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/analysis.js) |
| `textIntConverter` | 文本↔大整数 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/analysis.js) |
| `extractHashes` | 提取哈希串 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/analysis.js) |
| `getAllCasings` | 大小写全排列 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/analysis.js) |
| `alternatingCaps` | 交替大小写 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/analysis.js) |
| `zeroWidth` | 零宽字符隐写 | stegoText | existing-review | `zero_width_encode`, `zero_width_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stego.js) |
| `zeroChar` | 零宽摩斯密码 | stegoText | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stego.js) |
| `zwTags` | Unicode Tag 走私 | stegoText | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stego.js) |
| `zwVarSel` | 变体选择器隐写 | stegoText | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stego.js) |
| `emojiSubst` | emoji 替换隐写 | stegoText | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stego.js) |
| `tadpole` | 蝌蚪文 | stegoText | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stego.js) |
| `lsbImage` | LSB 像素隐写 | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoImage.js) |
| `pixelJihad` | PixelJihad | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoImage.js) |
| `arnoldCat` | Arnold 猫脸变换 | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoImage.js) |
| `arnoldCatBrute` | Arnold 猫脸暴破 | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoImage.js) |
| `imageBasic` | 图像基础操作 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoImage.js) |
| `pngText` | PNG 文本块读写 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoImage.js) |
| `pngHeight` | PNG 高度修改 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoImage.js) |
| `exifExtract` | EXIF 提取 | image | existing-review | `exif_extract` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoImage.js) |
| `bitplaneSlicing` | 位平面分解 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoImage.js) |
| `imageDiff` | 图像差异对比 | image | existing-review | `image_diff` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoImage.js) |
| `pngSizeRecover` | PNG 宽高爆破恢复 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/imagefix.js) |
| `bmpSizeRecover` | BMP 宽高修复 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/imagefix.js) |
| `trailerCarve` | 文件附加数据剥离 | forensic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/trailerCarve.js) |
| `foremostCarve` | 文件雕刻（Foremost JS） | forensic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/foremostJs.js) |
| `argon2` | Argon2 KDF（Argon2d/i/id） | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashFrontier.js) |
| `tiger` | Tiger / Tiger2 哈希（192-bit） | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashFrontier.js) |
| `kupyna` | Kupyna 哈希（DSTU 7564:2014） | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashFrontier.js) |
| `f5stego` | F5 JPEG 隐写 编/解 | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoDetect.js) |
| `stegoDetect` | 隐写检测（文本 / 文件） | data | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegoDetect.js) |
| `textBlindWatermark` | 文本盲水印 | stegoText | existing-review | `text_blind_watermark` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/textBlindWatermark.js) |
| `qrGen` | QR 码生成 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/qrFormatBrute.js) |
| `barcodeIdentify` | 条码类型判定 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/qrFormatBrute.js) |
| `qrDecode` | QR 码解码 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/qrFormatBrute.js) |
| `qrFormatBrute` | QR 格式信息爆破 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/qrFormatBrute.js) |
| `shzyhxjzg` | 社会主义核心价值观 | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/exclusiveCodec.js) |
| `makkaPakka` | 玛卡巴卡 | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/exclusiveCodec.js) |
| `yuanYin` | 元音密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/exclusiveCodec.js) |
| `columnReplace` | 列置换密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/exclusiveCodec.js) |
| `rowsReplace` | 行置换密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/exclusiveCodec.js) |
| `jwt` | JWT | modern | existing-review | `jwt_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/token.js) |
| `jwtNone` | JWT None 攻击 | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/token.js) |
| `jweIdentify` | JWE 结构识别 | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/token.js) |
| `pasetoIdentify` | PASETO 识别 | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/token.js) |
| `b64urlJson` | Base64url ↔ JSON | data | migrated | `b64url_json_encode`, `b64url_json_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/token.js) |
| `aesCmac` | AES-CMAC | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cmac.js) |
| `sm4Cmac` | SM4-CMAC | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cmac.js) |
| `kmac` | KMAC | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cmac.js) |
| `camellia` | Camellia（RFC 3713） | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cmacMore.js) |
| `seed` | SEED（RFC 4269） | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cmacMore.js) |
| `cmacExt` | CMAC 扩展 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cmacMore.js) |
| `ecdsaReuseK` | ECDSA nonce 重用攻击 | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/jwtsign.js) |
| `primeGen` | 大素数生成 | radix | migrated | `prime_generate` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/primeGen.js) |
| `primeTest` | 素性检验（Miller-Rabin） | radix | migrated | `prime_test` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/primeGen.js) |
| `ecdsaKeyGen` | ECDSA 密钥对生成 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/jwtsign.js) |
| `ecdsaSign` | ECDSA 签名 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/jwtsign.js) |
| `ecdsaVerify` | ECDSA 验签 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/jwtsign.js) |
| `ecdsaSigConvert` | ECDSA 签名格式转换 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/jwtsign.js) |
| `eccCalc` | ECC 点运算 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/jwtsign.js) |
| `rsaGenKeyPair` | RSA 密钥对生成 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/jwtsign.js) |
| `jwtSign` | JWT 签发 | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/jwtsign.js) |
| `jwtVerify` | JWT 验签 | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/jwtsign.js) |
| `mlkemKeyGen` | ML-KEM 密钥生成 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/mlkem.js) |
| `mlkemEncaps` | ML-KEM 封装 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/mlkem.js) |
| `mlkemDecaps` | ML-KEM 解封装 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/mlkem.js) |
| `ipv4Int` | IPv4 ↔ 整数 | radix | migrated | `ipv4_integer_encode`, `ipv4_integer_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/netcodec.js) |
| `ipv6Format` | IPv6 压缩/展开 | radix | migrated | `ipv6_compress`, `ipv6_expand` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/netcodec.js) |
| `macFormat` | MAC 地址格式互转 | radix | migrated | `mac_format_encode`, `mac_format_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/netcodec.js) |
| `cidrCalc` | CIDR 子网计算 | radix | migrated | `cidr_calculate` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/netcodec.js) |
| `userAgentParse` | User-Agent 解析 | radix | migrated | `user_agent_parse` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/netcodec.js) |
| `unixTime` | Unix 时间戳 ↔ ISO8601 | radix | migrated | `unix_time_encode`, `unix_time_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/timecodec.js) |
| `filetime` | Windows FILETIME ↔ ISO8601 | radix | migrated | `windows_filetime_encode`, `windows_filetime_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/timecodec.js) |
| `hfsTime` | Mac HFS+ 时间 ↔ ISO8601 | radix | migrated | `hfs_time_encode`, `hfs_time_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/timecodec.js) |
| `cocoaTime` | Cocoa 时间 ↔ ISO8601 | radix | migrated | `cocoa_time_encode`, `cocoa_time_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/timecodec.js) |
| `dosDateTime` | DOS 日期时间 ↔ ISO8601 | radix | migrated | `dos_datetime_encode`, `dos_datetime_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/timecodec.js) |
| `chineseDate` | 汉字日期 ↔ ISO8601 | radix | migrated | `chinese_date_encode`, `chinese_date_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/timecodec.js) |
| `tzConvert` | 时区转换 | radix | migrated | `timezone_convert` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/timecodec.js) |
| `luhn` | Luhn 校验位 | radix | migrated | `luhn_calculate`, `luhn_validate` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/checkdigit.js) |
| `isbn` | ISBN-10/13 校验位 | radix | migrated | `isbn_calculate`, `isbn_validate` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/checkdigit.js) |
| `ean13` | EAN-13 校验位 | radix | migrated | `ean13_calculate`, `ean13_validate` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/checkdigit.js) |
| `cnidCheck` | 身份证 18 位校验位 | radix | migrated | `cnid_check_calculate`, `cnid_check_validate` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/checkdigit.js) |
| `upc` | UPC-A 校验位 | radix | migrated | `upc_a_calculate`, `upc_a_validate` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/checkdigit.js) |
| `bankBin` | 银行卡 BIN 识别 | radix | migrated | `bank_bin_identify` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/checkdigit.js) |
| `americanMorse` | 美式摩斯码 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/morseExt.js) |
| `cnTelegraphMorse` | 中文电码摩斯 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/morseExt.js) |
| `tapCode` | 敲击码 Tap Code | fancy | existing-review | `tap_code` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/morseExt.js) |
| `semaphore` | 旗语 Semaphore | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/morseExt.js) |
| `dtmf` | DTMF 双音多频 | fancy | existing-review | `dtmf_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/morseExt.js) |
| `morseRhythm` | 摩斯节奏规范化 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/morseExt.js) |
| `whitespace` | Whitespace | esolang | existing-review | `whitespace_encode`, `whitespace_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy3.js) |
| `pigpen` | 猪圈密码 Pigpen | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy3.js) |
| `keyboardShift` | 键盘漂移 | fancy | existing-review | `keyboard_shift` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy3.js) |
| `malbolge` | Malbolge 识别 | esolang | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fancy3.js) |
| `xianziCode` | 哦齁编码（仙子曰） | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/memeCodecs.js) |
| `hajimiCode` | 哈基米加密（三进制） | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/memeCodecs.js) |
| `zakoCode` | 杂鱼编码（Base8 开源版） | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/memeCodecs.js) |
| `zakoMystery` | 杂鱼密语（官方版） | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/zakoOfficial.js) |
| `hashTypeIdentify` | 哈希类型识别 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashCrack.js) |
| `hashDictCrack` | 哈希字典爆破（小字典演示） | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashCrack.js) |
| `rainbowQuery` | 彩虹表查询（本地演示小表） | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashCrack.js) |
| `hmacKeyBrute` | HMAC 密钥爆破（小字典演示） | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashCrack.js) |
| `vigenereAuto` | 维吉尼亚全自动破解 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cryptanalysis.js) |
| `hillKnownPlain` | Hill 已知明文攻击 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cryptanalysis.js) |
| `playfairCrack` | Playfair 爬山破解 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cryptanalysis.js) |
| `freqAnalysis` | 频率分析（n-gram） | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cryptanalysis.js) |
| `icAnalysis` | 重合指数 IC（含分组） | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cryptanalysis.js) |
| `kasiskiTest` | Kasiski 检验 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cryptanalysis.js) |
| `chiSquareAnalysis` | 卡方检验（详细） | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cryptanalysis.js) |
| `subCipherSolver` | 单表替换自动求解 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cryptanalysis.js) |
| `caesarBrute` | 凯撒/ROT 自动求位移 | analysis | existing-review | `caesar_brute` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cryptanalysis.js) |
| `rabbit` | Rabbit 流密码 | stream | existing-review | `rabbit` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cryptoTryAll.js) |
| `godzillaPhpXorBase64` | 哥斯拉 PHP_XOR_BASE64 | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/webshell.js) |
| `behinderAesEcb` | 冰蝎 AES-ECB | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/webshell.js) |
| `qrScanImage` | 二维码扫描解析 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/qrscan.js) |
| `layoutMap` | 键盘布局映射 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/keyboardExt.js) |
| `t9Phone` | 手机九宫格 T9 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/keyboardExt.js) |
| `stenoLetter` | Steno 速记字母 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/keyboardExt.js) |
| `arrowKey` | 方向键编码 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/keyboardExt.js) |
| `rsaParams` | RSA 参数计算（p,q→n,φ,d） | asym | existing-review | `rsa_params` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsatool.js) |
| `rsaSmallE` | RSA 小 e 攻击（整数开根） | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsatool.js) |
| `rsaCommonModulus` | RSA 共模攻击 | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsatool.js) |
| `rsaWiener` | RSA Wiener 攻击（连分数） | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsatool.js) |
| `rsaFermat` | 费马分解（p,q 相近） | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsatool.js) |
| `rsaPollard` | Pollard rho 分解 | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsatool.js) |
| `rsaModinv` | 模逆（a⁻¹ mod m） | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsatool.js) |
| `rsaEgcd` | 扩展欧几里得（Bézout） | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsatool.js) |
| `rsaCrt` | 中国剩余定理 CRT | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsatool.js) |
| `rsaModpow` | 大数快速幂（base^exp mod m） | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsatool.js) |
| `lllAttack` | 格基归约 LLL 攻击 | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsatoolExt.js) |
| `rsaDpDqLeak` | RSA dp/dq 泄露求 d | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsatoolExt.js) |
| `rsaLsbOracle` | RSA LSB Oracle 攻击 | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsatoolExt.js) |
| `rsaBleichenbacher` | RSA Bleichenbacher 攻击 | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsatoolExt.js) |
| `rsaCoppersmith` | RSA Coppersmith 小根攻击 | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsatoolExt.js) |
| `rsaBonehDurfee` | RSA Boneh-Durfee 提示 | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsatoolExt.js) |
| `pemParse` | PEM/DER 结构解析 | data | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/eccdetect.js) |
| `asn1Parse` | ASN.1 TLV 解析 | data | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/eccdetect.js) |
| `ecCurveIdent` | 椭圆曲线参数识别 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/eccdetect.js) |
| `sshPubkeyParse` | SSH 公钥解析 | data | migrated | `ssh_public_key_parse` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/eccdetect.js) |
| `btcAddressIdent` | 比特币地址识别 | data | migrated | `bitcoin_address_inspect` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/eccdetect.js) |
| `ethAddressIdent` | 以太坊地址识别 | data | migrated | `ethereum_address_inspect` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/eccdetect.js) |
| `gbCharset` | GBK / GB2312 / GB18030 | text | migrated | `gb_charset_encode`, `gb_charset_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/charset.js) |
| `gb2312QuWei` | GB2312 区位码 | text | migrated | `gb2312_quwei_encode`, `gb2312_quwei_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/charset.js) |
| `big5` | Big5 繁体中文 | text | migrated | `big5_encode`, `big5_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/charset.js) |
| `shiftJis` | Shift-JIS 日文 | text | migrated | `shift_jis_encode`, `shift_jis_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/charset.js) |
| `eucKr` | EUC-KR 韩文 | text | migrated | `euc_kr_encode`, `euc_kr_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/charset.js) |
| `latinCharset` | Latin / ISO-8859 / Windows 单字节 | text | migrated | `latin_charset_encode`, `latin_charset_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/charset.js) |
| `ebcdic` | EBCDIC | text | migrated | `ebcdic_encode`, `ebcdic_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/charset.js) |
| `utf16` | UTF-16 BE/LE | text | migrated | `utf16_encode`, `utf16_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/charset.js) |
| `mojibakeFix` | 乱码修复 (Mojibake) | text | migrated | `mojibake_encode`, `mojibake_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/charset.js) |
| `manchester` | 曼彻斯特编码 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/signal.js) |
| `diffManchester` | 差分曼彻斯特编码 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/signal.js) |
| `nrzi` | NRZI 编码 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/signal.js) |
| `miller` | 密勒码 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/signal.js) |
| `fourB5B` | 4B5B 编码 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/signal.js) |
| `pwmPpm` | PWM/PPM 脉冲调制 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/signal.js) |
| `geoDms` | 度分秒 ↔ 十进制 | radix | migrated | `geo_dms_encode`, `geo_dms_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/geo.js) |
| `geoHash` | Geohash 编码 | radix | migrated | `geo_hash_encode`, `geo_hash_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/geo.js) |
| `geoPlusCode` | Plus Code / OLC | radix | migrated | `geo_plus_code_encode`, `geo_plus_code_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/geo.js) |
| `geoMaidenhead` | Maidenhead 网格 | radix | migrated | `geo_maidenhead_encode`, `geo_maidenhead_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/geo.js) |
| `geoUtm` | UTM 坐标 | radix | migrated | `geo_utm_encode`, `geo_utm_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/geo.js) |
| `color` | 颜色编码互转 | radix | migrated | `color_convert_encode`, `color_convert_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/color.js) |
| `colorInfo` | 颜色全息信息 | radix | migrated | `color_info` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/color.js) |
| `musicNotation` | 音乐记号互转 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/music.js) |
| `musicInfo` | 音符全息信息 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/music.js) |
| `crcGeneric` | 通用 CRC（参数化） | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/checksumExt.js) |
| `crc16Modbus` | CRC-16/MODBUS | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/checksumExt.js) |
| `crc16CcittTrue` | CRC-16/CCITT-FALSE | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/checksumExt.js) |
| `crc16Arc` | CRC-16/ARC | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/checksumExt.js) |
| `crc16Xmodem` | CRC-16/XMODEM | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/checksumExt.js) |
| `fletcher` | Fletcher | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/checksumExt.js) |
| `bsdSum` | BSD checksum | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/checksumExt.js) |
| `sysvSum` | SysV checksum | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/checksumExt.js) |
| `bitReverse` | 位反转 | radix | migrated | `bit_reverse_encode`, `bit_reverse_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bitops.js) |
| `bitRotate` | 位循环移位 | radix | migrated | `bit_rotate_encode`, `bit_rotate_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bitops.js) |
| `byteSwap` | 字节序反转 | radix | migrated | `byte_swap_encode`, `byte_swap_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bitops.js) |
| `bitPlaneExtract` | 位平面提取 | radix | migrated | `bit_plane_extract` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bitops.js) |
| `ror13Hash` | ROR13 API 哈希 | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bitops.js) |
| `byteArith` | 字节算术 (mod 256) | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bitops.js) |
| `byteReverse` | 整串字节倒序 | radix | migrated | `byte_reverse_encode`, `byte_reverse_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bitops.js) |
| `urlQueryParse` | URL Query 解析 | text | migrated | `url_query_parse` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/netcodecExt.js) |
| `cookieParse` | Cookie 解析 | text | migrated | `cookie_parse` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/netcodecExt.js) |
| `httpBasicAuth` | HTTP Basic 认证 | text | migrated | `http_basic_auth_encode`, `http_basic_auth_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/netcodecExt.js) |
| `dataUriParse` | Data URI 解析 | text | migrated | `data_uri_encode`, `data_uri_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/netcodecExt.js) |
| `magnetParse` | Magnet 链接解析 | text | migrated | `magnet_parse` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/netcodecExt.js) |
| `protobufParse` | Protobuf Wire 解析 | data | migrated | `protobuf_wire_parse` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/serial.js) |
| `msgpackParse` | MessagePack 解析 | data | migrated | `msgpack_parse` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/serial.js) |
| `cborParse` | CBOR 解析 | data | migrated | `cbor_parse` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/serial.js) |
| `bsonParse` | BSON 文档解析 | data | migrated | `bson_parse` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/serial.js) |
| `phpSerializeParse` | PHP serialize 解析 | data | migrated | `php_serialize_parse` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/serial.js) |
| `javaSerializeIdent` | Java 序列化识别 | data | migrated | `java_serialize_identify` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/serial.js) |
| `julianDate` | 儒略日 ↔ ISO8601 | radix | migrated | `julian_date_encode`, `julian_date_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/timecodecExt.js) |
| `excelDate` | Excel 序列日期 ↔ ISO8601 | radix | migrated | `excel_date_encode`, `excel_date_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/timecodecExt.js) |
| `chromeTime` | Chrome 时间 ↔ ISO8601 | radix | migrated | `chrome_time_encode`, `chrome_time_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/timecodecExt.js) |
| `snowflakeId` | 雪花 ID 解析 | radix | migrated | `snowflake_parse` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/timecodecExt.js) |
| `hexRange` | Hex 区间提取 | data | migrated | `hex_range` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hexview.js) |
| `hexStats` | 字节分布统计 | data | migrated | `hex_stats` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hexview.js) |
| `wavHeader` | WAV 头解析 | audio | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/audiostego.js) |
| `audioLsb` | 音频 LSB 提取 | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/audiostego.js) |
| `audioLsbEmbed` | 音频 LSB 嵌入（出题） | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/audiostego.js) |
| `sstvIdent` | SSTV 模式识别 | audio | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/audiostego.js) |
| `pcmTransforms` | PCM 波形变换 | audio | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pcmTransforms.js) |
| `barcodeGen` | 条码生成（Code128/EAN-13/DataMatrix） | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/barcodeGen.js) |
| `sstv` | SSTV 慢扫描电视 编/解 | audio | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/sstv.js) |
| `deadfish` | Deadfish | esolang | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/esolang2.js) |
| `befunge` | Befunge-93 执行 | esolang | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/esolang2.js) |
| `emojicodeIdent` | Emojicode 识别 | esolang | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/esolang2.js) |
| `pietIdent` | Piet 识别 | esolang | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/esolang2.js) |
| `malbolgeExec` | Malbolge 执行 | esolang | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/malbolgeExec.js) |
| `diffTool` | 差异对比 | data | migrated | `diff_tool` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/difftool.js) |
| `otp` | 一次一密 OTP | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classicExt3.js) |
| `keywordcipher` | 关键字密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classicExt3.js) |
| `simplesub` | 简单替换 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classicExt3.js) |
| `runningkey` | 滚动密钥 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classicExt3.js) |
| `homophonic` | 同音替换 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classicExt4.js) |
| `doubleTrans` | 双重列移位 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classicExt4.js) |
| `pollux` | Pollux 密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classicExt4.js) |
| `morbit` | Morbit 密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classicExt4.js) |
| `bookCipher` | 书卷密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classicExt4.js) |
| `turningGrille` | 转动格栅 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classicExt4.js) |
| `kenny` | Kenny 语 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/classicExt4.js) |
| `hexdump` | Hexdump 互转（xxd） | text | migrated | `to_hexdump`, `from_hexdump` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/engEncoding.js) |
| `modhex` | Modhex（YubiKey） | base | migrated | `modhex_encode`, `modhex_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/engEncoding.js) |
| `citrixCtx1` | Citrix CTX1 | base | migrated | `citrix_ctx1_encode`, `citrix_ctx1_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/engEncoding.js) |
| `scriptDecoder` | MS 脚本解码（.vbe/.jse） | filefmt | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/engEncoding.js) |
| `rison` | Rison | data | migrated | `rison_encode`, `rison_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/engEncoding.js) |
| `unixPerms` | UNIX 文件权限 | radix | migrated | `unix_permissions` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/engEncoding.js) |
| `crockford32` | Crockford Base32 | base | migrated | `crockford32_encode`, `crockford32_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/encodingExt3.js) |
| `alienAlphabet` | 外星字母 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/encodingExt3.js) |
| `futhark` | 卢恩符文 Futhark | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/encodingExt3.js) |
| `countingRods` | 算筹数字 | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/encodingExt3.js) |
| `chuckUnary` | Chuck Norris 一元码 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/encodingExt3.js) |
| `wingdings` | Wingdings 符号字体 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/encodingExt3.js) |
| `cardanGrille` | 卡丹格 Cardan | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/encodingExt3.js) |
| `rle` | RLE 行程编码 | archive | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/compressExt2.js) |
| `lzw` | 标准 LZW（GIF/TIFF） | archive | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/compressExt2.js) |
| `elias` | Elias Gamma/Delta 编码 | archive | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/compressExt2.js) |
| `verhoeff` | Verhoeff 校验 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/compressExt2.js) |
| `lz4Dec` | LZ4 解压 | archive | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/compressExt2.js) |
| `bzip2Dec` | bzip2 解压 | archive | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/compressExt2.js) |
| `fracmorse` | 分数摩斯 FracMorse | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fracmorse.js) |
| `hammingCode` | 海明码 Hamming Code | radix | migrated | `hamming_code_encode`, `hamming_code_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hamming.js) |
| `snow` | SNOW 空白隐写 | stegoText | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/snow.js) |
| `bazeries` | Bazeries 密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bazeries.js) |
| `fenham` | Fenham 密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fenham.js) |
| `pizzini` | Pizzini 密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pizzini.js) |
| `kamasutra` | Kamasutra 爱经密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/kamasutra.js) |
| `suiYanSuiYu` | 随言随语 | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/suiYanSuiYu.js) |
| `fuyouyue` | 佛又曰 | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fuyouyue.js) |
| `tianshu` | 天书 | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fuyouyue.js) |
| `xiongyue` | 熊曰 | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/xiongyue.js) |
| `huoxingwen` | 火星文 | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/huoxingwen.js) |
| `jianfan` | 简繁转换 | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/huoxingwen.js) |
| `qqxiuzi_arrow` | 千千秀字·箭头 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/qqxiuzi_hex.js) |
| `qqxiuzi_flower` | 千千秀字·花 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/qqxiuzi_hex.js) |
| `qqxiuzi_ipa` | 千千秀字·IPA | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/qqxiuzi_hex.js) |
| `qqxiuzi_letter` | 千千秀字·字母 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/qqxiuzi_hex.js) |
| `qqxiuzi_braille` | 千千秀字·盲文 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/qqxiuzi_misc.js) |
| `qqxiuzi_chinese` | 千千秀字·汉字 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/qqxiuzi_misc.js) |
| `qqxiuzi_music` | 千千秀字·音乐 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/qqxiuzi_misc.js) |
| `lolcode` | LOLCODE | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/lolcode.js) |
| `enigma` | Enigma 恩尼格玛机 | classic | existing-review | `enigma` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/enigma.js) |
| `m209` | M-209 转轮密码机 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/m209.js) |
| `bwt` | BWT 块排序变换 | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bwt.js) |
| `clockCipher` | 表盘码 / 时钟码 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/clockcipher.js) |
| `bech32` | Bech32 编码 | radix | migrated | `bech32_encode`, `bech32_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bech32.js) |
| `uuidParse` | UUID 解析 | radix | migrated | `uuid_parse` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/byteTools.js) |
| `varint` | VarInt (LEB128) | radix | migrated | `varint_encode`, `varint_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/byteTools.js) |
| `lzstring` | LZString 压缩 (LZW) | archive | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/lzcodec.js) |
| `flashSwirl` | FlashSwirl 闪旋 | stream | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/flashswirl.js) |
| `flaskSessionDecode` | Flask Session 解码 | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/flasksession.js) |
| `flaskSessionSign` | Flask Session 签发 | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/flasksession.js) |
| `flaskSessionVerify` | Flask Session 验签 | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/flasksession.js) |
| `sevenZipExtract` | 7z 归档解析 / 解压 | archive | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/sevenzip.js) |
| `pycExeDecompile` | pyc/exe 反编（本地桥） | filefmt | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/exebridge.js) |
| `crc32Collision` | CRC32 碰撞爆破 | crack | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/crc32collision.js) |
| `rotSpecial` | Rot 任意位移 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rotspecial.js) |
| `rot8000` | ROT8000 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rotspecial.js) |
| `pickleDisasm` | Pickle 反汇编 | data | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pickle.js) |
| `jjencode` | JJEncode | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/jjencode.js) |
| `sm2KeyGen` | SM2 密钥对生成 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/sm2.js) |
| `sm2Encrypt` | SM2 加密 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/sm2.js) |
| `sm2Decrypt` | SM2 解密 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/sm2.js) |
| `sm2Sign` | SM2 签名 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/sm2.js) |
| `sm2Verify` | SM2 验签 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/sm2.js) |
| `sm2KeyExchange` | SM2 密钥交换 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/sm2.js) |
| `zuc` | ZUC 祖冲之 | stream | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/sm.js) |
| `sm9KeyGen` | SM9 密钥生成 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/sm9ops.js) |
| `sm9Sign` | SM9 签名 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/sm9ops.js) |
| `sm9Verify` | SM9 验签 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/sm9ops.js) |
| `sm9Encrypt` | SM9 加密 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/sm9ops.js) |
| `sm9Decrypt` | SM9 解密 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/sm9ops.js) |
| `archiveUnified` | 压缩 / 归档归一分析 | archive | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/archiveUnified.js) |
| `spoon` | Spoon | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/spoon.js) |
| `sstiKeyword` | SSTI 关键字识别 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ssti.js) |
| `numToPinyin` | 数字转拼音 | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pinyin.js) |
| `hanziToPinyin` | 汉字转拼音 | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pinyin.js) |
| `goldbug` | GoldBug 金甲虫密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/goldbug.js) |
| `trafficReadable` | 流量可读结论 | forensic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/trafficReadable.js) |
| `usbKeyboard` | USB 键盘流量解析 | forensic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/usbHid.js) |
| `usbMouse` | USB 鼠标流量解析 | forensic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/usbHid.js) |
| `jphswinLaunch` | JPHS · JPEG 隐写 | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/exeTools.js) |
| `openpuffLaunch` | OpenPuff · 多载体 | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/exeTools.js) |
| `oursecretLaunch` | OurSecret · 隐写 | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/exeTools.js) |
| `acrostic` | 藏头/藏尾/藏中 | stegoText | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/textStego.js) |
| `everyN` | 等距取字隐写 | stegoText | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/textStego.js) |
| `caseBitStego` | 大小写位隐写 | stegoText | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/textStego.js) |
| `nthChar` | 第 N 字隐写 | stegoText | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/textStego.js) |
| `wordSpacingBits` | 词距位隐写 | stegoText | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/textStego.js) |
| `imageStructUnified` | 图像结构分析（归一） | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/imageStructUnified.js) |
| `cryptoAddrUnified` | 加密货币地址解析 | data | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cryptoAddrUnified.js) |
| `fullwidth` | 全角密码 | text | migrated | `fullwidth_encode`, `fullwidth_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fullwidth.js) |
| `routeCipher` | 曲路密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/routecipher.js) |
| `md2` | MD2 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/md2.js) |
| `hotp` | HOTP | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/otp.js) |
| `totp` | TOTP | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/otp.js) |
| `pbkdf2` | PBKDF2 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/kdf.js) |
| `hkdf` | HKDF | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/kdf.js) |
| `pbeAesBrute` | PBE-AES 口令爆破（小字典演示） | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/kdf.js) |
| `pietExec` | Piet 执行 | esolang | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pietExec.js) |
| `carbonaro` | Carbonaro 码 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/carbonaro.js) |
| `albam` | Albam 码 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/albam.js) |
| `blub` | Blub! | esolang | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bfDialects.js) |
| `cow` | COW / MOO | esolang | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bfDialects.js) |
| `brainlollerDecode` | Brainloller 解码 | esolang | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bftoolsImg.js) |
| `brainlollerEncode` | Brainloller 编码 | esolang | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bftoolsImg.js) |
| `braincopterDecode` | Braincopter 解码 | esolang | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bftoolsImg.js) |
| `braincopterEncode` | Braincopter 编码 | esolang | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bftoolsImg.js) |
| `x25519KeyGen` | X25519 密钥生成 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/xwing.js) |
| `x25519Shared` | X25519 共享密钥（双方私钥） | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/xwing.js) |
| `x25519SharedFromPub` | X25519 共享密钥（私钥+对方公钥） | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/xwing.js) |
| `xwingKeyGen` | X-Wing 密钥生成 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/xwing.js) |
| `xwingEncaps` | X-Wing 封装 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/xwing.js) |
| `xwingDecaps` | X-Wing 解封装 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/xwing.js) |
| `jsteg` | jsteg JPEG 隐写 编/解 | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/jsteg.js) |
| `dtmfWav` | DTMF 拨号音 WAV | audio | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/dtmfWav.js) |
| `morseWav` | 摩斯音频编解码 | audio | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/morseWav.js) |
| `xmssKeyGen` | XMSS 密钥生成 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/xmssLms.js) |
| `xmssSign` | XMSS 签名 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/xmssLms.js) |
| `xmssVerify` | XMSS 验签 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/xmssLms.js) |
| `lmsSign` | LMS/HSS 签名 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/xmssLms.js) |
| `lmsVerify` | LMS/HSS 验签 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/xmssLms.js) |
| `hqcKeyGen` | HQC 密钥生成 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hqc.js) |
| `hqcEncrypt` | HQC 加密 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hqc.js) |
| `hqcDecrypt` | HQC 解密 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hqc.js) |
| `ntruKeyGen` | NTRU 密钥生成（真参数） | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ntruReal.js) |
| `ntruEncrypt` | NTRU 加密（真参数） | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ntruReal.js) |
| `ntruDecrypt` | NTRU 解密（真参数） | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ntruReal.js) |
| `blsKeyGen` | BLS 密钥生成 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bls.js) |
| `blsSign` | BLS 签名 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bls.js) |
| `blsVerify` | BLS 验签 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bls.js) |
| `blsAggVerify` | BLS 聚合验签 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bls.js) |
| `merkleProve` | Merkle 包含证明 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/protocolSig.js) |
| `merkleVerify` | Merkle 证明验证 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/protocolSig.js) |
| `pedersenCommit` | Pedersen 承诺 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/protocolSig.js) |
| `feldmanVss` | Feldman VSS | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/protocolSig.js) |
| `lsagSign` | LSAG 环签名 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/protocolSig.js) |
| `lsagVerify` | LSAG 环签名验证 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/protocolSig.js) |
| `twinHex` | Twin-Hex 双字符编码 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ctfCipherExt.js) |
| `trollScript` | TrollScript | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ctfCipherExt.js) |
| `asciiSum` | ASCII 前缀累加和 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ctfCipherExt.js) |
| `caesarBox` | 凯撒箱换位 Caesar Box | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ctfCipherExt.js) |
| `curveCipher` | 曲路密码 Curve Cipher | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ctfCipherExt.js) |
| `zipBrute` | ZIP 弱口令爆破（小字典快验） | crack | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/zipCrack.js) |
| `chaocipher` | Chaocipher | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/chaocipher.js) |
| `zip2john` | ZIP 哈希提取（zip2john） | crack | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/john_zip.js) |
| `rsaBatchGcd` | RSA 公共因子分解（批量 GCD） | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsaBatchGcd.js) |
| `rsaHastad` | RSA Hastad 广播攻击 | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsaHastad.js) |
| `rsaPollardPm1` | RSA Pollard p-1 分解 | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsaPollardPm1.js) |
| `straddleCheckerboard` | 跨界棋盘 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/straddleCheckerboard.js) |
| `xorCribDrag` | XOR crib-drag 已知明文拖动 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/xorCribDrag.js) |
| `zipCrc32Brute` | ZIP CRC32 内容爆破 | crack | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/zipCrc32Brute.js) |
| `nihilistCipher` | Nihilist 密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/nihilist.js) |
| `solitaire` | Solitaire 扑克流密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/solitaire.js) |
| `ls47` | LS47 字母牌密码 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ls47.js) |
| `alberti` | Alberti 圆盘 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/alberti.js) |
| `wabun` | Wabun 和文摩尔斯 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/wabun.js) |
| `gematria` | Gematria 数值 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/gematria.js) |
| `foursquarekw` | Four-square 四方（keyword） | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/squareCiphers.js) |
| `twosquare` | Two-square 双方 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/squareCiphers.js) |
| `sevenZip2john` | 7z 哈希提取（7z2john） | crack | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/john_7z.js) |
| `pdf2john` | PDF 哈希提取（pdf2john） | crack | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/john_pdf.js) |
| `rar2john` | RAR 哈希提取（rar2john） | crack | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/john_rar.js) |
| `sshkey2john` | SSH 私钥哈希提取（sshkey2john） | crack | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/john_ssh.js) |
| `rc2` | RC2 | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cryptoGap.js) |
| `lmHash` | LM Hash | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cryptoGap.js) |
| `evpBytesToKey` | EVP_BytesToKey | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cryptoGap.js) |
| `dctWatermark` | DCT 盲水印 | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/blindWatermark.js) |
| `dualFftWatermark` | 双图盲水印(F) 频域叠加 | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/blindWatermarkDualFft.js) |
| `outguess` | OutGuess 隐写（双向） | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/outguess.js) |
| `steghide` | steghide 隐写（双向） | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/steghide.js) |
| `gifshuffle` | GifShuffle 调色板隐写 | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/gifshuffle.js) |
| `watermarkhFft` | WaterMarkH 频域隐形水印 | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/watermarkhFft.js) |
| `dwtSvdWatermark` | DWT-DCT-SVD 盲水印 | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/blindWatermarkDwtSvd.js) |
| `mirageTank` | 幻影坦克 | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cnImageStego.js) |
| `tomatoScramble` | 小番茄图片混淆 | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cnImageStego.js) |
| `astroSymbols` | 天文 / 黄道符号 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/astroSymbols.js) |
| `pipNumerals` | 点数记数（骰面 / 骨牌） | radix | migrated | `pip_numerals_encode`, `pip_numerals_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pipNumerals.js) |
| `bkcrackAttack` | ZipCrypto 已知明文攻击 (bkcrack) | crack | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bkcrack.js) |
| `shaLengthExtend` | SHA 长度扩展 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ftpExtract.js) |
| `birthdayCollision` | 生日碰撞演示 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ftpExtract.js) |
| `ftpExtract` | FTP 对象提取 | forensic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ftpExtract.js) |
| `tlsDecrypt` | TLS1.2 已知密钥还原 | forensic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/tlsDecrypt.js) |
| `wpaDecrypt` | WPA2 握手校验/CCMP 解密 | forensic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/wpaDecrypt.js) |
| `dlp` | 离散对数求解（DLP） | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/dlp.js) |
| `bigCalc` | 大数计算器（BigInt） | radix | migrated | `big_integer_calc` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bigmath.js) |
| `primeInspector` | 素数判定与筛选 | radix | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/primeInspector.js) |
| `randomSeed` | 随机种子生成 | radix | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/randomSeed.js) |
| `randomSecure` | 增强随机安全 | radix | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/randomSecure.js) |
| `randBeacon` | 公开随机信标验证 | radix | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/randBeacon.js) |
| `dictGen` | 字典生成 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/dictGen.js) |
| `elgamal` | ElGamal | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/elgamal.js) |
| `elgamalKeyGen` | ElGamal 密钥生成 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/elgamalKeyGen.js) |
| `pemToHex` | PEM → DER | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pemkeys.js) |
| `hexToPem` | DER → PEM | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pemkeys.js) |
| `pemToJwk` | PEM → JWK | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pemkeys.js) |
| `jwkToPem` | JWK → PEM | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pemkeys.js) |
| `pubFromPriv` | 私钥 → 公钥 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pemkeys.js) |
| `ascon` | Ascon-AEAD128 | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ascon.js) |
| `asconHash` | Ascon-Hash256 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ascon.js) |
| `aesKeyWrap` | AES Key Wrap | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/keywrap.js) |
| `rsaSign` | RSA 签名 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsasign.js) |
| `rsaVerify` | RSA 验签 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rsasign.js) |
| `ed448Sign` | Ed448 签名 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ed448x448.js) |
| `ed448Verify` | Ed448 验签 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ed448x448.js) |
| `x448KeyGen` | X448 密钥生成 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ed448x448.js) |
| `x448Shared` | X448 共享密钥 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ed448x448.js) |
| `streebog` | Streebog（GOST R 34.11-2012） | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/gostsign.js) |
| `gostSign` | GOST R 34.10-2012 签名 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/gostsign.js) |
| `gostVerify` | GOST R 34.10-2012 验签 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/gostsign.js) |
| `pgpGenKeyPair` | PGP 密钥对生成 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pgp.js) |
| `pgpEncrypt` | PGP 加密 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pgp.js) |
| `pgpDecrypt` | PGP 解密 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pgp.js) |
| `pgpSign` | PGP 签名 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pgp.js) |
| `pgpVerify` | PGP 验签 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pgp.js) |
| `pgpEncryptAndSign` | PGP 加密并签名 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pgp.js) |
| `pgpDecryptAndVerify` | PGP 解密并验签 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pgp.js) |
| `pgpParseKey` | PGP 密钥解析 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pgp.js) |
| `bb84Qkd` | BB84 量子密钥分发仿真 | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/quantumBB84.js) |
| `ed25519KeyGen` | Ed25519 密钥生成 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/tokensign.js) |
| `ed25519Sign` | Ed25519 签名 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/tokensign.js) |
| `ed25519Verify` | Ed25519 验签 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/tokensign.js) |
| `jwsSign` | JWS 签发 | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/tokensign.js) |
| `jwsVerify` | JWS 验签 | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/tokensign.js) |
| `jweEncrypt` | JWE 加密 | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/tokensign.js) |
| `jweDecrypt` | JWE 解密 | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/tokensign.js) |
| `pasetoV4Sign` | PASETO v4 签发（public） | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/tokensign.js) |
| `pasetoV4Verify` | PASETO v4 验签（public） | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/tokensign.js) |
| `mldsaKeyGen` | ML-DSA 密钥生成 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/mldsa.js) |
| `mldsaSign` | ML-DSA 签名 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/mldsa.js) |
| `mldsaVerify` | ML-DSA 验签 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/mldsa.js) |
| `slhdsaKeyGen` | SLH-DSA 密钥生成 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/slhdsa.js) |
| `slhdsaSign` | SLH-DSA 签名 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/slhdsa.js) |
| `slhdsaVerify` | SLH-DSA 验签 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/slhdsa.js) |
| `emojiAes` | emoji-aes 加密 | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/emojiAes.js) |
| `moyue` | 魔曰 | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/moyue.js) |
| `mcTextExtract` | Minecraft 文本情报提取 | forensic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/mcText.js) |
| `bin2img` | 二进制转图片 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bin2img.js) |
| `img2dataURI` | 图片转 Base64/dataURI | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bin2img.js) |
| `dataURIToImage` | Base64/dataURI 转图片 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bin2img.js) |
| `img2bits` | 图片转二进制点阵 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bin2img.js) |
| `imageToData` | 图片还原数值数据 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/dataToImage.js) |
| `dataToImage` | 数值数据渲染成图 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/dataToImage.js) |
| `huffmanCodec` | 哈夫曼编解码（通用） | data | migrated | `huffman_encode`, `huffman_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/huffman.js) |
| `imgFft` | 图像 2D FFT 幅度谱 | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/imgFft.js) |
| `mcNbtView` | Minecraft NBT 树查看器 | forensic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/mcNbt.js) |
| `formatSniff` | 格式嗅探 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/formatSniff.js) |
| `bcrypt` | Bcrypt | hash | existing-review | `bcrypt` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bcrypt.js) |
| `prngAttack` | PRNG 破解（LCG / MT19937） | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/prngAttack.js) |
| `hashLengthExtension` | 哈希长度扩展攻击（MD5/SHA1/SHA256） | crypto | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hashLengthExtension.js) |
| `flagExtract` | flag 自动提取器 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/flagExtract.js) |
| `xorAnalyze` | xortool 一体化（重复密钥 XOR 分析） | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/xorAnalyze.js) |
| `ttlStego` | TTL 隐写（IP 包 TTL 序列） | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ttlStego.js) |
| `spiralMatrix` | 螺旋矩阵读取 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/spiralMatrix.js) |
| `nonogram` | 数织 / Nonogram 求解 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/nonogram.js) |
| `simonSpeck` | Simon / Speck 轻量密码 | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/simonSpeck.js) |
| `knapsack` | 背包加密（Merkle-Hellman） | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/knapsack.js) |
| `dsaParamGen` | DSA 密钥对/参数组生成 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/dsa.js) |
| `dsaSign` | DSA 签名 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/dsa.js) |
| `dsaVerify` | DSA 验签 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/dsa.js) |
| `dsaReuseK` | DSA 重用 k 攻击 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/dsa.js) |
| `shamir` | Shamir 秘密共享 | modern | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/shamir.js) |
| `bmpPalette` | BMP 调色板隐写分析 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bmpPalette.js) |
| `stegosaurus` | Stegosaurus pyc 隐写检测 | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegosaurus.js) |
| `bubblebabble` | BubbleBabble 编码 | text | migrated | `bubblebabble_encode`, `bubblebabble_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/bubblebabble.js) |
| `jsEscape` | JS escape 编码 | text | migrated | `js_escape_encode`, `js_escape_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/jsEscape.js) |
| `ppencode` | ppencode | text | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ppencode.js) |
| `stegpy` | stegpy 隐写（stegv3） | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stegpy.js) |
| `stereogramSolver` | 立体图求解 | stegoFile | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stereogram.js) |
| `x509Parse` | X.509 证书解析 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/certparse.js) |
| `sshHostKeyParse` | SSH 公钥解析（authorized_keys/known_hosts） | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/certparse.js) |
| `csrParse` | CSR 证书请求解析（PKCS#10） | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/csrlparse.js) |
| `crlParse` | X.509 CRL 吊销列表解析 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/csrlparse.js) |
| `des2Mitm` | 2DES 中间相遇 | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/des2Mitm.js) |
| `mimeMultipart` | MIME multipart 解析 | text | migrated | `mime_multipart_encode`, `mime_multipart_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/mimeMultipart.js) |
| `randu` | RANDU 弱 LCG | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/lcgMore.js) |
| `truncLcgRecover` | 截断 LCG 种子恢复 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/lcgMore.js) |
| `babaiCvp` | Babai 最近平面（CVP） | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/latticeMore.js) |
| `hnpRecover` | HNP 隐藏数问题 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/latticeMore.js) |
| `spnAnalysis` | SPN 差分/线性分析 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/spnAnalysis.js) |
| `md5CollisionShow` | MD5 截断碰撞演示 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/collisionShow.js) |
| `lweToy` | LWE 玩具加解密 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pqcLite.js) |
| `ntruToy` | NTRU 玩具加解密 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pqcLite.js) |
| `crc32Reverse` | CRC32 反向碰撞 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/crc32Reverse.js) |
| `roar` | 兽音译者（嗷呜啊~） | fancy | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/roar.js) |
| `geffeGenerate` | Geffe 生成器 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/geffe.js) |
| `geffeAttack` | Geffe 相关攻击 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/geffe.js) |
| `pcapRepair` | pcap 文件修复 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pcapRepair.js) |
| `spectrogram` | 音频频谱图（STFT） | audio | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/spectrogram.js) |
| `lfsrRecover` | LFSR 序列恢复 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/lfsrRecover.js) |
| `xorshiftRecover` | xorshift 状态恢复 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/xorshiftRecover.js) |
| `yenc` | yEnc 编 / 解码 | text | migrated | `yenc_encode`, `yenc_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/yenc.js) |
| `binhex` | BinHex 4.0 编 / 解码 | text | migrated | `binhex_encode`, `binhex_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/binhex.js) |
| `a51` | A5/1 流密码 | stream | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/a51.js) |
| `a52` | A5/2 流密码 | stream | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/a52.js) |
| `e0` | E0 流密码 | stream | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/e0.js) |
| `hc128` | HC-128 流密码 | stream | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hc128.js) |
| `hc256` | HC-256 流密码 | stream | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/hc256.js) |
| `sosemanuk` | Sosemanuk | stream | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/sosemanuk.js) |
| `spritz` | Spritz 流密码 | stream | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/spritz.js) |
| `vmpc` | VMPC 流密码 | stream | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/vmpc.js) |
| `mickey` | MICKEY-128 2.0 | stream | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/mickey.js) |
| `rabin` | Rabin 密码 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rabin.js) |
| `siphash` | SipHash-2-4 / 1-3 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/siphash.js) |
| `scrypt` | scrypt 密钥派生 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/scrypt.js) |
| `balloon` | Balloon 密钥派生 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/balloon.js) |
| `lyra2` | Lyra2 密钥派生 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/lyra2.js) |
| `yescrypt` | yescrypt 密钥派生 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/yescrypt.js) |
| `blake3` | BLAKE3 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/blake3.js) |
| `paillierKeyGen` | Paillier 密钥对生成 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/paillier.js) |
| `paillierEncrypt` | Paillier 加密 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/paillier.js) |
| `paillierDecrypt` | Paillier 解密 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/paillier.js) |
| `paillierHomAdd` | Paillier 同态加 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/paillier.js) |
| `schnorrKeyGen` | Schnorr 密钥对生成 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/schnorr.js) |
| `schnorrSign` | Schnorr 签名 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/schnorr.js) |
| `schnorrVerify` | Schnorr 验签 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/schnorr.js) |
| `schnorrReuseK` | Schnorr nonce 重用攻击 | asym | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/schnorr.js) |
| `magma` | Magma（GOST R 34.12-2015） | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/magma.js) |
| `present` | PRESENT 轻量分组密码 | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/present.js) |
| `serpent` | Serpent | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/serpent.js) |
| `aria` | ARIA（RFC 5794） | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/aria.js) |
| `pearson` | Pearson 哈希 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pearson.js) |
| `whirlpool` | Whirlpool | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/whirlpool.js) |
| `threefish` | Threefish 可调分组密码 | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/skein.js) |
| `skein` | Skein | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/skein.js) |
| `grostl` | Grøstl | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/grostl.js) |
| `jh` | JH | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/jh.js) |
| `skipjack` | Skipjack 分组密码 | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/skipjack.js) |
| `mars` | MARS 分组密码 | block | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/mars.js) |
| `xxhash` | xxHash 极速哈希 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/xxhash.js) |
| `cityhash` | CityHash 非加密哈希 | hash | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/cityhash.js) |
| `rc4Visualize` | RC4 KSA/PRGA 可视化 | analysis | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rc4Visualize.js) |
| `xiangyue` | 想曰 XiangYue | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/xiangyue.js) |
| `xiangyueEnc` | 想曰 XiangYue 加密 | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/xiangyue.js) |
| `trivium` | Trivium 流密码 | stream | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/lightweightStream.js) |
| `grainV1` | Grain v1 流密码 | stream | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/lightweightStream.js) |
| `grain128aead` | Grain-128AEAD 认证加密 | stream | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/lightweightStream.js) |
| `dxBase64` | DXBase64 | base | migrated | `dx_base64_encode`, `dx_base64_decode` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fengCodec.js) |
| `yueChang` | 曰唱 | cn | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/fengCodec.js) |
| `radixAll` | 一键多进制转换 | radix | migrated | `radix_all` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/radixAll.js) |
| `progCalc` | 程序员计算器 | radix | migrated | `programmer_calc` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/progCalc.js) |
| `unitConv` | 单位换算 | data | migrated | `unit_convert` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/unitConv.js) |
| `gifTiming` | GIF 帧时序隐写 | image | existing-review | `gif_timing` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/gifTiming.js) |
| `jpgSizeRecover` | JPEG 宽高修复 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/jpgSizeRecover.js) |
| `zipRepair` | ZIP 伪加密修复 | archive | existing-review | `zip_repair` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/zipRepair.js) |
| `zipPseudoEncrypt` | ZIP 伪加密（置位） | archive | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/zipRepair.js) |
| `adsTool` | NTFS ADS 备用数据流 | filefmt | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/adsTools.js) |
| `stringsExtract` | 字符串提取（strings） | filefmt | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/stringsExtract.js) |
| `jwtCrack` | JWT 密钥爆破（小字典演示） | modern | existing-review | `jwt_crack` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/jwtCrack.js) |
| `pdfObjects` | PDF 对象解析 | filefmt | existing-review | `pdf_objects` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/pdfObjects.js) |
| `ooxmlMeta` | OOXML 元数据提取 | filefmt | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/ooxmlMeta.js) |
| `apkManifest` | APK Manifest 解析 | filefmt | existing-review | `apk_manifest` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/apkManifest.js) |
| `elfInfo` | ELF 可执行信息 | filefmt | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/elfInfo.js) |
| `peInfo` | PE 可执行信息 | filefmt | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/peInfo.js) |
| `lsbEmbed` | LSB 嵌入（出题） | stegoFile | existing-review | `lsb_embed` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/lsbEmbed.js) |
| `zipCreate` | ZIP 创建（出题） | archive | existing-review | `zip_create` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/zipCreate.js) |
| `deepsoundExtract` | DeepSound 提取 | stegoFile | existing-review | `deepsound_extract` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/deepsoundExtract.js) |
| `beaufortVariant` | 变体 Beaufort | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/beaufortVariant.js) |
| `redefence` | Redefence 栅栏 | classic | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/redefence.js) |
| `objectIdTime` | ObjectID 时间戳解析 | radix | migrated | `object_id_time` | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/objectIdTime.js) |
| `rc4Drop` | RC4-drop / CipherSaber-2 | stream | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/rc4Drop.js) |
| `xsalsa20` | XSalsa20 | stream | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/xsalsa20.js) |
| `imageCompress` | 图片压缩 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/imageToolbox.js) |
| `imageGray` | 图片黑白化 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/imageToolbox.js) |
| `imagePixelate` | 图片像素化 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/imageToolbox.js) |
| `imageFrosted` | 图片毛玻璃 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/imageToolbox.js) |
| `imageSketch` | 图片转素描 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/imageToolbox.js) |
| `imageToText` | 图片文字化（字符画） | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/imageToolbox.js) |
| `gifCompose` | GIF 合成 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/imageToolbox.js) |
| `spriteSheet` | 精灵图互转 | image | pending | — | [源码](https://github.com/Henglie/EBCTFCodeBox/blob/ea2bc36dae31c97060f7f2cddad8df5518afad7b/src/core/imageToolbox.js) |
