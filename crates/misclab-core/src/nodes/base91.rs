//! basE91 bit packing, interoperable with EBCTFCodeBox base91.
use super::{codec_support::*, prelude::*};
use serde_json::Value;
const ALPHABET: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!#$%&()*+,./:;<=>?@[]^_`{|}~\"";
fn encode(data: &[u8], p: &Value) -> Result<String, CoreError> {
    let a = alphabet(pstr(p, "alphabet", ALPHABET), Some(91))?;
    let (mut b, mut n) = (0u32, 0u32);
    let mut out = String::new();
    for &byte in data {
        b |= (byte as u32) << n;
        n += 8;
        if n > 13 {
            let mut v = b & 8191;
            let width = if v > 88 {
                13
            } else {
                v = b & 16383;
                14
            };
            b >>= width;
            n -= width;
            out.push(a[(v % 91) as usize]);
            out.push(a[(v / 91) as usize]);
        }
    }
    if n > 0 {
        out.push(a[(b % 91) as usize]);
        if n > 7 || b > 90 {
            out.push(a[(b / 91) as usize]);
        }
    }
    Ok(out)
}
fn decode(text: &str, p: &Value) -> Result<Vec<u8>, CoreError> {
    let a = alphabet(pstr(p, "alphabet", ALPHABET), Some(91))?;
    let (mut b, mut n, mut v) = (0u32, 0u32, None);
    let mut out = Vec::new();
    for ch in text.chars() {
        let Some(c) = a.iter().position(|&x| x == ch) else {
            if pbool(p, "strip", false) || ch.is_whitespace() {
                continue;
            }
            return Err(CoreError::Parse(format!("Base91 非法字符：{ch}")));
        };
        if let Some(first) = v.take() {
            let value = first + c as u32 * 91;
            b |= value << n;
            n += if value & 8191 > 88 { 13 } else { 14 };
            while n >= 8 {
                out.push(b as u8);
                b >>= 8;
                n -= 8;
            }
        } else {
            v = Some(c as u32);
        }
    }
    if let Some(value) = v {
        out.push((b | value << n) as u8);
    }
    Ok(out)
}
pub fn register(reg: &mut NodeRegistry) {
    register_pair(
        reg,
        "base91",
        "Base91",
        vec![
            ParamSpec::text("alphabet", "码表（91 字符）", ALPHABET, false),
            ParamSpec::toggle("strip", "解码时忽略码表外字符", false),
        ],
        encode,
        decode,
    );
}
