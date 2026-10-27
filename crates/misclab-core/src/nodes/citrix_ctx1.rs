use super::prelude::*;
fn encode(text: &str) -> String {
    let mut previous = 0u8;
    let mut out = String::new();
    for byte in text.encode_utf16().flat_map(u16::to_le_bytes) {
        previous ^= byte ^ 0xa5;
        out.push((b'A' + (previous >> 4)) as char);
        out.push((b'A' + (previous & 15)) as char);
    }
    out
}
fn decode(text: &str) -> Result<String, CoreError> {
    let text = text.trim().to_ascii_uppercase();
    if !text.len().is_multiple_of(4) || !text.bytes().all(|b| (b'A'..=b'P').contains(&b)) {
        return Err(CoreError::Parse(
            "CTX1 必须由 A–P 组成且长度为 4 的倍数".into(),
        ));
    }
    let mut previous = 0;
    let bytes: Vec<u8> = text
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let value = ((pair[0] - b'A') << 4) | (pair[1] - b'A');
            let byte = value ^ 0xa5 ^ previous;
            previous = value;
            byte
        })
        .collect();
    let utf16: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|p| u16::from_le_bytes([p[0], p[1]]))
        .collect();
    String::from_utf16(&utf16).map_err(|e| CoreError::Parse(format!("CTX1 UTF-16 无效：{e}")))
}
struct Ctx1(bool);
impl Node for Ctx1 {
    fn run(
        &self,
        inputs: &PortMap,
        _: &serde_json::Value,
        _: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(inputs, "text")?;
        Ok(out_text(if self.0 { encode(text) } else { decode(text)? }))
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for (id, name, encoding) in [
        ("citrix_ctx1_encode", "Citrix CTX1 编码", true),
        ("citrix_ctx1_decode", "Citrix CTX1 解码", false),
    ] {
        let mut d = desc(id, ENC, name, BLUE, vec![t_in()], vec![t_out()], vec![]);
        d.description =
            "Citrix CTX1 文本编码：UTF-16LE 字节经链式异或映射为 A–P；解码还原原文并校验代理对。"
                .into();
        d.examples.push(crate::node::descriptor::NodeExample {
            label: "入门样例".into(),
            inputs: one(
                "text",
                PortValue::Text(if encoding {
                    "Hello, 中文!".into()
                } else {
                    encode("Hello, 中文!")
                }),
            ),
            params: serde_json::json!({}),
        });
        reg.register(d, Arc::new(move || Arc::new(Ctx1(encoding))));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vector_and_unicode() {
        assert_eq!(encode("A"), "OEEB");
        assert_eq!(decode("OEEB").unwrap(), "A");
        let text = " 中文🔑\n";
        assert_eq!(decode(&encode(text)).unwrap(), text);
        assert!(decode("AAA").is_err());
        assert!(decode("!!!!").is_err());
    }
}
