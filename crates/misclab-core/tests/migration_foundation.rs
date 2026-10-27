use misclab_core::{
    cancel::CancellationToken,
    error::CoreError,
    graph::{executor::GraphExecutor, port::PortValue},
    node::{registry::NodeRegistry, PortMap},
    nodes::default_registry,
    progress::{NullSink, ProgressEvent, ProgressSink},
};
use serde_json::{json, Value};

fn inputs(port: &str, value: PortValue) -> PortMap {
    [(port.into(), value)].into()
}
fn call(
    reg: &NodeRegistry,
    id: &str,
    port: &str,
    v: PortValue,
    p: &Value,
) -> Result<PortMap, CoreError> {
    GraphExecutor::run_node(
        reg,
        id,
        &inputs(port, v),
        p,
        &NullSink,
        &CancellationToken::new(),
    )
}
fn text(reg: &NodeRegistry, id: &str, s: &str, p: &Value) -> Result<PortMap, CoreError> {
    call(reg, id, "text", PortValue::Text(s.into()), p)
}

#[test]
fn pinned_foundation_vectors_cover_eight_operations() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/ebctf-foundation-batch14.json")).unwrap();
    assert_eq!(
        fixture["revision"],
        "ea2bc36dae31c97060f7f2cddad8df5518afad7b"
    );
    let vectors = fixture["vectors"].as_array().unwrap();
    assert_eq!(vectors.len(), 576);
    assert_eq!(
        vectors
            .iter()
            .map(|v| v["operation"].as_str().unwrap())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        8
    );
    let reg = default_registry();
    let mut errors = Vec::new();
    for (i, v) in vectors.iter().enumerate() {
        let id = v["node"].as_str().unwrap();
        let result = GraphExecutor::run_node(
            &reg,
            id,
            &serde_json::from_value(v["input"].clone()).unwrap(),
            &v["params"],
            &NullSink,
            &CancellationToken::new(),
        );
        if v.get("error").is_some() {
            if result.is_ok() {
                errors.push(format!("{i} {id}: expected error"));
            }
            continue;
        }
        match result {
            Err(e) => errors.push(format!("{i} {id}: {e}")),
            Ok(out) => {
                if out["text"].as_text().unwrap() != v["text"].as_str().unwrap() {
                    errors.push(format!(
                        "{i} {id} {:?} {:?}: got {:?}, expected {:?}",
                        v["input"],
                        v["params"],
                        out["text"].as_text().unwrap(),
                        v["text"]
                    ));
                }
                if let Some(expected) = v.get("bytes") {
                    let b: Vec<u8> = serde_json::from_value(expected.clone()).unwrap();
                    assert_eq!(
                        out["bytes"].as_bytes().unwrap().as_ref(),
                        b.as_slice(),
                        "{i}"
                    );
                }
            }
        }
    }
    assert!(
        errors.is_empty(),
        "{} differences\n{}",
        errors.len(),
        errors
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn dx_frames_preserve_binary_and_support_reproducible_salt() {
    use base64::Engine as _;
    let reg = default_registry();
    let all: Vec<u8> = (0..=255).collect();
    let mut exported = Vec::new();
    for n in [0, 1, 2, 3, 7, 16, 255, 256] {
        let data = &all[..n];
        for salt in ["", "1234"] {
            let out = call(
                &reg,
                "dx_base64_encode",
                "data",
                PortValue::Bytes(data.to_vec().into()),
                &json!({"salt":salt}),
            )
            .unwrap();
            let encoded = out["text"].as_text().unwrap();
            let back = text(&reg, "dx_base64_decode", encoded, &json!({})).unwrap();
            assert_eq!(back["bytes"].as_bytes().unwrap().as_ref(), data);
            let mut frame = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .unwrap();
            if !salt.is_empty() {
                assert_eq!(&frame[2..4], &[0x12, 0x34]);
            }
            frame[0] ^= 1;
            assert!(text(
                &reg,
                "dx_base64_decode",
                &base64::engine::general_purpose::STANDARD.encode(frame),
                &json!({})
            )
            .is_err());
            exported.push(json!({"encoded":encoded,"bytes":data}));
        }
    }
    assert_eq!(
        text(&reg, "dx_base64_decode", "", &json!({})).unwrap()["text"]
            .as_text()
            .unwrap(),
        ""
    );
    for salt in ["0", "001122", "zzzz"] {
        assert!(call(
            &reg,
            "dx_base64_encode",
            "data",
            PortValue::Text("test".into()),
            &json!({"salt":salt})
        )
        .is_err());
    }
    // Maintainer opt-in: feed real Rust frames to the pinned upstream decoder.
    if let Ok(path) = std::env::var("MISCLAB_DX_REFERENCE_EXPORT") {
        std::fs::write(path, serde_json::to_string_pretty(&exported).unwrap()).unwrap();
    }
}

#[test]
fn permissions_roundtrip_every_octal_mode_and_never_execute_chmod() {
    let reg = default_registry();
    for mode in 0..=0o7777 {
        let oct = format!("{mode:04o}");
        let out = text(&reg, "unix_permissions", &oct, &json!({})).unwrap();
        let data = serde_json::to_value(&out["data"]).unwrap();
        let sym = data["value"]["symbolic"].as_str().unwrap();
        let back = text(&reg, "unix_permissions", sym, &json!({})).unwrap();
        let back = serde_json::to_value(&back["data"]).unwrap();
        assert_eq!(back["value"]["octal"], oct);
    }
}

#[test]
fn statistics_and_planes_expose_exact_structured_results() {
    let reg = default_registry();
    let out = call(
        &reg,
        "hex_stats",
        "data",
        PortValue::Bytes((0..=255).collect::<Vec<u8>>().into()),
        &json!({"window":16}),
    )
    .unwrap();
    let data = serde_json::to_value(&out["data"]).unwrap();
    let d = &data["value"];
    assert_eq!(d["total"], 256);
    assert_eq!(d["printable"], 98);
    assert_eq!(d["entropy"], 8.0);
    assert_eq!(d["counts"], json!(vec![1; 256]));
    assert_eq!(d["buckets"], json!([32, 95, 129]));
    assert_eq!(d["chunks"].as_array().unwrap().len(), 16);
    assert_eq!(d["chunks"][0]["entropy"], 4.0);
    for (plane, expected) in [("0", "0101"), ("7", "0011")] {
        let out = call(
            &reg,
            "bit_plane_extract",
            "data",
            PortValue::Bytes(vec![0, 1, 128, 255].into()),
            &json!({"plane":plane}),
        )
        .unwrap();
        assert_eq!(out["text"].as_text().unwrap(), expected);
    }
    let out = call(
        &reg,
        "hex_stats",
        "data",
        PortValue::Bytes(vec![0; 40_000].into()),
        &json!({"window":16}),
    )
    .unwrap();
    assert!(out["text"].as_text().unwrap().contains("已显示前 2048 段"));
    let data = serde_json::to_value(&out["data"]).unwrap();
    assert_eq!(data["value"]["chunks"].as_array().unwrap().len(), 2500);
}

#[test]
fn range_to_dx_to_statistics_graph_equals_standalone() {
    let reg = default_registry();
    let graph = serde_json::from_value(json!({"nodes":[
        {"id":"r","descriptorId":"hex_range","params":{"start":"1","end":"3"}},
        {"id":"e","descriptorId":"dx_base64_encode","params":{"salt":"1234"}},
        {"id":"d","descriptorId":"dx_base64_decode","params":{}},
        {"id":"s","descriptorId":"hex_stats","params":{}}
    ],"edges":[
        {"from":{"node":"r","port":"bytes"},"to":{"node":"e","port":"data"}},
        {"from":{"node":"e","port":"text"},"to":{"node":"d","port":"text"}},
        {"from":{"node":"d","port":"bytes"},"to":{"node":"s","port":"data"}}
    ]}))
    .unwrap();
    let seed = [(
        ("r".into(), "data".into()),
        PortValue::Bytes(vec![99, 0, 255, 65, 77].into()),
    )]
    .into();
    let out = GraphExecutor::new(&reg, &graph)
        .unwrap()
        .with_seed_inputs(seed)
        .run(&NullSink, &CancellationToken::new())
        .unwrap();
    assert_eq!(
        out["d"]["bytes"].as_bytes().unwrap().as_ref(),
        &[0, 255, 65]
    );
    let direct = call(
        &reg,
        "hex_stats",
        "data",
        PortValue::Bytes(vec![0, 255, 65].into()),
        &json!({}),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&out["s"]).unwrap(),
        serde_json::to_value(direct).unwrap()
    );
}

#[test]
fn bounded_parsers_reject_bad_inputs_and_depth() {
    let reg = default_registry();
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/ebctf-foundation-batch14.json")).unwrap();
    let error = text(
        &reg,
        "dx_base64_decode",
        fixture["overlimitFrame"].as_str().unwrap(),
        &json!({}),
    )
    .unwrap_err();
    assert!(error.to_string().contains("8 MiB"));
    for (id, input, p) in [
        (
            "rison_encode",
            format!("{}0{}", "[".repeat(70), "]".repeat(70)),
            json!({}),
        ),
        (
            "rison_decode",
            format!("{}0{}", "!(".repeat(70), ")".repeat(70)),
            json!({}),
        ),
        ("uuid_parse", "z".repeat(5000), json!({})),
        ("object_id_time", "5".repeat(5000), json!({})),
    ] {
        assert!(text(&reg, id, &input, &p).is_err(), "{id}");
    }
    assert!(call(
        &reg,
        "hex_range",
        "data",
        PortValue::Text("00".into()),
        &json!({"start":"NaN"})
    )
    .is_err());
    for id in ["hex_range", "hex_stats", "bit_plane_extract"] {
        assert!(call(
            &reg,
            id,
            "data",
            PortValue::Bytes(vec![0; 1_048_577].into()),
            &json!({})
        )
        .is_err());
    }
    assert!(call(
        &reg,
        "dx_base64_encode",
        "data",
        PortValue::Bytes(vec![0; 8 * 1024 * 1024 + 1].into()),
        &json!({})
    )
    .is_err());
}

#[test]
fn running_foundation_operations_observe_cancellation() {
    struct Stop(CancellationToken);
    impl ProgressSink for Stop {
        fn emit(&self, e: ProgressEvent) {
            if matches!(e,ProgressEvent::NodeProgress{pct,..} if pct>0.0 && pct<1.0) {
                self.0.cancel();
            }
        }
    }
    let reg = default_registry();
    for (id, port, value) in [
        (
            "dx_base64_encode",
            "data",
            PortValue::Bytes(vec![65; 1_048_576].into()),
        ),
        (
            "hex_stats",
            "data",
            PortValue::Bytes(vec![65; 1_048_576].into()),
        ),
        (
            "bit_plane_extract",
            "data",
            PortValue::Bytes(vec![65; 1_048_576].into()),
        ),
        (
            "rison_encode",
            "text",
            PortValue::Text(format!("[{}0]", "0,".repeat(1000))),
        ),
        (
            "rison_decode",
            "text",
            PortValue::Text(format!("!({}0)", "0,".repeat(1000))),
        ),
        (
            "dx_base64_decode",
            "text",
            PortValue::Text("Bl23oUTpemh+dubZBcJaEt5MqEd4zeWltw==".into()),
        ),
    ] {
        let cancel = CancellationToken::new();
        let out = GraphExecutor::run_node(
            &reg,
            id,
            &inputs(port, value),
            &json!({}),
            &Stop(cancel.clone()),
            &cancel,
        );
        assert!(matches!(out, Err(CoreError::Cancelled)), "{id}: {out:?}");
    }
}
