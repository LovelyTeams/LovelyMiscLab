use super::{basex, codec_support::*, prelude::*};
use serde_json::Value;
use sha2::{Digest, Sha256};
fn encode(data: &[u8], p: &Value) -> Result<String, CoreError> {
    let a = alphabet(pstr(p, "alphabet", basex::B58_BITCOIN), Some(58))?;
    let mut bytes = data.to_vec();
    bytes.extend_from_slice(&Sha256::digest(Sha256::digest(data))[..4]);
    Ok(basex::radix_encode(&bytes, &a, true))
}
fn decode(text: &str, p: &Value) -> Result<Vec<u8>, CoreError> {
    let a = alphabet(pstr(p, "alphabet", basex::B58_BITCOIN), Some(58))?;
    let bytes = basex::radix_decode(text.trim(), &a, true, false)?;
    if bytes.len() < 4 {
        return Err(CoreError::Parse("Base58Check 数据过短".into()));
    }
    let (payload, checksum) = bytes.split_at(bytes.len() - 4);
    if checksum != &Sha256::digest(Sha256::digest(payload))[..4] {
        return Err(CoreError::Parse("Base58Check 校验失败".into()));
    }
    Ok(payload.to_vec())
}
pub fn register(reg: &mut NodeRegistry) {
    register_pair(
        reg,
        "base58check",
        "Base58Check",
        vec![ParamSpec::text(
            "alphabet",
            "码表（58 字符）",
            basex::B58_BITCOIN,
            false,
        )],
        encode,
        decode,
    );
}
