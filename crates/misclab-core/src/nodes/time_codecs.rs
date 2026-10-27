//! Offline timestamp codecs. Integer epochs never pass through a JSON number.
use super::{
    codec_support::{document_text, help},
    numeric_support::{bad, integer, space},
    prelude::*,
};
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Timelike, Utc};
use num_traits::ToPrimitive;
use serde_json::{json, Value};
const EPOCH_MS: i128 = 11_644_473_600_000;
fn numeric(s: &str) -> Result<f64, CoreError> {
    let s = s.trim_matches(space);
    if s.is_empty() {
        return Err(bad("时间数字不能为空"));
    }
    let n = s.parse::<f64>().map_err(|_| bad("时间数字无效"))?;
    if !n.is_finite() {
        return Err(bad("需要有限数值"));
    }
    Ok(n)
}
fn bigint(s: &str) -> Result<i128, CoreError> {
    let s = s.trim_matches(space);
    let t = s.to_ascii_lowercase();
    let n = if let Some(h) = t.strip_prefix("0x") {
        integer(h, 16)?
    } else if let Some(h) = t.strip_prefix("0o") {
        integer(h, 8)?
    } else if let Some(h) = t.strip_prefix("0b") {
        integer(h, 2)?
    } else {
        integer(s, 10)?
    };
    n.to_i128().ok_or_else(|| bad("时间整数超出范围"))
}
pub(super) fn date(ms: i64) -> Result<DateTime<Utc>, CoreError> {
    DateTime::from_timestamp_millis(ms).ok_or_else(|| bad("日期超出可表示范围"))
}
pub(super) fn clip(n: f64) -> Result<i64, CoreError> {
    if !n.is_finite() || n.abs() > 8_640_000_000_000_000.0 {
        return Err(bad("时间戳超出日期范围"));
    }
    let ms = n.trunc() as i64;
    date(ms)?;
    Ok(ms)
}
fn int_ms(n: i128) -> Result<i64, CoreError> {
    let n = i64::try_from(n).map_err(|_| bad("时间戳超出范围"))?;
    date(n)?;
    Ok(n)
}
pub(super) fn explicit_zone(s: &str) -> bool {
    s.ends_with(['z', 'Z']) || regex::Regex::new(r"[+-]\d{2}:?\d{2}$").unwrap().is_match(s)
}
pub(super) fn parse_iso(s: &str) -> Result<i64, CoreError> {
    let mut s = s.trim_matches(space).replace(' ', "T");
    if s.is_empty() {
        return Err(bad("时间不能为空"));
    }
    if s.ends_with('z') {
        s.pop();
        s.push('Z');
    }
    if regex::Regex::new(r"[+-]\d{4}$").unwrap().is_match(&s) {
        s.insert(s.len() - 2, ':');
    }
    if let Ok(d) = DateTime::parse_from_rfc3339(&s) {
        return Ok(d.timestamp_millis());
    }
    let naive = s.strip_suffix('Z').unwrap_or(&s);
    for format in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%dT%H:%M"] {
        if let Ok(d) = NaiveDateTime::parse_from_str(naive, format) {
            return Ok(d.and_utc().timestamp_millis());
        }
    }
    NaiveDate::parse_from_str(naive, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|d| d.and_utc().timestamp_millis())
        .ok_or_else(|| bad("无效 ISO8601 日期；无时区输入按 UTC"))
}
fn iso(ms: i64) -> Result<String, CoreError> {
    let d = date(ms)?;
    let y = d.year();
    let ys = if (0..=9999).contains(&y) {
        format!("{y:04}")
    } else {
        y.to_string()
    };
    Ok(format!(
        "{ys}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        d.month(),
        d.day(),
        d.hour(),
        d.minute(),
        d.second()
    ))
}
pub(super) fn offset(s: &str) -> Result<i64, CoreError> {
    let s = s.trim_matches(space).to_ascii_uppercase();
    if ["", "UTC", "GMT", "Z"].contains(&s.as_str()) {
        return Ok(0);
    }
    let r = regex::Regex::new(r"^([+-])(\d{1,2})(?::?(\d{2}))?$").unwrap();
    let c = r
        .captures(&s)
        .ok_or_else(|| bad("时区需要 UTC 或 ±HH:MM 偏移"))?;
    let h = c[2].parse::<i64>().unwrap();
    let m = c.get(3).map_or(0, |v| v.as_str().parse::<i64>().unwrap());
    if h > 23 || m > 59 {
        return Err(bad("时区偏移超出 23:59"));
    }
    Ok((h * 60 + m) * if &c[1] == "-" { -1 } else { 1 })
}
fn rounded6(v: f64) -> String {
    if v.fract() == 0.0 {
        ryu_js::Buffer::new().format(v).to_string()
    } else {
        format!("{v:.6}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .into()
    }
}
fn cn(n: u32) -> String {
    const D: [char; 10] = ['〇', '一', '二', '三', '四', '五', '六', '七', '八', '九'];
    if n < 10 {
        D[n as usize].to_string()
    } else {
        format!(
            "{}十{}",
            if n < 20 {
                String::new()
            } else {
                D[(n / 10) as usize].to_string()
            },
            if n.is_multiple_of(10) {
                String::new()
            } else {
                D[(n % 10) as usize].to_string()
            }
        )
    }
}
fn digit(c: char) -> Result<u32, CoreError> {
    if c == '零' {
        return Ok(0);
    }
    "〇一二三四五六七八九"
        .chars()
        .position(|d| d == c)
        .map(|d| d as u32)
        .ok_or_else(|| bad("无效汉字数字"))
}
fn cn_number(s: &str) -> Result<u32, CoreError> {
    if let Some((a, b)) = s.split_once('十') {
        let a = if a.is_empty() {
            1
        } else if a.chars().count() == 1 {
            digit(a.chars().next().unwrap())?
        } else {
            return Err(bad("汉字数位无效"));
        };
        let b = if b.is_empty() {
            0
        } else if b.chars().count() == 1 {
            digit(b.chars().next().unwrap())?
        } else {
            return Err(bad("汉字数位无效"));
        };
        Ok(a * 10 + b)
    } else if s.chars().count() == 1 {
        digit(s.chars().next().unwrap())
    } else {
        Err(bad("汉字数位无效"))
    }
}
#[derive(Clone, Copy)]
enum Kind {
    Unix,
    Filetime,
    Hfs,
    Cocoa,
    Dos,
    Chinese,
    Zone,
    Julian,
    Excel,
    Chrome,
    Snowflake,
}
struct Codec {
    kind: Kind,
    decode: bool,
}
impl Node for Codec {
    fn run(&self, i: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let s = in_text(i, "text")?.trim_matches(space);
        if s.len() > 16384 {
            return Err(bad("时间输入最多 16384 字符"));
        }
        let mut data = json!({});
        let mut timestamp = None;
        let text = match self.kind {
            Kind::Zone => {
                let from = offset(pstr(p, "fromTz", "UTC"))?;
                let to = offset(pstr(p, "toTz", "+08:00"))?;
                let ms = parse_iso(s)? - if explicit_zone(s) { 0 } else { from * 60_000 };
                timestamp = Some(ms);
                let local = iso(ms + to * 60_000)?;
                format!(
                    "{}{}{:02}:{:02}",
                    local.trim_end_matches('Z'),
                    if to < 0 { '-' } else { '+' },
                    to.abs() / 60,
                    to.abs() % 60
                )
            }
            Kind::Snowflake => {
                if s.is_empty() {
                    "（空输入）".into()
                } else {
                    let id = bigint(s)?;
                    if !(0..1i128 << 63).contains(&id) {
                        return Err(bad("雪花 ID 需要 63 位非负整数"));
                    }
                    let ep = pstr(p, "epoch", "twitter");
                    let epoch = match ep {
                        "twitter" => 1_288_834_974_657i64,
                        "discord" => 1_420_070_400_000,
                        "custom" => clip(numeric(pstr(p, "customEpoch", "1288834974657"))?)?,
                        _ => return Err(bad("未知雪花纪元")),
                    };
                    let delta = (id >> 22) as i64;
                    let ms = epoch
                        .checked_add(delta)
                        .ok_or_else(|| bad("雪花时间溢出"))?;
                    let dc = (id >> 17) & 31;
                    let worker = (id >> 12) & 31;
                    let seq = id & 4095;
                    let name = match ep {
                        "twitter" => "Twitter",
                        "discord" => "Discord",
                        _ => "自定义",
                    };
                    let time = iso(ms)?;
                    timestamp = Some(ms);
                    data = json!({"id":id.to_string(),"timestampOffset":delta,"epoch":epoch.to_string(),"datacenter":dc as u32,"worker":worker as u32,"sequence":seq as u32});
                    format!("=== 雪花 ID 解析 ===\n输入: {id}\nID (hex): 0x{id:X}\n--- 字段分解 ---\n时间戳偏移 (ms): {delta}\n纪元: {name} ({epoch})\n生成时间 (UTC): {time}\n数据中心 ID: {dc}\n工作节点 ID: {worker}\n序列号: {seq}\n--- 位拆分 ---\nbits 63-22 (timestamp): {delta}\nbits 21-17 (datacenter): {dc}\nbits 16-12 (worker): {worker}\nbits 11-0  (sequence): {seq}")
                }
            }
            kind => {
                if self.decode {
                    let ms = match kind {
                        Kind::Unix => {
                            let n = numeric(s)?;
                            let unit = pstr(p, "unit", "auto");
                            let unit = if unit == "auto" {
                                if n.abs() < 1e10 {
                                    "sec"
                                } else if n.abs() < 1e13 {
                                    "ms"
                                } else {
                                    "us"
                                }
                            } else {
                                unit
                            };
                            clip(match unit {
                                "sec" => n * 1000.0,
                                "ms" => n,
                                "us" => n / 1000.0,
                                _ => return Err(bad("未知 Unix 时间单位")),
                            })?
                        }
                        Kind::Filetime => {
                            let n = bigint(s)?;
                            if !(0..=u64::MAX as i128).contains(&n) {
                                return Err(bad("FILETIME 必须是 64 位无符号整数"));
                            }
                            int_ms(n / 10000 - EPOCH_MS)?
                        }
                        Kind::Chrome => int_ms(bigint(s)? / 1000 - EPOCH_MS)?,
                        Kind::Hfs => {
                            let n = numeric(s)?;
                            if !(0.0..=u32::MAX as f64).contains(&n) {
                                return Err(bad("HFS+ 必须在 32 位无符号范围"));
                            }
                            clip((n - 2_082_844_800.0) * 1000.0)?
                        }
                        Kind::Cocoa => clip((numeric(s)? + 978_307_200.0) * 1000.0)?,
                        Kind::Dos => {
                            let n = u32::try_from(bigint(s)?)
                                .map_err(|_| bad("DOS 日期需要 32 位无符号整数"))?;
                            let t = n & 65535;
                            let d = n >> 16;
                            NaiveDate::from_ymd_opt(
                                ((d >> 9) & 127) as i32 + 1980,
                                (d >> 5) & 15,
                                d & 31,
                            )
                            .and_then(|d| {
                                d.and_hms_opt((t >> 11) & 31, (t >> 5) & 63, (t & 31) * 2)
                            })
                            .map(|d| d.and_utc().timestamp_millis())
                            .ok_or_else(|| bad("DOS 日期或时分秒无效"))?
                        }
                        Kind::Chinese => {
                            let (y, rest) = s
                                .split_once('年')
                                .ok_or_else(|| bad("汉字日期需要 年月日"))?;
                            let (m, d) =
                                rest.split_once('月').ok_or_else(|| bad("汉字日期缺少月"))?;
                            let d = d.strip_suffix('日').ok_or_else(|| bad("汉字日期缺少日"))?;
                            if y.is_empty() || y.chars().count() > 4 {
                                return Err(bad("汉字年份需要 1..4 位数字"));
                            }
                            let mut year = 0;
                            for c in y.chars() {
                                year = year * 10 + digit(c)? as i32;
                            }
                            NaiveDate::from_ymd_opt(year, cn_number(m)?, cn_number(d)?)
                                .and_then(|d| d.and_hms_opt(0, 0, 0))
                                .map(|d| d.and_utc().timestamp_millis())
                                .ok_or_else(|| bad("汉字日期不在有效日历范围"))?
                        }
                        Kind::Julian => {
                            let jd = numeric(s)?;
                            let z = (jd + 0.5).floor();
                            let f = jd + 0.5 - z;
                            let a = if z < 2_299_161.0 {
                                z
                            } else {
                                let alpha = ((z - 1_867_216.25) / 36524.25).floor();
                                z + 1.0 + alpha - (alpha / 4.0).floor()
                            };
                            let b = a + 1524.0;
                            let c = ((b - 122.1) / 365.25).floor();
                            let dd = (365.25 * c).floor();
                            let e = ((b - dd) / 30.6001).floor();
                            let day = b - dd - (30.6001 * e).floor();
                            let month = if e < 14.0 { e - 1.0 } else { e - 13.0 };
                            let year = if month > 2.0 { c - 4716.0 } else { c - 4715.0 };
                            if year.abs() > 262000.0 {
                                return Err(bad("儒略日超出日期范围"));
                            }
                            let base =
                                NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32)
                                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                                    .ok_or_else(|| bad("儒略日无效"))?
                                    .and_utc()
                                    .timestamp_millis();
                            base + (f * 86_400_000.0).round() as i64
                        }
                        Kind::Excel => {
                            let n = numeric(s)?;
                            let system = pstr(p, "system", "1900");
                            let value = match system {
                                "1904" => (n - 24107.0) * 86_400_000.0,
                                "1900" => {
                                    if (60.0..61.0).contains(&n) {
                                        return Err(bad("Excel 序列 60 是不存在的 1900-02-29"));
                                    }
                                    (n - 25569.0) * 86_400_000.0
                                        + if n < 60.0 { 86_400_000.0 } else { 0.0 }
                                }
                                _ => return Err(bad("未知 Excel 日期系统")),
                            };
                            clip((value + 0.5).floor())?
                        }
                        _ => unreachable!(),
                    };
                    timestamp = Some(ms);
                    iso(ms)?
                } else {
                    let ms = parse_iso(s)?;
                    timestamp = Some(ms);
                    let d = date(ms)?;
                    match kind {
                        Kind::Unix => match pstr(p, "unit", "auto") {
                            "auto" | "sec" => ms.div_euclid(1000).to_string(),
                            "ms" => ms.to_string(),
                            "us" => (i128::from(ms) * 1000).to_string(),
                            _ => return Err(bad("未知 Unix 时间单位")),
                        },
                        Kind::Filetime => {
                            let n = (i128::from(ms) + EPOCH_MS) * 10000;
                            if !(0..=u64::MAX as i128).contains(&n) {
                                return Err(bad("日期不在 FILETIME 无符号范围"));
                            }
                            n.to_string()
                        }
                        Kind::Chrome => (i128::from(ms) * 1000 + EPOCH_MS * 1000).to_string(),
                        Kind::Hfs => {
                            let n = ms.div_euclid(1000) + 2_082_844_800;
                            if !(0..=u32::MAX as i64).contains(&n) {
                                return Err(bad("日期不在 HFS+ 无符号范围"));
                            }
                            n.to_string()
                        }
                        Kind::Cocoa => (ms.div_euclid(1000) - 978_307_200).to_string(),
                        Kind::Dos => {
                            if !(1980..=2107).contains(&d.year()) {
                                return Err(bad("DOS 日期年份需要 1980..2107"));
                            }
                            let days = ((d.year() as u32 - 1980) << 9) | (d.month() << 5) | d.day();
                            let time = (d.hour() << 11) | (d.minute() << 5) | (d.second() / 2);
                            ((days << 16) | time).to_string()
                        }
                        Kind::Chinese => {
                            if !(0..=9999).contains(&d.year()) {
                                return Err(bad("汉字日期年份需要 0..9999"));
                            }
                            let year = d
                                .year()
                                .to_string()
                                .chars()
                                .map(|c| {
                                    "〇一二三四五六七八九"
                                        .chars()
                                        .nth(c.to_digit(10).unwrap() as usize)
                                        .unwrap()
                                })
                                .collect::<String>();
                            format!("{year}年{}月{}日", cn(d.month()), cn(d.day()))
                        }
                        Kind::Julian => {
                            let (mut y, mut m) = (d.year() as f64, d.month() as f64);
                            if m <= 2.0 {
                                y -= 1.0;
                                m += 12.0;
                            }
                            let a = (y / 100.0).floor();
                            let b = 2.0 - a + (a / 4.0).floor();
                            let jd = (365.25 * (y + 4716.0)).floor()
                                + (30.6001 * (m + 1.0)).floor()
                                + d.day() as f64
                                + b
                                - 1524.5
                                + (d.hour() as f64
                                    + d.minute() as f64 / 60.0
                                    + (d.second() as f64
                                        + d.timestamp_subsec_millis() as f64 / 1000.0)
                                        / 3600.0)
                                    / 24.0;
                            rounded6(jd)
                        }
                        Kind::Excel => {
                            let v = match pstr(p, "system", "1900") {
                                "1904" => ms as f64 / 86_400_000.0 + 24107.0,
                                "1900" => {
                                    ms as f64 / 86_400_000.0 + 25569.0
                                        - if d.year() < 1900 || (d.year() == 1900 && d.month() < 3)
                                        {
                                            1.0
                                        } else {
                                            0.0
                                        }
                                }
                                _ => return Err(bad("未知 Excel 日期系统")),
                            };
                            rounded6(v)
                        }
                        _ => unreachable!(),
                    }
                }
            }
        };
        let ms = timestamp.map(|ms| ms.to_string()).unwrap_or_default();
        if let Some(t) = timestamp {
            data["iso"] = json!(iso(t)?);
            data["isoMilliseconds"] =
                json!(date(t)?.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
            data["unixMilliseconds"] = json!(ms);
        }
        Ok([
            ("text".into(), PortValue::Text(text)),
            ("milliseconds".into(), PortValue::Text(ms)),
            ("data".into(), PortValue::Json(data)),
        ]
        .into_iter()
        .collect())
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for(kind,stem,upstream,title,sample,encoded,description)in[
 (Kind::Unix,"unix_time","unixTime","Unix 时间","2026-10-27T00:00:00Z","1793059200","ISO8601 与 Unix 秒/毫秒/微秒互转。auto 按绝对值区间识别；无时区按 UTC，支持负时间戳。"),
 (Kind::Filetime,"windows_filetime","filetime","Windows FILETIME","2026-10-27T00:00:00Z","134374464000000000","1601 纪元、100ns 单位、64 位无符号；整数精确解析。ISO 文本按秒显示，metadata 保留毫秒。"),
 (Kind::Hfs,"hfs_time","hfsTime","Mac HFS+ 时间","2026-10-27T00:00:00Z","3875904000","1904 纪元、32 位无符号秒；校验 1904..2040 范围。"),
 (Kind::Cocoa,"cocoa_time","cocoaTime","Cocoa 时间","2026-10-27T00:00:00Z","814752000","2001 纪元、有符号秒；早于纪元可用负值。"),
 (Kind::Dos,"dos_datetime","dosDateTime","DOS 日期时间","2026-10-27T12:34:56Z","1565557852","FAT 32 位日期时间打包；有效年份 1980..2107，秒精度为 2 秒，严格验证日期和时间。"),
 (Kind::Chinese,"chinese_date","chineseDate","汉字日期","2026-10-27","二〇二六年十月二十七日","逐位汉字年份与十位月日互转；仅日期，UTC 午夜，0..9999 年。"),
 (Kind::Julian,"julian_date","julianDate","儒略日","2000-01-01T12:00:00Z","2451545","JD 与 ISO8601 互转；编码按公历，解码在 1582-10-15 前采用儒略历，沿用上游改历分支；小数最多 6 位。"),
 (Kind::Excel,"excel_date","excelDate","Excel 序列日期","1970-01-01T12:00:00Z","25569.5","支持 1900/1904 系统，修正 1900 年 1/2 月偏移；拒绝整个虚构的序列 60 日，包含小数时间。"),
 (Kind::Chrome,"chrome_time","chromeTime","Chrome 时间","2026-10-27T00:00:00Z","13437446400000000","1601 纪元、有符号微秒，精确整数运算；按上游截断不足毫秒部分。"),
 (Kind::Zone,"timezone_convert","tzConvert","时区转换","2026-10-27T00:00:00","","支持 UTC/GMT/Z 与 ±HH:MM；无时区输入应用 fromTz，有时区输入保持其实际时刻。只按固定偏移，不推测夏令时。"),
 (Kind::Snowflake,"snowflake_parse","snowflakeId","雪花 ID 解析","175928847299117063","","63 位非负 ID 拆分时间偏移、数据中心、工作节点、序列号；Twitter/Discord/自定义毫秒纪元，报告和 JSON 可接续。") ]{
 let single=matches!(kind,Kind::Zone|Kind::Snowflake);for decode in if single{vec![false]}else{vec![false,true]}{
 let params=match kind{
 Kind::Unix=>vec![help(ParamSpec::select("unit","单位",&["auto","sec","ms","us"],"auto"),"auto 解码按 1e10/1e13 阈值判断；编码 auto 输出秒。")],
 Kind::Excel=>vec![help(ParamSpec::select("system","日期系统",&["1900","1904"],"1900"),"1900 包含闰年兼容修正；1904 是旧 Mac 系统。")],
 Kind::Zone=>vec![help(ParamSpec::text("fromTz","源时区","UTC",false),"无时区输入的固定偏移，例 +8、-05:30。"),help(ParamSpec::text("toTz","目标时区","+08:00",false),"目标固定偏移，例 UTC、+08:00。")],
 Kind::Snowflake=>vec![help(ParamSpec::select("epoch","纪元",&["twitter","discord","custom"],"twitter"),"Twitter 为 1288834974657，Discord 为 1420070400000 毫秒。"),help(ParamSpec::text("customEpoch","自定义纪元","1288834974657",false),"epoch=custom 时使用的 Unix 毫秒整数。")],_=>vec![]};
 let id=if single{stem.into()}else{format!("{stem}_{}",if decode{"decode"}else{"encode"})};let label=if single{title.into()}else{format!("{title} {}",if decode{"解码"}else{"编码"})};
 let d=document_text(desc(&id,"numeric.time",&label,CYAN,vec![t_in()],vec![t_out(),req("milliseconds","Unix 毫秒",PortType::Text),req("data","时间明细",PortType::Json)],params),upstream,description,if decode{encoded}else{sample},json!({}));reg.register(d,Arc::new(move||Arc::new(Codec{kind,decode})));
 }}
}
