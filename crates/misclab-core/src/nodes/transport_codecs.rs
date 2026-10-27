//! Byte-preserving codecs reimplemented from pinned EBCTFCodeBox semantics.
use super::{basex::decoded, codec_support::alphabet, prelude::*};
use crate::node::descriptor::NodeExample;
use base64::Engine as _;
use serde_json::{json, Value};
use std::io::Write;

const Z85: &str =
    "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-:+=^!/*?&<>()[]{}@%$#";
const IPV6: &str =
    "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!#$%&()*+-;<=>?@^_\x60{|}~";
pub(super) const B64: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
pub(super) fn err(s: impl Into<String>) -> CoreError {
    CoreError::Parse(s.into())
}
pub(super) fn b64decode(text: &str) -> Result<Vec<u8>, CoreError> {
    let s: String = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            '-' => '+',
            '_' => '/',
            _ => c,
        })
        .collect();
    let config = base64::engine::GeneralPurposeConfig::new()
        .with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent)
        .with_decode_allow_trailing_bits(true);
    base64::engine::GeneralPurpose::new(&base64::alphabet::STANDARD, config)
        .decode(s)
        .map_err(|e| err(e.to_string()))
}
pub(super) fn hex_input(text: &str) -> Result<Vec<u8>, CoreError> {
    let s: String = text.trim().chars().filter(|c| !c.is_whitespace()).collect();
    hex::decode(
        s.strip_prefix("0x")
            .or_else(|| s.strip_prefix("0X"))
            .unwrap_or(&s),
    )
    .map_err(|e| err(e.to_string()))
}
fn utf8(bytes: &[u8]) -> Result<&str, CoreError> {
    std::str::from_utf8(bytes).map_err(|e| err(e.to_string()))
}
pub(super) fn help(mut p: ParamSpec, description: &str) -> ParamSpec {
    p.description = description.into();
    p
}

#[derive(Clone, Copy)]
enum Codec {
    Z85,
    Ipv6,
    Dict64,
    Multiline64,
    Zlib64,
    Netbios,
    Caret,
    Basic,
    BitReverse,
    BitRotate,
    ByteSwap,
    ByteReverse,
}
struct CodecNode {
    codec: Codec,
    decode: bool,
}
fn encode85(data: &[u8], dict: &[char]) -> String {
    let mut out = String::new();
    for chunk in data.chunks(4) {
        let mut n = 0u32;
        for i in 0..4 {
            n = (n << 8) | u32::from(*chunk.get(i).unwrap_or(&0));
        }
        let mut indices = [0usize; 5];
        for v in indices.iter_mut().rev() {
            *v = (n % 85) as usize;
            n /= 85;
        }
        out.extend(indices[..chunk.len() + 1].iter().map(|&i| dict[i]));
    }
    out
}
fn decode85(text: &str, dict: &[char]) -> Result<Vec<u8>, CoreError> {
    let chars: Vec<_> = text.chars().filter(|c| !c.is_whitespace()).collect();
    if chars.len() % 5 == 1 {
        return Err(err("Base85 末组不能只有 1 个字符"));
    }
    let mut out = Vec::new();
    for chunk in chars.chunks(5) {
        let mut n = 0u64;
        for i in 0..5 {
            let v = match chunk.get(i) {
                Some(c) => dict
                    .iter()
                    .position(|x| x == c)
                    .ok_or_else(|| err("Base85 码表外字符"))?,
                None => 84,
            };
            n = n * 85 + v as u64;
        }
        let n = u32::try_from(n).map_err(|_| err("Base85 分组超过 32 位"))?;
        out.extend_from_slice(&n.to_be_bytes()[..chunk.len() - 1]);
    }
    Ok(out)
}
fn encode_caret(data: &[u8]) -> String {
    let mut out = String::new();
    for &b in data {
        let lo = b & 127;
        if b >= 128 {
            out.push_str("M-");
        }
        match lo {
            0..=31 => {
                out.push('^');
                out.push((lo + 64) as char);
            }
            127 => out.push_str("^?"),
            _ => out.push(lo as char),
        }
    }
    out
}
fn decode_caret(text: &str) -> Result<Vec<u8>, CoreError> {
    let chars: Vec<_> = text.encode_utf16().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let meta = chars[i] == b'M' as u16 && chars.get(i + 1) == Some(&(b'-' as u16));
        if meta {
            i += 2;
        }
        let c = *chars.get(i).ok_or_else(|| err("M- 后缺字符"))?;
        if c == b'^' as u16 {
            let next = *chars.get(i + 1).ok_or_else(|| err("^ 后缺字符"))?;
            let valid = next <= 127
                && ((next as u8).is_ascii_alphabetic() || b"@?[\\]^_".contains(&(next as u8)));
            if meta || valid {
                out.push(
                    (if next == b'?' as u16 {
                        127
                    } else {
                        (next & 31) as u8
                    }) | if meta { 128 } else { 0 },
                );
                i += 2;
            } else {
                out.push(b'^');
                i += 1;
            }
        } else {
            out.push(if meta { (c as u8 & 127) | 128 } else { c as u8 });
            i += 1;
        }
    }
    Ok(out)
}
fn transform(
    kind: Codec,
    mut bytes: Vec<u8>,
    p: &Value,
    decode: bool,
) -> Result<Vec<u8>, CoreError> {
    match kind {
        Codec::BitReverse => bytes.iter_mut().for_each(|b| *b = b.reverse_bits()),
        Codec::BitRotate => {
            let bits = pnum(p, "bits", 1.0);
            if !(0.0..=7.0).contains(&bits) || bits.fract() != 0.0 {
                return Err(err("位数应为 0..7 的整数"));
            }
            let direction = pstr(p, "dir", "left");
            if !["left", "right"].contains(&direction) {
                return Err(err("方向应为 left/right"));
            }
            let left = (direction == "left") != decode;
            bytes.iter_mut().for_each(|b| {
                *b = if left {
                    b.rotate_left(bits as u32)
                } else {
                    b.rotate_right(bits as u32)
                }
            });
        }
        Codec::ByteSwap => {
            let group = p
                .get("group")
                .and_then(|v| {
                    v.as_str()
                        .and_then(|s| s.parse::<usize>().ok())
                        .or_else(|| v.as_u64().map(|n| n as usize))
                })
                .unwrap_or(2);
            if ![2, 4, 8].contains(&group) {
                return Err(err("分组字节数应为 2、4 或 8"));
            }
            if pstr(p, "mode", "text") == "hex" && !bytes.len().is_multiple_of(group) {
                return Err(err("Hex 模式字节长度必须是整组倍数"));
            }
            bytes.chunks_mut(group).for_each(|c| c.reverse());
        }
        Codec::ByteReverse => bytes.reverse(),
        _ => unreachable!(),
    }
    Ok(bytes)
}
impl CodecNode {
    fn encode(&self, data: &[u8], p: &Value, ctx: &mut NodeCtx) -> Result<String, CoreError> {
        match self.codec {
            Codec::Z85 => Ok(encode85(data, &alphabet(pstr(p, "dict", Z85), Some(85))?)),
            Codec::Ipv6 => {
                if data.len() != 16 {
                    return Err(err("RFC 1924 输入必须为 16 字节"));
                }
                let dict = alphabet(pstr(p, "dict", IPV6), Some(85))?;
                let mut n = u128::from_be_bytes(data.try_into().unwrap());
                if n == 0 {
                    return Ok(dict[0].to_string());
                }
                let mut chars = Vec::new();
                while n > 0 {
                    chars.push(dict[(n % 85) as usize]);
                    n /= 85;
                }
                Ok(chars.into_iter().rev().collect())
            }
            Codec::Dict64 => {
                let dict = alphabet(pstr(p, "key", B64), Some(64))?;
                if dict.contains(&'=') {
                    return Err(err("字典不能包含填充符 ="));
                }
                Ok(base64::engine::general_purpose::STANDARD
                    .encode(data)
                    .chars()
                    .map(|c| {
                        if c == '=' {
                            c
                        } else {
                            dict[B64.find(c).unwrap()]
                        }
                    })
                    .collect())
            }
            Codec::Multiline64 => {
                let len = pnum(p, "lineLen", 76.0);
                if !len.is_finite() || !(1.0..=1_000_000.0).contains(&len) {
                    return Err(err("行宽应为 1..1000000"));
                }
                let len = ((len as usize / 4) * 4).max(4);
                Ok(base64::engine::general_purpose::STANDARD
                    .encode(data)
                    .as_bytes()
                    .chunks(len)
                    .map(|c| std::str::from_utf8(c).unwrap())
                    .collect::<Vec<_>>()
                    .join("\n"))
            }
            Codec::Zlib64 => {
                let mut encoder =
                    flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                for (i, chunk) in data.chunks(16384).enumerate() {
                    ctx.check_cancel()?;
                    encoder.write_all(chunk)?;
                    ctx.progress((i * 16384) as f32 / data.len().max(1) as f32);
                }
                Ok(base64::engine::general_purpose::STANDARD.encode(encoder.finish()?))
            }
            Codec::Netbios => Ok(data
                .iter()
                .flat_map(|b| [(b'A' + (b >> 4)) as char, (b'A' + (b & 15)) as char])
                .collect()),
            Codec::Caret => Ok(encode_caret(data)),
            Codec::Basic => Ok(format!(
                "Basic {}",
                base64::engine::general_purpose::STANDARD.encode(data)
            )),
            Codec::BitReverse | Codec::BitRotate | Codec::ByteSwap | Codec::ByteReverse => {
                let bytes = if pstr(p, "mode", "text") == "hex" {
                    hex_input(utf8(data)?)?
                } else {
                    data.to_vec()
                };
                Ok(hex::encode_upper(transform(self.codec, bytes, p, false)?))
            }
        }
    }
    fn decode(&self, text: &str, p: &Value, ctx: &mut NodeCtx) -> Result<Vec<u8>, CoreError> {
        match self.codec {
            Codec::Z85 => decode85(text, &alphabet(pstr(p, "dict", Z85), Some(85))?),
            Codec::Ipv6 => {
                let dict = alphabet(pstr(p, "dict", IPV6), Some(85))?;
                let text = text.trim();
                if text.is_empty() {
                    return Err(err("RFC 1924 输入不能为空"));
                }
                if text.chars().count() > 25 {
                    if !text.chars().count().is_multiple_of(5) {
                        return Err(err("历史分组编码长度须为 5 的倍数"));
                    }
                    return decode85(text, &dict);
                }
                let mut n = 0u128;
                for c in text.chars() {
                    let d = dict
                        .iter()
                        .position(|&x| x == c)
                        .ok_or_else(|| err("RFC 1924 码表外字符"))?;
                    n = n
                        .checked_mul(85)
                        .and_then(|n| n.checked_add(d as u128))
                        .ok_or_else(|| err("RFC 1924 数值超过 128 位"))?;
                }
                Ok(n.to_be_bytes().to_vec())
            }
            Codec::Dict64 => {
                let dict = alphabet(pstr(p, "key", B64), Some(64))?;
                if dict.contains(&'=') {
                    return Err(err("字典不能包含填充符 ="));
                }
                let mut mapped = String::new();
                for c in text.chars() {
                    if c == '=' {
                        mapped.push(c);
                    } else if let Some(i) = dict.iter().position(|&d| d == c) {
                        mapped.push(B64.as_bytes()[i] as char);
                    } else if !pbool(p, "ignoreUnknown", true) && !c.is_whitespace() {
                        return Err(err("字典外字符"));
                    }
                }
                b64decode(&mapped)
            }
            Codec::Multiline64 => {
                let mut bytes = Vec::new();
                for (i, line) in text.lines().enumerate() {
                    ctx.check_cancel()?;
                    match b64decode(line) {
                        Ok(b) => bytes.extend(b),
                        Err(e) => {
                            if pbool(p, "skipInvalid", true) {
                                ctx.log(
                                    crate::progress::LogLevel::Warn,
                                    format!("跳过第 {} 行：{e}", i + 1),
                                );
                            } else {
                                return Err(e);
                            }
                        }
                    }
                }
                Ok(bytes)
            }
            Codec::Zlib64 => {
                let compressed = b64decode(text)?;
                let mut decoder = flate2::Decompress::new(true);
                let limit = pnum(p, "maxBytes", 16_777_216.0);
                if !(0.0..=134_217_728.0).contains(&limit) {
                    return Err(err("解压上限应为 0..128 MiB"));
                }
                let mut out = Vec::new();
                let mut buffer = [0u8; 16384];
                loop {
                    ctx.check_cancel()?;
                    let before_in = decoder.total_in();
                    let before_out = decoder.total_out();
                    let status = decoder
                        .decompress(
                            &compressed[before_in as usize..],
                            &mut buffer,
                            flate2::FlushDecompress::None,
                        )
                        .map_err(|e| err(e.to_string()))?;
                    let n = (decoder.total_out() - before_out) as usize;
                    if out.len() + n > limit as usize {
                        return Err(CoreError::BudgetExceeded("解压结果超过字节上限".into()));
                    }
                    out.extend_from_slice(&buffer[..n]);
                    ctx.progress(decoder.total_in() as f32 / compressed.len().max(1) as f32);
                    if status == flate2::Status::StreamEnd {
                        if decoder.total_in() != compressed.len() as u64 {
                            return Err(err("Zlib 流尾存在多余字节"));
                        }
                        break;
                    }
                    if n == 0 && decoder.total_in() == before_in {
                        return Err(err("Zlib 流被截断"));
                    }
                }
                Ok(out)
            }
            Codec::Netbios => {
                let chars: Vec<u8> = text
                    .bytes()
                    .filter(|b| !b.is_ascii_whitespace())
                    .map(|b| b.to_ascii_uppercase())
                    .collect();
                if !chars.len().is_multiple_of(2)
                    || chars.iter().any(|b| !(b'A'..=b'P').contains(b))
                {
                    return Err(err("NetBIOS 需要偶数个 A-P 字符"));
                }
                Ok(chars
                    .chunks(2)
                    .map(|c| ((c[0] - b'A') << 4) | (c[1] - b'A'))
                    .collect())
            }
            Codec::Caret => decode_caret(text),
            Codec::Basic => {
                let s = text.trim();
                let payload = s
                    .get(..5)
                    .filter(|prefix| prefix.eq_ignore_ascii_case("basic"))
                    .and_then(|_| s.get(5..))
                    .filter(|rest| rest.starts_with(char::is_whitespace))
                    .map(str::trim_start)
                    .unwrap_or(s);
                b64decode(payload)
            }
            Codec::BitReverse | Codec::BitRotate | Codec::ByteSwap | Codec::ByteReverse => {
                transform(self.codec, hex_input(text)?, p, true)
            }
        }
    }
}
impl Node for CodecNode {
    fn run(&self, inputs: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.check_cancel()?;
        if self.decode {
            let data = self.decode(in_text(inputs, "text")?, p, ctx)?;
            let text = if matches!(self.codec, Codec::Caret) {
                Some(data.iter().map(|&b| b as char).collect())
            } else if matches!(self.codec, Codec::ByteSwap | Codec::ByteReverse)
                && pstr(p, "mode", "text") == "hex"
            {
                Some(hex::encode_upper(&data))
            } else {
                None
            };
            let mut out = decoded(data);
            if let Some(text) = text {
                out.insert("text".into(), PortValue::Text(text));
            }
            Ok(out)
        } else {
            let data = if matches!(self.codec, Codec::Caret) {
                match inputs.get("data") {
                    Some(PortValue::Text(t)) => t
                        .chars()
                        .map(|c| {
                            let mut buf = [0u16; 2];
                            c.encode_utf16(&mut buf)[0] as u8
                        })
                        .collect(),
                    _ => in_bytes(inputs, "data")?,
                }
            } else {
                in_bytes(inputs, "data")?
            };
            Ok(out_text(self.encode(&data, p, ctx)?))
        }
    }
}
pub fn register(reg: &mut NodeRegistry) {
    let definitions = [
        (Codec::Z85, "z85", "z85", "Z85", "encoding.base", "4 字节转 5 个字符；兼容末尾短组。码表须含 85 个唯一字符。"),
        (Codec::Ipv6, "base85ipv6", "base85ipv6", "Base85 IPv6", "encoding.base", "RFC 1924：16 字节作为一个 128 位整数编码；解码兼容大于 25 字符的历史分组格式。"),
        (Codec::Dict64, "base64_dict", "base64dict", "字典 Base64", "encoding.base", "使用 64 字符唯一字典替换标准 Base64 字母表，填充字符仍为 =。"),
        (Codec::Multiline64, "multiline_base64", "multilineBase64", "多行 Base64", "encoding.base", "编码按 4 字符组对齐行宽；解码逐行还原再拼接完整字节，支持跳过错误行。"),
        (Codec::Zlib64, "base64_zlib", "base64decompress", "Base64 + Zlib", "archive.compression", "Zlib 压缩后编码 Base64，或 Base64 解码后解压。分块处理，支持取消和解压大小限制。"),
        (Codec::Netbios, "netbios", "netbios", "NetBIOS", "encoding.transport", "每字节高低半字节映射到 A-P；解码支持小写和空白，保持完整原始字节。"),
        (Codec::Caret, "caret_m", "caretMdecode", "Caret/M 控制字符", "encoding.transport", "控制字节转 ^X，高位字节转 M-X。文本按 Latin-1 低字节解释。字面 ^ 与 M- 存在上游记法歧义，原始二进制请优先使用其他无歧义编码。"),
        (Codec::Basic, "http_basic_auth", "httpBasicAuth", "HTTP Basic 认证", "encoding.transport", "原始 user:pass 编码为 Basic 前缀的 Base64；支持带前缀或裸 Base64 解码。"),
        (Codec::BitReverse, "bit_reverse", "bitReverse", "字节内位反转", "numeric.bits", "把每字节 bit 0..7 镜像反转；编码输出 Hex，解码输出完整字节及文本预览。"),
        (Codec::BitRotate, "bit_rotate", "bitRotate", "字节内循环移位", "numeric.bits", "按方向和位数循环移位，解码采用反向移位。文本或字节 → Hex。"),
        (Codec::ByteSwap, "byte_swap", "byteSwap", "分组字节序反转", "numeric.bits", "每 2/4/8 字节组反转。文本模式允许末尾短组；Hex 模式要求整组长度。"),
        (Codec::ByteReverse, "byte_reverse", "byteReverse", "整串字节倒序", "numeric.bits", "反转整个字节流，支持文本/字节和 Hex 模式；完整二进制从 bytes 输出。"),
    ];
    for (codec, id, upstream, title, category, description) in definitions {
        let params = match codec {
            Codec::Z85 | Codec::Ipv6 => vec![help(
                ParamSpec::text(
                    "dict",
                    "码表（85 字符）",
                    if matches!(codec, Codec::Z85) {
                        Z85
                    } else {
                        IPV6
                    },
                    false,
                ),
                "字符必须唯一；对编码和解码均生效。",
            )],
            Codec::Dict64 => vec![
                help(
                    ParamSpec::text("key", "字典（64 字符）", B64, false),
                    "64 个唯一字符，不含填充符 =。",
                ),
                help(
                    ParamSpec::toggle("ignoreUnknown", "忽略字典外字符", true),
                    "兼容上游的解码模式；关闭可严格校验输入。",
                ),
            ],
            Codec::Multiline64 => vec![
                help(
                    ParamSpec::number("lineLen", "每行字符数", 1.0, 1_000_000.0, 1.0, 76.0),
                    "向下对齐到 4 的倍数，最小 4。",
                ),
                help(
                    ParamSpec::toggle("skipInvalid", "跳过错误行", true),
                    "兼容上游；跳过时输出警告日志，关闭则遇错停止。",
                ),
            ],
            Codec::Zlib64 => vec![help(
                ParamSpec::number(
                    "maxBytes",
                    "最大解压字节数",
                    0.0,
                    134_217_728.0,
                    1.0,
                    16_777_216.0,
                ),
                "只影响解码；超过上限立即停止。",
            )],
            Codec::BitRotate => vec![
                help(
                    ParamSpec::select("dir", "编码方向", &["left", "right"], "left"),
                    "解码自动取反方向。",
                ),
                help(
                    ParamSpec::number("bits", "位数", 0.0, 7.0, 1.0, 1.0),
                    "0 表示不移位，1..7 为字节内循环位移。",
                ),
            ],
            Codec::ByteSwap | Codec::ByteReverse => {
                let mut ps = vec![help(
                    ParamSpec::select("mode", "输入模式", &["text", "hex"], "text"),
                    "text：文本或原始字节 ↔ Hex；hex：Hex ↔ Hex。",
                )];
                if matches!(codec, Codec::ByteSwap) {
                    ps.push(help(
                        ParamSpec::select("group", "每组字节数", &["2", "4", "8"], "2"),
                        "Hex 模式必须整组；文本模式反转最后的短组。",
                    ));
                }
                ps
            }
            _ => vec![],
        };
        for decode in [false, true] {
            let mut d = desc(
                &format!("{id}_{}", if decode { "decode" } else { "encode" }),
                category,
                &format!("{title} {}", if decode { "解码" } else { "编码" }),
                BLUE,
                if decode {
                    vec![t_in()]
                } else {
                    vec![req("data", "输入", PortType::Any)]
                },
                if decode {
                    vec![t_out(), req("bytes", "完整字节", PortType::Bytes)]
                } else {
                    vec![t_out()]
                },
                params
                    .iter()
                    .filter(|p| match p.name.as_str() {
                        "lineLen" => !decode,
                        "ignoreUnknown" | "skipInvalid" | "maxBytes" => decode,
                        _ => true,
                    })
                    .cloned()
                    .collect(),
            );
            d.description = description.into();
            d.upstream_ids = vec![upstream.into()];
            d.family = upstream.into();
            d.aliases = vec![title.into(), upstream.into()];
            d.cost = if matches!(codec, Codec::Zlib64) {
                Cost::Heavy
            } else {
                Cost::Cheap
            };
            let sample = if matches!(codec, Codec::Ipv6) {
                b"0123456789abcdef".to_vec()
            } else {
                b"Hello, CTF!".to_vec()
            };
            let example = match codec {
                Codec::Z85 => "HelloWorld",
                Codec::Ipv6 => "4)+k&C#VzJ4br>0wv%Yp",
                Codec::Dict64 | Codec::Multiline64 => "SGVsbG8sIENTRiE=",
                Codec::Zlib64 => "eJzzSM3JyddRcA5xUwQAFacDPw==",
                Codec::Netbios => "EB",
                Codec::Caret => "^@^M^JM-^?",
                Codec::Basic => "Basic dXNlcjpwYXNz",
                _ => "41424344",
            }
            .to_string();
            d.examples.push(NodeExample {
                label: "入门样例".into(),
                inputs: if decode {
                    one("text", PortValue::Text(example))
                } else {
                    one("data", PortValue::Bytes(sample.into()))
                },
                params: json!({}),
            });
            reg.register(d, Arc::new(move || Arc::new(CodecNode { codec, decode })));
        }
    }
}
