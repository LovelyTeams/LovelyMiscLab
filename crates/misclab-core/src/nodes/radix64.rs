use super::{codec_support::*, prelude::*};
use serde_json::Value;
const ALPHABET: &str = "./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
fn encode(data: &[u8], _: &Value) -> Result<String, CoreError> {
    Ok(bit_encode(data, &ALPHABET.chars().collect::<Vec<_>>(), 6))
}
fn decode(text: &str, _: &Value) -> Result<Vec<u8>, CoreError> {
    bit_decode(text, &ALPHABET.chars().collect::<Vec<_>>(), 6)
}
pub fn register(reg: &mut NodeRegistry) {
    register_pair(reg, "radix64", "Radix64 (crypt)", vec![], encode, decode);
}
