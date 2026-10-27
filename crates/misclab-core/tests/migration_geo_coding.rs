use misclab_core::{
    cancel::CancellationToken,
    error::CoreError,
    graph::{executor::GraphExecutor, port::PortValue},
    node::{registry::NodeRegistry, PortMap},
    nodes::default_registry,
    progress::{NullSink, ProgressEvent, ProgressSink},
};
use serde_json::{json, Value};
fn call(
    reg: &NodeRegistry,
    id: &str,
    port: &str,
    value: PortValue,
    p: Value,
) -> Result<PortMap, CoreError> {
    GraphExecutor::run_node(
        reg,
        id,
        &[(port.into(), value)].into(),
        &p,
        &NullSink,
        &CancellationToken::new(),
    )
}
fn text(reg: &NodeRegistry, id: &str, s: &str, p: Value) -> Result<PortMap, CoreError> {
    call(
        reg,
        id,
        if id.starts_with("huffman") {
            "data"
        } else {
            "text"
        },
        PortValue::Text(s.into()),
        p,
    )
}
#[test]
fn pinned_vectors_cover_seven_operations_and_all_declared_options() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/ebctf-geo-coding-batch15.json")).unwrap();
    assert_eq!(
        fixture["revision"],
        "ea2bc36dae31c97060f7f2cddad8df5518afad7b"
    );
    let vectors = fixture["vectors"].as_array().unwrap();
    assert_eq!(vectors.len(), 820);
    assert_eq!(
        vectors
            .iter()
            .map(|v| v["operation"].as_str().unwrap())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        7
    );
    let reg = default_registry();
    let mut errors = Vec::new();
    for (i, v) in vectors.iter().enumerate() {
        let id = v["node"].as_str().unwrap();
        let out = GraphExecutor::run_node(
            &reg,
            id,
            &serde_json::from_value(v["input"].clone()).unwrap(),
            &v["params"],
            &NullSink,
            &CancellationToken::new(),
        );
        if v.get("error").is_some() {
            if out.is_ok() {
                errors.push(format!("{i} {id}: expected error"));
            }
            continue;
        }
        match out {
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
            }
        }
    }
    assert!(
        errors.is_empty(),
        "{} differences\n{}",
        errors.len(),
        errors.into_iter().take(25).collect::<Vec<_>>().join("\n")
    );
}
#[test]
fn hamming_corrects_every_single_error_and_reports_padding() {
    let reg = default_registry();
    for k in [1, 2, 4, 11, 26, 57] {
        let s = (0..k)
            .map(|i| if i % 3 == 0 { '1' } else { '0' })
            .collect::<String>();
        let enc = text(&reg, "hamming_code_encode", &s, json!({"k":k})).unwrap();
        let code = enc["text"].as_text().unwrap();
        for i in 0..code.len() {
            let mut flipped = code.as_bytes().to_vec();
            flipped[i] ^= 1;
            let out = text(
                &reg,
                "hamming_code_decode",
                std::str::from_utf8(&flipped).unwrap(),
                json!({"k":k}),
            )
            .unwrap();
            assert_eq!(out["text"].as_text().unwrap(), s);
            let PortValue::Json(j) = &out["json"] else {
                panic!()
            };
            assert_eq!(j["corrections"], json!([{"block":0,"position":i+1}]));
        }
    }
    let out = text(&reg, "hamming_code_encode", "101", json!({})).unwrap();
    let PortValue::Json(j) = &out["json"] else {
        panic!()
    };
    assert_eq!(j["paddingBits"], 1);
    let out = text(
        &reg,
        "hamming_code_decode",
        &(out["text"].as_text().unwrap().to_owned() + "11"),
        json!({}),
    )
    .unwrap();
    let PortValue::Json(j) = &out["json"] else {
        panic!()
    };
    assert_eq!(j["ignoredTrailingBits"], 2);
    assert_eq!(out["text"].as_text().unwrap(), "1010");
}
#[test]
fn huffman_preserves_binary_unicode_and_long_tables_without_report_truncation() {
    let reg = default_registry();
    for bytes in [
        (0..=255).collect::<Vec<u8>>(),
        b"ABBA".to_vec(),
        "\u{feff}中文🙂".as_bytes().to_vec(),
        vec![0; 8193],
        (0..=255).cycle().take(65536).collect(),
    ] {
        let enc = call(
            &reg,
            "huffman_encode",
            "data",
            PortValue::Bytes(bytes.clone().into()),
            json!({}),
        )
        .unwrap();
        let dec = call(
            &reg,
            "huffman_decode",
            "data",
            enc["packet"].clone(),
            json!({}),
        )
        .unwrap();
        assert_eq!(dec["bytes"].as_bytes().unwrap().as_ref(), bytes);
        let inputs = [
            ("data".into(), enc["bytes"].clone()),
            ("code_table".into(), enc["code_table"].clone()),
            ("bit_length".into(), enc["bit_length"].clone()),
        ]
        .into();
        let explicit = GraphExecutor::run_node(
            &reg,
            "huffman_decode",
            &inputs,
            &json!({}),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(explicit).unwrap(),
            serde_json::to_value(dec).unwrap()
        );
        if bytes.len() > 4096 && bytes[0] != bytes[1] {
            assert!(enc["text"].as_text().unwrap().contains("截断"));
            assert!(enc["hex"].as_text().unwrap().len() > 4096);
        }
    }
    // Zero weights create a 255-deep tree: arbitrary-width canonical assignment,
    // without the upstream JS 32-bit shift / Number precision limitation.
    let weights = (0..=255)
        .map(|i| format!("\\x{i:02x} 0"))
        .collect::<Vec<_>>()
        .join("\n");
    let bytes = (0..=255).collect::<Vec<u8>>();
    let enc = call(
        &reg,
        "huffman_encode",
        "data",
        PortValue::Bytes(bytes.clone().into()),
        json!({"weights":weights}),
    )
    .unwrap();
    let dec = call(
        &reg,
        "huffman_decode",
        "data",
        enc["packet"].clone(),
        json!({}),
    )
    .unwrap();
    assert_eq!(dec["bytes"].as_bytes().unwrap().as_ref(), bytes);
}
#[test]
fn coordinate_boundaries_and_structured_precision_are_usable() {
    let reg = default_registry();
    let hash = text(
        &reg,
        "geo_hash_encode",
        "57.64911,10.40744",
        json!({"precision":11}),
    )
    .unwrap();
    assert_eq!(hash["text"].as_text().unwrap(), "u4pruydqqvj");
    let plus = text(
        &reg,
        "geo_plus_code_encode",
        "47.36559,8.524997",
        json!({"codeLength":"10"}),
    )
    .unwrap();
    assert_eq!(plus["text"].as_text().unwrap(), "8FVC9G8F+6X");
    let utm = text(&reg, "geo_utm_encode", "0,180", json!({})).unwrap();
    assert_eq!(utm["text"].as_text().unwrap(), "60N 833979 0");
    assert!(text(
        &reg,
        "geo_utm_decode",
        utm["text"].as_text().unwrap(),
        json!({})
    )
    .is_ok());
    for s in ["90,180", "-90,-180", "90,-180", "-90,180"] {
        let enc = text(&reg, "geo_maidenhead_encode", s, json!({"pairs":"4"})).unwrap();
        let dec = text(
            &reg,
            "geo_maidenhead_decode",
            enc["text"].as_text().unwrap(),
            json!({}),
        )
        .unwrap();
        let PortValue::Number(lat) = dec["latitude"] else {
            panic!()
        };
        assert!(lat.abs() <= 90.0);
    }
    let dec = text(&reg, "geo_hash_decode", "u4pruydqqvj", json!({})).unwrap();
    let PortValue::Json(j) = &dec["json"] else {
        panic!()
    };
    assert!(j["bounds"]["latMin"].as_f64().unwrap() < j["latitude"].as_f64().unwrap());
}
#[test]
fn graph_packet_matches_standalone_and_continues_into_byte_analysis() {
    let reg = default_registry();
    let bytes = (0..=255).collect::<Vec<u8>>();
    let graph = serde_json::from_value(json!({"nodes":[
        {"id":"e","descriptorId":"huffman_encode","params":{}},
        {"id":"d","descriptorId":"huffman_decode","params":{}},
        {"id":"s","descriptorId":"hex_stats","params":{}}
    ],"edges":[
        {"from":{"node":"e","port":"packet"},"to":{"node":"d","port":"data"}},
        {"from":{"node":"d","port":"bytes"},"to":{"node":"s","port":"data"}}
    ]}))
    .unwrap();
    let seed = [(
        ("e".into(), "data".into()),
        PortValue::Bytes(bytes.clone().into()),
    )]
    .into();
    let out = GraphExecutor::new(&reg, &graph)
        .unwrap()
        .with_seed_inputs(seed)
        .run(&NullSink, &CancellationToken::new())
        .unwrap();
    assert_eq!(out["d"]["bytes"].as_bytes().unwrap().as_ref(), bytes);
    let direct = call(
        &reg,
        "hex_stats",
        "data",
        PortValue::Bytes(bytes.into()),
        json!({}),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&out["s"]).unwrap(),
        serde_json::to_value(direct).unwrap()
    );
}
#[test]
fn invalid_tables_parameters_and_resource_limits_are_rejected() {
    let reg = default_registry();
    for (id, s, p) in [
        ("geo_hash_encode", "0,0", json!({"precision":0})),
        ("geo_plus_code_encode", "0,0", json!({"codeLength":"3"})),
        ("geo_maidenhead_encode", "0,0", json!({"pairs":"5"})),
        ("geo_utm_decode", "1C 999999999 999999999", json!({})),
        ("geo_dms_decode", "1oopsN,2E", json!({})),
        ("hamming_code_encode", "1", json!({"k":65537})),
        ("huffman_encode", "a", json!({"mode":"unknown"})),
        ("huffman_encode", "a", json!({"weights":"中 1"})),
        (
            "huffman_encode",
            "a",
            json!({"mode":"table","table":"a 0\nb 01"}),
        ),
        ("huffman_decode", "00", json!({"weights":"a 1","bitLen":-1})),
        (
            "huffman_decode",
            "00",
            json!({"weights":"a 1","bitLen":1.5}),
        ),
        (
            "huffman_decode",
            "ff",
            json!({"mode":"table","table":"a 0"}),
        ),
    ] {
        assert!(text(&reg, id, s, p).is_err(), "{id}");
    }
    for id in [
        "hamming_code_encode",
        "hamming_code_decode",
        "huffman_encode",
    ] {
        assert!(text(&reg, id, &"1".repeat(1_048_577), json!({})).is_err());
    }
    assert!(call(
        &reg,
        "huffman_decode",
        "data",
        PortValue::Json(json!({"format":"huffman-msb","hex":"00","bitLen":9,"table":"a 0"})),
        json!({})
    )
    .is_err());
}
#[test]
fn hamming_and_huffman_cancel_during_processing() {
    struct Stop(CancellationToken);
    impl ProgressSink for Stop {
        fn emit(&self, e: ProgressEvent) {
            if matches!(e,ProgressEvent::NodeProgress{pct,..} if pct>0.0 && pct<1.0) {
                self.0.cancel();
            }
        }
    }
    let reg = default_registry();
    for (id, port, input, p) in [
        (
            "hamming_code_encode",
            "text",
            PortValue::Text("1011".repeat(16384)),
            json!({}),
        ),
        (
            "hamming_code_decode",
            "text",
            PortValue::Text("0110011".repeat(16384)),
            json!({}),
        ),
        (
            "huffman_encode",
            "data",
            PortValue::Bytes(vec![65; 65536].into()),
            json!({}),
        ),
        (
            "huffman_decode",
            "data",
            PortValue::Bytes(vec![0; 65536].into()),
            json!({"weights":"A 1"}),
        ),
    ] {
        let cancel = CancellationToken::new();
        let out = GraphExecutor::run_node(
            &reg,
            id,
            &[(port.into(), input)].into(),
            &p,
            &Stop(cancel.clone()),
            &cancel,
        );
        assert!(matches!(out, Err(CoreError::Cancelled)), "{id}: {out:?}");
    }
}
