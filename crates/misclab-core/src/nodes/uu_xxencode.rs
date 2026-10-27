//! Traditional length-prefixed UU/XX streams, accepting framed or bare data.
use super::{codec_support::*, prelude::*};
use serde_json::Value;
const XX: &[u8] = b"+-0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
fn digit(value: u8, xx: bool) -> char {
    (if xx {
        XX[value as usize]
    } else if value == 0 {
        b'`'
    } else {
        value + 32
    }) as char
}
fn value(ch: u8, xx: bool) -> Result<u8, CoreError> {
    if xx {
        XX.iter()
            .position(|&b| b == ch)
            .map(|v| v as u8)
            .ok_or_else(|| CoreError::Parse("XXencode 非法字符".into()))
    } else if (32..=96).contains(&ch) {
        Ok((ch - 32) & 63)
    } else {
        Err(CoreError::Parse("UUencode 非法字符".into()))
    }
}
fn encode(data: &[u8], p: &Value, xx: bool) -> Result<String, CoreError> {
    let bare = pbool(p, "bare", false);
    let mut out = if bare {
        String::new()
    } else {
        "begin 644 -\n".into()
    };
    for line in data.chunks(45) {
        out.push(digit(line.len() as u8, xx));
        for chunk in line.chunks(3) {
            let a = chunk[0];
            let b = *chunk.get(1).unwrap_or(&0);
            let c = *chunk.get(2).unwrap_or(&0);
            for d in [
                a >> 2,
                ((a & 3) << 4) | (b >> 4),
                ((b & 15) << 2) | (c >> 6),
                c & 63,
            ] {
                out.push(if d == 0 && !xx && !bare {
                    ' '
                } else {
                    digit(d, xx)
                });
            }
        }
        out.push('\n');
    }
    if bare {
        out.pop();
    } else {
        out.push(digit(0, xx));
        out.push_str("\nend\n");
    }
    Ok(out)
}
fn decode(text: &str, xx: bool) -> Result<Vec<u8>, CoreError> {
    let mut out = Vec::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with("begin ") || line == "end" {
            continue;
        }
        let bytes = line.as_bytes();
        let count = value(bytes[0], xx)? as usize;
        if count == 0 {
            continue;
        }
        let length = count.div_ceil(3) * 4;
        if count > 45 || bytes.len() != length + 1 {
            return Err(CoreError::Parse("UU/XX 数据行长度无效或已截断".into()));
        }
        let start = out.len();
        for group in bytes[1..].chunks_exact(4) {
            let c: Vec<u8> = group
                .iter()
                .map(|&b| value(b, xx))
                .collect::<Result<_, _>>()?;
            out.extend_from_slice(&[
                (c[0] << 2) | (c[1] >> 4),
                (c[1] << 4) | (c[2] >> 2),
                (c[2] << 6) | c[3],
            ]);
        }
        out.truncate(start + count);
    }
    Ok(out)
}
pub fn register(reg: &mut NodeRegistry) {
    let params = || vec![ParamSpec::toggle("bare", "仅输出数据行", false)];
    register_pair(
        reg,
        "uuencode",
        "UUencode",
        params(),
        |d, p| encode(d, p, false),
        |t, _| decode(t, false),
    );
    register_pair(
        reg,
        "xxencode",
        "XXencode",
        params(),
        |d, p| encode(d, p, true),
        |t, _| decode(t, true),
    );
}
