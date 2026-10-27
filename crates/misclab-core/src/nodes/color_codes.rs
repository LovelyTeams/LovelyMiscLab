//! RGB/HSL/HSV/CMYK and 24-bit color conversions, pinned upstream semantics.
use super::{
    codec_support::{document_text, help},
    prelude::*,
    transport_codecs::err,
};
use serde_json::{json, Value};
use std::sync::LazyLock;
type Rgb = [u8; 3];
static NAMED: LazyLock<Vec<(String, Rgb)>> =
    LazyLock::new(|| serde_json::from_str(include_str!("css_named_colors.json")).unwrap());
const FORMATS: &[&str] = &["hex", "rgb", "hsl", "hsv", "cmyk", "int", "cssname"];
fn byte(n: f64) -> u8 {
    n.round().clamp(0.0, 255.0) as u8
}
fn unit(n: f64) -> f64 {
    n.clamp(0.0, 1.0)
}
fn pct(n: f64) -> f64 {
    if n <= 1.0 {
        n
    } else {
        n / 100.0
    }
}
fn rgbhex(rgb: Rgb, upper: bool) -> String {
    let h = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
    if upper {
        h.to_ascii_uppercase()
    } else {
        h
    }
}
fn integer(rgb: Rgb) -> u32 {
    (u32::from(rgb[0]) << 16) | (u32::from(rgb[1]) << 8) | u32::from(rgb[2])
}
fn parse_hex(s: &str) -> Result<Rgb, CoreError> {
    let s = s.trim();
    let s = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix('#'))
        .unwrap_or(s);
    if ![3, 6].contains(&s.len()) || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(err("颜色 Hex 需要 #RGB / #RRGGBB / 0xRRGGBB"));
    }
    let h = if s.len() == 3 {
        s.chars().flat_map(|c| [c, c]).collect::<String>()
    } else {
        s.to_owned()
    };
    Ok(hex::decode(h).unwrap().try_into().unwrap())
}
fn parse_int(s: &str) -> Result<Rgb, CoreError> {
    let s = s.trim().strip_prefix('#').unwrap_or(s.trim());
    let n = if let Some(h) = s.strip_prefix("0x") {
        u32::from_str_radix(h, 16)
    } else if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
        s.parse::<u32>()
    } else if s.len() == 6 && s.bytes().all(|b| b.is_ascii_hexdigit()) {
        u32::from_str_radix(s, 16)
    } else {
        return Err(err("整数色值需要十进制或 0x Hex"));
    };
    let n = n.map_err(|_| err("整数色值无效或溢出"))?;
    if n > 0xffffff {
        return Err(err("整数色值须在 0–16777215"));
    }
    Ok([(n >> 16) as u8, (n >> 8) as u8, n as u8])
}
fn name_rgb(s: &str) -> Option<Rgb> {
    let name = s
        .to_ascii_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>();
    NAMED.iter().find(|(n, _)| n == &name).map(|(_, c)| *c)
}
fn rgb_name(rgb: Rgb, closest: bool) -> Option<&'static str> {
    if let Some((name, _)) = NAMED.iter().find(|(_, c)| *c == rgb) {
        return Some(name);
    }
    if !closest {
        return None;
    }
    // Stable table order resolves equal distances and duplicate names.
    NAMED
        .iter()
        .min_by_key(|(_, c)| {
            c.iter()
                .zip(rgb)
                .map(|(&a, b)| (i32::from(a) - i32::from(b)).pow(2))
                .sum::<i32>()
        })
        .map(|(name, _)| name.as_str())
}
fn hue(rgb: Rgb) -> (f64, f64, f64, f64) {
    let [r, g, b] = rgb.map(|n| f64::from(n) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, d, max, min)
}
fn to_hsl(rgb: Rgb) -> [f64; 3] {
    let (h, d, max, min) = hue(rgb);
    let l = (max + min) / 2.0;
    let s = if d == 0.0 {
        0.0
    } else if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    [h, s, l]
}
fn to_hsv(rgb: Rgb) -> [f64; 3] {
    let (h, d, max, _) = hue(rgb);
    [h, if max == 0.0 { 0.0 } else { d / max }, max]
}
fn to_cmyk(rgb: Rgb) -> [f64; 4] {
    let [r, g, b] = rgb.map(|n| f64::from(n) / 255.0);
    let k = 1.0 - r.max(g).max(b);
    if k == 1.0 {
        [0.0, 0.0, 0.0, 1.0]
    } else {
        [
            (1.0 - r - k) / (1.0 - k),
            (1.0 - g - k) / (1.0 - k),
            (1.0 - b - k) / (1.0 - k),
            k,
        ]
    }
}
fn from_hsl(h: f64, s: f64, l: f64) -> Rgb {
    let h = ((h % 360.0) + 360.0) % 360.0;
    let s = unit(s);
    let l = unit(l);
    if s == 0.0 {
        return [byte(l * 255.0); 3];
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let component = |mut t: f64| {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        byte(
            255.0
                * if t < 1.0 / 6.0 {
                    p + (q - p) * 6.0 * t
                } else if t < 0.5 {
                    q
                } else if t < 2.0 / 3.0 {
                    p + (q - p) * (2.0 / 3.0 - t) * 6.0
                } else {
                    p
                },
        )
    };
    [
        component(h / 360.0 + 1.0 / 3.0),
        component(h / 360.0),
        component(h / 360.0 - 1.0 / 3.0),
    ]
}
fn from_hsv(h: f64, s: f64, v: f64) -> Rgb {
    let h = ((h % 360.0) + 360.0) % 360.0;
    let s = unit(s);
    let v = unit(v);
    let c = v * s;
    let hp = h / 60.0;
    let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());
    let m = v - c;
    let rgb = if hp < 1.0 {
        [c, x, 0.0]
    } else if hp < 2.0 {
        [x, c, 0.0]
    } else if hp < 3.0 {
        [0.0, c, x]
    } else if hp < 4.0 {
        [0.0, x, c]
    } else if hp < 5.0 {
        [x, 0.0, c]
    } else {
        [c, 0.0, x]
    };
    rgb.map(|n| byte((n + m) * 255.0))
}
fn from_cmyk(v: &[f64]) -> Rgb {
    let k = unit(v[3]);
    [v[0], v[1], v[2]].map(|n| byte(255.0 * (1.0 - unit(n)) * (1.0 - k)))
}
fn numbers(s: &str, percent: bool) -> Result<Vec<f64>, CoreError> {
    s.split(|c: char| c == ',' || c == '/' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .map(|token| {
            let token = if percent {
                token.trim_end_matches('%')
            } else {
                token
            };
            let n = token.parse::<f64>().map_err(|_| err("颜色包含无效数值"))?;
            if !n.is_finite() {
                return Err(err("颜色数值必须有限"));
            }
            Ok(n)
        })
        .collect()
}
fn model_rgb(model: &str, parts: &[f64]) -> Result<Rgb, CoreError> {
    if parts.len() < if model == "cmyk" { 4 } else { 3 } {
        return Err(err("颜色参数数量不足"));
    }
    Ok(match model {
        "rgb" => [parts[0], parts[1], parts[2]].map(|n| byte(if n <= 1.0 { n * 255.0 } else { n })),
        "hsl" => from_hsl(parts[0], pct(parts[1]), pct(parts[2])),
        "hsv" => from_hsv(parts[0], pct(parts[1]), pct(parts[2])),
        "cmyk" => from_cmyk(&parts.iter().copied().map(pct).collect::<Vec<_>>()),
        _ => return Err(err("未知颜色模型")),
    })
}
fn parse_auto(s: &str) -> Result<Rgb, CoreError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(err("颜色输入不能为空"));
    }
    if let Some(rgb) = name_rgb(s) {
        return Ok(rgb);
    }
    let re = regex::Regex::new(r"(?i)^(#|0x)?[0-9a-f]{3}([0-9a-f]{3})?$").unwrap();
    if re.is_match(s) {
        return parse_hex(s);
    }
    if s.starts_with("0x") || s.bytes().all(|b| b.is_ascii_digit()) {
        return parse_int(s);
    }
    if let Some(open) = s.find('(') {
        if !s.ends_with(')') {
            return Err(err("颜色函数缺少右括号"));
        }
        let model = s[..open].trim().to_ascii_lowercase();
        let parts = numbers(&s[open + 1..s.len() - 1], true)?;
        return model_rgb(&model, &parts);
    }
    let n = numbers(s, false)?;
    if n.len() == 3 {
        if n.iter().all(|v| (0.0..=1.0).contains(v)) {
            return Ok([byte(n[0] * 255.0), byte(n[1] * 255.0), byte(n[2] * 255.0)]);
        }
        if n.iter().all(|v| (0.0..=255.0).contains(v)) {
            return Ok([byte(n[0]), byte(n[1]), byte(n[2])]);
        }
        if (0.0..=360.0).contains(&n[0])
            && (0.0..=100.0).contains(&n[1])
            && (0.0..=100.0).contains(&n[2])
        {
            return Ok(from_hsl(n[0], n[1] / 100.0, n[2] / 100.0));
        }
    }
    if n.len() == 4 {
        return model_rgb("cmyk", &n);
    }
    Err(err("无法识别颜色格式"))
}
fn parse(s: &str, from: &str) -> Result<Rgb, CoreError> {
    match from {
        "auto" | "rgb" => parse_auto(s),
        "hex" => parse_hex(s),
        "int" => parse_int(s),
        "cssname" => name_rgb(s).ok_or_else(|| err("没有对应的 CSS3 命名色")),
        "hsl" | "hsv" | "cmyk" => {
            let s = s.trim();
            let lower = s.to_ascii_lowercase();
            let body = if lower.starts_with(from) && s.ends_with(')') {
                s.find('(').map(|i| &s[i + 1..s.len() - 1]).unwrap_or(s)
            } else {
                s
            };
            model_rgb(from, &numbers(body, true)?)
        }
        _ => Err(err("未知源颜色格式")),
    }
}
fn fmt_hsx(name: &str, v: [f64; 3]) -> String {
    let h = ryu_js::Buffer::new()
        .format((v[0] * 10.0).round() / 10.0)
        .to_owned();
    format!(
        "{name}({h}, {}%, {}%)",
        super::float_codecs::fixed(v[1] * 100.0, 1),
        super::float_codecs::fixed(v[2] * 100.0, 1)
    )
}
fn format(rgb: Rgb, to: &str, upper: bool) -> Result<String, CoreError> {
    Ok(match to {
        "hex" => rgbhex(rgb, upper),
        "rgb" => format!("rgb({}, {}, {})", rgb[0], rgb[1], rgb[2]),
        "hsl" => fmt_hsx("hsl", to_hsl(rgb)),
        "hsv" => fmt_hsx("hsv", to_hsv(rgb)),
        "cmyk" => {
            let v = to_cmyk(rgb);
            format!(
                "cmyk({}%, {}%, {}%, {}%)",
                super::float_codecs::fixed(v[0] * 100.0, 1),
                super::float_codecs::fixed(v[1] * 100.0, 1),
                super::float_codecs::fixed(v[2] * 100.0, 1),
                super::float_codecs::fixed(v[3] * 100.0, 1)
            )
        }
        "int" => integer(rgb).to_string(),
        "cssname" => rgb_name(rgb, false)
            .ok_or_else(|| {
                err(format!(
                    "无精确 CSS 命名色匹配（{}，最近：{}）",
                    rgbhex(rgb, true),
                    rgb_name(rgb, true).unwrap()
                ))
            })?
            .into(),
        _ => return Err(err("未知目标颜色格式")),
    })
}
fn report(rgb: Rgb) -> Result<String, CoreError> {
    let n = integer(rgb);
    let bin = format!("{n:024b}");
    let exact = rgb_name(rgb, false);
    Ok(format!("RGB:        {}\nHex:        {}\n整数:       {n} (0x{n:06X})\n二进制:     {} {} {}\nHSL:        {}\nHSV:        {}\nCMYK:       {}\nCSS 命名色: {}\n最近命名色: {}",format(rgb,"rgb",true)?,rgbhex(rgb,true),&bin[..8],&bin[8..16],&bin[16..],format(rgb,"hsl",true)?,format(rgb,"hsv",true)?,format(rgb,"cmyk",true)?,exact.map(|n|format!("{n}（精确匹配）")).unwrap_or_else(||"无精确匹配".into()),rgb_name(rgb,true).unwrap()))
}
struct Color(u8);
impl Node for Color {
    fn run(&self, inputs: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let text = in_text(inputs, "text")?;
        if text.len() > 4096 {
            return Err(err("颜色输入最多 4096 字节"));
        }
        let from = pstr(p, "from", "auto");
        let to = pstr(p, "to", "hex");
        let (rgb, text) = if self.0 == 2 {
            let rgb = parse_auto(text)?;
            (rgb, report(rgb)?)
        } else {
            let (from, to) = if self.0 == 1 {
                (to, if from == "auto" { "hex" } else { from })
            } else {
                (from, to)
            };
            let rgb = parse(text, from)?;
            (rgb, format(rgb, to, pbool(p, "upper", true))?)
        };
        let mut out = out_text(text);
        out.insert("bytes".into(), PortValue::Bytes(rgb.to_vec().into()));
        out.insert("json".into(),PortValue::Json(json!({"rgb":rgb,"hex":rgbhex(rgb,true),"integer":integer(rgb),"hsl":to_hsl(rgb),"hsv":to_hsv(rgb),"cmyk":to_cmyk(rgb),"name":rgb_name(rgb,false),"nearestName":rgb_name(rgb,true)})));
        for (key, b) in ["red", "green", "blue"].into_iter().zip(rgb) {
            out.insert(key.into(), PortValue::Number(f64::from(b)));
        }
        ctx.cancel.check()?;
        Ok(out)
    }
}
pub fn register(reg: &mut NodeRegistry) {
    let params=vec![
        help(ParamSpec::select("from","源格式",&["auto","hex","rgb","hsl","hsv","cmyk","int","cssname"],"auto"),"正向输入格式；反向时变为目标，auto 反向输出 Hex。保留上游 RGB 启发式：裸三元组全 ≤1 时归一化，rgb() 分量 ≤1 单独归一化。百分号按上游剥离，非浏览器 CSS 解析器。"),
        help(ParamSpec::select("to","目标格式",FORMATS,"hex"),"正向目标，反向输入格式。cssname 只接受精确命名色；最近色在分析/JSON 输出查看。"),
        help(ParamSpec::toggle("upper","Hex 大写",true),"只影响转换文本的 Hex 大小写；JSON 的规范 Hex 保持大写。"),
    ];
    for (kind, id, name) in [
        (0, "color_convert_encode", "颜色编码正向转换"),
        (1, "color_convert_decode", "颜色编码反向转换"),
        (2, "color_info", "颜色全息分析"),
    ] {
        let mut d=document_text(desc(id,"numeric.address",name,CYAN,vec![t_in()],vec![t_out(),req("bytes","RGB 三字节",PortType::Bytes),req("red","红 R",PortType::Number),req("green","绿 G",PortType::Number),req("blue","蓝 B",PortType::Number),req("json","颜色详情",PortType::Json)],if kind==2 {vec![]} else {params.clone()}),if kind==2 {"colorInfo"} else {"color"},"RGB/HSL/HSV/CMYK/Hex/24 位整数/CSS3 147 命名色互转；按上游 RGB 数值启发式。JSON 提供完整分量及最近命名色（RGB 欧氏距离），bytes 固定为 R/G/B 三字节，无透明度。输入最多 4096 字节。",if kind==1 {"#FF0000"} else {"red"},json!({}));
        d.family = "color conversion".into();
        d.aliases
            .extend(["RGB", "HSL", "HSV", "CMYK", "颜色", "CSS color"].map(String::from));
        reg.register(d, Arc::new(move || Arc::new(Color(kind))));
    }
}
