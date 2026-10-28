//! SSH public-key blob parsing and OpenSSH SHA256 fingerprints, without execution.
use super::{
    codec_support::{document_text, help},
    numeric_support::{number, space},
    prelude::*,
    transport_codecs::err,
};
use base64::{
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD},
    Engine as _,
};
use num_bigint::BigUint;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const MAX_BLOB: usize = 256 * 1024;
struct Reader<'a> {
    blob: &'a [u8],
    pos: usize,
}
impl<'a> Reader<'a> {
    fn field(&mut self) -> Result<&'a [u8], CoreError> {
        let header = self
            .blob
            .get(self.pos..self.pos + 4)
            .ok_or_else(|| err("SSH 字段缺少 4 字节长度"))?;
        let len = u32::from_be_bytes(header.try_into().unwrap()) as usize;
        self.pos += 4;
        let end = self
            .pos
            .checked_add(len)
            .filter(|&end| end <= self.blob.len())
            .ok_or_else(|| err("SSH 字段长度超出剩余 blob"))?;
        let value = &self.blob[self.pos..end];
        self.pos = end;
        Ok(value)
    }
    fn text(&mut self) -> Result<String, CoreError> {
        std::str::from_utf8(self.field()?)
            .map(str::to_owned)
            .map_err(|_| err("SSH 类型/曲线/应用名不是 UTF-8"))
    }
    fn positive(&mut self) -> Result<BigUint, CoreError> {
        let b = self.field()?;
        if b.len() > 8192 {
            return Err(err("单个 SSH mpint 最多 8192 字节"));
        }
        if b.first().is_some_and(|b| b & 0x80 != 0) {
            return Err(err("SSH 公钥 mpint 不能为负数；高位为 1 时需要前导 00"));
        }
        Ok(BigUint::from_bytes_be(b))
    }
}
fn b64(text: &str) -> Result<Vec<u8>, CoreError> {
    let text = text.replace('-', "+").replace('_', "/");
    let engine = if text.ends_with('=') {
        STANDARD
    } else {
        STANDARD_NO_PAD
    };
    engine
        .decode(text)
        .map_err(|_| err("SSH blob Base64 无效或截断"))
}
fn token(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && s.bytes().all(|b| (33..=126).contains(&b))
}
fn short(s: &str) -> String {
    let len = s.encode_utf16().count();
    if len <= 80 {
        return s.into();
    }
    let mut count = 0;
    let end = s
        .char_indices()
        .find_map(|(i, c)| {
            count += c.len_utf16();
            (count > 80).then_some(i)
        })
        .unwrap_or(s.len());
    format!("{}…({len} 字符)", &s[..end])
}
struct Ssh;
impl Node for Ssh {
    fn run(&self, inputs: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let selection = number(p, "line", 1, 1, 10000)? as usize;
        let (blob, outer, comment) = match inputs.get("data") {
            Some(PortValue::Bytes(b)) => {
                if b.len() > MAX_BLOB {
                    return Err(err("SSH blob 最多 256 KiB"));
                }
                (b.to_vec(), None, String::new())
            }
            Some(PortValue::Text(text)) => {
                if text.len() > 512 * 1024 {
                    return Err(err("SSH 文本最多 512 KiB"));
                }
                let line = text
                    .lines()
                    .map(|s| s.trim_matches(space))
                    .filter(|s| !s.is_empty() && !s.starts_with('#'))
                    .nth(selection - 1)
                    .ok_or_else(|| err("所选公钥行不存在（已跳过空行和 # 注释）"))?;
                let (first, rest) = line.find(space).map_or((line, ""), |pos| {
                    (&line[..pos], line[pos..].trim_start_matches(space))
                });
                if rest.is_empty() {
                    (b64(first)?, None, String::new())
                } else {
                    let (encoded, comment) = rest.find(space).map_or((rest, ""), |pos| {
                        (&rest[..pos], rest[pos..].trim_start_matches(space))
                    });
                    if !token(first) {
                        return Err(err("SSH 公钥类型标识无效"));
                    }
                    (b64(encoded)?, Some(first.to_owned()), comment.to_owned())
                }
            }
            _ => return Err(err("SSH 公钥需要 Text 或原始 Bytes blob")),
        };
        if blob.len() > MAX_BLOB {
            return Err(err("SSH blob 最多 256 KiB"));
        }
        let mut reader = Reader {
            blob: &blob,
            pos: 0,
        };
        let key_type = reader.text()?;
        if !token(&key_type) {
            return Err(err("SSH blob 内部类型标识无效"));
        }
        if outer.as_ref().is_some_and(|s| *s != key_type) {
            return Err(err("SSH 外部类型与 blob 内部类型不一致"));
        }
        let mut fields = serde_json::Map::new();
        let mut public_key = None;
        let mut warnings = Vec::new();
        ctx.progress(0.2);
        ctx.cancel.check()?;
        match key_type.as_str() {
            "ssh-rsa" => {
                let e = reader.positive()?;
                let n = reader.positive()?;
                fields.insert("e".into(), json!(e.to_string()));
                fields.insert("n".into(), json!(n.to_string()));
                fields.insert("modulusBits".into(), json!(n.bits().max(1)));
            }
            "ssh-dss" => {
                for field in ["p", "q", "g", "y"] {
                    ctx.cancel.check()?;
                    fields.insert(field.into(), json!(reader.positive()?.to_string()));
                }
            }
            "ssh-ed25519" | "sk-ssh-ed25519@openssh.com" => {
                let key = reader.field()?;
                if key.len() != 32 {
                    return Err(err("Ed25519 公钥必须为 32 字节"));
                }
                fields.insert("publicKey".into(), json!(hex::encode(key)));
                if key_type == "ssh-ed25519" {
                    fields.insert("publicKeyLen".into(), json!(key.len()));
                } else {
                    fields.insert("application".into(), json!(reader.text()?));
                }
                public_key = Some(key.to_vec());
            }
            t if t.starts_with("ecdsa-sha2-") => {
                let curve = reader.text()?;
                if t != format!("ecdsa-sha2-{curve}") {
                    return Err(err("ECDSA 类型名称与 curve 字段不一致"));
                }
                let point = reader.field()?;
                let coordinate = match curve.as_str() {
                    "nistp256" => Some(32),
                    "nistp384" => Some(48),
                    "nistp521" => Some(66),
                    _ => None,
                };
                if let Some(n) = coordinate {
                    let valid = match point.first() {
                        Some(4) => point.len() == 1 + 2 * n,
                        Some(2 | 3) => point.len() == 1 + n,
                        _ => false,
                    };
                    if !valid {
                        return Err(err("ECDSA SEC1 点编码前缀或长度无效"));
                    }
                } else {
                    warnings.push("未知 ECDSA 曲线，保留点字节，未验证坐标长度");
                }
                fields.insert("curve".into(), json!(curve));
                fields.insert("publicKeyPoint".into(), json!(hex::encode(point)));
                fields.insert("publicKeyLen".into(), json!(point.len()));
                public_key = Some(point.to_vec());
            }
            _ => {
                fields.insert("raw".into(), json!(hex::encode(&blob[reader.pos..])));
                reader.pos = blob.len();
                warnings.push("未知密钥类型，剩余字节仅作原始字段展示");
            }
        }
        if reader.pos != blob.len() {
            return Err(err("SSH 已知类型字段后存在多余字节"));
        }
        ctx.progress(0.8);
        ctx.cancel.check()?;
        let fingerprint = format!("SHA256:{}", STANDARD_NO_PAD.encode(Sha256::digest(&blob)));
        let mut text = format!("=== SSH 公钥解析 ===\n类型: {key_type}\n");
        if !comment.is_empty() {
            text.push_str(&format!("注释: {}\n", short(&comment)));
        }
        text.push_str(&format!(
            "blob: {} 字节\nSHA256 指纹: {fingerprint}\n\n字段:",
            blob.len()
        ));
        for (name, value) in &fields {
            let value = value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string());
            text.push_str(&format!("\n  {name}: {}", short(&value)));
        }
        let metadata = json!({"keyType":key_type,"comment":comment,"blobLength":blob.len(),"fingerprint":fingerprint,"fields":fields,"warnings":warnings,"validation":"wire-structure-only"});
        let mut out: PortMap = [
            ("text".into(), PortValue::Text(text)),
            ("fingerprint".into(), PortValue::Text(fingerprint)),
            ("keyType".into(), PortValue::Text(key_type)),
            ("bytes".into(), PortValue::Bytes(blob.into())),
            ("json".into(), PortValue::Json(metadata)),
        ]
        .into();
        if let Some(key) = public_key {
            out.insert("publicKey".into(), PortValue::Bytes(key.into()));
        }
        ctx.progress(1.0);
        ctx.cancel.check()?;
        Ok(out)
    }
}
pub fn register(reg: &mut NodeRegistry) {
    let mut blob = Vec::new();
    blob.extend_from_slice(&11u32.to_be_bytes());
    blob.extend_from_slice(b"ssh-ed25519");
    blob.extend_from_slice(&32u32.to_be_bytes());
    blob.extend_from_slice(&[1; 32]);
    let sample = format!("ssh-ed25519 {} demo", STANDARD.encode(blob));
    let mut d=document_text(desc("ssh_public_key_parse","data.structure","SSH 公钥解析",TEAL,vec![req("data","公钥文本或原始 blob",PortType::Any)],vec![t_out(),req("fingerprint","OpenSSH SHA256 指纹",PortType::Text),req("keyType","密钥类型",PortType::Text),req("bytes","完整公钥 blob",PortType::Bytes),opt("publicKey","Ed25519/EC 公钥字节",PortType::Bytes),req("json","完整字段",PortType::Json)],vec![help(ParamSpec::number("line","第几个公钥行",1.,10000.,1.,1.),"文本中跳过空行和 # 注释后，从 1 开始选择公钥。每行使用 type base64 comment 或单个 Base64；Bytes 输入直接解析，不使用行选择。")]),"sshPubkeyParse","解析 RSA、DSA、Ed25519、ECDSA 和 OpenSSH 安全密钥字段，输出 SHA256 指纹与完整 blob。校验字段结构、长度和类型一致性，不验证签名、密钥数学有效性或可信身份；不读取远程主机。最多 256 KiB blob，单个 mpint 最多 8192 字节。",&sample,json!({}));
    d.aliases.extend([
        "OpenSSH".into(),
        "authorized_keys".into(),
        "SSH fingerprint".into(),
    ]);
    reg.register(d, Arc::new(|| Arc::new(Ssh)));
}
