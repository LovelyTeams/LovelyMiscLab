use super::basex::*;
use super::prelude::*;
const ALPHABET: &str =
    "!#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_abcdefghijklmnopqrstuvwxyz{|}";
fn alpha(p: &serde_json::Value) -> Result<Vec<char>, CoreError> {
    let chars = super::codec_support::alphabet(pstr(p, "alphabet", ALPHABET), Some(91))?;
    if chars.contains(&'~') {
        return Err(CoreError::Parse("~ 保留为空输入标记".into()));
    }
    Ok(chars)
}

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        params: &serde_json::Value,
        _ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let alphabet = alpha(params)?;
        let data = in_bytes(inputs, "data")?;
        if data.is_empty() && pbool(params, "emptyMarker", false) {
            return Ok(out_text("~".into()));
        }
        let raw = base92_encode(&data);
        Ok(out_text(
            raw.chars()
                .map(|c| alphabet[ALPHABET.find(c).expect("standard base92")])
                .collect(),
        ))
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
        let alphabet = alpha(params)?;
        let s = in_text(inputs, "text")?;
        let s = if alphabet.iter().any(|c| c.is_whitespace()) {
            s
        } else {
            s.trim()
        };
        if s == "~" {
            return Ok(decoded(vec![]));
        }
        let values = s
            .chars()
            .map(|c| {
                alphabet
                    .iter()
                    .position(|a| *a == c)
                    .ok_or_else(|| CoreError::Parse("Base92 码表外字符".into()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if values.chunks(2).any(|p| {
            if p.len() == 2 {
                p[0] * 91 + p[1] > 8191
            } else {
                p[0] > 63
            }
        }) {
            return Err(CoreError::Parse("Base92 分组值超出位宽".into()));
        }
        let raw: String = values
            .iter()
            .map(|&i| ALPHABET.as_bytes()[i] as char)
            .collect();
        Ok(decoded(base92_decode(&raw)?))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let alphabet = || ParamSpec::text("alphabet", "码表（91 字符，~ 保留）", ALPHABET, false);
    reg.register(
        super::codec_support::document_base(
            desc(
                "base92_encode",
                ENC,
                "Base92 编码",
                BLUE,
                vec![req("data", "输入", PortType::Any)],
                vec![t_out()],
                vec![
                    alphabet(),
                    ParamSpec::toggle("emptyMarker", "空输入输出 ~", false),
                ],
            ),
            "base92",
            "Hello",
            serde_json::json!({"emptyMarker":true}),
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        super::codec_support::document_base(
            desc(
                "base92_decode",
                ENC,
                "Base92 解码",
                BLUE,
                vec![t_in()],
                vec![
                    req("text", "文本", PortType::Text),
                    opt("bytes", "字节", PortType::Bytes),
                ],
                vec![alphabet()],
            ),
            "base92",
            "~",
            serde_json::json!({}),
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
