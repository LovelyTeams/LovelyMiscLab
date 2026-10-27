//! DXBase64 framing (EBCTFCodeBox ea2bc36 / 风之暇想), implemented in Rust.
use super::{
    codec_support::{document_text, help},
    prelude::*,
    serial_support,
    transport_codecs::err,
};
use base64::Engine as _;
use flate2::{write::DeflateEncoder, Compression, Decompress, FlushDecompress, Status};
use serde_json::json;
use std::io::Write;

const LIMIT: usize = 8 * 1024 * 1024;

fn crc(data: &[u8], ctx: &NodeCtx) -> Result<u16, CoreError> {
    // Deliberately follows DX's right-shifting 0x1021 algorithm, not CCITT-FALSE.
    let mut crc = 0xffffu16;
    for (i, &byte) in data.iter().enumerate() {
        if i.is_multiple_of(8192) {
            ctx.cancel.check()?;
        }
        crc ^= u16::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0x1021 } else { 0 };
        }
    }
    Ok(crc)
}

struct Codec(bool);
impl Node for Codec {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let text = if self.0 {
            let text = in_text(inputs, "text")?;
            if text.len() > LIMIT * 2 {
                return Err(err("DXBase64 输入最多 16 MiB"));
            }
            if text.trim().is_empty() {
                return Ok(super::basex::decoded(vec![]));
            }
            let clean: String = text.chars().filter(|c| !c.is_whitespace()).collect();
            if clean
                .bytes()
                .any(|b| !b.is_ascii_alphanumeric() && !b"+/=".contains(&b))
            {
                return Err(err("DXBase64 需要标准 Base64 数据帧"));
            }
            let frame = super::transport_codecs::b64decode(&clean)?;
            if frame.len() < 4 {
                return Err(err("DXBase64 数据帧过短"));
            }
            if crc(&frame[4..], ctx)? != u16::from_be_bytes([frame[0], frame[1]]) {
                return Err(err("DXBase64 数据校验失败（CRC16 不匹配）"));
            }
            let compressed: Vec<u8> = frame[4..]
                .iter()
                .enumerate()
                .map(|(i, b)| b ^ frame[2 + i % 2])
                .collect();
            let mut decoder = Decompress::new(false);
            let mut decoded = Vec::new();
            loop {
                ctx.progress(0.25 + (decoded.len() as f32 / LIMIT as f32) * 0.7);
                ctx.cancel.check()?;
                let mut chunk = [0u8; 8192];
                let before_in = decoder.total_in();
                let before_out = decoder.total_out();
                let status = decoder
                    .decompress(
                        &compressed[before_in as usize..],
                        &mut chunk,
                        FlushDecompress::None,
                    )
                    .map_err(|e| err(format!("DXBase64 deflate 无效：{e}")))?;
                let n = (decoder.total_out() - before_out) as usize;
                if decoded.len() + n > LIMIT {
                    return Err(err("DXBase64 解压结果超过 8 MiB"));
                }
                decoded.extend_from_slice(&chunk[..n]);
                if status == Status::StreamEnd {
                    if decoder.total_in() as usize != compressed.len() {
                        return Err(err("DXBase64 deflate 后有多余数据"));
                    }
                    break;
                }
                if n == 0 && before_in == decoder.total_in() {
                    return Err(err("DXBase64 deflate 数据截断"));
                }
            }
            let mut out = super::basex::decoded(decoded.clone());
            out.insert(
                "text".into(),
                PortValue::Text(serial_support::utf8(&decoded)),
            );
            return Ok(out);
        } else {
            let bytes = in_bytes(inputs, "data")?;
            if bytes.len() > LIMIT {
                return Err(err("DXBase64 输入最多 8 MiB"));
            }
            // Upstream distinguishes empty text from an explicitly supplied empty byte buffer.
            if matches!(inputs.get("data"), Some(PortValue::Text(s)) if s.is_empty()) {
                return Ok(out_text(String::new()));
            }
            let mut salt = [0u8; 2];
            let fixed = pstr(params, "salt", "").trim();
            if fixed.is_empty() {
                getrandom::getrandom(&mut salt).map_err(|e| err(format!("随机盐生成失败：{e}")))?;
            } else {
                let raw = hex::decode(fixed).map_err(|_| err("固定盐需要 4 个 Hex 字符"))?;
                if raw.len() != 2 {
                    return Err(err("固定盐必须是 2 字节"));
                }
                salt.copy_from_slice(&raw);
            }
            let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
            for (i, chunk) in bytes.chunks(8192).enumerate() {
                ctx.progress((i * 8192) as f32 / bytes.len().max(1) as f32 * 0.8);
                ctx.cancel.check()?;
                encoder.write_all(chunk).map_err(|e| err(e.to_string()))?;
            }
            let compressed = encoder.finish().map_err(|e| err(e.to_string()))?;
            let xor: Vec<u8> = compressed
                .iter()
                .enumerate()
                .map(|(i, b)| b ^ salt[i % 2])
                .collect();
            let mut frame = crc(&xor, ctx)?.to_be_bytes().to_vec();
            frame.extend(salt);
            frame.extend(xor);
            base64::engine::general_purpose::STANDARD.encode(frame)
        };
        ctx.cancel.check()?;
        Ok(out_text(text))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    for decode in [false, true] {
        let id = if decode {
            "dx_base64_decode"
        } else {
            "dx_base64_encode"
        };
        let params = if decode {
            vec![]
        } else {
            vec![help(
                ParamSpec::text("salt", "固定盐（Hex）", "", false),
                "留空使用 2 字节随机盐；填写 4 个 Hex 字符可重复测试。盐不是密钥。",
            )]
        };
        let mut d = document_text(desc(id, "encoding.base", if decode {"DXBase64 解码"} else {"DXBase64 编码"}, GREEN,
            if decode {vec![t_in()]} else {vec![req("data", "文本或原始字节", PortType::Any)]},
            if decode {vec![t_out(), req("bytes", "完整字节", PortType::Bytes)]} else {vec![t_out()]}, params),
            "dxBase64", "raw deflate + 2 字节盐循环 XOR + DX CRC16 + Base64。支持原始字节；输入/解压结果最多 8 MiB，解码保留完整 bytes，文本仅为预览。",
            if decode {"Bl23oUTpemh+dubZBcJaEt5MqEd4zeWltw=="} else {"Hello, 中文🙂!"},
            if decode {json!({})} else {json!({"salt":"1234"})});
        d.aliases.extend(["DXBase64", "风之暇想"].map(String::from));
        d.cost = Cost::Medium;
        reg.register(d, Arc::new(move || Arc::new(Codec(decode))));
    }
}
