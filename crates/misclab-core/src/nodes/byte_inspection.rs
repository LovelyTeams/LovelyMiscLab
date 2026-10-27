//! Byte ranges, entropy/distribution, and bit planes. Pinned EBCTF semantics.
use super::{
    codec_support::{document_text, help},
    prelude::*,
    serial_support,
    transport_codecs::err,
};
use serde_json::{json, Value};

fn ascii(b: u8) -> char {
    if (32..=126).contains(&b) {
        b as char
    } else {
        '.'
    }
}
fn offset(s: &str) -> Result<i64, CoreError> {
    let s = s.trim();
    if let Some(h) = s.strip_prefix("0x") {
        i64::from_str_radix(h, 16)
    } else {
        s.parse()
    }
    .map_err(|_| err("偏移需要十进制整数或 0x 前缀 Hex"))
}
fn range(b: &[u8], p: &Value) -> Result<PortMap, CoreError> {
    let start = offset(pstr(p, "start", "0"))?;
    let end = if pstr(p, "end", "").is_empty() {
        b.len().saturating_sub(1) as i64
    } else {
        offset(pstr(p, "end", ""))?
    };
    let start = start.clamp(0, b.len().saturating_sub(1) as i64) as usize;
    let end = end.clamp(0, b.len().saturating_sub(1) as i64) as usize;
    let (text, slice) = if b.is_empty() || start > end {
        ("区间无效或输入为空".into(), &b[..0])
    } else {
        let slice = &b[start..=end];
        let hex = slice
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ");
        let dec = slice
            .iter()
            .map(|b| format!("{b:3}"))
            .collect::<Vec<_>>()
            .join(" ");
        let oct = slice
            .iter()
            .map(|b| format!("{b:03o}"))
            .collect::<Vec<_>>()
            .join(" ");
        let bin = slice
            .iter()
            .map(|b| format!("{b:08b}"))
            .collect::<Vec<_>>()
            .join(" ");
        let ascii: String = slice.iter().map(|b| ascii(*b)).collect();
        (format!("偏移区间: 0x{start:04x} - 0x{end:04x} ({} 字节)\nHex   : {hex}\nDec   : {dec}\nOct   : {oct}\nBin   : {bin}\nASCII : {ascii}\nUTF-8 : {}",slice.len(),serial_support::utf8(slice)),slice)
    };
    let mut out = out_text(text);
    out.insert("bytes".into(), PortValue::Bytes(slice.to_vec().into()));
    out.insert(
        "data".into(),
        PortValue::Json(
            json!({"start":start,"end":end,"length":slice.len(),"valid":!slice.is_empty()}),
        ),
    );
    Ok(out)
}
fn entropy(counts: &[usize; 256], n: usize) -> f64 {
    if n == 0 {
        return 0.0;
    }
    counts.iter().filter(|&&c| c > 0).fold(0.0, |h, &c| {
        let p = c as f64 / n as f64;
        h - p * p.log2()
    })
}
fn stats(b: &[u8], p: &Value, ctx: &NodeCtx) -> Result<PortMap, CoreError> {
    let mut counts = [0usize; 256];
    let mut printable = 0;
    for (i, &byte) in b.iter().enumerate() {
        if i.is_multiple_of(8192) {
            ctx.progress(i as f32 / b.len().max(1) as f32 * 0.4);
            ctx.cancel.check()?;
        }
        counts[byte as usize] += 1;
        if (32..=126).contains(&byte) || [9, 10, 13].contains(&byte) {
            printable += 1;
        }
    }
    let total = b.len();
    let ent = entropy(&counts, total);
    let window = pnum(p, "window", 256.0);
    let window = if !window.is_finite() || window <= 0.0 {
        256
    } else {
        window.floor().clamp(16.0, 65536.0) as usize
    };
    let show_curve = pbool(p, "showCurve", true);
    let show_hist = pbool(p, "showHist", true);
    let mut chunks = Vec::new();
    if show_curve {
        for (i, chunk) in b.chunks(window).enumerate() {
            if i.is_multiple_of(128) {
                ctx.progress(0.4 + 0.5 * (i * window) as f32 / total.max(1) as f32);
                ctx.cancel.check()?;
            }
            let mut freq = [0; 256];
            for &byte in chunk {
                freq[byte as usize] += 1;
            }
            chunks.push((i * window, entropy(&freq, chunk.len())));
        }
    }
    let mut top: Vec<_> = counts
        .iter()
        .enumerate()
        .filter(|(_, c)| **c > 0)
        .map(|(v, &c)| (v, c))
        .collect();
    top.sort_by_key(|&(v, c)| (std::cmp::Reverse(c), v));
    top.truncate(16);
    let buckets = [
        counts[..32].iter().sum::<usize>(),
        counts[32..127].iter().sum(),
        counts[127..].iter().sum(),
    ];
    let data = json!({"total":total,"printable":printable,"printableRatio":if total==0 {0.0} else {printable as f64/total as f64},"nullCount":counts[0],"entropy":ent,"counts":counts.to_vec(),"buckets":buckets,"top":top.iter().map(|&(value,count)|json!({"value":value,"count":count})).collect::<Vec<_>>(),"window":window,"chunks":chunks.iter().map(|&(offset,entropy)|json!({"offset":offset,"entropy":entropy})).collect::<Vec<_>>()});
    let pct = |n: usize| format!("{:.2}%", n as f64 / total.max(1) as f64 * 100.0);
    let mut lines = vec![
        format!("总字节数: {total}"),
        format!("可打印率: {} ({printable}/{total})", pct(printable)),
        format!("空字节(0x00): {} ({})", counts[0], pct(counts[0])),
        format!("香农熵: {ent:.4} bits/byte (随机字节≈8.0，英语≈4.0-4.5)"),
    ];
    if !chunks.is_empty() {
        let avg = chunks.iter().map(|c| c.1).sum::<f64>() / chunks.len() as f64;
        let max = chunks.iter().map(|c| c.1).fold(0.0, f64::max);
        lines.extend([String::new(),format!("滑窗熵曲线（窗口 {window} 字节 · {} 段 · 平均熵 {avg:.2} · 峰值 {max:.2} bits/byte）:",chunks.len())]);
        let width = format!("{:x}", chunks.last().unwrap().0).len().max(4);
        for row in chunks.iter().take(2048).collect::<Vec<_>>().chunks(64) {
            let bars: String = row
                .iter()
                .map(|c| {
                    ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█']
                        [(c.1 / 8.0 * 7.0).round().clamp(0.0, 7.0) as usize]
                })
                .collect();
            lines.push(format!("0x{:0width$x} {bars}", row[0].0));
        }
        if chunks.len() > 2048 {
            lines.push(format!(
                "…（共 {} 段，已显示前 2048 段，可增大窗口看全貌）",
                chunks.len()
            ));
        }
        lines.push("（接近 8 = 加密/压缩，低 = 文本/填充；看竖块突变处即结构边界）".into());
    }
    if show_hist {
        let max = *counts.iter().max().unwrap();
        lines.extend([
            String::new(),
            "字节分布图（16×16 密度网格，行=高 4 位，列=低 4 位）:".into(),
        ]);
        for row in 0..16 {
            let cells: String = counts[row * 16..(row + 1) * 16]
                .iter()
                .map(|&c| {
                    if c == 0 {
                        ' '
                    } else if max <= 1 {
                        '·'
                    } else {
                        ['·', '░', '▒', '▓', '█'][((c as f64).ln() / (max as f64).ln() * 4.0)
                            .floor()
                            .clamp(0.0, 4.0)
                            as usize]
                    }
                })
                .collect();
            lines.push(format!("0x{:02x} {cells}", row * 16));
        }
        lines.push(format!(
            "图例: 空=0 次 · ░▒▓█ 逐档升高（对数分档，█=最高 {max} 次）"
        ));
    }
    lines.extend([String::new(), "字节值分布桶:".into()]);
    for (label, c) in [
        "0x00-0x1F (控制字符)",
        "0x20-0x7E (可打印 ASCII)",
        "0x7F-0xFF (高字节/扩展)",
    ]
    .iter()
    .zip(buckets)
    {
        lines.push(format!("  {label}: {c} ({})", pct(c)));
    }
    lines.extend([String::new(), format!("高频字节 (top {}):", top.len())]);
    for (v, c) in top {
        lines.push(format!(
            "  0x{v:02x} ({v:3} {}) × {c} ({})",
            if (32..=126).contains(&v) {
                format!("'{}'", ascii(v as u8))
            } else {
                "(非可打印)".into()
            },
            pct(c)
        ));
    }
    let mut out = out_text(if total == 0 {
        "(空输入，0 字节)".into()
    } else {
        lines.join("\n")
    });
    out.insert("data".into(), PortValue::Json(data));
    Ok(out)
}
fn planes(b: &[u8], p: &Value, ctx: &NodeCtx) -> Result<PortMap, CoreError> {
    let plane = pstr(p, "plane", "all");
    let keys: Vec<usize> = if plane == "all" {
        (0..8).rev().collect()
    } else {
        let k = plane
            .parse::<usize>()
            .map_err(|_| err("位平面须为 all 或 0–7"))?;
        if k > 7 {
            return Err(err("位平面须为 0–7"));
        }
        vec![k]
    };
    let mut lines = Vec::new();
    let mut data = serde_json::Map::new();
    for (n, &k) in keys.iter().enumerate() {
        let mut bits = String::with_capacity(b.len());
        for (i, &byte) in b.iter().enumerate() {
            if i.is_multiple_of(8192) {
                ctx.progress((n as f32 + i as f32 / b.len().max(1) as f32) / keys.len() as f32);
                ctx.cancel.check()?;
            }
            bits.push(if byte >> k & 1 == 1 { '1' } else { '0' });
        }
        lines.push(if plane == "all" {
            format!(
                "位 {k}{}: {bits}",
                match k {
                    7 => " (MSB)",
                    0 => " (LSB)",
                    _ => "",
                }
            )
        } else {
            bits.clone()
        });
        data.insert(k.to_string(), json!(bits));
    }
    let mut out = out_text(lines.join("\n"));
    out.insert("data".into(), PortValue::Json(data.into()));
    Ok(out)
}
#[derive(Clone, Copy)]
enum Kind {
    Range,
    Stats,
    Planes,
}
struct Inspect(Kind);
impl Node for Inspect {
    fn run(
        &self,
        inputs: &PortMap,
        params: &Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let b = if matches!(self.0, Kind::Planes) {
            let b = in_bytes(inputs, "data")?;
            if b.len() > serial_support::MAX_INPUT {
                return Err(err("位平面输入最多 1 MiB"));
            }
            b
        } else {
            serial_support::input(inputs, params)?
        };
        let out = match self.0 {
            Kind::Range => range(&b, params)?,
            Kind::Stats => stats(&b, params, ctx)?,
            Kind::Planes => planes(&b, params, ctx)?,
        };
        ctx.cancel.check()?;
        Ok(out)
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for (kind,id,upstream,title,category,description,sample) in [
        (Kind::Range,"hex_range","hexRange","Hex 区间提取","numeric.bits","按包含结束偏移的区间提取字节，展示 Hex/Dec/Oct/Bin/ASCII/UTF-8，bytes 可继续连线。偏移越界夹到边界；解析输入最多 1 MiB。","00 ff e4 b8 ad"),
        (Kind::Stats,"hex_stats","hexStats","字节分布统计","analysis.statistics","256 桶频数、可打印率、全局/分块香农熵、16×16 密度图及 top 16。文本曲线最多显示 2048 段，JSON 保留全部窗口；输入最多 1 MiB。","00 00 01 41 42 43"),
        (Kind::Planes,"bit_plane_extract","bitPlaneExtract","位平面提取","numeric.bits","按每字节第 0–7 位输出比特串，all 从 MSB 到 LSB 展示全部平面。文本按 UTF-8，也接受原始字节；有损单向，输入最多 1 MiB。","Hello, 中文!")
    ] {
        let mut params=if matches!(kind,Kind::Planes) {vec![help(ParamSpec::select("plane","位平面",&["all","7","6","5","4","3","2","1","0"],"all"),"all 展示全部 8 位；0 是 LSB，7 是 MSB。每个输入字节贡献一个比特。")]} else {vec![help(ParamSpec::select("inputEnc","输入编码",&["auto","hex","base64","utf8"],"auto"),"文本 auto 优先 Hex，再 Base64/URL-safe，最后 UTF-8；原始 Bytes 直接使用，忽略此参数。") ]};
        match kind {
            Kind::Range=>params.extend([help(ParamSpec::text("start","起始偏移","0",false),"十进制或 0x Hex 偏移，负值按 0。"),help(ParamSpec::text("end","结束偏移（含）","",false),"包含结束位置，留空到末尾。超出范围夹到末尾；起点在终点之后输出空区间。")]),
            Kind::Stats=>params.extend([help(ParamSpec::number("window","熵窗口（字节）",16.0,65536.0,1.0,256.0),"不重叠分块窗口，尾部不足一窗仍计算；有效范围 16–65536。"),help(ParamSpec::toggle("showCurve","显示熵曲线",true),"生成分块熵曲线与完整 JSON 窗口序列。"),help(ParamSpec::toggle("showHist","显示密度图",true),"16×16 对数密度文本图；关闭不影响 JSON 中的 256 桶原始计数。")]),Kind::Planes=>{}
        }
        let mut outputs=vec![t_out(),req("data","结构化结果",PortType::Json)];
        if matches!(kind,Kind::Range) {outputs.push(req("bytes","区间字节",PortType::Bytes));}
        let mut d=document_text(desc(id,category,title,CYAN,vec![req("data","文本或原始字节",PortType::Any)],outputs,params),upstream,description,sample,json!({}));
        d.cost=Cost::Medium;
        d.aliases.extend(match kind {Kind::Range=>vec!["byte range","字节切片"],Kind::Stats=>vec!["byte histogram","entropy curve","熵曲线"],Kind::Planes=>vec!["bit plane","LSB","MSB"]}.into_iter().map(String::from));
        reg.register(d,Arc::new(move||Arc::new(Inspect(kind))));
    }
}
