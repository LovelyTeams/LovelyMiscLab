//! Java serialization header/marker inspection, never object deserialization.
use super::{numeric_support::bad, prelude::*, serial_support::*};
use serde_json::{json, Value};
fn modified_utf8(bytes: &[u8]) -> Result<(String, Vec<u16>), CoreError> {
    let (mut units, mut pos) = (Vec::new(), 0);
    while pos < bytes.len() {
        let b = bytes[pos];
        pos += 1;
        let n = if (1..128).contains(&b) {
            b as u16
        } else if b & 0xe0 == 0xc0 {
            let c = *bytes
                .get(pos)
                .ok_or_else(|| bad("Java modified UTF-8 截断"))?;
            pos += 1;
            if c & 0xc0 != 0x80 {
                return Err(bad("Java modified UTF-8 无效尾字节"));
            }
            let n = ((b & 31) as u16) << 6 | (c & 63) as u16;
            if n < 128 && !(b == 0xc0 && c == 0x80) {
                return Err(bad("Java modified UTF-8 非法过长编码"));
            }
            n
        } else if b & 0xf0 == 0xe0 {
            let tail = bytes
                .get(pos..pos + 2)
                .ok_or_else(|| bad("Java modified UTF-8 截断"))?;
            pos += 2;
            if tail.iter().any(|&b| b & 0xc0 != 0x80) {
                return Err(bad("Java modified UTF-8 无效尾字节"));
            }
            let n = ((b & 15) as u16) << 12 | ((tail[0] & 63) as u16) << 6 | (tail[1] & 63) as u16;
            if n < 2048 {
                return Err(bad("Java modified UTF-8 非法过长编码"));
            }
            n
        } else {
            return Err(bad("Java modified UTF-8 不接受零字节或四字节 UTF-8"));
        };
        units.push(n);
    }
    Ok((String::from_utf16_lossy(&units), units))
}
fn java_string(r: &mut Reader, width: usize, standard_utf8: bool) -> Result<Value, CoreError> {
    let len = r.len(width, false)?;
    let raw = r.take(len)?;
    let (text, units) = if standard_utf8 {
        let s = utf8(raw);
        let u = s.encode_utf16().collect();
        (s, u)
    } else {
        modified_utf8(raw)?
    };
    Ok(json!({"text":text,"rawHex":hex::encode(raw),"utf16":units,"length":len}))
}
fn tc_name(tc: u8) -> String {
    match tc {
        0x70 => "TC_NULL",
        0x71 => "TC_REFERENCE",
        0x72 => "TC_CLASSDESC",
        0x73 => "TC_OBJECT",
        0x74 => "TC_STRING",
        0x75 => "TC_ARRAY",
        0x76 => "TC_CLASS",
        0x77 => "TC_BLOCKDATA",
        0x78 => "TC_ENDBLOCKDATA",
        0x79 => "TC_RESET",
        0x7a => "TC_BLOCKDATALONG",
        0x7b => "TC_EXCEPTION",
        0x7c => "TC_LONGSTRING",
        0x7d => "TC_PROXYCLASSDESC",
        0x7e => "TC_ENUM",
        _ => return format!("0x{tc:02x}"),
    }
    .into()
}
fn flags(raw: u8, legacy: bool) -> Vec<&'static str> {
    let labels = if legacy {
        [
            "SC_WRITE_METHOD",
            "SC_BLOCK_DATA",
            "SC_SERIALIZABLE",
            "SC_EXTERNALIZABLE",
            "SC_ENUM",
        ]
    } else {
        [
            "SC_WRITE_METHOD",
            "SC_SERIALIZABLE",
            "SC_EXTERNALIZABLE",
            "SC_BLOCK_DATA",
            "SC_ENUM",
        ]
    };
    labels
        .into_iter()
        .enumerate()
        .filter_map(|(i, s)| if raw & (1 << i) != 0 { Some(s) } else { None })
        .collect()
}
struct Java;
impl Node for Java {
    fn run(&self, i: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let data = input(i, p)?;
        if data.is_empty() {
            return Ok(outputs(
                "请输入 Java 序列化二进制数据（hex 或 base64）".into(),
                Value::Null,
                0,
                &data,
                Some("空输入".into()),
            ));
        }
        let mut lines = vec![
            "=== Java 序列化识别 ===".into(),
            format!("输入: {} 字节", data.len()),
            String::new(),
        ];
        if data.len() < 4 {
            let s = "⚠ 输入不足 4 字节，无法识别 Java magic";
            lines.push(s.into());
            return Ok(outputs(
                lines.join("\n"),
                Value::Null,
                0,
                &data,
                Some(s.into()),
            ));
        }
        let magic = u16::from_be_bytes(data[..2].try_into().unwrap());
        let version = u16::from_be_bytes(data[2..4].try_into().unwrap());
        let mut tree =
            json!({"magic":format!("{magic:04X}"),"version":version,"scope":"marker-scan"});
        if magic != 0xaced {
            let s = format!("⚠ magic 不匹配: 期望 0xACED，得到 0x{magic:X}");
            lines.push(s.clone());
            lines.push("  （Java 序列化流必须以 0xACED 0x0005 开头）".into());
            return Ok(outputs(lines.join("\n"), tree, 4, &data, Some(s)));
        }
        lines.push("✓ Magic: 0xACED (Java Object Serialization)".into());
        lines.push(format!(
            "✓ Version: 0x{version:X}{}",
            if version == 5 {
                " (正确)"
            } else {
                " (非标准，期望 0x0005)"
            }
        ));
        lines.push(String::new());
        let legacy = pbool(p, "legacyReport", false);
        let encoding = pstr(p, "stringEncoding", "modified-utf8");
        if !["modified-utf8", "utf8"].contains(&encoding) {
            return Err(bad("未知 Java 字符串编码"));
        }
        let mut r = Reader::new(&data, 64);
        r.pos = 4;
        let (mut markers, mut block_lines, mut count, mut unsupported, mut error) =
            (Vec::new(), Vec::new(), 0usize, Vec::new(), None);
        while r.pos < r.limit {
            r.enter(0, ctx)?;
            let start = r.pos;
            let tc = r.byte()?;
            let name = tc_name(tc);
            let parsed = (|| {
                let mut marker = json!({"offset":start,"code":tc,"type":name});
                match tc {
                    0x70 | 0x78 | 0x79 | 0x73 => {
                        if tc == 0x73 {
                            marker["parsed"] = json!(false);
                            unsupported.push(start);
                        }
                        if tc == 0x70 {
                            count += 1;
                        }
                        block_lines.push(format!("  @{start}: {name}"));
                    }
                    0x74 | 0x7c => {
                        let s = java_string(
                            &mut r,
                            if tc == 0x74 { 2 } else { 8 },
                            encoding == "utf8",
                        )?;
                        marker["string"] = s.clone();
                        marker["scanIndex"] = json!(count);
                        block_lines.push(format!(
                            "  @{start}: {name} ({} #{count}): \"{}\"",
                            if legacy { "handle" } else { "扫描索引" },
                            s["text"].as_str().unwrap()
                        ));
                        count += 1;
                    }
                    0x72 => {
                        let s = java_string(&mut r, 2, encoding == "utf8")?;
                        let class = s["text"].as_str().unwrap();
                        let uid = hex::encode(r.take(8)?);
                        let raw_flags = r.byte()?;
                        let labels = flags(raw_flags, legacy);
                        let n = r.len(2, false)?;
                        if n > 20_000 {
                            return Err(bad("Java 字段数超过 20000"));
                        }
                        let mut fields = Vec::new();
                        for _ in 0..n {
                            r.enter(0, ctx)?;
                            let kind = r.byte()? as char;
                            if !"BCDFIJSZ[L".contains(kind) {
                                return Err(bad("Java 字段类型无效"));
                            }
                            let s = java_string(&mut r, 2, encoding == "utf8")?;
                            let mut f = json!({"type":kind.to_string(),"name":s["text"],"nameRawHex":s["rawHex"]});
                            if matches!(kind, 'L' | '[') {
                                let tag = r.byte()?;
                                match tag {
                                    0x74 | 0x7c => {
                                        f["class"] = java_string(
                                            &mut r,
                                            if tag == 0x74 { 2 } else { 8 },
                                            encoding == "utf8",
                                        )?;
                                    }
                                    0x71 => f["reference"] = json!(r.uint(4, false)?),
                                    _ => return Err(bad("Java 对象字段需要字符串或引用")),
                                }
                            }
                            fields.push(f);
                        }
                        block_lines.push(format!(
                            "  @{start}: TC_CLASSDESC ({} #{count}):",
                            if legacy { "handle" } else { "扫描索引" }
                        ));
                        block_lines.push(format!("    className: {class}"));
                        block_lines.push(format!("    serialVersionUID: {uid}"));
                        block_lines.push(format!(
                            "    flags: {}",
                            if labels.is_empty() {
                                "(none)".into()
                            } else {
                                labels.join(" | ")
                            }
                        ));
                        block_lines.push(format!("    fields ({n}):"));
                        for f in &fields {
                            let class = f["class"]["text"]
                                .as_str()
                                .filter(|s| !s.is_empty())
                                .map(|s| format!(" : {s}"))
                                .unwrap_or_default();
                            block_lines.push(format!(
                                "      {} {}{class}",
                                f["type"].as_str().unwrap(),
                                f["name"].as_str().unwrap()
                            ));
                        }
                        marker["className"] = s;
                        marker["serialVersionUID"] = json!(uid);
                        marker["flags"] = json!(raw_flags);
                        marker["flagNames"] = json!(flags(raw_flags, false));
                        marker["fields"] = json!(fields);
                        marker["scanIndex"] = json!(count);
                        count += 1;
                    }
                    0x77 | 0x7a => {
                        let n = r.len(if tc == 0x77 { 1 } else { 4 }, false)?;
                        let b = r.take(n)?;
                        marker["rawHex"] = json!(hex::encode(b));
                        marker["length"] = json!(n);
                        block_lines.push(format!(
                            "  @{start}: {name} ({n} bytes): {}",
                            hex_preview(b, 32)
                        ));
                    }
                    0x71 => {
                        let handle = r.uint(4, false)?;
                        marker["handle"] = json!(handle);
                        block_lines.push(format!(
                            "  @{start}: TC_REFERENCE → handle 0x{handle:X} (#{})",
                            handle as i64 - 0x7e0000
                        ));
                    }
                    _ => {
                        marker["parsed"] = json!(false);
                        unsupported.push(start);
                        block_lines.push(format!("  @{start}: {name} (未深入解析)"));
                    }
                }
                marker["end"] = json!(r.pos);
                Ok(marker)
            })();
            match parsed {
                Ok(v) => markers.push(v),
                Err(CoreError::Cancelled) => return Err(CoreError::Cancelled),
                Err(e) => {
                    let s = error_message(&e);
                    block_lines.push(format!("  ⚠ 扫描中断 @pos={}: {s}", r.pos));
                    error = Some(s);
                    break;
                }
            }
        }
        lines.push("--- 顶层结构 ---".into());
        if block_lines.is_empty() {
            lines.push("  （无顶层内容标记）".into());
        } else {
            lines.extend(block_lines);
        }
        lines.push(String::new());
        lines.push(format!(
            "{}: {count}",
            if legacy {
                "句柄数"
            } else {
                "扫描对象数（含 null）"
            }
        ));
        lines.push("说明: 完整 Java 序列化解析复杂，此处识别 magic + 扫描顶层 TC_* 标记（TC_STRING/TC_CLASSDESC/TC_BLOCKDATA 等提取关键信息）。".into());
        if error.is_none() && (!unsupported.is_empty() || version != 5) {
            error = Some("包含非标准版本或未深入解析标记；仅完成静态扫描".into());
        }
        tree["markers"] = json!(markers);
        tree["scanCount"] = json!(count);
        tree["unsupportedOffsets"] = json!(unsupported);
        ctx.cancel.check()?;
        Ok(outputs(lines.join("\n"), tree, r.pos, &data, error))
    }
}
pub fn register(reg: &mut NodeRegistry) {
    use super::codec_support::help;
    let mut d=descriptor("java_serialize_identify","Java 序列化静态识别","javaSerializeIdent","识别 ACED/0005 并扫描 TC_STRING/CLASSDESC/BLOCKDATA 等标记，保留原始字节、偏移和 modified UTF-8 的 UTF-16 单元。扫描索引不是完整协议句柄表；不深入的标记返回 complete=false。","aced0005740003486921");
    d.params.retain(|p| p.name != "maxDepth");
    d.params.extend([
        help(
            ParamSpec::select(
                "stringEncoding",
                "字符串编码",
                &["modified-utf8", "utf8"],
                "modified-utf8",
            ),
            "Java 标准使用 modified UTF-8；utf8 仅兼容上游普通 TextDecoder。",
        ),
        help(
            ParamSpec::toggle("legacyReport", "兼容上游报告", false),
            "复现上游 handle 索引名称和错位的 flags 文案；结构树 flagNames 始终为正确协议含义。",
        ),
    ]);
    reg.register(d, Arc::new(|| Arc::new(Java)));
}
