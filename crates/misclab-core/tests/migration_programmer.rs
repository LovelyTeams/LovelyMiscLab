use misclab_core::{
    cancel::CancellationToken,
    error::CoreError,
    graph::{executor::GraphExecutor, port::PortValue},
    node::{registry::NodeRegistry, PortMap},
    nodes::default_registry,
    progress::{NullSink, ProgressEvent, ProgressSink},
};
use num_bigint::BigUint;
use serde_json::{json, Value};
fn calc(reg: &NodeRegistry, input: &str, p: Value) -> Result<PortMap, CoreError> {
    GraphExecutor::run_node(
        reg,
        "programmer_calc",
        &[("text".into(), PortValue::Text(input.into()))].into(),
        &p,
        &NullSink,
        &CancellationToken::new(),
    )
}
fn detail(out: &PortMap) -> &Value {
    let PortValue::Json(v) = &out["json"] else {
        panic!()
    };
    v
}
#[test]
fn pinned_programmer_vectors_cover_operators_widths_signedness_and_errors() {
    let f: Value =
        serde_json::from_str(include_str!("fixtures/ebctf-programmer-batch20.json")).unwrap();
    assert_eq!(f["revision"], "ea2bc36dae31c97060f7f2cddad8df5518afad7b");
    let reg = default_registry();
    for (i, v) in f["vectors"].as_array().unwrap().iter().enumerate() {
        let out = calc(&reg, v["input"].as_str().unwrap(), v["params"].clone());
        if v.get("error").is_some() {
            assert!(out.is_err(), "vector {i}: {v}");
            continue;
        }
        let out = out.unwrap_or_else(|e| panic!("vector {i}: {e}: {v}"));
        assert_eq!(
            out["signed"].as_text().unwrap(),
            v["signed"].as_str().unwrap(),
            "vector {i}"
        );
        assert_eq!(
            out["unsigned"].as_text().unwrap(),
            v["unsigned"].as_str().unwrap(),
            "vector {i}"
        );
        let mut expected = v["text"].as_str().unwrap().to_owned();
        // The reference labels its first line unsigned but still prints signed.
        if v["params"]["signed"] == false {
            expected = expected.replacen(
                &format!("结果：{}", v["signed"].as_str().unwrap()),
                &format!("结果：{}", v["unsigned"].as_str().unwrap()),
                1,
            );
        }
        assert_eq!(out["text"].as_text().unwrap(), expected, "vector {i}");
    }
}
#[test]
fn arbitrary_width_outputs_preserve_bit_patterns_counts_and_byte_padding() {
    let reg = default_registry();
    for width in 1..=512usize {
        for input in ["0", "~0"] {
            let out = calc(&reg, input, json!({"width":width,"signed":false})).unwrap();
            let j = detail(&out);
            let PortValue::Bytes(bytes) = &out["bytes"] else {
                panic!()
            };
            assert_eq!(bytes.len(), width.div_ceil(8));
            let n = BigUint::from_bytes_be(bytes);
            assert_eq!(n.to_string(), out["value"].as_text().unwrap());
            assert_eq!(j["binary"].as_str().unwrap().len(), width);
            assert_eq!(j["hex"].as_str().unwrap().len(), 2 + width.div_ceil(4));
            if input == "0" {
                assert_eq!(j["popcount"], 0);
                assert_eq!(j["leadingZeros"], width);
                assert_eq!(j["trailingZeros"], width);
            } else {
                assert_eq!(j["popcount"], width);
                assert_eq!(j["leadingZeros"], 0);
                assert_eq!(j["trailingZeros"], 0);
                assert_eq!(n, (BigUint::from(1u8) << width) - 1u8);
                assert_eq!(out["signed"].as_text().unwrap(), "-1");
            }
        }
    }
}
#[test]
fn wrapping_u64_reference_checks_rotation_arithmetic_and_fixed_shift_semantics() {
    let reg = default_registry();
    let mut seed = 0xF5AD4ECEDA1CE2A9u64;
    for i in 0..64 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let n = i % 64;
        for (expression, expected) in [
            (format!("rotl({seed},{n})"), seed.rotate_left(n)),
            (format!("rotr({seed},{n})"), seed.rotate_right(n)),
            (
                format!("{seed} * 3 + 17"),
                seed.wrapping_mul(3).wrapping_add(17),
            ),
            (format!("{seed} >> {n}"), ((seed as i64) >> n) as u64),
            (format!("{seed} >>> {n}"), seed >> n),
        ] {
            let out = calc(&reg, &expression, json!({"width":64,"signed":false})).unwrap();
            assert_eq!(
                out["value"].as_text().unwrap(),
                expected.to_string(),
                "{expression}"
            );
        }
    }
}
#[test]
fn numeric_text_and_binary_ports_chain_and_match_standalone() {
    let reg = default_registry();
    let graph=serde_json::from_value(json!({"nodes":[
        {"id":"p","descriptorId":"programmer_calc","params":{"width":64,"signed":false}},
        {"id":"sum","descriptorId":"big_integer_calc","params":{"op":"add","b":"1"}},
        {"id":"stats","descriptorId":"hex_stats","params":{}}
    ],"edges":[{"from":{"node":"p","port":"value"},"to":{"node":"sum","port":"a"}},{"from":{"node":"p","port":"bytes"},"to":{"node":"stats","port":"data"}}]})).unwrap();
    let input = "1 << 63 | 1";
    let out = GraphExecutor::new(&reg, &graph)
        .unwrap()
        .with_seed_inputs([(("p".into(), "text".into()), PortValue::Text(input.into()))].into())
        .run(&NullSink, &CancellationToken::new())
        .unwrap();
    let direct = calc(&reg, input, json!({"width":64,"signed":false})).unwrap();
    assert_eq!(
        serde_json::to_value(&out["p"]).unwrap(),
        serde_json::to_value(direct).unwrap()
    );
    assert_eq!(out["sum"]["text"].as_text().unwrap(), "9223372036854775810");
    let PortValue::Bytes(bytes) = &out["p"]["bytes"] else {
        panic!()
    };
    assert_eq!(bytes.as_ref(), [0x80, 0, 0, 0, 0, 0, 0, 1]);
    assert!(out.contains_key("stats"));
}
#[test]
fn expression_resources_parameter_types_and_non_code_grammar_are_enforced() {
    let reg = default_registry();
    for input in [
        "+".repeat(130) + "1",
        "rotl(".repeat(33) + "1" + &",0)".repeat(33),
        "1".repeat(1025),
        " ".repeat(16384) + "1",
        "1;globalThis.secret".into(),
        "__import__('os')".into(),
    ] {
        assert!(calc(&reg, &input, json!({})).is_err(), "{input}");
    }
    for p in [
        json!({"width":0}),
        json!({"width":null}),
        json!({"width":true}),
        json!({"width":"NaN"}),
        json!({"signed":"false"}),
        json!({"signed":0}),
    ] {
        assert!(calc(&reg, "1", p.clone()).is_err(), "{p}");
    }
    assert!(calc(&reg, &("(".repeat(32) + "1" + &")".repeat(32)), json!({})).is_ok());
    assert!(calc(
        &reg,
        &std::iter::repeat_n("1", 512).collect::<Vec<_>>().join("+"),
        json!({})
    )
    .is_ok());
}
#[test]
fn parsing_and_modular_power_observe_cancellation() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Stop(CancellationToken);
    impl ProgressSink for Stop {
        fn emit(&self, e: ProgressEvent) {
            if matches!(e,ProgressEvent::NodeProgress{pct,..} if pct>0.1&&pct<1.0) {
                self.0.cancel();
            }
        }
    }
    let reg = default_registry();
    for input in [
        std::iter::repeat_n("1", 512).collect::<Vec<_>>().join("+"),
        format!("3 ** {}", (BigUint::from(1u8) << 511usize) - 1u8),
    ] {
        let cancel = CancellationToken::new();
        let out = GraphExecutor::run_node(
            &reg,
            "programmer_calc",
            &[("text".into(), PortValue::Text(input))].into(),
            &json!({"width":512}),
            &Stop(cancel.clone()),
            &cancel,
        );
        assert!(matches!(out, Err(CoreError::Cancelled)));
    }
    struct StopPower(CancellationToken, AtomicUsize);
    impl ProgressSink for StopPower {
        fn emit(&self, event: ProgressEvent) {
            // Last parsed token emits 0.9; power checkpoints emit it again.
            // Cancel after the first 32 exponent bits have been processed.
            if matches!(event, ProgressEvent::NodeProgress { pct, .. } if (0.89..1.0).contains(&pct))
                && self.1.fetch_add(1, Ordering::Relaxed) == 2
            {
                self.0.cancel();
            }
        }
    }
    let cancel = CancellationToken::new();
    let input = format!("3 ** {}", (BigUint::from(1u8) << 511usize) - 1u8);
    let out = GraphExecutor::run_node(
        &reg,
        "programmer_calc",
        &[("text".into(), PortValue::Text(input))].into(),
        &json!({"width":512}),
        &StopPower(cancel.clone(), AtomicUsize::new(0)),
        &cancel,
    );
    assert!(matches!(out, Err(CoreError::Cancelled)));
}
