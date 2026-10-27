use super::basex::*;
use super::prelude::*;

fn alpha(params: &serde_json::Value) -> Result<Vec<char>, CoreError> {
    let s = match pstr(params, "variant", "Bitcoin") {
        "Ripple" => B58_RIPPLE,
        "Flickr" => "123456789abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ",
        "自定义" => pstr(params, "alphabet", B58_BITCOIN),
        _ => B58_BITCOIN,
    };
    let text: String = if pbool(params, "literalAlphabet", false) {
        s.into()
    } else {
        expand_alph_range(s).iter().collect()
    };
    super::codec_support::alphabet(&text, Some(58))
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
            true,
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
        let strip = pbool(params, "strip", true);
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
            true,
            strip,
            Some(ctx),
        )?))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let variant = || {
        ParamSpec::select(
            "variant",
            "码表",
            &["Bitcoin", "Flickr", "Ripple", "自定义"],
            "Bitcoin",
        )
    };
    let custom = || ParamSpec::text("alphabet", "自定义码表(58字符)", "", false);
    let literal = || ParamSpec::toggle("literalAlphabet", "逐字码表（关闭则支持范围）", false);
    reg.register(
        super::codec_support::document_base(
            desc(
                "base58_encode",
                ENC,
                "Base58 编码",
                BLUE,
                vec![req("data", "输入", PortType::Any)],
                vec![t_out()],
                vec![variant(), custom(), literal()],
            ),
            "base58",
            "hello world",
            serde_json::json!({}),
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        super::codec_support::document_base(
            desc(
                "base58_decode",
                ENC,
                "Base58 解码",
                BLUE,
                vec![t_in()],
                vec![
                    req("text", "文本", PortType::Text),
                    opt("bytes", "字节", PortType::Bytes),
                ],
                vec![
                    variant(),
                    custom(),
                    literal(),
                    ParamSpec::toggle("strip", "去除非码表字符", true),
                ],
            ),
            "base58",
            "StV1DL6CwTryKyV",
            serde_json::json!({}),
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
