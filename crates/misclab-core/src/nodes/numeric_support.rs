//! Bounded, exact integer helpers for migrated numeral systems.
use super::prelude::*;
use num_bigint::BigInt;
use num_traits::Num;

pub const MAX_DIGITS: usize = 16_384;
pub fn bad(message: &str) -> CoreError {
    CoreError::Parse(message.into())
}
pub fn space(c: char) -> bool {
    (c.is_whitespace() && c != '\u{85}') || c == '\u{feff}'
}
pub fn compact(s: &str) -> String {
    s.chars().filter(|c| !space(*c)).collect()
}
pub fn integer(s: &str, radix: u32) -> Result<BigInt, CoreError> {
    let s = s.trim_matches(space);
    if s.len() > MAX_DIGITS {
        return Err(bad("整数输入最多 16384 个字符"));
    }
    let digits = s.strip_prefix(['+', '-']).unwrap_or(s);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii() && c.is_digit(radix)) {
        return Err(bad("需要完整整数"));
    }
    BigInt::from_str_radix(s, radix).map_err(|_| bad("整数包含无效字符或超出所选进制"))
}
pub fn number(
    p: &serde_json::Value,
    key: &str,
    default: i64,
    min: i64,
    max: i64,
) -> Result<i64, CoreError> {
    let n = match p.get(key) {
        None => default as f64,
        Some(v) => v
            .as_f64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            .ok_or_else(|| bad("数值参数类型无效"))?,
    };
    if !n.is_finite() || n.fract() != 0.0 || n < min as f64 || n > max as f64 {
        return Err(bad(&format!("{key} 必须是 {min}..{max} 范围的整数")));
    }
    Ok(n as i64)
}
pub fn fraction(s: &str) -> Result<(BigInt, BigInt), CoreError> {
    let (a, b) = s.split_once('/').unwrap_or((s, "1"));
    Ok((integer(a, 10)?, integer(b, 10)?))
}
