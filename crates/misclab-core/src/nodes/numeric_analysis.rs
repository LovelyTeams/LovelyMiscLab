//! Exact, cancellable numeric reports with independently connectable results.
use super::{
    codec_support::{document_text, help},
    numeric_support::{bad, integer, number, space},
    prelude::*,
};
use base64::Engine as _;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};
use serde_json::{json, Value};
#[derive(Clone, Copy)]
enum Kind {
    Prime,
    Collatz,
    Radix,
}
struct Tool(Kind);
fn base62(n: &BigInt, ctx: &NodeCtx) -> Result<String, CoreError> {
    let mut n = n.clone();
    let neg = n.is_negative();
    n = n.abs();
    if n.is_zero() {
        return Ok("0".into());
    }
    let mut chars = Vec::new();
    let alphabet = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    while !n.is_zero() {
        ctx.cancel.check()?;
        let (q, r) = n.div_rem(&BigInt::from(62));
        chars.push(alphabet[r.to_usize().unwrap()] as char);
        n = q;
    }
    if neg {
        chars.push('-');
    }
    Ok(chars.into_iter().rev().collect())
}
fn reads(s: &str) -> Result<Vec<(String, BigInt)>, CoreError> {
    if s.is_empty() || s.len() > 4096 {
        return Err(bad("多进制分析需要 1..4096 个输入字符"));
    }
    let sep = s.chars().any(|c| space(c) || c == '_' || c == ',');
    let joined: String = s
        .chars()
        .filter(|c| !space(*c) && *c != '_' && *c != ',')
        .collect();
    let neg = joined.starts_with('-');
    let s = joined.strip_prefix(['+', '-']).unwrap_or(&joined);
    let s = s.to_ascii_lowercase();
    let (b, s) = if let Some(s) = s.strip_prefix("0x") {
        (16, s)
    } else if let Some(s) = s.strip_prefix("0b") {
        (2, s)
    } else if let Some(s) = s.strip_prefix("0o") {
        (8, s)
    } else {
        (10, s.as_str())
    };
    if s.starts_with(['+', '-']) {
        return Err(bad("数值只能包含一个前导符号"));
    }
    let n = integer(s, b)?;
    let tag = |name: &str| {
        if neg {
            format!("-（负数，按{name}）")
        } else {
            name.into()
        }
    };
    let label = match b {
        2 => "二进制",
        8 => "八进制",
        16 => "十六进制",
        _ => "十进制",
    };
    let n = if neg { -n } else { n };
    let mut result = vec![(tag(label), n.clone())];
    if b == 10 && !sep && s.len() >= 2 && s.bytes().all(|c| c == b'0' || c == b'1') {
        let n2 = integer(s, 2)?;
        let n2 = if neg { -n2 } else { n2 };
        if n2 != n {
            result.push((tag("二进制"), n2));
        }
    }
    Ok(result)
}
fn render(n: &BigInt, label: &str, ctx: &NodeCtx) -> Result<(String, Value), CoreError> {
    let mut lines = vec![
        format!("数值：{n}（按{label}读入）"),
        "  进制对照（数值的 N 进制数字表示）：".into(),
    ];
    let mut row = json!({"source":label,"decimal":n.to_string()});
    for b in [2, 8, 10, 16, 32, 36, 62] {
        let repr = if b == 62 {
            base62(n, ctx)?
        } else {
            n.to_str_radix(b)
        };
        let shown = if b == 32 {
            format!("{repr}（数字表示，非 RFC 4648 Base32 编码）")
        } else {
            repr.clone()
        };
        lines.push(format!("    {b:<2} 进制 : {shown}"));
        row[format!("base{b}")] = json!(repr);
    }
    let abs = n.abs();
    let width = if abs.is_zero() {
        0
    } else {
        abs.to_str_radix(2).len()
    };
    if n.is_negative() {
        lines.push(format!("  补码表示（绝对值位宽 {width} bit）："));
        let mut complements = json!({});
        for bits in [8, 16, 32, 64] {
            let mask = (BigInt::one() << bits) - 1;
            let value: BigInt = n & mask;
            let h = format!(
                "{:0>width$}",
                value.to_str_radix(16).to_uppercase(),
                width = bits / 4
            );
            lines.push(format!("    {bits:2} 位补码 : 0x{h}"));
            complements[bits.to_string()] = json!(h);
        }
        row["complements"] = complements;
        lines.push(
            "  注：负数的 Base64 / 字节 / 字符视图依赖字宽，见上方补码各宽度后自行截取".into(),
        );
    } else {
        let mut bytes = n.to_bytes_be().1;
        if bytes.is_empty() {
            bytes.push(0);
        }
        let h = hex::encode_upper(&bytes);
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        lines.push(format!(
            "  Base64     : {b64}（按最小大端字节 RFC 4648 编码，非 64 进制数字表示）"
        ));
        lines.push("  字节 / 字符视图：".into());
        lines.push(format!("    数值字节 hex（最小大端）  : {h}"));
        row["hex"] = json!(h);
        row["base64"] = json!(b64);
        row["bitWidth"] = json!(width);
        if let Some(ch) = n.to_u32().and_then(char::from_u32) {
            let cp = ch as u32;
            let units: Vec<u16> = ch.to_string().encode_utf16().collect();
            let u16hex = units
                .iter()
                .map(|u| format!("{u:04X}"))
                .collect::<Vec<_>>()
                .join(" ");
            let u8hex = ch
                .to_string()
                .bytes()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" ");
            lines.push(format!(
                "    UTF-16 码元 (BE)         : {u16hex}（{} 个码元）",
                units.len()
            ));
            lines.push(format!(
                "    Unicode 码位             : U+{cp:04X} = \"{ch}\"{}",
                if cp < 32 || (127..=159).contains(&cp) {
                    "（控制字符，显示可能不可见）"
                } else {
                    ""
                }
            ));
            lines.push(format!("    该码位的 UTF-8 编码      : {u8hex}"));
            row["character"] = json!(ch.to_string());
            row["utf8"] = json!(u8hex);
            row["utf16"] = json!(u16hex);
        } else {
            lines.push(format!(
                "    UTF-16 码元 / Unicode 码位: 不适用（{}）",
                if n > &BigInt::from(0x10ffff) {
                    "超出 Unicode 上限 U+10FFFF"
                } else {
                    "落在代理区 U+D800–U+DFFF（非字符）"
                }
            ));
        }
        lines.push(format!("    位宽                     : {width} bit"));
    }
    Ok((lines.join("\n"), row))
}
impl Node for Tool {
    fn run(&self, i: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let s = in_text(i, "text")?.trim_matches(space);
        if s.len() > 4096 {
            return Err(bad("数值分析输入最多 4096 字符"));
        }
        let mut out = PortMap::new();
        let text = match self.0 {
            Kind::Prime => {
                let limit = number(p, "maxTrials", 1_000_000, 1, 100_000_000)? as u64;
                let mut complete = true;
                let mut values = Vec::new();
                let mut rows = Vec::new();
                let mut remainder = String::new();
                let report = if s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit()) {
                    complete = false;
                    "请输入非负整数（仅数字）".into()
                } else {
                    let mut n = integer(s, 10)?;
                    if n < BigInt::from(2) {
                        format!("{s} = {s}（< 2，无分解）")
                    } else {
                        let mut d = BigInt::from(2);
                        let mut iterations = 0;
                        let mut parts = Vec::new();
                        while &d * &d <= n {
                            if iterations % 256 == 0 {
                                ctx.progress(iterations as f32 / limit as f32);
                                ctx.cancel.check()?;
                            }
                            if iterations >= limit {
                                complete = false;
                                break;
                            }
                            iterations += 1;
                            let mut count = 0;
                            while (&n % &d).is_zero() {
                                ctx.cancel.check()?;
                                n /= &d;
                                count += 1;
                                values.push(d.to_string());
                            }
                            if count > 0 {
                                rows.push(json!({"factor":d.to_string(),"exponent":count}));
                                parts.push(if count == 1 {
                                    d.to_string()
                                } else {
                                    format!("{d}^{count}")
                                });
                            }
                            d += if d == BigInt::from(2) { 1 } else { 2 };
                        }
                        if complete && n > BigInt::one() {
                            values.push(n.to_string());
                            rows.push(json!({"factor":n.to_string(),"exponent":1}));
                            parts.push(n.to_string());
                        } else if !complete {
                            remainder = n.to_string();
                            parts.push(format!("({n}，未完成分解)"));
                        }
                        format!("{s} = {}", parts.join(" × "))
                    }
                };
                out.insert("factors".into(), PortValue::StringList(values));
                out.insert("rows".into(), PortValue::Json(json!(rows)));
                out.insert("complete".into(), PortValue::Bool(complete));
                out.insert("remainder".into(), PortValue::Text(remainder));
                report
            }
            Kind::Collatz => {
                let mut n = integer(s, 10)?;
                if n < BigInt::one() {
                    return Err(bad("Collatz 需要正整数"));
                }
                let limit = number(p, "maxSteps", 100000, 1, 100000)? as usize;
                let mut seq = vec![n.to_string()];
                let mut peak = n.clone();
                let mut bytes = seq[0].len();
                let mut steps = 0;
                while !n.is_one() && steps < limit {
                    if steps.is_multiple_of(128) {
                        ctx.progress(steps as f32 / limit as f32);
                    }
                    ctx.cancel.check()?;
                    n = if n.is_even() { n / 2 } else { 3 * n + 1 };
                    if n > peak {
                        peak = n.clone();
                    }
                    let value = n.to_string();
                    bytes += value.len() + 5;
                    if bytes > 16_777_216 {
                        return Err(bad("Collatz 序列超过 16 MiB；请缩小步数上限"));
                    }
                    seq.push(value);
                    steps += 1;
                }
                let complete = n.is_one();
                let tail = if steps >= limit {
                    format!("\n(已达步数上限 {limit})")
                } else {
                    String::new()
                };
                let report = format!(
                    "起始: {}\n步数: {steps}\n峰值: {peak}\n序列({} 项): {}{tail}",
                    seq[0],
                    seq.len(),
                    seq.join(" → ")
                );
                out.insert("sequence".into(), PortValue::StringList(seq));
                out.insert("complete".into(), PortValue::Bool(complete));
                out.insert("steps".into(), PortValue::Number(steps as f64));
                out.insert("peak".into(), PortValue::Text(peak.to_string()));
                report
            }
            Kind::Radix => {
                let readings = reads(s)?;
                let mut lines = Vec::new();
                let mut rows = Vec::new();
                if readings.len() > 1 {
                    lines.push(format!(
                        "输入 \"{s}\" 无法唯一确定进制，列出全部 {} 种解读：",
                        readings.len()
                    ));
                }
                for (i, (label, n)) in readings.iter().enumerate() {
                    ctx.cancel.check()?;
                    let label = if readings.len() == 1 {
                        label
                            .strip_prefix("-（负数，按")
                            .and_then(|s| s.strip_suffix('）'))
                            .unwrap_or(label)
                    } else {
                        label.as_str()
                    };
                    if readings.len() > 1 {
                        lines.extend([
                            String::new(),
                            format!("[解读 {}/{}] 按{label}：", i + 1, readings.len()),
                        ]);
                    }
                    let (report, row) = render(n, label, ctx)?;
                    lines.push(report);
                    rows.push(row);
                }
                out.insert("rows".into(), PortValue::Json(json!(rows)));
                out.insert(
                    "values".into(),
                    PortValue::StringList(readings.iter().map(|(_, n)| n.to_string()).collect()),
                );
                lines.join("\n")
            }
        };
        ctx.cancel.check()?;
        out.insert("text".into(), PortValue::Text(text));
        Ok(out)
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for(kind,id,upstream,title,sample,description)in[
 (Kind::Prime,"prime_factorization","primeFactor","精确试除分解","360","非负大整数试除分解，输出因子与指数表。达到尝试上限时 complete=false，并明确输出未分解余数；不把余数误标为素数。"),
 (Kind::Collatz,"collatz_sequence","collatz","Collatz 序列","27","正整数按偶数除 2、奇数乘 3 加 1 生成序列，记录步数、峰值、完成状态；默认上限 100000，支持进度和取消。"),
 (Kind::Radix,"radix_all","radixAll","一键多进制分析","1010","自动识别前缀；裸 01 串列出十进制/二进制两种解释；给出 2/8/10/16/32/36/62、Base64、字节与 Unicode、负数补码。") ]{
 let mut outputs=vec![t_out()];let params=match kind{
 Kind::Prime=>{outputs.extend([req("factors","因子列表",PortType::StringList),req("rows","指数明细",PortType::Json),req("complete","分解完成",PortType::Bool),req("remainder","待分解余数",PortType::Text)]);vec![help(ParamSpec::number("maxTrials","试除上限",1.0,100000000.0,1.0,1000000.0),"达到上限时输出未分解余数与 complete=false，可增大上限重试。")]},
 Kind::Collatz=>{outputs.extend([req("sequence","完整序列",PortType::StringList),req("complete","到达 1",PortType::Bool),req("steps","步数",PortType::Number),req("peak","峰值",PortType::Text)]);vec![help(ParamSpec::number("maxSteps","步数上限",1.0,100000.0,1.0,100000.0),"上游固定上限为 100000；可降低，序列还有 16 MiB 保护上限。")]},
 Kind::Radix=>{outputs.extend([req("rows","进制明细",PortType::Json),req("values","十进制候选",PortType::StringList)]);vec![]}};
 let mut d=document_text(desc(id,if matches!(kind,Kind::Radix){"numeric.radix"}else{"numeric.math"},title,CYAN,vec![t_in()],outputs,params),upstream,description,sample,json!({}));d.cost=Cost::Heavy;if !matches!(kind,Kind::Collatz){d.presentation.previews.insert("rows".into(),"table".into());}reg.register(d,Arc::new(move||Arc::new(Tool(kind))));
}
}
