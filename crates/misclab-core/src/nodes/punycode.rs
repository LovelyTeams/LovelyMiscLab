//! RFC 3492 Bootstring with checked arithmetic, bounded labels and cancellation.
//! Raw Punycode labels only: no DNS lookup, IDNA normalization or JS runtime.
use super::{codec_support::document_text, prelude::*};
use serde_json::json;

fn invalid() -> CoreError {
    CoreError::Parse("Punycode：输入截断、数值溢出或码点无效".into())
}
fn adapt(mut delta: u64, points: u64, first: bool) -> u64 {
    delta /= if first { 700 } else { 2 };
    delta += delta / points;
    let mut k = 0;
    while delta > 455 {
        delta /= 35;
        k += 36;
    }
    k + 36 * delta / (delta + 38)
}
fn threshold(k: u64, bias: u64) -> u64 {
    k.saturating_sub(bias).clamp(1, 26)
}
fn digit(d: u64) -> char {
    if d < 26 {
        (b'a' + d as u8) as char
    } else {
        (b'0' + (d - 26) as u8) as char
    }
}
fn value(c: u8) -> Result<u64, CoreError> {
    match c {
        b'a'..=b'z' => Ok((c - b'a') as u64),
        b'A'..=b'Z' => Ok((c - b'A') as u64),
        b'0'..=b'9' => Ok((c - b'0' + 26) as u64),
        _ => Err(invalid()),
    }
}
fn encode(label: &str, ctx: &mut NodeCtx) -> Result<String, CoreError> {
    if label.is_ascii() {
        return Ok(label.into());
    }
    let points: Vec<u64> = label.chars().map(|c| c as u64).collect();
    let mut output: String = label.chars().filter(char::is_ascii).collect();
    let basic = output.len() as u64;
    if basic > 0 {
        output.push('-');
    }
    let (mut n, mut delta, mut bias, mut h) = (128_u64, 0_u64, 72_u64, basic);
    while h < points.len() as u64 {
        ctx.progress(h as f32 / points.len() as f32);
        ctx.cancel.check()?;
        let m = *points
            .iter()
            .filter(|&&cp| cp >= n)
            .min()
            .ok_or_else(invalid)?;
        delta = delta
            .checked_add((m - n).checked_mul(h + 1).ok_or_else(invalid)?)
            .ok_or_else(invalid)?;
        n = m;
        for &cp in &points {
            if cp < n {
                delta = delta.checked_add(1).ok_or_else(invalid)?;
            }
            if cp != n {
                continue;
            }
            let mut q = delta;
            let mut k = 36;
            loop {
                let t = threshold(k, bias);
                if q < t {
                    break;
                }
                output.push(digit(t + (q - t) % (36 - t)));
                q = (q - t) / (36 - t);
                k += 36;
            }
            output.push(digit(q));
            bias = adapt(delta, h + 1, h == basic);
            delta = 0;
            h += 1;
        }
        delta += 1;
        n += 1;
    }
    Ok(format!("xn--{output}"))
}
fn decode(label: &str, ctx: &mut NodeCtx) -> Result<String, CoreError> {
    if !label
        .get(..4)
        .is_some_and(|p| p.eq_ignore_ascii_case("xn--"))
    {
        return Ok(label.into());
    }
    let label = &label[4..];
    if !label.is_ascii() {
        return Err(invalid());
    }
    let (basic, encoded) = label.rsplit_once('-').unwrap_or(("", label));
    let mut output: Vec<char> = basic.chars().collect();
    let encoded = encoded.as_bytes();
    let (mut n, mut i, mut bias, mut pos) = (128_u64, 0_u64, 72_u64, 0_usize);
    while pos < encoded.len() {
        ctx.progress(pos as f32 / encoded.len() as f32);
        ctx.cancel.check()?;
        let old = i;
        let (mut w, mut k) = (1_u64, 36_u64);
        loop {
            let d = value(*encoded.get(pos).ok_or_else(invalid)?)?;
            pos += 1;
            i = i
                .checked_add(d.checked_mul(w).ok_or_else(invalid)?)
                .ok_or_else(invalid)?;
            let t = threshold(k, bias);
            if d < t {
                break;
            }
            w = w.checked_mul(36 - t).ok_or_else(invalid)?;
            k += 36;
        }
        let count = output.len() as u64 + 1;
        bias = adapt(i - old, count, old == 0);
        n = n.checked_add(i / count).ok_or_else(invalid)?;
        i %= count;
        let cp = u32::try_from(n)
            .ok()
            .and_then(char::from_u32)
            .ok_or_else(invalid)?;
        output.insert(i as usize, cp);
        i += 1;
    }
    Ok(output.into_iter().collect())
}
struct Codec(bool);
impl Node for Codec {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(i, "text")?;
        if text.len() > 1_048_576 {
            return Err(CoreError::Parse("Punycode 输入上限 1 MiB".into()));
        }
        let mut output = Vec::new();
        for label in text.split('.') {
            c.cancel.check()?;
            if label.chars().count() > 4096 {
                return Err(CoreError::Parse("Punycode 每段上限 4096 字符".into()));
            }
            output.push(if self.0 {
                decode(label, c)?
            } else {
                encode(label, c)?
            });
        }
        c.progress(1.0);
        Ok(out_text(output.join(".")))
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for dec in [false, true] {
        let mut d = document_text(desc(if dec { "punycode_decode" } else { "punycode_encode" }, "encoding.transport", if dec { "Punycode 解码" } else { "Punycode 编码" }, BLUE, vec![t_in()], vec![t_out()], vec![]), "punycode", "按点号分段的 RFC 3492 Punycode；保留 ASCII 与大小写，不进行 IDNA 规范化或域名查询。每段上限 4096 字符，支持进度和取消。", if dec { "xn--fiq228c.xn--fsqu00a" } else { "中文.例子" }, json!({}));
        d.cost = Cost::Heavy;
        reg.register(d, Arc::new(move || Arc::new(Codec(dec))));
    }
}
