//! Byte-preserving BubbleBabble and yEnc, with explicit direction and limits.
use super::{
    codec_support::{document_text, help},
    numeric_support::{bad, number, space},
    prelude::*,
};
use serde_json::{json, Value};
const V: &[u8] = b"aeiouy";
const C: &[u8] = b"bcdfghklmnprstvzx";
fn progress(ctx: &mut NodeCtx, i: usize, total: usize) -> Result<(), CoreError> {
    if i.is_multiple_of(1024) {
        ctx.progress(i as f32 / total.max(1) as f32);
        ctx.cancel.check()?;
    }
    Ok(())
}
fn bubble_enc(b: &[u8], ctx: &mut NodeCtx) -> Result<String, CoreError> {
    let mut out = String::from("x");
    let mut seed = 1usize;
    for i in (0..=b.len()).step_by(2) {
        progress(ctx, i, b.len())?;
        if i == b.len() {
            out.push(V[seed % 6] as char);
            out.push('x');
            out.push(V[seed / 6] as char);
            break;
        }
        let first = b[i] as usize;
        out.push(V[((first >> 6 & 3) + seed) % 6] as char);
        out.push(C[first >> 2 & 15] as char);
        out.push(V[((first & 3) + seed / 6) % 6] as char);
        if i + 1 == b.len() {
            break;
        }
        let second = b[i + 1] as usize;
        out.push(C[second >> 4] as char);
        out.push('-');
        out.push(C[second & 15] as char);
        seed = (seed * 5 + first * 7 + second) % 36;
    }
    out.push('x');
    Ok(out)
}
fn index(table: &[u8], b: u8) -> Result<usize, CoreError> {
    table
        .iter()
        .position(|&c| c == b)
        .ok_or_else(|| bad("BubbleBabble 包含非法字符"))
}
fn bubble_dec(s: &str, ctx: &mut NodeCtx) -> Result<Vec<u8>, CoreError> {
    let s = s.trim_matches(space).as_bytes();
    if s.is_empty() {
        return Ok(Vec::new());
    }
    if s.len() % 6 != 5 || s.first() != Some(&b'x') || s.last() != Some(&b'x') {
        return Err(bad("BubbleBabble 必须以 x 包裹，长度为6n+5"));
    }
    let mut out = Vec::new();
    let mut seed = 1;
    let inner = &s[1..s.len() - 1];
    for (i, t) in inner.chunks(6).enumerate() {
        progress(ctx, i, inner.len().div_ceil(6))?;
        let a = index(V, t[0])?;
        let mid = index(C, t[1])?;
        let b = index(V, t[2])?;
        if t.len() == 3 && mid == 16 {
            if a != seed % 6 || b != seed / 6 {
                return Err(bad("BubbleBabble 校验元组不匹配"));
            }
            break;
        }
        let hi = (a + 6 - seed % 6) % 6;
        let lo = (b + 6 - seed / 6) % 6;
        if mid >= 16 || hi >= 4 || lo >= 4 {
            return Err(bad("BubbleBabble 数据位超出字节范围"));
        }
        let first = (hi << 6) | (mid << 2) | lo;
        out.push(first as u8);
        if t.len() == 6 {
            let a = index(C, t[3])?;
            let b = index(C, t[5])?;
            if t[4] != b'-' || a >= 16 || b >= 16 {
                return Err(bad("BubbleBabble 第二字节或分隔符无效"));
            }
            let second = a << 4 | b;
            out.push(second as u8);
            seed = (seed * 5 + first * 7 + second) % 36;
        }
    }
    Ok(out)
}
fn yenc_enc(b: &[u8], width: usize, ctx: &mut NodeCtx) -> Result<String, CoreError> {
    let (mut out, mut col) = (String::new(), 0);
    for (i, &byte) in b.iter().enumerate() {
        progress(ctx, i, b.len())?;
        let e = byte.wrapping_add(42);
        if [0, 10, 13, 61].contains(&e) || col == 0 && [9, 32, 46].contains(&e) {
            out.push('=');
            out.push(e.wrapping_add(64) as char);
            col += 2;
        } else {
            out.push(e as char);
            col += 1;
        }
        if col >= width {
            out.push_str("\r\n");
            col = 0;
        }
    }
    Ok(out)
}
fn yenc_dec(s: &str, strict: bool, ctx: &mut NodeCtx) -> Result<Vec<u8>, CoreError> {
    let mut out = Vec::new();
    for (i, line) in s.split_inclusive('\n').enumerate() {
        progress(ctx, i, s.len())?;
        let line = if let Some(body) = line.strip_suffix('\n') {
            body.strip_suffix('\r').unwrap_or(body)
        } else {
            line
        };
        if line.starts_with("=y") {
            continue;
        }
        if strict && line.chars().any(|c| c as u32 > 255) {
            return Err(bad("yEnc 密文须为每字符一个字节的 Latin1 表示"));
        }
        let units: Vec<u16> = line.encode_utf16().collect();
        let mut n = 0;
        while n < units.len() {
            if n.is_multiple_of(1024) {
                ctx.cancel.check()?;
            }
            let c = units[n] as u8;
            if c == b'=' {
                n += 1;
                let c = *units.get(n).ok_or_else(|| bad("yEnc 行尾孤立 = 转义"))? as u8;
                out.push(c.wrapping_sub(64).wrapping_sub(42));
            } else {
                out.push(c.wrapping_sub(42));
            }
            n += 1;
        }
    }
    Ok(out)
}
struct Codec(bool, bool);
impl Node for Codec {
    fn run(&self, i: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        if !self.1 {
            let data = in_bytes(i, "data")?;
            if data.len() > 1_048_576 {
                return Err(bad("字节输入最多1MiB"));
            }
            let s = if self.0 {
                bubble_enc(&data, ctx)?
            } else {
                yenc_enc(&data, number(p, "width", 128, 1, 4096)? as usize, ctx)?
            };
            ctx.progress(1.0);
            ctx.cancel.check()?;
            return Ok(out_text(s));
        }
        let s = in_text(i, "text")?;
        if s.len() > 6_291_456 {
            return Err(bad("编码文本最多6MiB"));
        }
        let bytes = if self.0 {
            bubble_dec(s, ctx)?
        } else {
            yenc_dec(s, pbool(p, "strict", true), ctx)?
        };
        if bytes.len() > 1_048_576 {
            return Err(bad("解码结果最多1MiB"));
        }
        let text = if self.0 {
            String::from_utf8_lossy(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes))
                .into_owned()
        } else {
            String::from_utf8(bytes.clone()).unwrap_or_else(|_| hex::encode(&bytes))
        };
        ctx.progress(1.0);
        ctx.cancel.check()?;
        let mut out = out_text(text);
        out.insert("bytes".into(), PortValue::Bytes(bytes.into()));
        Ok(out)
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for bubble in [true, false] {
        for decode in [false, true] {
            let (id, name, sample) = if bubble {
                (
                    "bubblebabble",
                    "BubbleBabble",
                    if decode {
                        "xigak-nyryk-humil-bosek-sonax"
                    } else {
                        "Pineapple"
                    },
                )
            } else {
                (
                    "yenc",
                    "yEnc",
                    if decode {
                        "r\u{8f}\u{96}\u{96}\u{99}"
                    } else {
                        "Hello"
                    },
                )
            };
            let params = if bubble {
                vec![]
            } else if decode {
                vec![help(ParamSpec::toggle("strict","拒绝非字节字符",true),"开启时拒绝超过U+00FF的字符；关闭时兼容上游UTF-16低字节截断。控制行只跳过，不校验长度或CRC。")]
            } else {
                vec![help(
                    ParamSpec::number("width", "行宽", 1.0, 4096.0, 1.0, 128.0),
                    "达到行宽后输出CRLF，转义对保持完整，必要时超出一列。",
                )]
            };
            let mut d = document_text(
                desc(
                    &format!("{id}_{}", if decode { "decode" } else { "encode" }),
                    "encoding.transport",
                    &format!("{name} {}", if decode { "解码" } else { "编码" }),
                    BLUE,
                    if decode {
                        vec![t_in()]
                    } else {
                        vec![req("data", "文本或原始字节", PortType::Any)]
                    },
                    if decode {
                        vec![t_out(), req("bytes", "完整字节", PortType::Bytes)]
                    } else {
                        vec![t_out()]
                    },
                    params,
                ),
                id,
                if bubble {
                    "可读字节指纹编码，校验元组和分隔符严格验证；文本预览按上游剥离UTF-8 BOM，完整bytes保留全部数据。"
                } else {
                    "yEnc 裸数据体：每字节+42并转义关键值，支持控制行跳过；不处理分块重组或头部CRC。解码的非UTF-8预览为Hex，bytes可连接下游。"
                },
                sample,
                json!({}),
            );
            d.cost = Cost::Heavy;
            reg.register(d, Arc::new(move || Arc::new(Codec(bubble, decode))));
        }
    }
}
