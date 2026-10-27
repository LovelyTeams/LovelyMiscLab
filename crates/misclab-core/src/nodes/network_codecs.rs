//! Offline address/protocol conversions, pinned to EBCTFCodeBox ea2bc36.
use super::{
    codec_support::{document_text, help},
    numeric_support::{bad, space},
    prelude::*,
};
use serde_json::{json, Value};
use std::net::{Ipv4Addr, Ipv6Addr};
fn auto(s: &str) -> Result<u64, CoreError> {
    let s = s.trim_matches(space).to_ascii_lowercase();
    if s.is_empty() || s.starts_with(['+', '-']) {
        return Err(bad("需要无符号整数"));
    }
    let (b, t) = if let Some(t) = s.strip_prefix("0x") {
        (16, t)
    } else if let Some(t) = s.strip_prefix("0o") {
        (8, t)
    } else if let Some(t) = s.strip_prefix("0b") {
        (2, t)
    } else if s.len() > 1 && s.starts_with('0') && s.bytes().all(|c| (b'0'..=b'7').contains(&c)) {
        (8, &s[1..])
    } else {
        (10, s.as_str())
    };
    u64::from_str_radix(t, b).map_err(|_| bad("整数无效或超出范围"))
}
fn v4(s: &str, format: Option<&str>) -> Result<u32, CoreError> {
    let s = s.trim_matches(space);
    let mut nums = Vec::new();
    if !s.contains('.') {
        let prefix = s.to_ascii_lowercase();
        let n = match format {
            Some(f) if !["0x", "0o", "0b"].iter().any(|p| prefix.starts_with(p)) => {
                let b = match f {
                    "hex" => 16,
                    "oct" => 8,
                    "bin" => 2,
                    "dec" => 0,
                    _ => return Err(bad("未知 IPv4 格式")),
                };
                if b == 0 {
                    auto(s)?
                } else {
                    u64::from_str_radix(s, b).map_err(|_| bad("无效 IPv4 数字"))?
                }
            }
            _ => auto(s)?,
        };
        return u32::try_from(n).map_err(|_| bad("IPv4 超过 32 位"));
    }
    for p in s.split('.') {
        nums.push(auto(p)?);
    }
    if !(2..=4).contains(&nums.len()) {
        return Err(bad("IPv4 需要 1..4 段"));
    }
    let last_bits = 8 * (5 - nums.len());
    let mut out = 0u64;
    for (i, &n) in nums.iter().enumerate() {
        let bits = if i == nums.len() - 1 { last_bits } else { 8 };
        if n >= 1u64 << bits {
            return Err(bad("IPv4 分段超出位宽"));
        }
        out = (out << bits) | n;
    }
    Ok(out as u32)
}
fn v6(s: &str) -> Result<[u16; 8], CoreError> {
    let mut s = s.trim_matches(space).to_string();
    if s.contains('.') {
        let i = s
            .rfind(':')
            .ok_or_else(|| bad("IPv6 中的 IPv4 地址缺少冒号"))?;
        let n = v4(&s[i + 1..], None)?;
        s = format!("{}{:x}:{:x}", &s[..i + 1], n >> 16, n & 65535);
    }
    s.parse::<Ipv6Addr>()
        .map(|v| v.segments())
        .map_err(|_| bad("IPv6 地址格式无效"))
}
fn compress(g: [u16; 8]) -> String {
    let (mut start, mut length) = (0, 0);
    let mut i = 0;
    while i < 8 {
        if g[i] != 0 {
            i += 1;
            continue;
        }
        let begin = i;
        while i < 8 && g[i] == 0 {
            i += 1;
        }
        if i - begin > length {
            start = begin;
            length = i - begin;
        }
    }
    let h: Vec<_> = g.iter().map(|n| format!("{n:x}")).collect();
    if length < 2 {
        h.join(":")
    } else {
        format!(
            "{}::{}",
            h[..start].join(":"),
            h[start + length..].join(":")
        )
    }
}
fn mac(s: &str) -> Result<u64, CoreError> {
    let s = s.trim_matches(space);
    if !s.contains([':', '-', '.']) {
        let n = auto(s)?;
        return if n < 1u64 << 48 {
            Ok(n)
        } else {
            Err(bad("MAC 超过 48 位"))
        };
    }
    let parts: Vec<_> = if s.contains('.') {
        s.split('.').collect()
    } else {
        s.split([':', '-']).collect()
    };
    let expected = if s.contains('.') { 3 } else { 6 };
    let h = parts.join("");
    if parts.len() != expected || h.len() != 12 || !h.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(bad("MAC 需要 12 个十六进制字符与有效分段"));
    }
    u64::from_str_radix(&h, 16).map_err(|_| bad("MAC 无效"))
}
fn capture(s: &str, pattern: &str) -> Option<String> {
    regex::Regex::new(pattern)
        .unwrap()
        .captures(s)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().into())
}
fn user_agent(s: &str) -> Result<(String, Value), CoreError> {
    if s.is_empty() {
        return Err(bad("UA 不能为空"));
    }
    let mut browser = "未知";
    let mut bv = String::new();
    for (label, re) in [
        ("Microsoft Edge", r"Edg/([\d.]+)"),
        ("Opera", r"OPR/([\d.]+)"),
        ("Internet Explorer", r"MSIE ([\d.]+)"),
        ("Internet Explorer", r"Trident/[\d.]+.*rv:([\d.]+)"),
        ("Firefox", r"Firefox/([\d.]+)"),
        ("Chrome", r"Chrome/([\d.]+)"),
        ("Safari", r"Version/([\d.]+).*Safari"),
    ] {
        if let Some(v) = capture(s, re) {
            browser = label;
            bv = v;
            break;
        }
    }
    if browser == "未知" && s.contains("Safari") {
        browser = "Safari";
    }
    let (engine, ev) = if let Some(v) = capture(s, r"Trident/([\d.]+)") {
        ("Trident", v)
    } else if capture(s, r"Gecko/([\d.]+)").is_some() && !s.contains("Chrome/") {
        ("Gecko", capture(s, r"rv:([\d.]+)").unwrap_or_default())
    } else if let Some(v) = capture(s, r"Chrome/([\d.]+)") {
        ("Blink", v)
    } else if let Some(v) = capture(s, r"AppleWebKit/([\d.]+)") {
        ("WebKit", v)
    } else {
        ("未知", String::new())
    };
    let (os, ov) = if let Some(v) = capture(s, r"Windows NT ([\d.]+)") {
        let name = match v.as_str() {
            "10.0" => "10/11",
            "6.3" => "8.1",
            "6.2" => "8",
            "6.1" => "7",
            "6.0" => "Vista",
            "5.1" => "XP",
            "5.2" => "XP x64/Server 2003",
            "5.0" => "2000",
            _ => "",
        };
        (
            "Windows",
            if name.is_empty() {
                format!("NT {v}")
            } else {
                name.into()
            },
        )
    } else if s.contains("iPad") || s.contains("iPhone") || s.contains("iPod") {
        (
            if s.contains("iPad") { "iPadOS" } else { "iOS" },
            capture(s, r"OS (\d+[_\d]*)")
                .unwrap_or_default()
                .replace('_', "."),
        )
    } else if let Some(v) = capture(s, r"Android ([\d.]+)") {
        ("Android", v)
    } else if let Some(v) = capture(s, r"Mac OS X ([\d_.]+)") {
        ("macOS", v.replace('_', "."))
    } else if s.contains("CrOS") {
        ("Chrome OS", String::new())
    } else if s.contains("Linux") {
        ("Linux", String::new())
    } else {
        ("未知", String::new())
    };
    let device = if s.contains("iPad") || s.contains("Tablet") {
        "平板"
    } else if s.contains("Mobi") || s.contains("iPhone") {
        "手机"
    } else {
        "桌面"
    };
    let model = capture(s, r"\(iPhone; ([^)]+)\)")
        .or_else(|| capture(s, r"\(Linux; Android [\d.]+; ([^)]+)\)"))
        .unwrap_or_default();
    let units: Vec<_> = s.encode_utf16().collect();
    let shown = if units.len() > 80 {
        format!("{} …", String::from_utf16_lossy(&units[..80]))
    } else {
        s.into()
    };
    let with_version = |name: &str, v: &str| {
        if v.is_empty() {
            name.to_string()
        } else {
            format!("{name} {v}")
        }
    };
    let mut lines = vec![
        format!("原始 UA:    {shown}"),
        format!("浏览器:     {}", with_version(browser, &bv)),
        format!("引擎:       {}", with_version(engine, &ev)),
        format!("操作系统:   {}", with_version(os, &ov)),
        format!("设备类型:   {device}"),
    ];
    if !model.is_empty() {
        lines.push(format!("设备型号:   {model}"));
    }
    Ok((
        lines.join("\n"),
        json!({"browser":browser,"browserVersion":bv,"engine":engine,"engineVersion":ev,"os":os,"osVersion":ov,"device":device,"model":model}),
    ))
}
#[derive(Clone, Copy)]
enum Kind {
    V4(bool),
    V6(bool),
    Mac,
    Cidr,
    Ua,
    Magnet,
}
struct Tool(Kind);
impl Node for Tool {
    fn run(&self, i: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let s = in_text(i, "text")?.trim_matches(space);
        if s.len() > 1_048_576 {
            return Err(bad("输入最多 1 MiB"));
        }
        let mut out = PortMap::new();
        let text = match self.0 {
            Kind::V4(decode) => {
                let f = pstr(p, "format", "dec");
                if !["dec", "hex", "oct", "bin"].contains(&f) {
                    return Err(bad("未知 IPv4 格式"));
                }
                let n = v4(s, decode.then_some(f))?;
                if decode {
                    Ipv4Addr::from(n).to_string()
                } else {
                    let bare = pbool(p, "compat", false);
                    match f {
                        "hex" => {
                            if bare {
                                format!("{n:08x}")
                            } else {
                                format!("0x{n:08X}")
                            }
                        }
                        "oct" => format!("{}{n:011o}", if bare { "" } else { "0" }),
                        "bin" => format!("{}{n:032b}", if bare { "" } else { "0b" }),
                        _ => n.to_string(),
                    }
                }
            }
            Kind::V6(decode) => {
                let g = v6(s)?;
                if decode {
                    g.iter()
                        .map(|n| format!("{n:04x}"))
                        .collect::<Vec<_>>()
                        .join(":")
                } else {
                    compress(g)
                }
            }
            Kind::Mac => {
                let n = mac(s)?;
                let h = format!("{n:012x}");
                match pstr(p, "format", "colon") {
                    "int" => n.to_string(),
                    f @ ("colon" | "hyphen" | "dot") => h
                        .as_bytes()
                        .chunks(if f == "dot" { 4 } else { 2 })
                        .map(|c| std::str::from_utf8(c).unwrap())
                        .collect::<Vec<_>>()
                        .join(match f {
                            "dot" => ".",
                            "hyphen" => "-",
                            _ => ":",
                        }),
                    _ => return Err(bad("未知 MAC 格式")),
                }
            }
            Kind::Cidr => {
                let (ip, pre) = s
                    .split_once('/')
                    .ok_or_else(|| bad("CIDR 需要 A.B.C.D/N"))?;
                if !ip.chars().all(|c| c.is_ascii_digit() || c == '.')
                    || pre.is_empty()
                    || !pre.chars().all(|c| c.is_ascii_digit())
                {
                    return Err(bad("CIDR 格式无效"));
                }
                let ip = v4(ip, None)?;
                let pre: u32 = pre.parse().map_err(|_| bad("CIDR 前缀无效"))?;
                if pre > 32 {
                    return Err(bad("CIDR 前缀必须在 0..32"));
                }
                let mask = if pre == 0 { 0 } else { u32::MAX << (32 - pre) };
                let network = ip & mask;
                let wildcard = !mask;
                let broadcast = network | wildcard;
                let (first, last, count) = if pre == 32 {
                    (network, network, 1u64)
                } else if pre == 31 {
                    (network, broadcast, 2)
                } else {
                    (network + 1, broadcast - 1, (1u64 << (32 - pre)) - 2)
                };
                let a = ip >> 24;
                let b = (ip >> 16) & 255;
                let private =
                    a == 10 || (a == 172 && (16..=31).contains(&b)) || (a == 192 && b == 168);
                let class = match a {
                    0..128 => "A 类",
                    128..192 => "B 类",
                    192..224 => "C 类",
                    224..240 => "D 类（组播）",
                    _ => "E 类（保留）",
                };
                let addr = |n| Ipv4Addr::from(n).to_string();
                let data = json!({"ip":addr(ip),"prefix":pre,"mask":addr(mask),"wildcard":addr(wildcard),"network":addr(network),"broadcast":addr(broadcast),"hostMin":addr(first),"hostMax":addr(last),"hostCount":count,"private":private,"class":class,"integer":ip});
                out.insert("data".into(), PortValue::Json(data));
                out.insert("hosts".into(), PortValue::Number(count as f64));
                format!("CIDR:        {}/{pre}\nIP 地址:     {}  ({class}，{})\n前缀长度:    /{pre}\n子网掩码:    {}  (0x{mask:08X})\n反掩码:      {}\n网络地址:    {}\n广播地址:    {}\n主机最小:    {}\n主机最大:    {}\n主机数量:    {count}\nIP 整数:     {ip}  (0x{ip:08X})",addr(ip),addr(ip),if private{"私有"}else{"公共"},addr(mask),addr(wildcard),addr(network),addr(broadcast),addr(first),addr(last))
            }
            Kind::Ua => {
                let (t, v) = user_agent(s)?;
                out.insert("data".into(), PortValue::Json(v));
                t
            }
            Kind::Magnet => {
                if s.get(..8)
                    .is_none_or(|p| !p.eq_ignore_ascii_case("magnet:?"))
                {
                    return Err(bad("需要 magnet:? 链接"));
                }
                let suffix = regex::Regex::new(r"\.\d+$").unwrap();
                let mut rows = Vec::new();
                let mut lines = Vec::new();
                let mut trackers = Vec::new();
                let mut hashes = Vec::new();
                let mut xt = false;
                for pair in s[8..].split('&').filter(|x| !x.is_empty()) {
                    ctx.cancel.check()?;
                    let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
                    let k = suffix.replace(k, "");
                    let value = super::structured_text::query_decode(v);
                    if k == "xt" && !v.trim_matches(space).is_empty() {
                        xt = true;
                    }
                    rows.push(json!({"key":k,"value":value}));
                    if k == "tr" {
                        trackers.push(value);
                        continue;
                    }
                    let label = match &*k {
                        "xt" => "精确主题 (xt)",
                        "dn" => "显示名 (dn)",
                        "xl" => "文件大小 (xl)",
                        "ws" => "Web Seed (ws)",
                        "as" => "备用源 (as)",
                        "xs" => "精确源 (xs)",
                        "kt" => "关键词 (kt)",
                        "mt" => "清单主题 (mt)",
                        _ => "",
                    };
                    lines.push(format!(
                        "{}: {value}",
                        if label.is_empty() {
                            format!("{k} 参数")
                        } else {
                            label.into()
                        }
                    ));
                    if k == "xt" {
                        if let Some(h) = capture(&value, r"(?i)urn:btih:([0-9a-z]+)") {
                            lines.push(format!("  ↳ BTIH 哈希: {h}"));
                            hashes.push(h);
                        }
                    }
                }
                if !xt {
                    return Err(bad("Magnet 链接缺少非空 xt 精确主题"));
                }
                if !trackers.is_empty() {
                    lines.push(format!("--- Tracker 列表 ({}) ---", trackers.len()));
                    lines.extend(trackers.iter().map(|t| format!("  {t}")));
                }
                out.insert("rows".into(), PortValue::Json(json!(rows)));
                out.insert("trackers".into(), PortValue::StringList(trackers));
                out.insert("hashes".into(), PortValue::StringList(hashes));
                lines.join("\n")
            }
        };
        out.insert("text".into(), PortValue::Text(text));
        Ok(out)
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for (kind,id,upstream,title,category,description,sample)in[
 (Kind::V4(false),"ipv4_integer_encode","ipv4Int","IPv4 转整数","numeric.address","IPv4 与 32 位整数互转；支持 inet_aton 缩写、每段 0x/0o/0b 和前导零八进制。","192.168.1.1"),
 (Kind::V4(true),"ipv4_integer_decode","ipv4Int","整数转 IPv4","numeric.address","无前缀整数按 format 指定进制，带前缀或点分输入自动识别；严格检查分段范围。","3232235777"),
 (Kind::V6(false),"ipv6_compress","ipv6Format","IPv6 压缩","numeric.address","按 RFC 5952 压缩最长零段，并列取首段，不压缩单零。支持嵌入式 IPv4。","2001:0db8:0000:0000:0000:0000:0000:0001"),
 (Kind::V6(true),"ipv6_expand","ipv6Format","IPv6 展开","numeric.address","展开成 8 组 4 位小写十六进制，拒绝多重压缩和非法分组。","2001:db8::1"),
 (Kind::Mac,"mac_format_encode","macFormat","MAC 格式编码","numeric.address","48 位 MAC 自动识别冒号、连字符、点分或带前缀整数，输出由 format 指定。","aa:bb:cc:dd:ee:ff"),
 (Kind::Mac,"mac_format_decode","macFormat","MAC 格式解码","numeric.address","两方向使用相同的自动解析与格式化规则，可转换为冒号、连字符、点分或十进制。","187723572702975"),
 (Kind::Cidr,"cidr_calculate","cidrCalc","CIDR 子网计算","numeric.address","离线计算 IPv4 网络、广播、掩码、主机范围；/31 支持点对点，/32 单主机。不枚举或访问网络。","192.168.1.123/24"),
 (Kind::Ua,"user_agent_parse","userAgentParse","User-Agent 解析","data.structure","按固定上游规则解析浏览器、引擎、系统与设备，报告与结构化 JSON 同时输出。","Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120.0 Safari/537.36 Edg/120.0"),
 (Kind::Magnet,"magnet_parse","magnetParse","Magnet 链接解析","data.structure","离线解析主题、BTIH 哈希、显示名与 Tracker；保留重复字段和编号后缀，错误百分号转义保持原文。","magnet:?xt=urn:btih:0123456789ABCDEF&dn=example+file&tr=udp%3A%2F%2Ftracker.example")]{
 let params=match kind{Kind::V4(_)=>vec![help(ParamSpec::select("format","整数格式",&["dec","hex","oct","bin"],"dec"),"整数输入/输出进制；带前缀输入自动识别。"),help(ParamSpec::toggle("compat","省略输出前缀",false),"编码省略 0x/0b/八进制前导标记；hex 改为小写。")],Kind::Mac=>vec![help(ParamSpec::select("format","输出格式",&["colon","hyphen","dot","int"],"colon"),"colon 冒号；hyphen 连字符；dot 点分；int 十进制整数。")],_=>vec![]};
 let mut outputs=vec![t_out()];
 if matches!(kind,Kind::Cidr|Kind::Ua){outputs.push(req("data","结构化数据",PortType::Json));}
 if matches!(kind,Kind::Cidr){outputs.push(req("hosts","主机数量",PortType::Number));}
 if matches!(kind,Kind::Magnet){outputs.extend([req("rows","字段明细",PortType::Json),req("trackers","Tracker 列表",PortType::StringList),req("hashes","BTIH 哈希",PortType::StringList)]);}
 let mut d=document_text(desc(id,category,title,CYAN,vec![t_in()],outputs,params),upstream,description,sample,json!({}));if matches!(kind,Kind::Magnet){d.presentation.previews.insert("rows".into(),"table".into());}reg.register(d,Arc::new(move||Arc::new(Tool(kind))));
 }
}
