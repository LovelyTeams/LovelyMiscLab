//! Deterministic canonical Huffman codes and user prefix tables, MSB-first.
use super::{
    codec_support::{document_text, help},
    prelude::*,
    transport_codecs::err,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const INPUT_LIMIT: usize = 1024 * 1024;
const OUTPUT_LIMIT: usize = 8 * 1024 * 1024;
type Codes = BTreeMap<u8, String>;
fn label(b: u8) -> String {
    if (0x21..=0x7e).contains(&b) {
        (b as char).to_string()
    } else {
        format!("\\x{b:02x}")
    }
}
fn symbol(token: &str) -> Result<u8, CoreError> {
    let token = if token.len() >= 2
        && ((token.starts_with('"') && token.ends_with('"'))
            || (token.starts_with('\'') && token.ends_with('\'')))
    {
        &token[1..token.len() - 1]
    } else {
        token
    };
    if let Some(h) = token
        .strip_prefix("\\x")
        .filter(|s| (1..=2).contains(&s.len()))
        .or_else(|| token.strip_prefix("0x").filter(|s| s.len() == 2))
    {
        return u8::from_str_radix(h, 16).map_err(|_| err("符号 Hex 无效"));
    }
    let mut chars = token.chars();
    if let Some(c) = chars.next() {
        if chars.next().is_none() && (c as u32) <= 255 {
            return Ok(c as u8);
        }
    }
    Err(err(
        "Huffman 符号必须是单字节值；非 ASCII 文本请逐个 UTF-8 字节使用 \\xNN",
    ))
}
fn table_lines(raw: &str, weights: bool) -> Result<Vec<(u8, String)>, CoreError> {
    if raw.len() > 64 * 1024 {
        return Err(err("权重/码表最多 64 KiB"));
    }
    let regex = regex::Regex::new(if weights {
        r#"^(\S+|"[^"]*"|'[^']*')\s*[\s:,=]\s*(-?\d+(?:\.\d+)?)$"#
    } else {
        r#"^(\S+|"[^"]*"|'[^']*')\s*[\s=]\s*([01]+)$"#
    })
    .unwrap();
    let mut out = Vec::new();
    for (i, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let c = regex
            .captures(line)
            .ok_or_else(|| err(format!("权重/码表第 {} 行格式错误", i + 1)))?;
        out.push((symbol(&c[1])?, c[2].to_owned()));
    }
    Ok(out)
}
fn weights(raw: &str) -> Result<BTreeMap<u8, f64>, CoreError> {
    let mut out = BTreeMap::new();
    for (b, s) in table_lines(raw, true)? {
        let f = s.parse::<f64>().map_err(|_| err("权重无效"))?;
        if !f.is_finite() || f < 0.0 {
            return Err(err("权重必须为有限非负数"));
        }
        // The upstream keeps the last value for duplicate weight symbols.
        out.insert(b, f);
    }
    Ok(out)
}
fn validate(codes: &Codes) -> Result<(), CoreError> {
    if codes.is_empty() {
        return Err(err("码表不能为空"));
    }
    let mut sorted = codes.values().collect::<Vec<_>>();
    sorted.sort();
    for pair in sorted.windows(2) {
        if pair[1].starts_with(pair[0]) {
            return Err(err("码表有重复码字或前缀冲突"));
        }
    }
    Ok(())
}
fn table(raw: &str) -> Result<Codes, CoreError> {
    let mut out = Codes::new();
    for (b, code) in table_lines(raw, false)? {
        if code.len() > 4096 {
            return Err(err("单个 Huffman 码字最多 4096 位"));
        }
        if out.insert(b, code).is_some() {
            return Err(err("码表符号重复"));
        }
    }
    validate(&out)?;
    Ok(out)
}
fn canonical(freq: BTreeMap<u8, f64>) -> Result<Codes, CoreError> {
    if freq.is_empty() {
        return Err(err("频率档需要非空权重表"));
    }
    // At most 256 leaves: a sorted forest is small, deterministic, and bounded.
    let mut forest = freq
        .into_iter()
        .map(|(b, f)| (f, b, vec![(b, 0usize)]))
        .collect::<Vec<_>>();
    while forest.len() > 1 {
        forest.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let (af, ab, mut leaves) = forest.remove(0);
        let (bf, bb, other) = forest.remove(0);
        if !(af + bf).is_finite() {
            return Err(err("累计 Huffman 权重溢出"));
        }
        leaves.extend(other);
        for (_, len) in &mut leaves {
            *len += 1;
        }
        forest.push((af + bf, ab.min(bb), leaves));
    }
    let mut order = forest.remove(0).2;
    for (_, len) in &mut order {
        *len = (*len).max(1);
    }
    order.sort_by_key(|&(b, len)| (len, b));
    let mut codes = Codes::new();
    let mut bits = vec![b'0'; order[0].1];
    for (i, (b, len)) in order.into_iter().enumerate() {
        if i > 0 {
            let mut carry = true;
            for bit in bits.iter_mut().rev() {
                if *bit == b'0' {
                    *bit = b'1';
                    carry = false;
                    break;
                }
                *bit = b'0';
            }
            if carry {
                return Err(err("Huffman 码长超出前缀容量"));
            }
            bits.resize(len, b'0');
        }
        codes.insert(b, String::from_utf8(bits.clone()).unwrap());
    }
    validate(&codes)?;
    Ok(codes)
}
fn code_table(codes: &Codes) -> String {
    codes
        .iter()
        .map(|(b, c)| format!("\\x{b:02x} {c}"))
        .collect::<Vec<_>>()
        .join("\n")
}
fn progress(ctx: &NodeCtx, index: usize, total: usize) -> Result<(), CoreError> {
    if index.is_multiple_of(2048) {
        ctx.progress(index as f32 / total.max(1) as f32);
        ctx.cancel.check()?;
    }
    Ok(())
}
fn encode(bytes: &[u8], p: &Value, ctx: &NodeCtx) -> Result<PortMap, CoreError> {
    if bytes.is_empty() {
        return Err(err("Huffman 编码输入为空"));
    }
    if bytes.len() > INPUT_LIMIT {
        return Err(err("Huffman 编码输入最多 1 MiB"));
    }
    let mode = pstr(p, "mode", "freq");
    let codes = match mode {
        "table" => table(pstr(p, "table", ""))?,
        "freq" => {
            let mut freq = weights(pstr(p, "weights", ""))?;
            if freq.is_empty() {
                for (i, &b) in bytes.iter().enumerate() {
                    progress(ctx, i, bytes.len())?;
                    *freq.entry(b).or_default() += 1.0;
                }
            }
            canonical(freq)?
        }
        _ => return Err(err("Huffman 模式必须为 freq / table")),
    };
    let mut packed = Vec::new();
    let mut bitlen = 0usize;
    for (i, b) in bytes.iter().enumerate() {
        progress(ctx, i, bytes.len())?;
        let code = codes
            .get(b)
            .ok_or_else(|| err(format!("码表缺少符号 {}", label(*b))))?;
        if bitlen + code.len() > OUTPUT_LIMIT * 8 {
            return Err(err("Huffman 位流超过 8 MiB"));
        }
        for bit in code.bytes() {
            if bitlen.is_multiple_of(8) {
                packed.push(0);
            }
            if bit == b'1' {
                *packed.last_mut().unwrap() |= 0x80 >> (bitlen % 8);
            }
            bitlen += 1;
        }
    }
    let mut order = codes.iter().collect::<Vec<_>>();
    order.sort_by_key(|&(b, c)| (c.len(), *b));
    let mut lines = vec![
        format!(
            "符号数: {}，输入 {} 字节 → {} bit（{} 字节 + {} bit 填充）",
            codes.len(),
            bytes.len(),
            bitlen,
            packed.len(),
            (8 - bitlen % 8) % 8
        ),
        String::new(),
        format!(
            "--- 码表（{}）---",
            if mode == "table" {
                "用户码表"
            } else {
                "canonical（频率档确定性规则）"
            }
        ),
    ];
    lines.extend(
        order
            .into_iter()
            .map(|(&b, c)| format!("{:<4} → {c}  ({} bit)", label(b), c.len())),
    );
    let hex = hex::encode(&packed);
    lines.push(String::new());
    lines.push(format!("bitLen: {bitlen}"));
    lines.push(format!(
        "hex: {}{}",
        &hex[..hex.len().min(4096)],
        if hex.len() > 4096 { "…(截断)" } else { "" }
    ));
    if bitlen <= 256 {
        lines.push(format!(
            "bits: {}",
            (0..bitlen)
                .map(|i| if packed[i / 8] & (0x80 >> (i % 8)) != 0 {
                    '1'
                } else {
                    '0'
                })
                .collect::<String>()
        ));
    }
    if mode == "freq" {
        lines.push(String::new());
        lines.push(format!("解码：同 op 切 decode，粘贴 hex + 填 bitLen={bitlen} + 提供「与编码一致」的权重表（或输入统计口径相同的原文符号集）即可重建同一张码表。"));
    }
    let table = code_table(&codes);
    let mut out = out_text(lines.join("\n"));
    out.insert("bytes".into(), PortValue::Bytes(packed.into()));
    out.insert("hex".into(), PortValue::Text(hex.clone()));
    out.insert("bit_length".into(), PortValue::Number(bitlen as f64));
    out.insert("code_table".into(), PortValue::Text(table.clone()));
    out.insert(
        "packet".into(),
        PortValue::Json(json!({"format":"huffman-msb","hex":hex,"bitLen":bitlen,"table":table})),
    );
    ctx.cancel.check()?;
    Ok(out)
}
fn bit_length(value: &Value) -> Result<usize, CoreError> {
    let n = value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse::<f64>().ok()))
        .ok_or_else(|| err("bitLen 须为非负整数"))?;
    if !n.is_finite() || n < 0.0 || n.fract() != 0.0 || n > OUTPUT_LIMIT as f64 * 8.0 {
        return Err(err("bitLen 须为 0–67108864 的整数"));
    }
    Ok(n as usize)
}
#[derive(Default)]
struct Branch {
    next: [Option<usize>; 2],
    symbol: Option<u8>,
}
fn decode(inputs: &PortMap, p: &Value, ctx: &NodeCtx) -> Result<PortMap, CoreError> {
    let value = inputs.get("data").ok_or_else(|| err("缺少 Huffman 位流"))?;
    let mut table_override = None;
    let mut length_override = None;
    let bytes = match value {
        PortValue::Bytes(b) => {
            if b.len() > OUTPUT_LIMIT {
                return Err(err("Huffman 位流最多 8 MiB"));
            }
            b.to_vec()
        }
        PortValue::Text(s) => {
            if s.len() > OUTPUT_LIMIT * 3 {
                return Err(err("Huffman Hex 输入过长"));
            }
            // Preserve upstream separators / non-Hex filtering for text input.
            let s = s
                .chars()
                .filter(char::is_ascii_hexdigit)
                .collect::<String>();
            hex::decode(s).map_err(|_| err("Hex 输入需要偶数个字符"))?
        }
        PortValue::Json(v) => {
            if v["format"] != "huffman-msb" {
                return Err(err("需要 huffman-msb 数据包"));
            }
            let s = v["hex"].as_str().ok_or_else(|| err("数据包缺少 hex"))?;
            if s.len() > OUTPUT_LIMIT * 2 {
                return Err(err("Huffman 数据包过长"));
            }
            table_override = Some(v["table"].as_str().ok_or_else(|| err("数据包缺少码表"))?);
            length_override = Some(bit_length(&v["bitLen"])?);
            hex::decode(s).map_err(|_| err("数据包 hex 无效"))?
        }
        _ => {
            return Err(err(
                "Huffman 解码输入需要 Hex 文本、原始字节或编码节点的数据包",
            ))
        }
    };
    if bytes.is_empty() || bytes.len() > OUTPUT_LIMIT {
        return Err(err("Huffman 位流需要 1 字节至 8 MiB"));
    }
    if let Some(v) = inputs.get("code_table") {
        table_override = Some(v.as_text()?);
    }
    if let Some(v) = inputs.get("bit_length") {
        length_override = Some(match v {
            PortValue::Number(n) => bit_length(&json!(n))?,
            _ => return Err(err("真实位数输入须为数值")),
        });
    }
    let bits = match length_override {
        Some(n) => n,
        None => bit_length(p.get("bitLen").unwrap_or(&json!(0)))?,
    };
    let bits = if bits == 0 { bytes.len() * 8 } else { bits };
    if bits > bytes.len() * 8 {
        return Err(err("bitLen 超出位流字节容量"));
    }
    let codes = if let Some(raw) = table_override {
        table(raw)?
    } else {
        match pstr(p, "mode", "freq") {
            "table" => table(pstr(p, "table", ""))?,
            "freq" => canonical(weights(pstr(p, "weights", ""))?)?,
            _ => return Err(err("Huffman 模式必须为 freq / table")),
        }
    };
    let mut trie = vec![Branch::default()];
    for (&symbol, code) in &codes {
        let mut at = 0;
        for c in code.bytes() {
            let bit = usize::from(c == b'1');
            at = if let Some(next) = trie[at].next[bit] {
                next
            } else {
                let next = trie.len();
                trie.push(Branch::default());
                trie[at].next[bit] = Some(next);
                next
            };
        }
        trie[at].symbol = Some(symbol);
    }
    let mut out = Vec::new();
    let mut at = 0;
    for i in 0..bits {
        progress(ctx, i, bits)?;
        let bit = usize::from(bytes[i / 8] & (0x80 >> (i % 8)) != 0);
        at = trie[at].next[bit]
            .ok_or_else(|| err(format!("第 {} 位不是码表中的有效前缀", i + 1)))?;
        if let Some(b) = trie[at].symbol {
            if out.len() >= OUTPUT_LIMIT {
                return Err(err("Huffman 解码结果超过 8 MiB"));
            }
            out.push(b);
            at = 0;
        }
    }
    if at != 0 {
        return Err(err("Huffman 位流截断：检查 bitLen 和码表"));
    }
    let utf8 = std::str::from_utf8(&out).ok();
    let is_text = utf8.is_some_and(|s| {
        s.chars()
            .filter(|&c| c < ' ' && !matches!(c, '\n' | '\r' | '\t'))
            .count() as f64
            / (s.encode_utf16().count().max(1) as f64)
            < 0.1
    });
    let hex = hex::encode(&out);
    let preview = if is_text {
        utf8.unwrap().to_owned()
    } else {
        format!(
            "hex: {}{}",
            &hex[..hex.len().min(4096)],
            if hex.len() > 4096 { "…" } else { "" }
        )
    };
    let mut ports = out_text(format!("解码: {} 字节（{bits} bit）\n{preview}", out.len()));
    ports.insert(
        "decoded".into(),
        PortValue::Text(String::from_utf8_lossy(&out).into_owned()),
    );
    ports.insert("json".into(),PortValue::Json(json!({"byteLength":out.len(),"bitLen":bits,"symbols":codes.len(),"validUtf8":utf8.is_some(),"textPreview":is_text})));
    ports.insert("bytes".into(), PortValue::Bytes(out.into()));
    ctx.cancel.check()?;
    Ok(ports)
}
struct Huffman(bool);
impl Node for Huffman {
    fn run(&self, inputs: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        if self.0 {
            decode(inputs, p, ctx)
        } else {
            encode(&in_bytes(inputs, "data")?, p, ctx)
        }
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for decode in [false, true] {
        let mut params=vec![
            help(ParamSpec::select("mode","码表来源",&["freq","table"],"freq"),"freq 为确定性 canonical 编码，按 (频率, 最小字节) 合树；table 使用前缀码表。数据包或码表连线优先。"),
            help(ParamSpec::text("weights","权重表","",true),"每行 符号 频率，例如 a 5；支持 \\xNN/0xNN。编码留空自动统计 UTF-8/原始字节，解码需与编码一致。"),
            help(ParamSpec::text("table","用户码表","",true),"每行 符号 01码字；例如 a 0、b 10、c 11。拒绝重复、非前缀表；字节符号可写 \\xNN。# 开头是注释。"),
        ];
        if decode {
            params.push(help(ParamSpec::number("bitLen","真实位数",0.0,(OUTPUT_LIMIT*8) as f64,1.0,0.0),"0 表示使用全部字节的位数（含末尾填充）；正确还原应使用编码输出 bit_length 或直接连接 packet。"));
        }
        let mut inputs = vec![req(
            "data",
            if decode {
                "Hex / 字节 / 数据包"
            } else {
                "文本或原始字节"
            },
            PortType::Any,
        )];
        if decode {
            inputs.extend([
                opt("code_table", "码表（可选）", PortType::Text),
                opt("bit_length", "真实位数（可选）", PortType::Number),
            ]);
        }
        let outputs = if decode {
            vec![
                t_out(),
                req("bytes", "完整解码字节", PortType::Bytes),
                req("decoded", "解码文本", PortType::Text),
                req("json", "解码详情", PortType::Json),
            ]
        } else {
            vec![
                t_out(),
                req("bytes", "MSB 位流", PortType::Bytes),
                req("hex", "完整 Hex", PortType::Text),
                req("bit_length", "真实位数", PortType::Number),
                req("code_table", "可复用码表", PortType::Text),
                req("packet", "位流及码表数据包", PortType::Json),
            ]
        };
        let mut d=document_text(desc(if decode {"huffman_decode"} else {"huffman_encode"},"archive.compression",if decode {"Huffman 哈夫曼解码"} else {"Huffman 哈夫曼编码"},CYAN,inputs,outputs,params),"huffmanCodec","通用 MSB-first Huffman，不是 DEFLATE 文件格式。报告兼容上游，独立端口保留完整数据；packet 可直接接解码 data，或同时连接 bytes / code_table / bit_length。权重/码表在侧栏编辑。编码最多 1 MiB，位流/解码结果最多 8 MiB。",if decode {"60"} else {"ABBA"},if decode {json!({"weights":"A 2\nB 2","bitLen":4})} else {json!({})});
        d.aliases
            .extend(["Huffman", "哈夫曼", "前缀码", "canonical"].map(String::from));
        d.cost = Cost::Medium;
        d.presentation.inline_params = if decode {
            vec!["mode".into(), "bitLen".into()]
        } else {
            vec!["mode".into()]
        };
        reg.register(d, Arc::new(move || Arc::new(Huffman(decode))));
    }
}
