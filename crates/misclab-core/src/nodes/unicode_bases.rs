//! Base69 and Unicode alphabets, transcribed from pinned upstream numeric tables.
use super::{
    basex,
    codec_support::{alphabet, document_text, help},
    numeric_support::{bad, number, space},
    prelude::*,
};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::OnceLock};
const B69: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/-*<>|";
struct Tables {
    b2048: Vec<char>,
    r2048: HashMap<char, usize>,
    b65: HashMap<i32, u32>,
    r65: HashMap<u32, i32>,
    eco: Vec<char>,
    reco: HashMap<char, usize>,
}
fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| {
        let v: Value = serde_json::from_str(include_str!("unicode_base_tables.json")).unwrap();
        let chars = |key: &str| {
            v[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| char::from_u32(x.as_u64().unwrap() as u32).unwrap())
                .collect::<Vec<_>>()
        };
        let b2048 = chars("B2048_11");
        let eco = chars("ECO_MAP");
        let b65: HashMap<i32, u32> = v["B65_START"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.parse().unwrap(), v.as_u64().unwrap() as u32))
            .collect();
        Tables {
            r2048: b2048
                .iter()
                .copied()
                .enumerate()
                .map(|(i, c)| (c, i))
                .collect(),
            reco: eco
                .iter()
                .copied()
                .enumerate()
                .map(|(i, c)| (c, i))
                .collect(),
            r65: b65.iter().map(|(k, v)| (*v, *k)).collect(),
            b2048,
            b65,
            eco,
        }
    })
}
const ECO_PAD: char = '\u{2615}';
const ECO_PAD4: [char; 4] = ['\u{269c}', '\u{1f3cd}', '\u{1f4d1}', '\u{1f64b}'];
#[derive(Clone, Copy)]
enum Kind {
    Radix,
    B69,
    B2048,
    B65,
    Eco,
}
struct Codec {
    kind: Kind,
    decode: bool,
}
fn tick(ctx: &NodeCtx, i: usize, len: usize) -> Result<(), CoreError> {
    if i.is_multiple_of(4096) {
        ctx.progress(i as f32 / len.max(1) as f32);
        ctx.cancel.check()?;
    }
    Ok(())
}
fn encode(kind: Kind, data: &[u8], p: &Value, ctx: &NodeCtx) -> Result<String, CoreError> {
    let t = tables();
    let mut out = String::new();
    match kind {
        Kind::Radix => {
            let radix = number(p, "radix", 36, 2, 95)? as usize;
            let default: String = (33..=126).map(|n| n as u8 as char).collect();
            let dict = pstr(p, "dict", "");
            let chars: Vec<char> = if dict.is_empty() {
                default.as_str()
            } else {
                dict
            }
            .chars()
            .take(radix)
            .collect();
            let a = alphabet(&chars.iter().collect::<String>(), Some(radix))?;
            return basex::radix_encode_checked(data, &a, true, Some(ctx));
        }
        Kind::B69 => {
            let a = alphabet(pstr(p, "dict", B69), Some(69))?;
            if a.contains(&'=') {
                return Err(bad("Base69 码表不能包含 padding 标记 ="));
            }
            let (mut acc, mut bits) = (0u32, 0);
            for (i, &b) in data.iter().enumerate() {
                tick(ctx, i, data.len())?;
                acc = (acc << 8) | b as u32;
                bits += 8;
                while bits >= 7 {
                    bits -= 7;
                    let n = ((acc >> bits) & 127) as usize;
                    out.push(a[n % 69]);
                    out.push(a[n / 69]);
                }
                acc &= (1 << bits) - 1;
            }
            if bits > 0 {
                let n = (acc << (7 - bits)) as usize;
                out.push(a[n % 69]);
                out.push(a[n / 69]);
            }
            let pad = (7 - data.len() % 7) % 7;
            if pad > 0 {
                out.push_str(&"AA".repeat(pad - 1));
                out.push(char::from_digit(pad as u32, 10).unwrap());
                out.push('=');
            }
        }
        Kind::B2048 => {
            let (mut acc, mut bits) = (0usize, 0);
            for (i, &b) in data.iter().enumerate() {
                tick(ctx, i, data.len())?;
                acc = (acc << 8) | b as usize;
                bits += 8;
                while bits >= 11 {
                    bits -= 11;
                    out.push(t.b2048[(acc >> bits) & 2047]);
                }
                acc &= (1 << bits) - 1;
            }
            if bits > 0 {
                let width = if bits <= 3 { 3 } else { 11 };
                acc = (acc << (width - bits)) | ((1 << (width - bits)) - 1);
                out.push(if width == 3 {
                    (b'0' + acc as u8) as char
                } else {
                    t.b2048[acc]
                });
            }
        }
        Kind::B65 => {
            for (i, c) in data.chunks(2).enumerate() {
                tick(ctx, i, data.len() / 2)?;
                let b2 = c.get(1).map_or(-1, |b| *b as i32);
                out.push(char::from_u32(t.b65[&b2] + c[0] as u32).unwrap());
            }
        }
        Kind::Eco => {
            for (i, c) in data.chunks(5).enumerate() {
                tick(ctx, i, data.len() / 5)?;
                let mut b = [0usize; 5];
                for (i, &v) in c.iter().enumerate() {
                    b[i] = v as usize;
                }
                let mut chars = [t.eco[(b[0] << 2) | (b[1] >> 6)], ECO_PAD, ECO_PAD, ECO_PAD];
                if c.len() >= 2 {
                    chars[1] = t.eco[((b[1] & 63) << 4) | (b[2] >> 4)];
                }
                if c.len() >= 3 {
                    chars[2] = t.eco[((b[2] & 15) << 6) | (b[3] >> 2)];
                }
                if c.len() == 4 {
                    chars[3] = ECO_PAD4[b[3] & 3];
                } else if c.len() == 5 {
                    chars[3] = t.eco[((b[3] & 3) << 8) | b[4]];
                }
                out.extend(chars);
            }
        }
    }
    ctx.cancel.check()?;
    Ok(out)
}
fn decode(kind: Kind, s: &str, p: &Value, ctx: &NodeCtx) -> Result<Vec<u8>, CoreError> {
    let strict = pbool(p, "strict", true);
    let t = tables();
    let mut out = Vec::new();
    match kind {
        Kind::Radix => {
            let radix = number(p, "radix", 36, 2, 95)? as usize;
            let default: String = (33..=126).map(|n| n as u8 as char).collect();
            let dict = pstr(p, "dict", "");
            let a: Vec<char> = if dict.is_empty() {
                default.as_str()
            } else {
                dict
            }
            .chars()
            .take(radix)
            .collect();
            let a = alphabet(&a.iter().collect::<String>(), Some(radix))?;
            let s = if a.iter().any(|c| space(*c)) {
                s
            } else {
                s.trim_matches(space)
            };
            return basex::radix_decode_checked(s, &a, true, false, Some(ctx));
        }
        Kind::B69 => {
            let a = alphabet(pstr(p, "dict", B69), Some(69))?;
            if a.contains(&'=') {
                return Err(bad("Base69 码表不能包含 ="));
            }
            let mut c: Vec<char> = s.trim_matches(space).chars().collect();
            if c.is_empty() {
                return Ok(out);
            }
            let legacy: Vec<char> = "AAAAAAAAAAAA7=".chars().collect();
            if !c.len().is_multiple_of(16) {
                if c.len() > 14 && c.len() % 16 == 14 && c.ends_with(&legacy) {
                    c.truncate(c.len() - 14);
                } else {
                    return Err(bad("Base69 长度需要 16 的倍数"));
                }
            }
            let pad = if c.last() == Some(&'=') {
                c[c.len() - 2]
                    .to_digit(10)
                    .filter(|n| (1..=6).contains(n))
                    .ok_or_else(|| bad("Base69 padding 无效"))? as usize
            } else {
                0
            };
            let count = c.len() / 2 - pad;
            let (mut acc, mut bits) = (0u32, 0);
            for (i, pair) in c[..count * 2].chunks(2).enumerate() {
                tick(ctx, i, count)?;
                let lo = a
                    .iter()
                    .position(|c| c == &pair[0])
                    .ok_or_else(|| bad("Base69 无效字符"))?;
                let hi = a
                    .iter()
                    .position(|c| c == &pair[1])
                    .ok_or_else(|| bad("Base69 无效字符"))?;
                let n = hi * 69 + lo;
                if n > 127 {
                    return Err(bad("Base69 字符对超过 7 位"));
                }
                acc = (acc << 7) | n as u32;
                bits += 7;
                while bits >= 8 {
                    bits -= 8;
                    out.push(((acc >> bits) & 255) as u8);
                }
                acc &= (1 << bits) - 1;
            }
            if strict {
                let canonical = encode(kind, &out, p, ctx)?;
                if canonical.chars().collect::<Vec<_>>() != c {
                    return Err(bad("Base69 padding 或尾部补位不规范"));
                }
            }
        }
        Kind::B2048 => {
            let (mut acc, mut bits) = (0usize, 0);
            for (i, c) in s.chars().enumerate() {
                tick(ctx, i, s.len())?;
                let (width, n) = if let Some(&n) = t.r2048.get(&c) {
                    (11, n)
                } else if ('0'..='7').contains(&c) {
                    (3, c as usize - '0' as usize)
                } else if strict {
                    return Err(bad("Base2048 字符不在码表中"));
                } else {
                    continue;
                };
                acc = (acc << width) | n;
                bits += width;
                while bits >= 8 {
                    bits -= 8;
                    out.push(((acc >> bits) & 255) as u8);
                }
                acc &= (1 << bits) - 1;
            }
        }
        Kind::B65 => {
            for (i, c) in s.chars().enumerate() {
                tick(ctx, i, s.len())?;
                let cp = c as u32;
                let low = (cp & 255) as u8;
                let block = cp & !255;
                let high = t.r65.get(&block);
                if strict && high.is_none() {
                    return Err(bad("Base65536 字符不在码表中"));
                }
                out.push(low);
                if let Some(&h) = high.filter(|h| **h >= 0) {
                    out.push(h as u8);
                }
            }
        }
        Kind::Eco => {
            let chars: Vec<_> = s.chars().collect();
            if strict && !chars.len().is_multiple_of(4) {
                return Err(bad("Ecoji 需要完整 4 字符组"));
            }
            for (i, g) in chars.chunks_exact(4).enumerate() {
                tick(ctx, i, chars.len() / 4)?;
                let len = if g[1] == ECO_PAD {
                    1
                } else if g[2] == ECO_PAD {
                    2
                } else if g[3] == ECO_PAD {
                    3
                } else if ECO_PAD4.contains(&g[3]) {
                    4
                } else {
                    5
                };
                let mut n = [0usize; 4];
                for (k, c) in g.iter().enumerate() {
                    n[k] = if k == 3 && ECO_PAD4.contains(c) {
                        ECO_PAD4.iter().position(|d| d == c).unwrap() << 8
                    } else if *c == ECO_PAD {
                        0
                    } else {
                        match t.reco.get(c) {
                            Some(&n) => n,
                            None if !strict => 0,
                            _ => return Err(bad("Ecoji 未知字符")),
                        }
                    };
                }
                let bytes = [
                    n[0] >> 2,
                    ((n[0] & 3) << 6) | (n[1] >> 4),
                    ((n[1] & 15) << 4) | (n[2] >> 6),
                    ((n[2] & 63) << 2) | (n[3] >> 8),
                    n[3] & 255,
                ];
                out.extend(bytes[..len].iter().map(|v| *v as u8));
            }
        }
    }
    if strict
        && matches!(kind, Kind::B2048 | Kind::B65 | Kind::Eco)
        && encode(kind, &out, p, ctx)? != s
    {
        return Err(bad("尾部补位、短组位置或 padding 不规范"));
    }
    ctx.cancel.check()?;
    Ok(out)
}
impl Node for Codec {
    fn run(&self, i: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        if self.decode {
            let s = in_text(i, "text")?;
            if s.len() > 16_777_216 {
                return Err(bad("编码文本最多 16 MiB"));
            }
            Ok(basex::decoded(decode(self.kind, s, p, ctx)?))
        } else {
            let b = in_bytes(i, "data")?;
            if b.len() > 1_048_576 {
                return Err(bad("编码输入最多 1 MiB"));
            }
            Ok(out_text(encode(self.kind, &b, p, ctx)?))
        }
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for(kind,stem,upstream,title,description)in[
 (Kind::Radix,"radix_n","radixN","字节任意进制","原始字节按大整数转 2..95 进制，前导零逐个保留；默认码表从 ASCII ! 开始，95 进制须提供自定义码表。"),
 (Kind::B69,"base69","base69","Base69","pshihn 7 字节分块，含尾部 padding；兼容严格的旧 7= 尾形态，保留全部字节。"),
 (Kind::B2048,"base2048","base2048","Base2048","固定上游 qntm 11-bit Unicode 码表，短尾组使用 3-bit 数字码表；严格模式校验 1 补位和短组位置。"),
 (Kind::B65,"base65536","base65536","Base65536","两个字节映射一个 Unicode 字符，奇数尾字节有专用块；严格模式拒绝未知码块及非末尾短组。"),
 (Kind::Eco,"ecoji","ecoji","Ecoji","固定 1024 字符表，5 字节转换为 4 个 emoji，支持 1..4 字节短尾组和 padding 校验。") ]{
 for is_decode in [false,true]{let mut params=match kind{
 Kind::Radix=>vec![help(ParamSpec::number("radix","进制",2.0,95.0,1.0,36.0),"2..95；默认 ASCII 可见码表最多 94 字符。"),help(ParamSpec::text("dict","自定义码表","",false),"留空用 ASCII 33..126；取前 radix 个唯一字符，按 Unicode 字符计数。")],
 Kind::B69=>vec![help(ParamSpec::text("dict","码表",B69,false),"69 个唯一字符，禁止 =；占位 AA 按上游固定字面值。")],_=>vec![]};
 if is_decode&&!matches!(kind,Kind::Radix){params.push(help(ParamSpec::toggle("strict","严格校验",true),"关闭保留上游容错：Base2048 跳过未知字符、Base65536 取未知块低字节、Ecoji 忽略短尾与未知字符。"));}
 let outputs=if is_decode{vec![t_out(),req("bytes","原始字节",PortType::Bytes)]}else{vec![t_out()]};let sample="Hello, 中文🙂";let encoded=encode(kind,sample.as_bytes(),&json!({}),&NodeCtx { node_id: "catalog-example".into(), sink: &crate::progress::NullSink, cancel: &crate::cancel::CancellationToken::new(), env: &crate::node::NodeEnv::default(), registry: reg, depth: 0 });
 let sample=if is_decode{encoded.unwrap()}else{sample.into()};let mut d=document_text(desc(&format!("{stem}_{}",if is_decode{"decode"}else{"encode"}),"encoding.base",&format!("{title} {}",if is_decode{"解码"}else{"编码"}),BLUE,if is_decode{vec![t_in()]}else{vec![req("data","输入",PortType::Any)]},outputs,params),upstream,description,&sample,json!({}));d.cost=Cost::Heavy;reg.register(d,Arc::new(move||Arc::new(Codec{kind,decode:is_decode})));
 }}
}
