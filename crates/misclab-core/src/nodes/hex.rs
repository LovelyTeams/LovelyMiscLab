use super::codec_support::alphabet;
use super::prelude::*;
const HEX: &str = "0123456789ABCDEF";

fn table(p: &serde_json::Value) -> Result<Vec<char>, CoreError> {
    alphabet(pstr(p, "alphabet", HEX), Some(16))
}

struct Decode;
impl Node for Decode {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let raw = in_text(inputs, "text")?.replace("0x", "").replace("0X", "");
        let mut chars = table(params)?;
        let standard = chars.iter().all(|c| c.is_ascii_hexdigit());
        if standard {
            chars.iter_mut().for_each(|c| *c = c.to_ascii_lowercase());
        }
        let input: Vec<char> = raw
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| if standard { c.to_ascii_lowercase() } else { c })
            .filter(|c| !pbool(params, "strip", false) || chars.contains(c))
            .collect();
        if !input.len().is_multiple_of(2) {
            return Err(CoreError::Parse("Hex 需要偶数个字符".into()));
        }
        let bytes = input
            .chunks(2)
            .map(|pair| {
                let hi = chars
                    .iter()
                    .position(|c| *c == pair[0])
                    .ok_or_else(|| CoreError::Parse("Hex 码表外字符".into()))?;
                let lo = chars
                    .iter()
                    .position(|c| *c == pair[1])
                    .ok_or_else(|| CoreError::Parse("Hex 码表外字符".into()))?;
                Ok((hi * 16 + lo) as u8)
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
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
        let mut chars = table(params)?;
        if !pbool(params, "upper", false) {
            chars.iter_mut().for_each(|c| *c = c.to_ascii_lowercase());
        }
        let chunks: Vec<String> = in_bytes(inputs, "text")?
            .iter()
            .map(|b| {
                [chars[(b >> 4) as usize], chars[(b & 15) as usize]]
                    .iter()
                    .collect()
            })
            .collect();
        Ok(out_text(chunks.join(if pbool(params, "space", false) {
            " "
        } else {
            ""
        })))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let dict = || ParamSpec::text("alphabet", "码表（16 字符）", HEX, false);
    reg.register(
        super::codec_support::document_base(
            desc(
                "hex_decode",
                ENC,
                "Hex 解码",
                BLUE,
                vec![t_in()],
                vec![t_out(), req("bytes", "完整字节", PortType::Bytes)],
                vec![dict(), ParamSpec::toggle("strip", "忽略码表外字符", false)],
            ),
            "base16",
            "48656c6c6f",
            serde_json::json!({}),
        ),
        Arc::new(|| Arc::new(Decode)),
    );
    reg.register(
        super::codec_support::document_base(
            desc(
                "hex_encode",
                ENC,
                "Hex 编码",
                BLUE,
                vec![req("text", "输入", PortType::Any)],
                vec![t_out()],
                vec![
                    dict(),
                    ParamSpec::toggle("upper", "大写", false),
                    ParamSpec::toggle("space", "空格分组", false),
                ],
            ),
            "base16",
            "Hello",
            serde_json::json!({}),
        ),
        Arc::new(|| Arc::new(Encode)),
    );
}
