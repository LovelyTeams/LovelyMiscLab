//! Quoted-Printable (RFC 2045) encode/decode — common in email forensics.
use super::{
    codec_support::{document_text, help},
    prelude::*,
};
use serde_json::json;

fn emit(out: &mut String, chunk: &str, line: &mut usize) {
    if *line + chunk.len() > 75 {
        out.push_str("=\r\n");
        *line = 0;
    }
    out.push_str(chunk);
    *line += chunk.len();
}

fn qp_encode(data: &[u8]) -> String {
    let mut out = String::new();
    let mut line = 0usize;
    for (i, &b) in data.iter().enumerate() {
        if b == b'\n' {
            out.push_str("\r\n");
            line = 0;
            continue;
        }
        if b == b'\r' {
            continue;
        }
        let is_ws = b == b' ' || b == b'\t';
        let next_break = matches!(data.get(i + 1), Some(b'\r') | Some(b'\n') | None);
        if ((33..=126).contains(&b) && b != b'=') || (is_ws && !next_break) {
            emit(&mut out, &(b as char).to_string(), &mut line);
        } else {
            emit(&mut out, &format!("={b:02X}"), &mut line);
        }
    }
    out
}

fn qp_decode(s: &str, strict: bool) -> Result<Vec<u8>, CoreError> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'=' {
            if i + 1 < b.len() && b[i + 1] == b'\n' {
                i += 2;
                continue;
            }
            if i + 2 < b.len() && b[i + 1] == b'\r' && b[i + 2] == b'\n' {
                i += 3;
                continue;
            }
            if i + 2 < b.len() {
                if let (Some(h), Some(l)) = (
                    (b[i + 1] as char).to_digit(16),
                    (b[i + 2] as char).to_digit(16),
                ) {
                    out.push((h * 16 + l) as u8);
                    i += 3;
                    continue;
                }
            }
            if strict {
                return Err(CoreError::Parse(format!(
                    "Quoted-Printable：偏移 {i} 的转义无效"
                )));
            }
            out.push(b'=');
            i += 1;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    Ok(out)
}

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        p: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let bytes = in_bytes(inputs, "data")?;
        if pstr(p, "mode", "邮件文本") == "邮件文本" {
            return Ok(out_text(qp_encode(&bytes)));
        }
        let mut out = String::new();
        for (i, b) in bytes.into_iter().enumerate() {
            if i % 4096 == 0 {
                ctx.cancel.check()?;
            }
            if ((33..=126).contains(&b) && b != b'=') || b == b' ' || b == b'\t' {
                out.push(b as char);
            } else {
                out.push_str(&format!("={b:02X}"));
            }
        }
        Ok(out_text(out))
    }
}

struct Dec;
impl Node for Dec {
    fn run(
        &self,
        inputs: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let bytes = qp_decode(in_text(inputs, "text")?, pbool(p, "strict", false))?;
        let mut m = PortMap::new();
        m.insert(
            "text".to_string(),
            PortValue::Text(String::from_utf8_lossy(&bytes).into_owned()),
        );
        m.insert(
            "bytes".to_string(),
            PortValue::Bytes(Arc::from(bytes.into_boxed_slice())),
        );
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        document_text(desc(
            "quoted_printable_encode",
            CHARSET,
            "Quoted-Printable 编码",
            INDIGO,
            vec![req("data", "输入", PortType::Any)],
            vec![t_out()],
            vec![help(ParamSpec::select("mode", "换行方式", &["邮件文本", "原始字节"], "邮件文本"), "邮件文本保留旧行为：统一 CRLF、折行与行尾空白编码；原始字节逐字节编码，CR/LF 也转义，不折行。")],
        ), "quotedPrintable", "Quoted-Printable 编码；原始字节模式精确保留 CR、LF 与全部二进制数据。", "Hello, 中文!\r\n", json!({"mode":"原始字节"})),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        document_text(
            desc(
                "quoted_printable_decode",
                CHARSET,
                "Quoted-Printable 解码",
                INDIGO,
                vec![t_in()],
                vec![
                    req("text", "文本", PortType::Text),
                    opt("bytes", "字节", PortType::Bytes),
                ],
                vec![help(
                    ParamSpec::toggle("strict", "严格转义校验", false),
                    "开启时拒绝残缺或非 Hex 的 =XX；关闭原样保留，兼容旧流程。",
                )],
            ),
            "quotedPrintable",
            "还原 =XX 与软换行；文本预览和完整字节分开输出，未知转义可选择严格拒绝。",
            "Hello =E4=B8=AD=E6=96=87=21",
            json!({"strict":true}),
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
