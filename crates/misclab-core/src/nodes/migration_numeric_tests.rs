use super::prelude::*;
use crate::{
    cancel::CancellationToken,
    graph::executor::GraphExecutor,
    progress::{NullSink, ProgressEvent, ProgressSink},
};
use serde_json::{json, Value};
fn run(id: &str, input: &PortMap, p: &Value) -> Result<PortMap, CoreError> {
    GraphExecutor::run_node(
        &super::default_registry(),
        id,
        input,
        p,
        &NullSink,
        &CancellationToken::new(),
    )
}
fn text(id: &str, input: &str, p: &Value) -> Result<PortMap, CoreError> {
    run(id, &one("text", PortValue::Text(input.into())), p)
}
#[test]
fn pinned_numeric_vectors_cover_twenty_operations() {
    let f: Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/ebctf-numeric-batch5.json"
    ))
    .unwrap();
    assert_eq!(f["revision"], "ea2bc36dae31c97060f7f2cddad8df5518afad7b");
    let vs = f["vectors"].as_array().unwrap();
    assert_eq!(vs.len(), 405);
    assert_eq!(
        vs.iter()
            .map(|v| v["operation"].as_str().unwrap())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        20
    );
    for (i, v) in vs.iter().enumerate() {
        let id = v["node"].as_str().unwrap();
        let r = run(
            id,
            &serde_json::from_value(v["input"].clone()).unwrap(),
            &v["params"],
        );
        if v.get("error").is_some() {
            assert!(r.is_err(), "vector {i} {id}: expected error");
            continue;
        }
        let o = r.unwrap_or_else(|e| panic!("vector {i} {id}: {e}"));
        assert_eq!(
            o["text"].as_text().unwrap(),
            v["text"].as_str().unwrap(),
            "vector {i} {id}"
        );
        if let Some(b) = v.get("bytes") {
            let b: Vec<u8> = serde_json::from_value(b.clone()).unwrap();
            assert_eq!(&*o["bytes"].as_bytes().unwrap(), b, "vector {i} {id}");
        }
    }
}
#[test]
fn byte_radix_roundtrips_preserve_binary_and_empty_inputs() {
    let data: Vec<u8> = (0..=255).collect();
    for base in [2, 8, 10, 16, 36] {
        for bits in [7, 8] {
            for flags in [(false, false), (true, false), (false, true), (true, true)] {
                let p = json!({"base":base,"bits":bits,"invert":flags.0,"bitReverse":flags.1});
                for len in [0, 1, 3, 127, 128, 256] {
                    let input = &data[..len];
                    let e = run(
                        "ascii_radix_encode",
                        &one("data", PortValue::Bytes(input.to_vec().into())),
                        &p,
                    );
                    if base == 2 && bits == 7 && len > 128 {
                        assert!(e.is_err());
                        continue;
                    }
                    let e = e.unwrap();
                    let d = text("ascii_radix_decode", e["text"].as_text().unwrap(), &p).unwrap();
                    assert_eq!(&*d["bytes"].as_bytes().unwrap(), input);
                }
            }
        }
    }
    let o = text("ascii_radix_decode", "EF BB BF FF 00", &json!({"base":16})).unwrap();
    assert_eq!(&*o["bytes"].as_bytes().unwrap(), [239, 187, 191, 255, 0]);
    assert_eq!(o["text"].as_text().unwrap(), "\u{feff}\u{fffd}\0");
}
#[test]
fn exact_numerals_roundtrip_beyond_float_and_upstream_limits() {
    for id in [
        "negative_base",
        "balanced_ternary",
        "factorial_base",
        "zeckendorf",
        "chinese_number",
    ] {
        for input in [
            "0",
            "1",
            "13",
            "463",
            "9007199254740993",
            "231584178474632390847141970017375815706539969331281128078915168015826259279872",
        ] {
            let e = text(&format!("{id}_encode"), input, &json!({})).unwrap();
            let d = text(
                &format!("{id}_decode"),
                e["text"].as_text().unwrap(),
                &json!({}),
            )
            .unwrap();
            assert_eq!(d["text"].as_text().unwrap(), input, "{id}");
        }
    }
    for id in ["negative_base", "balanced_ternary", "chinese_number"] {
        for input in ["-1", "-13", "-9007199254740993"] {
            let e = text(&format!("{id}_encode"), input, &json!({})).unwrap();
            let d = text(
                &format!("{id}_decode"),
                e["text"].as_text().unwrap(),
                &json!({}),
            )
            .unwrap();
            assert_eq!(d["text"].as_text().unwrap(), input);
        }
    }
    for input in ["1/1", "355/113", "5/3", "987/610"] {
        for id in ["continued_fraction", "stern_brocot"] {
            let e = text(&format!("{id}_encode"), input, &json!({})).unwrap();
            let d = text(
                &format!("{id}_decode"),
                e["text"].as_text().unwrap(),
                &json!({}),
            )
            .unwrap();
            assert_eq!(d["text"].as_text().unwrap(), input);
        }
    }
}
#[test]
fn numeric_boundaries_are_explicit_and_structured_outputs_connect() {
    for (n, bits) in [(-128, 8), (-32768, 16), (-2147483648_i64, 32)] {
        let o = text("binary_complement", &n.to_string(), &json!({})).unwrap();
        assert_eq!(
            o["complement"].as_text().unwrap(),
            format!("1{}", "0".repeat(bits - 1))
        );
        assert!(o["original"].as_text().unwrap().starts_with("无法表示"));
        let d = text(
            "complement_to_decimal",
            o["complement"].as_text().unwrap(),
            &json!({}),
        )
        .unwrap();
        assert_eq!(d["complement"].as_text().unwrap(), n.to_string());
    }
    let o = text("gcd_lcm", "0 0", &json!({})).unwrap();
    assert_eq!(o["gcd"].as_text().unwrap(), "0");
    assert_eq!(o["lcm"].as_text().unwrap(), "0");
    let o = text("complement_to_decimal", "10000000", &json!({})).unwrap();
    assert_eq!(o["original"].as_text().unwrap(), "-0");
    let o = text("ascii_offset_encode", "🙂", &json!({"offset":3})).unwrap();
    assert_eq!(&*o["bytes"].as_bytes().unwrap(), [0x40, 0xd8]);
    assert_eq!(o["text"].as_text().unwrap(), "\u{fffd}");
    let o = text("ascii_offset_decode", "Khoor", &json!({"offset":0})).unwrap();
    assert!(
        matches!(&o["results"],PortValue::StringList(v)if v.len()==52&&v.iter().any(|s|s=="Hello"))
    );
    let o = text("separate_ascii", "72101108108111", &json!({})).unwrap();
    assert!(matches!(&o["results"],PortValue::StringList(v)if v.iter().any(|s|s=="Hello")));
}
#[test]
fn malformed_numeric_data_and_excessive_results_are_rejected() {
    for (id, s, p) in [
        ("radix_transform_encode", "1_0", json!({"fromBase":10})),
        ("negative_base_encode", "1", json!({"base":-1})),
        ("factorial_base_decode", "3:4:2", json!({})),
        ("roman_decode", "IIII", json!({})),
        ("binary_complement", "2147483648", json!({})),
        ("complement_to_decimal", "11111111", json!({"bits":7})),
        ("gcd_lcm", "1.5 3", json!({})),
        ("hybrid_code_decode", "x110000", json!({})),
        ("hybrid_code_decode", "xD800", json!({})),
        ("stern_brocot_encode", "100002/1", json!({})),
        ("continued_fraction_decode", "[0, 0]", json!({})),
        ("ascii_radix_decode", "1FF 00", json!({"base":16})),
        ("balanced_ternary_encode", "", json!({})),
        ("binary_pad_encode", "256", json!({"bits":8})),
    ] {
        assert!(text(id, s, &p).is_err(), "{id} {s}");
    }
}
#[test]
fn numeric_enumeration_and_path_generation_cancel_mid_execution() {
    struct Stop(CancellationToken);
    impl ProgressSink for Stop {
        fn emit(&self, e: ProgressEvent) {
            if matches!(e,ProgressEvent::NodeProgress{pct,..}if pct>0.0&&pct<1.0) {
                self.0.cancel();
            }
        }
    }
    for (id, input) in [
        ("ascii_offset_encode", "Hello"),
        ("stern_brocot_encode", "987/610"),
    ] {
        let cancel = CancellationToken::new();
        let r = GraphExecutor::run_node(
            &super::default_registry(),
            id,
            &one("text", PortValue::Text(input.into())),
            &json!({}),
            &Stop(cancel.clone()),
            &cancel,
        );
        assert!(matches!(r, Err(CoreError::Cancelled)), "{id}: {r:?}");
    }
}
