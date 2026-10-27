use super::prelude::*;
use base64::Engine as _;
const STANDARD: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
fn custom(p: &serde_json::Value) -> Result<Option<Vec<char>>, CoreError> {
    if pstr(p, "variant", "标准") != "自定义" {
        return Ok(None);
    }
    let chars = super::codec_support::alphabet(pstr(p, "alphabet", ""), Some(64))?;
    if chars.contains(&'=') || chars.iter().any(|c| c.is_whitespace()) {
        return Err(CoreError::Parse("Base64 码表不能含 = 或空白".into()));
    }
    Ok(Some(chars))
}

struct Decode;
impl Node for Decode {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let mut cleaned: String = in_text(inputs, "text")?.split_whitespace().collect();
        let mapped = custom(params)?;
        if let Some(chars) = &mapped {
            if pbool(params, "urlsafe", false) {
                cleaned = cleaned.replace('-', "+").replace('_', "/");
            }
            cleaned = cleaned
                .chars()
                .map(|c| {
                    if c == '=' {
                        Ok('=')
                    } else {
                        chars
                            .iter()
                            .position(|a| *a == c)
                            .map(|i| STANDARD.as_bytes()[i] as char)
                            .ok_or_else(|| CoreError::Parse("Base64 码表外字符".into()))
                    }
                })
                .collect::<Result<String, _>>()?;
        } else if pstr(params, "variant", "标准") == "标准" {
            cleaned = cleaned.replace('-', "+").replace('_', "/");
        }
        let engine_params = if mapped.is_some() {
            serde_json::json!({})
        } else {
            params.clone()
        };
        let bytes = base64_engine(&engine_params)?
            .decode(cleaned.as_bytes())
            .map_err(|e| CoreError::Parse(format!("Base64 解码失败: {e}")))?;
        Ok(super::basex::decoded(bytes))
    }
}

struct Encode;
impl Node for Encode {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let data = in_bytes(inputs, "text")?;
        let mut encoded: String = if let Some(chars) = custom(params)? {
            let raw = base64::engine::general_purpose::STANDARD.encode(data);
            let raw = if pbool(params, "padding", true) {
                raw.as_str()
            } else {
                raw.trim_end_matches('=')
            };
            raw.chars()
                .map(|c| {
                    if c == '=' {
                        '='
                    } else {
                        chars[STANDARD.find(c).expect("standard base64")]
                    }
                })
                .collect()
        } else {
            base64_engine(params)?.encode(data)
        };
        if pbool(params, "urlsafe", false) {
            encoded = encoded.replace('+', "-").replace('/', "_");
        }
        Ok(out_text(encoded))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        super::codec_support::document_base(
            desc(
                "base64_decode",
                ENC,
                "Base64 解码",
                BLUE,
                vec![t_in()],
                vec![t_out(), req("bytes", "完整字节", PortType::Bytes)],
                params(),
            ),
            "base64",
            "SGVsbG8=",
            serde_json::json!({}),
        ),
        Arc::new(|| Arc::new(Decode)),
    );
    reg.register(
        super::codec_support::document_base(
            desc(
                "base64_encode",
                ENC,
                "Base64 编码",
                BLUE,
                vec![req("text", "输入", PortType::Any)],
                vec![t_out()],
                params(),
            ),
            "base64",
            "Hello",
            serde_json::json!({}),
        ),
        Arc::new(|| Arc::new(Encode)),
    );
}

fn params() -> Vec<ParamSpec> {
    let mut params = base64_params();
    let mut url = ParamSpec::toggle("urlsafe", "URL 安全字符替换", false);
    url.description = "对标准或自定义码表的 +/ 进行 -_ 替换；可与自定义模式同时开启。".into();
    params.push(url);
    params
}
