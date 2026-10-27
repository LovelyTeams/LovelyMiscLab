//! Local-time reports use an explicit offset so saved workflows are reproducible.
use super::{
    codec_support::{document_text, help},
    numeric_support::{bad, space},
    prelude::*,
    time_codecs::{clip, date, explicit_zone, offset, parse_iso},
};
use chrono::{Datelike, Timelike};
use serde_json::{json, Value};
struct Report(Option<bool>);
fn iso(ms: i64) -> Result<String, CoreError> {
    let d = date(ms)?;
    let year = d.year();
    let year = if (0..=9999).contains(&year) {
        format!("{year:04}")
    } else {
        format!("{year:+07}")
    };
    Ok(format!(
        "{year}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        d.month(),
        d.day(),
        d.hour(),
        d.minute(),
        d.second(),
        d.timestamp_subsec_millis()
    ))
}
impl Node for Report {
    fn run(&self, i: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let s = in_text(i, "text")?.trim_matches(space);
        if s.len() > 16384 {
            return Err(bad("时间输入最多 16384 字符"));
        }
        let zone = pstr(p, "timezone", "+08:00");
        let offset = offset(zone)?;
        let to_time = self
            .0
            .unwrap_or_else(|| regex::Regex::new(r"^-?\d+(\.\d+)?$").unwrap().is_match(s));
        let (ms, report) = if to_time {
            let n = super::float_codecs::js_number(s)?;
            let ms = clip(if n < 1e12 { n * 1000.0 } else { n })?;
            let local = date(ms + offset * 60000)?;
            let utc = iso(ms)?;
            let n = ryu_js::Buffer::new().format(n).to_string();
            (ms,format!("时间戳: {n}\n本地时间: {}/{}/{} {:02}:{:02}:{:02}\nUTC: {utc}\nUnix秒: {}\nUnix毫秒: {ms}",local.year(),local.month(),local.day(),local.hour(),local.minute(),local.second(),ms.div_euclid(1000)))
        } else {
            let normalized = s.replace('/', "-");
            let date_only = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$")
                .unwrap()
                .is_match(s);
            let ms = parse_iso(&normalized)?
                - if explicit_zone(s) || date_only {
                    0
                } else {
                    offset * 60000
                };
            let utc = iso(ms)?;
            (
                ms,
                format!(
                    "时间: {s}\nUnix时间戳(秒): {}\nUnix时间戳(毫秒): {ms}\nUTC: {utc}",
                    ms.div_euclid(1000)
                ),
            )
        };
        Ok([("text".into(),PortValue::Text(report)),("seconds".into(),PortValue::Text(ms.div_euclid(1000).to_string())),("milliseconds".into(),PortValue::Text(ms.to_string())),("data".into(),PortValue::Json(json!({"iso":iso(ms)?,"timezone":zone,"offsetMinutes":offset,"milliseconds":ms.to_string()})))].into_iter().collect())
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for (dir, id, title, sample) in [
        (None, "timestamp_report", "时间戳自动分析", "1793059200"),
        (
            Some(true),
            "timestamp_to_time",
            "时间戳转日期报告",
            "1793059200",
        ),
        (
            Some(false),
            "time_to_timestamp",
            "日期转时间戳报告",
            "2026-10-27 08:00:00",
        ),
    ] {
        let d=document_text(desc(id,"numeric.time",title,CYAN,vec![t_in()],vec![t_out(),req("seconds","Unix 秒",PortType::Text),req("milliseconds","Unix 毫秒",PortType::Text),req("data","时间明细",PortType::Json)],vec![help(ParamSpec::text("timezone","本地时间偏移","+08:00",false),"显式固定偏移，替代浏览器所在时区；纯日期按 UTC，有时区按输入，无时区时分秒按此偏移。")]),"timestamp","按固定上游规则识别秒/毫秒（数值 < 1e12 当秒，含负值）；本地时间使用显式时区偏移，保存的流程可复现。Unix 秒/毫秒另行输出供下游。",sample,json!({}));
        reg.register(d, Arc::new(move || Arc::new(Report(dir))));
    }
}
