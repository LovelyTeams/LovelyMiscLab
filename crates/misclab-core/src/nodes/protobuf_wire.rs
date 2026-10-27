//! Protobuf wire inspection without a schema. Length fields remain heuristic.
use super::{
    numeric_support::{bad, number},
    prelude::*,
    serial_support::*,
};
use serde_json::{json, Value};
fn varint(r: &mut Reader) -> Result<u64, CoreError> {
    let start = r.pos;
    let value = (|| {
        let mut n = 0u64;
        for k in 0..10 {
            if r.pos == r.limit {
                return Err(bad("varint 读取越界"));
            }
            let b = r.byte()?;
            if k == 9 && b > 1 {
                return Err(bad("varint 超出 64 位或超过 10 字节"));
            }
            n |= ((b & 127) as u64) << (k * 7);
            if b & 128 == 0 {
                return Ok(n);
            }
        }
        Err(bad("varint 超过 10 字节（损坏）"))
    })();
    if value.is_err() {
        r.pos = start;
    }
    value
}
fn parse_fields(
    r: &mut Reader,
    depth: usize,
    ctx: &NodeCtx,
) -> Result<(Vec<Value>, Option<String>), CoreError> {
    let base = r.pos;
    let mut fields = Vec::new();
    while r.pos < r.limit {
        r.enter(depth, ctx)?;
        let start = r.pos;
        let field = (|| {
            let key = varint(r).map_err(|e| bad(&format!("key varint: {}", error_message(&e))))?;
            let wire = key & 7;
            let number = key >> 3;
            if number == 0 || number > 0x1fffffff {
                return Err(bad(&format!("非法 field number: {number}")));
            }
            let label = match wire {
                0 => "Varint",
                1 => "64-bit",
                2 => "Length-delimited",
                3 => "Start group",
                4 => "End group",
                5 => "32-bit",
                _ => return Err(bad(&format!("未知 wire type: {wire}"))),
            };
            let mut f = json!({"fieldNumber":number,"wireType":wire,"wireTypeStr":label,"offset":start-base,"absoluteOffset":start});
            match wire {
                0 => {
                    let n = varint(r)?;
                    // Zigzag is (n>>1) XOR -(n&1), evaluated over signed integers.
                    let zz = if n & 1 == 0 {
                        (n >> 1) as i128
                    } else {
                        -((n >> 1) as i128) - 1
                    };
                    f["value"] = json!(n.to_string());
                    f["valueHex"] = json!(format!("0x{n:X}"));
                    f["zigzag"] = json!(zz.to_string());
                    if (-1_000_000..=1_000_000).contains(&zz) {
                        f["hint"] = json!(format!("zigzag→{zz}"));
                    }
                }
                1 | 5 => {
                    let len = if wire == 1 { 8 } else { 4 };
                    if r.pos + len > r.limit {
                        return Err(bad(if wire == 1 {
                            "64-bit 数据不足"
                        } else {
                            "32-bit 数据不足"
                        }));
                    }
                    let bytes = r.take(len)?;
                    f["valueHex"] = json!(format!("0x{}", hex::encode_upper(bytes)));
                    if wire == 1 {
                        let b: [u8; 8] = bytes.try_into().unwrap();
                        f["value"] = json!(format!(
                            "int64(LE)={}, double(LE)={}",
                            i64::from_le_bytes(b),
                            float(f64::from_le_bytes(b))
                        ));
                        f["integer"] = json!(i64::from_le_bytes(b).to_string());
                        f["float"] = json!(float(f64::from_le_bytes(b)));
                    } else {
                        let b: [u8; 4] = bytes.try_into().unwrap();
                        f["value"] = json!(format!(
                            "int32(LE)={}, float(LE)={}",
                            i32::from_le_bytes(b),
                            float(f32::from_le_bytes(b) as f64)
                        ));
                        f["integer"] = json!(i32::from_le_bytes(b).to_string());
                        f["float"] = json!(float(f32::from_le_bytes(b) as f64));
                    }
                }
                2 => {
                    let raw_len = varint(r)?;
                    let len =
                        usize::try_from(raw_len).map_err(|_| bad("length-delimited 长度越界"))?;
                    let end = r
                        .pos
                        .checked_add(len)
                        .filter(|&n| n <= r.limit)
                        .ok_or_else(|| bad(&format!("length-delimited 长度 {len} 越界")))?;
                    let payload = &r.data[r.pos..end];
                    f["length"] = json!(len);
                    let text = std::str::from_utf8(payload).ok().filter(|s| {
                        let control = s
                            .chars()
                            .filter(|&c| c < ' ' && !matches!(c, '\n' | '\r' | '\t'))
                            .count();
                        !s.is_empty() && control as f64 / (s.encode_utf16().count() as f64) < 0.1
                    });
                    if let Some(s) = text {
                        f["value"] = json!(format!("string=\"{s}\""));
                        f["text"] = json!(s);
                        f["rawHex"] = json!(hex::encode(payload));
                    } else if depth < r.max_depth.min(32) && len > 0 {
                        let limit = r.limit;
                        r.limit = end;
                        let parsed = parse_fields(r, depth + 1, ctx);
                        let consumed = r.pos == end;
                        r.limit = limit;
                        match parsed {
                            Ok((nested, None)) if !nested.is_empty() && consumed => {
                                f["value"] = json!(format!("message({} fields)", nested.len()));
                                f["nested"] = json!(nested);
                            }
                            Err(CoreError::Cancelled) => return Err(CoreError::Cancelled),
                            _ => {
                                f["value"] =
                                    json!(format!("bytes[{len}]={}", hex_preview(payload, 32)));
                                f["rawHex"] = json!(hex::encode(payload));
                            }
                        }
                    } else {
                        f["value"] = json!(format!("bytes[{len}]={}", hex_preview(payload, 32)));
                        f["rawHex"] = json!(hex::encode(payload));
                    }
                    r.pos = end;
                }
                3 => f["value"] = json!("（start group，已弃用）"),
                4 => f["value"] = json!("（end group，已弃用）"),
                _ => unreachable!(),
            }
            f["end"] = json!(r.pos);
            Ok(f)
        })();
        match field {
            Ok(f) => fields.push(f),
            Err(CoreError::Cancelled) => return Err(CoreError::Cancelled),
            Err(e) => return Ok((fields, Some(error_message(&e)))),
        }
    }
    Ok((fields, None))
}
fn format_fields(fields: &[Value], indent: &str, lines: &mut Vec<String>) {
    for f in fields {
        let mut line = format!(
            "{indent}field {} ({}): {}",
            f["fieldNumber"],
            f["wireTypeStr"].as_str().unwrap(),
            f["value"].as_str().unwrap()
        );
        if let Some(s) = f["hint"].as_str() {
            line.push_str(&format!("  [{s}]"));
        }
        if let Some(s) = f["valueHex"].as_str() {
            line.push_str(&format!("  {s}"));
        }
        line.push_str(&format!("  @off={}", f["offset"]));
        lines.push(line);
        if let Some(n) = f["nested"].as_array() {
            format_fields(n, &format!("{indent}  "), lines);
        }
    }
}
struct Protobuf;
impl Node for Protobuf {
    fn run(&self, i: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let data = input(i, p)?;
        if data.is_empty() {
            return Ok(outputs(
                "请输入 protobuf 二进制数据（hex 或 base64）".into(),
                json!([]),
                0,
                &data,
                Some("空输入".into()),
            ));
        }
        let mut r = Reader::new(&data, number(p, "maxDepth", 64, 1, 128)? as usize);
        let (fields, error) = parse_fields(&mut r, 0, ctx)?;
        let mut lines = vec![
            "=== Protobuf Wire 解析 ===".into(),
            format!("输入: {} 字节", data.len()),
            String::new(),
        ];
        if fields.is_empty() {
            lines.push("未解析出任何字段（输入可能不是合法 protobuf wire 格式）".into());
        } else {
            format_fields(&fields, "", &mut lines);
            if let Some(s) = &error {
                lines.extend([String::new(), format!("⚠ 解析中断: {s}")]);
            }
            if r.pos < data.len() {
                lines.extend([
                    String::new(),
                    format!(
                        "⚠ 仅消费 {}/{} 字节，剩余 {} 字节未解析",
                        r.pos,
                        data.len(),
                        data.len() - r.pos
                    ),
                ]);
            }
        }
        ctx.cancel.check()?;
        Ok(outputs(
            lines.join("\n"),
            json!(fields),
            r.pos,
            &data,
            error,
        ))
    }
}
pub fn register(reg: &mut NodeRegistry) {
    reg.register(descriptor("protobuf_wire_parse","Protobuf Wire 结构解析","protobufParse","无 schema 检查全部 wire 类型；长度段按可读 UTF-8、嵌套消息、原始 Hex 顺序尝试。结构树保存偏移、完整字节和整数；组标记仅展示，不推断 schema。嵌套消息探测最多 32 层。","089601120548656c6c6f"),Arc::new(||Arc::new(Protobuf)));
}
