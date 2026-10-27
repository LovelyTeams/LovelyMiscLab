//! Byte/line comparison with the pinned equal-length/LCS/fallback contract.
use super::{
    codec_support::{document_text, help},
    numeric_support,
    prelude::*,
    transport_codecs::err,
};
use base64::Engine as _;
use serde_json::{json, Value};
const LIMIT: usize = 256 * 1024;
const REPORT_LIMIT: usize = 1024 * 1024;
const JSON_RUN_LIMIT: usize = 16384;
#[derive(Clone, Copy)]
enum Op {
    Equal(usize, usize),
    Delete(usize),
    Insert(usize),
}
impl Op {
    fn a(self) -> Option<usize> {
        match self {
            Self::Equal(i, _) | Self::Delete(i) => Some(i),
            _ => None,
        }
    }
    fn b(self) -> Option<usize> {
        match self {
            Self::Equal(_, j) | Self::Insert(j) => Some(j),
            _ => None,
        }
    }
    fn kind(self) -> &'static str {
        match self {
            Self::Equal(..) => "eq",
            Self::Delete(_) => "del",
            Self::Insert(_) => "ins",
        }
    }
}
fn seq<T: Eq>(a: &[T], b: &[T], ctx: &NodeCtx) -> Result<(Vec<Op>, &'static str), CoreError> {
    let (n, m) = (a.len(), b.len());
    let mut out = Vec::with_capacity(n + m);
    if n == m {
        for i in 0..n {
            if i.is_multiple_of(8192) {
                ctx.progress(0.6 * i as f32 / n.max(1) as f32);
                ctx.cancel.check()?;
            }
            if a[i] == b[i] {
                out.push(Op::Equal(i, i));
            } else {
                out.extend([Op::Delete(i), Op::Insert(i)]);
            }
        }
        return Ok((out, "等长逐位"));
    }
    if n == 0 || m == 0 {
        out.extend((0..n).map(Op::Delete));
        out.extend((0..m).map(Op::Insert));
        return Ok((out, if n == 0 { "A 空" } else { "B 空" }));
    }
    if n * m > 2_000_000 {
        let mut pre = 0;
        while pre < n && pre < m && a[pre] == b[pre] {
            if pre.is_multiple_of(8192) {
                ctx.cancel.check()?;
            }
            pre += 1;
        }
        let mut suf = 0;
        while suf < n - pre && suf < m - pre && a[n - 1 - suf] == b[m - 1 - suf] {
            if suf.is_multiple_of(8192) {
                ctx.cancel.check()?;
            }
            suf += 1;
        }
        out.extend((0..pre).map(|i| Op::Equal(i, i)));
        out.extend((pre..n - suf).map(Op::Delete));
        out.extend((pre..m - suf).map(Op::Insert));
        out.extend((0..suf).map(|i| Op::Equal(n - suf + i, m - suf + i)));
        ctx.progress(0.6);
        ctx.cancel.check()?;
        return Ok((out, "朴素（输入过大）"));
    }
    let stride = m + 1;
    let mut dp = vec![0u32; (n + 1) * stride];
    for i in (0..n).rev() {
        if i.is_multiple_of(32) {
            ctx.progress(0.6 * (n - i) as f32 / n as f32);
            ctx.cancel.check()?;
        }
        for j in (0..m).rev() {
            dp[i * stride + j] = if a[i] == b[j] {
                dp[(i + 1) * stride + j + 1] + 1
            } else {
                dp[(i + 1) * stride + j].max(dp[i * stride + j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if (i + j).is_multiple_of(8192) {
            ctx.cancel.check()?;
        }
        if a[i] == b[j] {
            out.push(Op::Equal(i, j));
            i += 1;
            j += 1;
        } else if dp[(i + 1) * stride + j] >= dp[i * stride + j + 1] {
            out.push(Op::Delete(i));
            i += 1;
        } else {
            out.push(Op::Insert(j));
            j += 1;
        }
    }
    out.extend((i..n).map(Op::Delete));
    out.extend((j..m).map(Op::Insert));
    Ok((out, "LCS"))
}
fn hunks(ops: &[Op], context: usize, ctx: &NodeCtx) -> Result<Vec<(usize, usize)>, CoreError> {
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        if i.is_multiple_of(8192) {
            ctx.cancel.check()?;
        }
        if matches!(op, Op::Equal(..)) {
            continue;
        }
        let start = i.saturating_sub(context);
        let end = (i + context + 1).min(ops.len());
        if let Some(last) = ranges.last_mut() {
            if start <= last.1 {
                last.1 = last.1.max(end);
                continue;
            }
        }
        ranges.push((start, end));
    }
    Ok(ranges)
}
fn extent(ops: &[Op], a: bool) -> Option<[usize; 2]> {
    let mut indices = ops.iter().filter_map(|op| if a { op.a() } else { op.b() });
    let first = indices.next()?;
    Some([first, indices.next_back().unwrap_or(first) + 1])
}
fn offset(n: usize) -> String {
    format!("0x{n:X}")
}
fn range_text(range: Option<[usize; 2]>, line: bool) -> String {
    match range {
        None => "∅".into(),
        Some([a, b]) => {
            if line {
                format!("L{}-L{}", a + 1, b)
            } else {
                format!("{}-{}", offset(a), offset(b - 1))
            }
        }
    }
}
fn byte_preview(bytes: &[u8]) -> String {
    let mut h = hex::encode(&bytes[..bytes.len().min(64)]);
    if bytes.len() > 64 {
        h.push_str(&format!("…({}B)", bytes.len()));
    }
    let mut ascii = bytes
        .iter()
        .take(16)
        .map(|&b| {
            if (0x20..0x7f).contains(&b) {
                b as char
            } else {
                '·'
            }
        })
        .collect::<String>();
    if bytes.len() > 16 {
        ascii.push('…');
    }
    format!("({}B)  {h}  '{ascii}'", bytes.len())
}
struct Report {
    text: String,
    truncated: bool,
}
impl Report {
    fn push(&mut self, line: &str) {
        if self.truncated {
            return;
        }
        if self.text.len() + line.len() + 1 > REPORT_LIMIT {
            self.text
                .push_str("\n…报告超过 1 MiB，显示已截断；统计与完整输入仍保留。");
            self.truncated = true;
            return;
        }
        if !self.text.is_empty() {
            self.text.push('\n');
        }
        self.text.push_str(line);
    }
}
enum Data {
    Bytes(Vec<u8>),
    Lines(Vec<String>),
}
impl Data {
    fn len(&self) -> usize {
        match self {
            Self::Bytes(b) => b.len(),
            Self::Lines(l) => l.len(),
        }
    }
}
fn render(
    a: &Data,
    b: &Data,
    ops: &[Op],
    context: usize,
    kind: &str,
    enc: &str,
    ctx: &NodeCtx,
) -> Result<PortMap, CoreError> {
    let line = matches!(a, Data::Lines(_));
    let mut counts = [0usize; 3];
    for op in ops {
        counts[match op {
            Op::Equal(..) => 0,
            Op::Delete(_) => 1,
            Op::Insert(_) => 2,
        }] += 1;
    }
    let ranges = hunks(ops, context, ctx)?;
    let mut report = Report {
        text: String::new(),
        truncated: false,
    };
    report.push(&if line {
        "差异对比 · line 模式".into()
    } else {
        format!(
            "差异对比 · byte 模式{}",
            if enc == "auto" {
                String::new()
            } else {
                format!("（编码: {enc}）")
            }
        )
    });
    report.push(&format!(
        "A = {} {} | B = {} {} | 策略: {kind}",
        a.len(),
        if line { "行" } else { "字节" },
        b.len(),
        if line { "行" } else { "字节" }
    ));
    report.push(&format!(
        "相同 {} | 仅 A {} | 仅 B {}{}",
        counts[0],
        counts[1],
        counts[2],
        if line { "" } else { " | 差异块计数见下" }
    ));
    report.push("");
    let equal = counts[1] == 0 && counts[2] == 0;
    let mut details = Vec::new();
    let mut stored_runs = 0usize;
    let mut json_truncated = false;
    if equal {
        report.push("✓ 两段输入完全一致，无差异。");
    } else {
        report.push(&format!("差异块: {}", ranges.len()));
        report.push("");
        for (index, &(start, end)) in ranges.iter().enumerate() {
            ctx.progress(0.6 + 0.4 * index as f32 / ranges.len() as f32);
            ctx.cancel.check()?;
            let hunk = &ops[start..end];
            let ar = extent(hunk, true);
            let br = extent(hunk, false);
            report.push(&format!(
                "[{}] @@ A:{} | B:{} @@",
                index + 1,
                range_text(ar, line),
                range_text(br, line)
            ));
            let mut at = 0;
            let mut runs = Vec::new();
            while at < hunk.len() {
                if at.is_multiple_of(8192) {
                    ctx.cancel.check()?;
                }
                let mut next = at + 1;
                while next < hunk.len() && hunk[next].kind() == hunk[at].kind() {
                    next += 1;
                }
                let run = &hunk[at..next];
                let op = run[0];
                let ra = extent(run, true);
                let rb = extent(run, false);
                if stored_runs < JSON_RUN_LIMIT {
                    runs.push(json!({"kind":op.kind(),"a":ra,"b":rb}));
                    stored_runs += 1;
                } else {
                    json_truncated = true;
                }
                match (a, b) {
                    (Data::Bytes(a), Data::Bytes(b)) => match op {
                        Op::Equal(..) => {
                            let [i, j] = ra.unwrap();
                            report.push(&format!("  = @{} {}", offset(i), byte_preview(&a[i..j])));
                        }
                        Op::Delete(..) => {
                            let [i, j] = ra.unwrap();
                            report.push(&format!(
                                "  - A @{} {}",
                                offset(i),
                                byte_preview(&a[i..j])
                            ));
                        }
                        Op::Insert(..) => {
                            let [i, j] = rb.unwrap();
                            report.push(&format!(
                                "  + B @{} {}",
                                offset(i),
                                byte_preview(&b[i..j])
                            ));
                        }
                    },
                    (Data::Lines(a), Data::Lines(b)) => match op {
                        Op::Equal(..) => {
                            let [i, j] = ra.unwrap();
                            report.push(&format!("  = A L{}: {}", i + 1, a[i]));
                            if j - i > 1 {
                                report.push(&format!("    …至 L{j}: {}", a[j - 1]));
                            }
                        }
                        Op::Delete(..) => {
                            let [i, j] = ra.unwrap();
                            for (k, s) in a.iter().enumerate().take(j).skip(i) {
                                report.push(&format!("  - A L{}: {s}", k + 1));
                            }
                        }
                        Op::Insert(..) => {
                            let [i, j] = rb.unwrap();
                            for (k, s) in b.iter().enumerate().take(j).skip(i) {
                                report.push(&format!("  + B L{}: {s}", k + 1));
                            }
                        }
                    },
                    _ => unreachable!(),
                }
                at = next;
            }
            if !runs.is_empty() {
                details.push(json!({"a":ar,"b":br,"runs":runs}));
            }
            report.push("");
        }
    }
    let mut out = out_text(report.text);
    out.insert("equal".into(), PortValue::Bool(equal));
    out.insert("json".into(),PortValue::Json(json!({"mode":if line {"line"} else {"byte"},"encoding":enc,"strategy":kind,"aLength":a.len(),"bLength":b.len(),"equal":counts[0],"deleted":counts[1],"inserted":counts[2],"hunkCount":ranges.len(),"hunks":details,"rangeConvention":"zero-based-half-open","reportTruncated":report.truncated,"hunksTruncated":json_truncated})));
    ctx.cancel.check()?;
    Ok(out)
}
fn compact(s: &str) -> String {
    s.chars().filter(|&c| !numeric_support::space(c)).collect()
}
fn detect(s: &str) -> &'static str {
    let s = compact(s);
    if !s.is_empty() && s.len().is_multiple_of(2) && s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return "hex";
    }
    if !s.is_empty()
        && s.len().is_multiple_of(4)
        && regex::Regex::new(r"^[A-Za-z0-9+/]+={0,2}$")
            .unwrap()
            .is_match(&s)
    {
        return "base64";
    }
    "utf8"
}
fn bytes(v: &PortValue, enc: &str) -> Result<Vec<u8>, CoreError> {
    let out = match v {
        PortValue::Bytes(b) => {
            if b.len() > LIMIT {
                return Err(err("每段输入最多 256 KiB"));
            }
            b.to_vec()
        }
        PortValue::Text(s) => {
            if s.len() > LIMIT {
                return Err(err("每段输入最多 256 KiB"));
            }
            match enc {
                "utf8" => s.as_bytes().to_vec(),
                "hex" => hex::decode(compact(s))
                    .map_err(|_| err("Hex 输入须为偶数个合法十六进制字符"))?,
                "base64" => {
                    let config = base64::engine::GeneralPurposeConfig::new()
                        .with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent)
                        .with_decode_allow_trailing_bits(true);
                    base64::engine::GeneralPurpose::new(&base64::alphabet::STANDARD, config)
                        .decode(compact(s))
                        .map_err(|_| err("Base64 输入无效"))?
                }
                _ => return Err(err("输入编码必须为 auto/hex/base64/utf8")),
            }
        }
        _ => return Err(err("差异对比输入需要文本或原始字节")),
    };
    if out.len() > LIMIT {
        return Err(err("每段解码数据最多 256 KiB"));
    }
    Ok(out)
}
fn lines(bytes: &[u8]) -> Result<Vec<String>, CoreError> {
    let s = std::str::from_utf8(bytes)
        .map_err(|_| err("逐行模式要求合法 UTF-8，请用字节模式对比二进制"))?;
    if s.is_empty() {
        Ok(Vec::new())
    } else {
        let separators = s.bytes().filter(|&b| b == b'\n').count();
        Ok(s.split('\n')
            .enumerate()
            .map(|(i, l)| {
                if i < separators {
                    l.strip_suffix('\r').unwrap_or(l).to_owned()
                } else {
                    l.to_owned()
                }
            })
            .collect())
    }
}
struct Diff;
impl Node for Diff {
    fn run(&self, inputs: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let a = inputs.get("a").ok_or_else(|| err("缺少输入 A"))?;
        if matches!(a,PortValue::Text(s) if s.len()>LIMIT) {
            return Err(err("每段输入最多 256 KiB"));
        }
        let fallback = if inputs.contains_key("b") {
            None
        } else {
            let text = pstr(p, "textB", "");
            if text.len() > LIMIT {
                return Err(err("每段输入最多 256 KiB"));
            }
            Some(PortValue::Text(text.into()))
        };
        let b = inputs.get("b").or(fallback.as_ref()).unwrap();
        let mode = pstr(p, "mode", "byte");
        if !["byte", "line"].contains(&mode) {
            return Err(err("对比模式须为 byte/line"));
        }
        let encoding = pstr(p, "inputEnc", "auto");
        if !["auto", "hex", "base64", "utf8"].contains(&encoding) {
            return Err(err("无效输入编码"));
        }
        let enc = if mode == "line" {
            "utf8"
        } else if encoding == "auto" {
            match a {
                PortValue::Text(s) => detect(s),
                _ => "utf8",
            }
        } else {
            encoding
        };
        let a = bytes(a, enc)?;
        let b = bytes(b, enc)?;
        let context = numeric_support::number(p, "context", 0, 0, (LIMIT * 2) as i64)? as usize;
        let ad = if mode == "line" {
            Data::Lines(lines(&a)?)
        } else {
            Data::Bytes(a.clone())
        };
        let bd = if mode == "line" {
            Data::Lines(lines(&b)?)
        } else {
            Data::Bytes(b.clone())
        };
        let (ops, kind) = match (&ad, &bd) {
            (Data::Bytes(a), Data::Bytes(b)) => seq(a, b, ctx)?,
            (Data::Lines(a), Data::Lines(b)) => seq(a, b, ctx)?,
            _ => unreachable!(),
        };
        let mut out = render(&ad, &bd, &ops, context, kind, encoding, ctx)?;
        out.insert("a_bytes".into(), PortValue::Bytes(a.into()));
        out.insert("b_bytes".into(), PortValue::Bytes(b.into()));
        Ok(out)
    }
}
pub fn register(reg: &mut NodeRegistry) {
    let mut d=document_text(desc("diff_tool","data.text","文本与字节差异对比",CYAN,vec![req("a","输入 A",PortType::Any),opt("b","输入 B（优先）",PortType::Any)],vec![t_out(),req("equal","内容相同",PortType::Bool),req("json","差异区间与统计",PortType::Json),req("a_bytes","A 完整字节",PortType::Bytes),req("b_bytes","B 完整字节",PortType::Bytes)],vec![
        help(ParamSpec::text("textB","输入 B（无连线时）","",true),"可粘贴第二段文本；b 端口优先。byte 模式 auto 按 A 选择编码并同时用于 B；原始 Bytes 不再解码。"),
        help(ParamSpec::select("mode","对比模式",&["byte","line"],"byte"),"byte 逐字节；line 按 UTF-8、LF/CRLF 分行，忽略编码参数。等长逐位置，不等长用 LCS 或大输入回退策略。"),
        help(ParamSpec::select("inputEnc","字节输入编码",&["auto","hex","base64","utf8"],"auto"),"auto 优先 Hex，其次标准 Base64，再 UTF-8；以 A 探测结果同时解释 B。A 为 Bytes 时 auto 下的文本 B 按 UTF-8。"),
        help(ParamSpec::number("context","上下文数量",0.0,(LIMIT*2) as f64,1.0,0.0),"差异前后的操作数量，与上游一致；相邻区间合并。JSON 区间使用从 0 开始、右端不含的索引。"),
    ]),"diffTool","对比两段文本或字节，保留等长逐位、不等长 LCS、超过 200 万格回退前后缀策略；每段最多 256 KiB。报告 1 MiB、JSON 16384 个连续段上限，超出明确标记；完整输入从字节端口取得。支持取消。","00 ff 41",json!({"textB":"00 fe 41","inputEnc":"hex","context":1}));
    d.presentation.inline_params = vec!["mode".into(), "inputEnc".into(), "context".into()];
    d.aliases
        .extend(["diffTool", "diff", "LCS", "差异对比", "二进制比较"].map(String::from));
    d.cost = Cost::Medium;
    reg.register(d, Arc::new(|| Arc::new(Diff)));
}
