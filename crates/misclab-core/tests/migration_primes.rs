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
fn run(reg: &NodeRegistry, id: &str, inputs: PortMap, p: Value) -> Result<PortMap, CoreError> {
    GraphExecutor::run_node(reg, id, &inputs, &p, &NullSink, &CancellationToken::new())
}
fn test(reg: &NodeRegistry, n: &str, rounds: u32) -> Result<PortMap, CoreError> {
    run(
        reg,
        "prime_test",
        PortMap::new(),
        json!({"n":n,"rounds":rounds}),
    )
}
fn details(out: &PortMap) -> &Value {
    let PortValue::Json(v) = &out["json"] else {
        panic!()
    };
    v
}
#[test]
fn pinned_prime_vectors_preserve_verdicts_bit_lengths_and_round_counts() {
    let f: Value = serde_json::from_str(include_str!("fixtures/ebctf-prime-batch17.json")).unwrap();
    assert_eq!(f["revision"], "ea2bc36dae31c97060f7f2cddad8df5518afad7b");
    assert_eq!(f["vectors"].as_array().unwrap().len(), 207);
    let reg = default_registry();
    for (i, v) in f["vectors"].as_array().unwrap().iter().enumerate() {
        let id = if v["operation"] == "primeGen" {
            "prime_generate"
        } else {
            "prime_test"
        };
        let out = run(&reg, id, PortMap::new(), v["params"].clone());
        if v.get("error").is_some() {
            assert!(out.is_err(), "{i}");
            continue;
        }
        let out = out.unwrap_or_else(|e| panic!("{i}: {e}"));
        let original = v["text"].as_str().unwrap();
        if id == "prime_generate" {
            assert_eq!(out["text"].as_text().unwrap(), original);
            continue;
        }
        let j = details(&out);
        assert_eq!(j["passed"], original.starts_with("判定：素数"), "{i}");
        assert_eq!(
            out["text"].as_text().unwrap().lines().nth(1),
            original.lines().nth(1),
            "{i}"
        );
        if j["classification"] == "probable-prime" {
            assert!(out["text"].as_text().unwrap().starts_with("判定：可能素数"));
            assert_eq!(j["testedRounds"], v["params"]["rounds"]);
        }
    }
    for v in f["generated"].as_array().unwrap() {
        let out = test(&reg, v["n"].as_str().unwrap(), 24).unwrap();
        assert_eq!(details(&out)["passed"], true);
        assert_eq!(details(&out)["bits"], v["bits"]);
    }
}
#[test]
fn small_integers_match_independent_trial_division_and_pseudoprimes_are_not_proofs() {
    let reg = default_registry();
    for n in 0..1000u32 {
        let expected = n >= 2 && !(2..).take_while(|d| d * d <= n).any(|d| n % d == 0);
        let out = test(&reg, &n.to_string(), 1).unwrap();
        assert_eq!(details(&out)["passed"], expected, "{n}");
        assert_eq!(details(&out)["deterministic"], true);
    }
    // First strong pseudoprime to all 13 fixed prime bases: it must never be
    // labelled a proven prime at the bound. An additional base detects it.
    let n = "3317044064679887385961981";
    let a = BigUint::parse_bytes(b"1287836182261", 10).unwrap();
    let b = BigUint::parse_bytes(b"2575672364521", 10).unwrap();
    assert_eq!((a * b).to_string(), n);
    let weak = test(&reg, n, 13).unwrap();
    assert_eq!(details(&weak)["classification"], "probable-prime");
    assert_eq!(details(&weak)["deterministic"], false);
    let strong = test(&reg, n, 14).unwrap();
    assert_eq!(details(&strong)["classification"], "composite");
    assert!(weak["text"].as_text().unwrap().contains("不适用随机基"));
}
#[test]
fn generated_candidates_have_exact_bits_and_interoperate_with_reference() {
    let reg = default_registry();
    let mut exported = Vec::new();
    for bits in [2u32, 3, 8, 16, 32, 64, 128, 256, 512, 1024] {
        let count = if bits <= 16 { 3 } else { 1 };
        let out = run(
            &reg,
            "prime_generate",
            PortMap::new(),
            json!({"bits":bits,"count":count}),
        )
        .unwrap();
        let PortValue::StringList(primes) = &out["primes"] else {
            panic!()
        };
        assert_eq!(primes.len(), count);
        assert_eq!(out["text"].as_text().unwrap(), primes.join("\n"));
        for s in primes {
            let n = BigUint::parse_bytes(s.as_bytes(), 10).unwrap();
            assert_eq!(n.bits(), u64::from(bits));
            assert!(n.bit(0));
            assert_eq!(details(&test(&reg, s, 24).unwrap())["passed"], true);
            if bits <= 16 {
                let n = s.parse::<u32>().unwrap();
                assert!(!(2..).take_while(|d| d * d <= n).any(|d| n % d == 0));
            }
        }
        exported.push(json!({"bits":bits,"primes":primes}));
    }
    if let Ok(path) = std::env::var("MISCLAB_PRIME_REFERENCE_EXPORT") {
        std::fs::write(path, serde_json::to_string_pretty(&exported).unwrap()).unwrap();
    }
}
#[test]
fn generated_numbers_feed_the_typed_primality_node_and_override_parameters() {
    let reg = default_registry();
    let graph = serde_json::from_value(json!({"nodes":[
        {"id":"g","descriptorId":"prime_generate","params":{"bits":16,"count":1}},
        {"id":"t","descriptorId":"prime_test","params":{"n":"4"}}
    ],"edges":[{"from":{"node":"g","port":"text"},"to":{"node":"t","port":"text"}}]}))
    .unwrap();
    let out = GraphExecutor::new(&reg, &graph)
        .unwrap()
        .run(&NullSink, &CancellationToken::new())
        .unwrap();
    assert_eq!(details(&out["t"])["classification"], "prime");
    assert_eq!(details(&out["t"])["bits"], 16);
    let direct = test(&reg, out["g"]["text"].as_text().unwrap(), 24).unwrap();
    assert_eq!(
        serde_json::to_value(&out["t"]).unwrap(),
        serde_json::to_value(direct).unwrap()
    );
}
#[test]
fn prime_tools_reject_unbounded_and_invalid_requests() {
    let reg = default_registry();
    for n in [
        "".into(),
        "中文".into(),
        "0x17".into(),
        "2e9".into(),
        "--2".into(),
        "9".repeat(16385),
        (BigUint::from(1u8) << 4096usize).to_string(),
    ] {
        assert!(test(&reg, &n, 24).is_err());
    }
    for p in [
        json!({"bits":1}),
        json!({"bits":1025}),
        json!({"bits":3.5}),
        json!({"count":0}),
        json!({"count":101}),
        json!({"count":"NaN"}),
    ] {
        assert!(run(&reg, "prime_generate", PortMap::new(), p).is_err());
    }
    for rounds in [0, 65] {
        assert!(test(&reg, "17", rounds).is_err());
    }
}
#[test]
fn prime_tools_cancel_inside_modular_exponentiation_and_generation() {
    struct Stop(CancellationToken);
    impl ProgressSink for Stop {
        fn emit(&self, e: ProgressEvent) {
            if matches!(e,ProgressEvent::NodeProgress{pct,..} if pct>0.0&&pct<1.0) {
                self.0.cancel();
            }
        }
    }
    let reg = default_registry();
    let n = ((BigUint::from(1u8) << 521usize) - 1u8).to_string();
    for (id, p) in [
        ("prime_test", json!({"n":n,"rounds":64})),
        ("prime_generate", json!({"bits":1024,"count":100})),
    ] {
        let cancel = CancellationToken::new();
        let out = GraphExecutor::run_node(
            &reg,
            id,
            &PortMap::new(),
            &p,
            &Stop(cancel.clone()),
            &cancel,
        );
        assert!(matches!(out, Err(CoreError::Cancelled)), "{id}");
    }
}
