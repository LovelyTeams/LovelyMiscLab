//! Bounded byte readers shared by serialization/forensics nodes.
use super::{
    numeric_support::{bad, compact},
    prelude::*,
};
use serde_json::{json, Value};

pub(super) const MAX_INPUT: usize = 1_048_576;
pub(super) fn input(i: &PortMap, p: &Value) -> Result<Vec<u8>, CoreError> {
    let bytes = match i.get("data") {
        Some(PortValue::Bytes(b)) => b.to_vec(),
        Some(PortValue::Text(s)) => {
            if s.len() > MAX_INPUT * 4 {
                return Err(bad("文本输入最多 4 MiB"));
            }
            let enc = pstr(p, "inputEnc", "auto");
            let c = compact(s);
            let is_hex = !c.is_empty()
                && c.len().is_multiple_of(2)
                && c.bytes().all(|b| b.is_ascii_hexdigit());
            let is_b64 = !c.is_empty()
                && c.len().is_multiple_of(4)
                && c.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"+/=".contains(&b));
            let is_url = c.len() >= 4
                && c.contains(['-', '_'])
                && c.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b));
            match enc {
                "hex" if !is_hex => return Err(bad("输入不是合法 hex（偶数长度 0-9a-f）")),
                "hex" => hex::decode(c).map_err(|e| bad(&e.to_string()))?,
                "base64" => {
                    if c.bytes()
                        .any(|b| !b.is_ascii_alphanumeric() && !b"+/=".contains(&b))
                    {
                        return Err(bad("输入不是合法 base64"));
                    }
                    super::transport_codecs::b64decode(&c)?
                }
                "utf8" => s.as_bytes().to_vec(),
                "auto" if is_hex => hex::decode(c).map_err(|e| bad(&e.to_string()))?,
                "auto" if is_b64 || is_url => {
                    super::transport_codecs::b64decode(&c).unwrap_or_else(|_| s.as_bytes().to_vec())
                }
                "auto" => s.as_bytes().to_vec(),
                _ => return Err(bad("未知输入编码")),
            }
        }
        _ => return Err(bad("需要文本或原始字节输入")),
    };
    if bytes.len() > MAX_INPUT {
        return Err(bad("解析输入最多 1 MiB"));
    }
    Ok(bytes)
}
pub(super) struct Reader<'a> {
    pub data: &'a [u8],
    pub pos: usize,
    pub limit: usize,
    pub nodes: usize,
    pub max_depth: usize,
}
impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8], max_depth: usize) -> Self {
        Self {
            data,
            pos: 0,
            limit: data.len(),
            nodes: 0,
            max_depth,
        }
    }
    pub fn enter(&mut self, depth: usize, ctx: &NodeCtx) -> Result<(), CoreError> {
        ctx.cancel.check()?;
        if depth > self.max_depth {
            return Err(bad("嵌套深度超限"));
        }
        self.nodes += 1;
        if self.nodes > 20_000 {
            return Err(bad("解析项数超过 20000"));
        }
        if self.nodes.is_multiple_of(128) {
            ctx.progress(self.pos as f32 / self.data.len().max(1) as f32);
            ctx.cancel.check()?;
        }
        Ok(())
    }
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], CoreError> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&e| e <= self.limit)
            .ok_or_else(|| bad(&format!("数据越界 @{}，需要 {n} 字节", self.pos)))?;
        let b = &self.data[self.pos..end];
        self.pos = end;
        Ok(b)
    }
    pub fn byte(&mut self) -> Result<u8, CoreError> {
        Ok(self.take(1)?[0])
    }
    pub fn uint(&mut self, n: usize, le: bool) -> Result<u64, CoreError> {
        let b = self.take(n)?;
        let mut value = 0;
        if le {
            for (i, &v) in b.iter().enumerate() {
                value |= (v as u64) << (8 * i);
            }
        } else {
            for &v in b {
                value = (value << 8) | v as u64;
            }
        }
        Ok(value)
    }
    pub fn len(&mut self, n: usize, le: bool) -> Result<usize, CoreError> {
        usize::try_from(self.uint(n, le)?).map_err(|_| bad("长度无法表示"))
    }
}
pub(super) fn utf8(b: &[u8]) -> String {
    String::from_utf8_lossy(b.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(b)).into_owned()
}
pub(super) fn hex_preview(b: &[u8], max: usize) -> String {
    let mut s = hex::encode(&b[..b.len().min(max)]);
    if b.len() > max {
        s.push_str(&format!("…({} bytes)", b.len()));
    }
    s
}
pub(super) fn float(n: f64) -> String {
    ryu_js::Buffer::new().format(n).into()
}
pub(super) fn scalar(kind: &str, value: impl Into<Value>) -> Value {
    json!({"type":kind,"value":value.into()})
}
pub(super) fn string_value(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::String(s) => s.clone(),
        Value::Array(v) => v
            .iter()
            .map(|i| {
                if i.is_null() {
                    String::new()
                } else if i.is_object() {
                    "[object Object]".into()
                } else {
                    string_value(i)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".into(),
        _ => v.to_string(),
    }
}
pub(super) fn error_message(e: &CoreError) -> String {
    match e {
        CoreError::Parse(s) => s.clone(),
        _ => e.to_string(),
    }
}
pub(super) fn outputs(
    text: String,
    tree: Value,
    consumed: usize,
    data: &[u8],
    error: Option<String>,
) -> PortMap {
    let consumed = consumed.min(data.len());
    [
        ("text".into(), PortValue::Text(text)),
        ("tree".into(), PortValue::Json(tree)),
        ("consumed".into(), PortValue::Number(consumed as f64)),
        (
            "remainder".into(),
            PortValue::Bytes(data[consumed..].into()),
        ),
        (
            "complete".into(),
            PortValue::Bool(error.is_none() && consumed == data.len()),
        ),
        ("error".into(), PortValue::Text(error.unwrap_or_default())),
    ]
    .into_iter()
    .collect()
}
pub(super) fn descriptor(
    id: &str,
    name: &str,
    upstream: &str,
    description: &str,
    sample: &str,
) -> NodeDescriptor {
    use super::codec_support::{document_text, help};
    let mut d=document_text(desc(id,"data.serialization",name,TEAL,vec![req("data","文本或原始字节",PortType::Any)],vec![t_out(),req("tree","结构树",PortType::Json),req("consumed","已消费字节",PortType::Number),req("remainder","剩余原始字节",PortType::Bytes),req("complete","完整解析",PortType::Bool),req("error","解析错误",PortType::Text)],vec![
        help(ParamSpec::select("inputEnc","输入编码",&["auto","hex","base64","utf8"],"auto"),"文本自动按 Hex、Base64、Base64URL、原始 UTF-8 顺序识别；直接字节输入跳过识别。"),
        help(ParamSpec::number("maxDepth","嵌套深度",1.0,128.0,1.0,64.0),"递归深度 1..128；最多 1 MiB 输入和 20000 个解析项。")
    ]),upstream,description,sample,json!({}));
    d.presentation.previews.insert("tree".into(), "json".into());
    d.cost = Cost::Heavy;
    d
}
