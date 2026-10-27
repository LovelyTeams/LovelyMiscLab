//! IEEE bit patterns and radix fractions. Half precision has an explicit legacy mode.
use super::{
    codec_support::{document_text, help},
    numeric_support::{bad, compact, integer, number, space},
    prelude::*,
};
use num_bigint::BigUint;
use num_traits::{One, ToPrimitive};
use serde_json::{json, Value};
pub(super) fn js_number(s: &str) -> Result<f64, CoreError> {
    let s = s.trim_matches(space);
    if s.is_empty() {
        return Ok(0.0);
    }
    let lower = s.to_ascii_lowercase();
    for (prefix, base) in [("0x", 16), ("0b", 2), ("0o", 8)] {
        if let Some(n) = lower.strip_prefix(prefix) {
            return integer(n, base)?
                .to_f64()
                .ok_or_else(|| bad("浮点转换超出范围"));
        }
    }
    if matches!(s, "Infinity" | "+Infinity") {
        return Ok(f64::INFINITY);
    }
    if s == "-Infinity" {
        return Ok(f64::NEG_INFINITY);
    }
    if !regex::Regex::new(r"^[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?$")
        .unwrap()
        .is_match(s)
    {
        return Err(bad("无效浮点数字"));
    }
    s.parse().map_err(|_| bad("无效浮点数字"))
}
fn shown(n: f64) -> String {
    ryu_js::Buffer::new().format(n).into()
}
/// ECMAScript toFixed rounds exact binary64 rational values, ties away from zero.
pub(super) fn fixed(n: f64, precision: u32) -> String {
    if n.abs() >= 1e21 {
        return shown(n);
    }
    let b = n.abs().to_bits();
    let exponent = ((b >> 52) & 2047) as i32;
    let mantissa = (b & ((1u64 << 52) - 1)) | if exponent == 0 { 0 } else { 1u64 << 52 };
    let exponent = if exponent == 0 {
        -1074
    } else {
        exponent - 1023 - 52
    } + precision as i32;
    let scaled = BigUint::from(mantissa) * BigUint::from(5u8).pow(precision);
    let rounded = if exponent >= 0 {
        scaled << exponent as usize
    } else {
        let divisor = BigUint::one() << (-exponent) as usize;
        let q = &scaled / &divisor;
        let rem = scaled % &divisor;
        if rem * 2u8 >= divisor {
            q + 1u8
        } else {
            q
        }
    };
    let raw = rounded.to_string();
    let mut s = if precision == 0 {
        raw
    } else {
        let raw = format!("{raw:0>width$}", width = precision as usize + 1);
        let i = raw.len() - precision as usize;
        format!("{}.{}", &raw[..i], &raw[i..])
    };
    if n < 0.0 {
        s.insert(0, '-');
    }
    s
}
fn legacy_half(n: f64) -> u32 {
    let bits = (n as f32).to_bits();
    let sign = (bits >> 16) & 0x8000;
    let exp = ((bits >> 23) & 255) as i32;
    let man = bits & 0x7fffff;
    if exp == 255 {
        return sign | 0x7c00 | if man > 0 { 0x200 } else { 0 };
    }
    let exp = exp - 127 + 15;
    if exp >= 31 {
        return sign | 0x7c00;
    }
    if exp <= 0 {
        return sign | ((man | 0x800000) >> ((1 - exp + 13) & 31));
    }
    sign | ((exp as u32) << 10) | (man >> 13)
}
fn legacy_half_read(h: u16) -> f64 {
    let sign = ((h as u32) & 0x8000) << 16;
    let exp = (h >> 10) & 31;
    let man = (h & 1023) as u32;
    let bits = if exp == 0 {
        if man == 0 {
            sign
        } else {
            let mut m = man;
            let mut e = 0;
            while m & 0x400 == 0 {
                m <<= 1;
                e += 1;
            }
            sign | ((15 - e - 1 + 127) << 23) | ((m & 1023) << 13)
        }
    } else if exp == 31 {
        sign | 0x7f800000 | (man << 13)
    } else {
        sign | (((exp as u32 + 127 - 15) & 255) << 23) | (man << 13)
    };
    f32::from_bits(bits) as f64
}
fn decimal_radix(n: f64, base: u32, precision: usize) -> String {
    let alphabet = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut int = n.abs().floor();
    let mut digits = Vec::new();
    while int > 0.0 {
        digits.push(alphabet[(int % base as f64) as usize] as char);
        int = (int / base as f64).floor();
    }
    if digits.is_empty() {
        digits.push('0');
    }
    let mut text: String = digits.into_iter().rev().collect();
    let mut frac = n.abs() % 1.0;
    let mut after = String::new();
    for _ in 0..precision {
        frac *= base as f64;
        let d = frac.floor();
        after.push(alphabet[d as usize] as char);
        frac -= d;
    }
    let after = after.trim_end_matches('0');
    if !after.is_empty() {
        text.push('.');
        text.push_str(after);
    }
    if n < 0.0 {
        text.insert(0, '-');
    }
    text
}
fn parse_radix(s: &str, base: u32) -> Result<f64, CoreError> {
    let s = compact(s);
    if s.is_empty() {
        return Err(bad("浮点输入不能为空"));
    }
    let neg = s.starts_with('-');
    let s = s.strip_prefix(['+', '-']).unwrap_or(&s);
    let s = s.to_ascii_lowercase();
    let prefix = match base {
        2 => "0b",
        8 => "0o",
        16 => "0x",
        _ => "",
    };
    let s = if prefix.is_empty() {
        s.as_str()
    } else {
        s.strip_prefix(prefix).unwrap_or(&s)
    };
    let mut pieces = s.split('.');
    let integral = pieces.next().unwrap();
    let frac = pieces.next().unwrap_or("");
    if pieces.next().is_some() {
        return Err(bad("浮点输入包含多个小数点"));
    }
    let mut value = 0.0;
    for c in integral.chars() {
        let d = c.to_digit(base).ok_or_else(|| bad("数位与进制不匹配"))?;
        value = value * base as f64 + d as f64;
    }
    let mut place = 1.0 / base as f64;
    for c in frac.chars() {
        let d = c.to_digit(base).ok_or_else(|| bad("数位与进制不匹配"))?;
        value += d as f64 * place;
        place /= base as f64;
    }
    if !value.is_finite() {
        return Err(bad("结果不是有限数"));
    }
    Ok(if neg { -value } else { value })
}
#[derive(Clone, Copy)]
enum Kind {
    Ieee(bool),
    ToRadix,
    FromRadix,
}
struct Tool(Kind);
impl Node for Tool {
    fn run(&self, i: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let s = in_text(i, "text")?.trim_matches(space);
        if s.len() > 16384 {
            return Err(bad("浮点输入最多 16384 字符"));
        }
        let mut out = PortMap::new();
        let text = match self.0 {
            Kind::Ieee(decode) => {
                let precision = pstr(p, "precision", "single");
                let width = match precision {
                    "half" => 4,
                    "single" => 8,
                    "double" => 16,
                    _ => return Err(bad("未知 IEEE 精度")),
                };
                let mode = pstr(p, "halfMode", "ieee");
                if !["ieee", "upstream"].contains(&mode) {
                    return Err(bad("未知半精度模式"));
                }
                let legacy = mode == "upstream";
                if decode {
                    let h: String = s
                        .chars()
                        .filter(char::is_ascii_hexdigit)
                        .take(width)
                        .collect();
                    let bits = u64::from_str_radix(&h, 16).map_err(|_| bad("需要 Hex 位模式"))?;
                    let n = match precision {
                        "half" => {
                            if legacy {
                                legacy_half_read(bits as u16)
                            } else {
                                half::f16::from_bits(bits as u16).to_f64()
                            }
                        }
                        "single" => f32::from_bits(bits as u32) as f64,
                        _ => f64::from_bits(bits),
                    };
                    shown(n)
                } else {
                    let n = js_number(s)?;
                    let bits = match precision {
                        "half" => {
                            if legacy {
                                legacy_half(n) as u64
                            } else {
                                half::f16::from_f64(n).to_bits() as u64
                            }
                        }
                        "single" => (n as f32).to_bits() as u64,
                        _ => n.to_bits(),
                    };
                    format!("{bits:0width$X}")
                }
            }
            Kind::ToRadix => {
                let n = js_number(s)?;
                if !n.is_finite() {
                    return Err(bad("进制浮点转换需要有限数"));
                }
                let precision = number(p, "precision", 16, 1, 64)? as usize;
                let mut lines = vec![format!("输入: {s}"), format!("精度: {precision} 位小数")];
                let mut rows = Vec::new();
                for base in [2, 8, 10, 16] {
                    let value = decimal_radix(n, base, precision);
                    lines.push(format!("{base} 进制: {value}"));
                    rows.push(json!({"base":base,"value":value}));
                }
                out.insert("rows".into(), PortValue::Json(json!(rows)));
                lines.join("\n")
            }
            Kind::FromRadix => {
                let base = number(p, "base", 2, 2, 16)? as u32;
                if ![2, 8, 10, 16].contains(&base) {
                    return Err(bad("进制仅支持 2/8/10/16"));
                }
                let n = parse_radix(s, base)?;
                let precision = number(p, "precision", 16, 0, 64)? as u32;
                let precision = if precision == 0 { 16 } else { precision };
                let v = fixed(n, precision);
                let v = if v.contains('.') {
                    v.trim_end_matches('0').trim_end_matches('.')
                } else {
                    &v
                };
                out.insert("value".into(), PortValue::Number(n));
                out.insert("decimal".into(), PortValue::Text(v.into()));
                format!("{base} 进制浮点: {s}\n十进制: {v}\n精度: {precision} 位小数")
            }
        };
        ctx.cancel.check()?;
        out.insert("text".into(), PortValue::Text(text));
        Ok(out)
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for(kind,id,upstream,title,sample,description)in[
 (Kind::Ieee(false),"ieee754_encode","ieee754","IEEE754 位模式编码","3.141592653589793","浮点数转大端显示的 Hex 位模式，支持半/单/双精度；half 默认 IEEE 最近偶数舍入，可显式选上游兼容旧半精度。"),
 (Kind::Ieee(true),"ieee754_decode","ieee754","IEEE754 位模式解码","40490FDB","Hex 位模式转浮点文本，支持非正规数、负零、无穷与 NaN。halfMode=upstream 仅用于复现旧半精度次正规值缺陷。"),
 (Kind::ToRadix,"decimal_to_radix_float","decimalToFloat","十进制浮点转多进制","10.625","按 binary64 运算转 2/8/10/16 进制小数，指定小数位数；报告和表格可连接下游。"),
 (Kind::FromRadix,"radix_float_to_decimal","floatToDecimal","进制浮点转十进制","1010.101","2/8/10/16 进制浮点解析，仅剥离匹配进制的前缀；按上游 toFixed 规则输出，value 为 binary64 数值。") ]{
 let params=match kind{Kind::Ieee(_)=>vec![help(ParamSpec::select("precision","精度",&["half","single","double"],"single"),"half 16 位、single 32 位、double 64 位。"),help(ParamSpec::select("halfMode","半精度规则",&["ieee","upstream"],"ieee"),"ieee 为正确标准舍入；upstream 精确复现 ea2bc36 的截断、移位和次正规解码行为。")],Kind::ToRadix=>vec![help(ParamSpec::number("precision","小数位数",1.0,64.0,1.0,16.0),"每个进制最多保留 1..64 位小数，去掉末尾零。")],Kind::FromRadix=>vec![help(ParamSpec::select("base","输入进制",&["2","8","10","16"],"2"),"只剥离匹配的 0b/0o/0x；0BEEF 在 16 进制视为数字。"),help(ParamSpec::number("precision","输出小数位数",0.0,64.0,1.0,16.0),"0 按上游回退为 16；舍入后去掉小数尾零。")]};
 let mut outputs=vec![t_out()];
 if matches!(kind,Kind::ToRadix){outputs.push(req("rows","进制对照",PortType::Json));}
 if matches!(kind,Kind::FromRadix){outputs.extend([req("value","数值",PortType::Number),req("decimal","十进制文本",PortType::Text)]);}
 let mut d=document_text(desc(id,"numeric.radix",title,CYAN,vec![t_in()],outputs,params),upstream,description,sample,json!({}));if matches!(kind,Kind::ToRadix){d.presentation.previews.insert("rows".into(),"table".into());}reg.register(d,Arc::new(move||Arc::new(Tool(kind))));
 }
}
