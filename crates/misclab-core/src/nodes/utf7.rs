//! RFC 2152 shifted UTF-16BE, implemented entirely with Rust Base64/UTF-16.
use super::{codec_support::document_text, prelude::*};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use serde_json::json;

fn direct(c: char) -> bool {
    c.is_ascii_alphanumeric() || "'(),-./:?!\"#$%&*;<=>@_^{|}\\ \r\n\t".contains(c)
}
fn encode(text: &str, ctx: &mut NodeCtx) -> Result<String, CoreError> {
    let mut shifted = Vec::new();
    let mut out = String::new();
    let flush = |bytes: &mut Vec<u8>, out: &mut String| {
        if !bytes.is_empty() {
            out.push('+');
            out.push_str(&STANDARD_NO_PAD.encode(&*bytes));
            out.push('-');
            bytes.clear();
        }
    };
    for (i, c) in text.chars().enumerate() {
        if i % 4096 == 0 {
            ctx.cancel.check()?;
        }
        if c == '+' || direct(c) {
            flush(&mut shifted, &mut out);
            if c == '+' {
                out.push_str("+-");
            } else {
                out.push(c);
            }
        } else {
            let mut units = [0; 2];
            for u in c.encode_utf16(&mut units) {
                shifted.extend_from_slice(&u.to_be_bytes());
            }
        }
    }
    flush(&mut shifted, &mut out);
    Ok(out)
}
fn decode(text: &str, ctx: &mut NodeCtx) -> Result<String, CoreError> {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        ctx.cancel.check()?;
        if c != '+' {
            out.push(c);
            continue;
        }
        let mut segment = String::new();
        while chars
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '+' || *c == '/')
        {
            segment.push(chars.next().unwrap());
        }
        if chars.peek() == Some(&'-') {
            chars.next();
        }
        if segment.is_empty() {
            out.push('+');
            continue;
        }
        let bytes = STANDARD_NO_PAD
            .decode(segment)
            .map_err(|e| CoreError::Parse(format!("UTF-7 移位段无效：{e}")))?;
        if bytes.is_empty() || bytes.len() % 2 != 0 {
            return Err(CoreError::Parse("UTF-7：UTF-16BE 码元被截断".into()));
        }
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|b| u16::from_be_bytes([b[0], b[1]]))
            .collect();
        out.push_str(
            &String::from_utf16(&units)
                .map_err(|e| CoreError::Parse(format!("UTF-7：无效代理对 {e}")))?,
        );
    }
    Ok(out)
}
struct Codec(bool);
impl Node for Codec {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let t = in_text(i, "text")?;
        Ok(out_text(if self.0 { decode(t, c)? } else { encode(t, c)? }))
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for dec in [false, true] {
        let d = document_text(
            desc(
                if dec { "utf7_decode" } else { "utf7_encode" },
                "encoding.charset",
                if dec { "UTF-7 解码" } else { "UTF-7 编码" },
                BLUE,
                vec![t_in()],
                vec![t_out()],
                vec![],
            ),
            "utf7",
            "UTF-7 的 UTF-16BE 移位序列；支持显式 -、隐式终止和补充平面字符，+ 用 +- 表示。",
            if dec {
                "+Ti1lh9g93kI- +- CTF"
            } else {
                "中文🙂 + CTF"
            },
            json!({}),
        );
        reg.register(d, Arc::new(move || Arc::new(Codec(dec))));
    }
}
