//! Upstream JS escape notation, with lossless UTF-16 units for isolated surrogates.
use super::{
    codec_support::{document_text, help},
    numeric_support::bad,
    prelude::*,
};
use serde_json::{json, Value};
use std::fmt::Write;
struct Escape(bool);
impl Node for Escape {
    fn run(&self, i: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let s = in_text(i, "text")?;
        if s.len() > 6_291_456 {
            return Err(bad("escape文本输入最多6MiB"));
        }
        if !self.0 {
            if s.len() > 1_048_576 {
                return Err(bad("待编码文本最多1MiB"));
            }
            let mut out = String::new();
            for (index, c) in s.char_indices() {
                if index % 1024 < c.len_utf8() {
                    ctx.progress(index as f32 / s.len().max(1) as f32);
                    ctx.cancel.check()?;
                }
                if c.is_ascii_alphanumeric() || "@*_+-./".contains(c) {
                    out.push(c);
                } else if c.is_ascii() {
                    write!(out, "%{:02X}", c as u32).unwrap();
                } else {
                    for u in c.encode_utf16(&mut [0; 2]) {
                        write!(out, "%u{u:04X}").unwrap();
                    }
                }
            }
            ctx.progress(1.0);
            ctx.cancel.check()?;
            return Ok(out_text(out));
        }
        let units: Vec<u16> = s.encode_utf16().collect();
        let mut result = Vec::new();
        let mut n = 0;
        while n < units.len() {
            if n % 1024 < 6 {
                ctx.progress(n as f32 / units.len().max(1) as f32);
                ctx.cancel.check()?;
            }
            let mut decoded = None;
            if units[n] == b'%' as u16 {
                let (start, width) = if units.get(n + 1) == Some(&(b'u' as u16)) {
                    (n + 2, 4)
                } else {
                    (n + 1, 2)
                };
                if let Some(hex) = units.get(start..start + width) {
                    let mut value = 0u16;
                    let mut valid = true;
                    for &u in hex {
                        if let Some(v) = char::from_u32(u as u32)
                            .filter(char::is_ascii_hexdigit)
                            .and_then(|c| c.to_digit(16))
                        {
                            value = value * 16 + v as u16;
                        } else {
                            valid = false;
                            break;
                        }
                    }
                    if valid {
                        decoded = Some((value, start + width));
                    }
                }
            }
            if let Some((u, end)) = decoded {
                result.push(u);
                n = end;
            } else {
                result.push(units[n]);
                n += 1;
            }
        }
        let parsed = String::from_utf16(&result);
        let valid = parsed.is_ok();
        if !valid && pbool(p, "strict", true) {
            return Err(bad("escape结果含孤立UTF-16代理；关闭strict可读取完整码元"));
        }
        let text = parsed.unwrap_or_else(|_| String::from_utf16_lossy(&result));
        let bytes: Vec<u8> = result.into_iter().flat_map(u16::to_le_bytes).collect();
        ctx.progress(1.0);
        ctx.cancel.check()?;
        Ok([
            ("text".into(), PortValue::Text(text)),
            ("utf16le".into(), PortValue::Bytes(bytes.into())),
            ("validUnicode".into(), PortValue::Bool(valid)),
        ]
        .into_iter()
        .collect())
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for decode in [false, true] {
        let d=document_text(desc(if decode{"js_escape_decode"}else{"js_escape_encode"},"encoding.transport",if decode{"JS escape 解码"}else{"JS escape 编码"},BLUE,vec![t_in()],if decode{vec![t_out(),req("utf16le","完整UTF-16LE码元",PortType::Bytes),req("validUnicode","有效Unicode",PortType::Bool)]}else{vec![t_out()]},if decode{vec![help(ParamSpec::toggle("strict","拒绝孤立代理",true),"关闭后文本预览使用替换符，utf16le保留所有原始码元，validUnicode显示完整性；非法百分号语法原样透传。")]}else{vec![]}),"jsEscape","复现上游旧式escape：非安全ASCII转%XX，所有非ASCII转%uXXXX；与标准escape的Latin1缩写存在差异。解码支持两种表示，UTF-16代理对合并。",if decode{"%u4E2D%u6587%uD83D%uDE42"}else{"中文🙂"},json!({}));
        reg.register(d, Arc::new(move || Arc::new(Escape(decode))));
    }
}
