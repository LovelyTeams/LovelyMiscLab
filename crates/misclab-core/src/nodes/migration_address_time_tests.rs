use super::prelude::*;
use crate::{
    cancel::CancellationToken,
    graph::executor::GraphExecutor,
    progress::{NullSink, ProgressEvent, ProgressSink},
};
use serde_json::{json, Value};
fn run(id: &str, s: &str, p: &Value) -> Result<PortMap, CoreError> {
    GraphExecutor::run_node(
        &super::default_registry(),
        id,
        &one("text", PortValue::Text(s.into())),
        p,
        &NullSink,
        &CancellationToken::new(),
    )
}
#[test]
fn pinned_address_time_vectors_cover_twenty_operations() {
    let f: Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/ebctf-address-time-batch6.json"
    ))
    .unwrap();
    assert_eq!(f["revision"], "ea2bc36dae31c97060f7f2cddad8df5518afad7b");
    let vs = f["vectors"].as_array().unwrap();
    assert_eq!(vs.len(), 469);
    assert_eq!(
        vs.iter()
            .map(|v| v["operation"].as_str().unwrap())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        20
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
                if o["text"].as_text().unwrap() != v["text"].as_str().unwrap() {
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
fn dates_retain_milliseconds_and_reject_overflow_instead_of_wrapping() {
    assert_eq!(
        run("unix_time_encode", "0001-01-01", &json!({})).unwrap()["text"]
            .as_text()
            .unwrap(),
        "-62135596800"
    );
    assert_eq!(
        run("chinese_date_encode", "0099-02-28", &json!({})).unwrap()["text"]
            .as_text()
            .unwrap(),
        "九九年二月二十八日"
    );
    for stem in ["windows_filetime", "chrome_time", "unix_time"] {
        let e = run(
            &format!("{stem}_encode"),
            "2026-10-27T12:34:56.789Z",
            &json!({"unit":"ms"}),
        )
        .unwrap();
        let d = run(
            &format!("{stem}_decode"),
            e["text"].as_text().unwrap(),
            &json!({"unit":"ms"}),
        )
        .unwrap();
        assert_eq!(
            e["milliseconds"].as_text().unwrap(),
            d["milliseconds"].as_text().unwrap()
        );
        assert!(
            matches!(&d["data"],PortValue::Json(v)if v["isoMilliseconds"]=="2026-10-27T12:34:56.789Z")
        );
    }
    for (id, s, p) in [
        ("dos_datetime_encode", "2108-01-01", json!({})),
        ("dos_datetime_decode", "4294967296", json!({})),
        ("dos_datetime_decode", "0x5021003f", json!({})),
        ("chinese_date_decode", "二〇二六年二月三十一日", json!({})),
        ("excel_date_decode", "60.5", json!({})),
        ("unix_time_encode", "2026-02-30", json!({})),
        ("unix_time_decode", "Infinity", json!({})),
        ("windows_filetime_decode", "-1", json!({})),
        (
            "chrome_time_decode",
            "99999999999999999999999999999999999",
            json!({}),
        ),
        ("timezone_convert", "2026-01-01", json!({"toTz":"+25:00"})),
        ("julian_date_decode", "1e300", json!({})),
    ] {
        assert!(run(id, s, &p).is_err(), "{id} {s}");
    }
    assert_eq!(
        run(
            "unix_time_encode",
            "1969-12-31T23:59:59.999Z",
            &json!({"unit":"sec"})
        )
        .unwrap()["text"]
            .as_text()
            .unwrap(),
        "-1"
    );
    assert_eq!(
        run(
            "timezone_convert",
            "2026-01-01T00:00:00",
            &json!({"fromTz":"+08:00","toTz":"UTC"})
        )
        .unwrap()["text"]
            .as_text()
            .unwrap(),
        "2025-12-31T16:00:00+00:00"
    );
}
#[test]
fn address_reports_have_structured_fields_and_boundary_masks() {
    for (input, hosts, network) in [
        ("0.0.0.0/0", 4294967294.0, "0.0.0.0"),
        ("192.168.0.1/31", 2.0, "192.168.0.0"),
        ("192.168.0.1/32", 1.0, "192.168.0.1"),
    ] {
        let o = run("cidr_calculate", input, &json!({})).unwrap();
        assert!(matches!(&o["hosts"], PortValue::Number(n) if *n==hosts));
        assert!(matches!(&o["data"],PortValue::Json(v)if v["network"]==network));
    }
    let o = run(
        "magnet_parse",
        "magnet:?xt=urn:btih:aabb&dn=%E4%B8%AD&tr=one&tr.2=two",
        &json!({}),
    )
    .unwrap();
    assert!(matches!(&o["trackers"],PortValue::StringList(v)if v==&["one","two"]));
    assert!(matches!(&o["hashes"],PortValue::StringList(v)if v==&["aabb"]));
    assert!(matches!(&o["rows"],PortValue::Json(v)if v[1]["value"]=="中"));
    for (id, s) in [
        ("ipv6_expand", "12345::1"),
        ("ipv6_expand", "1g::1"),
        ("ipv4_integer_encode", "1.2.3.256"),
        ("mac_format_encode", "aa:bb:cc:dd:ee:gg"),
    ] {
        assert!(run(id, s, &json!({})).is_err());
    }
    let o = run("user_agent_parse", "未知🙂", &json!({})).unwrap();
    assert!(matches!(&o["data"],PortValue::Json(v)if v["browser"]=="未知"));
}
#[test]
fn numeric_reports_expose_incomplete_work_and_exact_values() {
    let o = run("prime_factorization", "10403", &json!({"maxTrials":2})).unwrap();
    assert!(matches!(&o["complete"], PortValue::Bool(false)));
    assert_eq!(o["remainder"].as_text().unwrap(), "10403");
    assert!(matches!(&o["factors"],PortValue::StringList(v)if v.is_empty()));
    let o = run("collatz_sequence", "27", &json!({"maxSteps":2})).unwrap();
    assert!(matches!(&o["complete"], PortValue::Bool(false)));
    assert!(matches!(&o["sequence"],PortValue::StringList(v)if v==&["27","82","41"]));
    let o = run("radix_all", "1010", &json!({})).unwrap();
    assert!(matches!(&o["values"],PortValue::StringList(v)if v==&["1010","10"]));
    let o = run("radix_all", "0xFF", &json!({})).unwrap();
    assert!(
        matches!(&o["rows"],PortValue::Json(v)if v[0]["hex"]=="FF"&&v[0]["utf8"]=="C3 BF"&&v[0]["base62"]=="47")
    );
    let n = (num_bigint::BigInt::from(1) << 256usize).to_string();
    let o = run("radix_all", &n, &json!({})).unwrap();
    assert!(matches!(&o["rows"],PortValue::Json(v)if v[0]["decimal"]==n&&v[0]["bitWidth"]==257));
}
#[test]
fn analysis_operations_cancel_after_progress() {
    struct Stop(CancellationToken);
    impl ProgressSink for Stop {
        fn emit(&self, e: ProgressEvent) {
            if matches!(e,ProgressEvent::NodeProgress{pct,..}if pct>0.0&&pct<1.0) {
                self.0.cancel();
            }
        }
    }
    for (id, s) in [
        ("prime_factorization", "1000000007".into()),
        (
            "collatz_sequence",
            (num_bigint::BigInt::from(1) << 256usize).to_string(),
        ),
    ] {
        let cancel = CancellationToken::new();
        let r = GraphExecutor::run_node(
            &super::default_registry(),
            id,
            &one("text", PortValue::Text(s)),
            &json!({}),
            &Stop(cancel.clone()),
            &cancel,
        );
        assert!(matches!(r, Err(CoreError::Cancelled)), "{id}: {r:?}");
    }
}
