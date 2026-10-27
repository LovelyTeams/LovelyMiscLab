//! The pinned upstream textual forms of Maya, Babylonian, Egyptian and pips.
use super::{
    codec_support::{document_text, help},
    numeric_support::{bad, space},
    prelude::*,
};
use num_bigint::BigUint;
use num_traits::{ToPrimitive, Zero};
use serde_json::{json, Value};
const BAB_TENS: [u32; 5] = [0x1230b, 0x12399, 0x1230d, 0x1240f, 0x12410];
const EGY: [(u32, u32); 7] = [
    (1_000_000, 0x13068),
    (100_000, 0x13190),
    (10_000, 0x130ad),
    (1000, 0x131bc),
    (100, 0x13362),
    (10, 0x13386),
    (1, 0x133fa),
];
fn symbol(cp: u32) -> char {
    char::from_u32(cp).unwrap()
}
fn separator(c: char) -> bool {
    space(c) || matches!(c, ',' | ';')
}
fn decimal(s: &str) -> Result<BigUint, CoreError> {
    let s = s.trim_matches(space);
    if s.is_empty()
        || s.len() > 16384
        || !s.bytes().all(|b| b.is_ascii_digit())
        || (s.len() > 1 && s.starts_with('0'))
    {
        return Err(bad("需要无前导零的非负十进制整数，最多 16384 位"));
    }
    BigUint::parse_bytes(s.as_bytes(), 10).ok_or_else(|| bad("十进制整数无效"))
}
fn choice<'a>(
    p: &'a Value,
    name: &str,
    default: &'a str,
    allowed: &[&str],
) -> Result<&'a str, CoreError> {
    let s = pstr(p, name, default);
    if !allowed.contains(&s) {
        return Err(bad("未知记数参数"));
    }
    Ok(s)
}
fn progress(ctx: &NodeCtx, i: usize, total: usize) -> Result<(), CoreError> {
    if i.is_multiple_of(128) {
        ctx.progress(i as f32 / total.max(1) as f32);
        ctx.cancel.check()?;
    }
    Ok(())
}
#[derive(Clone, Copy)]
enum Kind {
    Maya,
    Babylon,
    Egypt,
    Pips,
}
struct Numeral(Kind, bool);
impl Numeral {
    fn positional(&self, s: &str, p: &Value, ctx: &NodeCtx) -> Result<String, CoreError> {
        let maya = matches!(self.0, Kind::Maya);
        let long =
            maya && choice(p, "mode", "vigesimal", &["vigesimal", "longcount"])? == "longcount";
        let form = choice(
            p,
            "form",
            "unicode",
            if maya {
                &["unicode", "dotbar"]
            } else {
                &["unicode", "ascii"]
            },
        )?;
        let base = if maya { 20u32 } else { 60u32 };
        if !self.1 {
            let mut n = decimal(s)?;
            let mut ds = Vec::new();
            if n.is_zero() {
                ds.push(0);
            }
            while !n.is_zero() {
                progress(ctx, ds.len(), s.len() * 4)?;
                let radix = if long && ds.len() == 1 { 18 } else { base };
                ds.push((&n % radix).to_u32().unwrap());
                n /= radix;
            }
            Ok(ds
                .into_iter()
                .rev()
                .map(|d| {
                    if maya {
                        if form == "unicode" {
                            symbol(0x1d2e0 + d).to_string()
                        } else if d == 0 {
                            "0".into()
                        } else {
                            format!(
                                "{}{}",
                                ".".repeat((d % 5) as usize),
                                "-".repeat((d / 5) as usize)
                            )
                        }
                    } else if d == 0 {
                        "0".into()
                    } else if form == "ascii" {
                        format!(
                            "{}{}",
                            "<".repeat((d / 10) as usize),
                            "|".repeat((d % 10) as usize)
                        )
                    } else {
                        let mut s = String::new();
                        if d >= 10 {
                            s.push(symbol(BAB_TENS[(d / 10 - 1) as usize]));
                        }
                        if d % 10 != 0 {
                            s.push(symbol(0x12414 + d % 10));
                        }
                        s
                    }
                })
                .collect::<Vec<_>>()
                .join(" "))
        } else {
            let s = s.trim_matches(space);
            if s.is_empty() || s.chars().count() > 16384 {
                return Err(bad("密文为空或超过 16384 字符"));
            }
            let tokens: Vec<String> = if maya && form == "unicode" {
                s.chars()
                    .filter(|&c| !separator(c))
                    .map(|c| c.to_string())
                    .collect()
            } else {
                s.split(separator)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect()
            };
            if tokens.is_empty() {
                return Err(bad("没有记数符号"));
            }
            let mut ds = Vec::new();
            for (i, t) in tokens.iter().enumerate() {
                progress(ctx, i, tokens.len())?;
                let d = if maya {
                    if form == "unicode" {
                        let cp = t.chars().next().unwrap() as u32;
                        if !(0x1d2e0..=0x1d2f3).contains(&cp) {
                            return Err(bad("不在玛雅数字块中"));
                        }
                        cp - 0x1d2e0
                    } else if t == "0" {
                        0
                    } else {
                        let dots = t.bytes().take_while(|&b| b == b'.').count();
                        let tail = &t[dots..];
                        let bars = tail.len();
                        if dots > 4
                            || bars > 3
                            || !tail.bytes().all(|b| b == b'-')
                            || dots + bars == 0
                        {
                            return Err(bad("玛雅点横形态无效"));
                        }
                        dots as u32 + 5 * bars as u32
                    }
                } else if t == "0" {
                    0
                } else {
                    let mut v = 0u32;
                    for c in t.chars() {
                        v += if form == "ascii" {
                            match c {
                                '<' => 10,
                                '|' => 1,
                                _ => return Err(bad("巴比伦 ASCII 只允许 < 与 |")),
                            }
                        } else {
                            let cp = c as u32;
                            if (0x12415..=0x1241d).contains(&cp) {
                                cp - 0x12414
                            } else if cp == 0x12079 {
                                1
                            } else if let Some(i) = BAB_TENS.iter().position(|&n| n == cp) {
                                (i as u32 + 1) * 10
                            } else {
                                return Err(bad("不在巴比伦数字表中"));
                            }
                        };
                        if v > 59 {
                            return Err(bad("巴比伦每位最多 59"));
                        }
                    }
                    v
                };
                ds.push(d);
            }
            if !maya && ds.len() > 1 && ds[0] == 0 {
                return Err(bad("巴比伦数字不允许前导零位"));
            }
            let mut n = BigUint::zero();
            for (i, &d) in ds.iter().enumerate() {
                progress(ctx, i, ds.len())?;
                if long && ds.len() >= 2 && i == ds.len() - 2 && d > 17 {
                    return Err(bad("长纪历第二位最多 17"));
                }
                let radix = if long && ds.len() >= 3 && i == ds.len() - 2 {
                    18
                } else {
                    base
                };
                n = n * radix + d;
            }
            Ok(n.to_string())
        }
    }
    fn egypt(&self, s: &str, ctx: &NodeCtx) -> Result<String, CoreError> {
        if !self.1 {
            let n = decimal(s)?
                .to_u64()
                .filter(|&n| n > 0 && n <= 9_999_999_999)
                .ok_or_else(|| bad("埃及编码范围 1..9999999999，不含零"))?;
            let mut n = n;
            let mut out = String::new();
            for (i, &(v, cp)) in EGY.iter().enumerate() {
                ctx.cancel.check()?;
                ctx.progress(i as f32 / 7.0);
                out.push_str(&symbol(cp).to_string().repeat((n / v as u64) as usize));
                n %= v as u64;
            }
            Ok(out)
        } else {
            let count = s.chars().count();
            if count > 16384 {
                return Err(bad("埃及密文最多 16384 字符"));
            }
            let mut n = 0u64;
            for (i, c) in s.chars().enumerate() {
                progress(ctx, i, count)?;
                if separator(c) {
                    continue;
                }
                let v = EGY
                    .iter()
                    .find(|(_, cp)| *cp == c as u32)
                    .ok_or_else(|| bad("不在七种埃及数字符号中"))?
                    .0;
                n += v as u64;
            }
            if n == 0 {
                return Err(bad("埃及数字没有零或空输入写法"));
            }
            Ok(n.to_string())
        }
    }
    fn pips(&self, s: &str, p: &Value, ctx: &NodeCtx) -> Result<String, CoreError> {
        if s.trim_matches(space).is_empty() {
            return Err(bad("点数输入为空"));
        }
        let domino = choice(p, "system", "dice", &["dice", "domino"])? == "domino";
        let base =
            if choice(p, "orientation", "horizontal", &["horizontal", "vertical"])? == "vertical" {
                0x1f063
            } else {
                0x1f031
            };
        let mut out = String::new();
        let mut first = None;
        for (i, c) in s.chars().enumerate() {
            progress(ctx, i, s.len())?;
            if space(c) || (domino && ",/|()".contains(c)) {
                if first.is_some() {
                    return Err(bad("骨牌数字段长度必须为偶数"));
                }
                out.push(c);
                continue;
            }
            if domino {
                if self.1 {
                    let cp = c as u32;
                    if !(base..base + 49).contains(&cp) {
                        return Err(bad("骨牌字符无效、朝向不匹配或为背面"));
                    }
                    let v = cp - base;
                    out.push(char::from(b'0' + (v / 7) as u8));
                    out.push(char::from(b'0' + (v % 7) as u8));
                } else {
                    let v = c
                        .to_digit(10)
                        .filter(|&v| v <= 6)
                        .ok_or_else(|| bad("骨牌每端只允许 0..6"))?;
                    if let Some(a) = first.take() {
                        out.push(symbol(base + 7 * a + v));
                    } else {
                        first = Some(v);
                    }
                }
            } else if self.1 {
                let cp = c as u32;
                if !(0x2680..=0x2685).contains(&cp) {
                    return Err(bad("只接受六种骰面符号，GAME DIE 不是数码"));
                }
                out.push(char::from(b'1' + (cp - 0x2680) as u8));
            } else {
                let v = c
                    .to_digit(10)
                    .filter(|&v| (1..=6).contains(&v))
                    .ok_or_else(|| bad("骰面只允许数字 1..6"))?;
                out.push(symbol(0x267f + v));
            }
        }
        if first.is_some() {
            return Err(bad("骨牌数字段长度必须为偶数"));
        }
        Ok(out)
    }
}
impl Node for Numeral {
    fn run(&self, i: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let s = in_text(i, "text")?;
        if s.len() > 1_048_576 {
            return Err(bad("记数输入最多 1 MiB"));
        }
        let text = match self.0 {
            Kind::Maya | Kind::Babylon => self.positional(s, p, ctx)?,
            Kind::Egypt => self.egypt(s, ctx)?,
            Kind::Pips => self.pips(s, p, ctx)?,
        };
        ctx.cancel.check()?;
        Ok(out_text(text))
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for(kind,id,upstream,title,enc,dec,description)in[
        (Kind::Maya,"maya_numerals","mayaNumerals","玛雅数字","406","𝋡 𝋠 𝋦","非负整数与玛雅符号互转；纯 20 进制或长纪历 20×18×20，支持 Unicode 与点横文本。十进制拒绝前导零，长纪历次低位最多 17。"),
        (Kind::Babylon,"babylonian_numerals","babylonianNumerals","巴比伦数字","61","𒐕 𒐕","六十进制，使用固定上游 14 字形或 < / | ASCII 记法；0 为工具占位约定，拒绝前导零位。"),
        (Kind::Egypt,"egyptian_numerals","egyptianNumerals","埃及数字","123","𓍢𓎆𓎆𓏺𓏺𓏺","七种加法十进制圣书体符号，编码范围 1..9999999999，解码任意符号顺序。没有零、紧凑组合字形或分数。"),
        (Kind::Pips,"pip_numerals","pipNumerals","骰面与骨牌记数","123456","⚀⚁⚂⚃⚄⚅","骰面 1..6 或骨牌成对 0..6，横/竖朝向可选；分隔符原样保留，拒绝背面、异朝向和无法表示的数字。"),
    ]{
        let params=match kind{
            Kind::Maya=>vec![help(ParamSpec::select("mode","进制模式",&["vigesimal","longcount"],"vigesimal"),"vigesimal 每位 ×20；longcount 次低位基数 18，其余 ×20。"),help(ParamSpec::select("form","符号形态",&["unicode","dotbar"],"unicode"),"Unicode 每位一字符；dotbar 为 . 表示 1、- 表示 5，0 占位。")],
            Kind::Babylon=>vec![help(ParamSpec::select("form","符号形态",&["unicode","ascii"],"unicode"),"Unicode 使用固定 14 字形；ascii 用 < 表示十、| 表示一。位间空格。")],
            Kind::Egypt=>vec![],
            Kind::Pips=>vec![help(ParamSpec::select("system","符号系统",&["dice","domino"],"dice"),"dice 每位为 1..6；domino 每两位为 0..6，数字段长度须偶数。"),help(ParamSpec::select("orientation","骨牌朝向",&["horizontal","vertical"],"horizontal"),"仅影响 domino；解码必须选择实际朝向。")]
        };
        for decode in [false,true]{
            let mut d=document_text(desc(&format!("{id}_{}",if decode{"decode"}else{"encode"}),"numeric.radix",&format!("{title} {}",if decode{"解码"}else{"编码"}),CYAN,vec![t_in()],vec![t_out()],params.clone()),upstream,description,if decode{dec}else{enc},json!({}));
            d.cost=Cost::Heavy;reg.register(d,Arc::new(move||Arc::new(Numeral(kind,decode))));
        }
    }
}
