//! Byte-radix and code-point notation nodes; raw bytes never pass through lossy text.
use super::{
    basex::decoded,
    codec_support::{document_text, help},
    numeric_support::*,
    prelude::*,
};
use num_traits::{Signed, ToPrimitive};
use serde_json::{json, Value};
#[derive(Clone, Copy)]
enum Kind {
    Ascii,
    Bcd,
    Pad,
    Completion,
    Hybrid,
}
struct Codec {
    kind: Kind,
    decode: bool,
}
impl Node for Codec {
    fn run(&self, inputs: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        if matches!(self.kind, Kind::Ascii) {
            let base = number(p, "base", 16, 2, 36)? as u32;
            let width = if base == 2 {
                number(p, "bits", 8, 7, 8)? as usize
            } else {
                match base {
                    8 | 10 => 3,
                    16 => 2,
                    _ => 0,
                }
            };
            let inv = base == 2 && pbool(p, "invert", false);
            let rev = base == 2 && pbool(p, "bitReverse", false);
            if self.decode {
                let text = in_text(inputs, "text")?.trim_matches(space);
                if text.len() > 4_194_304 {
                    return Err(bad("进制字节串最大 4 MiB"));
                }
                let tokens: Vec<String> =
                    if width > 0 && !text.is_empty() && !text.chars().any(space) {
                        if !text.is_ascii() || text.len() % width != 0 {
                            return Err(bad("无分隔输入长度必须是位宽的倍数"));
                        }
                        text.as_bytes()
                            .chunks(width)
                            .map(|b| String::from_utf8(b.to_vec()).unwrap())
                            .collect()
                    } else {
                        text.split(space)
                            .filter(|s| !s.is_empty())
                            .map(str::to_owned)
                            .collect()
                    };
                let mut out = Vec::new();
                for (i, mut token) in tokens.into_iter().enumerate() {
                    if i % 4096 == 0 {
                        ctx.cancel.check()?;
                    }
                    if inv {
                        token = token
                            .chars()
                            .map(|c| match c {
                                '0' => '1',
                                '1' => '0',
                                _ => c,
                            })
                            .collect();
                    }
                    if rev {
                        token = token.chars().rev().collect();
                    }
                    let b = integer(&token, base)?
                        .to_u8()
                        .ok_or_else(|| bad("进制数值必须在 0..255"))?;
                    out.push(b);
                }
                return Ok(decoded(out));
            }
            let bytes = in_bytes(inputs, "data")?;
            let mut out = Vec::new();
            for (i, b) in bytes.into_iter().enumerate() {
                if i % 4096 == 0 {
                    ctx.cancel.check()?;
                }
                if base == 2 && width == 7 && b >= 128 {
                    return Err(bad("7-bit 模式仅接受 ASCII 字节"));
                }
                let s = num_bigint::BigUint::from(b).to_str_radix(base);
                let mut token = if base != 2 && pbool(p, "compat", false) {
                    s
                } else {
                    format!("{s:0>width$}")
                };
                if rev {
                    token = token.chars().rev().collect();
                }
                if inv {
                    token = token
                        .chars()
                        .map(|c| if c == '0' { '1' } else { '0' })
                        .collect();
                }
                out.push(token);
            }
            return Ok(out_text(out.join(" ")));
        }
        let text = in_text(inputs, "text")?;
        if text.len() > 4_194_304 {
            return Err(bad("输入最大 4 MiB"));
        }
        let out = match self.kind {
            Kind::Bcd => {
                let binary = pbool(p, "compat", false);
                if self.decode {
                    let digits: String = text
                        .chars()
                        .filter(|c| {
                            if binary {
                                *c == '0' || *c == '1'
                            } else {
                                c.is_ascii_hexdigit()
                            }
                        })
                        .collect();
                    if digits.is_empty() {
                        return Err(bad("BCD 输入不能为空"));
                    }
                    let width = if binary { 4 } else { 1 };
                    if !digits.len().is_multiple_of(width) {
                        return Err(bad("BCD 二进制位数必须是 4 的倍数"));
                    }
                    let mut out = String::new();
                    for chunk in digits.as_bytes().chunks(width) {
                        let n = u8::from_str_radix(
                            std::str::from_utf8(chunk).unwrap(),
                            if binary { 2 } else { 16 },
                        )
                        .unwrap();
                        if n > 9 {
                            return Err(bad("BCD 半字节不能大于 9"));
                        }
                        out.push((b'0' + n) as char);
                    }
                    out
                } else {
                    let digits: String = text.chars().filter(char::is_ascii_digit).collect();
                    if digits.is_empty() {
                        return Err(bad("BCD 需要十进制数字串"));
                    }
                    if binary {
                        digits
                            .bytes()
                            .map(|b| format!("{:04b}", b - b'0'))
                            .collect::<Vec<_>>()
                            .join(" ")
                    } else {
                        digits
                    }
                }
            }
            Kind::Pad => {
                if self.decode {
                    let s: String = text.chars().filter(|c| *c == '0' || *c == '1').collect();
                    integer(&s, 2)?.to_string()
                } else {
                    let bits = number(p, "bits", 8, 1, 16384)? as usize;
                    let n = integer(text, 10)?;
                    if n.is_negative() {
                        return Err(bad("补零对齐需要非负整数"));
                    }
                    let s = n.to_str_radix(2);
                    if s.len() > bits {
                        return Err(bad("数值超出指定位宽"));
                    }
                    format!("{s:0>bits$}")
                }
            }
            Kind::Completion => {
                if self.decode {
                    text.split(space)
                        .filter(|s| !s.is_empty())
                        .map(|s| {
                            let mut rest = s;
                            while rest.starts_with('0')
                                && rest.as_bytes().get(1).is_some_and(u8::is_ascii_digit)
                            {
                                rest = &rest[1..];
                            }
                            rest
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                } else {
                    let mut s = text.to_string();
                    for prefix in ["0b", "0x", "0o"] {
                        if s.contains(prefix) {
                            s = s.replace(prefix, "");
                            break;
                        }
                    }
                    let parts: Vec<_> = s.split(space).filter(|s| !s.is_empty()).collect();
                    if parts.is_empty() {
                        return Err(bad("补零对齐输入不能为空"));
                    }
                    let target = number(p, "bits", 0, 0, 16384)? as usize;
                    let width = if target == 0 {
                        parts
                            .iter()
                            .map(|s| s.encode_utf16().count())
                            .max()
                            .unwrap()
                    } else {
                        target
                    };
                    if width.saturating_mul(parts.len()) > 4_194_304 {
                        return Err(bad("补零结果超过 4 MiB"));
                    }
                    parts
                        .iter()
                        .map(|s| {
                            format!(
                                "{}{s}",
                                "0".repeat(width.saturating_sub(s.encode_utf16().count()))
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                }
            }
            Kind::Hybrid => {
                if self.decode {
                    let s = text.trim_matches(space);
                    let parts: Vec<_> = if s.contains('\\') {
                        s.split('\\').filter(|s| !s.is_empty()).collect()
                    } else if s.contains(' ') {
                        s.split(space).filter(|s| !s.is_empty()).collect()
                    } else if s.contains(',') {
                        s.split(',').filter(|s| !s.is_empty()).collect()
                    } else {
                        vec![s]
                    };
                    let mut out = String::new();
                    for token in parts {
                        ctx.cancel.check()?;
                        if token.is_empty() {
                            continue;
                        }
                        let prefix = token.as_bytes()[0].to_ascii_lowercase();
                        let base = match prefix {
                            b'b' => 2,
                            b'x' => 16,
                            b'o' => 8,
                            b'd' => 10,
                            _ => return Err(bad("混合进制前缀必须为 b/x/o/d")),
                        };
                        let rest = token.get(1..).ok_or_else(|| bad("前缀无效"))?;
                        let n = integer(rest, base)?
                            .to_u32()
                            .and_then(char::from_u32)
                            .ok_or_else(|| bad("混合进制不是有效 Unicode 码点"))?;
                        out.push(n);
                    }
                    out
                } else {
                    let mode = pstr(p, "mode", "hex");
                    let sep = pstr(p, "sep", " ");
                    let sep = if sep.is_empty() { " " } else { sep };
                    let mut out = Vec::new();
                    for (i, c) in text.chars().enumerate() {
                        if i % 4096 == 0 {
                            ctx.cancel.check()?;
                        }
                        let n = c as u32;
                        let options = [
                            (format!("{n}"), 'd'),
                            (format!("{n:x}"), 'x'),
                            (format!("{n:o}"), 'o'),
                            (format!("{n:b}"), 'b'),
                        ];
                        let index = match mode {
                            "dec" => 0,
                            "hex" => 1,
                            "oct" => 2,
                            "bin" => 3,
                            "auto" => {
                                options
                                    .iter()
                                    .enumerate()
                                    .min_by_key(|(_, v)| v.0.len())
                                    .unwrap()
                                    .0
                            }
                            _ => return Err(bad("混合进制编码模式无效")),
                        };
                        out.push(format!("{}{}", options[index].1, options[index].0));
                    }
                    out.join(sep)
                }
            }
            Kind::Ascii => unreachable!(),
        };
        Ok(out_text(out))
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for (kind, id, upstream, title, sample, encoded) in [
        (
            Kind::Ascii,
            "ascii_radix",
            "asciiRadix",
            "字节进制表示",
            "Hello, 中文!",
            "48 69 21",
        ),
        (Kind::Bcd, "bcd", "bcd", "BCD 码", "2026", "2026"),
        (
            Kind::Pad,
            "binary_pad",
            "binPad",
            "二进制补零",
            "42",
            "00101010",
        ),
        (
            Kind::Completion,
            "zero_align",
            "completion",
            "分段补零对齐",
            "0b1 0b101 0b11",
            "001 101 011",
        ),
        (
            Kind::Hybrid,
            "hybrid_code",
            "hybridCode",
            "前缀混合进制",
            "中A🙂",
            "x4e2d d65 b11111011001000010",
        ),
    ] {
        for decode in [false, true] {
            let params = match kind {
                Kind::Ascii => vec![
                    help(
                        ParamSpec::number("base", "进制", 2.0, 36.0, 1.0, 16.0),
                        "2/8/10/16 为定宽字节组，其他进制用空格分隔。",
                    ),
                    help(
                        ParamSpec::select("bits", "二进制位宽", &["8", "7"], "8"),
                        "仅在 base=2 生效；7 位编码只接受 ASCII。",
                    ),
                    help(
                        ParamSpec::toggle("invert", "二进制取反", false),
                        "逐位交换 0/1，其他进制不受影响。",
                    ),
                    help(
                        ParamSpec::toggle("bitReverse", "字节内位反转", false),
                        "逐组镜像位序，其他进制不受影响。",
                    ),
                    help(
                        ParamSpec::toggle("compat", "省略非二进制前导零", false),
                        "只改变编码显示；无分隔解码仍按照固定宽度拆分。",
                    ),
                ],
                Kind::Bcd => vec![help(
                    ParamSpec::toggle("compat", "二进制半字节", false),
                    "关闭以 Hex 半字节表示每位十进制；开启用四位二进制组，解码语义不会自动猜测。",
                )],
                Kind::Pad if !decode => vec![help(
                    ParamSpec::number("bits", "位宽", 1.0, 16384.0, 1.0, 8.0),
                    "输出宽度；超过范围的整数报错。",
                )],
                Kind::Completion if !decode => vec![help(
                    ParamSpec::number("bits", "对齐长度", 0.0, 16384.0, 1.0, 0.0),
                    "0 按最长分段；正数指定最小宽度，较长分段不会截断。",
                )],
                Kind::Hybrid if !decode => vec![
                    help(
                        ParamSpec::select(
                            "mode",
                            "进制",
                            &["hex", "dec", "oct", "bin", "auto"],
                            "hex",
                        ),
                        "auto 对每个 Unicode 码点选择最短表示；相同长度优先十进制。",
                    ),
                    help(
                        ParamSpec::text("sep", "分隔符", " ", false),
                        "上游兼容分隔符为空格、反斜杠或逗号；其他分隔符需先通过文本替换节点分离。",
                    ),
                ],
                _ => vec![],
            };
            let input = if matches!(kind, Kind::Ascii) && !decode {
                vec![req("data", "文本或字节", PortType::Any)]
            } else {
                vec![t_in()]
            };
            let output = if matches!(kind, Kind::Ascii) && decode {
                vec![t_out(), req("bytes", "完整字节", PortType::Bytes)]
            } else {
                vec![t_out()]
            };
            let description=match kind{Kind::Ascii=>"按 UTF-8 或原始字节逐字节转换进制，支持二进制位宽、取反和位序；bytes 保留完整二进制。",Kind::Bcd=>"十进制每位数字与 BCD 半字节互转，支持 Hex 与四位二进制表示。",Kind::Pad=>"非负十进制整数与定宽二进制互转，使用精确大整数；解码移除非二进制字符。",Kind::Completion=>"逐段补零到统一宽度或移除数字前导零；处理 0b/0x/0o 前缀，不截断长分段。",Kind::Hybrid=>"Unicode 码点与 b/x/o/d 前缀进制互转，支持中文和 emoji。解码拒绝无效数字与孤立代理项。"};
            let d = document_text(
                desc(
                    &format!("{id}_{}", if decode { "decode" } else { "encode" }),
                    "numeric.radix",
                    &format!("{title} {}", if decode { "解码" } else { "编码" }),
                    CYAN,
                    input,
                    output,
                    params,
                ),
                upstream,
                description,
                if decode { encoded } else { sample },
                json!({}),
            );
            reg.register(d, Arc::new(move || Arc::new(Codec { kind, decode })));
        }
    }
}
