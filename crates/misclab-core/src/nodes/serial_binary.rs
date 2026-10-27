//! Schema-free MessagePack and CBOR inspection; no object instantiation.
use super::{
    numeric_support::{bad, number},
    prelude::*,
    serial_support::*,
};
use serde_json::{json, Value};

fn sequence(
    r: &mut Reader,
    depth: usize,
    n: Option<usize>,
    map: bool,
    cbor: bool,
    ctx: &NodeCtx,
) -> Result<Value, CoreError> {
    if n.is_some_and(|n| n > 20_000) {
        return Err(bad("容器项数超过 20000"));
    }
    let mut values = Vec::new();
    loop {
        if n.is_some_and(|n| values.len() == n) {
            break;
        }
        if n.is_none() {
            if r.pos >= r.limit {
                return Err(bad("cbor indefinite 容器未结束"));
            }
            if r.data[r.pos] == 0xff {
                r.pos += 1;
                break;
            }
        }
        let value = if cbor {
            parse_cbor(r, depth + 1, ctx)?
        } else {
            parse_msgpack(r, depth + 1, ctx)?
        };
        if map {
            let second = if cbor {
                parse_cbor(r, depth + 1, ctx)?
            } else {
                parse_msgpack(r, depth + 1, ctx)?
            };
            values.push(json!({"key":value,"value":second}));
        } else {
            values.push(value);
        }
    }
    Ok(json!({"type":if map{"map"}else{"array"},"value":values,"count":n,"indefinite":n.is_none()}))
}
fn blob(r: &mut Reader, len: usize, kind: &str, ext: Option<u8>) -> Result<Value, CoreError> {
    let b = r.take(len)?;
    let v = if kind == "str" || kind == "tstr" {
        Value::String(utf8(b))
    } else {
        Value::String(hex::encode(b))
    };
    let mut o = json!({"type":kind,"value":v,"length":len});
    if let Some(ext) = ext {
        o["extType"] = json!(ext);
        o["signedExtType"] = json!(ext as i8);
    }
    if matches!(kind, "str" | "tstr") {
        o["rawHex"] = json!(hex::encode(b));
    }
    Ok(o)
}
fn parse_msgpack(r: &mut Reader, depth: usize, ctx: &NodeCtx) -> Result<Value, CoreError> {
    r.enter(depth, ctx)?;
    let start = r.pos;
    let b = r.byte()?;
    let mut node = match b {
        0..=0x7f => scalar("uint8", b),
        0x80..=0x8f => sequence(r, depth, Some((b & 15) as usize), true, false, ctx)?,
        0x90..=0x9f => sequence(r, depth, Some((b & 15) as usize), false, false, ctx)?,
        0xa0..=0xbf => blob(r, (b & 31) as usize, "str", None)?,
        0xc0 => scalar("nil", Value::Null),
        0xc2 | 0xc3 => scalar("bool", b == 0xc3),
        0xc4..=0xc6 => {
            let n = r.len(1usize << (b - 0xc4), false)?;
            blob(r, n, "bin", None)?
        }
        0xc7..=0xc9 => {
            let n = r.len(1usize << (b - 0xc7), false)?;
            let ext = r.byte()?;
            blob(r, n, "ext", Some(ext))?
        }
        0xca => {
            let n = r.uint(4, false)? as u32;
            scalar("float32", float(f32::from_bits(n) as f64))
        }
        0xcb => {
            let n = r.uint(8, false)?;
            scalar("float64", float(f64::from_bits(n)))
        }
        0xcc..=0xcf => {
            let width = 1usize << (b - 0xcc);
            let n = r.uint(width, false)?;
            scalar(&format!("uint{}", width * 8), n.to_string())
        }
        0xd0..=0xd3 => {
            let width = 1usize << (b - 0xd0);
            let n = r.uint(width, false)?;
            let signed = ((n << (64 - width * 8)) as i64) >> (64 - width * 8);
            scalar(&format!("int{}", width * 8), signed.to_string())
        }
        0xd4..=0xd8 => {
            let ext = r.byte()?;
            blob(r, 1usize << (b - 0xd4), "ext", Some(ext))?
        }
        0xd9..=0xdb => {
            let n = r.len(1usize << (b - 0xd9), false)?;
            blob(r, n, "str", None)?
        }
        0xdc..=0xdd => {
            let n = r.len(if b == 0xdc { 2 } else { 4 }, false)?;
            sequence(r, depth, Some(n), false, false, ctx)?
        }
        0xde..=0xdf => {
            let n = r.len(if b == 0xde { 2 } else { 4 }, false)?;
            sequence(r, depth, Some(n), true, false, ctx)?
        }
        0xe0..=0xff => scalar("int8", (b as i8).to_string()),
        _ => return Err(bad(&format!("未知 msgpack 字节: 0x{b:02x} @pos={start}"))),
    };
    node["offset"] = json!(start);
    node["end"] = json!(r.pos);
    Ok(node)
}
fn parse_cbor(r: &mut Reader, depth: usize, ctx: &NodeCtx) -> Result<Value, CoreError> {
    r.enter(depth, ctx)?;
    let start = r.pos;
    let b = r.byte()?;
    let major = b >> 5;
    let ai = b & 31;
    let raw = match ai {
        0..=23 => ai as u64,
        24 => r.uint(1, false)?,
        25 => r.uint(2, false)?,
        26 => r.uint(4, false)?,
        27 => r.uint(8, false)?,
        31 => 0,
        _ => return Err(bad(&format!("cbor 保留 ai={ai}"))),
    };
    if ai == 31 && !(2..=5).contains(&major) {
        return Err(bad("CBOR break/indefinite 出现在非法位置"));
    }
    let mut node = match major {
        0 => scalar("uint", raw.to_string()),
        1 => scalar("nint", (-1i128 - raw as i128).to_string()),
        2 | 3 => {
            let kind = if major == 2 { "bstr" } else { "tstr" };
            if ai == 31 {
                let mut bytes = Vec::new();
                let mut text = String::new();
                loop {
                    if r.pos >= r.limit {
                        return Err(bad(&format!("cbor indefinite {kind} 未结束")));
                    }
                    if r.data[r.pos] == 0xff {
                        r.pos += 1;
                        break;
                    }
                    if r.data[r.pos] >> 5 != major || r.data[r.pos] & 31 == 31 {
                        return Err(bad("CBOR 不定长字符串只允许同类型定长块"));
                    }
                    let sub = parse_cbor(r, depth + 1, ctx)?;
                    if major == 2 {
                        bytes.extend(hex::decode(sub["value"].as_str().unwrap()).unwrap());
                    } else {
                        text.push_str(sub["value"].as_str().unwrap());
                        bytes.extend(hex::decode(sub["rawHex"].as_str().unwrap()).unwrap());
                    }
                }
                json!({"type":kind,"value":if major==2{hex::encode(&bytes)}else{text},"rawHex":hex::encode(&bytes),"indefinite":true})
            } else {
                let len = usize::try_from(raw).map_err(|_| bad("CBOR 长度越界"))?;
                if len > r.limit - r.pos {
                    return Err(bad(&format!("cbor {kind} 越界")));
                }
                blob(r, len, kind, None)?
            }
        }
        4 | 5 => {
            let n = if ai == 31 {
                None
            } else {
                Some(usize::try_from(raw).map_err(|_| bad("CBOR 长度越界"))?)
            };
            sequence(r, depth, n, major == 5, true, ctx)?
        }
        6 => {
            let sub = parse_cbor(r, depth + 1, ctx)?;
            json!({"type":"tag","tag":raw.to_string(),"value":sub})
        }
        7 => match ai {
            20 | 21 => scalar("bool", ai == 21),
            22 => scalar("null", Value::Null),
            23 => scalar("undefined", "undefined"),
            25 => scalar("float16", float(half::f16::from_bits(raw as u16).to_f64())),
            26 => scalar("float32", float(f32::from_bits(raw as u32) as f64)),
            27 => scalar("float64", float(f64::from_bits(raw))),
            // RFC 8949: simple values 24..31 encoded through ai=24 are invalid.
            24 if raw < 32 => return Err(bad("CBOR simple 值的扩展编码必须 >= 32")),
            _ => scalar("simple", raw),
        },
        _ => unreachable!(),
    };
    node["offset"] = json!(start);
    node["end"] = json!(r.pos);
    Ok(node)
}
fn format_tree(node: &Value, indent: &str, lines: &mut Vec<String>) {
    let kind = node["type"].as_str().unwrap();
    let v = &node["value"];
    let indefinite = node["indefinite"] == true;
    let suffix = if indefinite { " (indefinite)" } else { "" };
    let count = node["count"]
        .as_u64()
        .map(|n| n.to_string())
        .unwrap_or_else(|| "?".into());
    let value = string_value(v);
    let line = match kind {
        "nil" | "null" | "undefined" => kind.into(),
        "bool" => value,
        "str" | "tstr" => format!("{kind}: \"{value}\"{suffix}"),
        "bin" | "bstr" | "ext" => {
            let data = hex::decode(v.as_str().unwrap()).unwrap();
            let len = node["length"]
                .as_u64()
                .map(|n| n.to_string())
                .unwrap_or_else(|| "?".into());
            if kind == "ext" {
                format!(
                    "ext[{len}] type={}: {}",
                    node["extType"],
                    hex_preview(&data, 32)
                )
            } else {
                format!("{kind}[{len}]: {}{suffix}", hex_preview(&data, 32))
            }
        }
        "array" | "map" => format!("{kind}[{count}]:{suffix}"),
        "tag" => format!("tag({}):", string_value(&node["tag"])),
        _ => format!("{kind}: {value}"),
    };
    lines.push(format!("{indent}{line}"));
    if kind == "array" {
        for v in v.as_array().unwrap() {
            format_tree(v, &format!("{indent}  "), lines);
        }
    }
    if kind == "map" {
        for pair in v.as_array().unwrap() {
            let key = &pair["key"];
            let mut ks = string_value(&key["value"]);
            if matches!(key["type"].as_str(), Some("str" | "tstr")) {
                ks = format!("\"{ks}\"");
            }
            lines.push(format!("{indent}  {ks}:"));
            format_tree(&pair["value"], &format!("{indent}    "), lines);
        }
    }
    if kind == "tag" {
        format_tree(v, &format!("{indent}  "), lines);
    }
}
struct Parser(bool);
impl Node for Parser {
    fn run(&self, i: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let data = input(i, p)?;
        let name = if self.0 { "CBOR" } else { "MessagePack" };
        if data.is_empty() {
            return Ok(outputs(
                format!("请输入 {name} 二进制数据（hex 或 base64）"),
                Value::Null,
                0,
                &data,
                Some("空输入".into()),
            ));
        }
        let depth = number(p, "maxDepth", 64, 1, 128)? as usize;
        let mut r = Reader::new(&data, depth);
        let parsed = if self.0 {
            parse_cbor(&mut r, 0, ctx)
        } else {
            parse_msgpack(&mut r, 0, ctx)
        };
        let title = if self.0 {
            "CBOR 解析（RFC 8949）"
        } else {
            "MessagePack 解析"
        };
        let mut lines = vec![
            format!("=== {title} ==="),
            format!("输入: {} 字节", data.len()),
            String::new(),
        ];
        let (mut tree, mut error) = (Value::Null, None);
        match parsed {
            Ok(v) => {
                format_tree(&v, "", &mut lines);
                tree = v;
                if r.pos < data.len() {
                    lines.extend([
                        String::new(),
                        format!(
                            "⚠ 仅消费 {}/{} 字节，剩余 {} 字节",
                            r.pos,
                            data.len(),
                            data.len() - r.pos
                        ),
                    ]);
                }
            }
            Err(CoreError::Cancelled) => return Err(CoreError::Cancelled),
            Err(e) => {
                let s = error_message(&e);
                lines.push(format!("⚠ 解析失败: {s}"));
                error = Some(s);
            }
        }
        ctx.cancel.check()?;
        Ok(outputs(lines.join("\n"), tree, r.pos, &data, error))
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for(cbor,id,upstream,title,sample,description)in[
        (false,"msgpack_parse","msgpackParse","MessagePack 结构解析","82a46e616d65a34d6978a16e2a","解析全部 MessagePack 基本、容器、二进制与扩展类型；64 位整数保持十进制文本，原始字符串字节保存在结构树。截断数据明确报错，不补零。"),
        (true,"cbor_parse","cborParse","CBOR 结构解析","a2646e616d65634d6978616e182a","解析 RFC 8949 major type 0..7、标签、半精度和不定长容器；保留重复/非字符串键及原始字节，严格检查块类型、break 和边界。")
    ]{reg.register(descriptor(id,title,upstream,description,sample),Arc::new(move||Arc::new(Parser(cbor))));}
}
