//! Classic xxd-style hexdump — to / from.
use super::basex::decoded;
use super::prelude::*;
use super::{
    codec_support::{document_text, help},
    numeric_support::{bad, number},
};
use serde_json::json;

struct Enc;
impl Node for Enc {
    fn run(
        &self,
        inputs: &PortMap,
        p: &serde_json::Value,
        c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        c.cancel.check()?;
        let data = in_bytes(inputs, "data")?;
        if data.len() > 1_048_576 {
            return Err(bad("Hexdump编码输入最多1MiB"));
        }
        match pstr(p, "style", "legacy") {
            "xxd" => return Ok(out_text(super::hexdump_xxd::encode(&data, p, c)?)),
            "legacy" => {}
            _ => return Err(bad("Hexdump格式无效")),
        }
        let mut out = String::new();
        for (i, chunk) in data.chunks(16).enumerate() {
            if i.is_multiple_of(256) {
                c.progress((i * 16) as f32 / data.len().max(1) as f32);
                c.cancel.check()?;
            }
            let hex = chunk
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            let ascii: String = chunk
                .iter()
                .map(|&b| {
                    if (0x20..0x7f).contains(&b) {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            out.push_str(&format!("{:08x}  {:<47}  {}\n", i * 16, hex, ascii));
        }
        c.progress(1.0);
        c.cancel.check()?;
        Ok(out_text(out))
    }
}

struct Dec;
impl Node for Dec {
    fn run(
        &self,
        inputs: &PortMap,
        p: &serde_json::Value,
        c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        c.cancel.check()?;
        let raw = in_text(inputs, "text")?;
        if raw.len() > 8_388_608 {
            return Err(bad("Hexdump文本最多8MiB"));
        }
        match pstr(p, "style", "legacy") {
            "xxd" => return super::hexdump_xxd::decode(raw, p, c),
            "legacy" => {}
            _ => return Err(bad("Hexdump格式无效")),
        }
        let limit = number(p, "maxBytes", 16_777_216, 1, 67_108_864)? as usize;
        let mut out = Vec::new();
        for (index, line) in raw.lines().enumerate() {
            if index.is_multiple_of(256) {
                c.progress(index as f32 / raw.len().max(1) as f32);
                c.cancel.check()?;
            }
            let mut toks = line.split_whitespace().peekable();
            // Skip a leading offset token (long hex, or ends with ':').
            if let Some(first) = toks.peek() {
                let f = first.trim_end_matches(':');
                if (f.len() > 2 || first.ends_with(':')) && f.chars().all(|c| c.is_ascii_hexdigit())
                {
                    toks.next();
                }
            }
            for tok in toks {
                if tok.len() == 2 && tok.chars().all(|c| c.is_ascii_hexdigit()) {
                    out.push(u8::from_str_radix(tok, 16).unwrap());
                } else {
                    break; // reached the ASCII column
                }
            }
            if out.len() > limit {
                return Err(bad("Hexdump输出超过maxBytes"));
            }
        }
        c.progress(1.0);
        c.cancel.check()?;
        Ok(decoded(out))
    }
}

pub fn register(reg: &mut NodeRegistry) {
    for decode in [false, true] {
        let mut params = vec![help(
            ParamSpec::select("style", "格式", &["legacy", "xxd"], "legacy"),
            "legacy保留已有流程格式；xxd为两字节分组与冒号偏移，解码支持重复行。",
        )];
        if decode {
            params.extend([
                help(ParamSpec::toggle("strict","严格重建",true),"xxd模式验证半字节、重复范围和终止偏移；关闭复现上游宽松解析。严格模式修复-C中间分组和末偏移重复展开。"),
                help(ParamSpec::number("maxBytes","最大输出字节",1.0,67_108_864.0,1.0,16_777_216.0),"包含重复行展开后的总字节上限，防止小转储展开为巨大结果。"),
            ]);
        } else {
            params.extend([
                help(
                    ParamSpec::number("width", "每行字节数", 1.0, 512.0, 1.0, 16.0),
                    "仅xxd模式使用，1..512的完整整数。",
                ),
                help(
                    ParamSpec::toggle("upperCase", "十六进制大写", false),
                    "仅xxd模式改变偏移和Hex字符，ASCII列不变。",
                ),
            ]);
        }
        let mut d=document_text(desc(if decode{"from_hexdump"}else{"to_hexdump"},"numeric.bytes",if decode{"Hexdump 转字节"}else{"转 Hexdump"},BLUE,if decode{vec![t_in()]}else{vec![req("data","输入",PortType::Any)]},if decode{vec![req("text","文本",PortType::Text),opt("bytes","字节",PortType::Bytes)]}else{vec![t_out()]},params),"hexdump","显示或恢复xxd/Hexdump字节转储，保留旧ID、端口和默认格式。完整字节独立输出，xxd严格模式支持-C的ASCII边界及终止偏移，重复行受上限控制。",if decode{"00000000  48 65 6c 6c 6f                                   Hello"}else{"Hello 中文"},json!({}));
        d.cost = Cost::Heavy;
        if decode {
            reg.register(d, Arc::new(|| Arc::new(Dec)));
        } else {
            reg.register(d, Arc::new(|| Arc::new(Enc)));
        }
    }
}
