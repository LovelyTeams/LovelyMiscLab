//! Common byte-preserving contract for paired codecs. Algorithms stay in Rust.
use super::{basex::decoded, prelude::*};
use serde_json::Value;

pub type Encode = fn(&[u8], &Value) -> Result<String, CoreError>;
pub type Decode = fn(&str, &Value) -> Result<Vec<u8>, CoreError>;

/// Add reviewed text-codec documentation without changing legacy defaults/ports.
pub fn document_text(
    mut d: NodeDescriptor,
    upstream: &str,
    description: &str,
    sample: &str,
    params: Value,
) -> NodeDescriptor {
    d.upstream_ids = vec![upstream.into()];
    d.family = upstream.into();
    d.aliases.push(upstream.into());
    d.description = description.into();
    d.examples.push(crate::node::descriptor::NodeExample {
        label: "入门样例".into(),
        inputs: one(&d.inputs[0].name, PortValue::Text(sample.into())),
        params,
    });
    d
}

pub fn help(mut p: ParamSpec, description: &str) -> ParamSpec {
    p.description = description.into();
    p
}

/// Metadata for the audited legacy Base nodes. Keeps their IDs/ports/defaults.
pub fn document_base(
    mut d: NodeDescriptor,
    upstream: &str,
    sample: &str,
    params: Value,
) -> NodeDescriptor {
    d.upstream_ids = vec![upstream.into()];
    d.family = upstream.into();
    if ["base58", "base62"].contains(&upstream) {
        d.cost = Cost::Heavy;
    }
    d.aliases.push(upstream.into());
    d.description = if d.id.ends_with("_encode") {
        format!(
            "{}；支持文本和原始字节。已有参数默认行为保持兼容，其他变体在参数中选择。",
            d.display_name
        )
    } else {
        format!(
            "{}；提供文本预览和完整 bytes，二进制处理请连接字节端口。",
            d.display_name
        )
    };
    for p in &mut d.params {
        if !p.description.is_empty() {
            continue;
        }
        p.description = match p.name.as_str() {
            "variant" => "选择标准码表或变体；自定义模式使用下方码表。旧流程参数值保持兼容。",
            "alphabet" => "自定义字符表，不能重复；范围语法仅在未开启逐字码表的节点中生效。",
            "padding" => "编码时添加 = 填充；Crockford 与 z-base-32 不填充，解码同时接受有无填充。",
            "strip" => "解码是否忽略码表外字符。关闭后严格报错；合法编码符号中的空格仍保留。",
            "upper" => "控制 Hex 编码字母大小写；旧流程默认小写，上游默认大写需要开启。",
            "space" => "在每个编码字节之间添加一个空格。",
            "delim" => "Ascii85 编码包含 <~ ~> 信封；关闭对应上游 compat=true。",
            "preserveZeros" => {
                "把每个前导零字节保留为码表首字符；开启以匹配 EBCTFCodeBox，关闭保持旧大整数模式。"
            }
            "literalAlphabet" => {
                "开启时逐字解释自定义码表（上游 dict）；关闭保留 a-z 范围语法兼容旧流程。"
            }
            "emptyMarker" => "开启时空输入编码为 ~（上游约定）；关闭保留旧流程空字符串输出。",
            _ => "按参数类型填写，默认值保持旧流程兼容。",
        }
        .into();
    }
    d.examples.push(crate::node::descriptor::NodeExample {
        label: "入门样例".into(),
        inputs: one(&d.inputs[0].name, PortValue::Text(sample.into())),
        params,
    });
    d
}

struct Encoder(Encode);
struct Decoder(Decode);
impl Node for Encoder {
    fn run(
        &self,
        inputs: &PortMap,
        params: &Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        Ok(out_text((self.0)(&in_bytes(inputs, "data")?, params)?))
    }
}
impl Node for Decoder {
    fn run(
        &self,
        inputs: &PortMap,
        params: &Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        Ok(decoded((self.0)(in_text(inputs, "text")?, params)?))
    }
}

pub fn register_pair(
    reg: &mut NodeRegistry,
    id: &str,
    name: &str,
    mut params: Vec<ParamSpec>,
    encode: Encode,
    decode: Decode,
) {
    for param in &mut params {
        if param.description.is_empty() {
            param.description = match param.name.as_str() {
                "alphabet" => "逐字列出唯一字符；按 Unicode 字符计数，不能重复。",
                "strip" => "解码时忽略码表外字符；关闭后严格拒绝无效输入。",
                "check" => "编码追加模 37 校验字符，解码验证并移除校验字符。",
                "dashGroup" => "每组字符之间插入连字符；0 表示不分组。解码会忽略连字符。",
                "delimiter" => "控制编码字节之间的分隔符；解码兼容常见分隔符。",
                "bare" => "只输出编码数据行；关闭时包含 begin/end 信封，解码同时兼容两种形式。",
                _ => "保留默认值可使用标准格式；按当前参数声明配置变体。",
            }
            .into();
        }
    }
    let defaults = Value::Object(
        params
            .iter()
            .map(|p| (p.name.clone(), p.default.clone()))
            .collect(),
    );
    let sample = "Hello, 中文!";
    let encoded_sample =
        encode(sample.as_bytes(), &defaults).expect("valid default codec parameters");
    let mut enc = desc(
        &format!("{id}_encode"),
        ENC,
        &format!("{name} 编码"),
        BLUE,
        vec![req("data", "输入", PortType::Any)],
        vec![t_out()],
        params.clone(),
    );
    enc.description = format!("{name}，支持文本与原始字节输入。");
    enc.examples.push(crate::node::descriptor::NodeExample {
        label: "入门样例".into(),
        inputs: one("data", PortValue::Text(sample.into())),
        params: defaults.clone(),
    });
    reg.register(enc, Arc::new(move || Arc::new(Encoder(encode))));
    let mut dec = desc(
        &format!("{id}_decode"),
        ENC,
        &format!("{name} 解码"),
        BLUE,
        vec![t_in()],
        vec![t_out(), req("bytes", "原始字节", PortType::Bytes)],
        params,
    );
    dec.description = "解码结果同时提供文本预览和完整字节，二进制数据请使用字节端口。".into();
    dec.examples.push(crate::node::descriptor::NodeExample {
        label: "入门样例".into(),
        inputs: one("text", PortValue::Text(encoded_sample)),
        params: defaults,
    });
    reg.register(dec, Arc::new(move || Arc::new(Decoder(decode))));
}

pub fn alphabet(text: &str, size: Option<usize>) -> Result<Vec<char>, CoreError> {
    let chars: Vec<char> = text.chars().collect();
    let unique: std::collections::HashSet<char> = chars.iter().copied().collect();
    if chars.len() < 2 || unique.len() != chars.len() || size.is_some_and(|n| n != chars.len()) {
        return Err(CoreError::Parse(
            "码表长度无效或包含重复字符（至少 2 个唯一字符）".into(),
        ));
    }
    Ok(chars)
}

pub fn bit_encode(data: &[u8], alphabet: &[char], width: u32) -> String {
    let mut out = String::new();
    let (mut buffer, mut bits) = (0u32, 0u32);
    let mask = (1 << width) - 1;
    for &byte in data {
        buffer = (buffer << 8) | byte as u32;
        bits += 8;
        while bits >= width {
            bits -= width;
            out.push(alphabet[((buffer >> bits) & mask) as usize]);
        }
        buffer &= (1 << bits) - 1;
    }
    if bits > 0 {
        out.push(alphabet[((buffer << (width - bits)) & mask) as usize]);
    }
    out
}

pub fn bit_decode(text: &str, alphabet: &[char], width: u32) -> Result<Vec<u8>, CoreError> {
    let (mut buffer, mut bits) = (0u32, 0u32);
    let mut out = Vec::new();
    let mut count = 0;
    for ch in text.chars().filter(|c| !c.is_whitespace()) {
        let value = alphabet
            .iter()
            .position(|&c| c == ch)
            .ok_or_else(|| CoreError::Parse(format!("码表外字符：{ch}")))?;
        count += 1;
        buffer = (buffer << width) | value as u32;
        bits += width;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
        buffer &= (1 << bits) - 1;
    }
    if (count > 0 && bits >= width) || buffer != 0 {
        return Err(CoreError::Parse("编码长度或尾部补位无效".into()));
    }
    Ok(out)
}
