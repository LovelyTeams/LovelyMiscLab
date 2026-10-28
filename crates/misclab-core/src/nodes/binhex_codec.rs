//! BinHex 4.0: HQX, RLE90, three CRC-16/XMODEM sections and lossless forks.
use super::{
    codec_support::{document_text, help},
    mail_support::{bytes, checkpoint, limit, preview, string, MAX_DATA, MAX_WIRE},
    numeric_support::number,
    prelude::*,
    transport_codecs::err,
};
use serde_json::{json, Value};

const ALPHABET: &[u8; 64] = b"!\"#$%&'()*+,-012345689@ABCDEFGHIJKLMNPQRSTUVXYZ[`abcdefhijklmpqr";
const BANNER: &str = "(This file must be converted with BinHex 4.0)";

fn crc(data: &[u8], ctx: &NodeCtx) -> Result<u16, CoreError> {
    let mut value = 0u16;
    for (i, &b) in data.iter().enumerate() {
        if i % 4096 == 0 {
            ctx.cancel.check()?;
        }
        value ^= (b as u16) << 8;
        for _ in 0..8 {
            value = (value << 1) ^ if value & 0x8000 != 0 { 0x1021 } else { 0 };
        }
    }
    Ok(value)
}

fn rle_encode(data: &[u8], ctx: &NodeCtx) -> Result<Vec<u8>, CoreError> {
    let mut result = Vec::new();
    let mut pos = 0;
    while pos < data.len() {
        // A run can skip an exact multiple of 4096, so check every run.
        ctx.cancel.check()?;
        let b = data[pos];
        if b == 0x90 {
            result.extend_from_slice(&[0x90, 0]);
            pos += 1;
        } else {
            let run = data[pos..]
                .iter()
                .take(255)
                .take_while(|&&x| x == b)
                .count();
            if run >= 3 {
                result.extend_from_slice(&[b, 0x90, run as u8]);
                pos += run;
            } else {
                result.push(b);
                pos += 1;
            }
        }
    }
    Ok(result)
}
fn rle_decode(data: &[u8], ctx: &NodeCtx) -> Result<Vec<u8>, CoreError> {
    let mut result = Vec::new();
    let mut pos = 0;
    while pos < data.len() {
        ctx.cancel.check()?;
        let b = data[pos];
        pos += 1;
        if b != 0x90 {
            result.push(b);
        } else {
            let count = *data
                .get(pos)
                .ok_or_else(|| err("BinHex RLE90 尾部缺少计数"))?;
            pos += 1;
            if count == 0 {
                result.push(0x90);
            } else {
                let last = *result
                    .last()
                    .ok_or_else(|| err("BinHex RLE90 重复标记缺少前导字节"))?;
                limit(
                    result.len() + count as usize - 1,
                    MAX_DATA + 89,
                    "BinHex 解压后内容",
                )?;
                result.resize(result.len() + count as usize - 1, last);
            }
        }
        limit(result.len(), MAX_DATA + 89, "BinHex 解压后内容")?;
    }
    Ok(result)
}

fn hqx_encode(data: &[u8], ctx: &NodeCtx) -> Result<String, CoreError> {
    let mut result = format!("{BANNER}\n\n:");
    let (mut acc, mut bits, mut col) = (0u32, 0, 0);
    let emit = |value: u8, result: &mut String, col: &mut usize| {
        if *col == 64 {
            result.push('\n');
            *col = 0;
        }
        result.push(ALPHABET[value as usize] as char);
        *col += 1;
    };
    for (i, &b) in data.iter().enumerate() {
        if i % 4096 == 0 {
            ctx.cancel.check()?;
        }
        acc = (acc << 8) | b as u32;
        bits += 8;
        while bits >= 6 {
            bits -= 6;
            emit(((acc >> bits) & 63) as u8, &mut result, &mut col);
        }
    }
    if bits > 0 {
        emit(((acc << (6 - bits)) & 63) as u8, &mut result, &mut col);
    }
    result.push(':');
    Ok(result)
}
fn hqx_decode(text: &str, ctx: &NodeCtx) -> Result<Vec<u8>, CoreError> {
    let text = text
        .find(BANNER)
        .map_or(text, |pos| &text[pos + BANNER.len()..]);
    let start = text
        .find(':')
        .ok_or_else(|| err("未找到 BinHex 数据起始 ':'"))?;
    let body = &text[start + 1..];
    let end = body
        .find(':')
        .ok_or_else(|| err("未找到 BinHex 数据结束 ':'"))?;
    let mut result = Vec::new();
    let (mut acc, mut bits) = (0u32, 0);
    for (i, b) in body[..end].bytes().enumerate() {
        if i % 4096 == 0 {
            ctx.cancel.check()?;
        }
        if b.is_ascii_whitespace() {
            continue;
        }
        let value = ALPHABET
            .iter()
            .position(|&v| v == b)
            .ok_or_else(|| err("BinHex 包含 HQX 码表外字符"))?;
        acc = (acc << 6) | value as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            result.push((acc >> bits) as u8);
        }
    }
    if bits == 6 || (acc & ((1 << bits) - 1)) != 0 {
        return Err(err("BinHex 尾部位数或补零位无效"));
    }
    Ok(result)
}

fn fourcc(p: &Value, name: &str, default: &str) -> Result<[u8; 4], CoreError> {
    let s = string(p, name, default)?;
    if s.len() != 4 || !s.bytes().all(|b| (32..=126).contains(&b)) {
        return Err(err(format!("{name} 需要 4 个可打印 ASCII 字符")));
    }
    Ok(s.as_bytes().try_into().unwrap())
}
fn data_fork(inputs: &PortMap, p: &Value) -> Result<Vec<u8>, CoreError> {
    let mode = string(p, "textEncoding", "auto")?;
    if !["auto", "UTF-8", "Latin-1"].contains(&mode) {
        return Err(err("textEncoding 必须是 auto、UTF-8 或 Latin-1"));
    }
    let b = bytes(inputs, "data")?;
    limit(b.len(), MAX_DATA, "数据叉")?;
    if let Some(PortValue::Text(s)) = inputs.get("data") {
        let latin = s.chars().all(|c| (c as u32) <= 255);
        if mode == "Latin-1" && !latin {
            return Err(err("Latin-1 无法表示输入中的字符，请选择 UTF-8"));
        }
        if mode == "Latin-1" || mode == "auto" && latin {
            return Ok(s.chars().map(|c| c as u8).collect());
        }
    }
    Ok(b.to_vec())
}

struct Encoder;
impl Node for Encoder {
    fn run(&self, inputs: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        checkpoint(ctx, 0.05)?;
        let data = data_fork(inputs, p)?;
        let resource = match inputs.get("resource") {
            None | Some(PortValue::None) => &[][..],
            Some(PortValue::Bytes(b)) => b,
            _ => return Err(err("资源叉仅接受 Bytes")),
        };
        limit(data.len() + resource.len(), MAX_DATA, "两叉总大小")?;
        let filename = string(p, "filename", "file.txt")?.trim();
        let filename = if filename.is_empty() {
            "file.txt"
        } else {
            filename
        };
        if filename.len() > 63 || filename.chars().any(char::is_control) {
            return Err(err("文件名需要 1–63 个 UTF-8 字节且不能包含控制字符"));
        }
        let mut raw = vec![filename.len() as u8];
        raw.extend_from_slice(filename.as_bytes());
        raw.push(0);
        raw.extend_from_slice(&fourcc(p, "fileType", "TEXT")?);
        raw.extend_from_slice(&fourcc(p, "creator", "CTFB")?);
        raw.extend_from_slice(&(number(p, "flags", 0, 0, 65535)? as u16).to_be_bytes());
        raw.extend_from_slice(&(data.len() as u32).to_be_bytes());
        raw.extend_from_slice(&(resource.len() as u32).to_be_bytes());
        raw.extend_from_slice(&crc(&raw, ctx)?.to_be_bytes());
        raw.extend_from_slice(&data);
        raw.extend_from_slice(&crc(&data, ctx)?.to_be_bytes());
        raw.extend_from_slice(resource);
        raw.extend_from_slice(&crc(resource, ctx)?.to_be_bytes());
        checkpoint(ctx, 0.4)?;
        let packed = rle_encode(&raw, ctx)?;
        checkpoint(ctx, 0.7)?;
        let text = hqx_encode(&packed, ctx)?;
        checkpoint(ctx, 1.0)?;
        Ok(out_text(text))
    }
}

fn cc(b: &[u8]) -> String {
    b.iter()
        .map(|&v| {
            if (32..=126).contains(&v) {
                v as char
            } else {
                '.'
            }
        })
        .collect()
}
fn crc_info(stored: u16, actual: u16) -> Value {
    json!({"stored":format!("{stored:04x}"),"calculated":format!("{actual:04x}"),"valid":stored==actual})
}
fn crc_mark(j: &Value) -> String {
    if j["valid"] == true {
        "✓ 匹配".into()
    } else {
        format!(
            "✗ 不匹配 (存 0x{} ≠ 算 0x{})",
            j["stored"].as_str().unwrap(),
            j["calculated"].as_str().unwrap()
        )
    }
}
struct Decoder;
impl Node for Decoder {
    fn run(&self, inputs: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        checkpoint(ctx, 0.05)?;
        let text = in_text(inputs, "text")?;
        limit(text.len(), MAX_WIRE, "BinHex 文本")?;
        let strict = match p.get("strictCrc") {
            None => false,
            Some(Value::Bool(b)) => *b,
            _ => return Err(err("strictCrc 必须是布尔值")),
        };
        let name_encoding = string(p, "nameEncoding", "UTF-8")?;
        if !["UTF-8", "MacRoman"].contains(&name_encoding) {
            return Err(err("nameEncoding 必须是 UTF-8 或 MacRoman"));
        }
        let packed = hqx_decode(text, ctx)?;
        checkpoint(ctx, 0.3)?;
        let raw = rle_decode(&packed, ctx)?;
        let nl = *raw.first().ok_or_else(|| err("BinHex 主体为空"))? as usize;
        let header_len = nl + 20;
        if !(1..=63).contains(&nl) || raw.len() < header_len + 6 {
            return Err(err("BinHex 文件名长度无效或文件头不完整"));
        }
        let dlen = u32::from_be_bytes(raw[nl + 12..nl + 16].try_into().unwrap()) as usize;
        let rlen = u32::from_be_bytes(raw[nl + 16..nl + 20].try_into().unwrap()) as usize;
        limit(dlen.saturating_add(rlen), MAX_DATA, "两叉总大小")?;
        if raw.len() != header_len + 6 + dlen + rlen {
            return Err(err(
                "BinHex 数据叉、资源叉或 CRC 长度不符（含截断或多余尾部数据）",
            ));
        }
        let data_start = header_len + 2;
        let resource_start = data_start + dlen + 2;
        let data = &raw[data_start..data_start + dlen];
        let resource = &raw[resource_start..resource_start + rlen];
        let read16 = |pos| u16::from_be_bytes([raw[pos], raw[pos + 1]]);
        let header_crc = crc_info(read16(header_len), crc(&raw[..header_len], ctx)?);
        let data_crc = crc_info(read16(data_start + dlen), crc(data, ctx)?);
        let resource_crc = crc_info(read16(resource_start + rlen), crc(resource, ctx)?);
        let valid = [&header_crc, &data_crc, &resource_crc]
            .iter()
            .all(|j| j["valid"] == true);
        if strict && !valid {
            return Err(err("BinHex CRC 校验不匹配；可关闭严格 CRC 以检查受损内容"));
        }
        checkpoint(ctx, 0.8)?;
        let name_bytes = &raw[1..1 + nl];
        let name = if name_encoding == "MacRoman" {
            encoding_rs::MACINTOSH
                .decode_without_bom_handling(name_bytes)
                .0
                .into_owned()
        } else {
            String::from_utf8_lossy(name_bytes).into_owned()
        };
        let version = raw[nl + 1];
        let file_type = cc(&raw[nl + 2..nl + 6]);
        let creator = cc(&raw[nl + 6..nl + 10]);
        let flags = read16(nl + 10);
        let printable =
            !data.is_empty() && data.iter().all(|b| *b >= 32 || [9, 10, 13].contains(b));
        let decoded = String::from_utf8_lossy(data);
        let body = if printable {
            preview(&decoded, 4096)
        } else {
            let mut s = hex::encode(&data[..data.len().min(2048)]);
            if data.len() > 2048 {
                s.push_str("\n…（截断）");
            }
            s
        };
        let mut report = format!("=== BinHex 4.0 解码 ===\n文件名 = {name}\n版本 = {version}\ntype = '{file_type}'  creator = '{creator}'  flags = 0x{flags:04x}\n数据叉长度 = {dlen}  资源叉长度 = {rlen}\n头 CRC     = {}\n数据叉 CRC = {}\n资源叉 CRC = {}\n\n--- 数据叉内容（{}）---\n{body}", crc_mark(&header_crc), crc_mark(&data_crc), crc_mark(&resource_crc), if printable { "UTF-8 文本" } else { "十六进制" });
        if rlen > 0 {
            report.push_str(&format!(
                "\n\n--- 资源叉（hex，{rlen} 字节）---\n{}",
                hex::encode(&resource[..rlen.min(2048)])
            ));
            if rlen > 2048 {
                report.push_str("\n…（截断）");
            }
        }
        let mut warnings = Vec::new();
        if !valid {
            warnings.push("CRC 不匹配，当前内容可能损坏");
        }
        if version != 0 {
            warnings.push("版本字段非 0，按 BinHex 4.0 结构解析");
        }
        if std::str::from_utf8(data).is_err() {
            warnings.push("文本预览包含不可解码字节，请使用 bytes 输出");
        }
        let metadata = json!({"name":name,"nameHex":hex::encode(name_bytes),"version":version,"fileType":file_type,"creator":creator,"flags":flags,"dataLength":dlen,"resourceLength":rlen,"crcValid":valid,"crc":{"header":header_crc,"data":data_crc,"resource":resource_crc},"warnings":warnings});
        let result = [
            ("text".into(), PortValue::Text(report)),
            ("decoded".into(), PortValue::Text(decoded.into_owned())),
            ("bytes".into(), PortValue::Bytes(data.into())),
            ("resource".into(), PortValue::Bytes(resource.into())),
            ("json".into(), PortValue::Json(metadata)),
        ]
        .into();
        checkpoint(ctx, 1.0)?;
        Ok(result)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let mut enc = document_text(desc("binhex_encode", "encoding.transport", "BinHex 4.0 编码", GREEN,
        vec![req("data", "数据叉：文本或字节", PortType::Any), opt("resource", "资源叉（可选）", PortType::Bytes)], vec![t_out()], vec![
            help(ParamSpec::text("filename", "文件名", "file.txt", false), "1–63 个 UTF-8 字节；空值使用 file.txt，过长或含控制字符报错，不截断多字节字符。"),
            help(ParamSpec::select("textEncoding", "文本字节编码", &["auto", "UTF-8", "Latin-1"], "auto"), "auto 与上游一致：全为 Latin-1 字符时逐字符取字节，否则整串 UTF-8。Bytes 输入始终原样保留。"),
            help(ParamSpec::text("fileType", "文件类型 FourCC", "TEXT", false), "4 个可打印 ASCII 字符，例如 TEXT 或 BINA。"),
            help(ParamSpec::text("creator", "创建者 FourCC", "CTFB", false), "4 个可打印 ASCII 字符；默认 CTFB 对应上游。"),
            help(ParamSpec::number("flags", "Finder 标志", 0., 65535., 1., 0.), "保存 16 位 Finder 标志原值；默认 0。两叉总大小不超过 16 MiB。"),
        ]), "binhex", "封装 BinHex 4.0 的文件头、数据叉、资源叉与三处 CRC。可取消，输出 HQX 文本。", "Hello, 中文!", json!({}));
    enc.presentation.inline_params = vec!["filename".into(), "textEncoding".into()];
    enc.aliases.extend(["HQX 编码".into(), "Mac BinHex".into()]);
    reg.register(enc, Arc::new(|| Arc::new(Encoder)));
    let mut dec = document_text(desc("binhex_decode", "encoding.transport", "BinHex 4.0 解码", GREEN, vec![t_in()], vec![t_out(), req("decoded", "数据叉文本", PortType::Text), req("bytes", "完整数据叉", PortType::Bytes), req("resource", "完整资源叉", PortType::Bytes), req("json", "文件头与 CRC", PortType::Json)], vec![
        help(ParamSpec::toggle("strictCrc", "CRC 不匹配时报错", false), "默认保留上游取证行为：报告三处 CRC 的结果并输出受损内容；开启后任一不匹配即报错。结构残缺始终报错。"),
        help(ParamSpec::select("nameEncoding", "文件名字符集", &["UTF-8", "MacRoman"], "UTF-8"), "上游用 UTF-8；早期 Mac 文件名可选择 MacRoman。原始文件名字节保存在 JSON nameHex。"),
    ]), "binhex", "解析 HQX/RLE90，校验头、数据和资源三处 CRC；报告有界预览，完整字节可接文件产物存储或后续节点。编码文本最多 64 MiB，解压后两叉合计最多 16 MiB。", "(This file must be converted with BinHex 4.0)\n\n:#'CTE'8ZG(Kd!&4&@&4$9%C#!*!&\"3#3\"%5h5'9XE'r,eJ!!:", json!({}));
    dec.presentation.inline_params = vec!["strictCrc".into(), "nameEncoding".into()];
    dec.aliases.extend(["HQX 解码".into(), "Mac BinHex".into()]);
    reg.register(dec, Arc::new(|| Arc::new(Decoder)));
}
