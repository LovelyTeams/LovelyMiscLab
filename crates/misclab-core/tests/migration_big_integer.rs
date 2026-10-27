use misclab_core::{
    cancel::CancellationToken,
    error::CoreError,
    graph::{executor::GraphExecutor, port::PortValue},
    node::{registry::NodeRegistry, PortMap},
    nodes::default_registry,
    progress::{NullSink, ProgressEvent, ProgressSink},
};
use num_bigint::{BigInt, BigUint};
use num_integer::Integer;
use num_traits::{One, Zero};
use serde_json::{json, Value};

fn run(reg: &NodeRegistry, inputs: PortMap, p: Value) -> Result<PortMap, CoreError> {
    GraphExecutor::run_node(
        reg,
        "big_integer_calc",
        &inputs,
        &p,
        &NullSink,
        &CancellationToken::new(),
    )
}
fn calc(reg: &NodeRegistry, p: Value) -> Result<PortMap, CoreError> {
    run(reg, PortMap::new(), p)
}
fn result(out: &PortMap) -> &Value {
    let PortValue::Json(v) = &out["json"] else {
        panic!()
    };
    &v["result"]
}
fn integer(s: &str) -> BigInt {
    BigInt::parse_bytes(s.as_bytes(), 10).unwrap()
}

#[test]
fn all_seventeen_operations_match_pinned_legal_vectors_and_reject_errors() {
    let f: Value =
        serde_json::from_str(include_str!("fixtures/ebctf-big-integer-batch19.json")).unwrap();
    assert_eq!(f["revision"], "ea2bc36dae31c97060f7f2cddad8df5518afad7b");
    let reg = default_registry();
    let mut operations = std::collections::BTreeSet::new();
    for (i, v) in f["vectors"].as_array().unwrap().iter().enumerate() {
        let out = calc(&reg, v["params"].clone());
        if v.get("error").is_some() {
            assert!(out.is_err(), "vector {i}: {v}");
            continue;
        }
        let out = out.unwrap_or_else(|e| panic!("vector {i}: {e}: {v}"));
        operations.insert(v["params"]["op"].as_str().unwrap());
        let expected = v["text"].as_str().unwrap();
        let expected = if v["params"]["op"] == "isPrime"
            && result(&out)["classification"] == "probable-prime"
        {
            expected.replacen("素数", "可能素数", 1)
        } else {
            expected.to_owned()
        };
        assert_eq!(out["text"].as_text().unwrap(), expected, "vector {i}: {v}");
    }
    assert_eq!(operations.len(), 17);
}

#[test]
fn exact_arithmetic_preserves_bezout_remainders_and_square_root_bounds() {
    let reg = default_registry();
    for a in -17i32..=17 {
        for b in -7i32..=7 {
            let out = calc(
                &reg,
                json!({"op":"extgcd","a":a.to_string(),"b":b.to_string()}),
            )
            .unwrap();
            let j = result(&out);
            let g = integer(j["g"].as_str().unwrap());
            let x = integer(j["x"].as_str().unwrap());
            let y = integer(j["y"].as_str().unwrap());
            assert_eq!(BigInt::from(a) * x + BigInt::from(b) * y, g);
            assert_eq!(g, BigInt::from(a.gcd(&b)));
            if b != 0 {
                for (op, expected) in [("div", a / b), ("mod", a % b)] {
                    let out =
                        calc(&reg, json!({"op":op,"a":a.to_string(),"b":b.to_string()})).unwrap();
                    assert_eq!(out["text"].as_text().unwrap(), expected.to_string());
                }
            }
        }
    }
    for n in [
        BigUint::zero(),
        BigUint::one(),
        (BigUint::one() << 4095usize) - 1u8,
        BigUint::one() << 4095usize,
    ] {
        let out = calc(&reg, json!({"op":"sqrtInt","a":n.to_string()})).unwrap();
        let x = BigUint::parse_bytes(out["text"].as_text().unwrap().as_bytes(), 10).unwrap();
        assert!(&x * &x <= n);
        assert!((&x + 1u8) * (&x + 1u8) > n);
    }
    // A decimal greater than JS's exact Number range stays text throughout.
    assert_eq!(
        calc(&reg, json!({"op":"add","a":"9007199254740993","b":"2"})).unwrap()["text"]
            .as_text()
            .unwrap(),
        "9007199254740995"
    );
    for op in [
        "add",
        "sub",
        "mul",
        "pow",
        "div",
        "mod",
        "modpow",
        "modinv",
        "gcd",
        "extgcd",
        "lcm",
        "isPrime",
        "nextPrime",
        "prevPrime",
        "factor",
        "sqrtInt",
        "bitLen",
    ] {
        assert!(calc(&reg, json!({"op":op})).is_ok(), "default {op}");
    }
}

#[test]
fn factorization_preserves_product_and_distinguishes_incomplete_and_probable_results() {
    let reg = default_registry();
    for (n, budget, complete) in [
        ("1", 0, true),
        ("65536", 0, true),
        ("1000036000099", 0, false),
        ("1000036000099", 3145728, true),
        ("1000073001431003663", 3145728, true),
    ] {
        let out = calc(&reg, json!({"op":"factor","a":n,"factorBudget":budget})).unwrap();
        let j = result(&out);
        let factors = j["factors"].as_array().unwrap();
        let unfinished = j["unfinished"].as_array().unwrap();
        let product = factors
            .iter()
            .map(|p| integer(p["value"].as_str().unwrap()))
            .chain(unfinished.iter().map(|p| integer(p.as_str().unwrap())))
            .product::<BigInt>();
        assert_eq!(product, integer(n));
        assert_eq!(j["provenComplete"], complete);
        assert!(j["rhoSteps"].as_u64().unwrap() <= budget);
        if !complete {
            assert!(out["text"].as_text().unwrap().contains("未分解"));
        }
    }
    let n = ((BigUint::one() << 127usize) - 1u8).to_string();
    for op in ["isPrime", "factor"] {
        let out = calc(&reg, json!({"op":op,"a":n})).unwrap();
        assert!(out["text"].as_text().unwrap().contains("可能素数"));
        if op == "factor" {
            assert_eq!(result(&out)["provenComplete"], false);
        } else {
            assert_eq!(result(&out)["deterministic"], false);
        }
    }
    let out = calc(
        &reg,
        json!({"op":"isPrime","a":"3317044064679887385961981"}),
    )
    .unwrap();
    assert_eq!(result(&out)["classification"], "composite");
}

#[test]
fn operand_connections_override_sidebars_and_results_chain_without_float_conversion() {
    let reg = default_registry();
    let graph = serde_json::from_value(json!({"nodes":[
        {"id":"inv","descriptorId":"big_integer_calc","params":{"op":"modinv","a":"unused","m":"unused"}},
        {"id":"power","descriptorId":"big_integer_calc","params":{"op":"modpow","a":"unused","b":"1","m":"101"}}
    ],"edges":[{"from":{"node":"inv","port":"text"},"to":{"node":"power","port":"a"}}]})).unwrap();
    let input: PortMap = [
        ("a".into(), PortValue::Text("42".into())),
        ("m".into(), PortValue::Text("101".into())),
    ]
    .into();
    let seeds = input
        .iter()
        .map(|(k, v)| (("inv".into(), k.clone()), v.clone()))
        .collect();
    let out = GraphExecutor::new(&reg, &graph)
        .unwrap()
        .with_seed_inputs(seeds)
        .run(&NullSink, &CancellationToken::new())
        .unwrap();
    let direct = run(
        &reg,
        input,
        json!({"op":"modinv","a":"unused","m":"unused"}),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&out["inv"]).unwrap(),
        serde_json::to_value(direct).unwrap()
    );
    assert_eq!(out["power"]["text"].as_text().unwrap(), "89");
    let out = run(
        &reg,
        [("b".into(), PortValue::Text("5".into()))].into(),
        json!({"a":"9","b":"invalid"}),
    )
    .unwrap();
    assert_eq!(out["text"].as_text().unwrap(), "14");
    assert!(run(
        &reg,
        [("a".into(), PortValue::Bytes(vec![49].into()))].into(),
        json!({})
    )
    .is_err());
}

#[test]
fn big_integer_resource_limits_and_invalid_parameter_types_are_explicit() {
    let reg = default_registry();
    for p in [
        json!({"a":"9".repeat(16385)}),
        json!({"a":(BigUint::one()<<4096usize).to_string()}),
        json!({"a":123}),
        json!({"a":null}),
        json!({"op":42}),
        json!({"a":""}),
        json!({"op":"factor","factorBudget":-1}),
        json!({"op":"factor","factorBudget":3145729}),
        json!({"op":"factor","factorBudget":2.5}),
        json!({"op":"factor","factorBudget":"NaN"}),
    ] {
        assert!(calc(&reg, p.clone()).is_err(), "{p}");
    }
    let out = calc(&reg, json!({"op":"pow","a":"0","b":"0"})).unwrap();
    assert_eq!(out["text"].as_text().unwrap(), "1");
    let out = calc(&reg, json!({"op":"modpow","a":"0","b":"0","m":"1"})).unwrap();
    assert_eq!(out["text"].as_text().unwrap(), "0");
    let out = calc(
        &reg,
        json!({"op":"bitLen","a":"-1,024_000\u{feff}","b":"ignored","m":"ignored"}),
    )
    .unwrap();
    assert_eq!(out["text"].as_text().unwrap(), "20");
}

#[test]
fn expensive_number_theory_cancels_during_work_instead_of_publishing_late_results() {
    struct Stop(CancellationToken, f32);
    impl ProgressSink for Stop {
        fn emit(&self, e: ProgressEvent) {
            if matches!(e,ProgressEvent::NodeProgress{pct,..} if pct > self.1 && pct < 1.0) {
                self.0.cancel();
            }
        }
    }
    let reg = default_registry();
    let large = ((BigUint::one() << 521usize) - 1u8).to_string();
    for (p, stage) in [
        (
            json!({"op":"modpow","a":"2","b":large,"m":"9999999967"}),
            0.0,
        ),
        (json!({"op":"pow","a":"2","b":"500000"}), 0.0),
        (json!({"op":"extgcd","a":"6765","b":"4181"}), 0.1),
        (json!({"op":"sqrtInt","a":large}), 0.1),
        (json!({"op":"isPrime","a":large}), 0.0),
        (json!({"op":"nextPrime","a":large}), 0.05),
        (json!({"op":"prevPrime","a":large}), 0.05),
        (
            json!({"op":"factor","a":(((BigUint::one()<<61usize)-1u8)*((BigUint::one()<<127usize)-1u8)).to_string()}),
            0.05,
        ),
    ] {
        let cancel = CancellationToken::new();
        let out = GraphExecutor::run_node(
            &reg,
            "big_integer_calc",
            &PortMap::new(),
            &p,
            &Stop(cancel.clone(), stage),
            &cancel,
        );
        assert!(matches!(out, Err(CoreError::Cancelled)), "{p}: {out:?}");
    }
}
