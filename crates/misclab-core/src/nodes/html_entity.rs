//! HTML entities — encode / decode (named + numeric).
use super::{
    codec_support::{document_text, help},
    prelude::*,
};
use serde_json::json;

fn named() -> &'static std::collections::HashMap<String, String> {
    static TABLE: std::sync::OnceLock<std::collections::HashMap<String, String>> =
        std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str(include_str!("html_named.json")).expect("pinned HTML entity table")
    })
}

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let all = pstr(params, "mode", "仅特殊字符") == "全部非ASCII";
        let mut out = String::new();
        for c in in_text(inputs, "text")?.chars() {
            match c {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '"' => out.push_str("&quot;"),
                '\'' => out.push_str(if pbool(params, "namedApostrophe", false) {
                    "&apos;"
                } else {
                    "&#39;"
                }),
                c if all && !c.is_ascii() => out.push_str(&format!("&#{};", c as u32)),
                c => out.push(c),
            }
        }
        Ok(out_text(out))
    }
}

struct Dec;
impl Node for Dec {
    fn run(
        &self,
        inputs: &PortMap,
        _p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let re = regex::Regex::new(r"&(?:#([0-9]+)|#[xX]([0-9a-fA-F]+)|([A-Za-z][A-Za-z0-9]*));")
            .unwrap();
        let out = re.replace_all(in_text(inputs, "text")?, |caps: &regex::Captures| {
            if let Some(name) = caps.get(3) {
                return named()
                    .get(name.as_str())
                    .cloned()
                    .unwrap_or_else(|| caps[0].to_string());
            }
            let value = if let Some(dec) = caps.get(1) {
                dec.as_str().parse().ok()
            } else {
                u32::from_str_radix(&caps[2], 16).ok()
            };
            value
                .and_then(char::from_u32)
                .map(|c| c.to_string())
                .unwrap_or_else(|| caps[0].to_string())
        });
        Ok(out_text(out.into_owned()))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        document_text(
            desc(
                "html_entity_encode",
                ENC,
                "HTML 实体编码",
                BLUE,
                vec![t_in()],
                vec![t_out()],
                vec![
                    help(
                        ParamSpec::select(
                            "mode",
                            "范围",
                            &["仅特殊字符", "全部非ASCII"],
                            "仅特殊字符",
                        ),
                        "仅特殊字符转换 HTML 结构字符；全部非ASCII 额外用十进制实体表示非 ASCII。",
                    ),
                    help(
                        ParamSpec::toggle("namedApostrophe", "使用命名单引号实体", false),
                        "开启输出 &apos;；关闭保留旧默认 &#39;，两者均可解码。",
                    ),
                ],
            ),
            "htmlEntity",
            "转义 HTML 结构字符；可选择非 ASCII 数字实体和单引号表示。",
            "<b>中文 & 'CTF'</b>",
            json!({"namedApostrophe":true}),
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        document_text(
            desc(
                "html_entity_decode",
                ENC,
                "HTML 实体解码",
                BLUE,
                vec![t_in()],
                vec![t_out()],
                vec![],
            ),
            "htmlEntity",
            "一次扫描解码常用命名实体和十/十六进制实体；未知实体保留，避免递归双重解码。",
            "&alpha; &rarr; &#x1F642; &amp;lt;",
            json!({}),
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
