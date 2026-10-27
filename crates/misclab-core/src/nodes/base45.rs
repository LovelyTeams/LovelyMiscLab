use super::basex::*;
use super::prelude::*;
const DEFAULT: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";
fn alpha(p: &serde_json::Value) -> Result<Vec<char>, CoreError> {
    super::codec_support::alphabet(pstr(p, "alphabet", DEFAULT), Some(45))
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
        Ok(out_text(base45_encode(
            &in_bytes(inputs, "data")?,
            &alphabet,
        )))
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
        let strip = pbool(params, "strip", true);
        let text = in_text(inputs, "text")?.replace(['\r', '\n'], "");
        Ok(decoded(base45_decode(&text, &alphabet, strip)?))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    let alphabet = || ParamSpec::text("alphabet", "码表（45 字符）", DEFAULT, false);
    reg.register(
        super::codec_support::document_base(
            desc(
                "base45_encode",
                ENC,
                "Base45 编码",
                BLUE,
                vec![req("data", "输入", PortType::Any)],
                vec![t_out()],
                vec![alphabet()],
            ),
            "base45",
            "AB",
            serde_json::json!({}),
        ),
        Arc::new(|| Arc::new(Enc)),
    );
    reg.register(
        super::codec_support::document_base(
            desc(
                "base45_decode",
                ENC,
                "Base45 解码",
                BLUE,
                vec![t_in()],
                vec![
                    req("text", "文本", PortType::Text),
                    opt("bytes", "字节", PortType::Bytes),
                ],
                vec![
                    alphabet(),
                    ParamSpec::toggle("strip", "去除非码表字符", true),
                ],
            ),
            "base45",
            "BB8",
            serde_json::json!({}),
        ),
        Arc::new(|| Arc::new(Dec)),
    );
}
