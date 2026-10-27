//! Explicit charset conversion with byte-preserving inputs and documented legacy mode.
use super::{
    codec_support::{document_text, help},
    numeric_support::{bad, space},
    prelude::*,
};
use encoding_rs::Encoding;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};
pub(super) static TABLES: LazyLock<Value> =
    LazyLock::new(|| serde_json::from_str(include_str!("charset_tables.json")).unwrap());
type ReverseMap = HashMap<char, Vec<u8>>;
static MAPS: LazyLock<Mutex<HashMap<&'static str, Arc<ReverseMap>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
const LIMIT: usize = 1_048_576;
pub(super) fn choice<'a>(
    p: &'a Value,
    key: &str,
    default: &'a str,
    values: &[&str],
) -> Result<&'a str, CoreError> {
    let s = pstr(p, key, default);
    if !values.contains(&s) {
        return Err(bad(&format!("{key} 参数无效")));
    }
    Ok(s)
}
pub(super) fn input(i: &PortMap) -> Result<Vec<u8>, CoreError> {
    let b = in_bytes(i, "data")?;
    if b.len() > LIMIT {
        return Err(bad("字符集输入最多 1 MiB"));
    }
    Ok(b)
}
pub(super) fn text(bytes: &[u8]) -> Result<&str, CoreError> {
    std::str::from_utf8(bytes).map_err(|_| bad("待编码数据必须是合法 UTF-8 文本"))
}
pub(super) fn codec(label: &str) -> Result<&'static Encoding, CoreError> {
    Encoding::for_label_no_replacement(label.as_bytes())
        .ok_or_else(|| bad(&format!("不支持的字符集：{label}")))
}
fn strip_bom<'a>(codec: &'static Encoding, b: &'a [u8]) -> &'a [u8] {
    let bom: &[u8] = if codec == encoding_rs::UTF_8 {
        &[0xef, 0xbb, 0xbf]
    } else if codec == encoding_rs::UTF_16BE {
        &[0xfe, 0xff]
    } else if codec == encoding_rs::UTF_16LE {
        &[0xff, 0xfe]
    } else {
        &[]
    };
    b.strip_prefix(bom).unwrap_or(b)
}
pub(super) fn decode(label: &str, b: &[u8], strict: bool) -> Result<(String, bool), CoreError> {
    let c = codec(label)?;
    let (s, errors) = c.decode_without_bom_handling(strip_bom(c, b));
    if strict && errors {
        return Err(bad("字节中包含该字符集的非法或截断序列"));
    }
    Ok((s.into_owned(), errors))
}
fn reverse_map(c: &'static Encoding, ctx: &mut NodeCtx) -> Result<Arc<ReverseMap>, CoreError> {
    if let Some(m) = MAPS.lock().unwrap().get(c.name()).cloned() {
        return Ok(m);
    }
    let mut map = ReverseMap::new();
    let mut insert = |bytes: &[u8]| {
        let (s, _) = c.decode_without_bom_handling(strip_bom(c, bytes));
        let mut chars = s.chars();
        if let Some(ch) = chars.next() {
            // Upstream requires exactly one UTF-16 code unit, excluding U+FFFD.
            if chars.next().is_none() && ch <= '\u{ffff}' && ch != '\u{fffd}' {
                map.entry(ch).or_insert_with(|| bytes.to_vec());
            }
        }
    };
    for b in 0..=255 {
        insert(&[b]);
    }
    if [
        encoding_rs::GBK,
        encoding_rs::GB18030,
        encoding_rs::BIG5,
        encoding_rs::SHIFT_JIS,
        encoding_rs::EUC_KR,
        encoding_rs::EUC_JP,
    ]
    .contains(&c)
    {
        for lead in 0x81..=0xfe {
            ctx.progress((lead - 0x81) as f32 / 126.0 * 0.3);
            ctx.cancel.check()?;
            for trail in 0x40..=0xfe {
                if trail != 0x7f {
                    insert(&[lead, trail]);
                }
            }
        }
    }
    let map = Arc::new(map);
    ctx.cancel.check()?;
    MAPS.lock().unwrap().insert(c.name(), map.clone());
    Ok(map)
}
pub(super) fn encode(
    label: &str,
    s: &str,
    legacy: bool,
    ctx: &mut NodeCtx,
) -> Result<(Vec<u8>, bool), CoreError> {
    let c = codec(label)?;
    ctx.cancel.check()?;
    if c == encoding_rs::UTF_8 {
        return Ok((s.as_bytes().to_vec(), false));
    }
    if !legacy {
        ctx.progress(0.4);
        ctx.cancel.check()?;
        let b = if c == encoding_rs::UTF_16BE {
            s.encode_utf16().flat_map(u16::to_be_bytes).collect()
        } else if c == encoding_rs::UTF_16LE {
            s.encode_utf16().flat_map(u16::to_le_bytes).collect()
        } else {
            let (b, _, error) = c.encode(s);
            if error {
                return Err(bad(&format!(
                    "{label} 无法表示部分字符；可显式选择 upstream 模式复现混合 UTF-8 回退"
                )));
            }
            b.into_owned()
        };
        ctx.cancel.check()?;
        return Ok((b, false));
    }
    let map = reverse_map(c, ctx)?;
    let mut out = Vec::new();
    let mut fallback = false;
    for (index, ch) in s.char_indices() {
        if index % 1024 < ch.len_utf8() {
            ctx.progress(0.3 + index as f32 / s.len().max(1) as f32 * 0.6);
            ctx.cancel.check()?;
        }
        if let Some(bytes) = map.get(&ch) {
            out.extend(bytes);
        } else {
            out.extend(ch.encode_utf8(&mut [0; 4]).as_bytes());
            fallback = true;
        }
    }
    Ok((out, fallback))
}
pub(super) fn byte_text(bytes: &[u8], format: &str) -> String {
    if format == "latin1" {
        bytes.iter().map(|&b| b as char).collect()
    } else {
        hex::encode(bytes)
    }
}
fn parse_bytes(s: &str, format: &str) -> Result<Vec<u8>, CoreError> {
    let compact: String = s
        .trim_matches(space)
        .chars()
        .filter(|&c| !space(c) && !",;:".contains(c))
        .collect();
    let sniff_hex = !compact.is_empty()
        && compact.len().is_multiple_of(2)
        && compact.bytes().all(|b| b.is_ascii_hexdigit());
    if format == "hex" || format == "auto" && sniff_hex {
        let trimmed = s.trim_matches(space);
        let body = if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
            &trimmed[2..]
        } else {
            trimmed
        };
        let body: String = body
            .chars()
            .filter(|&c| !space(c) && !",;:".contains(c))
            .collect();
        hex::decode(body).map_err(|_| bad("Hex 必须包含完整字节，可用空白或 ,;: 分隔"))
    } else {
        // JS iterates code points, then charCodeAt(0) takes the first UTF-16 unit.
        Ok(s.chars()
            .map(|ch| ch.encode_utf16(&mut [0; 2])[0] as u8)
            .collect())
    }
}
pub(super) fn outputs(s: String, bytes: Vec<u8>, source: Vec<u8>, lossy: bool) -> PortMap {
    [
        ("text".into(), PortValue::Text(s)),
        ("bytes".into(), PortValue::Bytes(bytes.into())),
        ("sourceBytes".into(), PortValue::Bytes(source.into())),
        ("lossy".into(), PortValue::Bool(lossy)),
    ]
    .into_iter()
    .collect()
}
pub(super) fn ports() -> Vec<PortSpec> {
    vec![
        t_out(),
        req("bytes", "转换后字节", PortType::Bytes),
        req("sourceBytes", "原始输入字节", PortType::Bytes),
        req("lossy", "有损或兼容回退", PortType::Bool),
    ]
}
pub(super) fn mode_param() -> ParamSpec {
    help(ParamSpec::select("codecMode","编码策略",&["standard","upstream"],"standard"),"standard 使用目标字符集，无法表示则报错；upstream 按上游反向枚举表编码，未知字符混入 UTF-8，lossy=true。")
}
pub(super) fn strict_param() -> ParamSpec {
    help(
        ParamSpec::toggle("strict", "拒绝有损解码", false),
        "关闭时以替换符保留可读结果并输出 lossy；原始字节始终保留在 sourceBytes。",
    )
}
#[derive(Clone, Copy)]
enum Kind {
    Gb,
    Big5,
    Sjis,
    Kr,
    Latin,
    Ebcdic,
    Utf16,
}
struct Charset(Kind, bool);
impl Node for Charset {
    fn run(&self, i: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let source = input(i)?;
        let format = choice(p, "format", "hex", &["hex", "latin1", "auto"])?;
        let legacy = choice(p, "codecMode", "standard", &["standard", "upstream"])? == "upstream";
        let strict = pbool(p, "strict", false);
        let label = match self.0 {
            Kind::Gb => choice(p, "charset", "gbk", &["gbk", "gb2312", "gb18030"])?,
            Kind::Big5 => "big5",
            Kind::Sjis => "shift_jis",
            Kind::Kr => "euc-kr",
            Kind::Latin => choice(p, "charset", "iso-8859-1", &LATIN)?,
            Kind::Ebcdic => choice(p, "codepage", "cp1047", &["cp037", "cp1047"])?,
            Kind::Utf16 => choice(p, "endian", "BE", &["BE", "LE", "auto"])?,
        };
        if self.1 {
            let bytes = if matches!(i.get("data"), Some(PortValue::Text(_))) {
                parse_bytes(text(&source)?, format)?
            } else {
                source
            };
            let (s, lossy) = match self.0 {
                Kind::Ebcdic => {
                    let table = TABLES["ebcdic"][label].as_object().unwrap();
                    let mut rev = [None; 256];
                    for (c, b) in table {
                        rev[b.as_u64().unwrap() as usize] = c.chars().next();
                    }
                    let mut lossy = false;
                    let s = bytes
                        .iter()
                        .map(|&b| {
                            rev[b as usize].unwrap_or_else(|| {
                                lossy = true;
                                '\u{fffd}'
                            })
                        })
                        .collect();
                    if strict && lossy {
                        return Err(bad("字节不在上游 EBCDIC ASCII 子集码表中"));
                    }
                    (s, lossy)
                }
                Kind::Utf16 => {
                    // Upstream removes one BOM explicitly, then TextDecoder removes
                    // a second matching BOM. Keep this precise compatibility.
                    let (body, endian) = if let Some(body) = bytes.strip_prefix(&[0xfe, 0xff]) {
                        (body, "utf-16be")
                    } else if let Some(body) = bytes.strip_prefix(&[0xff, 0xfe]) {
                        (body, "utf-16le")
                    } else {
                        (
                            bytes.as_slice(),
                            if label == "LE" {
                                "utf-16le"
                            } else {
                                "utf-16be"
                            },
                        )
                    };
                    decode(endian, body, strict)?
                }
                _ => decode(label, &bytes, strict)?,
            };
            ctx.progress(1.0);
            ctx.cancel.check()?;
            return Ok(outputs(s.clone(), s.into_bytes(), bytes, lossy));
        }
        let s = text(&source)?;
        let (bytes, lossy) = match self.0 {
            Kind::Ebcdic => {
                let table = &TABLES["ebcdic"][label];
                let bytes = s
                    .chars()
                    .map(|c| {
                        table[c.to_string()]
                            .as_u64()
                            .map(|n| n as u8)
                            .ok_or_else(|| {
                                bad(&format!(
                                    "{label} 上游 ASCII 子集无法表示 U+{:04X}",
                                    c as u32
                                ))
                            })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                (bytes, false)
            }
            Kind::Utf16 => {
                let le = label == "LE";
                let mut bytes = Vec::new();
                if pbool(p, "bom", true) {
                    bytes.extend(if le { [0xff, 0xfe] } else { [0xfe, 0xff] });
                }
                bytes.extend(s.encode_utf16().flat_map(|u| {
                    if le {
                        u.to_le_bytes()
                    } else {
                        u.to_be_bytes()
                    }
                }));
                (bytes, false)
            }
            _ => encode(label, s, legacy, ctx)?,
        };
        ctx.progress(1.0);
        ctx.cancel.check()?;
        Ok(outputs(byte_text(&bytes, format), bytes, source, lossy))
    }
}
const LATIN: [&str; 14] = [
    "iso-8859-1",
    "iso-8859-2",
    "iso-8859-5",
    "iso-8859-6",
    "iso-8859-7",
    "iso-8859-8",
    "iso-8859-9",
    "iso-8859-15",
    "windows-1250",
    "windows-1251",
    "windows-1252",
    "windows-1253",
    "windows-1254",
    "windows-1257",
];
pub fn register(reg: &mut NodeRegistry) {
    for (kind,id,upstream,name,example,encoded,note) in [
        (Kind::Gb,"gb_charset","gbCharset","GBK / GB2312 / GB18030","中文","d6d0cec4","GB2312 标签按浏览器约定指向 GBK；standard 支持 GB18030 四字节，upstream 只枚举一/二字节。"),
        (Kind::Big5,"big5","big5","Big5 繁体中文","中文","a4a4a4e5","使用 WHATWG Big5 字符表。"),
        (Kind::Sjis,"shift_jis","shiftJis","Shift-JIS 日文","日本語","93fa967b8cea","支持半角片假名和双字节日文。"),
        (Kind::Kr,"euc_kr","eucKr","EUC-KR 韩文","한글","c7d1b1db","使用 WHATWG EUC-KR / Windows-949 扩展。"),
        (Kind::Latin,"latin_charset","latinCharset","Latin / Windows 单字节","café","636166e9","支持14种标签；ISO-8859-1 按浏览器约定映射 Windows-1252，format=latin1 仅指字节表示。"),
        (Kind::Ebcdic,"ebcdic","ebcdic","EBCDIC ASCII 子集","Hello, World!","c8859393966b40e6969993845a","复现固定上游037/1047的可打印ASCII子集，不声称是完整EBCDIC字符表；未知字符编码报错。"),
        (Kind::Utf16,"utf16","utf16","UTF-16 BE/LE","中文🙂","feff4e2d6587d83dde42","BOM 优先于字节序；无BOM时按参数，auto 默认BE。保留上游连续双BOM剥离规则。"),
    ] {
        for decode in [false,true] {
            let mut params = match kind {
                Kind::Gb => vec![help(ParamSpec::select("charset","字符集",&["gbk","gb2312","gb18030"],"gbk"),note)],
                Kind::Latin => vec![help(ParamSpec::select("charset","字符集",&LATIN,"iso-8859-1"),note)],
                Kind::Ebcdic => vec![help(ParamSpec::select("codepage","码页",&["cp037","cp1047"],"cp1047"),note)],
                Kind::Utf16 => vec![help(ParamSpec::select("endian","字节序",&["BE","LE","auto"],"BE"),note)],
                _ => vec![],
            };
            if matches!(kind,Kind::Utf16) && !decode { params.push(help(ParamSpec::toggle("bom","添加 BOM",true),"在输出前添加 FEFF（BE）或 FFFE（LE）；与文本中的 U+FEFF 分别处理。")); }
            params.push(help(ParamSpec::select("format","字节表示",&["hex","latin1","auto"],"hex"),"Hex 或每字节一个 Latin1 字符；auto 解码只把偶数纯 Hex 当字节，否则按 Latin1，编码等同 Hex。直接字节输入不使用此参数。"));
            if decode { params.push(strict_param()); }
            else if !matches!(kind,Kind::Ebcdic|Kind::Utf16) { params.push(mode_param()); }
            let mut d = document_text(desc(&format!("{id}_{}",if decode{"decode"}else{"encode"}),"encoding.charset",&format!("{name} {}",if decode{"解码"}else{"编码"}),TEAL,vec![req("data",if decode{"字节或字节表示"}else{"文本或 UTF-8 字节"},PortType::Any)],ports(),params),upstream,&format!("{note} 编码输出目标字节，解码输出UTF-8字节；sourceBytes保留输入，lossy报告替换或兼容回退。"),if decode{encoded}else{example},json!({}));
            d.cost = Cost::Heavy;
            reg.register(d,Arc::new(move || Arc::new(Charset(kind,decode))));
        }
    }
}
