//! Exact decimal arithmetic and bounded, cancellable number theory.
//! Compatibility reference: EBCTFCodeBox ea2bc36/src/core/bigmath.js.
use super::{
    codec_support::{document_text, help},
    numeric_support,
    prelude::*,
    prime_tools::{modpow, probable, BOUND},
    transport_codecs::err,
};
use num_bigint::{BigInt, BigUint};
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};
use serde_json::{json, Value};
use std::sync::LazyLock;

const OPS: [&str; 17] = [
    "add",
    "sub",
    "mul",
    "pow",
    "div",
    "mod",
    "modpow",
    "modinv",
    "gcd",
    "extgcd",
    "lcm",
    "isPrime",
    "nextPrime",
    "prevPrime",
    "factor",
    "sqrtInt",
    "bitLen",
];
const BINARY: [&str; 10] = [
    "add", "sub", "mul", "pow", "div", "mod", "modpow", "gcd", "extgcd", "lcm",
];
const RHO_BUDGET: usize = 3 * (1 << 20);
static SMALL_PRIMES: LazyLock<Vec<u32>> = LazyLock::new(|| {
    let mut composite = vec![false; 100001];
    let mut primes = Vec::new();
    for i in 2..=100000usize {
        if !composite[i] {
            primes.push(i as u32);
            if i <= 316 {
                for j in (i * i..=100000).step_by(i) {
                    composite[j] = true;
                }
            }
        }
    }
    primes
});

fn operand(inputs: &PortMap, p: &Value, key: &str, default: &str) -> Result<BigInt, CoreError> {
    let raw = match inputs.get(key) {
        Some(value) => value.as_text()?,
        None => match p.get(key) {
            Some(value) => value
                .as_str()
                .ok_or_else(|| err(format!("{key} 需为十进制文本")))?,
            None => default,
        },
    };
    if raw.len() > 16384 {
        return Err(err(format!("{key} 输入最多 16384 字节")));
    }
    let compact: String = raw
        .chars()
        .filter(|&c| !numeric_support::space(c) && c != ',' && c != '_')
        .collect();
    let n = numeric_support::integer(&compact, 10)?;
    if n.bits() > 4096 {
        return Err(err(format!("{key} 最多 4096 位二进制")));
    }
    Ok(n)
}

fn gcd(mut a: BigUint, mut b: BigUint, ctx: &NodeCtx) -> Result<BigUint, CoreError> {
    while !b.is_zero() {
        ctx.cancel.check()?;
        let r = a % &b;
        a = b;
        b = r;
    }
    Ok(a)
}

fn extgcd(a: &BigInt, b: &BigInt, ctx: &NodeCtx) -> Result<(BigInt, BigInt, BigInt), CoreError> {
    let (mut old_r, mut r) = (a.abs(), b.abs());
    let (mut old_s, mut s) = (BigInt::one(), BigInt::zero());
    let (mut old_t, mut t) = (BigInt::zero(), BigInt::one());
    let mut steps = 0;
    while !r.is_zero() {
        ctx.progress(0.1 + 0.7 * steps as f32 / (steps + 32) as f32);
        ctx.cancel.check()?;
        let q = &old_r / &r;
        (old_r, r) = (r.clone(), old_r - &q * r);
        (old_s, s) = (s.clone(), old_s - &q * s);
        (old_t, t) = (t.clone(), old_t - q * t);
        steps += 1;
    }
    if a.is_negative() {
        old_s = -old_s;
    }
    if b.is_negative() {
        old_t = -old_t;
    }
    Ok((old_r, old_s, old_t))
}

fn power(a: &BigInt, b: &BigInt, ctx: &NodeCtx) -> Result<BigInt, CoreError> {
    let exp = b
        .to_u32()
        .filter(|&n| n <= 10_000_000)
        .ok_or_else(|| err("整数幂指数 b 必须为 0–10000000"))?;
    if a.bits() > 1 && a.bits() * u64::from(exp) > 1_000_000 {
        return Err(err("整数幂估计结果超过 100 万位二进制，请减少底数或指数"));
    }
    let mut base = a.clone();
    let mut out = BigInt::one();
    let bits = u32::BITS - exp.leading_zeros();
    for i in 0..bits {
        ctx.progress(i as f32 / bits as f32);
        ctx.cancel.check()?;
        if exp & (1 << i) != 0 {
            out *= &base;
        }
        if i + 1 < bits {
            base = &base * &base;
        }
    }
    Ok(out)
}

fn sqrt(n: &BigUint, ctx: &NodeCtx) -> Result<BigUint, CoreError> {
    if n < &BigUint::from(2u8) {
        return Ok(n.clone());
    }
    let mut x = BigUint::one() << n.bits().div_ceil(2) as usize;
    let mut steps = 0;
    loop {
        ctx.progress(0.1 + 0.7 * steps as f32 / (steps + 8) as f32);
        ctx.cancel.check()?;
        let y = (&x + n / &x) >> 1usize;
        if y >= x {
            return Ok(x);
        }
        x = y;
        steps += 1;
    }
}

fn prime(n: &BigUint, ctx: &NodeCtx, start: f32, span: f32) -> Result<bool, CoreError> {
    if n < &BigUint::from(2u8) {
        return Ok(false);
    }
    for p in SMALL_PRIMES.iter().take(168) {
        ctx.cancel.check()?;
        if (n % p).is_zero() {
            return Ok(n == &BigUint::from(*p));
        }
    }
    Ok(probable(n, 32, ctx, start, span)?.0)
}

fn classification(n: &BigInt, passes: bool) -> &'static str {
    if n < &BigInt::from(2) {
        "not-prime"
    } else if !passes {
        "composite"
    } else if n.magnitude() < &*BOUND {
        "prime"
    } else {
        "probable-prime"
    }
}

fn neighbor(a: &BigInt, next: bool, ctx: &NodeCtx) -> Result<BigUint, CoreError> {
    if next && a < &BigInt::from(2) {
        return Ok(BigUint::from(2u8));
    }
    if !next && a <= &BigInt::from(2) {
        return Err(err("a 以下没有素数，prevPrime 需 a >= 3"));
    }
    let mut c = if next {
        a.magnitude() + 1u8
    } else {
        a.magnitude() - 1u8
    };
    if !next && c == BigUint::from(2u8) {
        return Ok(c);
    }
    if c.is_even() {
        if next {
            c += 1u8;
        } else {
            c -= 1u8;
        }
    }
    for step in 0..5_000_000 {
        let at = |s| 0.05 + 0.9 * s as f32 / (s + 64) as f32;
        ctx.progress(at(step));
        ctx.cancel.check()?;
        if prime(&c, ctx, at(step), at(step + 1) - at(step))? {
            return Ok(c);
        }
        if next {
            c += 2u8;
        } else {
            c -= 2u8;
        }
    }
    Err(err("邻素数搜索超过 5000000 个候选，请缩小输入范围"))
}

fn random_below(n: &BigUint, ctx: &NodeCtx) -> Result<BigUint, CoreError> {
    let mut data = vec![0; n.bits().div_ceil(8) as usize];
    for _ in 0..1024 {
        ctx.cancel.check()?;
        getrandom::getrandom(&mut data).map_err(|e| err(format!("系统随机源失败：{e}")))?;
        data[0] &= 0xff >> (data.len() * 8 - n.bits() as usize);
        let value = BigUint::from_bytes_be(&data);
        if &value < n {
            return Ok(value);
        }
    }
    Err(err("系统随机源连续拒绝采样失败"))
}

struct RhoWork<'a, 'b> {
    ctx: &'a NodeCtx<'b>,
    budget: usize,
    used: usize,
}
impl RhoWork<'_, '_> {
    fn progress(&self) -> f32 {
        0.05 + 0.9 * self.used as f32 / self.budget.max(1) as f32
    }
    fn tick(&mut self) -> Result<(), CoreError> {
        self.used += 1;
        if self.used.is_multiple_of(256) {
            self.ctx.progress(self.progress());
            self.ctx.cancel.check()?;
        }
        Ok(())
    }
    fn brent(&mut self, n: &BigUint) -> Result<Option<BigUint>, CoreError> {
        let minus = n - 1u8;
        for _ in 0..3 {
            self.ctx.cancel.check()?;
            let end = self.budget.min(self.used + (1 << 20));
            if self.used >= end {
                break;
            }
            let mut y = random_below(&minus, self.ctx)? + 1u8;
            let c = random_below(&minus, self.ctx)? + 1u8;
            let (mut x, mut ys) = (y.clone(), y.clone());
            let (mut g, mut q) = (BigUint::one(), BigUint::one());
            let mut r = 1usize;
            while g.is_one() && self.used < end {
                x = y.clone();
                for _ in 0..r.min(end - self.used) {
                    y = (&y * &y + &c) % n;
                    self.tick()?;
                }
                let mut k = 0;
                while k < r && g.is_one() && self.used < end {
                    ys = y.clone();
                    for _ in 0..128.min(r - k).min(end - self.used) {
                        y = (&y * &y + &c) % n;
                        let diff = if x > y { &x - &y } else { &y - &x };
                        q = q * diff % n;
                        self.tick()?;
                    }
                    g = gcd(q.clone(), n.clone(), self.ctx)?;
                    k += 128;
                }
                r *= 2;
            }
            if &g == n {
                g = BigUint::one();
                // Unlike the reference's unlimited rollback, every step consumes
                // the same finite budget and observes cancellation.
                while g.is_one() && self.used < end {
                    ys = (&ys * &ys + &c) % n;
                    let diff = if x > ys { &x - &ys } else { &ys - &x };
                    g = gcd(diff, n.clone(), self.ctx)?;
                    self.tick()?;
                }
            }
            if g > BigUint::one() && &g < n {
                return Ok(Some(g));
            }
        }
        Ok(None)
    }
}

fn factor(n: &BigUint, budget: usize, ctx: &NodeCtx) -> Result<(String, Value), CoreError> {
    let mut work = RhoWork {
        ctx,
        budget,
        used: 0,
    };
    let (mut factors, mut unfinished) = (Vec::new(), Vec::new());
    let mut m = n.clone();
    for (i, &p) in SMALL_PRIMES.iter().enumerate() {
        ctx.progress(0.05 * i as f32 / SMALL_PRIMES.len() as f32);
        ctx.cancel.check()?;
        if BigUint::from(u64::from(p) * u64::from(p)) > m {
            break;
        }
        while (&m % p).is_zero() {
            ctx.cancel.check()?;
            factors.push(BigUint::from(p));
            m /= p;
        }
    }
    let mut stack = if m > BigUint::one() {
        vec![m]
    } else {
        Vec::new()
    };
    while let Some(t) = stack.pop() {
        ctx.cancel.check()?;
        if prime(&t, ctx, work.progress(), 0.0)? {
            factors.push(t);
        } else if let Some(d) = work.brent(&t)? {
            stack.push(&t / &d);
            stack.push(d);
        } else {
            unfinished.push(t);
        }
    }
    factors.sort();
    unfinished.sort();
    let mut parts: Vec<String> = factors
        .iter()
        .map(|p| {
            if p < &*BOUND {
                p.to_string()
            } else {
                format!("{p}（可能素数）")
            }
        })
        .collect();
    parts.extend(unfinished.iter().map(|n| format!("{n}（过大未分解）")));
    let mut lines = parts.clone();
    let suffix = if factors.len() == 1 && unfinished.is_empty() && &factors[0] == n {
        if n < &*BOUND {
            "（素数）"
        } else {
            "（可能素数，未证明）"
        }
    } else {
        ""
    };
    if n.is_one() {
        lines.push("1 = 1（无素因子，空积）".into());
    } else {
        lines.push(format!("{n} = {}{suffix}", parts.join(" × ")));
    }
    let detail = json!({
        "factors":factors.iter().map(|p| json!({"value":p.to_string(),"deterministic":p < &*BOUND})).collect::<Vec<_>>(),
        "unfinished":unfinished.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "splitComplete":unfinished.is_empty(),
        "provenComplete":unfinished.is_empty() && factors.iter().all(|p| p < &*BOUND),
        "rhoSteps":work.used,"factorBudget":budget,
        "note":"超出确定性界的通过者只标可能素数；未拆开的合数保留原值；全部因子和剩余部分的乘积等于输入。"
    });
    Ok((lines.join("\n"), detail))
}

struct BigIntegerCalc;
impl Node for BigIntegerCalc {
    fn run(&self, inputs: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let op = match p.get("op") {
            Some(v) => v.as_str().ok_or_else(|| err("运算 op 必须为文本"))?,
            None => "add",
        };
        if !OPS.contains(&op) {
            return Err(err(format!("未知大整数运算：{op}")));
        }
        let a = operand(inputs, p, "a", "42")?;
        let b = if BINARY.contains(&op) {
            operand(inputs, p, "b", "17")?
        } else {
            BigInt::zero()
        };
        let m = if ["modpow", "modinv"].contains(&op) {
            operand(inputs, p, "m", "101")?
        } else {
            BigInt::zero()
        };
        if ["modpow", "modinv"].contains(&op) && m < BigInt::one() {
            return Err(err("模数 m 需为 >= 1 的整数"));
        }
        if ["div", "mod"].contains(&op) && b.is_zero() {
            return Err(err("除数 b 不能为 0"));
        }
        let (text, result) = match op {
            "extgcd" => {
                let (g, x, y) = extgcd(&a, &b, ctx)?;
                (
                    format!("g = {g}\nx = {x}\ny = {y}\n校验：{a} × ({x}) + {b} × ({y}) = {g}"),
                    json!({"g":g.to_string(),"x":x.to_string(),"y":y.to_string()}),
                )
            }
            "isPrime" => {
                let passes = a >= BigInt::from(2) && prime(a.magnitude(), ctx, 0.0, 0.95)?;
                let kind = classification(&a, passes);
                let word = match kind {
                    "not-prime" => "非素数（n < 2）",
                    "prime" => "素数",
                    "composite" => "合数",
                    _ => "可能素数",
                };
                (
                    format!("{word}（{} 位二进制）", a.bits()),
                    json!({"classification":kind,"passed":passes,"bits":a.bits(),"deterministic":kind!="probable-prime","deterministicBound":BOUND.to_string(),"basis":"fixed-small-primes","rounds":if a.magnitude() < &*BOUND {13} else {32}}),
                )
            }
            "factor" => {
                if a < BigInt::one() {
                    return Err(err("素因子分解需 a >= 1"));
                }
                let budget = numeric_support::number(
                    p,
                    "factorBudget",
                    RHO_BUDGET as i64,
                    0,
                    RHO_BUDGET as i64,
                )? as usize;
                factor(a.magnitude(), budget, ctx)?
            }
            "nextPrime" | "prevPrime" => {
                let n = neighbor(&a, op == "nextPrime", ctx)?;
                (
                    n.to_string(),
                    json!({"value":n.to_string(),"deterministic":n < *BOUND,"classification":if n < *BOUND {"prime"}else{"probable-prime"},"note":"严格相邻的通过检验者；超出确定性范围不构成素性证明。"}),
                )
            }
            _ => {
                let value: BigInt = match op {
                    "add" => &a + &b,
                    "sub" => &a - &b,
                    "mul" => &a * &b,
                    "div" => &a / &b,
                    "mod" => &a % &b,
                    "pow" => power(&a, &b, ctx)?,
                    "modpow" => {
                        if b.is_negative() {
                            return Err(err("模幂指数需为非负整数"));
                        }
                        if m.is_one() {
                            BigInt::zero()
                        } else {
                            modpow(
                                a.mod_floor(&m).into_parts().1,
                                b.magnitude(),
                                m.magnitude(),
                                ctx,
                                0.0,
                                0.95,
                            )?
                            .into()
                        }
                    }
                    "modinv" => {
                        let (g, x, _) = extgcd(&a.mod_floor(&m), &m, ctx)?;
                        if !g.is_one() {
                            return Err(err(format!("不存在模逆：gcd(a, m) = {g}，需为 1")));
                        }
                        x.mod_floor(&m)
                    }
                    "gcd" => gcd(a.magnitude().clone(), b.magnitude().clone(), ctx)?.into(),
                    "lcm" => {
                        let g = gcd(a.magnitude().clone(), b.magnitude().clone(), ctx)?;
                        if g.is_zero() {
                            BigInt::zero()
                        } else {
                            (a.magnitude() / g * b.magnitude()).into()
                        }
                    }
                    "sqrtInt" => {
                        if a.is_negative() {
                            return Err(err("负数无整数平方根"));
                        }
                        sqrt(a.magnitude(), ctx)?.into()
                    }
                    "bitLen" => a.bits().into(),
                    _ => unreachable!(),
                };
                let text = value.to_string();
                (text.clone(), Value::String(text))
            }
        };
        let mut out = out_text(text);
        out.insert("json".into(),PortValue::Json(json!({"op":op,"operands":{"a":a.to_string(),"b":if BINARY.contains(&op){Some(b.to_string())}else{None},"m":if ["modpow","modinv"].contains(&op){Some(m.to_string())}else{None}},"result":result})));
        ctx.progress(1.0);
        ctx.cancel.check()?;
        Ok(out)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let mut d = document_text(desc("big_integer_calc","numeric.math","大整数计算器 BigInt",CYAN,
        vec![opt("a","操作数 a（优先）",PortType::Text),opt("b","操作数 b（优先）",PortType::Text),opt("m","模数 m（优先）",PortType::Text)],
        vec![t_out(),req("json","精确结果与计算依据",PortType::Json)],
        vec![help(ParamSpec::select("op","运算",&OPS,"add"),"17 种运算：四则/余数/幂/模幂/模逆/gcd/extgcd/lcm/素性/邻素数/分解/整数开方/位长。div 向零截断，mod 符号随 a。"),
        help(ParamSpec::text("a","十进制 a","42",true),"可连接 a 覆盖；支持正负号、空白、逗号、下划线。每个操作数上限 4096 位二进制；必须用文本保留精度。"),
        help(ParamSpec::text("b","十进制 b / 指数","17",true),"二元运算使用，连接 b 优先。pow 指数 0–10000000、结果估计不超过 100 万位；modpow 指数非负。"),
        help(ParamSpec::text("m","十进制模数 m","101",true),"modpow/modinv 使用，连接 m 优先；模数至少为 1。模逆不存在时报错并报告 gcd。"),
        help(ParamSpec::number("factorBudget","分解迭代预算",0.0,RHO_BUDGET as f64,1.0,RHO_BUDGET as f64),"factor 的全部 Pollard–Brent 正向/回退迭代共用预算；0 仅小素数试除和素性检验。耗尽时保留未分解项，支持取消。")]),
        "bigCalc","纯 Rust 精确大整数计算与数论；a/b/m 连线优先于侧栏参数，十进制文本和 JSON 均不经浮点。素性采用固定基：小于 3317044064679887385961981 为确定性范围，更大通过者仅为可能素数。分解保留未完成部分并支持进度/取消。","42",json!({"op":"modinv","m":"101"}));
    d.presentation.inline_params = vec!["op".into()];
    d.family = "big integer number theory".into();
    d.aliases.extend(
        [
            "bigCalc",
            "BigInt",
            "Bézout",
            "Pollard rho",
            "大数计算",
            "模逆",
            "整数开方",
            "素因子分解",
        ]
        .map(String::from),
    );
    d.cost = Cost::Heavy;
    reg.register(d, Arc::new(|| Arc::new(BigIntegerCalc)));
}
