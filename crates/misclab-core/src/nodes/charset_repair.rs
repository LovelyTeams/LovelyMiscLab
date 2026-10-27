//! GB2312 grid notation and reversible, explicitly configured mojibake steps.
use super::{
    charset_codecs as cc,
    codec_support::{document_text, help},
    numeric_support::bad,
    prelude::*,
};
use serde_json::{json, Value};
fn split_ascii(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let mut end = i + 1;
            while end < chars.len() && chars[end].is_ascii_digit() {
                end += 1;
            }
            if end - i >= 4 {
                if !buf.is_empty() {
                    out.push(std::mem::take(&mut buf));
                }
                out.extend(chars[i..end].iter().map(char::to_string));
            } else {
                buf.extend(&chars[i..end]);
            }
            i = end;
        } else {
            buf.push(chars[i]);
            i += 1;
        }
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out
}
fn quwei_encode(
    s: &str,
    sep: &str,
    legacy: bool,
    ctx: &mut NodeCtx,
) -> Result<(String, bool), CoreError> {
    let (bytes, fallback) = cc::encode("gb2312", s, legacy, ctx)?;
    let mut groups: Vec<String> = Vec::new();
    let (mut i, mut digits, mut auto_newline, mut last_han) = (0, 0, false, false);
    while i < bytes.len() {
        if i % 1024 < 2 {
            ctx.cancel.check()?;
        }
        let b = bytes[i];
        if (0xa1..=0xfe).contains(&b) && bytes.get(i + 1).is_some_and(|b| (0xa1..=0xfe).contains(b))
        {
            groups.push(format!("{:02}{:02}", b - 0xa0, bytes[i + 1] - 0xa0));
            if sep == "none" && digits > 0 {
                auto_newline = true;
            }
            digits = 0;
            last_han = true;
            i += 2;
        } else {
            if !legacy && !b.is_ascii() {
                return Err(bad("字符不能用 GB2312 94×94 区位网格表示"));
            }
            if b.is_ascii_digit() {
                if sep == "none" && last_han {
                    auto_newline = true;
                }
                digits += 1;
                if sep == "none" && digits >= 4 {
                    auto_newline = true;
                }
            } else {
                digits = 0;
            }
            last_han = false;
            groups.push((b as char).to_string());
            i += 1;
        }
    }
    if sep == "none" && auto_newline {
        let mut merged = Vec::new();
        let mut buf = String::new();
        for g in groups {
            if g.chars().count() == 1 {
                buf.push_str(&g);
            } else {
                if !buf.is_empty() {
                    merged.extend(split_ascii(&buf));
                    buf.clear();
                }
                merged.push(g);
            }
        }
        if !buf.is_empty() {
            merged.extend(split_ascii(&buf));
        }
        return Ok((merged.join("\n"), fallback));
    }
    Ok((
        groups.join(match sep {
            "space" => " ",
            "newline" => "\n",
            _ => "",
        }),
        fallback,
    ))
}
fn quwei_decode(
    s: &str,
    sep: &str,
    strict: bool,
    ctx: &mut NodeCtx,
) -> Result<(String, bool), CoreError> {
    let chars: Vec<char> = s.chars().collect();
    let mut bytes = Vec::new();
    let (mut i, mut lossy) = (0, false);
    while i < chars.len() {
        if i % 1024 < 4 {
            ctx.progress(i as f32 / chars.len().max(1) as f32);
            ctx.cancel.check()?;
        }
        if chars
            .get(i..i + 4)
            .is_some_and(|g| g.iter().all(char::is_ascii_digit))
        {
            let q = (chars[i] as u8 - b'0') * 10 + chars[i + 1] as u8 - b'0';
            let w = (chars[i + 2] as u8 - b'0') * 10 + chars[i + 3] as u8 - b'0';
            if (1..=94).contains(&q) && (1..=94).contains(&w) {
                bytes.extend([q + 0xa0, w + 0xa0]);
            } else if strict {
                return Err(bad("区位的两部分均须为01..94"));
            } else {
                bytes.push(b'?');
                lossy = true;
            }
            i += 4;
        } else {
            let c = chars[i];
            let sep_char = if sep == "space" {
                c == ' '
            } else {
                c == '\r' || c == '\n'
            };
            if c.is_ascii() {
                if !sep_char {
                    bytes.push(c as u8);
                }
            } else if strict {
                return Err(bad("区位码仅接受数字与 ASCII 透传字符"));
            } else {
                lossy = true;
            }
            i += 1;
        }
    }
    let (out, errors) = cc::decode("gb2312", &bytes, strict)?;
    Ok((out, lossy || errors))
}
struct Repair(bool, bool);
impl Node for Repair {
    fn run(&self, i: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let source = cc::input(i)?;
        let s = cc::text(&source)?;
        let legacy =
            cc::choice(p, "codecMode", "standard", &["standard", "upstream"])? == "upstream";
        let strict = pbool(p, "strict", false);
        let (out, lossy) = if self.0 {
            let sep = cc::choice(p, "sep", "none", &["none", "space", "newline"])?;
            if self.1 {
                quwei_decode(s, sep, strict, ctx)?
            } else {
                quwei_encode(s, sep, legacy, ctx)?
            }
        } else {
            let preset = pstr(p, "preset", "utf8_as_cp1252");
            let (wrong, right) = if preset == "custom" {
                (pstr(p, "wrong", "windows-1252"), pstr(p, "right", "utf-8"))
            } else {
                let cfg = &cc::TABLES["presets"][preset];
                (
                    cfg["wrong"].as_str().ok_or_else(|| bad("乱码预设无效"))?,
                    cfg["right"].as_str().unwrap(),
                )
            };
            // Encoding and decoding are distinct steps; losses are never hidden.
            let (from, to) = if self.1 {
                (wrong, right)
            } else {
                (right, wrong)
            };
            let (bytes, fallback) = cc::encode(from, s, legacy, ctx)?;
            let (out, errors) = cc::decode(to, &bytes, strict)?;
            (out, fallback || errors)
        };
        ctx.progress(1.0);
        ctx.cancel.check()?;
        Ok(cc::outputs(out.clone(), out.into_bytes(), source, lossy))
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for quwei in [true, false] {
        for decode in [false, true] {
            let (id, upstream, name) = if quwei {
                ("gb2312_quwei", "gb2312QuWei", "GB2312 区位码")
            } else {
                ("mojibake", "mojibakeFix", "乱码")
            };
            let mut params = if quwei {
                vec![help(ParamSpec::select("sep","分组分隔",&["none","space","newline"],"none"),"无分隔时遇到长数字或汉字紧邻数字会自动换行；解码忽略所选分隔字符，原文含相同分隔字符时可能丢失。")]
            } else {
                vec![
                    help(ParamSpec::select("preset","错配类型",&["utf8_as_cp1252","utf8_as_latin1","utf8_as_gbk","gbk_as_latin1","gbk_as_utf8","big5_as_gbk","gbk_as_big5","shiftjis_as_latin1","euckr_as_latin1","custom"],"utf8_as_cp1252"),"修复=按错误字符集重新编码，再按正确字符集解码；制造方向相反。已经产生替换符的乱码通常不可逆。"),
                    help(ParamSpec::text("wrong","错误字符集","windows-1252",false),"custom 时使用，接受 WHATWG 字符集标签。"),
                    help(ParamSpec::text("right","正确字符集","utf-8",false),"custom 时使用，如 utf-8、gbk、big5、shift_jis。"),
                ]
            };
            if !quwei || !decode {
                params.push(cc::mode_param());
            }
            if !quwei || decode {
                params.push(cc::strict_param());
            }
            let suffix = if decode { "decode" } else { "encode" };
            let action = if quwei {
                if decode {
                    "解码"
                } else {
                    "编码"
                }
            } else if decode {
                "修复"
            } else {
                "制造"
            };
            let mut d = document_text(
                desc(
                    &format!("{id}_{suffix}"),
                    "encoding.charset",
                    &format!("{name} {action}"),
                    TEAL,
                    vec![req("data", "文本或 UTF-8 字节", PortType::Any)],
                    cc::ports(),
                    params,
                ),
                upstream,
                if quwei {
                    "GB2312 汉字转换为四位区位码，ASCII透传；默认拒绝网格外字符。保留上游分隔策略，原始文本字节另行输出。"
                } else {
                    "通过明确的错误/正确字符集还原或制造乱码；不猜测已经丢失的字符。lossy标记替换或兼容回退，原始输入始终保留。"
                },
                if quwei {
                    if decode {
                        "54484644"
                    } else {
                        "中文"
                    }
                } else if decode {
                    "cafÃ©"
                } else {
                    "café"
                },
                json!({}),
            );
            d.cost = Cost::Heavy;
            if !quwei {
                d.presentation.inline_params =
                    vec!["preset".into(), "codecMode".into(), "strict".into()];
            }
            reg.register(d, Arc::new(move || Arc::new(Repair(quwei, decode))));
        }
    }
}
