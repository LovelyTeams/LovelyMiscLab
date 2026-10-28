//! MIME multipart framing, transfer encodings and explicit part selection.
use super::{
    codec_support::{document_text, help},
    mail_support::{bytes, checkpoint, limit, preview, string, MAX_DATA, MAX_WIRE},
    numeric_support::{number, space},
    prelude::*,
    transport_codecs::err,
};
use base64::{
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD},
    Engine as _,
};
use serde_json::{json, Value};

const MAX_PARTS: usize = 256;
const MAX_HEADERS: usize = 65536;
type Fields = Vec<(String, String)>;
const SAMPLE: &str = "Content-Type: multipart/mixed; boundary=demo\r\n\r\n--demo\r\nContent-Type: text/plain\r\nContent-Transfer-Encoding: base64\r\n\r\nSGVsbG8=\r\n--demo--";

fn boundary_ok(b: &str) -> Result<(), CoreError> {
    if b.is_empty()
        || b.len() > 70
        || b.ends_with(' ')
        || !b
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"'()+_,-./:=? ".contains(&c))
    {
        return Err(err(
            "MIME boundary 需要 1–70 个合法 ASCII 字符，末尾不能为空格",
        ));
    }
    Ok(())
}
fn header_value(s: &str) -> Result<(), CoreError> {
    if s.len() > 4096 || s.chars().any(char::is_control) {
        return Err(err("MIME 头字段最多 4096 字节且不能包含控制字符"));
    }
    Ok(())
}
fn quoted(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
fn token(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| (33..=126).contains(&b) && !b"()<>@,;:\\\"/[]?=".contains(&b))
}

/// Semicolons and escaped quotes inside a quoted parameter are not separators.
fn field(s: &str) -> Result<(String, Fields), CoreError> {
    let mut segments = Vec::new();
    let (mut start, mut in_quote, mut escape) = (0, false, false);
    for (i, c) in s.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if in_quote && c == '\\' {
            escape = true;
        } else if c == '"' {
            in_quote = !in_quote;
        } else if c == ';' && !in_quote {
            segments.push(&s[start..i]);
            start = i + 1;
        }
    }
    if in_quote || escape {
        return Err(err("MIME 参数引号未闭合"));
    }
    segments.push(&s[start..]);
    let mut params = Vec::new();
    for item in &segments[1..] {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let (name, value) = item.split_once('=').ok_or_else(|| err("MIME 参数缺少 ="))?;
        let name = name.trim().to_ascii_lowercase();
        if !token(&name) {
            return Err(err("MIME 参数名无效"));
        }
        let value = value.trim();
        let value = if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
            let mut out = String::new();
            let mut chars = value[1..value.len() - 1].chars();
            while let Some(c) = chars.next() {
                out.push(if c == '\\' {
                    chars.next().ok_or_else(|| err("MIME 引号内转义残缺"))?
                } else {
                    c
                });
            }
            out
        } else {
            if value.contains('"') {
                return Err(err("MIME 参数引号位置无效"));
            }
            value.to_owned()
        };
        if params.iter().any(|(n, _)| n == &name) {
            return Err(err("MIME 参数重复，无法确定值"));
        }
        params.push((name, value));
    }
    Ok((segments[0].trim().into(), params))
}
fn param<'a>(params: &'a [(String, String)], key: &str) -> Option<&'a str> {
    params
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, v)| v.as_str())
}

/// A line excludes its CRLF/LF; next points just after the line ending.
fn line(data: &[u8], pos: usize) -> (&[u8], usize) {
    let end = data[pos..]
        .iter()
        .position(|&b| b == b'\n')
        .map_or(data.len(), |i| pos + i);
    let text = &data[pos..end];
    (
        text.strip_suffix(b"\r").unwrap_or(text),
        if end < data.len() { end + 1 } else { end },
    )
}
fn headers(data: &[u8], mut pos: usize, ctx: &NodeCtx) -> Result<(Fields, usize), CoreError> {
    let start = pos;
    let mut fields: Fields = Vec::new();
    while pos < data.len() {
        ctx.cancel.check()?;
        let (row, next) = line(data, pos);
        if next - start > MAX_HEADERS {
            return Err(err("MIME 单组头字段超过 64 KiB"));
        }
        pos = next;
        if row.is_empty() {
            return Ok((fields, pos));
        }
        let row = std::str::from_utf8(row).map_err(|_| err("MIME 头字段不是 UTF-8/ASCII"))?;
        if row.chars().any(|c| c.is_control() && c != '\t') {
            return Err(err("MIME 头字段包含控制字符"));
        }
        if row.starts_with([' ', '\t']) {
            let last = fields
                .last_mut()
                .ok_or_else(|| err("MIME 折叠头字段缺少前一行"))?;
            last.1.push(' ');
            last.1.push_str(row.trim());
        } else {
            let (name, value) = row
                .split_once(':')
                .ok_or_else(|| err("MIME 头字段缺少冒号或正文前缺少空行"))?;
            if !token(name) {
                return Err(err("MIME 头字段名称无效"));
            }
            let name = name.to_ascii_lowercase();
            if name.starts_with("content-") && fields.iter().any(|(n, _)| *n == name) {
                return Err(err("MIME Content 头字段重复，无法确定解析方式"));
            }
            fields.push((name, value.trim().into()));
        }
    }
    Err(err("MIME 头字段后缺少空行"))
}

/// Only complete delimiter lines qualify; an embedded --token is body data.
fn delimiter(row: &[u8], marker: &[u8]) -> Option<bool> {
    let rest = row.strip_prefix(marker)?;
    let (closing, rest) = rest
        .strip_prefix(b"--")
        .map_or((false, rest), |s| (true, s));
    rest.iter()
        .all(|b| [b' ', b'\t'].contains(b))
        .then_some(closing)
}
fn collides(data: &[u8], boundary: &str) -> bool {
    let marker = format!("--{boundary}");
    // Prefix collisions also matter if another MIME implementation uses a prefix match.
    data.split(|&b| b == b'\n')
        .any(|row| row.starts_with(marker.as_bytes()))
}
fn qp_encode(data: &[u8], ctx: &NodeCtx) -> Result<String, CoreError> {
    let mut out = String::new();
    let mut col = 0;
    for (i, &b) in data.iter().enumerate() {
        if i % 4096 == 0 {
            ctx.cancel.check()?;
        }
        let chunk = if (33..=126).contains(&b) && b != b'=' {
            (b as char).to_string()
        } else {
            format!("={b:02X}")
        };
        if col + chunk.len() > 75 {
            out.push_str("=\r\n");
            col = 0;
        }
        col += chunk.len();
        out.push_str(&chunk);
    }
    Ok(out)
}
fn transfer_decode(body: &[u8], encoding: &str, ctx: &NodeCtx) -> Result<Vec<u8>, CoreError> {
    match encoding.to_ascii_lowercase().as_str() {
        "base64" => {
            let mut compact = Vec::new();
            for (i, &b) in body.iter().enumerate() {
                if i % 4096 == 0 {
                    ctx.cancel.check()?;
                }
                if !b.is_ascii_whitespace() {
                    compact.push(b);
                }
            }
            let engine = if compact.ends_with(b"=") {
                STANDARD
            } else {
                STANDARD_NO_PAD
            };
            engine
                .decode(compact)
                .map_err(|_| err("MIME part 的 Base64 非法或截断"))
        }
        "quoted-printable" | "quoted_printable" => {
            let mut out = Vec::new();
            let mut pos = 0;
            while pos < body.len() {
                ctx.cancel.check()?;
                if body[pos] == b'=' {
                    if body[pos + 1..].starts_with(b"\r\n") {
                        pos += 3;
                        continue;
                    }
                    if body.get(pos + 1) == Some(&b'\n') {
                        pos += 2;
                        continue;
                    }
                    if let (Some(h), Some(l)) = (
                        body.get(pos + 1).and_then(|b| (*b as char).to_digit(16)),
                        body.get(pos + 2).and_then(|b| (*b as char).to_digit(16)),
                    ) {
                        out.push((h * 16 + l) as u8);
                        pos += 3;
                        continue;
                    }
                }
                out.push(body[pos]);
                pos += 1;
            }
            Ok(out)
        }
        "" | "7bit" | "8bit" | "binary" => Ok(body.to_vec()),
        // Preserve unknown encodings as upstream does, with an explicit warning in the report.
        _ => Ok(body.to_vec()),
    }
}

struct Encoder;
impl Node for Encoder {
    fn run(&self, inputs: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        checkpoint(ctx, 0.05)?;
        let (parts, binary): (Vec<&[u8]>, bool) = match inputs.get("data") {
            Some(PortValue::Text(s)) => {
                limit(s.len(), MAX_DATA, "MIME 输入")?;
                (
                    s.split('|')
                        .map(|s| s.trim_matches(space))
                        .filter(|s| !s.is_empty())
                        .take(MAX_PARTS + 1)
                        .map(str::as_bytes)
                        .collect(),
                    false,
                )
            }
            Some(PortValue::StringList(list)) => {
                if list.len() > MAX_PARTS {
                    return Err(err("MIME part 数超过 256"));
                }
                limit(list.iter().map(String::len).sum(), MAX_DATA, "MIME 输入")?;
                (list.iter().map(|s| s.as_bytes()).collect(), false)
            }
            Some(PortValue::Bytes(b)) => {
                limit(b.len(), MAX_DATA, "MIME 输入")?;
                (vec![b], true)
            }
            _ => {
                return Err(err(
                    "MIME 编码输入需要文本（| 分隔）、字符串列表或单个 Bytes",
                ))
            }
        };
        if parts.is_empty() || parts.len() > MAX_PARTS {
            return Err(err("MIME 编码需要 1–256 个 part；空 part 请使用字符串列表"));
        }
        let encoding = string(p, "encoding", "auto")?;
        let encoding = match encoding {
            "auto" => {
                if binary {
                    "base64"
                } else {
                    "raw"
                }
            }
            "raw" | "base64" | "quoted-printable" => encoding,
            _ => return Err(err("encoding 必须是 auto、raw、base64 或 quoted-printable")),
        };
        let filename = string(p, "filename", "")?;
        header_value(filename)?;
        if !filename.is_empty() && parts.len() != 1 {
            return Err(err("filename 仅用于单个附件，多段消息请留空"));
        }
        let content_type = string(p, "contentType", "")?;
        let content_type = if content_type.is_empty() {
            if binary {
                "application/octet-stream"
            } else {
                "text/plain; charset=utf-8"
            }
        } else {
            content_type
        };
        header_value(content_type)?;
        let (media_type, _) = field(content_type)?;
        if !media_type
            .split_once('/')
            .is_some_and(|(a, b)| token(a) && token(b))
        {
            return Err(err("contentType 需要合法的 type/subtype"));
        }
        let mut bodies = Vec::new();
        for (i, part) in parts.iter().enumerate() {
            checkpoint(ctx, 0.1 + 0.5 * i as f32 / parts.len() as f32)?;
            bodies.push(match encoding {
                "base64" => STANDARD
                    .encode(part)
                    .as_bytes()
                    .chunks(76)
                    .map(|line| std::str::from_utf8(line).unwrap())
                    .collect::<Vec<_>>()
                    .join("\r\n"),
                "quoted-printable" => qp_encode(part, ctx)?,
                _ => std::str::from_utf8(part)
                    .map_err(|_| {
                        err("raw 正文必须是 UTF-8；二进制请选择 Base64 或 Quoted-Printable")
                    })?
                    .to_owned(),
            });
        }
        let configured = string(p, "boundary", "")?.trim_matches(space);
        let mut boundary = if configured.is_empty() {
            format!("----ctf-part{}", parts.len())
        } else {
            configured.to_owned()
        };
        boundary_ok(&boundary)?;
        if configured.is_empty() {
            let base = boundary.clone();
            let mut suffix = 0;
            while bodies.iter().any(|b| collides(b.as_bytes(), &boundary)) {
                ctx.cancel.check()?;
                suffix += 1;
                boundary = format!("{base}-{suffix}");
                if suffix > 1024 {
                    return Err(err("无法自动选择无冲突 boundary，请手工指定"));
                }
            }
        } else if bodies.iter().any(|b| collides(b.as_bytes(), &boundary)) {
            return Err(err("boundary 与正文行冲突，请更换分隔符"));
        }
        let mut text = format!("Content-Type: multipart/mixed; boundary=\"{boundary}\"\r\n");
        for (i, body) in bodies.iter().enumerate() {
            checkpoint(ctx, 0.65 + 0.3 * i as f32 / bodies.len() as f32)?;
            text.push_str(&format!(
                "\r\n--{boundary}\r\nContent-Type: {content_type}\r\n"
            ));
            if !filename.is_empty() {
                text.push_str(&format!(
                    "Content-Disposition: attachment; filename=\"{}\"\r\n",
                    quoted(filename)
                ));
            }
            if encoding != "raw" {
                text.push_str(&format!("Content-Transfer-Encoding: {encoding}\r\n"));
            }
            text.push_str("\r\n");
            text.push_str(body);
        }
        text.push_str(&format!("\r\n--{boundary}--"));
        limit(text.len(), MAX_WIRE, "MIME 编码结果")?;
        checkpoint(ctx, 1.0)?;
        Ok([
            ("bytes".into(), PortValue::Bytes(text.as_bytes().into())),
            ("text".into(), PortValue::Text(text)),
        ]
        .into())
    }
}

struct Part<'a> {
    headers: Fields,
    body: &'a [u8],
}
fn split_parts<'a>(
    data: &'a [u8],
    boundary: &str,
    ctx: &NodeCtx,
) -> Result<(Vec<Part<'a>>, bool), CoreError> {
    let marker = format!("--{boundary}");
    let mut parts = Vec::new();
    let mut current: Option<(Fields, usize)> = None;
    let mut pos = 0;
    let mut found = false;
    let mut closed = false;
    while pos < data.len() {
        ctx.cancel.check()?;
        let (row, next) = line(data, pos);
        if let Some(closing) = delimiter(row, marker.as_bytes()) {
            found = true;
            if let Some((headers, start)) = current.take() {
                let mut end = pos;
                if end > start && data[end - 1] == b'\n' {
                    end -= 1;
                    if end > start && data[end - 1] == b'\r' {
                        end -= 1;
                    }
                }
                parts.push(Part {
                    headers,
                    body: &data[start..end],
                });
            }
            if closing {
                closed = true;
                break;
            }
            if parts.len() >= MAX_PARTS {
                return Err(err("MIME part 数超过 256"));
            }
            let (h, body_start) = headers(data, next, ctx)?;
            current = Some((h, body_start));
            pos = body_start;
        } else {
            pos = next;
        }
    }
    if let Some((headers, start)) = current {
        parts.push(Part {
            headers,
            body: &data[start..],
        });
    }
    if !found {
        return Err(err("没有找到指定 boundary 的独立分隔行"));
    }
    Ok((parts, closed))
}

struct Decoder;
impl Node for Decoder {
    fn run(&self, inputs: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        checkpoint(ctx, 0.05)?;
        let data = bytes(inputs, "data")?;
        limit(data.len(), MAX_WIRE, "MIME 消息")?;
        let configured = string(p, "boundary", "")?.trim_matches(space);
        let (boundary, body_start) = if configured.is_empty() {
            let (root, end) = headers(data, 0, ctx)?;
            let (media_type, params) = field(
                param(&root, "content-type")
                    .ok_or_else(|| err("缺少 Content-Type；原始消息体请填写 boundary"))?,
            )?;
            if !media_type
                .split_once('/')
                .is_some_and(|(a, b)| a.eq_ignore_ascii_case("multipart") && token(b))
            {
                return Err(err("Content-Type 不是 multipart"));
            }
            (
                param(&params, "boundary")
                    .ok_or_else(|| err("未找到 MIME boundary 参数"))?
                    .to_owned(),
                end,
            )
        } else {
            (configured.to_owned(), 0)
        };
        boundary_ok(&boundary)?;
        let selection = number(p, "part", 1, 1, MAX_PARTS as i64)? as usize;
        let (parts, closed) = split_parts(&data[body_start..], &boundary, ctx)?;
        if selection > parts.len() && !(parts.is_empty() && selection == 1) {
            return Err(err(format!(
                "所选 part {selection} 不存在，共 {} 段",
                parts.len()
            )));
        }
        let mut reports = Vec::new();
        let mut rows = Vec::new();
        let mut total = 0;
        let mut selected = Vec::new();
        let mut selected_text = String::new();
        let mut warnings = Vec::new();
        if !closed {
            warnings.push("缺少结束 boundary，最后一段可能不完整".to_owned());
        }
        for (i, part) in parts.iter().enumerate() {
            checkpoint(ctx, 0.25 + 0.7 * i as f32 / parts.len().max(1) as f32)?;
            let (media_type, type_params) =
                field(param(&part.headers, "content-type").unwrap_or(""))?;
            let (_, disposition_params) =
                field(param(&part.headers, "content-disposition").unwrap_or(""))?;
            let name = param(&disposition_params, "filename")
                .or_else(|| param(&type_params, "name"))
                .unwrap_or("");
            let encoding = param(&part.headers, "content-transfer-encoding").unwrap_or("");
            let unknown = ![
                "",
                "base64",
                "quoted-printable",
                "quoted_printable",
                "7bit",
                "8bit",
                "binary",
            ]
            .contains(&encoding.to_ascii_lowercase().as_str());
            if unknown {
                warnings.push(format!(
                    "part {} 使用未知传输编码 {encoding}，字节按原样输出",
                    i + 1
                ));
            }
            let decoded = transfer_decode(part.body, encoding, ctx)?;
            total += decoded.len();
            limit(total, MAX_DATA, "MIME 解码后总大小")?;
            let decoded_text = String::from_utf8_lossy(&decoded);
            // The upstream normalizes literal CRLF before transfer decoding. Keep
            // that report convention, while the typed outputs preserve every byte.
            let normalized = part.body.windows(2).any(|w| w == b"\r\n");
            let report_bytes = if normalized {
                let mut body = Vec::with_capacity(part.body.len());
                for (j, &b) in part.body.iter().enumerate() {
                    if j % 4096 == 0 {
                        ctx.cancel.check()?;
                    }
                    if b == b'\r' && part.body.get(j + 1) == Some(&b'\n') {
                        continue;
                    }
                    body.push(b);
                }
                transfer_decode(&body, encoding, ctx)?
            } else {
                decoded.clone()
            };
            let content = preview(&String::from_utf8_lossy(&report_bytes), 500);
            let mut heading = format!(
                "part {}：{}",
                i + 1,
                if media_type.is_empty() {
                    "(无类型)"
                } else {
                    &media_type
                }
            );
            if !name.is_empty() {
                heading.push_str(&format!("  name={name}"));
            }
            if !encoding.is_empty() {
                heading.push_str(&format!("  encoding={encoding}"));
            }
            reports.push(format!("{heading}\n{content}"));
            rows.push(json!({"index":i+1,"type":media_type,"name":name,"encoding":encoding,"charset":param(&type_params,"charset"),"encodedLength":part.body.len(),"length":decoded.len(),"utf8Valid":std::str::from_utf8(&decoded).is_ok(),"preview":content}));
            if i + 1 == selection {
                selected_text = decoded_text.into_owned();
                selected = decoded;
            }
        }
        let mut report = if reports.is_empty() {
            format!("解析完成：boundary={boundary}，无 part")
        } else {
            reports.join(&format!("\n\n{}\n\n", "─".repeat(20)))
        };
        if !warnings.is_empty() {
            report.push_str(&format!("\n\n提示：{}", warnings.join("；")));
        }
        let metadata = json!({"boundary":boundary,"closed":closed,"count":parts.len(),"selectedPart":if parts.is_empty(){Value::Null}else{json!(selection)},"decodedBytes":total,"parts":rows,"warnings":warnings});
        checkpoint(ctx, 1.0)?;
        Ok([
            ("text".into(), PortValue::Text(report)),
            ("decoded".into(), PortValue::Text(selected_text)),
            ("bytes".into(), PortValue::Bytes(selected.into())),
            ("parts".into(), PortValue::Json(json!(rows))),
            ("json".into(), PortValue::Json(metadata)),
        ]
        .into())
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let mut enc = document_text(desc("mime_multipart_encode", "encoding.transport", "MIME multipart 编码", GREEN, vec![req("data", "文本、列表或附件字节", PortType::Any)], vec![t_out(), req("bytes", "完整消息字节", PortType::Bytes)], vec![
        help(ParamSpec::text("boundary", "分隔符（留空自动）", "", false), "1–70 个合法 ASCII 字符；自动使用 ----ctf-partN 并避开正文行冲突。手工指定的冲突值会报错。"),
        help(ParamSpec::select("encoding", "正文传输编码", &["auto", "raw", "base64", "quoted-printable"], "auto"), "auto：文本/列表直接保存，Bytes 使用 Base64；raw 需要 UTF-8。QP 逐字节保留 CR/LF。"),
        help(ParamSpec::text("contentType", "内容类型（留空自动）", "", false), "文本默认 text/plain; charset=utf-8，字节默认 application/octet-stream；统一用于各 part，不能包含换行。"),
        help(ParamSpec::text("filename", "单个附件文件名", "", false), "仅用于单段输入；填写后添加 Content-Disposition: attachment。最多 4096 字节，不允许控制字符。"),
    ]), "mimeMultipart", "将 | 分隔文本封装为 MIME multipart；字符串列表保留空段、空白与 | 本身，Bytes 封装单个二进制附件。最多 256 段、16 MiB 输入。", "Hello | 中文", json!({}));
    enc.aliases
        .extend(["多段邮件编码".into(), "multipart/mixed".into()]);
    enc.presentation.inline_params = vec!["boundary".into(), "encoding".into()];
    reg.register(enc, Arc::new(|| Arc::new(Encoder)));
    let mut dec = document_text(desc("mime_multipart_decode", "data.structure", "MIME multipart 解析", TEAL, vec![req("data", "完整消息或消息体", PortType::Any)], vec![t_out(), req("decoded", "所选段 UTF-8 文本", PortType::Text), req("bytes", "所选段完整字节", PortType::Bytes), req("parts", "分段目录", PortType::Json), req("json", "边界、完整性与分段信息", PortType::Json)], vec![
        help(ParamSpec::text("boundary", "覆盖分隔符（可选）", "", false), "默认从 Content-Type（含折叠头）读取；分析没有顶层头字段的原始消息体时需填写。"),
        help(ParamSpec::number("part", "输出第几段", 1., 256., 1., 1.), "从 1 开始；bytes/decoded 输出这一段，parts 列出全部段。解码 Base64 与 QP，不递归解嵌套 MIME，可继续连本节点。"),
    ]), "mimeMultipart", "按独立分隔行解析 multipart，解码传输编码，表格预览每段并选择完整字节供下游处理。最多 64 MiB 消息、256 段、16 MiB 解码总量；缺失结束边界会明确提示。文本使用 UTF-8，其他字符集请连接字符解码节点。", SAMPLE, json!({}));
    dec.aliases.extend([
        "多段邮件解析".into(),
        "附件提取".into(),
        "multipart/mixed".into(),
    ]);
    dec.presentation.inline_params = vec!["part".into(), "boundary".into()];
    dec.presentation
        .previews
        .insert("parts".into(), "table".into());
    reg.register(dec, Arc::new(|| Arc::new(Decoder)));
}
