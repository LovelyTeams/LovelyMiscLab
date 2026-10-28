//! BIP173 generic byte-payload codec. This does not infer SegWit witness versions.
use super::{
    codec_support::{document_text, help},
    numeric_support::bad,
    prelude::*,
};
use serde_json::{json, Value};
pub(super) const TABLE: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
pub(super) fn polymod(values: impl IntoIterator<Item = u8>) -> u32 {
    let mut chk = 1u32;
    for v in values {
        let top = chk >> 25;
        chk = ((chk & 0x1ff_ffff) << 5) ^ v as u32;
        for (i, g) in [
            0x3b6a_57b2,
            0x2650_8e6d,
            0x1ea1_19fa,
            0x3d42_33dd,
            0x2a14_62b3,
        ]
        .into_iter()
        .enumerate()
        {
            if top >> i & 1 != 0 {
                chk ^= g;
            }
        }
    }
    chk
}
pub(super) fn hrp_expanded(s: &str) -> Vec<u8> {
    s.bytes()
        .map(|b| b >> 5)
        .chain([0])
        .chain(s.bytes().map(|b| b & 31))
        .collect()
}
pub(super) fn bits(data: &[u8], from: u32, to: u32, pad: bool) -> Result<Vec<u8>, CoreError> {
    let (mut acc, mut n) = (0u32, 0u32);
    let mut out = Vec::new();
    let mask = (1 << to) - 1;
    for &b in data {
        if b as u32 >> from != 0 {
            return Err(bad("Bech32 数据超过位宽"));
        }
        acc = ((acc << from) | b as u32) & ((1 << (from + to - 1)) - 1);
        n += from;
        while n >= to {
            n -= to;
            out.push(((acc >> n) & mask) as u8);
        }
    }
    if pad {
        if n > 0 {
            out.push(((acc << (to - n)) & mask) as u8);
        }
    } else if n >= from || (acc << (to - n)) & mask != 0 {
        return Err(bad("Bech32 非零补位或多余尾组"));
    }
    Ok(out)
}
fn encode(hrp: &str, payload: &[u8]) -> Result<String, CoreError> {
    let hrp = hrp.to_ascii_lowercase();
    if hrp.is_empty() || hrp.len() > 83 || !hrp.bytes().all(|b| (33..=126).contains(&b)) {
        return Err(bad("HRP 须为1..83个可打印ASCII字符"));
    }
    let data = bits(payload, 8, 5, true)?;
    if hrp.len() + 7 + data.len() > 90 {
        return Err(bad("Bech32 总长度不得超过90字符"));
    }
    let checksum = polymod(
        hrp_expanded(&hrp)
            .into_iter()
            .chain(data.iter().copied())
            .chain([0; 6]),
    ) ^ 1;
    let mut out = format!("{hrp}1");
    out.extend(data.into_iter().map(|v| TABLE[v as usize] as char));
    out.extend((0..6).map(|i| TABLE[(checksum >> (5 * (5 - i)) & 31) as usize] as char));
    Ok(out)
}
fn decode(s: &str) -> Result<(String, Vec<u8>), CoreError> {
    if !(8..=90).contains(&s.len()) || !s.bytes().all(|b| (33..=126).contains(&b)) {
        return Err(bad("Bech32 须为8..90个可打印ASCII字符"));
    }
    if s.bytes().any(|b| b.is_ascii_lowercase()) && s.bytes().any(|b| b.is_ascii_uppercase()) {
        return Err(bad("Bech32 禁止混合大小写"));
    }
    let s = s.to_ascii_lowercase();
    let pos = s.rfind('1').ok_or_else(|| bad("Bech32 缺少分隔符1"))?;
    if pos == 0 || pos + 7 > s.len() {
        return Err(bad("Bech32 HRP或校验段过短"));
    }
    let hrp = &s[..pos];
    let data = s[pos + 1..]
        .bytes()
        .map(|b| {
            TABLE
                .iter()
                .position(|&c| c == b)
                .map(|v| v as u8)
                .ok_or_else(|| bad("Bech32 数据字符非法"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if polymod(hrp_expanded(hrp).into_iter().chain(data.iter().copied())) != 1 {
        return Err(bad("Bech32 校验和不匹配"));
    }
    Ok((hrp.into(), bits(&data[..data.len() - 6], 5, 8, false)?))
}
struct Bech(bool);
impl Node for Bech {
    fn run(&self, i: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        if self.0 {
            let (hrp, bytes) = decode(in_text(i, "text")?)?;
            let mut out = out_text(format!("hrp={hrp}\npayload={}", hex::encode(&bytes)));
            out.insert("hrp".into(), PortValue::Text(hrp));
            out.insert("bytes".into(), PortValue::Bytes(bytes.into()));
            ctx.progress(1.0);
            ctx.cancel.check()?;
            return Ok(out);
        }
        let bytes = match i.get("data") {
            Some(PortValue::Text(s)) => {
                if s.len() > 1024 {
                    return Err(bad("Bech32 Hex 输入过长"));
                }
                if pbool(p, "strip", false) {
                    hex::decode(
                        s.chars()
                            .filter(char::is_ascii_hexdigit)
                            .collect::<String>(),
                    )
                    .map_err(|_| bad("Hex 长度须为偶数"))?
                } else {
                    super::transport_codecs::hex_input(s)?
                }
            }
            _ => in_bytes(i, "data")?,
        };
        let result = encode(pstr(p, "hrp", "bc"), &bytes)?;
        ctx.progress(1.0);
        ctx.cancel.check()?;
        Ok(out_text(result))
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for decode in [false, true] {
        let params = if decode {
            vec![]
        } else {
            vec![
                help(
                    ParamSpec::text("hrp", "HRP前缀", "bc", false),
                    "1..83个可打印ASCII字符；编码统一小写，总长度最多90。",
                ),
                help(
                    ParamSpec::toggle("strip", "忽略非Hex字符", false),
                    "开启对应上游的宽松Hex过滤；直接bytes输入不受影响。",
                ),
            ]
        };
        let d=document_text(desc(if decode{"bech32_decode"}else{"bech32_encode"},"encoding.base",if decode{"Bech32 解码"}else{"Bech32 编码"},BLUE,if decode{vec![t_in()]}else{vec![req("data","Hex 或原始字节",PortType::Any)]},if decode{vec![t_out(),req("hrp","HRP前缀",PortType::Text),req("bytes","载荷字节",PortType::Bytes)]}else{vec![t_out()]},params),"bech32","BIP173通用字节载荷与BCH校验和；校验大小写、长度及补位。与上游一致，不隐式添加见证版本，不把通用载荷当成地址生成器。",if decode{"a12uel5l"}else{"00112233"},json!({}));
        reg.register(d, Arc::new(move || Arc::new(Bech(decode))));
    }
}
