use super::prelude::*;
use crate::{
    cancel::CancellationToken,
    graph::executor::GraphExecutor,
    progress::{NullSink, ProgressEvent, ProgressSink},
};
use serde_json::{json, Value};
// Node's ICU TextDecoder used to capture these references differs from WHATWG
// browser decoders on legacy C1/control and private-use mappings. Assert exact
// reviewed browser-standard alternatives, never ignore an entire result.
fn reviewed_reference(v: &Value) -> String {
    let mut text = v["text"].as_str().unwrap().to_string();
    if v["node"] == "charset_probe_decode" {
        let s = v["input"]["data"]["value"].as_str().unwrap();
        let corrections: &[(&str, &str)] = match s {
            "e4b8ade69687" => &[
                ("latin1", "ä¸\u{ad}æ–‡"),
                ("iso-8859-1", "ä¸\u{ad}æ–‡"),
                ("windows-1252", "ä¸\u{ad}æ–‡"),
                ("euc-jp", "筝㊥\u{fffd}\u{fffd}  （含替换符 U+FFFD）"),
            ],
            "FFFE4100" => &[
                ("gbk", "\u{fffd}嗀\0  （含替换符 U+FFFD）"),
                ("big5", "\u{fffd}爕\0  （含替换符 U+FFFD）"),
            ],
            "FEFF0041" => &[
                ("gbk", "\u{fffd}\0A  （含替换符 U+FFFD）"),
                ("big5", "\u{fffd}\0A  （含替换符 U+FFFD）"),
            ],
            "ff00" => &[
                ("gbk", "\u{fffd}\0  （含替换符 U+FFFD）"),
                ("big5", "\u{fffd}\0  （含替换符 U+FFFD）"),
            ],
            "800081829F" => &[
                ("latin1", "€\0\u{81}‚Ÿ"),
                ("iso-8859-1", "€\0\u{81}‚Ÿ"),
                ("windows-1252", "€\0\u{81}‚Ÿ"),
                ("big5", "\u{fffd}\0\u{fffd}\u{fffd}  （含替换符 U+FFFD）"),
                ("shift_jis", "\u{80}\0≠\u{fffd}  （含替换符 U+FFFD）"),
                (
                    "euc-jp",
                    "\u{fffd}\0\u{fffd}\u{fffd}\u{fffd}  （含替换符 U+FFFD）",
                ),
            ],
            _ => &[],
        };
        for &(charset, expected) in corrections {
            let prefix = format!("{charset} 解码: ");
            text = text
                .lines()
                .map(|line| {
                    if line.starts_with(&prefix) {
                        let replacement = format!("{prefix}{expected}");
                        assert_ne!(
                            line, replacement,
                            "correction must describe a real runtime difference"
                        );
                        replacement
                    } else {
                        line.into()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
        }
    }
    if v["input"]["text"]["value"] == "-1000000000000" {
        // An explicit +08:00 offset has no historical Shanghai +08:05:43 LMT.
        text = text.replace(
            "本地时间: 29720/4/6 06:19:03",
            "本地时间: -29719/4/6 06:13:20",
        );
    }
    text
}
fn call(id: &str, port: &str, input: PortValue, p: &Value) -> Result<PortMap, CoreError> {
    GraphExecutor::run_node(
        &super::default_registry(),
        id,
        &one(port, input),
        p,
        &NullSink,
        &CancellationToken::new(),
    )
}
fn run(id: &str, s: &str, p: &Value) -> Result<PortMap, CoreError> {
    call(id, "text", PortValue::Text(s.into()), p)
}
#[test]
fn pinned_float_charset_vectors_cover_five_operations() {
    let f: Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/ebctf-float-charset-batch8.json"
    ))
    .unwrap();
    assert_eq!(f["revision"], "ea2bc36dae31c97060f7f2cddad8df5518afad7b");
    let vs = f["vectors"].as_array().unwrap();
    assert_eq!(vs.len(), 350);
    assert_eq!(
        vs.iter()
            .map(|v| v["operation"].as_str().unwrap())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        5
    );
    let registry = super::default_registry();
    let mut errors = Vec::new();
    for (i, v) in vs.iter().enumerate() {
        let id = v["node"].as_str().unwrap();
        let result = GraphExecutor::run_node(
            &registry,
            id,
            &serde_json::from_value(v["input"].clone()).unwrap(),
            &v["params"],
            &NullSink,
            &CancellationToken::new(),
        );
        if v.get("error").is_some() {
            if result.is_ok() {
                errors.push(format!("vector {i} {id}: expected error"));
            }
            continue;
        }
        match result {
            Err(e) => errors.push(format!("vector {i} {id}: {e}")),
            Ok(o) => {
                if o["text"].as_text().unwrap() != reviewed_reference(v) {
                    errors.push(format!(
                        "vector {i} {id} input {:?}: actual {:?}, expected {:?}",
                        v["input"],
                        o["text"].as_text().unwrap(),
                        v["text"]
                    ));
                }
            }
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
#[test]
fn standard_half_rounding_subnormals_and_special_values() {
    let p = json!({"precision":"half"});
    for (s, hex) in [
        ("0", "0000"),
        ("-0", "8000"),
        ("1", "3C00"),
        ("-2", "C000"),
        ("0.000000059604644775390625", "0001"),
        ("0.00006103515625", "0400"),
        ("65504", "7BFF"),
        ("65520", "7C00"),
        ("1.00048828125", "3C00"),
        ("1.00146484375", "3C02"),
        ("Infinity", "7C00"),
        ("-Infinity", "FC00"),
        ("1e-100", "0000"),
    ] {
        assert_eq!(
            run("ieee754_encode", s, &p).unwrap()["text"]
                .as_text()
                .unwrap(),
            hex,
            "{s}"
        );
    }
    for (hex, s) in [
        ("0001", "5.960464477539063e-8"),
        ("0400", "0.00006103515625"),
        ("7C00", "Infinity"),
        ("FC00", "-Infinity"),
        ("7E00", "NaN"),
        ("8000", "0"),
    ] {
        assert_eq!(
            run("ieee754_decode", hex, &p).unwrap()["text"]
                .as_text()
                .unwrap(),
            s,
            "{hex}"
        );
    }
    assert_eq!(
        run(
            "ieee754_decode",
            "0001",
            &json!({"precision":"half","halfMode":"upstream"})
        )
        .unwrap()["text"]
            .as_text()
            .unwrap(),
        "16"
    );
    for (s, p) in [("Infinity", json!({})), ("1", json!({"precision":65}))] {
        assert!(run("decimal_to_radix_float", s, &p).is_err());
    }
    for (s, expected) in [
        ("1.005", "1"),
        ("2.675", "2.67"),
        ("-0.001", "-0"),
        ("0.125", "0.13"),
    ] {
        assert_eq!(
            run(
                "radix_float_to_decimal",
                s,
                &json!({"base":10,"precision":2})
            )
            .unwrap()["decimal"]
                .as_text()
                .unwrap(),
            expected
        );
    }
}
#[test]
fn charset_probe_encodes_real_legacy_charsets_and_keeps_raw_input() {
    for (s, enc, hex) in [
        ("中文", "gbk", "d6d0cec4"),
        ("中文", "big5", "a4a4a4e5"),
        ("日本", "shift_jis", "93fa967b"),
        ("日本", "euc-jp", "c6fccbdc"),
        ("€", "windows-1252", "80"),
    ] {
        let o = run("charset_probe_encode", s, &json!({})).unwrap();
        let PortValue::Json(rows) = &o["rows"] else {
            panic!()
        };
        let row = rows
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["charset"] == enc)
            .unwrap();
        assert_eq!(row["hex"], hex);
        let o = call(
            "charset_probe_decode",
            "data",
            PortValue::Bytes(hex::decode(hex).unwrap().into()),
            &json!({}),
        )
        .unwrap();
        let PortValue::Json(rows) = &o["rows"] else {
            panic!()
        };
        assert_eq!(
            rows.as_array()
                .unwrap()
                .iter()
                .find(|r| r["charset"] == enc)
                .unwrap()["text"],
            s
        );
    }
    let o = run("charset_probe_encode", "🙂", &json!({})).unwrap();
    let PortValue::Json(rows) = &o["rows"] else {
        panic!()
    };
    let gbk = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["charset"] == "gbk")
        .unwrap();
    assert_eq!(gbk["status"], "unrepresentable");
    assert!(gbk.get("hex").is_none());
    for (bytes, enc) in [
        (vec![0xef, 0xbb, 0xbf, 0x41], "utf-8"),
        (vec![0xff, 0xfe, 0x41, 0], "utf-16le"),
        (vec![0xfe, 0xff, 0, 0x41], "utf-16be"),
    ] {
        let o = call(
            "charset_probe_decode",
            "data",
            PortValue::Bytes(bytes.into()),
            &json!({}),
        )
        .unwrap();
        let PortValue::Json(rows) = &o["rows"] else {
            panic!()
        };
        assert_eq!(
            rows.as_array()
                .unwrap()
                .iter()
                .find(|r| r["charset"] == enc)
                .unwrap()["text"],
            "A"
        );
    }
    let o = call(
        "charset_probe_decode",
        "data",
        PortValue::Bytes((0..=255).collect()),
        &json!({}),
    )
    .unwrap();
    assert!(matches!(&o["rows"],PortValue::Json(v)if v.as_array().unwrap().len()==11));
    assert!(call(
        "charset_probe_decode",
        "data",
        PortValue::Text("0xGG".into()),
        &json!({})
    )
    .is_err());
}
#[test]
fn timestamp_reports_use_explicit_offsets_and_reject_invalid_dates() {
    for (s, zone, ms) in [
        ("1970-01-01 00:00:00", "+08:00", "-28800000"),
        ("1970-01-01T00:00:00", "-05:30", "19800000"),
        ("1970-01-01", "+08:00", "0"),
        ("1970-01-01T00:00:00Z", "+08:00", "0"),
        ("1970-01-01T00:00:00+02:00", "-05:30", "-7200000"),
    ] {
        assert_eq!(
            run("time_to_timestamp", s, &json!({"timezone":zone})).unwrap()["milliseconds"]
                .as_text()
                .unwrap(),
            ms
        );
    }
    for s in ["2026-02-30", "2026-13-01", "中文", ""] {
        assert!(run("time_to_timestamp", s, &json!({})).is_err());
    }
    assert!(run("timestamp_to_time", "Infinity", &json!({})).is_err());
}
#[test]
fn charset_analysis_cancels_between_candidates() {
    struct Stop(CancellationToken);
    impl ProgressSink for Stop {
        fn emit(&self, e: ProgressEvent) {
            if matches!(e,ProgressEvent::NodeProgress{pct,..}if pct>0.0&&pct<1.0) {
                self.0.cancel();
            }
        }
    }
    for (id, port, input) in [
        (
            "charset_probe_encode",
            "text",
            PortValue::Text("中文🙂".repeat(100)),
        ),
        (
            "charset_probe_decode",
            "data",
            PortValue::Bytes((0..=255).collect()),
        ),
    ] {
        let cancel = CancellationToken::new();
        let result = GraphExecutor::run_node(
            &super::default_registry(),
            id,
            &one(port, input),
            &json!({}),
            &Stop(cancel.clone()),
            &cancel,
        );
        assert!(
            matches!(result, Err(CoreError::Cancelled)),
            "{id}: {result:?}"
        );
    }
}
