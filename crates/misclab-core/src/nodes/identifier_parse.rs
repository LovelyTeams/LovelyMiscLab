//! UUID and MongoDB ObjectID inspection, with structured output for graph reuse.
use super::{codec_support::document_text, prelude::*, transport_codecs::err};
use chrono::{DateTime, Datelike, Timelike};
use serde_json::{json, Value};

fn iso(ms: i64, millis: bool) -> Result<String, CoreError> {
    let t = DateTime::from_timestamp_millis(ms).ok_or_else(|| err("时间戳超出日期范围"))?;
    let y = t.year();
    let year = if (0..=9999).contains(&y) {
        format!("{y:04}")
    } else {
        format!("{}{:#06}", if y < 0 { '-' } else { '+' }, y.unsigned_abs())
    };
    Ok(format!(
        "{year}-{:02}-{:02}T{:02}:{:02}:{:02}{}Z",
        t.month(),
        t.day(),
        t.hour(),
        t.minute(),
        t.second(),
        if millis {
            format!(".{:03}", t.timestamp_subsec_millis())
        } else {
            String::new()
        }
    ))
}
fn uuid_report(text: &str) -> Result<(String, Value), CoreError> {
    // Retain the pinned upstream's separator-tolerant parsing contract.
    let s: String = text
        .to_ascii_lowercase()
        .chars()
        .filter(char::is_ascii_hexdigit)
        .collect();
    if s.len() != 32 {
        return Err(err("UUID 需要 32 个 Hex 字符（可带连字符）"));
    }
    let b = hex::decode(&s).unwrap();
    let version = b[6] >> 4;
    let version_name = match version {
        1 => "v1 时间戳（时间 + MAC 地址）".into(),
        2 => "v2 DCE Security（POSIX UID/GID）".into(),
        3 => "v3 命名空间 + 名字 MD5 哈希".into(),
        4 => "v4 随机生成".into(),
        5 => "v5 命名空间 + 名字 SHA-1 哈希".into(),
        6 => "v6 时间戳（字段重排序，便于字典序）".into(),
        7 => "v7 时间戳（Unix 毫秒 + 随机）".into(),
        8 => "v8 自定义".into(),
        _ => format!("未知（v{version}）"),
    };
    let (variant, variant_name) = match b[8] >> 5 {
        0..=3 => ("ncs", "NCS backward compatibility（0xx）"),
        4..=5 => ("rfc4122", "RFC 4122 / ISO 11578（10x）"),
        6 => ("microsoft", "Microsoft Corporation（110x）"),
        _ => ("reserved", "保留（111x）"),
    };
    let canonical = format!(
        "{}-{}-{}-{}-{}",
        &s[..8],
        &s[8..12],
        &s[12..16],
        &s[16..20],
        &s[20..]
    );
    let mut data = json!({"uuid":canonical,"version":version,"variant":variant});
    let mut lines = vec![
        format!("UUID: {canonical}"),
        format!("版本: {version_name}"),
        format!("变体: {variant_name}"),
    ];
    if version == 1 {
        let ticks = ((u64::from(b[6] & 15) << 8 | u64::from(b[7])) << 48)
            | ((u64::from(b[4]) << 8 | u64::from(b[5])) << 32)
            | u64::from(u32::from_be_bytes(b[..4].try_into().unwrap()));
        let ms = (ticks / 10000) as i64 - 12_219_292_800_000;
        let time = iso(ms, true)?;
        let mac = b[10..]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(":");
        let sequence = (u16::from(b[8] & 63) << 8) | u16::from(b[9]);
        lines.extend([
            format!("时间戳: {time} (UTC)"),
            format!("时间戳(100ns): {ticks}"),
            format!("MAC 地址: {mac}"),
            format!("时钟序列: {sequence}"),
        ]);
        data["milliseconds"] = json!(ms.to_string());
        data["timestamp100ns"] = json!(ticks.to_string());
        data["time"] = json!(time);
        data["mac"] = json!(mac);
        data["clockSequence"] = json!(sequence);
    }
    if version == 7 {
        let ms = b[..6].iter().fold(0i64, |n, b| (n << 8) | i64::from(*b));
        let time = iso(ms, true)?;
        lines.extend([format!("时间戳: {time} (UTC)"), format!("Unix 毫秒: {ms}")]);
        data["milliseconds"] = json!(ms.to_string());
        data["time"] = json!(time);
    }
    if version == 3 || version == 5 {
        lines.push(format!(
            "说明: 命名空间 UUID + 名字的 {} 哈希，无法逆推原命名空间和名字",
            if version == 3 { "MD5" } else { "SHA-1" }
        ));
    }
    if version == 4 {
        lines.push("说明: 随机生成（除版本和变体位外，其余位全随机）".into());
    }
    Ok((lines.join("\n"), data))
}
fn object_id_report(text: &str) -> Result<(String, Value), CoreError> {
    let s = text.trim().to_ascii_lowercase();
    if s.is_empty() {
        return Ok(("（空输入）".into(), Value::Null));
    }
    if s.len() != 24 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(err("ObjectID 需要恰好 24 个 Hex 字符"));
    }
    let b = hex::decode(&s).unwrap();
    let timestamp = u32::from_be_bytes(b[..4].try_into().unwrap());
    let counter = (u32::from(b[9]) << 16) | (u32::from(b[10]) << 8) | u32::from(b[11]);
    let time = iso(i64::from(timestamp) * 1000, false)?;
    let random = &s[8..18];
    let report=format!("=== ObjectID 时间戳解析 ===\n输入: {s}\n字节 (12): {}\n--- 字段分解 ---\n时间戳 (Unix 秒): {timestamp}\n生成时间 (UTC): {time}\n随机值 (5 字节): {random}\n计数器 (3 字节): {counter}\n--- 位拆分（大端）---\nbytes 0-3  (timestamp): 0x{}\nbytes 4-8  (random)   : {random}\nbytes 9-11 (counter)  : 0x{}", b.iter().map(|b|format!("{b:02x}")).collect::<Vec<_>>().join(" "),&s[..8],&s[18..]);
    Ok((
        report,
        json!({"objectId":s,"timestamp":timestamp,"time":time,"random":random,"counter":counter}),
    ))
}
struct Identifier(bool);
impl Node for Identifier {
    fn run(&self, inputs: &PortMap, _: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let text = in_text(inputs, "text")?;
        if text.len() > 4096 {
            return Err(err("标识符输入最多 4096 字节"));
        }
        let (report, data) = if self.0 {
            uuid_report(text)?
        } else {
            object_id_report(text)?
        };
        let mut out = out_text(report);
        out.insert("data".into(), PortValue::Json(data));
        Ok(out)
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for (uuid,id,upstream,title,sample,description) in [
        (true,"uuid_parse","uuidParse","UUID 解析","f81d4fae-7dec-11d0-a765-00a0c91e6bf6","解析 UUID 版本与变体；v1 提取时间、MAC 和时钟序列，v7 提取 Unix 毫秒。v3/v5 提供命名空间哈希说明，不能逆推名字；兼容上游分隔符。"),
        (false,"object_id_time","objectIdTime","ObjectID 时间戳解析","507c7f79bcf86cd7994f6c0e","解析 MongoDB ObjectID 的大端 Unix 秒、5 字节随机值和 3 字节计数器。输入 24 个 Hex 字符，UTC 时间及结构化字段可接续。")
    ] {
        let mut d=document_text(desc(id,"numeric.time",title,CYAN,vec![t_in()],vec![t_out(),req("data","标识符明细",PortType::Json)],vec![]),upstream,description,sample,json!({}));
        d.aliases.push(if uuid {"GUID"} else {"MongoDB ObjectID"}.into());
        reg.register(d,Arc::new(move||Arc::new(Identifier(uuid))));
    }
}
