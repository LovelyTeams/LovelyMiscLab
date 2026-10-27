use super::{codec_support::*, prelude::*};
use serde_json::Value;
fn encode(data: &[u8], _: &Value) -> Result<String, CoreError> {
    Ok(data
        .iter()
        .map(|&b| char::from_u32(0x1f3f7 + b as u32).unwrap())
        .collect())
}
fn decode(text: &str, _: &Value) -> Result<Vec<u8>, CoreError> {
    text.chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| {
            let value = c as u32;
            if (0x1f3f7..=0x1f4f6).contains(&value) {
                Ok((value - 0x1f3f7) as u8)
            } else {
                Err(CoreError::Parse(format!("非 Base100 字符：{c}")))
            }
        })
        .collect()
}
pub fn register(reg: &mut NodeRegistry) {
    register_pair(reg, "base100", "Base100 Emoji", vec![], encode, decode);
}
