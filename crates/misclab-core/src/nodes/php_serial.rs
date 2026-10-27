//! Static PHP serialize reader. Strings are byte counted; objects stay data.
use super::{
    numeric_support::{bad, integer, number, space},
    prelude::*,
    serial_support::*,
};
use serde_json::{json, Value};
fn expect(r: &mut Reader, expected: u8) -> Result<(), CoreError> {
    let pos = r.pos;
    if r.byte()? != expected {
        return Err(bad(&format!("PHP 需要 '{}' @pos={pos}", expected as char)));
    }
    Ok(())
}
fn token(r: &mut Reader, delimiter: u8) -> Result<String, CoreError> {
    let n = r.data[r.pos..r.limit]
        .iter()
        .position(|&b| b == delimiter)
        .ok_or_else(|| bad("PHP 值缺少终止分隔符"))?;
    if n > 16384 {
        return Err(bad("PHP 数字/长度超过 16384 字符"));
    }
    let s = std::str::from_utf8(r.take(n)?)
        .map_err(|_| bad("PHP 数字/长度必须为 ASCII"))?
        .to_string();
    r.byte()?;
    Ok(s)
}
fn length(r: &mut Reader) -> Result<usize, CoreError> {
    let s = token(r, b':')?;
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad("PHP 长度必须为非负十进制整数"));
    }
    s.parse::<usize>()
        .ok()
        .filter(|&n| n <= MAX_INPUT)
        .ok_or_else(|| bad("PHP 长度超过 1 MiB"))
}
fn quoted(r: &mut Reader) -> Result<(String, String, usize), CoreError> {
    let n = length(r)?;
    expect(r, b'"')?;
    let raw = r.take(n)?;
    let value = String::from_utf8_lossy(raw).into_owned();
    let hex = hex::encode(raw);
    expect(r, b'"')?;
    Ok((value, hex, n))
}
fn ws(r: &mut Reader) {
    while r.pos < r.limit && r.data[r.pos].is_ascii_whitespace() {
        r.pos += 1;
    }
}
fn parse(r: &mut Reader, depth: usize, ctx: &NodeCtx) -> Result<Value, CoreError> {
    r.enter(depth, ctx)?;
    ws(r);
    let start = r.pos;
    let kind = r.byte()?;
    if kind == b'N' {
        expect(r, b';')?;
        return Ok(json!({"type":"null","value":null,"offset":start,"end":r.pos}));
    }
    expect(r, b':')?;
    let mut node = match kind {
        b'b' => {
            let s = token(r, b';')?;
            if !["0", "1"].contains(&s.as_str()) {
                return Err(bad("PHP bool 必须为 0 或 1"));
            }
            scalar("bool", s == "1")
        }
        b'i' => {
            let s = token(r, b';')?;
            scalar("int", integer(&s, 10)?.to_string())
        }
        b'd' => {
            let s = token(r, b';')?;
            let value = match s.as_str() {
                "INF" => "Infinity".into(),
                "-INF" => "-Infinity".into(),
                "NAN" => "NaN".into(),
                _ => {
                    if !regex::Regex::new(
                        r"^[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?$",
                    )
                    .unwrap()
                    .is_match(&s)
                    {
                        return Err(bad("PHP 浮点格式无效"));
                    }
                    float(super::float_codecs::js_number(&s)?)
                }
            };
            scalar("float", value)
        }
        b's' => {
            let (s, raw, len) = quoted(r)?;
            expect(r, b';')?;
            json!({"type":"string","value":s,"rawHex":raw,"length":len})
        }
        b'a' | b'O' => {
            let mut class = None;
            if kind == b'O' {
                let (name, raw, _) = quoted(r)?;
                class = Some((name, raw));
                expect(r, b':')?;
            }
            let count = length(r)?;
            if count > 20_000 {
                return Err(bad("PHP 容器项数超过 20000"));
            }
            expect(r, b'{')?;
            let mut entries = Vec::new();
            for _ in 0..count {
                let key = parse(r, depth + 1, ctx)?;
                let value = parse(r, depth + 1, ctx)?;
                if !matches!(key["type"].as_str(), Some("int" | "string")) {
                    return Err(bad("PHP 键必须为整数或字符串"));
                }
                entries.push(json!({"key":key,"value":value}));
            }
            ws(r);
            expect(r, b'}')?;
            let mut n =
                json!({"type":if kind==b'a'{"array"}else{"object"},"count":count,"value":entries});
            if let Some((name, raw)) = class {
                n["className"] = json!(name);
                n["classNameHex"] = json!(raw);
            }
            n
        }
        b'C' => {
            let (name, raw, _) = quoted(r)?;
            expect(r, b':')?;
            let len = length(r)?;
            expect(r, b'{')?;
            let data = r.take(len)?;
            let text = String::from_utf8_lossy(data).into_owned();
            let bytes = hex::encode(data);
            expect(r, b'}')?;
            json!({"type":"serializable","className":name,"classNameHex":raw,"length":len,"value":text,"rawHex":bytes})
        }
        b'r' | b'R' => {
            let s = token(r, b';')?;
            let n = s
                .parse::<u64>()
                .ok()
                .filter(|&n| n > 0)
                .ok_or_else(|| bad("PHP 引用编号必须为正整数"))?;
            scalar(if kind == b'r' { "ref" } else { "refObj" }, n.to_string())
        }
        _ => {
            return Err(bad(&format!(
                "PHP serialize 未知类型 '{}' @pos={start}",
                kind as char
            )))
        }
    };
    node["offset"] = json!(start);
    node["end"] = json!(r.pos);
    Ok(node)
}
fn format_node(n: &Value, indent: &str, legacy: bool, nested: bool, lines: &mut Vec<String>) {
    let kind = n["type"].as_str().unwrap();
    let value = string_value(&n["value"]);
    let count = if legacy && nested {
        "undefined".into()
    } else {
        string_value(&n["count"])
    };
    let length = if legacy && nested {
        "undefined".into()
    } else {
        string_value(&n["length"])
    };
    let class = if legacy && nested {
        "undefined".into()
    } else {
        string_value(&n["className"])
    };
    let line = match kind {
        "null" => "null".into(),
        "bool" | "int" | "float" => format!("{kind}: {value}"),
        "string" => format!("string[{length}]: \"{value}\""),
        "array" => format!("array[{count}]:"),
        "object" => format!("object({class})[{count}]:"),
        "serializable" => format!("serializable({class})[{length}]: \"{value}\""),
        "ref" => format!("&{value} (reference)"),
        "refObj" => format!("&{value} (object reference)"),
        _ => unreachable!(),
    };
    lines.push(format!("{indent}{line}"));
    if matches!(kind, "array" | "object") {
        for item in n["value"].as_array().unwrap() {
            let k = &item["key"];
            let key = string_value(&k["value"]);
            let kt = k["type"].as_str().unwrap();
            lines.push(if kind == "array" {
                format!("{indent}  [{key}] ({kt}):")
            } else {
                format!("{indent}  {key} ({kt}):")
            });
            format_node(
                &item["value"],
                &format!("{indent}    "),
                legacy,
                true,
                lines,
            );
        }
    }
}
struct Php;
impl Node for Php {
    fn run(&self, i: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        // Text input keeps the original text path; raw bytes preserve invalid UTF-8.
        let data = match i.get("data") {
            Some(PortValue::Text(s)) => {
                if s.len() > MAX_INPUT {
                    return Err(bad("PHP 输入最多 1 MiB"));
                }
                s.trim_matches(space).as_bytes().to_vec()
            }
            _ => input(i, p)?,
        };
        if data.is_empty() {
            return Ok(outputs(
                "请输入 PHP serialize() 字符串".into(),
                Value::Null,
                0,
                &data,
                Some("空输入".into()),
            ));
        }
        let mut r = Reader::new(&data, number(p, "maxDepth", 64, 1, 128)? as usize);
        let chars = String::from_utf8_lossy(&data).encode_utf16().count();
        let mut lines = vec![
            "=== PHP serialize 解析 ===".into(),
            format!("输入: {chars} 字符"),
            String::new(),
        ];
        let (mut tree, mut error) = (Value::Null, None);
        match parse(&mut r, 0, ctx) {
            Ok(v) => {
                format_node(&v, "", pbool(p, "legacyReport", false), false, &mut lines);
                tree = v;
                if r.pos < data.len() {
                    let consumed = String::from_utf8_lossy(&data[..r.pos])
                        .encode_utf16()
                        .count();
                    let rest = String::from_utf8_lossy(&data[r.pos..])
                        .chars()
                        .take(40)
                        .collect::<String>();
                    lines.extend([
                        String::new(),
                        format!("⚠ 仅消费 {consumed}/{chars} 字符，剩余: \"{rest}\""),
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
    let mut d=descriptor("php_serialize_parse","PHP serialize 结构解析","phpSerializeParse","静态解析 N/b/i/d/s/a/O/C/r/R，字符串和类名按实际字节长度读取，完整树保留 NUL、非 UTF-8 原始字节与引用编号。只展示对象和代码数据。文本输入始终为原始 PHP 字符串。","a:2:{s:4:\"name\";s:3:\"Mix\";s:1:\"n\";i:42;}");
    d.params.retain(|p| p.name != "inputEnc");
    d.params.push(super::codec_support::help(ParamSpec::toggle("legacyReport","兼容上游报告",false),"仅报告复现上游丢失嵌套 length/count/className 的 undefined 文案；结构树始终保留正确元数据。"));
    reg.register(d, Arc::new(|| Arc::new(Php)));
}
