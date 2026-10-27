//! xxd formatting and bounded reconstruction, independent of the legacy layout.
use super::{
    numeric_support::{bad, number, space},
    prelude::*,
};
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;
static COLON: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([0-9A-Fa-f]{1,16})h?[ \t]*:(.*)$").unwrap());
static OFFSET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([0-9A-Fa-f]{6,16})[ \t]{2,}(.*)$|^([0-9A-Fa-f]{4,16})h[ \t]+(.*)$").unwrap()
});
pub(super) fn encode(data: &[u8], p: &Value, c: &mut NodeCtx) -> Result<String, CoreError> {
    let width = number(p, "width", 16, 1, 512)? as usize;
    let upper = pbool(p, "upperCase", false);
    let mut out = String::new();
    for (index, chunk) in data.chunks(width).enumerate() {
        if index.is_multiple_of(256) {
            c.progress((index * width) as f32 / data.len().max(1) as f32);
            c.cancel.check()?;
        }
        let ascii: String = chunk
            .iter()
            .map(|&b| {
                if (0x20..0x7f).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        let groups = width.div_ceil(2);
        let mut h = String::new();
        for g in 0..groups {
            if let Some(b) = chunk.get(g * 2) {
                h.push_str(&format!("{b:02x}"));
            }
            if let Some(b) = chunk.get(g * 2 + 1) {
                h.push_str(&format!("{b:02x}"));
            }
            h.push(' ');
        }
        let h = format!("{h:<0$}", width * 2 + groups);
        let offset = format!("{:08x}", index * width);
        let (h, offset) = if upper {
            (h.to_ascii_uppercase(), offset.to_ascii_uppercase())
        } else {
            (h, offset)
        };
        if index > 0 {
            out.push('\n');
        }
        out.push_str(&format!("{offset}: {h} {ascii}"));
    }
    c.progress(1.0);
    c.cancel.check()?;
    Ok(out)
}
type DumpLine = (Option<u64>, Vec<u8>);
fn parse_line(line: &str, strict: bool) -> Result<Option<DumpLine>, CoreError> {
    let s = line.trim_start_matches([' ', '\t']);
    let (mut offset, mut body) = (None, s);
    if let Some(m) = COLON.captures(s) {
        offset = Some(u64::from_str_radix(&m[1], 16).map_err(|_| bad("偏移溢出"))?);
        body = m.get(2).unwrap().as_str();
    } else if let Some(m) = OFFSET.captures(s) {
        let (off, b) = if let Some(o) = m.get(1) {
            (o.as_str(), m.get(2).unwrap().as_str())
        } else {
            (m.get(3).unwrap().as_str(), m.get(4).unwrap().as_str())
        };
        if b.bytes().take_while(u8::is_ascii_hexdigit).count() >= 2 {
            offset = Some(u64::from_str_radix(off, 16).map_err(|_| bad("偏移溢出"))?);
            body = b;
        }
    }
    body = body.trim_start_matches([' ', '\t']);
    // hexdump -C explicitly delimits its ASCII column; retain both 8-byte groups.
    let canonical = strict && body.contains('|');
    let body = if canonical {
        body.split('|').next().unwrap()
    } else {
        body
    };
    let b = body.as_bytes();
    let mut out = Vec::new();
    let mut n = 0;
    while n < b.len() {
        let start = n;
        while n < b.len() && b[n].is_ascii_hexdigit() {
            n += 1;
        }
        let count = n - start;
        if count < 2 {
            break;
        }
        if strict && !count.is_multiple_of(2) {
            return Err(bad("Hexdump遇到未成对的十六进制数字"));
        }
        for k in (start..start + count - count % 2).step_by(2) {
            out.push(u8::from_str_radix(&body[k..k + 2], 16).unwrap());
        }
        if n < b.len() && b[n] == b'-' && b.get(n + 1).is_some_and(u8::is_ascii_hexdigit) {
            n += 1;
            continue;
        }
        let before = n;
        while n < b.len() && b[n] == b' ' {
            n += 1;
        }
        if n == before || !canonical && n - before >= 2 {
            break;
        }
    }
    if out.is_empty() && offset.is_none() {
        Ok(None)
    } else {
        Ok(Some((offset, out)))
    }
}
pub(super) fn decode(raw: &str, p: &Value, c: &mut NodeCtx) -> Result<PortMap, CoreError> {
    let limit = number(p, "maxBytes", 16_777_216, 1, 67_108_864)? as usize;
    let strict = pbool(p, "strict", true);
    let mut out = Vec::new();
    let (mut prev, mut star): (Option<(u64, Vec<u8>)>, bool) = (None, false);
    for (index, line) in raw.lines().enumerate() {
        if index.is_multiple_of(256) {
            c.progress(index as f32 / raw.len().max(1) as f32);
            c.cancel.check()?;
        }
        let trim = line.trim_matches(space);
        if trim.is_empty() {
            continue;
        }
        if trim == "*" {
            star = true;
            continue;
        }
        // A final offset in canonical hexdump -C carries no data.
        let terminal =
            strict && (6..=16).contains(&trim.len()) && trim.bytes().all(|b| b.is_ascii_hexdigit());
        let parsed = if terminal {
            Some((
                Some(u64::from_str_radix(trim, 16).map_err(|_| bad("偏移溢出"))?),
                Vec::new(),
            ))
        } else {
            parse_line(line, strict)?
        };
        let Some((offset, bytes)) = parsed else {
            star = false;
            continue;
        };
        if bytes.is_empty() && !terminal {
            star = false;
            continue;
        }
        if star {
            if let (Some((before, previous)), Some(next)) = (&prev, offset) {
                let end = before
                    .checked_add(previous.len() as u64)
                    .ok_or_else(|| bad("偏移溢出"))?;
                if let Some(gap) = next.checked_sub(end) {
                    if strict && !gap.is_multiple_of(previous.len() as u64) {
                        return Err(bad("Hexdump重复范围不是整行倍数"));
                    }
                    if gap > 0 && gap.is_multiple_of(previous.len() as u64) {
                        if gap > limit.saturating_sub(out.len()) as u64 {
                            return Err(bad("Hexdump重复展开超过maxBytes"));
                        }
                        for k in 0..gap / previous.len() as u64 {
                            if k.is_multiple_of(1024) {
                                c.cancel.check()?;
                            }
                            out.extend(previous);
                        }
                    }
                } else if strict {
                    return Err(bad("Hexdump重复范围偏移倒退"));
                }
            } else if strict {
                return Err(bad("Hexdump重复行缺少前后偏移"));
            }
        }
        star = false;
        if bytes.len() > limit.saturating_sub(out.len()) {
            return Err(bad("Hexdump输出超过maxBytes"));
        }
        out.extend(&bytes);
        if let Some(off) = offset {
            if !bytes.is_empty() {
                prev = Some((off, bytes));
            }
        }
    }
    if star && strict {
        return Err(bad("Hexdump重复行缺少结束偏移"));
    }
    if out.is_empty() && !raw.trim_matches(space).is_empty() {
        return Err(bad("Hexdump没有解析出任何字节"));
    }
    let text =
        String::from_utf8_lossy(out.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&out)).into_owned();
    c.progress(1.0);
    c.cancel.check()?;
    Ok([
        ("text".into(), PortValue::Text(text)),
        ("bytes".into(), PortValue::Bytes(out.into())),
    ]
    .into_iter()
    .collect())
}
