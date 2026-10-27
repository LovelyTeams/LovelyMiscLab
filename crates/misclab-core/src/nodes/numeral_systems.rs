//! Exact arbitrary-precision numeral systems. Pair nodes keep directions explicit.
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
    Radix,
    Negative,
    Balanced,
    Factorial,
    Zeckendorf,
    Roman,
    Chinese,
    Fraction,
    Stern,
}
struct Codec {
    kind: Kind,
    decode: bool,
}
const ROMAN: &[(u32, &str)] = &[
    (1000, "M"),
    (900, "CM"),
    (500, "D"),
    (400, "CD"),
    (100, "C"),
    (90, "XC"),
    (50, "L"),
    (40, "XL"),
    (10, "X"),
    (9, "IX"),
    (5, "V"),
    (4, "IV"),
    (1, "I"),
];
fn roman_encode(mut n: u32) -> Result<String, CoreError> {
    if !(1..=3999).contains(&n) {
        return Err(bad("罗马数字范围为 1..3999"));
    }
    let mut out = String::new();
    for &(v, s) in ROMAN {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    Ok(out)
}
fn fibonacci(count: usize, ctx: &mut NodeCtx) -> Result<Vec<BigInt>, CoreError> {
    let mut seq = vec![BigInt::one(), BigInt::from(2)];
    for i in 2..count {
        if i % 256 == 0 {
            ctx.cancel.check()?;
        }
        seq.push(&seq[i - 1] + &seq[i - 2]);
    }
    Ok(seq)
}
impl Node for Codec {
    fn run(
        &self,
        inputs: &PortMap,
        params: &Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let s = in_text(inputs, "text")?.trim_matches(space);
        let limit = if matches!(self.kind, Kind::Stern) {
            100_000
        } else {
            MAX_DIGITS
        };
        if s.len() > limit {
            return Err(bad("输入超过当前节点的字符上限"));
        }
        let output = match self.kind {
            Kind::Radix => {
                let a = number(params, "fromBase", 16, 2, 36)? as u32;
                let b = number(params, "toBase", 2, 2, 36)? as u32;
                let (from, to) = if self.decode { (b, a) } else { (a, b) };
                integer(s, from)?.to_str_radix(to)
            }
            Kind::Negative => {
                let base = number(params, "base", -2, -36, -2)?;
                if self.decode {
                    let mut n = BigInt::zero();
                    for c in compact(s).chars() {
                        ctx.cancel.check()?;
                        let d = c
                            .to_digit((-base) as u32)
                            .ok_or_else(|| bad("负进制包含无效字符"))?;
                        n = n * base + d;
                    }
                    n.to_string()
                } else {
                    let mut n = integer(s, 10)?;
                    if n.is_zero() {
                        "0".into()
                    } else {
                        let mut digits = Vec::new();
                        while !n.is_zero() {
                            ctx.cancel.check()?;
                            let (mut q, mut r) = n.div_rem(&BigInt::from(base));
                            if r.is_negative() {
                                r -= base;
                                q += 1;
                            }
                            digits.push(
                                char::from_digit(r.to_u32().unwrap(), (-base) as u32)
                                    .unwrap()
                                    .to_ascii_uppercase(),
                            );
                            n = q;
                        }
                        digits.into_iter().rev().collect()
                    }
                }
            }
            Kind::Balanced => {
                if self.decode {
                    let s = compact(s).to_ascii_uppercase();
                    if s.is_empty() {
                        return Err(bad("平衡三进制不能为空"));
                    }
                    let mut n = BigInt::zero();
                    for c in s.chars() {
                        ctx.cancel.check()?;
                        n = n * 3
                            + match c {
                                'T' => -1,
                                '0' => 0,
                                '1' => 1,
                                _ => return Err(bad("平衡三进制仅含 T/0/1")),
                            };
                    }
                    n.to_string()
                } else {
                    let original = integer(s, 10)?;
                    let negative = original.is_negative();
                    let mut n = original.abs();
                    let mut out = Vec::new();
                    if n.is_zero() {
                        out.push('0');
                    }
                    while !n.is_zero() {
                        ctx.cancel.check()?;
                        let (q, r) = n.div_rem(&BigInt::from(3));
                        n = q;
                        let c = match r.to_u32().unwrap() {
                            0 => '0',
                            1 => {
                                if negative {
                                    'T'
                                } else {
                                    '1'
                                }
                            }
                            _ => {
                                n += 1;
                                if negative {
                                    '1'
                                } else {
                                    'T'
                                }
                            }
                        };
                        out.push(c);
                    }
                    out.into_iter().rev().collect()
                }
            }
            Kind::Factorial => {
                if self.decode {
                    let parts: Vec<_> = s
                        .split(|c: char| space(c) || c == ':' || c == ',')
                        .filter(|s| !s.is_empty())
                        .collect();
                    if parts.is_empty() || parts.len() > 254 {
                        return Err(bad("阶乘进制需要 1..254 个数位"));
                    }
                    let mut n = BigInt::zero();
                    let mut fact = BigInt::one();
                    for (i, d) in parts.iter().rev().enumerate() {
                        ctx.cancel.check()?;
                        let d = integer(d, 10)?;
                        if pbool(params, "strict", true)
                            && (d.is_negative() || d > BigInt::from(i + 1))
                        {
                            return Err(bad("阶乘数位超出位置允许范围"));
                        }
                        n += d * &fact;
                        fact *= i + 2;
                    }
                    n.to_string()
                } else {
                    let mut n = integer(s, 10)?;
                    if n.is_negative() {
                        return Err(bad("阶乘进制不接受负数"));
                    }
                    let mut digits = Vec::new();
                    let mut base = 2_u32;
                    if n.is_zero() {
                        digits.push("0".into());
                    }
                    while !n.is_zero() {
                        ctx.cancel.check()?;
                        let (q, r) = n.div_rem(&BigInt::from(base));
                        n = q;
                        digits.push(r.to_string());
                        base += 1;
                        if base > 256 {
                            return Err(bad("阶乘进制数值超过 254 个数位"));
                        }
                    }
                    digits.reverse();
                    digits.join(":")
                }
            }
            Kind::Zeckendorf => {
                if self.decode {
                    let s: String = s.chars().filter(|c| !space(*c) && *c != ',').collect();
                    if s.is_empty() || !s.bytes().all(|b| b == b'0' || b == b'1') {
                        return Err(bad("Zeckendorf 仅含 0/1"));
                    }
                    let fibs = fibonacci(s.len(), ctx)?;
                    let mut n = BigInt::zero();
                    for (i, c) in s.bytes().rev().enumerate() {
                        if c == b'1' {
                            n += &fibs[i];
                        }
                    }
                    n.to_string()
                } else {
                    let mut n = integer(s, 10)?;
                    if n.is_negative() {
                        return Err(bad("Zeckendorf 不接受负数"));
                    }
                    let mut fibs = vec![BigInt::one(), BigInt::from(2)];
                    while fibs.last().unwrap() <= &n {
                        ctx.cancel.check()?;
                        let len = fibs.len();
                        fibs.push(&fibs[len - 1] + &fibs[len - 2]);
                        if fibs.len() > MAX_DIGITS {
                            return Err(bad("Zeckendorf 输出超过 16384 位"));
                        }
                    }
                    while fibs.len() > 1 && fibs.last().unwrap() > &n {
                        fibs.pop();
                    }
                    let mut out = String::new();
                    for f in fibs.iter().rev() {
                        if &n >= f {
                            out.push('1');
                            n -= f;
                        } else {
                            out.push('0');
                        }
                    }
                    out
                }
            }
            Kind::Roman => {
                if self.decode {
                    let upper = s.to_ascii_uppercase();
                    let mut rest = upper.as_str();
                    let mut n = 0;
                    for &(v, sym) in ROMAN {
                        while rest.starts_with(sym) {
                            n += v;
                            rest = &rest[sym.len()..];
                        }
                    }
                    if !rest.is_empty() || !roman_encode(n).is_ok_and(|expected| expected == upper)
                    {
                        return Err(bad("不是规范的罗马数字"));
                    }
                    n.to_string()
                } else {
                    roman_encode(
                        integer(s, 10)?
                            .to_u32()
                            .ok_or_else(|| bad("罗马数字范围 1..3999"))?,
                    )?
                }
            }
            Kind::Chinese => {
                let symbols: Vec<char> = "零一二三四五六七八九".chars().collect();
                if self.decode {
                    let (negative, s) = s.strip_prefix('负').map_or((false, s), |s| (true, s));
                    let mut out = String::new();
                    for c in s.chars() {
                        let i = symbols
                            .iter()
                            .position(|s| *s == c)
                            .ok_or_else(|| bad("中文逐位数字仅支持 零一二三四五六七八九"))?;
                        out.push((b'0' + i as u8) as char);
                    }
                    let n = if out.is_empty() {
                        BigInt::zero()
                    } else {
                        integer(&out, 10)?
                    };
                    if negative {
                        (-n).to_string()
                    } else {
                        n.to_string()
                    }
                } else {
                    let n = integer(s, 10)?;
                    let mut out = if n.is_negative() {
                        "负".into()
                    } else {
                        String::new()
                    };
                    for b in n.abs().to_string().bytes() {
                        out.push(symbols[(b - b'0') as usize]);
                    }
                    out
                }
            }
            Kind::Fraction => {
                if self.decode {
                    let s = s.replace(['[', ']'], "");
                    let parts: Vec<_> = s
                        .split(|c: char| space(c) || c == ',' || c == ';')
                        .filter(|s| !s.is_empty())
                        .collect();
                    if parts.is_empty() {
                        return Err(bad("连分数序列不能为空"));
                    }
                    let (mut n, mut d) = (BigInt::one(), BigInt::zero());
                    for a in parts.iter().rev() {
                        ctx.cancel.check()?;
                        let next = integer(a, 10)? * &n + &d;
                        d = n;
                        n = next;
                    }
                    if d.is_zero() {
                        return Err(bad("连分数还原后分母为零"));
                    }
                    format!("{n}/{d}")
                } else {
                    let (mut n, mut d) = fraction(s)?;
                    if d.is_zero() {
                        return Err(bad("分母不能为零"));
                    }
                    if d.is_negative() {
                        n = -n;
                        d = -d;
                    }
                    let mut parts = Vec::new();
                    while !d.is_zero() {
                        ctx.cancel.check()?;
                        let (q, r) = n.div_mod_floor(&d);
                        parts.push(q.to_string());
                        n = d;
                        d = r;
                    }
                    format!("[{}]", parts.join(", "))
                }
            }
            Kind::Stern => {
                if self.decode {
                    let s: String = s
                        .chars()
                        .filter(|c| !space(*c) && *c != ',')
                        .collect::<String>()
                        .to_ascii_uppercase();
                    let (mut a, mut b, mut c, mut d) =
                        (BigInt::zero(), BigInt::one(), BigInt::one(), BigInt::zero());
                    if s != "(ROOT)" {
                        for ch in s.chars() {
                            ctx.cancel.check()?;
                            let (n, m) = (&a + &c, &b + &d);
                            match ch {
                                'L' => {
                                    c = n;
                                    d = m;
                                }
                                'R' => {
                                    a = n;
                                    b = m;
                                }
                                _ => return Err(bad("Stern-Brocot 路径仅含 L/R")),
                            };
                        }
                    }
                    format!("{}/{}", a + c, b + d)
                } else {
                    let (mut n, mut d) = fraction(s)?;
                    if n <= BigInt::zero() || d <= BigInt::zero() {
                        return Err(bad("Stern-Brocot 需要正分数"));
                    }
                    let mut path = String::new();
                    while n != d {
                        ctx.progress(path.len() as f32 / 100_000.0);
                        ctx.cancel.check()?;
                        let (ch, count) = if n < d {
                            ('L', (&d - 1_u8) / &n)
                        } else {
                            ('R', (&n - 1_u8) / &d)
                        };
                        let count = count
                            .to_usize()
                            .filter(|c| *c <= 100_000 - path.len())
                            .ok_or_else(|| bad("Stern-Brocot 路径超过 100000 步"))?;
                        path.extend(std::iter::repeat_n(ch, count));
                        if ch == 'L' {
                            d -= &n * count;
                        } else {
                            n -= &d * count;
                        }
                    }
                    if path.is_empty() {
                        "(root)".into()
                    } else {
                        path
                    }
                }
            }
        };
        ctx.cancel.check()?;
        if output.len() > limit {
            return Err(bad("结果超过字符上限，无法安全接续反向转换"));
        }
        Ok(out_text(output))
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for (kind, id, upstream, title, category, example, encoded) in [
        (
            Kind::Radix,
            "radix_transform",
            "radixConvert",
            "任意进制转换",
            "numeric.radix",
            "ff",
            "11111111",
        ),
        (
            Kind::Negative,
            "negative_base",
            "negabase",
            "负进制",
            "numeric.radix",
            "-13",
            "110111",
        ),
        (
            Kind::Balanced,
            "balanced_ternary",
            "balancedTernary",
            "平衡三进制",
            "numeric.radix",
            "-13",
            "TTT",
        ),
        (
            Kind::Factorial,
            "factorial_base",
            "factorialBase",
            "阶乘进制",
            "numeric.radix",
            "463",
            "3:4:1:0:1",
        ),
        (
            Kind::Zeckendorf,
            "zeckendorf",
            "zeckendorf",
            "Zeckendorf 表示",
            "numeric.radix",
            "100",
            "1000010100",
        ),
        (
            Kind::Roman,
            "roman",
            "roman",
            "罗马数字",
            "numeric.radix",
            "2026",
            "MMXXVI",
        ),
        (
            Kind::Chinese,
            "chinese_number",
            "chineseNum",
            "中文逐位数字",
            "encoding.chinese",
            "-2026",
            "负二零二六",
        ),
        (
            Kind::Fraction,
            "continued_fraction",
            "continuedFraction",
            "连分数",
            "numeric.math",
            "355/113",
            "[3, 7, 16]",
        ),
        (
            Kind::Stern,
            "stern_brocot",
            "sternBrocot",
            "Stern-Brocot 路径",
            "numeric.math",
            "5/3",
            "RLR",
        ),
    ] {
        for decode in [false, true] {
            let params = match kind {
                Kind::Radix => vec![
                    help(
                        ParamSpec::number("fromBase", "源进制", 2.0, 36.0, 1.0, 16.0),
                        "编码方向的输入进制；解码方向交换源与目标。",
                    ),
                    help(
                        ParamSpec::number("toBase", "目标进制", 2.0, 36.0, 1.0, 2.0),
                        "编码方向的输出进制；整数按任意精度处理。",
                    ),
                ],
                Kind::Negative => vec![help(
                    ParamSpec::number("base", "负进制", -36.0, -2.0, 1.0, -2.0),
                    "常用 -2/-3/-10，范围 -36..-2；数位使用 0-9A-Z。",
                )],
                Kind::Factorial if decode => vec![help(
                    ParamSpec::toggle("strict", "校验数位范围", true),
                    "从右起第 i 位须为 0..i；关闭可按上游宽松方式求和。",
                )],
                _ => vec![],
            };
            let description=match kind {
                Kind::Radix=>"2..36 进制有符号大整数互转；不经过浮点，保留所有有效数位。输入最多 16384 字符。",
                Kind::Negative=>"十进制整数与负进制表示互转，余数始终落在 0..|base|-1；支持负数。",
                Kind::Balanced=>"十进制与平衡三进制互转；T 代表 -1，支持任意精度与负数。",
                Kind::Factorial=>"阶乘进制，以冒号分隔数位，从右侧 1! 开始加权；最多 254 个数位。",
                Kind::Zeckendorf=>"按 1,2,3,5,… 斐波那契数位表示非负整数；编码输出不含相邻 1 的规范表示。",
                Kind::Roman=>"十进制 1..3999 与规范罗马数字互转；解码大小写兼容，拒绝无效减法组合。",
                Kind::Chinese=>"十进制逐位映射为零一二三四五六七八九，负数用负字前缀；不使用十百千万位权。",
                Kind::Fraction=>"有理数 p/q 与连分数序列互转，使用精确整数运算，支持负数和负分母。",
                Kind::Stern=>"正分数与 Stern-Brocot 树的 L/R 路径互转；根为 (root)。超过十万步报错，支持取消。",
            };
            let mut d = document_text(
                desc(
                    &format!("{id}_{}", if decode { "decode" } else { "encode" }),
                    category,
                    &format!("{title} {}", if decode { "解码" } else { "编码" }),
                    CYAN,
                    vec![t_in()],
                    vec![t_out()],
                    params,
                ),
                upstream,
                description,
                if decode { encoded } else { example },
                json!({}),
            );
            d.cost = Cost::Heavy;
            reg.register(d, Arc::new(move || Arc::new(Codec { kind, decode })));
        }
    }
}
