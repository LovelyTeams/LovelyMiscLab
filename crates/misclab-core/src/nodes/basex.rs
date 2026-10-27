//! Shared base-N encoding algorithms, ported from CyberChef (Apache-2.0) so the
//! alphabets and edge cases match byte-for-byte. Not a node itself — the
//! `base32/45/58/62/85/92` node files build on these.

use super::prelude::*;

// ---- alphabet presets (CyberChef-compatible, `-` denotes a range) ----------
pub const B58_BITCOIN: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
pub const B58_RIPPLE: &str = "rpshnaf39wBUDNEGHJKLM4PQRST7VWXYZ2bcdeCg65jkm8oFqi1tuvAxyz";
pub const B62_STANDARD: &str = "0-9A-Za-z";
pub const B85_STANDARD: &str = "!-u";
pub const B85_Z85: &str = "0-9a-zA-Z.\\-:+=^!/*?&<>()[]{}@%$#";
pub const B85_IPV6: &str = "0-9A-Za-z!#$%&()*+\\-;<=>?@^_`{|}~";

/// Expand `a-z`-style ranges into an explicit char list (`\-` = literal dash).
/// Port of CyberChef's `Utils.expandAlphRange`.
pub fn expand_alph_range(s: &str) -> Vec<char> {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        if i + 2 < n && chars[i + 1] == '-' && chars[i] != '\\' {
            let (start, end) = (chars[i] as u32, chars[i + 2] as u32);
            for j in start..=end {
                if let Some(c) = char::from_u32(j) {
                    out.push(c);
                }
            }
            i += 3;
        } else if i + 1 < n && chars[i] == '\\' && chars[i + 1] == '-' {
            out.push('-');
            i += 2;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// Wrap a decoded byte vector as `text` (lossy) + `bytes` outputs.
pub fn decoded(bytes: Vec<u8>) -> PortMap {
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let mut m = PortMap::new();
    m.insert("text".to_string(), PortValue::Text(text));
    m.insert(
        "bytes".to_string(),
        PortValue::Bytes(Arc::from(bytes.into_boxed_slice())),
    );
    m
}

// ---- Base58 / Base62: big-integer radix conversion -------------------------

/// Encode bytes as a base-N string. `preserve_zeros` keeps leading zero bytes as
/// leading `alphabet[0]` (Base58 behavior); off drops them (Base62/bignum).
pub fn radix_encode(input: &[u8], alphabet: &[char], preserve_zeros: bool) -> String {
    radix_encode_checked(input, alphabet, preserve_zeros, None).expect("uncancelled radix encoding")
}

pub fn radix_encode_checked(
    input: &[u8],
    alphabet: &[char],
    preserve_zeros: bool,
    ctx: Option<&NodeCtx>,
) -> Result<String, CoreError> {
    if input.is_empty() {
        return Ok(String::new());
    }
    let base = alphabet.len();
    let mut zero_prefix = 0usize;
    if preserve_zeros {
        for &b in input {
            if b == 0 {
                zero_prefix += 1;
            } else {
                break;
            }
        }
    }
    let mut result: Vec<usize> = Vec::new();
    for (index, &b) in input.iter().enumerate() {
        if let Some(ctx) = ctx {
            if index.is_multiple_of(64) {
                ctx.progress(index as f32 / input.len() as f32);
                ctx.check_cancel()?;
            }
        }
        let mut carry = b as usize;
        for r in result.iter_mut() {
            carry += *r << 8;
            *r = carry % base;
            carry /= base;
        }
        while carry > 0 {
            result.push(carry % base);
            carry /= base;
        }
    }
    if !preserve_zeros && result.is_empty() {
        result.push(0); // represent the zero value as a single digit
    }
    let mut s: String = result.iter().rev().map(|&d| alphabet[d]).collect();
    for _ in 0..zero_prefix {
        s.insert(0, alphabet[0]);
    }
    if let Some(ctx) = ctx {
        ctx.progress(1.0);
        ctx.check_cancel()?;
    }
    Ok(s)
}

pub fn radix_decode(
    input: &str,
    alphabet: &[char],
    preserve_zeros: bool,
    strip: bool,
) -> Result<Vec<u8>, CoreError> {
    radix_decode_checked(input, alphabet, preserve_zeros, strip, None)
}

pub fn radix_decode_checked(
    input: &str,
    alphabet: &[char],
    preserve_zeros: bool,
    strip: bool,
    ctx: Option<&NodeCtx>,
) -> Result<Vec<u8>, CoreError> {
    let chars: Vec<char> = input
        .chars()
        .filter(|c| !strip || alphabet.contains(c))
        .collect();
    if chars.is_empty() {
        return Ok(Vec::new());
    }
    let base = alphabet.len();
    let mut zero_prefix = 0usize;
    if preserve_zeros {
        for &c in &chars {
            if c == alphabet[0] {
                zero_prefix += 1;
            } else {
                break;
            }
        }
    }
    let mut result: Vec<usize> = Vec::new();
    for (pos, &c) in chars.iter().enumerate() {
        if let Some(ctx) = ctx {
            if pos.is_multiple_of(64) {
                ctx.progress(pos as f32 / chars.len() as f32);
                ctx.check_cancel()?;
            }
        }
        let idx = match alphabet.iter().position(|&a| a == c) {
            Some(i) => i,
            None if strip => continue,
            None => {
                return Err(CoreError::Parse(format!(
                    "字符 '{c}' (位置 {pos}) 不在码表中"
                )))
            }
        };
        let mut carry = idx;
        for r in result.iter_mut() {
            carry += *r * base;
            *r = carry & 0xff;
            carry >>= 8;
        }
        while carry > 0 {
            result.push(carry & 0xff);
            carry >>= 8;
        }
    }
    result.extend(std::iter::repeat_n(0, zero_prefix));
    result.reverse();
    if let Some(ctx) = ctx {
        ctx.progress(1.0);
        ctx.check_cancel()?;
    }
    Ok(result.iter().map(|&b| b as u8).collect())
}

// ---- Base45 (RFC 9285) -----------------------------------------------------

pub fn base45_encode(input: &[u8], alphabet: &[char]) -> String {
    let mut res = String::new();
    for pair in input.chunks(2) {
        let mut b: u32 = 0;
        for &e in pair {
            b = b * 256 + e as u32;
        }
        let mut chars = 0;
        loop {
            res.push(alphabet[(b % 45) as usize]);
            chars += 1;
            b /= 45;
            if b == 0 {
                break;
            }
        }
        if chars < 2 {
            res.push(alphabet[0]);
            chars += 1;
        }
        if pair.len() > 1 && chars < 3 {
            res.push(alphabet[0]);
        }
    }
    res
}

pub fn base45_decode(input: &str, alphabet: &[char], strip: bool) -> Result<Vec<u8>, CoreError> {
    let chars: Vec<char> = if strip {
        input.chars().filter(|c| alphabet.contains(c)).collect()
    } else {
        input.chars().collect()
    };
    let mut res = Vec::new();
    for triple in chars.chunks(3) {
        if triple.len() == 1 {
            return Err(CoreError::Parse("Base45 末组不能只有一个字符".into()));
        }
        let mut b: u32 = 0;
        for &c in triple.iter().rev() {
            let idx = alphabet
                .iter()
                .position(|&a| a == c)
                .ok_or_else(|| CoreError::Parse(format!("字符不在码表中: '{c}'")))?;
            b = b * 45 + idx as u32;
        }
        if b > 65535 || (triple.len() == 2 && b > 255) {
            return Err(CoreError::Parse(format!(
                "三元组过大: '{}'",
                triple.iter().collect::<String>()
            )));
        }
        if triple.len() > 2 {
            res.push((b >> 8) as u8);
        }
        res.push((b & 0xff) as u8);
    }
    Ok(res)
}

// ---- Base85 / Ascii85 ------------------------------------------------------

fn strip_delims(s: &str) -> String {
    if s.len() >= 4 && s.starts_with("<~") && s.ends_with("~>") {
        s[2..s.len() - 2].to_string()
    } else {
        s.to_string()
    }
}

pub fn base85_encode(
    input: &[u8],
    alphabet: &[char],
    standard: bool,
    include_delim: bool,
) -> String {
    let n = input.len();
    let mut result = String::new();
    let mut i = 0;
    while i < n {
        let block: u64 = ((input[i] as u64) << 24)
            + ((*input.get(i + 1).unwrap_or(&0) as u64) << 16)
            + ((*input.get(i + 2).unwrap_or(&0) as u64) << 8)
            + (*input.get(i + 3).unwrap_or(&0) as u64);
        if !standard || block > 0 || i + 4 > n {
            let mut digits = [0usize; 5];
            let mut b = block;
            for d in digits.iter_mut() {
                *d = (b % 85) as usize;
                b /= 85;
            }
            digits.reverse();
            let keep = if n < i + 4 { (n - i) + 1 } else { 5 };
            for &d in &digits[..keep] {
                result.push(alphabet[d]);
            }
        } else {
            result.push('z');
        }
        i += 4;
    }
    if include_delim {
        format!("<~{result}~>")
    } else {
        result
    }
}

pub fn base85_decode(
    input: &str,
    alphabet: &[char],
    strip: bool,
    zero_char: Option<char>,
) -> Result<Vec<u8>, CoreError> {
    let mut s = strip_delims(input);
    if strip {
        s = s
            .chars()
            .filter(|&c| c == '~' || Some(c) == zero_char || alphabet.contains(&c))
            .collect();
        s = strip_delims(&s);
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return Ok(Vec::new());
    }
    let n = chars.len();
    let mut result = Vec::new();
    let mut i = 0;
    while i < n {
        if Some(chars[i]) == zero_char {
            result.extend_from_slice(&[0, 0, 0, 0]);
            i += 1;
            continue;
        }
        let take = (n - i).min(5);
        if take == 1 {
            return Err(CoreError::Parse("Base85 尾组至少需要两个字符".into()));
        }
        let mut d = [84u64; 5];
        for (k, slot) in d.iter_mut().enumerate() {
            if k < take {
                let c = chars[i + k];
                match alphabet.iter().position(|&a| a == c) {
                    Some(p) => *slot = p as u64,
                    None => {
                        return Err(CoreError::Parse(format!("非法字符 '{c}' 于位置 {}", i + k)))
                    }
                }
            }
        }
        let block = d[0] * 52200625 + d[1] * 614125 + d[2] * 7225 + d[3] * 85 + d[4];
        if block > u32::MAX as u64 {
            return Err(CoreError::Parse("Base85 分组超过 32 位".into()));
        }
        let bb = [
            ((block >> 24) & 0xff) as u8,
            ((block >> 16) & 0xff) as u8,
            ((block >> 8) & 0xff) as u8,
            (block & 0xff) as u8,
        ];
        let keep = if take < 5 { take - 1 } else { 4 };
        result.extend_from_slice(&bb[..keep]);
        i += 5;
    }
    Ok(result)
}

// ---- Base92 ----------------------------------------------------------------

fn base92_chr(val: usize) -> char {
    if val == 0 {
        '!'
    } else if val <= 61 {
        (b'#' + val as u8 - 1) as char
    } else {
        (b'a' + val as u8 - 62) as char
    }
}

fn base92_ord(c: char) -> Result<u32, CoreError> {
    if c == '!' {
        Ok(0)
    } else if ('#'..='_').contains(&c) {
        Ok(c as u32 - '#' as u32 + 1)
    } else if ('a'..='}').contains(&c) {
        Ok(c as u32 - 'a' as u32 + 62)
    } else {
        Err(CoreError::Parse(format!("'{c}' 不是 base92 字符")))
    }
}

pub fn base92_encode(input: &[u8]) -> String {
    let mut res = String::new();
    let mut bits = String::new();
    let mut pos = 0;
    loop {
        while bits.len() < 13 && pos < input.len() {
            bits.push_str(&format!("{:08b}", input[pos]));
            pos += 1;
        }
        if bits.len() < 13 {
            break;
        }
        let i = u32::from_str_radix(&bits[..13], 2).unwrap();
        res.push(base92_chr((i / 91) as usize));
        res.push(base92_chr((i % 91) as usize));
        bits = bits[13..].to_string();
    }
    if !bits.is_empty() {
        if bits.len() < 7 {
            while bits.len() < 6 {
                bits.push('0');
            }
            let v = u32::from_str_radix(&bits, 2).unwrap();
            res.push(base92_chr(v as usize));
        } else {
            while bits.len() < 13 {
                bits.push('0');
            }
            let i = u32::from_str_radix(&bits[..13], 2).unwrap();
            res.push(base92_chr((i / 91) as usize));
            res.push(base92_chr((i % 91) as usize));
        }
    }
    res
}

pub fn base92_decode(input: &str) -> Result<Vec<u8>, CoreError> {
    let chars: Vec<char> = input.chars().collect();
    let mut res = Vec::new();
    let mut bits = String::new();
    let mut i = 0;
    while i < chars.len() {
        if i + 1 != chars.len() {
            let x = base92_ord(chars[i])? * 91 + base92_ord(chars[i + 1])?;
            bits.push_str(&format!("{x:013b}"));
        } else {
            let x = base92_ord(chars[i])?;
            bits.push_str(&format!("{x:06b}"));
        }
        i += 2;
        while bits.len() >= 8 {
            res.push(u8::from_str_radix(&bits[..8], 2).unwrap());
            bits = bits[8..].to_string();
        }
    }
    Ok(res)
}
