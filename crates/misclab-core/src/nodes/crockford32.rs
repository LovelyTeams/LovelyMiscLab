use super::{codec_support::*, prelude::*};
use serde_json::Value;
const ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const CHECK: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ*~$=U";
fn checksum(bytes: &[u8]) -> usize {
    bytes.iter().fold(0, |n, b| (n * 256 + *b as usize) % 37)
}
fn encode(data: &[u8], p: &Value) -> Result<String, CoreError> {
    let mut result = bit_encode(data, &ALPHABET.chars().collect::<Vec<_>>(), 5);
    if pbool(p, "check", false) {
        result.push(CHECK.as_bytes()[checksum(data)] as char);
    }
    let group = pnum(p, "dashGroup", 0.0) as usize;
    if group > 0 {
        result = result
            .as_bytes()
            .chunks(group)
            .map(|s| std::str::from_utf8(s).unwrap())
            .collect::<Vec<_>>()
            .join("-");
    }
    Ok(result)
}
fn decode(text: &str, p: &Value) -> Result<Vec<u8>, CoreError> {
    let mut normalized: String = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        })
        .collect();
    let check = if pbool(p, "check", false) {
        Some(
            normalized
                .pop()
                .ok_or_else(|| CoreError::Parse("缺少 Crockford 校验字符".into()))?,
        )
    } else {
        None
    };
    let bytes = bit_decode(&normalized, &ALPHABET.chars().collect::<Vec<_>>(), 5)?;
    if check.is_some_and(|c| c != CHECK.as_bytes()[checksum(&bytes)] as char) {
        return Err(CoreError::Parse("Crockford Base32 校验失败".into()));
    }
    Ok(bytes)
}
pub fn register(reg: &mut NodeRegistry) {
    register_pair(
        reg,
        "crockford32",
        "Crockford Base32",
        vec![
            ParamSpec::toggle("check", "模 37 校验", false),
            ParamSpec::number(
                "dashGroup",
                "编码分组长度（0 不分组）",
                0.0,
                128.0,
                1.0,
                0.0,
            ),
        ],
        encode,
        decode,
    );
}
