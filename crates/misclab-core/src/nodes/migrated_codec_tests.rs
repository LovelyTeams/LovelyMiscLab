//! Interoperability vectors captured from EBCTFCodeBox; see THIRD_PARTY_NOTICES.md.
use super::prelude::*;
use crate::{cancel::CancellationToken, graph::executor::GraphExecutor, progress::NullSink};
use serde_json::{json, Value};

fn run(id: &str, port: &str, input: PortValue, params: &Value) -> Result<PortMap, CoreError> {
    GraphExecutor::run_node(
        &super::default_registry(),
        id,
        &[(port.into(), input)].into(),
        params,
        &NullSink,
        &CancellationToken::new(),
    )
}

#[test]
fn upstream_known_vectors() {
    let vectors = [
        ("base36", "5pzcszu7"),
        ("base91", "TPwJh>A"),
        ("base100", "👟👜👣👣👦"),
        ("base58check", "2L5B5yqsVG8Vt"),
        ("radix64", "YETqZE6"),
        ("modhex", "hjhghrhrhv"),
        ("uuencode", "begin 644 -\n%:&5L;&\\ \n`\nend\n"),
        ("xxencode", "begin 644 -\n3O4JgP4w+\n+\nend\n"),
        ("crockford32", "D1JPRV3F"),
    ];
    for (id, expected) in vectors {
        let encoded = run(
            &format!("{id}_encode"),
            "data",
            PortValue::Text("hello".into()),
            &json!({}),
        )
        .unwrap();
        assert_eq!(encoded["text"].as_text().unwrap(), expected, "{id}");
        let decoded = run(
            &format!("{id}_decode"),
            "text",
            PortValue::Text(expected.into()),
            &json!({}),
        )
        .unwrap();
        assert_eq!(decoded["text"].as_text().unwrap(), "hello", "{id}");
    }
    let encoded = run(
        "citrix_ctx1_encode",
        "text",
        PortValue::Text("hello".into()),
        &json!({}),
    )
    .unwrap();
    assert_eq!(encoded["text"].as_text().unwrap(), "MNGIKIANMEGBKIANMHGC");
}

#[test]
fn arbitrary_bytes_and_boundaries_roundtrip() {
    let all: Vec<u8> = (0..=255).collect();
    for id in [
        "base36",
        "base91",
        "base100",
        "base58check",
        "base_custom",
        "radix64",
        "modhex",
        "crockford32",
        "uuencode",
        "xxencode",
    ] {
        for length in [0, 1, 2, 3, 7, 44, 45, 46, 255, 256] {
            let original = all[..length].to_vec();
            let encoded = run(
                &format!("{id}_encode"),
                "data",
                PortValue::Bytes(original.clone().into()),
                &json!({}),
            )
            .unwrap();
            let decoded = run(
                &format!("{id}_decode"),
                "text",
                encoded["text"].clone(),
                &json!({}),
            )
            .unwrap();
            assert!(
                matches!(&decoded["bytes"], PortValue::Bytes(bytes) if bytes.as_ref() == original),
                "{id} length {length}"
            );
        }
    }
}

#[test]
fn custom_alphabet_and_checksums() {
    let p = json!({"alphabet":"零一二三"});
    let encoded = run(
        "base_custom_encode",
        "data",
        PortValue::Bytes(vec![0, 0, 255].into()),
        &p,
    )
    .unwrap();
    let decoded = run("base_custom_decode", "text", encoded["text"].clone(), &p).unwrap();
    assert!(matches!(&decoded["bytes"], PortValue::Bytes(b) if b.as_ref() == [0, 0, 255]));
    assert!(run(
        "base_custom_encode",
        "data",
        PortValue::Text("x".into()),
        &json!({"alphabet":"aa"})
    )
    .is_err());
    assert!(run(
        "base91_encode",
        "data",
        PortValue::Text("x".into()),
        &json!({"alphabet":"abc"})
    )
    .is_err());
    let checked = run(
        "crockford32_encode",
        "data",
        PortValue::Text("hello".into()),
        &json!({"check":true,"dashGroup":4}),
    )
    .unwrap();
    assert_eq!(checked["text"].as_text().unwrap(), "D1JP-RV3F-J");
    assert!(run(
        "crockford32_decode",
        "text",
        PortValue::Text("D1JP-RV3F-K".into()),
        &json!({"check":true})
    )
    .is_err());
    assert!(run(
        "base58check_decode",
        "text",
        PortValue::Text("2L5B5yqsVG8Vu".into()),
        &json!({})
    )
    .is_err());
}

#[test]
fn rejects_invalid_or_truncated_streams() {
    for (id, input) in [
        ("base36", "!"),
        ("base100", "hello"),
        ("modhex", "c"),
        ("radix64", "A"),
        ("crockford32", "1"),
        ("uuencode", "%:&5"),
        ("xxencode", "3O4"),
    ] {
        assert!(
            run(
                &format!("{id}_decode"),
                "text",
                PortValue::Text(input.into()),
                &json!({})
            )
            .is_err(),
            "{id}"
        );
    }
}

#[test]
fn bare_transports_and_rot_vectors() {
    for id in ["uuencode", "xxencode"] {
        let encoded = run(
            &format!("{id}_encode"),
            "data",
            PortValue::Text("中文\0!".into()),
            &json!({"bare":true}),
        )
        .unwrap();
        assert!(!encoded["text"].as_text().unwrap().starts_with("begin"));
        let decoded = run(
            &format!("{id}_decode"),
            "text",
            encoded["text"].clone(),
            &json!({}),
        )
        .unwrap();
        assert_eq!(decoded["text"].as_text().unwrap(), "中文\0!");
    }
    for (id, expected) in [("rot5", "Abc-56789中文"), ("rot18", "Nop-56789中文")] {
        let encoded = run(
            id,
            "text",
            PortValue::Text("Abc-01234中文".into()),
            &json!({}),
        )
        .unwrap();
        assert_eq!(encoded["text"].as_text().unwrap(), expected);
        let decoded = run(id, "text", encoded["text"].clone(), &json!({})).unwrap();
        assert_eq!(decoded["text"].as_text().unwrap(), "Abc-01234中文");
    }
}

#[test]
fn custom_base36_and_modhex_delimiters() {
    let params = json!({"alphabet":"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"});
    let encoded = run(
        "base36_encode",
        "data",
        PortValue::Text("hello".into()),
        &params,
    )
    .unwrap();
    assert_eq!(encoded["text"].as_text().unwrap(), "5PZCSZU7");
    let decoded = run("base36_decode", "text", encoded["text"].clone(), &params).unwrap();
    assert_eq!(decoded["text"].as_text().unwrap(), "hello");
    for (delim, expected) in [
        ("colon", "hj:hg:hr:hr:hv"),
        ("space", "hj hg hr hr hv"),
        ("comma", "hj,hg,hr,hr,hv"),
    ] {
        let encoded = run(
            "modhex_encode",
            "data",
            PortValue::Text("hello".into()),
            &json!({"delim":delim}),
        )
        .unwrap();
        assert_eq!(encoded["text"].as_text().unwrap(), expected);
        let decoded = run("modhex_decode", "text", encoded["text"].clone(), &json!({})).unwrap();
        assert_eq!(decoded["text"].as_text().unwrap(), "hello");
    }
    let decoded = run(
        "modhex_decode",
        "text",
        PortValue::Text("HJ:HG;HR,HR.HV-_%%|+".into()),
        &json!({}),
    )
    .unwrap();
    assert_eq!(decoded["text"].as_text().unwrap(), "hello");
}
