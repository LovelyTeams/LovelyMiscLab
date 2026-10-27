//! Cancellable Miller–Rabin and CSPRNG candidate generation.
//! The 13-base deterministic bound is Sorenson/Webster, Theorem 1.1:
//! https://arxiv.org/abs/1509.00864 (not a FIPS random-base error bound).
use super::{
    codec_support::{document_text, help},
    numeric_support,
    prelude::*,
    transport_codecs::err,
};
use num_bigint::{BigInt, BigUint};
use num_integer::Integer;
use num_traits::{One, Signed};
use serde_json::{json, Value};
use std::sync::LazyLock;
const BASES: [u32; 64] = [
    2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97,
    101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167, 173, 179, 181, 191, 193,
    197, 199, 211, 223, 227, 229, 233, 239, 241, 251, 257, 263, 269, 271, 277, 281, 283, 293, 307,
    311,
];
static BOUND: LazyLock<BigUint> =
    LazyLock::new(|| BigUint::parse_bytes(b"3317044064679887385961981", 10).unwrap());
fn modpow(
    mut base: BigUint,
    exp: &BigUint,
    n: &BigUint,
    ctx: &NodeCtx,
    start: f32,
    span: f32,
) -> Result<BigUint, CoreError> {
    let mut out = BigUint::one();
    base %= n;
    for i in 0..exp.bits() {
        if i.is_multiple_of(32) {
            ctx.progress(start + span * i as f32 / exp.bits().max(1) as f32);
            ctx.cancel.check()?;
        }
        if exp.bit(i) {
            out = out * &base % n;
        }
        base = &base * &base % n;
    }
    ctx.cancel.check()?;
    Ok(out)
}
fn probable(
    n: &BigUint,
    rounds: usize,
    ctx: &NodeCtx,
    start: f32,
    span: f32,
) -> Result<(bool, usize), CoreError> {
    ctx.cancel.check()?;
    if n < &BigUint::from(2u8) {
        return Ok((false, 0));
    }
    if n == &BigUint::from(2u8) || n == &BigUint::from(3u8) {
        return Ok((true, 0));
    }
    if n.is_even() {
        return Ok((false, 0));
    }
    let rounds = if n < &*BOUND { 13 } else { rounds };
    let minus = n - 1u8;
    let mut d = minus.clone();
    let mut r = 0usize;
    while d.is_even() {
        d >>= 1;
        r += 1;
    }
    let mut tested = 0;
    for (i, a) in BASES.iter().take(rounds).enumerate() {
        let base = BigUint::from(*a);
        if &base >= n {
            continue;
        }
        let mut x = modpow(
            base,
            &d,
            n,
            ctx,
            start + span * i as f32 / rounds as f32,
            span / rounds as f32,
        )?;
        tested += 1;
        if x.is_one() || x == minus {
            continue;
        }
        let mut passes = false;
        for _ in 1..r {
            ctx.cancel.check()?;
            x = &x * &x % n;
            if x == minus {
                passes = true;
                break;
            }
        }
        if !passes {
            return Ok((false, tested));
        }
    }
    Ok((true, tested))
}
fn parse_integer(raw: &str) -> Result<BigInt, CoreError> {
    if raw.len() > 16384 {
        return Err(err("素性检验输入最多 16384 字节"));
    }
    let s = raw
        .chars()
        .filter(|&c| !numeric_support::space(c) && c != ',' && c != '_')
        .collect::<String>();
    let n = numeric_support::integer(&s, 10)?;
    if n.bits() > 4096 {
        return Err(err("素性检验最多 4096 位整数"));
    }
    Ok(n)
}
struct PrimeTest;
impl Node for PrimeTest {
    fn run(&self, inputs: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let raw = match inputs.get("text") {
            Some(v) => v.as_text()?,
            None => pstr(p, "n", ""),
        };
        let n = parse_integer(raw)?;
        let rounds = numeric_support::number(p, "rounds", 24, 1, 64)? as usize;
        let abs = n.magnitude();
        let bits = abs.bits();
        let invalid = n.is_negative() || abs < &BigUint::from(2u8);
        let (passes, tested) = if invalid {
            (false, 0)
        } else {
            probable(abs, rounds, ctx, 0.0, 1.0)?
        };
        let deterministic = invalid || !passes || abs < &*BOUND;
        let classification = if invalid {
            "not-prime"
        } else if !passes {
            "composite"
        } else if deterministic {
            "prime"
        } else {
            "probable-prime"
        };
        let report = if invalid {
            format!("判定：非素数（{}）\n位长：{bits} 位（二进制{}）\n方法：定义直接判定（素数需 n >= 2，此处不涉及 Miller-Rabin）",if n.is_negative() {"负数"} else {"n < 2"},if n.is_negative() {"，负数按 |n| 计"} else {""})
        } else {
            let result = if !passes {
                "合数"
            } else if deterministic {
                "素数"
            } else {
                "可能素数"
            };
            let method = if abs < &*BOUND {
                "Miller-Rabin 13 个固定质数基（2–41）；n < 3317044064679887385961981，确定性范围来自 Sorenson/Webster".into()
            } else {
                format!("Miller-Rabin 前 {rounds} 个小质数固定基；通过检验不等于素性证明，固定基不适用随机基的 4^-t 误判界")
            };
            format!("判定：{result}\n位长：{bits} 位（二进制）\n方法：{method}")
        };
        let mut out = out_text(report);
        out.insert("probable".into(), PortValue::Bool(passes));
        out.insert("bits".into(), PortValue::Number(bits as f64));
        out.insert("json".into(),PortValue::Json(json!({"n":n.to_string(),"classification":classification,"passed":passes,"deterministic":deterministic,"bits":bits,"testedRounds":tested,"requestedRounds":rounds,"deterministicBound":BOUND.to_string(),"basis":"fixed-small-primes"})));
        ctx.cancel.check()?;
        Ok(out)
    }
}
fn random_odd(bits: usize) -> Result<BigUint, CoreError> {
    let mut data = vec![0; bits.div_ceil(8)];
    getrandom::getrandom(&mut data).map_err(|e| err(format!("系统随机源失败：{e}")))?;
    data[0] &= 0xff >> (data.len() * 8 - bits);
    let mut n = BigUint::from_bytes_be(&data);
    n.set_bit((bits - 1) as u64, true);
    n.set_bit(0, true);
    Ok(n)
}
struct PrimeGenerate;
impl Node for PrimeGenerate {
    fn run(&self, _inputs: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let bits = numeric_support::number(p, "bits", 64, 2, 1024)? as usize;
        let count = numeric_support::number(p, "count", 1, 1, 100)? as usize;
        let mut primes = Vec::with_capacity(count);
        let mut detail = Vec::new();
        for i in 0..count {
            let start = i as f32 / count as f32;
            ctx.progress(start);
            ctx.cancel.check()?;
            if bits == 2 {
                primes.push("3".into());
                detail.push(json!({"n":"3","deterministic":true,"attempts":1}));
                continue;
            }
            let mut n = random_odd(bits)?;
            let max_attempts = bits * 50;
            let mut found = false;
            for attempt in 0..=max_attempts {
                let span = 1.0 / count as f32 / (max_attempts + 1) as f32;
                let (passes, _) = probable(&n, 13, ctx, start + attempt as f32 * span, span)?;
                if passes {
                    let value = n.to_string();
                    detail.push(json!({"n":value,"deterministic":n<*BOUND,"attempts":attempt+1}));
                    primes.push(value);
                    found = true;
                    break;
                }
                n += 2u8;
                if n.bits() > bits as u64 {
                    n = random_odd(bits)?;
                }
            }
            if !found {
                return Err(err(format!(
                    "{max_attempts} 次尝试内未找到 {bits} 位候选，请重试"
                )));
            }
        }
        let mut out = out_text(primes.join("\n"));
        out.insert("primes".into(), PortValue::StringList(primes));
        out.insert("json".into(),PortValue::Json(json!({"bits":bits,"count":count,"values":detail,"basis":"13-fixed-small-primes","randomSource":"system-csprng","note":"超出确定性范围的结果是可能素数，不是素性证明"})));
        ctx.cancel.check()?;
        Ok(out)
    }
}
pub fn register(reg: &mut NodeRegistry) {
    let mut d=document_text(desc("prime_test","numeric.math","素性检验 Miller–Rabin",CYAN,vec![opt("text","十进制整数（优先）",PortType::Text)],vec![t_out(),req("probable","通过素性检验",PortType::Bool),req("bits","位长",PortType::Number),req("json","检验依据",PortType::Json)],vec![help(ParamSpec::text("n","待检整数 n","",true),"未连接 text 时使用；十进制，可含空格、逗号和下划线；最多 4096 位。"),help(ParamSpec::number("rounds","检验轮数",1.0,64.0,1.0,24.0),"小于确定性界时固定用前 13 个质数基；更大整数用前 rounds 个质数固定基。通过时仅报告可能素数。")]),"primeTest","检验十进制大整数，text 连线优先于 n 参数；报告确定性范围与可能素数的区别。大数使用固定基，不宣称随机基误判概率。模幂中可取消。","2305843009213693951",json!({}));
    d.presentation.inline_params = vec!["rounds".into()];
    d.aliases
        .extend(["primeTest", "primality", "Miller Rabin", "素数判断"].map(String::from));
    d.cost = Cost::Medium;
    reg.register(d, Arc::new(|| Arc::new(PrimeTest)));
    let mut d = desc(
        "prime_generate",
        "numeric.math",
        "大素数候选生成",
        CYAN,
        vec![],
        vec![
            t_out(),
            req("primes", "候选列表", PortType::StringList),
            req("json", "生成依据", PortType::Json),
        ],
        vec![
            help(
                ParamSpec::number("bits", "位数", 2.0, 1024.0, 1.0, 64.0),
                "2–1024 位；系统随机奇数逐个 +2，越界重新取随机数。2 位固定返回 3。",
            ),
            help(
                ParamSpec::number("count", "数量", 1.0, 100.0, 1.0, 1.0),
                "1–100 个；允许重复，与上游一致。每个候选用 13 个固定质数基检验。",
            ),
        ],
    );
    d.description="本地系统 CSPRNG 生成指定位数的奇数，经 Miller–Rabin 前 13 个固定基筛选；超过 3317044064679887385961981 的候选未证明素性，JSON 标出依据。支持进度/取消，每个候选最多 bits×50+1 次尝试。".into();
    d.upstream_ids = vec!["primeGen".into()];
    d.family = "prime generation".into();
    d.aliases = vec![
        "primeGen".into(),
        "prime generation".into(),
        "素数生成".into(),
    ];
    d.cost = Cost::Heavy;
    d.examples.push(crate::node::descriptor::NodeExample {
        label: "2 位候选（可复现）".into(),
        inputs: PortMap::new(),
        params: json!({"bits":2,"count":3}),
    });
    reg.register(d, Arc::new(|| Arc::new(PrimeGenerate)));
}
