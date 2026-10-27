use super::{codec_support::*, prelude::*};
use serde_json::Value;
const ALPHABET: &str = "cbdefghijklnrtuv";
fn encode(data: &[u8], p: &Value) -> Result<String, CoreError> {
    let delimiter = match pstr(p, "delim", "none") {
        "none" => "",
        "space" => " ",
        "colon" => ":",
        "comma" => ",",
        _ => return Err(CoreError::Parse("无效的 Modhex 分隔符".into())),
    };
    let chars = ALPHABET.as_bytes();
    Ok(data
        .iter()
        .map(|b| {
            format!(
                "{}{}",
                chars[(b >> 4) as usize] as char,
                chars[(b & 15) as usize] as char
            )
        })
        .collect::<Vec<_>>()
        .join(delimiter))
}
fn decode(text: &str, _: &Value) -> Result<Vec<u8>, CoreError> {
    let normalized: String = text
        .chars()
        .filter(|c| !matches!(c, ':' | ';' | ',' | '.' | '-' | '_' | '%' | '|' | '+'))
        .flat_map(char::to_lowercase)
        .collect();
    bit_decode(&normalized, &ALPHABET.chars().collect::<Vec<_>>(), 4)
}
pub fn register(reg: &mut NodeRegistry) {
    register_pair(
        reg,
        "modhex",
        "Modhex (YubiKey)",
        vec![ParamSpec::select(
            "delim",
            "输出分隔符",
            &["none", "space", "colon", "comma"],
            "none",
        )],
        encode,
        decode,
    );
}
