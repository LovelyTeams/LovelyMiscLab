//! Text/byte notations from the pinned textExt contract. No JS evaluation.
use super::{
    basex::decoded,
    codec_support::{document_text, help},
    prelude::*,
};
use serde_json::{json, Value};

const LEET: &[(char, &str)] = &[
    ('a', "4"),
    ('b', "8"),
    ('c', "("),
    ('d', "|)"),
    ('e', "3"),
    ('g', "6"),
    ('h', "#"),
    ('i', "1"),
    ('j', "_|"),
    ('k', "|<"),
    ('l', "1"),
    ('m', "/\\/\\"),
    ('n', "|\\|"),
    ('o', "0"),
    ('p', "|D"),
    ('q', "9"),
    ('r', "|2"),
    ('s', "5"),
    ('t', "7"),
    ('u', "|_|"),
    ('v', "\\/"),
    ('w', "\\/\\/"),
    ('x', "><"),
    ('y', "`/"),
    ('z', "2"),
];
const NATO: &[&str] = &[
    "Alpha", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot", "Golf", "Hotel", "India", "Juliet",
    "Kilo", "Lima", "Mike", "November", "Oscar", "Papa", "Quebec", "Romeo", "Sierra", "Tango",
    "Uniform", "Victor", "Whiskey", "X-ray", "Yankee", "Zulu", "Zero", "One", "Two", "Three",
    "Four", "Five", "Six", "Seven", "Eight", "Niner", "(space)",
];
const NATO_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 ";

fn ecma_space(c: char) -> bool {
    (c.is_whitespace() && c != '\u{85}') || c == '\u{feff}'
}

#[derive(Clone, Copy)]
enum Kind {
    Js,
    Leet,
    Nato,
}
struct Codec {
    kind: Kind,
    decode: bool,
}
impl Node for Codec {
    fn run(
        &self,
        inputs: &PortMap,
        params: &Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        if matches!(self.kind, Kind::Js) {
            if !self.decode {
                let all = pstr(params, "mode", "nonascii") == "all";
                let mut out = String::new();
                for (i, b) in in_bytes(inputs, "data")?.into_iter().enumerate() {
                    if i % 4096 == 0 {
                        ctx.cancel.check()?;
                    }
                    if !all && (32..=126).contains(&b) && b != b'\\' {
                        out.push(b as char);
                    } else {
                        out.push_str(&format!("\\x{b:02x}"));
                    }
                }
                return Ok(out_text(out));
            }
            let text = in_text(inputs, "text")?;
            let mut bytes = Vec::new();
            let mut i = 0;
            while i < text.len() {
                if i % 4096 == 0 {
                    ctx.cancel.check()?;
                }
                let tail = &text[i..];
                if tail.starts_with("\\x") {
                    if let Some(pair) = tail.as_bytes().get(2..4) {
                        if let (Some(h), Some(l)) = (
                            (pair[0] as char).to_digit(16),
                            (pair[1] as char).to_digit(16),
                        ) {
                            bytes.push((h * 16 + l) as u8);
                            i += 4;
                            continue;
                        }
                    }
                    if pbool(params, "strict", false) {
                        return Err(CoreError::Parse(format!(
                            "JS Hex：偏移 {i} 的 \\x 后需两位 Hex"
                        )));
                    }
                }
                let c = tail.chars().next().unwrap();
                if pstr(params, "literal", "UTF-16 低字节") == "UTF-8" {
                    bytes.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
                } else {
                    bytes.extend(c.encode_utf16(&mut [0; 2]).iter().map(|u| *u as u8));
                }
                i += c.len_utf8();
            }
            return Ok(decoded(bytes));
        }
        let text = in_text(inputs, "text")?;
        if matches!(self.kind, Kind::Nato) {
            let out = if self.decode {
                text.split(|c: char| ecma_space(c) || c == ',' || c == ';')
                    .filter(|s| !s.is_empty())
                    .map(|token| {
                        NATO.iter()
                            .position(|v| v.eq_ignore_ascii_case(token))
                            .map(|i| (NATO_CHARS.as_bytes()[i] as char).to_string())
                            .unwrap_or_else(|| token.to_string())
                    })
                    .collect::<String>()
            } else {
                text.chars()
                    .map(|c| {
                        let upper: String = c.to_uppercase().collect();
                        if upper.len() == 1 {
                            if let Some(i) = NATO_CHARS.find(&upper) {
                                return NATO[i].to_string();
                            }
                        }
                        c.to_string()
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            return Ok(out_text(out));
        }
        let mut out = String::new();
        if self.decode {
            let mut tokens = LEET.to_vec();
            tokens.sort_by_key(|(_, token)| std::cmp::Reverse(token.len()));
            let mut tail = text;
            while !tail.is_empty() {
                ctx.cancel.check()?;
                if let Some((letter, token)) =
                    tokens.iter().find(|(_, token)| tail.starts_with(token))
                {
                    out.push(*letter);
                    tail = &tail[token.len()..];
                } else {
                    let c = tail.chars().next().unwrap();
                    out.push(c);
                    tail = &tail[c.len_utf8()..];
                }
            }
        } else {
            for (i, c) in text.chars().enumerate() {
                if i % 4096 == 0 {
                    ctx.cancel.check()?;
                }
                if let Some((_, token)) = LEET
                    .iter()
                    .find(|(letter, _)| *letter == c.to_ascii_lowercase())
                {
                    out.push_str(token);
                } else {
                    out.push(c);
                }
            }
        }
        Ok(out_text(out))
    }
}

fn prefix(bytes: &[u8]) -> Option<u32> {
    if bytes.first() != Some(&b'0') {
        return None;
    }
    match bytes.get(1)?.to_ascii_lowercase() {
        b'x' => Some(16),
        b'b' => Some(2),
        b'o' => Some(8),
        b'd' => Some(10),
        _ => None,
    }
}
fn packed(token: &str) -> Vec<&str> {
    // Choose the earliest next prefix after at least one valid digit, matching
    // upstream lazy-prefix splitting; fall back if it cannot cover the token.
    if !token.is_ascii() || prefix(token.as_bytes()).is_none() {
        return vec![token];
    }
    let mut out = Vec::new();
    let mut rest = token;
    while !rest.is_empty() {
        let Some(base) = prefix(rest.as_bytes()) else {
            return vec![token];
        };
        let mut end = rest.len();
        for i in 2..rest.len() {
            if i > 2 && prefix(&rest.as_bytes()[i..]).is_some() {
                end = i;
                break;
            }
            if (rest.as_bytes()[i] as char).to_digit(base).is_none() {
                return vec![token];
            }
        }
        if end <= 2 {
            return vec![token];
        }
        out.push(&rest[..end]);
        rest = &rest[end..];
    }
    out
}
struct Mixed;
impl Node for Mixed {
    fn run(
        &self,
        inputs: &PortMap,
        params: &Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(inputs, "text")?;
        let strict = pbool(params, "strict", true);
        let mut rows = Vec::new();
        let mut errors = Vec::new();
        let mut lines = Vec::new();
        let mut result = String::new();
        for token in text
            .split(|c: char| ecma_space(c) || c == ',' || c == ';')
            .filter(|s| !s.is_empty())
            .flat_map(packed)
        {
            ctx.cancel.check()?;
            let (base, digits) = match prefix(token.as_bytes()) {
                Some(base) => (base, &token[2..]),
                None if token.bytes().all(|b| b.is_ascii_digit()) => (10, token),
                _ => {
                    errors.push(token.to_string());
                    continue;
                }
            };
            let mut count = 0;
            let mut n = Some(0_u64);
            for c in digits.chars() {
                let Some(d) = c.to_digit(base) else {
                    break;
                };
                count += c.len_utf8();
                n = n.and_then(|v| v.checked_mul(base as u64)?.checked_add(d as u64));
            }
            if count == 0 || n.is_none() || (strict && count != digits.len()) {
                errors.push(token.to_string());
                continue;
            }
            let n = n.unwrap();
            let ch = u32::try_from(n)
                .ok()
                .and_then(char::from_u32)
                .unwrap_or('?');
            result.push(ch);
            rows.push(json!({"token":token,"base":base,"value":n,"character":ch.to_string()}));
            lines.push(format!("{token}\t({base} 进制 = {n})\t→ {ch}"));
        }
        let mut report = if lines.is_empty() {
            String::new()
        } else {
            format!("逐字符解码:\n{}\n\n拼接结果: {result}", lines.join("\n"))
        };
        if !errors.is_empty() {
            if !report.is_empty() {
                report.push('\n');
            }
            report.push_str(&format!("无法识别 token: {}", errors.join(" ")));
        }
        if report.is_empty() {
            report = "（空输入）".into();
        }
        let mut out = out_text(report);
        out.insert("decoded".into(), PortValue::Text(result));
        out.insert("rows".into(), PortValue::Json(Value::Array(rows)));
        out.insert("errors".into(), PortValue::StringList(errors));
        Ok(out)
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for (kind, id, upstream, title, category, description, sample) in [
        (
            Kind::Js,
            "js_hex",
            "jsHex",
            "JS Hex 转义",
            "encoding.transport",
            "字节级 \\xXX 转义；不会执行 JavaScript。反斜杠也转义，解码始终提供完整 bytes。",
            "\\x48\\x69\\x21",
        ),
        (
            Kind::Leet,
            "leet_speak",
            "leetSpeak",
            "Leet Speak",
            "encoding.fancy",
            "完整 1337 符号表，长符号优先匹配。解码为小写，1 取 i；大小写与 i/l 歧义不能恢复。",
            "#3110 /\\/\\|\\|",
        ),
        (
            Kind::Nato,
            "nato_alphabet",
            "natoAlphabet",
            "NATO 音标字母",
            "encoding.fancy",
            "字母与数字音标；空格为 (space)，未知标点保留。解码不区分音标大小写，结果字母为大写。",
            "Alpha (space) Juliet X-ray Niner",
        ),
    ] {
        for decode in [false, true] {
            let ps = if matches!(kind, Kind::Js) {
                if decode {
                    vec![
                    help(ParamSpec::select("literal", "未转义字符", &["UTF-16 低字节", "UTF-8"], "UTF-16 低字节"), "低字节保持上游 JS 字符串处理语义；UTF-8 可保留原文中未转义的 Unicode。"),
                    help(ParamSpec::toggle("strict", "严格转义校验", false), "开启时拒绝不完整的 \\xXX，关闭保留字面字符。"),
                ]
                } else {
                    vec![help(
                        ParamSpec::select("mode", "范围", &["nonascii", "all"], "nonascii"),
                        "nonascii 仅转义非可打印字节与反斜杠；all 转义全部字节。",
                    )]
                }
            } else {
                vec![]
            };
            let binary_input = matches!(kind, Kind::Js) && !decode;
            let mut outputs = vec![t_out()];
            if matches!(kind, Kind::Js) && decode {
                outputs.push(req("bytes", "完整字节", PortType::Bytes));
            }
            let d = document_text(
                desc(
                    &format!("{id}_{}", if decode { "decode" } else { "encode" }),
                    category,
                    &format!("{title} {}", if decode { "解码" } else { "编码" }),
                    BLUE,
                    if binary_input {
                        vec![req("data", "输入", PortType::Any)]
                    } else {
                        vec![t_in()]
                    },
                    outputs,
                    ps,
                ),
                upstream,
                description,
                if decode { sample } else { "Hello, 中文!" },
                json!({}),
            );
            reg.register(d, Arc::new(move || Arc::new(Codec { kind, decode })));
        }
    }
    let mut d = document_text(desc("mixed_radix_decode", "numeric.radix", "混排进制解码", CYAN, vec![t_in()], vec![t_out(), req("decoded", "拼接字符", PortType::Text), req("rows", "逐字符明细", PortType::Json), req("errors", "无法识别项", PortType::StringList)], vec![help(ParamSpec::toggle("strict", "完整数字校验", true), "开启要求 token 全部数字有效；关闭兼容 parseInt 式的有效前缀读取。错误 token 另行输出，不混入拼接字符。")]), "mixHexOctBin", "解码 0x/0b/0o/0d 混排或连写的 Unicode 码点，支持逗号/分号/空白分隔；输出报告、结构化明细和可接续的拼接文本。", "0x480o151;0b100001", json!({}));
    d.presentation
        .previews
        .insert("rows".into(), "table".into());
    reg.register(d, Arc::new(|| Arc::new(Mixed)));
}
