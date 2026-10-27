//! Geographic text codecs from the pinned EBCTFCodeBox contract.
use super::{
    codec_support::{document_text, help},
    numeric_support,
    prelude::*,
    transport_codecs::err,
};
use serde_json::{json, Value};

const HASH: &[u8] = b"0123456789bcdefghjkmnpqrstuvwxyz";
const OLC: &[u8] = b"23456789CFGHJMPQRVWX";
const LAT_SCALE: i64 = 25_000_000;
const LON_SCALE: i64 = 8_192_000;

pub(super) fn fixed(n: f64, precision: usize) -> String {
    let value: f64 = format!("{n:.precision$}").parse().unwrap_or(0.0);
    if value == 0.0 {
        "0".into()
    } else {
        ryu_js::Buffer::new().format(value).to_owned()
    }
}
fn coordinate(text: &str) -> Result<(f64, f64), CoreError> {
    let re = regex::Regex::new(r"^(-?\d+(?:\.\d+)?)\s*[,，\s]\s*(-?\d+(?:\.\d+)?)$").unwrap();
    let c = re
        .captures(text.trim())
        .ok_or_else(|| err("坐标需要 lat,lon，例如 57.649,10.407"))?;
    let lat = c[1].parse::<f64>().map_err(|_| err("纬度无效"))?;
    let lon = c[2].parse::<f64>().map_err(|_| err("经度无效"))?;
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return Err(err("坐标超出纬度 ±90° / 经度 ±180°"));
    }
    Ok((lat, lon))
}
fn dms(dd: f64, latitude: bool) -> String {
    let hemi = match (latitude, dd >= 0.0) {
        (true, true) => 'N',
        (true, false) => 'S',
        (false, true) => 'E',
        (false, false) => 'W',
    };
    let abs = dd.abs();
    let deg = abs.floor();
    let minutes = (abs - deg) * 60.0;
    format!(
        "{deg}°{}'{}\"{hemi}",
        minutes.floor(),
        fixed((minutes - minutes.floor()) * 60.0, 3)
    )
}
fn parse_dms(s: &str) -> Result<f64, CoreError> {
    let s = s.trim();
    let hemi = s
        .chars()
        .last()
        .ok_or_else(|| err("DMS 不能为空"))?
        .to_ascii_uppercase();
    if !matches!(hemi, 'N' | 'S' | 'E' | 'W') {
        return Err(err("DMS 缺少 N/S/E/W 半球标识"));
    }
    let body = s[..s.len() - 1].replace(['°', '′', '\'', '"', '″'], " ");
    let parts = body.split_whitespace().collect::<Vec<_>>();
    if parts.is_empty() || parts.len() > 3 {
        return Err(err("DMS 需要 1–3 个数值字段"));
    }
    let mut out = 0.0;
    for (i, p) in parts.iter().enumerate() {
        let v = p.parse::<f64>().map_err(|_| err("DMS 数值无效"))?;
        if !v.is_finite() {
            return Err(err("DMS 数值必须有限"));
        }
        out += v / 60f64.powi(i as i32);
    }
    Ok(if hemi == 'S' || hemi == 'W' {
        -out
    } else {
        out
    })
}
fn decode_dms(s: &str) -> Result<(f64, f64), CoreError> {
    let split = regex::Regex::new(r"[,，]|\s{2,}").unwrap();
    let parts = split
        .split(s.trim())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    if parts.len() != 2 {
        return Err(err("DMS 需要以逗号分隔纬度和经度"));
    }
    let lat = parse_dms(parts[0])?;
    let lon = parse_dms(parts[1])?;
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return Err(err("DMS 坐标越界"));
    }
    Ok((lat, lon))
}
fn hash_encode(lat: f64, lon: f64, precision: usize) -> String {
    let mut bounds = [[-180.0, 180.0], [-90.0, 90.0]];
    let mut result = String::new();
    let mut ch = 0;
    for i in 0..precision * 5 {
        let axis = i % 2;
        let mid = (bounds[axis][0] + bounds[axis][1]) / 2.0;
        let bit = usize::from((if axis == 0 { lon } else { lat }) >= mid);
        bounds[axis][1 - bit] = mid;
        ch = (ch << 1) | bit;
        if i % 5 == 4 {
            result.push(HASH[ch] as char);
            ch = 0;
        }
    }
    result
}
fn hash_decode(s: &str) -> Result<(f64, f64, Value), CoreError> {
    let s = s.trim().to_ascii_lowercase();
    if s.is_empty() || s.len() > 64 {
        return Err(err("Geohash 需要 1–64 个字符"));
    }
    let mut bounds = [[-180.0, 180.0], [-90.0, 90.0]];
    for (i, b) in s.bytes().enumerate() {
        let ch = HASH
            .iter()
            .position(|&c| c == b)
            .ok_or_else(|| err("Geohash 包含非法字符（不允许 a/i/l/o）"))?;
        for bit in 0..5 {
            let axis = (i * 5 + bit) % 2;
            let mid = (bounds[axis][0] + bounds[axis][1]) / 2.0;
            bounds[axis][1 - ((ch >> (4 - bit)) & 1)] = mid;
        }
    }
    Ok((
        (bounds[1][0] + bounds[1][1]) / 2.0,
        (bounds[0][0] + bounds[0][1]) / 2.0,
        json!({"latMin":bounds[1][0],"latMax":bounds[1][1],"lonMin":bounds[0][0],"lonMax":bounds[0][1]}),
    ))
}
fn olc_encode(lat: f64, lon: f64, len: usize) -> Result<String, CoreError> {
    if len < 2 || (len < 10 && len % 2 == 1) {
        return Err(err("OLC 码长须为 2/4/6/8/10 或 11–15"));
    }
    let len = len.min(15);
    let mut a =
        ((lat * LAT_SCALE as f64).floor() as i64 + 90 * LAT_SCALE).clamp(0, 180 * LAT_SCALE - 1);
    let mut b =
        ((lon * LON_SCALE as f64).floor() as i64 + 180 * LON_SCALE).rem_euclid(360 * LON_SCALE);
    let mut digits = Vec::new();
    if len > 10 {
        for _ in 0..5 {
            digits.push(OLC[((a % 5) * 4 + b % 4) as usize]);
            a /= 5;
            b /= 4;
        }
    } else {
        a /= 3125;
        b /= 1024;
    }
    for _ in 0..5 {
        digits.push(OLC[(b % 20) as usize]);
        digits.push(OLC[(a % 20) as usize]);
        a /= 20;
        b /= 20;
    }
    digits.reverse();
    digits.truncate(len);
    if len < 8 {
        digits.resize(8, b'0');
    }
    digits.insert(8, b'+');
    Ok(String::from_utf8(digits).unwrap())
}
fn olc_decode(s: &str) -> Result<(f64, f64), CoreError> {
    let s = s.trim().to_ascii_uppercase();
    let b = s.as_bytes();
    let invalid = || err("需要完整 OLC 全码：第 8 位后为 +；短码需要参考点，不能单独解码");
    if b.len() < 9 || b[8] != b'+' || b.iter().filter(|&&c| c == b'+').count() != 1 || b.len() == 10
    {
        return Err(invalid());
    }
    if b.iter()
        .any(|c| !OLC.contains(c) && *c != b'+' && *c != b'0')
    {
        return Err(invalid());
    }
    let index = |c: u8| {
        OLC.iter()
            .position(|&v| v == c)
            .map(|i| i as i64)
            .ok_or_else(invalid)
    };
    if index(b[0])? >= 9 || index(b[1])? >= 18 {
        return Err(invalid());
    }
    if let Some(p) = b.iter().position(|&c| c == b'0') {
        if p < 2 || p % 2 != 0 || p >= 8 || b.len() != 9 || b[p..8].iter().any(|&c| c != b'0') {
            return Err(invalid());
        }
    }
    let c = b
        .iter()
        .copied()
        .filter(|&b| b != b'+' && b != b'0')
        .take(15)
        .collect::<Vec<_>>();
    let n = c.len().min(10);
    let mut a = -90 * 8000;
    let mut b = -180 * 8000;
    let mut pv = 160000;
    for i in (0..n).step_by(2) {
        a += index(c[i])? * pv;
        b += index(c[i + 1])? * pv;
        if i < n - 2 {
            pv /= 20;
        }
    }
    let mut da = pv as f64 / 8000.0;
    let mut db = da;
    let mut ga = 0;
    let mut gb = 0;
    if c.len() > 10 {
        let mut row = 625;
        let mut col = 256;
        for i in 10..c.len() {
            let d = index(c[i])?;
            ga += (d / 4) * row;
            gb += (d % 4) * col;
            if i < c.len() - 1 {
                row /= 5;
                col /= 4;
            }
        }
        da = row as f64 / LAT_SCALE as f64;
        db = col as f64 / LON_SCALE as f64;
    }
    let lat = a as f64 / 8000.0 + ga as f64 / LAT_SCALE as f64;
    let lon = b as f64 / 8000.0 + gb as f64 / LON_SCALE as f64;
    let round = |v: f64| (v * 1e14).round() / 1e14;
    Ok((
        (round(lat) + (round(lat + da) - round(lat)) / 2.0).min(90.0),
        (round(lon) + (round(lon + db) - round(lon)) / 2.0).min(180.0),
    ))
}
fn maiden_encode(lat: f64, lon: f64, pairs: usize) -> String {
    // Pinned upstream uses a fourth letter pair (24x24), not the decimal extension.
    // The closed upper coordinate boundary belongs to the final grid cell.
    let mut x = (lon + 180.0).min(360.0 - f64::EPSILON * 360.0);
    let mut y = (lat + 90.0).min(180.0 - f64::EPSILON * 180.0);
    let mut out = String::new();
    for (i, (dx, dy, base)) in [
        (20.0, 10.0, b'A'),
        (2.0, 1.0, b'0'),
        (1.0 / 12.0, 1.0 / 24.0, b'a'),
        (1.0 / 288.0, 1.0 / 576.0, b'a'),
    ]
    .into_iter()
    .enumerate()
    .take(pairs)
    {
        // Multiplication matches the upstream's treatment of floating boundaries.
        let nx = if i < 2 {
            (x / dx).floor()
        } else if i == 2 {
            (x * 12.0).floor()
        } else {
            (x * 12.0 * 24.0).floor()
        };
        let ny = if i < 2 {
            (y / dy).floor()
        } else if i == 2 {
            (y * 24.0).floor()
        } else {
            (y * 24.0 * 24.0).floor()
        };
        out.push((base + nx as u8) as char);
        out.push((base + ny as u8) as char);
        x -= nx * dx;
        y -= ny * dy;
    }
    out
}
fn maiden_decode(s: &str) -> Result<(f64, f64), CoreError> {
    let s = s.trim().to_ascii_uppercase();
    let b = s.as_bytes();
    if ![4, 6, 8].contains(&b.len())
        || !b[..2].iter().all(|b| (b'A'..=b'R').contains(b))
        || !b[2..4].iter().all(u8::is_ascii_digit)
        || !b[4..].iter().all(|b| (b'A'..=b'X').contains(b))
    {
        return Err(err("Maidenhead 需要 FN20 / FN20XR / FN20XRAB 格式"));
    }
    let mut lon = (b[0] - b'A') as f64 * 20.0 + (b[2] - b'0') as f64 * 2.0;
    let mut lat = (b[1] - b'A') as f64 * 10.0 + (b[3] - b'0') as f64;
    let mut dx = 2.0;
    let mut dy = 1.0;
    if b.len() >= 6 {
        dx = 1.0 / 12.0;
        dy = 1.0 / 24.0;
        lon += (b[4] - b'A') as f64 / 12.0;
        lat += (b[5] - b'A') as f64 / 24.0;
    }
    if b.len() == 8 {
        dx = 1.0 / 288.0;
        dy = 1.0 / 576.0;
        lon += (b[6] - b'A') as f64 / 288.0;
        lat += (b[7] - b'A') as f64 / 576.0;
    }
    Ok((lat + dy / 2.0 - 90.0, lon + dx / 2.0 - 180.0))
}

struct Geo {
    kind: usize,
    decode: bool,
}
impl Node for Geo {
    fn run(&self, inputs: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let s = in_text(inputs, "text")?;
        if s.len() > 4096 {
            return Err(err("坐标或编码输入最多 4096 字节"));
        }
        if self.decode {
            let mut extra = Value::Null;
            let (lat, lon) = match self.kind {
                0 => decode_dms(s)?,
                1 => {
                    let (a, b, v) = hash_decode(s)?;
                    extra = v;
                    (a, b)
                }
                2 => olc_decode(s)?,
                3 => maiden_decode(s)?,
                _ => super::geo_utm::decode(s)?,
            };
            let mut out = out_text(format!("{},{}", fixed(lat, 6), fixed(lon, 6)));
            out.insert("latitude".into(), PortValue::Number(lat));
            out.insert("longitude".into(), PortValue::Number(lon));
            out.insert(
                "json".into(),
                PortValue::Json(json!({"latitude":lat,"longitude":lon,"bounds":extra})),
            );
            Ok(out)
        } else {
            let (lat, lon) = coordinate(s)?;
            let out = match self.kind {
                0 => format!("{}, {}", dms(lat, true), dms(lon, false)),
                1 => hash_encode(
                    lat,
                    lon,
                    numeric_support::number(p, "precision", 11, 1, 12)? as usize,
                ),
                2 => olc_encode(
                    lat,
                    lon,
                    numeric_support::number(p, "codeLength", 10, 2, 15)? as usize,
                )?,
                3 => maiden_encode(
                    lat,
                    lon,
                    numeric_support::number(p, "pairs", 3, 2, 4)? as usize,
                ),
                _ => super::geo_utm::encode(lat, lon)?,
            };
            Ok(out_text(out))
        }
    }
}
pub fn register(reg: &mut NodeRegistry) {
    let defs=[
        ("geo_dms","geoDms","度分秒 DMS","纬度,经度与度分秒互转；输出秒保留 3 位，解码文本保留 6 位小数，数值端口保留精度。","57°38'56.4\"N, 10°24'25.2\"E",vec![]),
        ("geo_hash","geoHash","Geohash","地理 Base32 交替二分；解码返回格子中心点及边界，文本保留 6 位小数。", "u4pruydqqvj",vec![help(ParamSpec::number("precision","编码长度",1.0,12.0,1.0,11.0),"1–12 个字符；越长精度越高。解码以输入自身长度为准。")]),
        ("geo_plus_code","geoPlusCode","Plus Code / OLC","Open Location Code 全码；支持填充及 4×5 网格细分，解码输出中心点；短码需参考点，不能独立解码。","9F9GJCX4+JR",vec![help(ParamSpec::select("codeLength","显著位数",&["2","4","6","8","10","11","12","13","14","15"],"10"),"不含 +；2/4/6 位用 0 填充，11–15 位增加细分。")]),
        ("geo_maidenhead","geoMaidenhead","Maidenhead 网格","业余无线电网格编码；2/3 对兼容常用网格，4 对保留 EBCTF 字母扩展（24×24），不是十进制扩展。解码输出格子中心。","JO57fp",vec![help(ParamSpec::select("pairs","精度对数",&["2","3","4"],"3"),"2 对=4 字符，3 对=6 字符；第 4 对按上游使用 a–x 字母扩展。")]),
        ("geo_utm","geoUtm","UTM 坐标","WGS84 / Snyder 投影，按上游每 6° 分区（无挪威/斯瓦尔巴特殊分区）；纬度 -80 至 84°，东距/北距按米取整。180° 经线归第 60 区。","32V 583245 6397615",vec![]),
    ];
    for (kind, (id, upstream, name, description, sample, params)) in defs.into_iter().enumerate() {
        for decode in [false, true] {
            let outputs = if decode {
                vec![
                    t_out(),
                    req("latitude", "纬度", PortType::Number),
                    req("longitude", "经度", PortType::Number),
                    req("json", "坐标详情", PortType::Json),
                ]
            } else {
                vec![t_out()]
            };
            let mut d = document_text(
                desc(
                    &format!("{id}_{}", if decode { "decode" } else { "encode" }),
                    "numeric.address",
                    &format!("{name} {}", if decode { "解码" } else { "编码" }),
                    CYAN,
                    vec![t_in()],
                    outputs,
                    if decode { vec![] } else { params.clone() },
                ),
                upstream,
                description,
                if decode { sample } else { "57.649,10.407" },
                json!({}),
            );
            d.family = "geographic coordinates".into();
            d.aliases
                .extend(["GPS".into(), "地理坐标".into(), upstream.into()]);
            reg.register(d, Arc::new(move || Arc::new(Geo { kind, decode })));
        }
    }
}
