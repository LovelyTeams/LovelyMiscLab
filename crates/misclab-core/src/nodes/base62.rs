use super::basex::*;
use super::prelude::*;

fn alpha(params: &serde_json::Value) -> Result<Vec<char>, CoreError> {
    let raw = pstr(params, "alphabet", B62_STANDARD);
    let text: String = if pbool(params, "literalAlphabet", false) {
        raw.into()
    } else {
        expand_alph_range(raw).iter().collect()
    };
    super::codec_support::alphabet(&text, None)
}

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        Ok(out_text(radix_encode_checked(
            &in_bytes(inputs, "data")?,
            &alpha(params)?,
            pbool(params, "preserveZeros", false),
            Some(ctx),
        )?))
    }
}

struct Dec;
impl Node for Dec {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let alpha = alpha(params)?;
        let text = in_text(inputs, "text")?;
        let text = if alpha.iter().any(|c| c.is_whitespace()) {
            text
        } else {
            text.trim()
        };
        Ok(decoded(radix_decode_checked(
            text,
            &alpha,
            pbool(params, "preserveZeros", false),
            pbool(params, "strip", true),
            Some(ctx),
        )?))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let alphabet = || ParamSpec::text("alphabet", "码表", B62_STANDARD, false);
    let zeros = || ParamSpec::toggle("preserveZeros", "保留前导零字节", false);
    let literal = || ParamSpec::toggle("literalAlphabet", "逐字码表（关闭则支持范围）", false);
    reg.register(
        super::codec_support::document_base(
            desc(
                "base62_encode",
                ENC,
                "Base62 编码",
                BLUE,
                vec![req("data", "输入", PortType::Any)],
                vec![t_out()],
                vec![alphabet(), literal(), zeros()],
            ),
            "base62",
            "Hello",
            serde_json::json!({"preserveZeros":true}),
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        super::codec_support::document_base(
            desc(
                "base62_decode",
                ENC,
                "Base62 解码",
                BLUE,
                vec![t_in()],
                vec![
                    req("text", "文本", PortType::Text),
                    opt("bytes", "字节", PortType::Bytes),
                ],
                vec![
                    alphabet(),
                    literal(),
                    zeros(),
                    ParamSpec::toggle("strip", "忽略码表外字符", true),
                ],
            ),
            "base62",
            "5TP3P3v",
            serde_json::json!({"preserveZeros":true}),
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
