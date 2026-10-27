use misclab_core::{
    cancel::CancellationToken,
    error::CoreError,
    graph::{executor::GraphExecutor, port::PortValue},
    node::{registry::NodeRegistry, PortMap},
    nodes::default_registry,
    progress::{NullSink, ProgressEvent, ProgressSink},
};
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::One;
use serde_json::{json, Value};
fn calc(reg: &NodeRegistry, input: &str, p: Value) -> Result<PortMap, CoreError> {
    GraphExecutor::run_node(
        reg,
        "unit_convert",
        &[("text".into(), PortValue::Text(input.into()))].into(),
        &p,
        &NullSink,
        &CancellationToken::new(),
    )
}
fn details(out: &PortMap) -> &Value {
    let PortValue::Json(v) = &out["json"] else {
        panic!()
    };
    v
}
fn integer(s: &str) -> BigInt {
    BigInt::parse_bytes(s.as_bytes(), 10).unwrap()
}
fn row<'a>(j: &'a Value, u: &str) -> &'a Value {
    j["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["unit"] == u)
        .unwrap()
}

#[test]
fn pinned_unit_vectors_cover_all_units_aliases_epochs_and_display_precision() {
    let f: Value = serde_json::from_str(include_str!("fixtures/ebctf-unit-batch21.json")).unwrap();
    assert_eq!(f["revision"], "ea2bc36dae31c97060f7f2cddad8df5518afad7b");
    assert_eq!(f["vectors"].as_array().unwrap().len(), 625);
    let reg = default_registry();
    for (i, v) in f["vectors"].as_array().unwrap().iter().enumerate() {
        let out = calc(&reg, v["input"].as_str().unwrap(), v["params"].clone());
        if v.get("error").is_some() {
            assert!(out.is_err(), "vector {i}: {v}");
            continue;
        }
        let out = out.unwrap_or_else(|e| panic!("vector {i}: {e}: {v}"));
        let j = details(&out);
        if let Some(ts) = v.get("timestamp") {
            assert_eq!(j["timestamp"]["epoch"], ts["epoch"], "vector {i}");
            assert_eq!(
                j["timestamp"]["unixSeconds"]["numerator"], ts["seconds"]["n"],
                "vector {i}"
            );
            assert_eq!(
                j["timestamp"]["unixSeconds"]["denominator"], ts["seconds"]["d"],
                "vector {i}"
            );
            if v["dos"] == "0x00000000" {
                // JS bitwise NaN coercion fabricates a zero DOS date after
                // Date overflows. Zero is never a valid FAT month/day.
                assert!(j["timestamp"]["dosUtc"].is_null(), "vector {i}");
            } else {
                assert_eq!(j["timestamp"]["dosUtc"], v["dos"], "vector {i}");
            }
            if ts["seconds"]["d"] == "1"
                && v["iso"].as_str().is_some_and(|s| s.ends_with('Z'))
                && !j["timestamp"]["isoUtc"].is_null()
            {
                assert_eq!(j["timestamp"]["isoUtc"], v["iso"], "vector {i}");
            }
        } else {
            let actual = j["rows"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| json!({"unit":r["unit"],"value":r["value"]}))
                .collect::<Vec<_>>();
            assert_eq!(Value::Array(actual), v["rows"], "vector {i}: {v}");
        }
    }
}
#[test]
fn rational_results_preserve_giant_integers_and_recover_every_declared_coefficient() {
    let tables: Value =
        serde_json::from_str(include_str!("../src/nodes/unit_definitions.json")).unwrap();
    let reg = default_registry();
    let qty = integer("900719925474099312345678901234567890");
    for group in tables["groups"]
        .as_array()
        .unwrap()
        .iter()
        .chain(tables["currencies"].as_array().unwrap())
    {
        for source in group["units"].as_array().unwrap() {
            let out = calc(
                &reg,
                &format!("{qty} {}", source["u"].as_str().unwrap()),
                json!({}),
            )
            .unwrap();
            let rows = details(&out)["rows"].as_array().unwrap();
            let source_n = integer(source["k"]["n"].as_str().unwrap());
            let source_d = integer(source["k"]["d"].as_str().unwrap());
            for target in group["units"].as_array().unwrap() {
                let r = rows
                    .iter()
                    .find(|r| r["group"] == group["id"] && r["unit"] == target["u"])
                    .unwrap();
                let n = integer(r["numerator"].as_str().unwrap());
                let d = integer(r["denominator"].as_str().unwrap());
                let target_n = integer(target["k"]["n"].as_str().unwrap());
                let target_d = integer(target["k"]["d"].as_str().unwrap());
                assert_eq!(&n * &source_d * target_n, &qty * &source_n * target_d * &d);
                assert_eq!(n.gcd(&d), BigInt::one());
                assert!(d > BigInt::from(0));
                if source["u"] == target["u"] {
                    assert_eq!(n, qty);
                    assert_eq!(d, BigInt::one());
                }
            }
        }
    }
    let out = calc(&reg, "1 尺", json!({})).unwrap();
    let r = row(details(&out), "m");
    assert_eq!(r["numerator"], "1");
    assert_eq!(r["denominator"], "3");
    assert_eq!(r["displayExact"], false);
    let out = calc(&reg, "1 GiB", json!({})).unwrap();
    assert_eq!(row(details(&out), "B")["value"], "1073741824");
    let out = calc(&reg, "100 Mbps", json!({})).unwrap();
    assert_eq!(row(details(&out), "MB/s")["value"], "12.5");
}
#[test]
fn fractional_epochs_retain_subseconds_and_currency_ambiguity_stays_in_separate_groups() {
    let reg = default_registry();
    for (input, epoch, sec, ms, iso) in [
        ("1.5", "unix_s", "1.5", "1500", "1970-01-01T00:00:01.500Z"),
        ("-0.5", "unix_s", "-0.5", "-500", "1969-12-31T23:59:59.500Z"),
        (
            "116444736001234567",
            "filetime",
            "0.1234567",
            "123.4567",
            "1970-01-01T00:00:00.123Z",
        ),
        (
            "116444735999999999",
            "filetime",
            "-0.0000001",
            "-0.0001",
            "1970-01-01T00:00:00.000Z",
        ),
        (
            "0.25",
            "cocoa",
            "978307200.25",
            "978307200250",
            "2001-01-01T00:00:00.250Z",
        ),
    ] {
        let out = calc(&reg, input, json!({"tsEpoch":epoch})).unwrap();
        let j = details(&out);
        let s = row(j, "unix_s");
        // Compare exact fractions: very small displays deliberately use scientific notation.
        let (whole, frac) = sec
            .trim_start_matches('-')
            .split_once('.')
            .unwrap_or((sec, ""));
        let n = integer(&format!("{whole}{frac}")) * if sec.starts_with('-') { -1 } else { 1 };
        let d = BigInt::from(10).pow(frac.len() as u32);
        assert_eq!(
            integer(s["numerator"].as_str().unwrap()) * d,
            n * integer(s["denominator"].as_str().unwrap())
        );
        assert_eq!(row(j, "unix_ms")["value"], ms);
        assert_eq!(j["timestamp"]["isoUtc"], iso);
    }
    let out = calc(&reg, "1 cent", json!({})).unwrap();
    let rows = details(&out)["rows"].as_array().unwrap();
    assert!(rows
        .iter()
        .any(|r| r["group"] == "cur_usd" && r["unit"] == "dollar" && r["value"] == "0.01"));
    assert!(rows
        .iter()
        .any(|r| r["group"] == "cur_eur" && r["unit"] == "euro" && r["value"] == "0.01"));
    assert!(rows
        .iter()
        .all(|r| r["group"] == "cur_usd" || r["group"] == "cur_eur"));
    let out = calc(&reg, "1 元", json!({})).unwrap();
    assert!(details(&out)["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["group"] == "cur_hkd"));
}
#[test]
fn unit_tables_chain_as_json_and_graph_matches_standalone() {
    let reg = default_registry();
    let graph = serde_json::from_value(json!({"nodes":[
        {"id":"input","descriptorId":"text_input","params":{"text":"1 GB"}},
        {"id":"convert","descriptorId":"unit_convert","params":{}},
        {"id":"next","descriptorId":"coalesce","params":{}}
    ],"edges":[{"from":{"node":"input","port":"text"},"to":{"node":"convert","port":"text"}},{"from":{"node":"convert","port":"rows"},"to":{"node":"next","port":"a"}}]}))
    .unwrap();
    let out = GraphExecutor::new(&reg, &graph)
        .unwrap()
        .run(&NullSink, &CancellationToken::new())
        .unwrap();
    let direct = calc(&reg, "1 GB", json!({})).unwrap();
    assert_eq!(
        serde_json::to_value(&out["convert"]).unwrap(),
        serde_json::to_value(direct).unwrap()
    );
    let PortValue::Json(rows) = &out["convert"]["rows"] else {
        panic!()
    };
    assert_eq!(rows.as_array().unwrap().len(), 11);
    assert_eq!(
        serde_json::to_value(&out["next"]["output"]).unwrap(),
        serde_json::to_value(&out["convert"]["rows"]).unwrap()
    );
}
#[test]
fn invalid_inputs_and_extreme_values_never_emit_invalid_calendar_or_float_results() {
    let reg = default_registry();
    for (input, p) in [
        ("1".repeat(4097), json!({})),
        (" ".repeat(16384) + "1", json!({})),
        ("1".into(), json!({"tsEpoch":null})),
        ("1".into(), json!({"tsEpoch":42})),
        ("1".repeat(400) + " deg", json!({})),
    ] {
        assert!(calc(&reg, &input, p).is_err());
    }
    let huge = "9".repeat(1000);
    let out = calc(&reg, &huge, json!({"tsEpoch":"unix_s"})).unwrap();
    assert!(details(&out)["timestamp"]["isoUtc"].is_null());
    assert!(details(&out)["timestamp"]["dosUtc"].is_null());
    assert_eq!(row(details(&out), "unix_s")["value"], huge);
    assert!(!out["text"].as_text().unwrap().contains("NaN"));
    let out = calc(&reg, "1 GB", json!({"tsEpoch":"ignored-for-units"})).unwrap();
    assert_eq!(details(&out)["category"], "dataSize");
}
#[test]
fn multirow_conversion_observes_cancellation() {
    struct Stop(CancellationToken);
    impl ProgressSink for Stop {
        fn emit(&self, e: ProgressEvent) {
            if matches!(e,ProgressEvent::NodeProgress{pct,..} if pct>0.1&&pct<1.0) {
                self.0.cancel();
            }
        }
    }
    let reg = default_registry();
    let cancel = CancellationToken::new();
    let out = GraphExecutor::run_node(
        &reg,
        "unit_convert",
        &[("text".into(), PortValue::Text("1 GB".into()))].into(),
        &json!({}),
        &Stop(cancel.clone()),
        &cancel,
    );
    assert!(matches!(out, Err(CoreError::Cancelled)));
}
