//! Offline Bitcoin address inspection and Ethereum EIP-55 checksums.
use super::{
    basex, bech32_codec as bech,
    codec_support::{document_text, help},
    numeric_support::space,
    prelude::*,
    transport_codecs::err,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sha3::Keccak256;

fn flag(p: &Value, key: &str) -> Result<bool, CoreError> {
    match p.get(key) {
        None => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        _ => Err(err(format!("{key} 必须是布尔值"))),
    }
}
struct Inspection {
    text: String,
    address: String,
    payload: Vec<u8>,
    script: Option<Vec<u8>>,
    valid: bool,
    metadata: Value,
}
fn segwit(address: &str) -> Result<Inspection, CoreError> {
    if !(8..=90).contains(&address.len()) || !address.bytes().all(|b| (33..=126).contains(&b)) {
        return Err(err("SegWit 地址需要 8–90 个可打印 ASCII 字符"));
    }
    if address.bytes().any(|b| b.is_ascii_lowercase())
        && address.bytes().any(|b| b.is_ascii_uppercase())
    {
        return Err(err("Bech32/Bech32m 不允许混合大小写"));
    }
    let lower = address.to_ascii_lowercase();
    let pos = lower.rfind('1').ok_or_else(|| err("缺少 Bech32 分隔符"))?;
    if pos == 0 || pos + 7 >= lower.len() {
        return Err(err("SegWit 地址缺少 witness 数据或校验段"));
    }
    let hrp = &lower[..pos];
    let network = match hrp {
        "bc" => "mainnet",
        "tb" => "testnet",
        "bcrt" => "regtest",
        _ => return Err(err("不是比特币网络的 HRP")),
    };
    let words = lower[pos + 1..]
        .bytes()
        .map(|b| {
            bech::TABLE
                .iter()
                .position(|&v| b == v)
                .map(|v| v as u8)
                .ok_or_else(|| err("Bech32 字符无效"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let version = words[0];
    if version > 16 {
        return Err(err("Witness 版本必须为 0–16"));
    }
    let expected = if version == 0 { 1 } else { 0x2bc830a3 };
    if bech::polymod(
        bech::hrp_expanded(hrp)
            .into_iter()
            .chain(words.iter().copied()),
    ) != expected
    {
        return Err(err("SegWit 校验失败：v0 需要 Bech32，v1–16 需要 Bech32m"));
    }
    let payload = bech::bits(&words[1..words.len() - 6], 5, 8, false)?;
    if !(2..=40).contains(&payload.len()) || version == 0 && ![20, 32].contains(&payload.len()) {
        return Err(err("Witness 程序需要 2–40 字节；v0 仅允许 20 或 32 字节"));
    }
    let (kind, label) = match (version, payload.len()) {
        (0, 20) => ("P2WPKH", "P2WPKH（原生 SegWit v0 付公钥哈希）"),
        (0, 32) => ("P2WSH", "P2WSH（原生 SegWit v0 付脚本哈希）"),
        (1, 32) => ("P2TR", "P2TR（Taproot，SegWit v1）"),
        _ => ("witness", "未知用途的有效 witness 程序"),
    };
    let encoding = if version == 0 { "Bech32" } else { "Bech32m" };
    let shown_encoding = if kind == "P2TR" {
        "Bech32m (Taproot)"
    } else {
        encoding
    };
    let mut script = vec![
        if version == 0 { 0 } else { 0x50 + version },
        payload.len() as u8,
    ];
    script.extend_from_slice(&payload);
    let text = format!("=== 比特币地址识别（Bech32 SegWit）===\n地址: {address}\n编码: {shown_encoding}\nHRP: {hrp}  →  网络: {network}\nWitness 版本: {version}\nWitness 程序: {} ({} 字节)\n类型: {label}\n校验: ✓ Bech32 校验通过",hex::encode(&payload),payload.len());
    let metadata = json!({"raw":address,"address":lower,"encoding":encoding,"type":kind,"network":network,"hrp":hrp,"witnessVersion":version,"payloadHex":hex::encode(&payload),"length":payload.len(),"checksumValid":true,"valid":true,"scriptPubKey":hex::encode(&script)});
    Ok(Inspection {
        text,
        address: lower,
        payload,
        script: Some(script),
        valid: true,
        metadata,
    })
}
fn bitcoin(address: &str) -> Result<Inspection, CoreError> {
    if address.is_empty() || address.len() > 90 {
        return Err(err("比特币地址需要 1–90 个字符"));
    }
    let lower = address.to_ascii_lowercase();
    if ["bc1", "tb1", "bcrt1"].iter().any(|p| lower.starts_with(p)) {
        return segwit(address);
    }
    let raw = basex::radix_decode(
        address,
        &basex::B58_BITCOIN.chars().collect::<Vec<_>>(),
        true,
        false,
    )?;
    if raw.len() < 5 {
        return Err(err("Base58 解码后长度不足"));
    }
    let version = raw[0];
    let payload = raw[1..raw.len() - 4].to_vec();
    let checksum = &raw[raw.len() - 4..];
    let hash = Sha256::digest(Sha256::digest(&raw[..raw.len() - 4]));
    let checksum_valid = checksum == &hash[..4];
    let (kind, network, description) = match version {
        0 => ("P2PKH", "mainnet", "传统付公钥哈希地址（1 开头）"),
        0x6f => ("P2PKH", "testnet", "测试网 P2PKH（m/n 开头）"),
        5 => ("P2SH", "mainnet", "付脚本哈希地址（3 开头）"),
        0xc4 => ("P2SH", "testnet", "测试网 P2SH（2 开头）"),
        _ => ("unknown", "unknown", ""),
    };
    let valid = checksum_valid && kind != "unknown" && payload.len() == 20;
    let mut text = format!("=== 比特币地址识别（Base58Check）===\n地址: {address}\n编码: Base58Check\n版本字节: 0x{version:02x}\n");
    if kind == "unknown" {
        text.push_str("类型: 未知版本字节\n");
    } else {
        text.push_str(&format!(
            "类型: {kind}  ·  网络: {network}\n说明: {description}\n"
        ));
    }
    text.push_str(&format!(
        "哈希: {} ({} 字节)\n校验和: {}  →  计算: {}  {}",
        hex::encode(&payload),
        payload.len(),
        hex::encode(checksum),
        hex::encode(&hash[..4]),
        if checksum_valid {
            "✓"
        } else {
            "✗ 不匹配"
        }
    ));
    if !checksum_valid {
        text.push_str("\n⚠ 校验失败：地址可能损坏或非比特币地址");
    }
    if payload.len() != 20 {
        text.push_str("\n提示：P2PKH/P2SH 哈希应为 20 字节，当前不是有效地址");
    }
    if kind == "unknown" {
        text.push_str("\n提示：校验和正确也不能确认此版本属于比特币地址");
    }
    let script = if valid {
        let mut s = if kind == "P2PKH" {
            vec![0x76, 0xa9, 0x14]
        } else {
            vec![0xa9, 0x14]
        };
        s.extend_from_slice(&payload);
        s.extend_from_slice(if kind == "P2PKH" {
            &[0x88, 0xac]
        } else {
            &[0x87]
        });
        Some(s)
    } else {
        None
    };
    let metadata = json!({"raw":address,"address":address,"encoding":"Base58Check","version":version,"type":kind,"network":network,"payloadHex":hex::encode(&payload),"length":payload.len(),"checksum":hex::encode(checksum),"calculatedChecksum":hex::encode(&hash[..4]),"checksumValid":checksum_valid,"valid":valid,"scriptPubKey":script.as_ref().map(hex::encode)});
    Ok(Inspection {
        text,
        address: address.into(),
        payload,
        script,
        valid,
        metadata,
    })
}
fn ethereum(raw: &str) -> Result<Inspection, CoreError> {
    let address = raw
        .strip_prefix("0x")
        .or_else(|| raw.strip_prefix("0X"))
        .unwrap_or(raw);
    if address.len() != 40 || !address.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(err("以太坊地址需要 40 位 Hex，可带 0x 前缀"));
    }
    let lower = address.to_ascii_lowercase();
    let hash = Keccak256::digest(lower.as_bytes());
    let mut canonical = String::from("0x");
    for (i, b) in lower.bytes().enumerate() {
        let nibble = if i % 2 == 0 {
            hash[i / 2] >> 4
        } else {
            hash[i / 2] & 15
        };
        canonical.push(if nibble >= 8 {
            b.to_ascii_uppercase()
        } else {
            b
        } as char);
    }
    let mixed = address.bytes().any(|b| b.is_ascii_lowercase())
        && address.bytes().any(|b| b.is_ascii_uppercase());
    let checksum_matches = address == &canonical[2..];
    let valid = !mixed || checksum_matches;
    let text = format!("=== 以太坊地址识别（EIP-55）===\n地址: {raw}\n长度: 40 位十六进制（20 字节）\nEIP-55 校验地址: {canonical}\n输入大小写: {}\n校验: {}",if mixed {"混合大小写"}else{"纯小写/大写（无校验信息）"},if checksum_matches {"✓ EIP-55 混合大小写校验通过"}else if mixed {"✗ EIP-55 校验失败（大小写与 Keccak-256 哈希不符）"}else{"— 纯小写/大写地址无 EIP-55 校验信息（建议用上面的校验地址）"});
    let payload = hex::decode(&lower).map_err(|_| err("地址 Hex 无效"))?;
    let metadata = json!({"raw":raw,"address":canonical,"lowercase":format!("0x{lower}"),"encoding":"EIP-55","mixedCase":mixed,"checksumMatches":checksum_matches,"valid":valid,"payloadHex":lower,"length":20});
    Ok(Inspection {
        text,
        address: canonical,
        payload,
        script: None,
        valid,
        metadata,
    })
}
struct Address(bool);
impl Node for Address {
    fn run(&self, inputs: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let input = in_text(inputs, "text")?;
        if input.len() > 4096 {
            return Err(err("地址输入最多 4096 字节"));
        }
        let input = input.trim_matches(space);
        let strict = flag(
            p,
            if self.0 {
                "requireChecksum"
            } else {
                "requireValid"
            },
        )?;
        let result = if self.0 {
            ethereum(input)?
        } else {
            bitcoin(input)?
        };
        if strict
            && (if self.0 {
                result.metadata["checksumMatches"] != true
            } else {
                !result.valid
            })
        {
            return Err(err("地址不满足所选的严格校验要求"));
        }
        ctx.progress(0.8);
        ctx.cancel.check()?;
        let mut out: PortMap = [
            ("text".into(), PortValue::Text(result.text)),
            ("address".into(), PortValue::Text(result.address)),
            ("bytes".into(), PortValue::Bytes(result.payload.into())),
            ("valid".into(), PortValue::Bool(result.valid)),
            ("json".into(), PortValue::Json(result.metadata)),
        ]
        .into();
        if let Some(script) = result.script {
            out.insert("script".into(), PortValue::Bytes(script.into()));
        }
        ctx.progress(1.0);
        ctx.cancel.check()?;
        Ok(out)
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for (eth, id, upstream, name, sample) in [
        (
            false,
            "bitcoin_address_inspect",
            "btcAddressIdent",
            "比特币地址识别",
            "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",
        ),
        (
            true,
            "ethereum_address_inspect",
            "ethAddressIdent",
            "以太坊地址识别",
            "0x52908400098527886E0F7030069857D2E4169EE7",
        ),
    ] {
        let mut outputs = vec![
            t_out(),
            req("address", "规范地址", PortType::Text),
            req("bytes", "哈希或 witness 字节", PortType::Bytes),
            req("valid", "输入地址有效", PortType::Bool),
            req("json", "结构与校验信息", PortType::Json),
        ];
        if !eth {
            outputs.push(opt("script", "有效地址的 scriptPubKey", PortType::Bytes));
        }
        let param = if eth {
            help(ParamSpec::toggle("requireChecksum","必须匹配 EIP-55",false),"默认报告错误大小写但保留字节供分析；开启后输入必须精确匹配 EIP-55 大小写，可省略 0x 前缀。")
        } else {
            help(ParamSpec::toggle("requireValid","无效地址时报错",false),"默认报告 Base58Check 损坏或未知版本；开启后校验、网络版本与 20 字节长度都须有效。非法 SegWit 结构/校验始终报错。")
        };
        let mut d = document_text(
            desc(
                id,
                "numeric.address",
                name,
                CYAN,
                vec![t_in()],
                outputs,
                vec![param],
            ),
            upstream,
            if eth {
                "离线分析 20 字节 Ethereum 地址，计算 Keccak-256 EIP-55 大小写并输出规范地址和完整字节；不推断链、账户状态或余额。"
            } else {
                "离线识别 P2PKH/P2SH/P2WPKH/P2WSH/P2TR、主网/测试网/regtest 与未知 witness 用途，按 BIP-173/350 校验；有效地址另输出 scriptPubKey。"
            },
            sample,
            json!({}),
        );
        d.aliases.extend(if eth {
            vec!["Ethereum".into(), "EIP-55".into(), "ERC-55".into()]
        } else {
            vec![
                "Bitcoin".into(),
                "SegWit".into(),
                "Taproot".into(),
                "Bech32m".into(),
            ]
        });
        reg.register(d, Arc::new(move || Arc::new(Address(eth))));
    }
}
