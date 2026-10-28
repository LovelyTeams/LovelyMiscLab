//! Bounded byte/text helpers shared by MIME and BinHex; no filesystem side effects.
use super::{prelude::*, transport_codecs::err};
use serde_json::Value;

pub const MAX_DATA: usize = 16 * 1024 * 1024;
pub const MAX_WIRE: usize = 64 * 1024 * 1024;

pub fn string<'a>(p: &'a Value, key: &str, default: &'a str) -> Result<&'a str, CoreError> {
    match p.get(key) {
        None => Ok(default),
        Some(Value::String(s)) => Ok(s),
        _ => Err(err(format!("{key} 必须是文本参数"))),
    }
}
pub fn bytes<'a>(inputs: &'a PortMap, key: &str) -> Result<&'a [u8], CoreError> {
    match inputs.get(key) {
        Some(PortValue::Text(s)) => Ok(s.as_bytes()),
        Some(PortValue::Bytes(b)) => Ok(b),
        None | Some(PortValue::None) => Err(CoreError::MissingInput(key.into())),
        _ => Err(err(format!(
            "{key} 仅接受文本或字节；文件产物请先连接产物读取节点"
        ))),
    }
}
pub fn limit(size: usize, max: usize, label: &str) -> Result<(), CoreError> {
    if size > max {
        Err(err(format!("{label} 超过 {} MiB 限制", max / 1024 / 1024)))
    } else {
        Ok(())
    }
}
pub fn checkpoint(ctx: &NodeCtx, fraction: f32) -> Result<(), CoreError> {
    ctx.progress(fraction);
    ctx.cancel.check()
}
/// UTF-16 count matches the upstream preview budget, without splitting a scalar.
pub fn preview(text: &str, max: usize) -> String {
    let mut units = 0;
    for (offset, c) in text.char_indices() {
        units += c.len_utf16();
        if units > max {
            return format!("{}\n…（截断）", &text[..offset]);
        }
    }
    text.into()
}
