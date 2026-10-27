use super::prelude::*;
struct Rot(bool);
impl Node for Rot {
    fn run(
        &self,
        inputs: &PortMap,
        _: &serde_json::Value,
        _: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        Ok(out_text(
            in_text(inputs, "text")?
                .chars()
                .map(|c| match c {
                    '0'..='9' => (b'0' + (c as u8 - b'0' + 5) % 10) as char,
                    'a'..='z' if self.0 => (b'a' + (c as u8 - b'a' + 13) % 26) as char,
                    'A'..='Z' if self.0 => (b'A' + (c as u8 - b'A' + 13) % 26) as char,
                    _ => c,
                })
                .collect(),
        ))
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for (id, name, letters) in [
        ("rot5", "ROT5 数字轮转", false),
        ("rot18", "ROT18 字母与数字轮转", true),
    ] {
        let mut d = desc(id, ENC, name, BLUE, vec![t_in()], vec![t_out()], vec![]);
        d.description = if letters {
            "数字轮转 5 位、英文字母轮转 13 位，其他字符保留；执行两次还原原文。"
        } else {
            "仅将数字 0–9 轮转 5 位，字母和其他字符保留；执行两次还原原文。"
        }
        .into();
        d.examples.push(crate::node::descriptor::NodeExample {
            label: "自逆变换样例".into(),
            inputs: one("text", PortValue::Text("flag{Hello123}".into())),
            params: serde_json::json!({}),
        });
        reg.register(d, Arc::new(move || Arc::new(Rot(letters))));
    }
}
