//! Exact rational conversion tables from the pinned unitConv.js operation.
//! Static coefficients are data; all parsing, arithmetic and rendering is Rust.
use super::{
    codec_support::{document_text, help},
    numeric_support,
    prelude::*,
    transport_codecs::err,
};
use chrono::{Datelike, Timelike};
use num_bigint::{BigInt, BigUint};
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};
use serde_json::{json, Value};
use std::sync::LazyLock;

static TABLES: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("unit_definitions.json")).expect("validated unit data")
});
const EPOCHS: [&str; 6] = [
    "auto",
    "unix_s",
    "unix_ms",
    "filetime",
    "cocoa",
    "chrome_us",
];

#[derive(Clone)]
struct Rat {
    n: BigInt,
    d: BigInt,
}
impl Rat {
    fn new(n: BigInt, d: BigInt) -> Self {
        debug_assert!(!d.is_zero());
        let g = n.gcd(&d);
        let (n, d) = (n / &g, d / g);
        if d.is_negative() {
            Self { n: -n, d: -d }
        } else {
            Self { n, d }
        }
    }
    fn int(n: i64) -> Self {
        Self {
            n: n.into(),
            d: BigInt::one(),
        }
    }
    fn mul(&self, b: &Self) -> Self {
        Self::new(&self.n * &b.n, &self.d * &b.d)
    }
    fn div(&self, b: &Self) -> Self {
        Self::new(&self.n * &b.d, &self.d * &b.n)
    }
    fn add(&self, b: &Self) -> Self {
        Self::new(&self.n * &b.d + &b.n * &self.d, &self.d * &b.d)
    }
    fn float(&self) -> f64 {
        self.n.to_f64().unwrap_or(if self.n.is_negative() {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        }) / self.d.to_f64().unwrap_or(f64::INFINITY)
    }
    fn from_data(v: &Value) -> Self {
        Self::new(
            BigInt::parse_bytes(v["n"].as_str().unwrap().as_bytes(), 10).unwrap(),
            BigInt::parse_bytes(v["d"].as_str().unwrap().as_bytes(), 10).unwrap(),
        )
    }
    fn shown(&self) -> (String, bool) {
        if self.n.is_zero() {
            return ("0".into(), true);
        }
        let n = self.n.abs();
        if &n * 1_000_000u32 < self.d {
            return (precision(self.float(), 13), false);
        }
        let (integer, mut rem) = n.div_rem(&self.d);
        let mut frac = String::new();
        for _ in 0..12 {
            if rem.is_zero() {
                break;
            }
            rem *= 10;
            let (q, r) = rem.div_rem(&self.d);
            frac.push(char::from(b'0' + q.to_u8().unwrap()));
            rem = r;
        }
        let exact = rem.is_zero();
        let frac = frac.trim_end_matches('0');
        let mut s = integer.to_string();
        if !frac.is_empty() {
            s.push('.');
            s.push_str(frac);
        }
        if !exact {
            s.push('…');
        }
        if self.n.is_negative() {
            s.insert(0, '-');
        }
        (s, exact)
    }
    fn row(&self, group: &str, unit: &str) -> Value {
        let (value, display_exact) = self.shown();
        json!({"group":group,"unit":unit,"value":value,"numerator":self.n.to_string(),"denominator":self.d.to_string(),"exact":true,"displayExact":display_exact})
    }
}

/// ECMAScript-style significant digits: round the exact binary64 ratio,
/// ties toward the larger magnitude, and retain significant trailing zeros.
fn precision(n: f64, p: usize) -> String {
    if !n.is_finite() {
        return ryu_js::Buffer::new().format(n).into();
    }
    if n == 0.0 {
        return format!("0.{}", "0".repeat(p - 1));
    }
    let bits = n.abs().to_bits();
    let exponent = ((bits >> 52) & 2047) as i32;
    let mantissa = (bits & ((1u64 << 52) - 1)) | if exponent == 0 { 0 } else { 1u64 << 52 };
    let binary_exp = if exponent == 0 {
        -1074
    } else {
        exponent - 1023 - 52
    };
    let (numerator, denominator) = if binary_exp >= 0 {
        (
            BigUint::from(mantissa) << binary_exp as usize,
            BigUint::one(),
        )
    } else {
        (
            BigUint::from(mantissa),
            BigUint::one() << (-binary_exp) as usize,
        )
    };
    let ten = BigUint::from(10u8);
    let less_than_power = |e: i32| {
        if e >= 0 {
            numerator < &denominator * ten.pow(e as u32)
        } else {
            &numerator * ten.pow((-e) as u32) < denominator
        }
    };
    let mut e = n.abs().log10().floor() as i32;
    while less_than_power(e) {
        e -= 1;
    }
    while !less_than_power(e + 1) {
        e += 1;
    }
    let scale = p as i32 - 1 - e;
    let (num, den) = if scale >= 0 {
        (numerator * ten.pow(scale as u32), denominator)
    } else {
        (numerator, denominator * ten.pow((-scale) as u32))
    };
    let (mut q, r) = num.div_rem(&den);
    if r * 2u8 >= den {
        q += 1u8;
    }
    let mut digits = q.to_string();
    if digits.len() > p {
        digits.pop();
        e += 1;
    }
    let body = if e < -6 || e >= p as i32 {
        format!("{}.{}e{e:+}", &digits[..1], &digits[1..])
    } else if e >= 0 {
        let at = e as usize + 1;
        if at == p {
            digits
        } else {
            format!("{}.{}", &digits[..at], &digits[at..])
        }
    } else {
        format!("0.{}{}", "0".repeat((-e - 1) as usize), digits)
    };
    if n < 0.0 {
        format!("-{body}")
    } else {
        body
    }
}

fn decimal(src: &str) -> Result<Rat, CoreError> {
    let neg = src.starts_with('-');
    let raw = src.strip_prefix(['-', '+']).unwrap_or(src);
    let (whole, frac) = raw.split_once('.').unwrap_or((raw, ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !frac.bytes().all(|b| b.is_ascii_digit())
        || (raw.contains('.') && frac.is_empty())
        || frac.len() > 18
    {
        return Err(err(
            "需要十进制整数或小数，小数最多 18 位；不接受指数、千分位或省略整数部分",
        ));
    }
    let d = BigInt::from(10u8).pow(frac.len() as u32);
    let n = BigInt::parse_bytes(format!("{whole}{frac}").as_bytes(), 10)
        .ok_or_else(|| err("数值无效"))?;
    Ok(Rat::new(if neg { -n } else { n }, d))
}
fn sniff(qty: &Rat) -> &'static str {
    let n = qty.n.abs() / &qty.d;
    if n >= BigInt::from(100_000_000_000_000_000i64) {
        "filetime"
    } else if n >= BigInt::from(100_000_000_000_000i64) {
        "chrome_us"
    } else if n >= BigInt::from(100_000_000_000i64) {
        "unix_ms"
    } else {
        "unix_s"
    }
}
fn iso(ms: i64) -> Option<String> {
    let d = super::time_codecs::date(ms).ok()?;
    let year = d.year();
    let year = if (0..=9999).contains(&year) {
        format!("{year:04}")
    } else {
        format!("{year:+07}")
    };
    Some(format!(
        "{year}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        d.month(),
        d.day(),
        d.hour(),
        d.minute(),
        d.second(),
        d.timestamp_subsec_millis()
    ))
}
fn timestamp(qty: &Rat, want: &str) -> Result<(Vec<Value>, Value, Vec<String>), CoreError> {
    let epoch = if want == "auto" { sniff(qty) } else { want };
    let sec = match epoch {
        "unix_s" => qty.clone(),
        "unix_ms" => qty.div(&Rat::int(1000)),
        "filetime" => qty
            .div(&Rat::int(10_000_000))
            .add(&Rat::int(-11_644_473_600)),
        "cocoa" => qty.add(&Rat::int(978_307_200)),
        "chrome_us" => qty
            .div(&Rat::int(1_000_000))
            .add(&Rat::int(-11_644_473_600)),
        _ => return Err(err("未知时间戳纪元")),
    };
    let mut rows = Vec::new();
    for (u, v) in [
        ("unix_s", sec.clone()),
        ("unix_ms", sec.mul(&Rat::int(1000))),
        (
            "filetime",
            sec.add(&Rat::int(11_644_473_600))
                .mul(&Rat::int(10_000_000)),
        ),
        ("cocoa", sec.add(&Rat::int(-978_307_200))),
        (
            "chrome_us",
            sec.add(&Rat::int(11_644_473_600)).mul(&Rat::int(1_000_000)),
        ),
    ] {
        rows.push(v.row("epoch", u));
    }
    // Keep fractional seconds for all epoch conversions; only the calendar
    // view truncates to milliseconds. The reference drops them too early.
    let millis = (&sec.n * 1000u32 / &sec.d)
        .to_i64()
        .filter(|n| n.unsigned_abs() <= 8_640_000_000_000_000);
    let iso = millis.and_then(iso);
    // DOS reference semantics: round to the nearest second, ties toward +∞,
    // then store seconds / 2 in the FAT fields. Both steps are explicit.
    let rounded = (&sec.n * 2u8 + &sec.d).div_floor(&(&sec.d * 2u8)).to_i64();
    let dos = rounded
        .and_then(|n| n.checked_mul(1000))
        .and_then(|ms| super::time_codecs::date(ms).ok())
        .filter(|d| (1980..=2107).contains(&d.year()))
        .map(|d| {
            let date = (((d.year() - 1980) as u32) << 9) | (d.month() << 5) | d.day();
            let time = (d.hour() << 11) | (d.minute() << 5) | (d.second() / 2);
            format!("0x{:08X}", (date << 16) | time)
        });
    let meta = json!({"epoch":epoch,"autoDetected":want=="auto","unixSeconds":{"numerator":sec.n.to_string(),"denominator":sec.d.to_string()},"isoUtc":iso,"dosUtc":dos});
    let notes = vec![
        format!(
            "输入按 {epoch} 解读{}；小数秒完整保留在各纪元有理数结果中。",
            if want == "auto" {
                "（数量级嗅探，可用参数覆盖）"
            } else {
                ""
            }
        ),
        format!(
            "ISO8601(UTC)：{}",
            iso.as_deref()
                .unwrap_or("超出日历显示范围，精确纪元值仍可用")
        ),
        format!(
            "DOS(UTC)：{}；1980–2107，先四舍五入到秒（半秒向正无穷）再保存 2 秒精度。",
            dos.as_deref().unwrap_or("超出 DOS 年份范围")
        ),
    ];
    Ok((rows, meta, notes))
}
fn angle(qty: &Rat, unit: &str) -> Result<Vec<Value>, CoreError> {
    let d = qty.float()
        * match unit {
            "rad" => 180.0 / std::f64::consts::PI,
            "gon" => 0.9,
            _ => 1.0,
        };
    let rad = d * std::f64::consts::PI / 180.0;
    let gon = d * 10.0 / 9.0;
    let mut rows = Vec::new();
    for (u, n) in [("deg", d), ("rad", rad), ("gon", gon)] {
        if !n.is_finite() {
            return Err(err("角度转换超出双精度可表示范围"));
        }
        let rounded: f64 = precision(n, 15).parse().map_err(|_| err("角度显示失败"))?;
        let value = ryu_js::Buffer::new().format(rounded).to_owned();
        rows.push(json!({"group":"angle","unit":u,"value":value,"exact":false,"displayExact":false,"precision":15}));
    }
    Ok(rows)
}

struct UnitConvert;
impl Node for UnitConvert {
    fn run(&self, inputs: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let raw = in_text(inputs, "text")?;
        if raw.len() > 16384 {
            return Err(err("单位换算原始输入最多 16384 字节"));
        }
        let input = raw.trim_matches(numeric_support::space);
        if input.is_empty() || input.encode_utf16().count() > 4096 {
            return Err(err("单位换算输入需为 1–4096 字符"));
        }
        let start = usize::from(input.starts_with(['+', '-']));
        let end = start
            + input.as_bytes()[start..]
                .iter()
                .take_while(|b| b.is_ascii_digit() || **b == b'.')
                .count();
        let qty = decimal(&input[..end])?;
        let unit = input[end..].trim_start_matches(numeric_support::space);
        let unit = TABLES["aliases"]
            .get(unit)
            .and_then(Value::as_str)
            .unwrap_or(unit);
        let mut rows = Vec::new();
        let mut notes = Vec::new();
        let mut metadata = Value::Null;
        let category;
        if unit.is_empty() {
            let want = match p.get("tsEpoch") {
                Some(v) => v.as_str().ok_or_else(|| err("tsEpoch 需为文本"))?,
                None => "auto",
            };
            if !EPOCHS.contains(&want) {
                return Err(err("未知时间戳纪元"));
            }
            (rows, metadata, notes) = timestamp(&qty, want)?;
            category = "epoch";
        } else if ["deg", "rad", "gon"].contains(&unit) {
            rows = angle(&qty, unit)?;
            category = "angle";
            notes.push(
                "角度按上游双精度路径展示 15 位有效数字；涉及弧度的换算包含 π，结果为近似值。"
                    .into(),
            );
        } else {
            let groups = TABLES["groups"].as_array().unwrap();
            let currencies = TABLES["currencies"].as_array().unwrap();
            let matching: Vec<_> = groups
                .iter()
                .chain(currencies)
                .filter(|g| {
                    g["units"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|u| u["u"] == unit)
                })
                .collect();
            if matching.is_empty() {
                return Err(err(format!(
                    "未知单位：{unit}；单位区分大小写，如 MB/s 与 Mbps"
                )));
            }
            category = matching[0]["id"].as_str().unwrap();
            for g in matching {
                let units = g["units"].as_array().unwrap();
                let source = units.iter().find(|u| u["u"] == unit).unwrap();
                let base = qty.mul(&Rat::from_data(&source["k"]));
                for target in units {
                    ctx.progress(0.1 + 0.8 * rows.len() as f32 / (rows.len() + 20) as f32);
                    ctx.cancel.check()?;
                    let n = base.div(&Rat::from_data(&target["k"]));
                    let mut row = n.row(g["id"].as_str().unwrap(), target["u"].as_str().unwrap());
                    row["groupName"] = g["name"].clone();
                    row["label"] = target
                        .get("zh")
                        .cloned()
                        .unwrap_or_else(|| target["u"].clone());
                    if let Some(tag) = target.get("sys").or_else(|| target.get("dim")) {
                        row["system"] = tag.clone();
                    }
                    rows.push(row);
                }
                if let Some(note) = g["note"].as_str() {
                    notes.push(format!("{}：{note}", g["name"].as_str().unwrap()));
                }
            }
            notes.push("系数采用固定上游版本；精确分子/分母保存在结果中。小数最多 12 位，未尽部分标 …；小于 10^-6 的非零值以 13 位有效数字近似展示。".into());
            if category.starts_with("cur_") {
                notes.push(
                    "歧义单位逐个列出匹配的币种组；仅同币种面额换算，各组独立，不涉及汇率。".into(),
                );
            }
        }
        let title = match category {
            "dataSize" => "数据量",
            "dataRate" => "数据速率",
            "time" => "时间",
            "freq" => "频率",
            "angle" => "角度",
            "length" => "长度",
            "speed" => "速度",
            "epoch" => "时间戳纪元",
            _ => "货币面额",
        };
        let mut report = vec![
            format!("单位换算｜{title}"),
            format!("输入：{} {unit}", qty.shown().0),
            String::new(),
        ];
        for row in &rows {
            report.push(format!(
                "[{}] {} = {}{}",
                row["groupName"].as_str().unwrap_or(title),
                row["unit"].as_str().unwrap(),
                row["value"].as_str().unwrap(),
                if row["exact"] == false {
                    "（近似）"
                } else {
                    ""
                }
            ));
        }
        report.push(String::new());
        report.extend(notes);
        let mut out = out_text(report.join("\n"));
        out.insert("rows".into(), PortValue::Json(Value::Array(rows.clone())));
        out.insert("json".into(),PortValue::Json(json!({"category":category,"unit":unit,"input":{"numerator":qty.n.to_string(),"denominator":qty.d.to_string()},"rows":rows,"timestamp":metadata})));
        ctx.progress(1.0);
        ctx.cancel.check()?;
        Ok(out)
    }
}
pub fn register(reg: &mut NodeRegistry) {
    let mut d=document_text(desc("unit_convert","numeric.address","单位与纪元换算",CYAN,vec![t_in()],vec![t_out(),req("rows","换算表",PortType::Json),req("json","精确分数与纪元依据",PortType::Json)],vec![help(ParamSpec::select("tsEpoch","纯数字纪元",&EPOCHS,"auto"),"输入纯数字时使用：auto 按数量级嗅探 Unix 秒/毫秒、FILETIME 或 Chrome 微秒；可显式指定 Cocoa。小数秒完整保留；有单位时忽略此参数。")]),"unitConv","74 种单位与 73 个别名：SI/IEC 数据量、bit/Byte 速率、时间、频率、角度、长度、速度、同币种面额及时间戳纪元对照。数值支持十进制正负整数/小数（最多 18 位小数），有理数结果保存完整分子/分母；角度仅近似。","1 GB",json!({}));
    d.presentation
        .previews
        .insert("rows".into(), "table".into());
    d.presentation.inline_params = vec!["tsEpoch".into()];
    d.family = "unit conversion rational arithmetic epochs".into();
    d.aliases.extend(
        [
            "unitConv",
            "单位换算",
            "GB GiB",
            "Mbps MB/s",
            "长度速度",
            "元角分",
            "纪元对照",
        ]
        .map(String::from),
    );
    reg.register(d, Arc::new(|| Arc::new(UnitConvert)));
}
