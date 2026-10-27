//! BSON inspection with document-local boundaries and preserved wire values.
use super::{
    numeric_support::{bad, number},
    prelude::*,
    serial_support::*,
};
use serde_json::{json, Value};
fn cstring(r: &mut Reader) -> Result<String, CoreError> {
    let n = r.data[r.pos..r.limit]
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| bad("cstring 无终止符"))?;
    let s = utf8(r.take(n)?);
    r.byte()?;
    Ok(s)
}
fn string(r: &mut Reader) -> Result<(String, String), CoreError> {
    let n = r.uint(4, true)? as i32;
    if n < 1 || n as usize > r.limit - r.pos {
        return Err(bad(&format!("BSON 字符串长度 {n} 非法/越界")));
    }
    let b = r.take(n as usize)?;
    if b[b.len() - 1] != 0 {
        return Err(bad("BSON 字符串缺少 NUL 终止符"));
    }
    Ok((utf8(&b[..b.len() - 1]), hex::encode(&b[..b.len() - 1])))
}
fn document(r: &mut Reader, depth: usize, ctx: &NodeCtx) -> Result<Vec<Value>, CoreError> {
    r.enter(depth, ctx)?;
    let start = r.pos;
    let n = r.uint(4, true)? as i32;
    let end = start
        .checked_add(n.max(0) as usize)
        .filter(|&n| n <= r.limit)
        .ok_or_else(|| bad(&format!("BSON 文档长度 {n} 非法/越界")))?;
    if n < 5 {
        return Err(bad(&format!("BSON 文档长度 {n} 非法/越界")));
    }
    if r.data[end - 1] != 0 {
        return Err(bad("BSON 文档缺少终止符"));
    }
    let limit = r.limit;
    r.limit = end - 1;
    let mut fields = Vec::new();
    while r.pos < r.limit {
        r.enter(depth, ctx)?;
        let off = r.pos;
        let code = r.byte()?;
        let name_start = r.pos;
        let name = cstring(r)?;
        let name_hex = hex::encode(&r.data[name_start..r.pos - 1]);
        let kind = match code {
            1 => "double",
            2 => "string",
            3 => "document",
            4 => "array",
            5 => "binary",
            6 => "undefined",
            7 => "ObjectId",
            8 => "bool",
            9 => "datetime",
            10 => "null",
            11 => "regex",
            12 => "dbPointer",
            13 => "javascript",
            14 => "symbol",
            15 => "javascriptScope",
            16 => "int32",
            17 => "timestamp",
            18 => "int64",
            19 => "decimal128",
            255 => "minKey",
            127 => "maxKey",
            _ => return Err(bad(&format!("BSON 类型 0x{code:x} 未实现"))),
        };
        let mut f =
            json!({"name":name,"nameRawHex":name_hex,"type":code,"typeName":kind,"offset":off});
        f["value"] = match code {
            1 => json!(float(f64::from_bits(r.uint(8, true)?))),
            2 | 13 | 14 => {
                let (s, raw) = string(r)?;
                f["rawHex"] = json!(raw);
                json!(s)
            }
            3 | 4 => {
                let v = document(r, depth + 1, ctx)?;
                f["nested"] = json!(true);
                f["isArray"] = json!(code == 4);
                json!(v)
            }
            5 => {
                let n = r.uint(4, true)? as i32;
                if n < 0 {
                    return Err(bad("BSON binary 长度为负"));
                }
                let sub = r.byte()?;
                let b = r.take(n as usize)?;
                f["subtype"] = json!(match sub {
                    0 => "generic".into(),
                    1 => "function".into(),
                    2 => "old binary".into(),
                    3 => "old UUID".into(),
                    4 => "UUID".into(),
                    5 => "MD5".into(),
                    6 => "encrypted".into(),
                    7 => "column".into(),
                    _ => format!("0x{sub:x}"),
                });
                f["subtypeCode"] = json!(sub);
                if sub == 2 {
                    if b.len() < 4
                        || i32::from_le_bytes(b[..4].try_into().unwrap()) < 0
                        || u32::from_le_bytes(b[..4].try_into().unwrap()) as usize != b.len() - 4
                    {
                        return Err(bad("BSON old binary 内外长度不一致"));
                    }
                    f["payloadHex"] = json!(hex::encode(&b[4..]));
                } else {
                    f["payloadHex"] = json!(hex::encode(b));
                }
                json!(hex::encode(b))
            }
            6 => json!("undefined"),
            7 => json!(hex::encode(r.take(12)?)),
            8 => {
                let b = r.byte()?;
                if b > 1 {
                    return Err(bad("BSON bool 必须为 0 或 1"));
                }
                json!(b == 1)
            }
            9 => {
                let n = r.uint(8, true)? as i64;
                f["milliseconds"] = json!(n.to_string());
                json!(format!("{n} ms"))
            }
            10 => Value::Null,
            11 => {
                let pat = cstring(r)?;
                let opt = cstring(r)?;
                f["pattern"] = json!(pat);
                f["options"] = json!(opt);
                json!(format!("/{pat}/{opt}"))
            }
            12 => {
                let (s, raw) = string(r)?;
                let id = hex::encode(r.take(12)?);
                f["namespaceHex"] = json!(raw);
                f["objectId"] = json!(id);
                json!(format!("{s} ({id})"))
            }
            15 => {
                let begin = r.pos;
                let len = r.uint(4, true)? as i32;
                if len < 14 || len as usize > r.limit - begin {
                    return Err(bad("BSON code-with-scope 长度非法"));
                }
                let old = r.limit;
                r.limit = begin + len as usize;
                let (code, raw) = string(r)?;
                let scope = document(r, depth + 1, ctx)?;
                if r.pos != r.limit {
                    return Err(bad("BSON code-with-scope 长度不一致"));
                }
                r.limit = old;
                f["rawHex"] = json!(raw);
                f["scope"] = json!(scope);
                json!(code)
            }
            16 => json!(r.uint(4, true)? as u32 as i32),
            17 => {
                let inc = r.uint(4, true)?;
                let sec = r.uint(4, true)?;
                f["seconds"] = json!(sec);
                f["increment"] = json!(inc);
                json!(format!("T:{sec} I:{inc}"))
            }
            18 => json!((r.uint(8, true)? as i64).to_string()),
            // Preserve all 128 bits; do not convert a decimal128 into binary64.
            19 => {
                let b = r.take(16)?;
                f["rawHex"] = json!(hex::encode(b));
                json!(format!("bits(LE)={}", hex::encode(b)))
            }
            255 => json!("minKey"),
            127 => json!("maxKey"),
            _ => unreachable!(),
        };
        f["end"] = json!(r.pos);
        fields.push(f);
    }
    r.limit = limit;
    r.pos = end;
    Ok(fields)
}
fn format_fields(fields: &[Value], indent: &str, lines: &mut Vec<String>) {
    for f in fields {
        let kind = f["typeName"].as_str().unwrap();
        let value = &f["value"];
        let shown = if f["nested"] == true {
            if f["isArray"] == true {
                format!("[{} items]", value.as_array().unwrap().len())
            } else {
                format!("{{{} fields}}", value.as_array().unwrap().len())
            }
        } else if kind == "binary" {
            let b = hex::decode(value.as_str().unwrap()).unwrap();
            format!(
                "bin[{}]({}): {}",
                b.len(),
                f["subtype"].as_str().unwrap(),
                hex_preview(&b, 32)
            )
        } else if matches!(kind, "double" | "int32" | "bool" | "null" | "undefined") {
            string_value(value)
        } else {
            format!("\"{}\"", string_value(value))
        };
        lines.push(format!(
            "{indent}\"{}\" ({kind}): {shown}",
            f["name"].as_str().unwrap()
        ));
        if f["nested"] == true {
            format_fields(value.as_array().unwrap(), &format!("{indent}  "), lines);
        }
        if let Some(scope) = f["scope"].as_array() {
            format_fields(scope, &format!("{indent}  "), lines);
        }
    }
}
struct Bson;
impl Node for Bson {
    fn run(&self, i: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let data = input(i, p)?;
        if data.is_empty() {
            return Ok(outputs(
                "请输入 BSON 二进制数据（hex 或 base64）".into(),
                json!([]),
                0,
                &data,
                Some("空输入".into()),
            ));
        }
        let mut r = Reader::new(&data, number(p, "maxDepth", 64, 1, 128)? as usize);
        let mut lines = vec![
            "=== BSON 文档解析 ===".into(),
            format!("输入: {} 字节", data.len()),
            String::new(),
        ];
        let (mut tree, mut error) = (Value::Null, None);
        match document(&mut r, 0, ctx) {
            Ok(v) => {
                format_fields(&v, "", &mut lines);
                tree = json!(v);
                if r.pos < data.len() {
                    lines.extend([
                        String::new(),
                        format!("⚠ 仅消费 {}/{} 字节", r.pos, data.len()),
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
    reg.register(descriptor("bson_parse","BSON 文档结构解析","bsonParse","解析 BSON 文档、嵌套数组、二进制 subtype、ObjectId、正则和整数。每个文档独立检查长度/NUL，修复上游嵌套终止符错位；decimal128 保留完整位模式，代码内容只展示。","0c000000106e002a00000000"),Arc::new(||Arc::new(Bson)));
}
