//! Frozen upstream vectors plus binary, error, lifecycle and descriptor checks.
use super::prelude::*;
use crate::{
    cancel::CancellationToken,
    graph::executor::GraphExecutor,
    progress::{NullSink, ProgressEvent, ProgressSink},
};
use serde_json::{json, Value};

fn run(id: &str, inputs: PortMap, params: &Value) -> Result<PortMap, CoreError> {
    GraphExecutor::run_node(
        &super::default_registry(),
        id,
        &inputs,
        params,
        &NullSink,
        &CancellationToken::new(),
    )
}
fn text(id: &str, port: &str, text: &str, params: &Value) -> Result<PortMap, CoreError> {
    run(id, one(port, PortValue::Text(text.into())), params)
}
#[test]
fn pinned_upstream_vectors_batch2() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/ebctf-batch2.json")).unwrap();
    assert_eq!(
        fixture["revision"],
        "ea2bc36dae31c97060f7f2cddad8df5518afad7b"
    );
    assert_eq!(fixture["vectors"].as_array().unwrap().len(), 177);
    for (i, v) in fixture["vectors"].as_array().unwrap().iter().enumerate() {
        let id = v["node"].as_str().unwrap();
        let result = run(
            id,
            serde_json::from_value(v["input"].clone()).unwrap(),
            &v["params"],
        );
        if v.get("error").is_some() {
            assert!(result.is_err(), "vector {i} {id}");
            continue;
        }
        let out = result.unwrap_or_else(|e| panic!("vector {i} {id}: {e}"));
        if let Some(expected) = v.get("text") {
            assert_eq!(
                out["text"].as_text().unwrap(),
                expected.as_str().unwrap(),
                "vector {i} {id}"
            );
        }
        if let Some(expected) = v.get("bytes") {
            let expected: Vec<u8> = serde_json::from_value(expected.clone()).unwrap();
            assert_eq!(
                &*out["bytes"].as_bytes().unwrap(),
                expected,
                "vector {i} {id}"
            );
        }
        if let Some(expected) = v.get("metadata") {
            let PortValue::Json(meta) = &out["metadata"] else {
                panic!("metadata type")
            };
            for (key, value) in expected.as_object().unwrap() {
                assert_eq!(&meta[key], value, "vector {i} metadata {key}");
            }
        }
        if let Some(expected) = v.get("zlibRoundtrip") {
            let decoded = text(
                "base64_zlib_decode",
                "text",
                out["text"].as_text().unwrap(),
                &json!({}),
            )
            .unwrap();
            let expected: Vec<u8> = serde_json::from_value(expected.clone()).unwrap();
            assert_eq!(&*decoded["bytes"].as_bytes().unwrap(), expected, "zlib {i}");
        }
    }
}
#[test]
fn all_binary_bytes_and_partial_blocks_roundtrip_batch2() {
    let all: Vec<u8> = (0..=255).collect();
    for id in [
        "z85",
        "base64_dict",
        "multiline_base64",
        "base64_zlib",
        "netbios",
        "bit_reverse",
        "bit_rotate",
        "byte_swap",
        "byte_reverse",
    ] {
        for length in [0, 1, 2, 3, 4, 5, 7, 16, 255, 256] {
            let bytes = all[..length].to_vec();
            let enc = run(
                &format!("{id}_encode"),
                one("data", PortValue::Bytes(bytes.clone().into())),
                &json!({}),
            )
            .unwrap();
            let dec = text(
                &format!("{id}_decode"),
                "text",
                enc["text"].as_text().unwrap(),
                &json!({}),
            )
            .unwrap();
            assert_eq!(
                &*dec["bytes"].as_bytes().unwrap(),
                bytes,
                "{id} length {length}"
            );
        }
    }
}
#[test]
fn malformed_and_truncated_inputs_fail_without_panics_batch2() {
    for (id, input, p) in [
        ("z85_decode", "a", json!({})),
        ("z85_decode", "#####", json!({})),
        ("base85ipv6_decode", "~~~~~~~~~~~~~~~~~~~~", json!({})),
        ("base85ipv6_encode", "short", json!({})),
        (
            "base64_dict_decode",
            "SGk=!",
            json!({"ignoreUnknown":false}),
        ),
        ("base64_dict_encode", "hi", json!({"key":"a".repeat(64)})),
        (
            "multiline_base64_decode",
            "!!",
            json!({"skipInvalid":false}),
        ),
        ("base64_zlib_decode", "", json!({})),
        ("base64_zlib_decode", "eJw=", json!({})),
        ("netbios_decode", "A", json!({})),
        ("netbios_decode", "ZZ", json!({})),
        ("caret_m_decode", "^", json!({})),
        ("caret_m_decode", "M-^", json!({})),
        (
            "byte_swap_decode",
            "000102",
            json!({"mode":"hex","group":"2"}),
        ),
        ("bit_rotate_decode", "00", json!({"bits":8})),
        ("varint_decode", "80", json!({})),
        ("varint_encode", "-1", json!({})),
        ("hex_reverse_encode", "123", json!({})),
        ("data_uri_decode", "http://example.com", json!({})),
        ("data_uri_decode", "data:;base64,!!", json!({})),
    ] {
        let port = if ["base85ipv6_encode", "base64_dict_encode"].contains(&id) {
            "data"
        } else {
            "text"
        };
        assert!(text(id, port, input, &p).is_err(), "{id}");
    }
}
#[test]
fn custom_alphabet_and_rfc1924_native_improvements_batch2() {
    let dict: String =
        "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-:+=^!/*?&<>()[]{}@%$#"
            .chars()
            .rev()
            .collect();
    let p = json!({"dict":dict});
    let enc = text("z85_encode", "data", "hello🙂", &p).unwrap();
    let dec = text("z85_decode", "text", enc["text"].as_text().unwrap(), &p).unwrap();
    assert_eq!(dec["text"].as_text().unwrap(), "hello🙂");
    let all: Vec<u8> = (0..=255).collect();
    let encoded = run(
        "data_uri_encode",
        one("data", PortValue::Bytes(all.clone().into())),
        &json!({"base64":false}),
    )
    .unwrap();
    let decoded = text(
        "data_uri_decode",
        "text",
        encoded["text"].as_text().unwrap(),
        &json!({}),
    )
    .unwrap();
    assert_eq!(&*decoded["bytes"].as_bytes().unwrap(), all);
}
#[test]
fn data_uri_artifacts_are_real_bytes_and_report_mime_mismatch() {
    let out = text(
        "data_uri_decode",
        "text",
        "data:text/plain;base64,iVBORw0KGgo=",
        &json!({}),
    )
    .unwrap();
    let PortValue::Artifact(id) = out["artifact"] else {
        panic!("artifact")
    };
    let meta = crate::artifacts::repository().metadata(id).unwrap();
    assert_eq!(meta.mime, "image/png");
    let read = run(
        "artifact_read",
        one("artifact", out["artifact"].clone()),
        &json!({}),
    )
    .unwrap();
    assert_eq!(&*read["bytes"].as_bytes().unwrap(), b"\x89PNG\r\n\x1a\n");
    let PortValue::Json(meta) = &out["metadata"] else {
        panic!("metadata")
    };
    assert_eq!(meta["mimeMismatch"], true);
    assert!(run(
        "artifact_read",
        one("artifact", PortValue::Text(id.to_string())),
        &json!({})
    )
    .is_err());
}
struct CancelOnProgress(CancellationToken);
impl ProgressSink for CancelOnProgress {
    fn emit(&self, event: ProgressEvent) {
        if matches!(event, ProgressEvent::NodeProgress { .. }) {
            self.0.cancel();
        }
    }
}
#[test]
fn zlib_limits_corruption_and_in_progress_cancellation_batch2() {
    let bytes = vec![b'A'; 128 * 1024];
    let inputs = one("data", PortValue::Bytes(bytes.into()));
    let encoded = run("base64_zlib_encode", inputs.clone(), &json!({})).unwrap();
    assert!(text(
        "base64_zlib_decode",
        "text",
        encoded["text"].as_text().unwrap(),
        &json!({"maxBytes":10})
    )
    .is_err());
    let compressed =
        super::transport_codecs::b64decode(encoded["text"].as_text().unwrap()).unwrap();
    use base64::Engine as _;
    for missing in [1, 2, 4, 8] {
        let truncated = base64::engine::general_purpose::STANDARD
            .encode(&compressed[..compressed.len() - missing]);
        assert!(text("base64_zlib_decode", "text", &truncated, &json!({})).is_err());
    }
    let cancel = CancellationToken::new();
    let sink = CancelOnProgress(cancel.clone());
    assert!(matches!(
        GraphExecutor::run_node(
            &super::default_registry(),
            "base64_zlib_encode",
            &inputs,
            &json!({}),
            &sink,
            &cancel
        ),
        Err(CoreError::Cancelled)
    ));
    let cancel = CancellationToken::new();
    let sink = CancelOnProgress(cancel.clone());
    assert!(matches!(
        GraphExecutor::run_node(
            &super::default_registry(),
            "base64_zlib_decode",
            &one("text", encoded["text"].clone()),
            &json!({}),
            &sink,
            &cancel
        ),
        Err(CoreError::Cancelled)
    ));
}
#[test]
fn every_batch2_node_has_a_runnable_example_and_parameter_help() {
    let upstream = [
        "z85",
        "base85ipv6",
        "base64dict",
        "multilineBase64",
        "base64decompress",
        "netbios",
        "caretMdecode",
        "httpBasicAuth",
        "bitReverse",
        "bitRotate",
        "byteSwap",
        "byteReverse",
        "hexReverse",
        "varint",
        "b64urlJson",
        "dataUriParse",
        "urlQueryParse",
        "cookieParse",
        "asciiControl",
        "splitHex",
    ];
    let reg = super::default_registry();
    let nodes: Vec<_> = reg
        .descriptors()
        .into_iter()
        .filter(|d| {
            d.upstream_ids
                .iter()
                .any(|id| upstream.contains(&id.as_str()))
        })
        .collect();
    assert_eq!(nodes.len(), 36);
    for d in nodes {
        assert!(!d.examples.is_empty(), "{}", d.id);
        assert!(
            d.params.iter().all(|p| !p.description.is_empty()),
            "{}",
            d.id
        );
        for ex in d.examples {
            let mut standalone = run(&d.id, ex.inputs.clone(), &ex.params)
                .unwrap_or_else(|e| panic!("{} example: {e}", d.id));
            let graph = serde_json::from_value(json!({"nodes":[{"id":"sample", "descriptorId":d.id,"params":ex.params}],"edges":[]})).unwrap();
            let seed = ex
                .inputs
                .into_iter()
                .map(|(port, v)| (("sample".into(), port), v))
                .collect();
            let mut graph_out = GraphExecutor::new(&reg, &graph)
                .unwrap()
                .with_seed_inputs(seed)
                .run(&NullSink, &CancellationToken::new())
                .unwrap()
                .remove("sample")
                .unwrap();
            // Stored artifacts get independent UUIDs; compare their bytes instead.
            for outputs in [&mut standalone, &mut graph_out] {
                if let Some(PortValue::Artifact(id)) = outputs.remove("artifact") {
                    outputs.insert(
                        "artifactBytes".into(),
                        PortValue::Bytes(
                            crate::artifacts::repository()
                                .read_all(id, crate::artifacts::MAX_ARTIFACT_BYTES)
                                .unwrap()
                                .into(),
                        ),
                    );
                }
            }
            assert_eq!(
                serde_json::to_value(standalone).unwrap(),
                serde_json::to_value(graph_out).unwrap(),
                "{} graph/standalone mismatch",
                d.id
            );
        }
    }
}

#[test]
fn binary_artifact_chain_requires_explicit_read_and_preserves_bytes() {
    let graph = serde_json::from_value(json!({
        "nodes":[{"id":"store","descriptorId":"artifact_store","params":{}}, {"id":"read","descriptorId":"artifact_read","params":{}}, {"id":"encode","descriptorId":"base64_dict_encode","params":{}}],
        "edges":[{"from":{"node":"store","port":"artifact"},"to":{"node":"read","port":"artifact"}}, {"from":{"node":"read","port":"bytes"},"to":{"node":"encode","port":"data"}}]
    })).unwrap();
    let reg = super::default_registry();
    let seed = [(
        ("store".into(), "bytes".into()),
        PortValue::Bytes(vec![0, 255, 65].into()),
    )]
    .into();
    let outputs = GraphExecutor::new(&reg, &graph)
        .unwrap()
        .with_seed_inputs(seed)
        .run(&NullSink, &CancellationToken::new())
        .unwrap();
    assert_eq!(outputs["encode"]["text"].as_text().unwrap(), "AP9B");
    for limit in [-1.0, 0.5, 2.0, 134217729.0] {
        assert!(run(
            "artifact_read",
            one("artifact", outputs["store"]["artifact"].clone()),
            &json!({"maxBytes":limit})
        )
        .is_err());
    }
}
