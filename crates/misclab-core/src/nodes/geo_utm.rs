//! WGS84 / Snyder series, preserving the upstream six-degree zoning contract.
use super::{prelude::CoreError, transport_codecs::err};
const A: f64 = 6_378_137.0;
const F: f64 = 1.0 / 298.257223563;
const K: f64 = 0.9996;
const BANDS: &[u8] = b"CDEFGHJKLMNPQRSTUVWX";

pub(super) fn encode(lat: f64, lon: f64) -> Result<String, CoreError> {
    if !(-80.0..=84.0).contains(&lat) {
        return Err(err("UTM 纬度须在 -80 至 84°"));
    }
    let zone = (((lon + 180.0) / 6.0).floor() as i32 + 1).clamp(1, 60);
    let letter = BANDS[(((lat + 80.0) / 8.0).floor() as usize).min(19)] as char;
    let e2 = 2.0 * F - F * F;
    let ep = e2 / (1.0 - e2);
    let rad = lat * std::f64::consts::PI / 180.0;
    let lon0 = ((zone - 1) * 6 - 180 + 3) as f64 * std::f64::consts::PI / 180.0;
    let n = A / (1.0 - e2 * rad.sin().powi(2)).sqrt();
    let t = rad.tan().powi(2);
    let c = ep * rad.cos().powi(2);
    let a = (lon * std::f64::consts::PI / 180.0 - lon0) * rad.cos();
    let m = A
        * ((1.0 - e2 / 4.0 - 3.0 * e2.powi(2) / 64.0 - 5.0 * e2.powi(3) / 256.0) * rad
            - (3.0 * e2 / 8.0 + 3.0 * e2.powi(2) / 32.0 + 45.0 * e2.powi(3) / 1024.0)
                * (2.0 * rad).sin()
            + (15.0 * e2.powi(2) / 256.0 + 45.0 * e2.powi(3) / 1024.0) * (4.0 * rad).sin()
            - (35.0 * e2.powi(3) / 3072.0) * (6.0 * rad).sin());
    let x = K
        * n
        * (a + (1.0 - t + c) * a.powi(3) / 6.0
            + (5.0 - 18.0 * t + t * t + 72.0 * c - 58.0 * ep) * a.powi(5) / 120.0)
        + 500000.0;
    let y = K
        * (m + n
            * rad.tan()
            * (a * a / 2.0
                + (5.0 - t + 9.0 * c + 4.0 * c * c) * a.powi(4) / 24.0
                + (61.0 - 58.0 * t + t * t + 600.0 * c - 330.0 * ep) * a.powi(6) / 720.0))
        + if lat >= 0.0 { 0.0 } else { 10000000.0 };
    Ok(format!("{zone}{letter} {} {}", x.round(), y.round()))
}
pub(super) fn decode(text: &str) -> Result<(f64, f64), CoreError> {
    let s = text.trim().replace([',', '，'], " ");
    let re = regex::Regex::new(r"^(\d{1,2})\s*([A-HJ-NP-Z])\s+(\d+(?:\.\d+)?)\s+(\d+(?:\.\d+)?)$")
        .unwrap();
    let m = re
        .captures(&s)
        .ok_or_else(|| err("UTM 格式需要 31U 448251 5411937"))?;
    let zone = m[1].parse::<u8>().map_err(|_| err("UTM 分区无效"))?;
    let band = BANDS
        .iter()
        .position(|&c| c == m[2].as_bytes()[0])
        .ok_or_else(|| err("UTM 字母带须 C–X（跳过 I/O）"))?;
    let easting = m[3].parse::<f64>().map_err(|_| err("东距无效"))?;
    let northing = m[4].parse::<f64>().map_err(|_| err("北距无效"))?;
    if !(1..=60).contains(&zone)
        || !(0.0..=1_000_000.0).contains(&easting)
        || !(0.0..=10_000_000.0).contains(&northing)
    {
        return Err(err(
            "UTM 分区须 1–60，东距 0–1000000 米，北距 0–10000000 米",
        ));
    }
    let e2 = 2.0 * F - F * F;
    let ep = e2 / (1.0 - e2);
    let e1 = (1.0 - (1.0 - e2).sqrt()) / (1.0 + (1.0 - e2).sqrt());
    let x = easting - 500000.0;
    let y = if band >= 10 {
        northing
    } else {
        northing - 10000000.0
    };
    let mu = (y / K) / (A * (1.0 - e2 / 4.0 - 3.0 * e2.powi(2) / 64.0 - 5.0 * e2.powi(3) / 256.0));
    let foot = mu
        + (3.0 * e1 / 2.0 - 27.0 * e1.powi(3) / 32.0) * (2.0 * mu).sin()
        + (21.0 * e1.powi(2) / 16.0 - 55.0 * e1.powi(4) / 32.0) * (4.0 * mu).sin()
        + (151.0 * e1.powi(3) / 96.0) * (6.0 * mu).sin()
        + (1097.0 * e1.powi(4) / 512.0) * (8.0 * mu).sin();
    let n = A / (1.0 - e2 * foot.sin().powi(2)).sqrt();
    let t = foot.tan().powi(2);
    let c = ep * foot.cos().powi(2);
    let r = A * (1.0 - e2) / (1.0 - e2 * foot.sin().powi(2)).powf(1.5);
    let d = x / (n * K);
    let lat = foot
        - (n * foot.tan() / r)
            * (d * d / 2.0
                - (5.0 + 3.0 * t + 10.0 * c - 4.0 * c * c - 9.0 * ep) * d.powi(4) / 24.0
                + (61.0 + 90.0 * t + 298.0 * c + 45.0 * t * t - 252.0 * ep - 3.0 * c * c)
                    * d.powi(6)
                    / 720.0);
    let lon0 = ((i32::from(zone) - 1) * 6 - 180 + 3) as f64 * std::f64::consts::PI / 180.0;
    let lon = lon0
        + (d - (1.0 + 2.0 * t + c) * d.powi(3) / 6.0
            + (5.0 - 2.0 * c + 28.0 * t - 3.0 * c * c + 8.0 * ep + 24.0 * t * t) * d.powi(5)
                / 120.0)
            / foot.cos();
    let lat = lat * 180.0 / std::f64::consts::PI;
    let lon = lon * 180.0 / std::f64::consts::PI;
    // Encoding rounds projected coordinates to metres. At the antimeridian,
    // that rounding can put the inverse a fraction of a metre beyond 180°.
    if !(-90.0..=90.0).contains(&lat) || !(-180.00001..=180.00001).contains(&lon) {
        return Err(err("UTM 解码坐标越界"));
    }
    Ok((lat, lon.clamp(-180.0, 180.0)))
}
