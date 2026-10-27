//! UTF-16 escapes and Unicode scalar notation. Never silently drop surrogates.
use super::{
    codec_support::{document_text, help},
    prelude::*,
};
use serde_json::json;

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let all = pstr(params, "mode", "仅非ASCII") == "全部";
        let fmt = pstr(params, "fmt", "uXXXX");
        if !["uXXXX", "U+", "hex"].contains(&fmt) {
            return Err(CoreError::Parse("Unicode 格式无效".into()));
        }
        let upper = pbool(params, "upper", false);
        let mut out = String::new();
        let mut buf = [0u16; 2];
        for (i, c) in in_text(inputs, "text")?.chars().enumerate() {
            if i % 4096 == 0 {
                ctx.cancel.check()?;
            }
            if !all && c.is_ascii() {
                out.push(c);
                continue;
            }
            let hex = |n: u32, width: usize| {
                if upper {
                    format!("{n:0width$X}")
                } else {
                    format!("{n:0width$x}")
                }
            };
            match fmt {
                "uXXXX" => {
                    for u in c.encode_utf16(&mut buf) {
                        out.push_str(&format!("\\u{}", hex(*u as u32, 4)));
                    }
                }
                "U+" => out.push_str(&format!("U+{}", hex(c as u32, 4))),
                _ => out.push_str(&format!("&#x{};", hex(c as u32, 0))),
            }
        }
        Ok(out_text(out))
    }
}

fn unicode_decode(text: &str) -> Result<String, CoreError> {
    let re = regex::Regex::new(r"\\u([dD][89aAbB][0-9a-fA-F]{2})\\u([dD][c-fC-F][0-9a-fA-F]{2})|\\u\{([0-9a-fA-F]+)\}|\\u([0-9a-fA-F]{4})").unwrap();
    let mut error = false;
    let first = re.replace_all(text, |caps: &regex::Captures| {
        if let Some(hi) = caps.get(1) {
            let h = u16::from_str_radix(hi.as_str(), 16).unwrap();
            let l = u16::from_str_radix(&caps[2], 16).unwrap();
            return String::from_utf16(&[h, l]).expect("regex matched a valid surrogate pair");
        }
        let hex = caps.get(3).or_else(|| caps.get(4)).unwrap().as_str();
        u32::from_str_radix(hex, 16)
            .ok()
            .and_then(char::from_u32)
            .map(|c| c.to_string())
            .unwrap_or_else(|| {
                error = true;
                caps[0].to_string()
            })
    });
    if error {
        return Err(CoreError::Parse(
            "Unicode：孤立代理项或无效码点，必须提供完整代理对".into(),
        ));
    }
    let mut out = first.into_owned();
    for pattern in [r"U\+([0-9a-fA-F]{4,6})", r"&#x([0-9a-fA-F]+);"] {
        out = regex::Regex::new(pattern)
            .unwrap()
            .replace_all(&out, |caps: &regex::Captures| {
                u32::from_str_radix(&caps[1], 16)
                    .ok()
                    .and_then(char::from_u32)
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| caps[0].to_string())
            })
            .into_owned();
    }
    Ok(out)
}

struct Dec;
impl Node for Dec {
    fn run(
        &self,
        inputs: &PortMap,
        _p: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        Ok(out_text(unicode_decode(in_text(inputs, "text")?)?))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        document_text(
            desc(
                "unicode_escape",
                ENC,
                "Unicode 转义",
                BLUE,
                vec![t_in()],
                vec![t_out()],
                vec![
                    help(
                        ParamSpec::select("mode", "范围", &["仅非ASCII", "全部"], "仅非ASCII"),
                        "旧默认只转非 ASCII；全部用于逐字符转义。",
                    ),
                    help(
                        ParamSpec::select("fmt", "格式", &["uXXXX", "U+", "hex"], "uXXXX"),
                        "uXXXX 使用 UTF-16 代理对；U+ 和 hex 使用完整 Unicode 码点。",
                    ),
                    help(
                        ParamSpec::toggle("upper", "大写 Hex", false),
                        "关闭保留旧默认小写；上游编码使用大写。",
                    ),
                ],
            ),
            "unicodeEscape",
            "支持 \\uXXXX（含代理对）、U+XXXX 与 HTML 十六进制实体。",
            "中文🙂",
            json!({"mode":"全部","upper":true}),
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(document_text(desc("unicode_unescape", ENC, "Unicode 反转义", BLUE, vec![t_in()], vec![t_out()], vec![]), "unicodeEscape", "还原 Unicode 转义、完整代理对、U+ 与十六进制实体；兼容旧 \\u{...} 写法，拒绝孤立代理项。", "\\u4E2D\\u6587\\uD83D\\uDE42", json!({})), Arc::new(|| Arc::new(Dec)));
}
