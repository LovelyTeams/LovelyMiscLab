use super::{basex, codec_support::*, prelude::*};
use serde_json::Value;
const DEFAULT: &str = "0123456789ABCDEF";
fn encode(data: &[u8], p: &Value) -> Result<String, CoreError> {
    let chars = alphabet(pstr(p, "alphabet", DEFAULT), None)?;
    Ok(basex::radix_encode(data, &chars, true))
}
fn decode(text: &str, p: &Value) -> Result<Vec<u8>, CoreError> {
    let chars = alphabet(pstr(p, "alphabet", DEFAULT), None)?;
    basex::radix_decode(
        if chars.iter().any(|c| c.is_whitespace()) {
            text
        } else {
            text.trim()
        },
        &chars,
        true,
        false,
    )
}
pub fn register(reg: &mut NodeRegistry) {
    register_pair(
        reg,
        "base_custom",
        "自定义字母表 Base",
        vec![ParamSpec::text(
            "alphabet",
            "码表（逐字列出）",
            DEFAULT,
            false,
        )],
        encode,
        decode,
    );
}
