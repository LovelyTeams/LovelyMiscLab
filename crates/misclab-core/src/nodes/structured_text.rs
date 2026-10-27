//! Structured text and numeric tools (EBCTF migration batch 2).
use super::{
    basex::decoded,
    prelude::*,
    transport_codecs::{b64decode, err, help, hex_input},
};
use crate::node::descriptor::NodeExample;
use base64::Engine as _;
use num_bigint::{BigInt, BigUint, Sign};
use num_traits::{ToPrimitive, Zero};
use serde_json::{json, Value};

#[derive(Clone, Copy)]
enum Tool {
    Query,
    Cookie,
    AsciiControl,
    SplitHex,
    HexReverse,
    VarintEncode,
    VarintDecode,
    JsonEncode,
    JsonDecode,
    DataUriEncode,
    DataUriDecode,
}
struct ToolNode(Tool);

fn percent_decode(s: &str) -> Vec<u8> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(a), Some(b)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                out.push((a * 16 + b) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}
pub(super) fn query_decode(s: &str) -> String {
    let replaced = s.replace('+', " ");
    // decodeURIComponent rejects a malformed escape or invalid UTF-8 as a whole.
    let bytes = replaced.as_bytes();
    if bytes.iter().enumerate().any(|(i, &c)| {
        c == b'%'
            && (i + 2 >= bytes.len()
                || !bytes[i + 1].is_ascii_hexdigit()
                || !bytes[i + 2].is_ascii_hexdigit())
    }) {
        return s.into();
    }
    String::from_utf8(percent_decode(&replaced)).unwrap_or_else(|_| s.into())
}
fn parse_query(text: &str) -> Result<PortMap, CoreError> {
    let s = text.trim();
    if s.is_empty() {
        return Err(err("空输入"));
    }
    let s = s
        .split_once('?')
        .map(|(_, q)| q)
        .unwrap_or(s)
        .split('#')
        .next()
        .unwrap_or("");
    let rows: Vec<_> = s
        .split('&')
        .filter(|s| !s.is_empty())
        .map(|s| {
            let (key, value) = s.split_once('=').unwrap_or((s, ""));
            json!({ "key": query_decode(key), "value": query_decode(value) })
        })
        .collect();
    let report = if s.is_empty() {
        "(无 query 参数)".into()
    } else {
        rows.iter()
            .map(|r| {
                format!(
                    "{} = {}",
                    r["key"].as_str().unwrap(),
                    r["value"].as_str().unwrap()
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let mut out = out_text(report);
    out.insert("rows".into(), PortValue::Json(json!(rows)));
    Ok(out)
}
fn parse_cookie(text: &str) -> Result<PortMap, CoreError> {
    let mut s = text.trim();
    if s.is_empty() {
        return Err(err("空输入"));
    }
    for prefix in ["set-cookie:", "cookie:"] {
        if s.get(..prefix.len())
            .is_some_and(|p| p.eq_ignore_ascii_case(prefix))
        {
            s = s[prefix.len()..].trim_start();
        }
    }
    let parts: Vec<_> = s
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let attrs = [
        "expires",
        "max-age",
        "domain",
        "path",
        "secure",
        "httponly",
        "samesite",
        "priority",
        "partitioned",
    ];
    let set_cookie = parts
        .iter()
        .skip(1)
        .any(|p| attrs.contains(&p.split('=').next().unwrap().trim().to_lowercase().as_str()));
    let rows: Vec<_> = parts.iter().enumerate().map(|(i, part)| { let (key, value) = part.split_once('=').unwrap_or((part, "")); json!({ "key": key.trim(), "value": value.trim(), "attribute": set_cookie && i > 0, "flag": !part.contains('=') }) }).collect();
    let report = if parts.is_empty() {
        "(无 Cookie)".into()
    } else if set_cookie {
        let (name, value) = parts[0].split_once('=').unwrap_or((parts[0], ""));
        let mut lines = vec![
            format!("Cookie 名:   {name}"),
            format!("Cookie 值:   {value}"),
            "--- 属性 ---".into(),
        ];
        lines.extend(parts.iter().skip(1).map(|p| {
            p.split_once('=')
                .map(|(k, v)| format!("{}: {}", k.trim(), v.trim()))
                .unwrap_or_else(|| format!("{p}: (标志)"))
        }));
        lines.join("\n")
    } else {
        rows.iter()
            .map(|r| {
                format!(
                    "{} = {}",
                    r["key"].as_str().unwrap(),
                    r["value"].as_str().unwrap()
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let mut out = out_text(report);
    out.insert("rows".into(), PortValue::Json(json!(rows)));
    Ok(out)
}
const CONTROL_NAMES: &[&str] = &[
    "NUL", "SOH", "STX", "ETX", "EOT", "ENQ", "ACK", "BEL", "BS", "HT", "LF", "VT", "FF", "CR",
    "SO", "SI", "DLE", "DC1", "DC2", "DC3", "DC4", "NAK", "SYN", "ETB", "CAN", "EM", "SUB", "ESC",
    "FS", "GS", "RS", "US", "SP", "DEL",
];
fn control_index(c: char) -> Option<usize> {
    if (c as u32) <= 32 {
        Some(c as usize)
    } else if c == '\u{7f}' {
        Some(33)
    } else {
        None
    }
}
fn ascii_control(text: &str, p: &Value) -> Result<String, CoreError> {
    let s = text.trim();
    if s.is_empty() {
        return Ok("（空输入）".into());
    }
    match pstr(p, "mode", "name") {
        "char" => Ok(s
            .chars()
            .map(|c| {
                control_index(c)
                    .map(|i| format!("[{}]", CONTROL_NAMES[i]))
                    .unwrap_or_else(|| c.to_string())
            })
            .collect()),
        "name" => Ok(s
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|s| !s.is_empty())
            .map(|token| {
                if let Some(i) = CONTROL_NAMES
                    .iter()
                    .position(|s| s.eq_ignore_ascii_case(token))
                {
                    let code = if i == 33 { 127 } else { i };
                    let symbol =
                        char::from_u32(0x2400 + if i == 33 { 33 } else { i as u32 }).unwrap();
                    format!(
                        "ASCII 值: {code}, 名称: {}, 符号: {symbol}",
                        CONTROL_NAMES[i]
                    )
                } else {
                    let codes = token
                        .chars()
                        .map(|c| {
                            let mut buf = [0u16; 2];
                            c.encode_utf16(&mut buf)[0].to_string()
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    format!("ASCII 值: {codes}, 名称: {token}, 符号: {token}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")),
        _ => Err(err("控制字符模式应为 name/char")),
    }
}
fn varint_encode(text: &str, signed: bool, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
    let text = text.trim();
    if text.len() > 10000 {
        return Err(CoreError::BudgetExceeded("整数输入最多 10000 字符".into()));
    }
    let (radix, digits) =
        if let Some(s) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            (16, s)
        } else if let Some(s) = text.strip_prefix("0b").or_else(|| text.strip_prefix("0B")) {
            (2, s)
        } else if let Some(s) = text.strip_prefix("0o").or_else(|| text.strip_prefix("0O")) {
            (8, s)
        } else {
            (10, if text.is_empty() { "0" } else { text })
        };
    let n = BigInt::parse_bytes(digits.as_bytes(), radix).ok_or_else(|| err("输入不是合法整数"))?;
    let n = if signed {
        if n.sign() == Sign::Minus {
            -n * 2 - 1
        } else {
            n * 2
        }
    } else {
        n
    };
    let mut n = n
        .to_biguint()
        .ok_or_else(|| err("无符号 VarInt 不支持负数"))?;
    let mut data = Vec::new();
    loop {
        ctx.check_cancel()?;
        let mut byte = (&n & BigUint::from(127u8)).to_u8().unwrap();
        n >>= 7;
        if !n.is_zero() {
            byte |= 128;
        }
        data.push(byte);
        if n.is_zero() {
            break;
        }
    }
    let mut out = out_text(hex::encode(&data));
    out.insert("bytes".into(), PortValue::Bytes(data.into()));
    Ok(out)
}
fn varint_decode(text: &str, signed: bool, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
    let data = hex_input(text)?;
    if data.len() > 5000 {
        return Err(CoreError::BudgetExceeded(
            "VarInt 输入最多 5000 字节".into(),
        ));
    }
    let mut number = BigUint::zero();
    for (i, &byte) in data.iter().enumerate() {
        ctx.check_cancel()?;
        number |= BigUint::from(byte & 127) << (7 * i);
        if byte & 128 == 0 {
            let n: BigInt = number.into();
            let n = if signed {
                if (&n & BigInt::from(1)) == BigInt::from(0) {
                    n / 2u32
                } else {
                    -(n + 1u32) / 2u32
                }
            } else {
                n
            };
            let mut out = out_text(n.to_string());
            out.insert("consumed".into(), PortValue::Number((i + 1) as f64));
            out.insert(
                "remaining".into(),
                PortValue::Bytes(data[i + 1..].to_vec().into()),
            );
            return Ok(out);
        }
    }
    Err(err("LEB128 缺少终止字节"))
}
#[derive(Clone, serde::Deserialize, serde::Serialize)]
struct Magic {
    name: String,
    mime: String,
    ext: String,
    sig: Vec<u8>,
    src: String,
}
fn magic(data: &[u8]) -> Option<Magic> {
    static TABLE: std::sync::OnceLock<Vec<Magic>> = std::sync::OnceLock::new();
    if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        return Some(Magic {
            name: "WebP 图像".into(),
            mime: "image/webp".into(),
            ext: "webp".into(),
            sig: vec![],
            src: "RIFF/WEBP".into(),
        });
    }
    TABLE
        .get_or_init(|| serde_json::from_str(include_str!("data_uri_magic.json")).unwrap())
        .iter()
        .find(|m| data.starts_with(&m.sig))
        .cloned()
}
fn escaped(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| {
            if (32..=126).contains(&b) && b != b'\\' {
                (b as char).to_string()
            } else {
                format!("\\x{b:02x}")
            }
        })
        .collect()
}
fn data_uri_decode(text: &str, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
    let text = text.trim();
    if !text
        .get(..5)
        .is_some_and(|s| s.eq_ignore_ascii_case("data:"))
    {
        return Err(err("需要 data: URI"));
    }
    let (header, payload) = text[5..]
        .split_once(',')
        .ok_or_else(|| err("Data URI 缺少逗号"))?;
    let mut mime = "";
    let mut base64 = false;
    let mut attrs = Vec::new();
    for p in header.split(';').map(str::trim).filter(|p| !p.is_empty()) {
        if p.eq_ignore_ascii_case("base64") {
            base64 = true;
        } else if p.contains('=') || !mime.is_empty() {
            attrs.push(p);
        } else {
            mime = p;
        }
    }
    if mime.is_empty() {
        mime = "text/plain";
    }
    let data = if base64 {
        b64decode(&String::from_utf8_lossy(&percent_decode(payload)))?
    } else {
        percent_decode(payload)
    };
    ctx.check_cancel()?;
    let detected = magic(&data);
    let actual_mime = detected.as_ref().map(|m| m.mime.as_str()).unwrap_or(mime);
    let norm = |s: &str| {
        let s = s.split(';').next().unwrap_or(s).trim().to_lowercase();
        if s == "image/jpg" {
            "image/jpeg".into()
        } else {
            s
        }
    };
    let mismatch = detected.is_some() && norm(mime) != norm(actual_mime);
    let name = format!(
        "datauri.{}",
        detected
            .as_ref()
            .map(|m| m.ext.as_str())
            .unwrap_or(if mime == "image/svg+xml" {
                "svg"
            } else {
                "bin"
            })
    );
    let artifact = crate::artifacts::repository().put(&name, actual_mime, &data)?;
    let mut lines = vec![
        format!("MIME 类型:   {mime}"),
        format!(
            "编码方式:   {}",
            if base64 {
                "base64"
            } else {
                "URL 编码/明文"
            }
        ),
        format!("数据长度:   {} 字节", data.len()),
    ];
    if !attrs.is_empty() {
        lines.push(format!("附加参数:   {}", attrs.join("; ")));
    }
    if let Some(m) = &detected {
        lines.push(format!(
            "文件头识别: {}（{}，依据 {}）",
            m.name, m.mime, m.src
        ));
    }
    if mismatch {
        lines.push(format!(
            "MIME 不符：声明 {mime}，文件头实际为 {actual_mime}"
        ));
    }
    let content = std::str::from_utf8(&data)
        .map(str::to_owned)
        .unwrap_or_else(|_| escaped(&data[..data.len().min(256)]));
    lines.push("--- 内容 ---".into());
    lines.push(content.clone());
    let mut out = out_text(lines.join("\n"));
    out.insert("content".into(), PortValue::Text(content));
    out.insert("metadata".into(), PortValue::Json(json!({ "mime": mime, "base64": base64, "attrs": attrs, "size": data.len(), "detected": detected, "mimeMismatch": mismatch })));
    out.insert("bytes".into(), PortValue::Bytes(data.into()));
    out.insert("artifact".into(), PortValue::Artifact(artifact.id));
    Ok(out)
}
// Match JSON.stringify: IEEE-754 number formatting and integer property keys
// first, while preserving insertion order for the remaining keys. Pure Rust.
fn ecma_json(v: &Value, pretty: bool, depth: usize) -> String {
    let (open, close, items) = match v {
        Value::Number(n) => {
            return n
                .as_f64()
                .filter(|f| f.is_finite())
                .map(|f| ryu_js::Buffer::new().format(f).to_owned())
                .unwrap_or_else(|| "null".into());
        }
        Value::Array(a) => (
            "[",
            "]",
            a.iter()
                .map(|v| ecma_json(v, pretty, depth + 1))
                .collect::<Vec<_>>(),
        ),
        Value::Object(o) => {
            let mut keys: Vec<_> = o.iter().collect();
            keys.sort_by_key(|(k, _)| {
                k.parse::<u32>()
                    .ok()
                    .filter(|n| *n != u32::MAX && n.to_string() == **k)
                    .map(u64::from)
                    .unwrap_or(u64::MAX)
            });
            let items = keys
                .into_iter()
                .map(|(k, v)| {
                    format!(
                        "{}:{}{}",
                        serde_json::to_string(k).expect("string"),
                        if pretty { " " } else { "" },
                        ecma_json(v, pretty, depth + 1)
                    )
                })
                .collect::<Vec<_>>();
            ("{", "}", items)
        }
        _ => return v.to_string(),
    };
    if items.is_empty() {
        return format!("{open}{close}");
    }
    if pretty {
        let pad = "  ".repeat(depth + 1);
        format!(
            "{open}\n{pad}{}\n{}{close}",
            items.join(&format!(",\n{pad}")),
            "  ".repeat(depth)
        )
    } else {
        format!("{open}{}{close}", items.join(","))
    }
}
impl Node for ToolNode {
    fn run(&self, inputs: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.check_cancel()?;
        if matches!(self.0, Tool::DataUriEncode) {
            let data = in_bytes(inputs, "data")?;
            let mime = pstr(p, "mime", "text/plain");
            if mime.contains([',', '\r', '\n']) {
                return Err(err("MIME 不能含逗号或换行"));
            }
            let is_text = matches!(inputs.get("data"), Some(PortValue::Text(_)));
            let payload = if pbool(p, "base64", false) {
                base64::engine::general_purpose::STANDARD.encode(&data)
            } else {
                data.iter()
                    .map(|&b| {
                        if is_text && (b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b)) {
                            (b as char).to_string()
                        } else {
                            format!("%{b:02X}")
                        }
                    })
                    .collect()
            };
            return Ok(out_text(format!(
                "data:{}{},{}",
                if mime.is_empty() { "text/plain" } else { mime },
                if pbool(p, "base64", false) {
                    ";base64"
                } else {
                    ""
                },
                payload
            )));
        }
        let text = in_text(inputs, "text")?;
        match self.0 {
            Tool::Query => parse_query(text),
            Tool::Cookie => parse_cookie(text),
            Tool::AsciiControl => Ok(out_text(ascii_control(text, p)?)),
            Tool::SplitHex => {
                let s: String = text.chars().filter(|c| !c.is_whitespace()).collect();
                if s.is_empty() {
                    return Ok(out_text("（空输入）".into()));
                }
                let chars: Vec<_> = s.chars().collect();
                Ok(out_text(
                    [2, 4, 8]
                        .iter()
                        .map(|&n| {
                            format!(
                                "{n} 位分割：{}",
                                chars
                                    .chunks(n)
                                    .map(|c| c.iter().collect::<String>())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                ))
            }
            Tool::HexReverse => {
                let s: String = text.chars().filter(|c| !c.is_whitespace()).collect();
                let s = s
                    .strip_prefix("0x")
                    .or_else(|| s.strip_prefix("0X"))
                    .unwrap_or(&s);
                if s.is_empty()
                    || !s.len().is_multiple_of(2)
                    || !s.bytes().all(|b| b.is_ascii_hexdigit())
                {
                    return Err(err("需要非空偶数位 Hex"));
                }
                Ok(out_text(
                    s.as_bytes()
                        .chunks(2)
                        .flat_map(|c| [c[1] as char, c[0] as char])
                        .collect(),
                ))
            }
            Tool::VarintEncode => varint_encode(text, pbool(p, "signed", false), ctx),
            Tool::VarintDecode => varint_decode(text, pbool(p, "signed", false), ctx),
            Tool::JsonEncode => {
                let compact = serde_json::from_str::<Value>(text)
                    .map(|v| ecma_json(&v, false, 0))
                    .unwrap_or_else(|_| text.into());
                Ok(out_text(
                    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(compact),
                ))
            }
            Tool::JsonDecode => {
                let bytes = b64decode(text)?;
                let mut out = decoded(bytes.clone());
                if let Ok(v) = serde_json::from_slice::<Value>(&bytes) {
                    out.insert("text".into(), PortValue::Text(ecma_json(&v, true, 0)));
                    out.insert(
                        "json".into(),
                        PortValue::Json(
                            serde_json::from_str(&ecma_json(&v, false, 0))
                                .map_err(|e| err(e.to_string()))?,
                        ),
                    );
                }
                Ok(out)
            }
            Tool::DataUriDecode => data_uri_decode(text, ctx),
            Tool::DataUriEncode => unreachable!(),
        }
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for (tool, id, upstream, name, category, description, sample) in [
        (Tool::Query, "url_query_parse", "urlQueryParse", "URL Query 解析", "data.structure", "解析完整 URL 或 query 字符串，保留同名键顺序；输出报告和键值表。非法百分号转义按上游保留原文。", "https://example.com/?a=1&a=2&name=%E4%B8%AD+%E6%96%87"),
        (Tool::Cookie, "cookie_parse", "cookieParse", "Cookie 解析", "data.structure", "解析 Cookie / Set-Cookie 请求响应头，区分属性和标志；输出报告和键值表。", "Set-Cookie: session=abc; Path=/; HttpOnly"),
        (Tool::AsciiControl, "ascii_control", "asciiControl", "ASCII 控制字符", "encoding.transport", "控制字符名称转 ASCII 值与可视符号，或把字符流内控制字符标注为名称。", "NUL SOH LF DEL"),
        (Tool::SplitHex, "split_hex", "splitHex", "Hex 分组显示", "numeric.bits", "移除空白后按 2、4、8 字符分组显示，保留末尾短组；此工具仅分组，不解释字节。", "123456789abcdef"),
        (Tool::HexReverse, "hex_reverse_encode", "hexReverse", "Hex 半字节互换 编码", "numeric.bits", "每两位 Hex 互换，保留字母大小写；这是自逆变换，输入可含空白和 0x 前缀。", "0x1a2B"),
        (Tool::HexReverse, "hex_reverse_decode", "hexReverse", "Hex 半字节互换 解码", "numeric.bits", "逆转每字节内的两位 Hex；空输入、奇数位和非 Hex 会报错。", "a1B2"),
        (Tool::VarintEncode, "varint_encode", "varint", "VarInt 编码", "numeric.bits", "任意精度非负整数或 ZigZag 有符号整数转 LEB128；输出 Hex 与字节，最多 10000 个输入字符。", "300"),
        (Tool::VarintDecode, "varint_decode", "varint", "VarInt 解码", "numeric.bits", "解析第一个 LEB128 整数，报告已消费字节并保留剩余字节；支持 ZigZag，输入上限 5000 字节。", "ac0201"),
        (Tool::JsonEncode, "b64url_json_encode", "b64urlJson", "JSON → Base64url", "data.serialization", "合法 JSON 按上游 JSON.stringify 规则紧凑化（数值按双精度、整数键排序）；非 JSON 文本原样编码。", "{\"message\":\"你好\",\"n\":1}"),
        (Tool::JsonDecode, "b64url_json_decode", "b64urlJson", "Base64url → JSON", "data.serialization", "解码 Base64url 并美化合法 JSON，同时保留原始字节；不验证签名。", "eyJtZXNzYWdlIjoiaGVsbG8ifQ"),
        (Tool::DataUriEncode, "data_uri_encode", "dataUriParse", "Data URI 构造", "encoding.transport", "按 MIME 将文本或原始字节封装为百分号编码或 Base64 Data URI。", "Hello, 世界"),
        (Tool::DataUriDecode, "data_uri_decode", "dataUriParse", "Data URI 解析", "data.structure", "还原完整字节，解析 MIME/参数，识别文件头和 MIME 不符；输出元数据及可导出的文件产物。", "data:text/plain;charset=utf-8,%E4%B8%AD%E6%96%87"),
    ] {
        let mut params = vec![];
        match tool {
            Tool::AsciiControl => params.push(help(ParamSpec::select("mode", "模式", &["name", "char"], "name"), "name 解析名称列表；char 标注字符流中的控制字符。")),
            Tool::VarintEncode | Tool::VarintDecode => params.push(help(ParamSpec::toggle("signed", "ZigZag 有符号", false), "开启后整数按 ZigZag 映射，再应用无符号 LEB128。")),
            Tool::DataUriEncode => { params.push(help(ParamSpec::text("mime", "MIME 类型", "text/plain", false), "可填写 image/png、application/octet-stream 等类型。")); params.push(help(ParamSpec::toggle("base64", "使用 Base64", false), "关闭使用逐字节百分号编码；不丢失二进制内容。")); },
            _ => {}
        }
        let data_input = matches!(tool, Tool::DataUriEncode);
        let mut outputs = vec![t_out()];
        match tool {
            Tool::Query | Tool::Cookie => outputs.push(req("rows", "键值表", PortType::Json)),
            Tool::VarintEncode => outputs.push(req("bytes", "原始字节", PortType::Bytes)),
            Tool::VarintDecode => { outputs.push(req("consumed", "已消费字节数", PortType::Number)); outputs.push(req("remaining", "剩余字节", PortType::Bytes)); },
            Tool::JsonDecode => { outputs.push(req("bytes", "原始字节", PortType::Bytes)); outputs.push(opt("json", "JSON 对象", PortType::Json)); },
            Tool::DataUriDecode => { outputs.extend([req("content", "内容预览", PortType::Text), req("bytes", "完整字节", PortType::Bytes), req("metadata", "元数据", PortType::Json), req("artifact", "文件产物", PortType::Artifact)]); },
            _ => {}
        }
        let mut d = desc(id, category, name, TEAL, if data_input { vec![req("data", "内容", PortType::Any)] } else { vec![t_in()] }, outputs, params);
        d.description = description.into(); d.upstream_ids = vec![upstream.into()]; d.family = upstream.into(); d.aliases = vec![upstream.into()];
        if matches!(tool, Tool::Query | Tool::Cookie) { d.presentation.previews.insert("rows".into(), "table".into()); }
        d.examples.push(NodeExample { label: "入门样例".into(), inputs: one(if data_input { "data" } else { "text" }, PortValue::Text(sample.into())), params: json!({}) });
        reg.register(d, Arc::new(move || Arc::new(ToolNode(tool))));
    }
}
