use super::basex::*;
use super::prelude::*;

fn alpha(params: &serde_json::Value) -> Result<Vec<char>, CoreError> {
    super::codec_support::alphabet(
        match pstr(params, "variant", "标准") {
            "Hex 扩展" => "0123456789ABCDEFGHIJKLMNOPQRSTUV",
            "Crockford" => "0123456789ABCDEFGHJKMNPQRSTVWXYZ",
            "z-base-32" => "ybndrfg8ejkmcpqxot1uwisza345h769",
            "自定义" => pstr(params, "alphabet", "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567"),
            _ => "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567",
        },
        Some(32),
    )
    .and_then(|a| {
        if a.contains(&'=') || a.iter().any(|c| c.is_whitespace()) {
            Err(CoreError::Parse("Base32 码表不能包含 = 或空白".into()))
        } else {
            Ok(a)
        }
    })
}

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let mut text =
            super::codec_support::bit_encode(&in_bytes(inputs, "data")?, &alpha(params)?, 5);
        if pbool(params, "padding", true)
            && !["Crockford", "z-base-32"].contains(&pstr(params, "variant", "标准"))
        {
            while !text.chars().count().is_multiple_of(8) {
                text.push('=');
            }
        }
        Ok(out_text(text))
    }
}

struct Dec;
impl Node for Dec {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let chars = alpha(params)?;
        let s: String = in_text(inputs, "text")?
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let s = s.trim_end_matches('=');
        let normalized = match pstr(params, "variant", "标准") {
            "自定义" => s.into(),
            "z-base-32" => s.to_ascii_lowercase(),
            "Crockford" => s
                .to_ascii_uppercase()
                .replace('O', "0")
                .replace(['I', 'L'], "1"),
            _ => s.to_ascii_uppercase(),
        };
        let s: String = normalized
            .chars()
            .filter(|c| !pbool(params, "strip", true) || chars.contains(c))
            .collect();
        Ok(decoded(super::codec_support::bit_decode(&s, &chars, 5)?))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let variant = || {
        ParamSpec::select(
            "variant",
            "码表",
            &["标准", "Hex 扩展", "Crockford", "z-base-32", "自定义"],
            "标准",
        )
    };
    let alphabet = || {
        ParamSpec::text(
            "alphabet",
            "自定义码表（32 字符）",
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567",
            false,
        )
    };
    reg.register(
        super::codec_support::document_base(
            desc(
                "base32_encode",
                ENC,
                "Base32 编码",
                BLUE,
                vec![req("data", "输入", PortType::Any)],
                vec![t_out()],
                vec![
                    variant(),
                    alphabet(),
                    ParamSpec::toggle("padding", "填充", true),
                ],
            ),
            "base32",
            "Hello",
            serde_json::json!({}),
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        super::codec_support::document_base(
            desc(
                "base32_decode",
                ENC,
                "Base32 解码",
                BLUE,
                vec![t_in()],
                vec![
                    req("text", "文本", PortType::Text),
                    opt("bytes", "字节", PortType::Bytes),
                ],
                vec![
                    variant(),
                    alphabet(),
                    ParamSpec::toggle("strip", "去除非码表字符", true),
                ],
            ),
            "base32",
            "JBSWY3DP",
            serde_json::json!({}),
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
