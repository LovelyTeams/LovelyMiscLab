//! Base32/Base64 padding-bit steganography, deterministic upstream carriers.
use super::{
    basex::decoded,
    codec_support::{bit_encode, document_text, help},
    numeric_support::{bad, space},
    prelude::*,
};
use base64::Engine as _;
use serde_json::{json, Value};
const B32: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
const B64: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
fn carrier(data: &[u8], base32: bool) -> String {
    if base32 {
        let mut s = bit_encode(data, &B32.chars().collect::<Vec<_>>(), 5);
        while !s.len().is_multiple_of(8) {
            s.push('=');
        }
        s
    } else {
        base64::engine::general_purpose::STANDARD.encode(data)
    }
}
fn encode(data: &[u8], base32: bool, ctx: &NodeCtx) -> Result<String, CoreError> {
    let alphabet = if base32 { B32 } else { B64 }.as_bytes();
    let mut bitpos = 0;
    let mut index = 0usize;
    let mut rows = Vec::new();
    while bitpos < data.len() * 8 {
        if index.is_multiple_of(4096) {
            ctx.progress(bitpos as f32 / (data.len() * 8).max(1) as f32);
            ctx.cancel.check()?;
        }
        let filler = if base32 {
            vec![(115 + index) as u8, (116 + index) as u8]
        } else if index.is_multiple_of(2) {
            format!("s{index}").bytes().take(2).collect()
        } else {
            vec![b's']
        };
        let mut row = carrier(&filler, base32).into_bytes();
        let pad = row.iter().rev().take_while(|b| **b == b'=').count();
        let width = if base32 { 4 } else { pad * 2 };
        let mut offset = 0usize;
        for _ in 0..width {
            offset <<= 1;
            if bitpos < data.len() * 8 {
                offset |= ((data[bitpos / 8] >> (7 - bitpos % 8)) & 1) as usize;
                bitpos += 1;
            }
        }
        let pos = row.len() - pad - 1;
        let value = alphabet.iter().position(|b| *b == row[pos]).unwrap();
        row[pos] = alphabet[value + offset];
        rows.push(String::from_utf8(row).unwrap());
        index += 1;
    }
    ctx.cancel.check()?;
    Ok(rows.join("\n"))
}
fn decode(text: &str, base32: bool, strict: bool, ctx: &NodeCtx) -> Result<PortMap, CoreError> {
    let alphabet = if base32 { B32 } else { B64 }.as_bytes();
    let mut bits = Vec::new();
    let mut rows = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if index.is_multiple_of(4096) {
            ctx.progress((index * 4) as f32 / text.len().max(1) as f32);
            ctx.cancel.check()?;
        }
        let mut s: String = line.chars().filter(|c| !space(*c)).collect();
        if base32 {
            s = s.to_ascii_uppercase();
        }
        if s.is_empty() {
            continue;
        }
        let pad = s.bytes().filter(|b| *b == b'=').count();
        let width = if base32 {
            match pad {
                6 => 2,
                4 => 4,
                3 => 1,
                1 => 3,
                _ => 0,
            }
        } else {
            match pad {
                1 => 2,
                2 => 4,
                _ => 0,
            }
        };
        if width == 0 {
            continue;
        }
        let data = s.trim_end_matches('=');
        if strict
            && (pad != s.len() - data.len()
                || !s.is_ascii()
                || !s.len().is_multiple_of(if base32 { 8 } else { 4 }))
        {
            return Err(bad("隐写载体分组或 padding 无效"));
        }
        let decoded = if base32 {
            let (mut acc, mut size) = (0u32, 0usize);
            let mut out = Vec::new();
            for c in data.bytes() {
                if let Some(n) = alphabet.iter().position(|b| *b == c) {
                    acc = (acc << 5) | n as u32;
                    size += 5;
                    if size >= 8 {
                        size -= 8;
                        out.push(((acc >> size) & 255) as u8);
                    }
                    acc &= (1 << size) - 1;
                } else if strict {
                    return Err(bad("Base32 载体包含非法字符"));
                }
            }
            out
        } else {
            if s.bytes().any(|b| !alphabet.contains(&b) && b != b'=') {
                return Err(bad("Base64 载体包含非法字符"));
            }
            super::transport_codecs::b64decode(&s)?
        };
        let canonical = carrier(&decoded, base32);
        let given = data
            .as_bytes()
            .last()
            .and_then(|b| alphabet.iter().position(|c| c == b))
            .map(|n| n as i32)
            .unwrap_or(-1);
        let expected = canonical
            .trim_end_matches('=')
            .as_bytes()
            .last()
            .and_then(|b| alphabet.iter().position(|c| c == b))
            .map(|n| n as i32)
            .unwrap_or(-1);
        let offset = (given - expected).unsigned_abs();
        if strict
            && (given < 0
                || expected < 0
                || offset >= 1 << width
                || canonical.len() != s.len()
                || canonical.bytes().filter(|b| *b == b'=').count() != pad)
        {
            return Err(bad("隐写载体尾组不规范"));
        }
        let bitstr = format!("{offset:0width$b}");
        bits.extend(bitstr.bytes().map(|b| b - b'0'));
        rows.push(json!({"line":index+1,"padding":pad,"bits":bitstr,"offset":offset,"carrierHex":hex::encode(decoded)}));
    }
    let bytes = bits
        .chunks_exact(8)
        .map(|g| g.iter().fold(0u8, |n, b| (n << 1) | b))
        .collect();
    let mut out = decoded(bytes);
    out.insert("bitCount".into(), PortValue::Number(bits.len() as f64));
    out.insert(
        "remainingBits".into(),
        PortValue::Text(
            bits[bits.len() / 8 * 8..]
                .iter()
                .map(|b| (b'0' + b) as char)
                .collect(),
        ),
    );
    out.insert("rows".into(), PortValue::Json(json!(rows)));
    ctx.cancel.check()?;
    Ok(out)
}
struct Codec {
    base32: bool,
    decode: bool,
}
impl Node for Codec {
    fn run(&self, i: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        if self.decode {
            let s = in_text(i, "text")?;
            if s.len() > 16_777_216 {
                return Err(bad("隐写载体最大 16 MiB"));
            }
            decode(s, self.base32, pbool(p, "strict", true), ctx)
        } else {
            let data = in_bytes(i, "data")?;
            if data.len() > 524288 {
                return Err(bad("待隐藏消息最大 512 KiB"));
            }
            Ok(out_text(encode(&data, self.base32, ctx)?))
        }
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for base32 in [false, true] {
        for dec in [false, true] {
            let stem = if base32 {
                "base32_padding_stego"
            } else {
                "base64_padding_stego"
            };
            let upstream = if base32 { "base32steg" } else { "base64steg" };
            let title = if base32 {
                "Base32 补位隐写"
            } else {
                "Base64 补位隐写"
            };
            let params = if dec {
                vec![help(
                    ParamSpec::toggle("strict", "严格载体校验", true),
                    "校验载体分组、字符与 padding；关闭兼容上游宽松 Base32 字符过滤。",
                )]
            } else {
                vec![]
            };
            let outputs = if dec {
                vec![
                    t_out(),
                    req("bytes", "隐藏字节", PortType::Bytes),
                    req("bitCount", "提取位数", PortType::Number),
                    req("remainingBits", "未满字节尾位", PortType::Text),
                    req("rows", "逐行明细", PortType::Json),
                ]
            } else {
                vec![t_out()]
            };
            let mut d=document_text(desc(&format!("{stem}_{}",if dec{"decode"}else{"encode"}),"stego.text",&format!("{title} {}",if dec{"提取"}else{"嵌入"}),PURPLE,if dec{vec![t_in()]}else{vec![req("data","待隐藏消息",PortType::Any)]},outputs,params),upstream,"使用固定可复现载体逐行藏入补位比特；提取报告提供隐藏字节、逐行位值和未满字节尾位，原有 base64_stego 保持兼容。",if dec{if base32{"ON2E====\nOR2R===="}else{"czB=\ncw==\nczJ="}}else{"A"},json!({}));
            if dec {
                d.presentation
                    .previews
                    .insert("rows".into(), "table".into());
            }
            d.cost = Cost::Heavy;
            reg.register(
                d,
                Arc::new(move || {
                    Arc::new(Codec {
                        base32,
                        decode: dec,
                    })
                }),
            );
        }
    }
}
