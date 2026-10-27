use super::prelude::*;
use crate::{cancel::CancellationToken, graph::executor::GraphExecutor, progress::NullSink};
use serde_json::{json, Value};

fn run(id: &str, inputs: &PortMap, params: &Value) -> Result<PortMap, CoreError> {
    GraphExecutor::run_node(
        &super::default_registry(),
        id,
        inputs,
        params,
        &NullSink,
        &CancellationToken::new(),
    )
}

#[test]
fn pinned_base_vectors_cover_all_eight_operations_and_variants() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/ebctf-base-batch3.json")).unwrap();
    assert_eq!(
        fixture["revision"],
        "ea2bc36dae31c97060f7f2cddad8df5518afad7b"
    );
    assert_eq!(fixture["vectors"].as_array().unwrap().len(), 314);
    for (i, v) in fixture["vectors"].as_array().unwrap().iter().enumerate() {
        let id = v["node"].as_str().unwrap();
        let result = run(
            id,
            &serde_json::from_value(v["input"].clone()).unwrap(),
            &v["params"],
        );
        if v.get("error").is_some() {
            assert!(result.is_err(), "{i}: {id}");
            continue;
        }
        let output = result.unwrap_or_else(|e| panic!("{i} {id}: {e}"));
        if let Some(expected) = v.get("text") {
            assert_eq!(
                output["text"].as_text().unwrap(),
                expected.as_str().unwrap(),
                "{i} {id}"
            );
        }
        if let Some(expected) = v.get("bytes") {
            let bytes: Vec<u8> = serde_json::from_value(expected.clone()).unwrap();
            assert_eq!(&*output["bytes"].as_bytes().unwrap(), bytes, "{i} {id}");
        }
    }
}

#[test]
fn base_binary_roundtrips_and_legacy_port_names_and_defaults() {
    let data: Vec<u8> = (0..=255).collect();
    for (id, input, p) in [
        ("hex", "text", json!({})),
        ("base64", "text", json!({})),
        ("base32", "data", json!({})),
        ("base45", "data", json!({})),
        ("base58", "data", json!({})),
        ("base62", "data", json!({"preserveZeros":true})),
        ("base85", "data", json!({})),
        ("base92", "data", json!({})),
    ] {
        for len in [0, 1, 2, 3, 4, 5, 7, 256] {
            let enc = run(
                &format!("{id}_encode"),
                &one(input, PortValue::Bytes(data[..len].to_vec().into())),
                &p,
            )
            .unwrap();
            let dec = run(
                &format!("{id}_decode"),
                &one("text", enc["text"].clone()),
                &p,
            )
            .unwrap();
            assert_eq!(
                &*dec["bytes"].as_bytes().unwrap(),
                &data[..len],
                "{id} {len}"
            );
        }
    }
    let legacy = [
        ("hex_encode", "text", "Hello", "48656c6c6f"),
        ("base64_encode", "text", "Hello", "SGVsbG8="),
        ("base62_encode", "data", "\0\0\0", "0"),
        ("base92_encode", "data", "", ""),
        ("base85_encode", "data", "Hello", "87cURDZ"),
    ];
    for (id, port, input, expected) in legacy {
        assert_eq!(
            run(id, &one(port, PortValue::Text(input.into())), &json!({})).unwrap()["text"]
                .as_text()
                .unwrap(),
            expected,
            "{id}"
        );
    }
}

#[test]
fn malformed_base_blocks_and_ambiguous_alphabets_are_rejected() {
    for (id, text, p) in [
        ("base32_decode", "M", json!({"strip":false})),
        ("base32_decode", "MZ======", json!({"strip":false})),
        ("base45_decode", "::", json!({"strip":false})),
        ("base45_decode", "0", json!({"strip":false})),
        ("base85_decode", "u", json!({"strip":false})),
        ("base85_decode", "uuuuu", json!({"strip":false})),
        ("base92_decode", "}}", json!({})),
        ("base64_decode", "AB==", json!({})),
        (
            "base58_encode",
            "x",
            json!({"variant":"自定义","alphabet":"a".repeat(58)}),
        ),
        ("base62_encode", "x", json!({"alphabet":"aa"})),
        (
            "base32_encode",
            "x",
            json!({"variant":"自定义","alphabet":"a".repeat(32)}),
        ),
    ] {
        let port = if id.ends_with("_encode") {
            "data"
        } else {
            "text"
        };
        assert!(
            run(id, &one(port, PortValue::Text(text.into())), &p).is_err(),
            "{id}"
        );
    }
}

#[test]
fn all_migrated_examples_are_runnable_through_graph_and_standalone() {
    let reg = super::default_registry();
    let nodes = reg.descriptors();
    assert!(nodes.iter().filter(|d| !d.examples.is_empty()).count() >= 77);
    for d in nodes.iter().filter(|d| !d.examples.is_empty()) {
        for ex in &d.examples {
            let direct =
                run(&d.id, &ex.inputs, &ex.params).unwrap_or_else(|e| panic!("{}: {e}", d.id));
            let graph = serde_json::from_value(
                json!({"nodes":[{"id":"x","descriptorId":d.id,"params":ex.params}],"edges":[]}),
            )
            .unwrap();
            let seed = ex
                .inputs
                .iter()
                .map(|(port, v)| (("x".into(), port.clone()), v.clone()))
                .collect();
            let from_graph = GraphExecutor::new(&reg, &graph)
                .unwrap()
                .with_seed_inputs(seed)
                .run(&NullSink, &CancellationToken::new())
                .unwrap();
            assert_eq!(
                direct.keys().collect::<std::collections::HashSet<_>>(),
                from_graph["x"].keys().collect()
            );
            for (port, value) in &direct {
                if port == "artifact" || (d.id == "artifact_store" && port == "metadata") {
                    continue;
                }
                assert_eq!(
                    serde_json::to_value(value).unwrap(),
                    serde_json::to_value(&from_graph["x"][port]).unwrap(),
                    "{} {port}",
                    d.id
                );
            }
        }
    }
}

#[test]
fn big_radix_nodes_cancel_during_encoding_and_decoding() {
    use crate::progress::{ProgressEvent, ProgressSink};
    struct Stop(CancellationToken);
    impl ProgressSink for Stop {
        fn emit(&self, event: ProgressEvent) {
            if matches!(event, ProgressEvent::NodeProgress { pct, .. } if pct > 0.0 && pct < 1.0) {
                self.0.cancel();
            }
        }
    }
    let reg = super::default_registry();
    for id in ["base58", "base62"] {
        for (direction, port, value) in [
            ("encode", "data", PortValue::Bytes(vec![255; 4096].into())),
            ("decode", "text", PortValue::Text("A".repeat(4096))),
        ] {
            let cancel = CancellationToken::new();
            let sink = Stop(cancel.clone());
            let out = GraphExecutor::run_node(
                &reg,
                &format!("{id}_{direction}"),
                &one(port, value),
                &json!({}),
                &sink,
                &cancel,
            );
            assert!(matches!(out, Err(CoreError::Cancelled)), "{id} {direction}");
        }
    }
}
