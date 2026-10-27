//! Charset comparison split into explicit text encoding and byte decoding nodes.
use super::{
    codec_support::{document_text, help},
    numeric_support::{bad, space},
    prelude::*,
};
use serde_json::{json, Value};
const ENCODINGS: [&str; 11] = [
    "utf-8",
    "utf-16le",
    "utf-16be",
    "latin1",
    "iso-8859-1",
    "gbk",
    "gb18030",
    "big5",
    "shift_jis",
    "euc-jp",
    "windows-1252",
];
struct Probe(bool);
impl Node for Probe {
    fn run(&self, i: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let mut rows = Vec::new();
        let mut results = Vec::new();
        let mut lines = Vec::new();
        if self.0 {
            let data = match i.get("data") {
                Some(PortValue::Text(s)) if s.trim_matches(space).is_empty() => Vec::new(),
                Some(PortValue::Text(s)) => super::transport_codecs::hex_input(s)?,
                _ => in_bytes(i, "data")?,
            };
            if data.len() > 1_048_576 {
                return Err(bad("字符集分析输入最多 1 MiB"));
            }
            if data.is_empty() {
                lines.push("（空输入）".into());
            } else {
                lines.push(format!("输入: hex ({} 字节)\n", data.len()));
                for (index, enc) in ENCODINGS.iter().enumerate() {
                    ctx.progress(index as f32 / 11.0);
                    ctx.cancel.check()?;
                    let codec = encoding_rs::Encoding::for_label(enc.as_bytes()).unwrap();
                    let bytes = match *enc {
                        "utf-8" => data.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&data),
                        "utf-16le" => data.strip_prefix(&[0xff, 0xfe]).unwrap_or(&data),
                        "utf-16be" => data.strip_prefix(&[0xfe, 0xff]).unwrap_or(&data),
                        _ => &data,
                    };
                    let (text, _) = codec.decode_without_bom_handling(bytes);
                    let replaced = text.contains('\u{fffd}');
                    lines.push(format!(
                        "{enc} 解码: {text}{}",
                        if replaced {
                            "  （含替换符 U+FFFD）"
                        } else {
                            ""
                        }
                    ));
                    rows.push(
                        json!({"charset":enc,"text":text,"replacements":replaced,"status":"ok"}),
                    );
                    results.push(text.into_owned());
                }
            }
        } else {
            let s = in_text(i, "text")?.trim_matches(space);
            if s.len() > 1_048_576 {
                return Err(bad("字符集分析输入最多 1 MiB"));
            }
            if s.is_empty() {
                lines.push("（空输入）".into());
            } else {
                lines.push(format!("输入: 文本 ({} 字符)\n", s.encode_utf16().count()));
                for (index, enc) in ENCODINGS.iter().enumerate() {
                    ctx.progress(index as f32 / 11.0);
                    ctx.cancel.check()?;
                    let bytes = match *enc {
                        "utf-8" => Some(s.as_bytes().to_vec()),
                        "utf-16le" => Some(s.encode_utf16().flat_map(u16::to_le_bytes).collect()),
                        "utf-16be" => Some(s.encode_utf16().flat_map(u16::to_be_bytes).collect()),
                        "latin1" | "iso-8859-1" => {
                            if s.chars().any(|c| c as u32 > 255) {
                                lines.push(format!("{enc} 编码: （非 Latin1 域，无法表示）"));
                                rows.push(json!({"charset":enc,"status":"unrepresentable"}));
                                None
                            } else {
                                Some(s.chars().map(|c| c as u8).collect())
                            }
                        }
                        _ => {
                            if pbool(p, "legacyReport", false) {
                                lines.push(format!("{enc} 编码: TextEncoder 不支持（仅 utf-8）"));
                                rows.push(json!({"charset":enc,"status":"upstream-unsupported"}));
                                None
                            } else {
                                let codec =
                                    encoding_rs::Encoding::for_label(enc.as_bytes()).unwrap();
                                let (bytes, _, error) = codec.encode(s);
                                if error {
                                    lines.push(format!(
                                        "{enc} 编码: （字符超出码表，无法无损表示）"
                                    ));
                                    rows.push(json!({"charset":enc,"status":"unrepresentable"}));
                                    None
                                } else {
                                    Some(bytes.into_owned())
                                }
                            }
                        }
                    };
                    if let Some(bytes) = bytes {
                        let h = hex::encode(bytes);
                        lines.push(format!("{enc} 编码: {h}"));
                        rows.push(json!({"charset":enc,"hex":h,"status":"ok"}));
                        results.push(h);
                    }
                }
            }
        }
        ctx.cancel.check()?;
        Ok([
            ("text".into(), PortValue::Text(lines.join("\n"))),
            ("rows".into(), PortValue::Json(json!(rows))),
            ("results".into(), PortValue::StringList(results)),
        ]
        .into_iter()
        .collect())
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for decode in [false, true] {
        let params = if decode {
            vec![]
        } else {
            vec![help(ParamSpec::toggle("legacyReport","兼容上游报告",false),"开启时保留上游未实现 GBK 等编码的报告文案；默认用 Rust 实际编码，无法表示的字符明确标注。")]
        };
        let mut d=document_text(desc(if decode{"charset_probe_decode"}else{"charset_probe_encode"},"encoding.charset",if decode{"多字符集解码分析"}else{"多字符集编码分析"},TEAL,if decode{vec![req("data","Hex 或原始字节",PortType::Any)]}else{vec![t_in()]},vec![t_out(),req("rows","字符集明细",PortType::Json),req("results","转换结果",PortType::StringList)],params),"standardCode","对比 UTF-8、UTF-16、Latin1、GBK/GB18030、Big5、Shift-JIS、EUC-JP、Windows-1252；按方向明确输入，保留替换符状态与可接续结果。",if decode{"e4b8ade69687"}else{"中文"},json!({}));
        d.presentation
            .previews
            .insert("rows".into(), "table".into());
        d.cost = Cost::Heavy;
        reg.register(d, Arc::new(move || Arc::new(Probe(decode))));
    }
}
