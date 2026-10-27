//! General even-parity SEC Hamming codes. This is not SECDED.
use super::{
    codec_support::{document_text, help},
    numeric_support,
    prelude::*,
    transport_codecs::err,
};
use serde_json::{json, Value};
const LIMIT: usize = 1024 * 1024;
struct Hamming(bool);
impl Node for Hamming {
    fn run(&self, inputs: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let text = in_text(inputs, "text")?;
        if text.len() > LIMIT {
            return Err(err("海明码输入最多 1 MiB"));
        }
        let k = if p.get("k").is_none_or(|v| v.is_null() || v == "") {
            4
        } else {
            numeric_support::number(p, "k", 4, 1, 65536)? as usize
        };
        let mut r = 0;
        while (1usize << r) < k + r + 1 {
            r += 1;
        }
        let n = k + r;
        let bits = text
            .bytes()
            .filter(|b| *b == b'0' || *b == b'1')
            .collect::<Vec<_>>();
        let size = if self.0 { n } else { k };
        let mut out = String::new();
        let mut corrections = Vec::new();
        let mut uncorrectable = Vec::new();
        let mut count = 0;
        for (block, data) in bits.chunks(size).enumerate() {
            ctx.progress((block * size) as f32 / bits.len().max(1) as f32);
            ctx.cancel.check()?;
            if self.0 && data.len() < n {
                break;
            }
            let mut code = vec![false; n + 1];
            if self.0 {
                let mut syndrome = 0;
                for (i, &b) in data.iter().enumerate() {
                    code[i + 1] = b == b'1';
                    if code[i + 1] {
                        syndrome ^= i + 1;
                    }
                }
                if syndrome != 0 && syndrome <= n {
                    code[syndrome] = !code[syndrome];
                    corrections.push(json!({"block":block,"position":syndrome}));
                } else if syndrome > n {
                    uncorrectable.push(block);
                }
                for (i, &b) in code.iter().enumerate().skip(1) {
                    if !i.is_power_of_two() {
                        out.push(if b { '1' } else { '0' });
                    }
                }
            } else {
                let mut di = 0;
                let mut syndrome = 0;
                for (i, b) in code.iter_mut().enumerate().skip(1) {
                    if !i.is_power_of_two() {
                        *b = data.get(di) == Some(&b'1');
                        di += 1;
                        if *b {
                            syndrome ^= i;
                        }
                    }
                }
                for i in 0..r {
                    let pos = 1usize << i;
                    code[pos] = syndrome & pos != 0;
                }
                out.extend(code.iter().skip(1).map(|b| if *b { '1' } else { '0' }));
            }
            count += 1;
        }
        let mut ports = out_text(out);
        ports.insert("json".into(),PortValue::Json(json!({"dataBits":k,"parityBits":r,"blockBits":n,"blocks":count,"corrections":corrections,"uncorrectableBlocks":uncorrectable,"paddingBits":if self.0 || bits.is_empty() {0} else {(k-bits.len()%k)%k},"ignoredTrailingBits":if self.0 {bits.len()%n} else {0},"singleErrorCorrectionOnly":true})));
        ctx.cancel.check()?;
        Ok(ports)
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for decode in [false, true] {
        let mut d=document_text(desc(if decode {"hamming_code_decode"} else {"hamming_code_encode"},"numeric.bits",if decode {"海明码 Hamming 纠错解码"} else {"海明码 Hamming 编码"},CYAN,vec![t_in()],vec![t_out(),req("json","分块与纠错详情",PortType::Json)],vec![help(ParamSpec::number("k","每块数据位 k",1.0,65536.0,1.0,4.0),"4 为 (7,4)，11 为 (15,11)；编码末块补 0，解码忽略不足一块的尾位，详情记录数量。")]),"hammingCode","通用偶校验单比特纠错海明码（SEC）；输入剔除非 0/1 字符。每块仅保证纠正 1 位错误，多位错误可能误纠正；不具备 SECDED 双错检测。输入最多 1 MiB。",if decode {"0110011"} else {"1011"},json!({}));
        d.aliases
            .extend(["Hamming Code", "汉明码", "单比特纠错", "SEC"].map(String::from));
        reg.register(d, Arc::new(move || Arc::new(Hamming(decode))));
    }
}
