use misclab_core::{
    cancel::CancellationToken,
    error::CoreError,
    graph::{executor::GraphExecutor, port::PortValue},
    node::{registry::NodeRegistry, PortMap},
    nodes::default_registry,
    progress::{NullSink, ProgressEvent, ProgressSink},
};
use serde_json::{json, Value};
fn run(
    reg: &NodeRegistry,
    a: PortValue,
    b: Option<PortValue>,
    p: Value,
) -> Result<PortMap, CoreError> {
    let mut inputs: PortMap = [("a".into(), a)].into();
    if let Some(b) = b {
        inputs.insert("b".into(), b);
    }
    GraphExecutor::run_node(
        reg,
        "diff_tool",
        &inputs,
        &p,
        &NullSink,
        &CancellationToken::new(),
    )
}
fn data(out: &PortMap) -> &Value {
    let PortValue::Json(v) = &out["json"] else {
        panic!()
    };
    v
}
#[test]
fn pinned_diff_vectors_cover_modes_encodings_context_and_all_strategies() {
    let f: Value = serde_json::from_str(include_str!("fixtures/ebctf-diff-batch18.json")).unwrap();
    assert_eq!(f["revision"], "ea2bc36dae31c97060f7f2cddad8df5518afad7b");
    assert_eq!(f["vectors"].as_array().unwrap().len(), 150);
    let reg = default_registry();
    for (i, v) in f["vectors"].as_array().unwrap().iter().enumerate() {
        let out = run(
            &reg,
            PortValue::Text(v["input"].as_str().unwrap().into()),
            None,
            v["params"].clone(),
        );
        if v.get("error").is_some() {
            assert!(out.is_err(), "{i}");
            continue;
        }
        let out = out.unwrap_or_else(|e| panic!("{i}: {e}"));
        assert_eq!(
            out["text"].as_text().unwrap(),
            v["text"].as_str().unwrap(),
            "{i}"
        );
    }
}
#[test]
fn raw_binary_and_line_results_preserve_inputs_and_usable_ranges() {
    let reg = default_registry();
    let all = (0..=255).collect::<Vec<u8>>();
    let mut changed = all.clone();
    changed[128] = 0;
    let out = run(
        &reg,
        PortValue::Bytes(all.clone().into()),
        Some(PortValue::Bytes(changed.clone().into())),
        json!({"textB":"ignored","inputEnc":"hex"}),
    )
    .unwrap();
    assert_eq!(out["a_bytes"].as_bytes().unwrap().as_ref(), all);
    assert_eq!(out["b_bytes"].as_bytes().unwrap().as_ref(), changed);
    let d = data(&out);
    assert_eq!(d["equal"], 255);
    assert_eq!(d["deleted"], 1);
    assert_eq!(d["inserted"], 1);
    assert_eq!(d["hunks"][0]["a"], json!([128, 129]));
    let out = run(
        &reg,
        PortValue::Text("中文🙂\r\n第二行\r".into()),
        Some(PortValue::Text("中文🙂\n第二行\r".into())),
        json!({"mode":"line"}),
    )
    .unwrap();
    assert_eq!(data(&out)["equal"], 2);
    let out = run(
        &reg,
        PortValue::Text("41".into()),
        None,
        json!({"textB":"42"}),
    )
    .unwrap();
    assert_eq!(out["a_bytes"].as_bytes().unwrap().as_ref(), [65]);
    assert_eq!(out["b_bytes"].as_bytes().unwrap().as_ref(), [66]);
    // A determines auto encoding for B, even if B alone looks like UTF-8.
    assert!(run(
        &reg,
        PortValue::Text("41".into()),
        None,
        json!({"textB":"xyz"})
    )
    .is_err());
}
#[test]
fn large_reports_are_bounded_and_fallback_is_explicit() {
    let reg = default_registry();
    let a = "ab".repeat(20000);
    let b = "ac".repeat(20000);
    let out = run(
        &reg,
        PortValue::Text(a.clone()),
        None,
        json!({"textB":b,"inputEnc":"utf8"}),
    )
    .unwrap();
    let d = data(&out);
    assert_eq!(d["hunkCount"], 20000);
    assert_eq!(d["reportTruncated"], true);
    assert_eq!(d["hunksTruncated"], true);
    assert!(out["text"].as_text().unwrap().len() < 1024 * 1024 + 200);
    assert_eq!(out["a_bytes"].as_bytes().unwrap().len(), a.len());
    let out = run(
        &reg,
        PortValue::Text("a\n".repeat(50000)),
        None,
        json!({"textB":"b\n".repeat(50001),"mode":"line"}),
    )
    .unwrap();
    assert_eq!(data(&out)["strategy"], "朴素（输入过大）");
}
#[test]
fn dual_input_graph_matches_direct_diff_and_downstream_bytes() {
    let reg = default_registry();
    let graph = serde_json::from_value(json!({"nodes":[
        {"id":"a","descriptorId":"text_input","params":{"text":"abcdef"}},
        {"id":"b","descriptorId":"text_input","params":{"text":"abcXef"}},
        {"id":"d","descriptorId":"diff_tool","params":{"inputEnc":"utf8","textB":"ignored"}},
        {"id":"s","descriptorId":"hex_stats","params":{}}
    ],"edges":[
        {"from":{"node":"a","port":"text"},"to":{"node":"d","port":"a"}},
        {"from":{"node":"b","port":"text"},"to":{"node":"d","port":"b"}},
        {"from":{"node":"d","port":"b_bytes"},"to":{"node":"s","port":"data"}}
    ]}))
    .unwrap();
    let out = GraphExecutor::new(&reg, &graph)
        .unwrap()
        .run(&NullSink, &CancellationToken::new())
        .unwrap();
    let direct = run(
        &reg,
        PortValue::Text("abcdef".into()),
        Some(PortValue::Text("abcXef".into())),
        json!({"inputEnc":"utf8"}),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&out["d"]).unwrap(),
        serde_json::to_value(direct).unwrap()
    );
    assert!(out.contains_key("s"));
}
#[test]
fn malformed_data_and_resource_limits_are_rejected() {
    let reg = default_registry();
    for (s, p) in [
        ("zz", json!({"inputEnc":"hex"})),
        ("ab", json!({"context":-1})),
        ("ab", json!({"context":1.5})),
        ("ab", json!({"mode":"invalid"})),
        ("ab", json!({"inputEnc":"invalid"})),
        ("a", json!({"inputEnc":"base64"})),
    ] {
        assert!(run(&reg, PortValue::Text(s.into()), None, p).is_err());
    }
    assert!(run(
        &reg,
        PortValue::Bytes(vec![0; 262145].into()),
        None,
        json!({})
    )
    .is_err());
    assert!(run(
        &reg,
        PortValue::Bytes(vec![255].into()),
        None,
        json!({"mode":"line"})
    )
    .is_err());
    assert!(run(&reg, PortValue::Json(json!({})), None, json!({})).is_err());
}
#[test]
fn every_diff_strategy_can_be_cancelled() {
    struct Stop(CancellationToken);
    impl ProgressSink for Stop {
        fn emit(&self, e: ProgressEvent) {
            if matches!(e,ProgressEvent::NodeProgress{pct,..} if pct>0.0 && pct<1.0) {
                self.0.cancel();
            }
        }
    }
    let reg = default_registry();
    for (n, m) in [(50000, 50000), (1300, 1301), (50000, 50001)] {
        let inputs = [
            ("a".into(), PortValue::Bytes(vec![0; n].into())),
            ("b".into(), PortValue::Bytes(vec![1; m].into())),
        ]
        .into();
        let cancel = CancellationToken::new();
        let out = GraphExecutor::run_node(
            &reg,
            "diff_tool",
            &inputs,
            &json!({}),
            &Stop(cancel.clone()),
            &cancel,
        );
        assert!(matches!(out, Err(CoreError::Cancelled)), "{n},{m}");
    }
}
