//! Numeric reports expose structured outputs for downstream nodes.
use super::{
    codec_support::{document_text, help},
    numeric_support::*,
    prelude::*,
};
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};
use serde_json::{json, Value};

#[derive(Clone, Copy)]
enum Kind {
    Separate,
    Offset(bool),
    Complement,
    ComplementRead,
    Gcd,
    Fibonacci,
}
struct Report(Kind);
fn fibs() -> Vec<BigInt> {
    let mut out = vec![BigInt::one(), BigInt::one()];
    for i in 2..128 {
        out.push(&out[i - 1] + &out[i - 2]);
    }
    out
}
fn shifted(s: &str, offset: i64) -> (String, Vec<u8>) {
    let units: Vec<u16> = s
        .chars()
        .map(|c| (i64::from(c.encode_utf16(&mut [0; 2])[0]) + offset).rem_euclid(65536) as u16)
        .collect();
    (
        String::from_utf16_lossy(&units),
        units.into_iter().flat_map(u16::to_le_bytes).collect(),
    )
}
impl Node for Report {
    fn run(&self, inputs: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let text = in_text(inputs, "text")?;
        if text.len() > 1_048_576 {
            return Err(bad("数值报告输入最多 1 MiB"));
        }
        let mut out = PortMap::new();
        let report = match self.0 {
            Kind::Separate => {
                let s = compact(text);
                let mut rows = Vec::new();
                let mut lines = Vec::new();
                if s.is_empty() {
                    "（空输入）".into()
                } else {
                    if s.bytes().all(|b| b.is_ascii_digit()) && s.len() <= MAX_DIGITS {
                        let n = integer(&s, 10)?;
                        let bytes = if n.is_zero() {
                            Vec::new()
                        } else {
                            n.to_bytes_be().1
                        };
                        if !bytes.is_empty() && bytes.iter().all(|b| (32..=126).contains(b)) {
                            let s = String::from_utf8(bytes).unwrap();
                            lines.push(format!("整数→字节: {s}"));
                            rows.push(json!({"method":"整数→字节","text":s}));
                        }
                    }
                    for base in [10, 16, 8, 2] {
                        ctx.cancel.check()?;
                        if !s.chars().all(|c| c.is_digit(base)) {
                            continue;
                        }
                        let (mut current, mut result, mut consumed) = (0_u32, String::new(), true);
                        for c in s.chars() {
                            let Some(n) = current
                                .checked_mul(base)
                                .and_then(|v| v.checked_add(c.to_digit(base).unwrap()))
                            else {
                                consumed = false;
                                break;
                            };
                            current = n;
                            if current > 126 {
                                consumed = false;
                                break;
                            }
                            if current >= 32 {
                                result.push(char::from_u32(current).unwrap());
                                current = 0;
                                consumed = true;
                            } else {
                                consumed = false;
                            }
                        }
                        if consumed && !result.is_empty() {
                            lines.push(format!("{base} 进制: {result}"));
                            rows.push(
                                json!({"method":format!("{base} 进制"),"base":base,"text":result}),
                            );
                        }
                    }
                    out.insert(
                        "results".into(),
                        PortValue::StringList(
                            rows.iter()
                                .map(|r| r["text"].as_str().unwrap().to_owned())
                                .collect(),
                        ),
                    );
                    out.insert("rows".into(), PortValue::Json(json!(rows)));
                    if lines.is_empty() {
                        "（无可行分割）".into()
                    } else {
                        lines.join("\n")
                    }
                }
            }
            Kind::Offset(decode) => {
                let offset = number(p, "offset", 0, -65535, 65535)?;
                if offset != 0 {
                    let (s, bytes) = shifted(text, if decode { -offset } else { offset });
                    out.insert("bytes".into(), PortValue::Bytes(bytes.into()));
                    s
                } else {
                    let mut rows = Vec::new();
                    let mut lines = Vec::new();
                    let mut results = Vec::new();
                    for off in -26..=26 {
                        if off == 0 {
                            continue;
                        }
                        ctx.progress((off + 26) as f32 / 52.0);
                        ctx.cancel.check()?;
                        let (s, bytes) = shifted(text, off);
                        lines.push(format!("{off}\t{s}"));
                        results.push(s.clone());
                        rows.push(json!({"offset":off,"text":s,"utf16le":hex::encode(bytes)}));
                    }
                    out.insert("rows".into(), PortValue::Json(json!(rows)));
                    out.insert("results".into(), PortValue::StringList(results));
                    lines.join("\n")
                }
            }
            Kind::Complement => {
                let n = integer(text, 10)?
                    .to_i32()
                    .ok_or_else(|| bad("整数必须位于有符号 32 位范围"))?;
                let bits = if (-128..=127).contains(&n) {
                    8
                } else if (-32768..=32767).contains(&n) {
                    16
                } else {
                    32
                };
                let magnitude = i64::from(n).unsigned_abs();
                let magnitude_fits = magnitude < (1_u64 << (bits - 1));
                let mag = format!("{magnitude:0width$b}", width = bits - 1);
                let original = if magnitude_fits {
                    format!("{}{mag}", if n < 0 { '1' } else { '0' })
                } else {
                    "无法表示（超出原码范围）".into()
                };
                let inverse = if !magnitude_fits {
                    "无法表示（超出反码范围）".into()
                } else if n < 0 {
                    format!(
                        "1{}",
                        mag.chars()
                            .map(|c| if c == '0' { '1' } else { '0' })
                            .collect::<String>()
                    )
                } else {
                    original.clone()
                };
                let raw = (i64::from(n) as u64) & ((1_u64 << bits) - 1);
                let comp = format!("{raw:0bits$b}");
                out.insert("original".into(), PortValue::Text(original.clone()));
                out.insert("inverse".into(), PortValue::Text(inverse.clone()));
                out.insert("complement".into(), PortValue::Text(comp.clone()));
                out.insert("bits".into(), PortValue::Number(bits as f64));
                format!("10 进制数：{n}\n位宽：{bits} bit\n原码：{original}\n反码：{inverse}\n补码：{comp}")
            }
            Kind::ComplementRead => {
                let mut s = compact(text);
                if s.is_empty() || !s.bytes().all(|b| b == b'0' || b == b'1') {
                    return Err(bad("需要仅含 0/1 的二进制位串"));
                }
                let bits = number(p, "bits", 0, 0, 64)? as usize;
                let bits = if bits == 0 { s.len() } else { bits };
                if bits > 64 || s.len() > bits {
                    return Err(bad("位串超过指定位宽或 64 位上限"));
                }
                s = format!("{s:0>bits$}");
                let raw = integer(&s, 2)?;
                let sign = s.starts_with('1');
                let half = BigInt::one() << (bits - 1);
                let full = &half * 2;
                let mag = &raw % &half;
                let original = if sign {
                    format!("-{mag}")
                } else {
                    raw.to_string()
                };
                let inverse = if sign {
                    format!("-{}", &half - 1 - &mag)
                } else {
                    raw.to_string()
                };
                let comp: BigInt = if sign { &raw - full } else { raw.clone() };
                let comp = comp.to_string();
                out.insert("original".into(), PortValue::Text(original.clone()));
                out.insert("inverse".into(), PortValue::Text(inverse.clone()));
                out.insert("complement".into(), PortValue::Text(comp.clone()));
                format!("位串: {s}\n位宽: {bits} bit（最高位 {} = {}）\n按原码解释: {original}\n按反码解释: {inverse}\n按补码解释: {comp}\n\n三种编码只在负数时不同；正数三者相同。CTF 中给出位串未说明编码时，优先看题目语境或逐个试。",if sign{1}else{0},if sign{"负"}else{"正"})
            }
            Kind::Gcd => {
                let parts: Vec<BigInt> = text
                    .split(|c: char| space(c) || c == ',')
                    .filter(|s| !s.is_empty())
                    .map(|s| integer(s, 10))
                    .collect::<Result<_, _>>()?;
                if parts.len() < 2 {
                    return Err(bad("至少输入两个整数，以空格或逗号分隔"));
                }
                let (mut g, mut l) = (parts[0].clone(), parts[0].clone());
                for n in &parts[1..] {
                    ctx.cancel.check()?;
                    g = g.gcd(n).abs();
                    let divisor = l.gcd(n);
                    l = if divisor.is_zero() {
                        BigInt::zero()
                    } else {
                        l * n / divisor
                    };
                }
                out.insert("gcd".into(), PortValue::Text(g.to_string()));
                out.insert("lcm".into(), PortValue::Text(l.to_string()));
                format!(
                    "输入: {}\n最大公约数 (GCD): {g}\n最小公倍数 (LCM): {l}",
                    parts
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            Kind::Fibonacci => {
                let mut s = text.to_string();
                let f = fibs();
                for i in (32..128).rev() {
                    ctx.cancel.check()?;
                    s = s.replace(
                        &f[i].to_string(),
                        &char::from_u32((i + 1) as u32).unwrap().to_string(),
                    );
                }
                s
            }
        };
        if matches!(self.0, Kind::Separate | Kind::Offset(_)) {
            out.entry("results".into())
                .or_insert_with(|| PortValue::StringList(Vec::new()));
            out.entry("rows".into())
                .or_insert_with(|| PortValue::Json(json!([])));
        }
        if matches!(self.0, Kind::Offset(_)) {
            out.entry("bytes".into())
                .or_insert_with(|| PortValue::Bytes(Vec::new().into()));
        }
        out.insert("text".into(), PortValue::Text(report));
        Ok(out)
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for(kind,id,upstream,title,category,sample,description)in[
        (Kind::Separate,"separate_ascii","separationAscii","数字串分割 ASCII","numeric.radix","72101108108111","尝试整数大端字节与 10/16/8/2 进制贪婪 ASCII 分割；输出报告、候选文本列表和明细。"),
        (Kind::Offset(false),"ascii_offset_encode","asciiOffset","字符偏移编码","numeric.bits","Hello","按上游首个 UTF-16 码元偏移，offset=0 枚举 -26..26。非 BMP 字符可能产生孤立代理项；文本预览使用替换符，bytes 保存完整 UTF-16LE 结果。"),
        (Kind::Offset(true),"ascii_offset_decode","asciiOffset","字符偏移解码","numeric.bits","Khoor","按偏移量反向还原，offset=0 枚举候选；结果同时提供 UTF-16LE 字节，完整候选明细可连接下游。"),
        (Kind::Complement,"binary_complement","binaryComplement","原码反码补码分析","numeric.bits","-42","有符号 32 位整数自动选择 8/16/32 位，输出三种表示。最小负数只可用补码表示，原码/反码明确标注无法表示。"),
        (Kind::ComplementRead,"complement_to_decimal","complementToDec","原反补码解释","numeric.bits","11010110","按原码、反码、补码分别解释二进制位串；最高 64 位，保留原码与反码中的 -0。"),
        (Kind::Gcd,"gcd_lcm","gcd","多整数 GCD 与 LCM","numeric.math","12, 18, 30","对多个精确整数计算最大公约数和最小公倍数；零输入有定义，不经过浮点舍入。"),
        (Kind::Fibonacci,"fibonacci_decode","fibonacci","斐波那契数字解码","numeric.radix","3524578","从第 128 项向第 33 项替换斐波那契数字为对应字符，保留其他文本。"),
    ]{
        let params=match kind{Kind::Offset(_)=>vec![help(ParamSpec::number("offset","偏移量",-65535.0,65535.0,1.0,0.0),"0 枚举 -26..26（跳过 0）；非零按模 65536 偏移 UTF-16 码元。")],Kind::ComplementRead=>vec![help(ParamSpec::number("bits","位宽",0.0,64.0,1.0,0.0),"0 按输入长度；指定更大位宽在左侧补零，不做符号扩展。")],_=>vec![]};
        let mut outputs=vec![t_out()];
        if matches!(kind,Kind::Separate|Kind::Offset(_)){outputs.extend([req("rows","候选明细",PortType::Json),req("results","候选文本",PortType::StringList)]);}
        if matches!(kind,Kind::Offset(_)){outputs.push(req("bytes","UTF-16LE 字节",PortType::Bytes));}
        if matches!(kind,Kind::Complement|Kind::ComplementRead){for(n,label)in[("original","原码"),("inverse","反码"),("complement","补码")]{outputs.push(req(n,label,PortType::Text));}}
        if matches!(kind,Kind::Complement){outputs.push(req("bits","位宽",PortType::Number));}
        if matches!(kind,Kind::Gcd){outputs.extend([req("gcd","GCD",PortType::Text),req("lcm","LCM",PortType::Text)]);}
        let mut d=document_text(desc(id,category,title,CYAN,vec![t_in()],outputs,params),upstream,description,sample,if matches!(kind,Kind::Offset(_)){json!({"offset":3})}else{json!({})});
        if matches!(kind,Kind::Separate|Kind::Offset(_)){d.presentation.previews.insert("rows".into(),"table".into());}
        d.cost=Cost::Heavy;reg.register(d,Arc::new(move||Arc::new(Report(kind))));
    }
}
