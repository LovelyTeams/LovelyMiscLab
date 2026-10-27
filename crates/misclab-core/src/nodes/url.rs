//! Percent encoding with explicit form/byte modes; legacy defaults stay intact.
use super::{
    basex::decoded,
    codec_support::{document_text, help},
    prelude::*,
};
use serde_json::json;

struct Decode;
impl Node for Decode {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let mut text = in_text(inputs, "text")?;
        if pbool(params, "trim", false) {
            text = text.trim();
        }
        let input = text.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < input.len() {
            if i % 4096 == 0 {
                ctx.cancel.check()?;
            }
            if input[i] == b'%' {
                if let Some(pair) = input.get(i + 1..i + 3) {
                    if let (Some(h), Some(l)) = (
                        (pair[0] as char).to_digit(16),
                        (pair[1] as char).to_digit(16),
                    ) {
                        out.push((h * 16 + l) as u8);
                        i += 3;
                        continue;
                    }
                }
                if pbool(params, "strict", false) {
                    return Err(CoreError::Parse(format!(
                        "URL：偏移 {i} 的 % 后需要两位 Hex"
                    )));
                }
            }
            out.push(if input[i] == b'+' && pbool(params, "plus", false) {
                b' '
            } else {
                input[i]
            });
            i += 1;
        }
        if pbool(params, "utf8", true) {
            std::str::from_utf8(&out).map_err(|e| {
                CoreError::Parse(format!(
                    "URL 结果不是 UTF-8；处理二进制请关闭 UTF-8 校验并使用 bytes：{e}"
                ))
            })?;
        }
        Ok(decoded(out))
    }
}

struct Encode;
impl Node for Encode {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let mode = pstr(params, "mode", "full");
        if !["standard", "full", "plus"].contains(&mode) {
            return Err(CoreError::Parse("URL 模式无效".into()));
        }
        let mut out = String::new();
        for (i, b) in in_bytes(inputs, "text")?.into_iter().enumerate() {
            if i % 4096 == 0 {
                ctx.cancel.check()?;
            }
            if b.is_ascii_alphanumeric()
                || b"_.~-".contains(&b)
                || (mode == "standard" && b"!*'()".contains(&b))
            {
                out.push(b as char);
            } else if b == b' ' && mode == "plus" {
                out.push('+');
            } else {
                out.push_str(&format!("%{b:02X}"));
            }
        }
        Ok(out_text(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        document_text(
            desc(
                "url_decode",
                ENC,
                "URL 解码",
                BLUE,
                vec![t_in()],
                vec![t_out(), req("bytes", "完整字节", PortType::Bytes)],
                vec![
                    help(
                        ParamSpec::toggle("plus", "+ 视为空格", false),
                        "表单模式开启；关闭保留字面 +，兼容旧流程。",
                    ),
                    help(
                        ParamSpec::toggle("trim", "移除首尾空白", false),
                        "只移除编码文本外部空白；上游解码默认开启。",
                    ),
                    help(
                        ParamSpec::toggle("strict", "严格百分号校验", false),
                        "开启时拒绝不完整或无效的 %XX；关闭保留原文。",
                    ),
                    help(
                        ParamSpec::toggle("utf8", "校验 UTF-8", true),
                        "关闭后允许任意字节；二进制处理请连接 bytes 端口。",
                    ),
                ],
            ),
            "url",
            "百分号解码；表单加号、外部空白和严格校验可独立配置，完整数据由 bytes 输出。",
            "a%2Bb+%E4%B8%AD",
            json!({"plus":true,"trim":true,"strict":true}),
        ),
        Arc::new(|| Arc::new(Decode)),
    );
    reg.register(
        document_text(
            desc(
                "url_encode",
                ENC,
                "URL 编码",
                BLUE,
                vec![req("text", "输入", PortType::Any)],
                vec![t_out()],
                vec![help(
                    ParamSpec::select("mode", "模式", &["standard", "full", "plus"], "full"),
                    "standard 保留 !*'()；full 编码它们（旧默认）；plus 额外把空格转为 +。",
                )],
            ),
            "url",
            "文本 UTF-8 或任意字节转百分号编码；标准、完整和表单三种模式。",
            "Hello, 中文!",
            json!({"mode":"standard"}),
        ),
        Arc::new(|| Arc::new(Encode)),
    );
}
