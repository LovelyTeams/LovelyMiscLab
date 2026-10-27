use super::prelude::*;
use crate::{
    cancel::CancellationToken,
    graph::executor::GraphExecutor,
    progress::{NullSink, ProgressEvent, ProgressSink},
};
use serde_json::{json, Value};
fn run(reg: &NodeRegistry, id: &str, i: &PortMap, p: &Value) -> Result<PortMap, CoreError> {
    GraphExecutor::run_node(reg, id, i, p, &NullSink, &CancellationToken::new())
}
#[test]
fn pinned_unicode_base_vectors_cover_seven_operations() {
    let f: Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/ebctf-unicode-base-batch7.json"
    ))
    .unwrap();
    assert_eq!(f["revision"], "ea2bc36dae31c97060f7f2cddad8df5518afad7b");
    let v = f["vectors"].as_array().unwrap();
    assert_eq!(v.len(), 276);
    assert_eq!(
        v.iter()
            .map(|v| v["operation"].as_str().unwrap())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        7
    );
    let reg = super::default_registry();
    for (i, v) in v.iter().enumerate() {
        let id = v["node"].as_str().unwrap();
        let o = run(
            &reg,
            id,
            &serde_json::from_value(v["input"].clone()).unwrap(),
            &v["params"],
        );
        if v.get("error").is_some() {
            assert!(o.is_err(), "{i} {id}");
            continue;
        }
        let o = o.unwrap_or_else(|e| panic!("{i} {id}: {e}"));
        if let Some(t) = v.get("text") {
            assert_eq!(
                o["text"].as_text().unwrap(),
                t.as_str().unwrap(),
                "{i} {id}"
            );
        }
        if let Some(b) = v.get("bytes") {
            let b: Vec<u8> = serde_json::from_value(b.clone()).unwrap();
            assert_eq!(&*o["bytes"].as_bytes().unwrap(), b, "{i} {id}");
        }
    }
}
#[test]
fn unicode_bases_and_padding_stego_preserve_all_bytes_and_tail_sizes() {
    let reg = super::default_registry();
    let data: Vec<u8> = (0..=255).collect();
    for stem in [
        "radix_n",
        "base69",
        "base2048",
        "base65536",
        "ecoji",
        "base32_padding_stego",
        "base64_padding_stego",
    ] {
        for len in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 16, 255, 256] {
            let e = run(
                &reg,
                &format!("{stem}_encode"),
                &one("data", PortValue::Bytes(data[..len].to_vec().into())),
                &json!({}),
            )
            .unwrap();
            let d = run(
                &reg,
                &format!("{stem}_decode"),
                &one("text", e["text"].clone()),
                &json!({}),
            )
            .unwrap();
            assert_eq!(
                &*d["bytes"].as_bytes().unwrap(),
                &data[..len],
                "{stem} {len}"
            );
        }
    }
}
#[test]
fn base65536_covers_all_blocks_and_unicode_tables_are_unique() {
    let reg = super::default_registry();
    let b: Vec<u8> = (0u16..=65535).flat_map(u16::to_le_bytes).collect();
    let e = run(
        &reg,
        "base65536_encode",
        &one("data", PortValue::Bytes(b.clone().into())),
        &json!({}),
    )
    .unwrap();
    assert_eq!(
        e["text"]
            .as_text()
            .unwrap()
            .chars()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        65536
    );
    let d = run(
        &reg,
        "base65536_decode",
        &one("text", e["text"].clone()),
        &json!({}),
    )
    .unwrap();
    assert_eq!(&*d["bytes"].as_bytes().unwrap(), b);
    let t: Value = serde_json::from_str(include_str!("unicode_base_tables.json")).unwrap();
    for (key, len) in [("B2048_11", 2048), ("ECO_MAP", 1024)] {
        let a = t[key].as_array().unwrap();
        assert_eq!(a.len(), len);
        assert_eq!(
            a.iter()
                .map(|n| n.as_u64().unwrap())
                .collect::<std::collections::HashSet<_>>()
                .len(),
            len
        );
    }
}
#[test]
fn bad_unicode_padding_and_ambiguous_alphabets_are_rejected() {
    let reg = super::default_registry();
    for (id, s, p) in [
        ("ecoji_decode", "ABCD", json!({})),
        ("ecoji_decode", "☕", json!({})),
        ("base2048_decode", "🙂", json!({})),
        ("base2048_decode", "7", json!({})),
        ("base65536_decode", "A", json!({})),
        ("base69_decode", "||||||||||||||||", json!({})),
        ("base69_decode", "AAAAAAAAAAAA7=", json!({})),
        ("base69_encode", "A", json!({"dict":"A".repeat(69)})),
        ("radix_n_encode", "A", json!({"radix":95})),
        ("radix_n_encode", "A", json!({"radix":2,"dict":"aa"})),
        ("base32_padding_stego_decode", "O!2E====", json!({})),
        ("base64_padding_stego_decode", "????==", json!({})),
    ] {
        let port = if id.ends_with("encode") {
            "data"
        } else {
            "text"
        };
        assert!(
            run(&reg, id, &one(port, PortValue::Text(s.into())), &p).is_err(),
            "{id} {s}"
        );
    }
    let d = run(
        &reg,
        "base64_padding_stego_decode",
        &one("text", PortValue::Text("cw==".into())),
        &json!({}),
    )
    .unwrap();
    assert!(d["bytes"].as_bytes().unwrap().is_empty());
    assert_eq!(d["remainingBits"].as_text().unwrap(), "0000");
    let e = run(
        &reg,
        "base65536_encode",
        &one("data", PortValue::Bytes(vec![65].into())),
        &json!({}),
    )
    .unwrap();
    let s = e["text"].as_text().unwrap().repeat(2);
    assert!(run(
        &reg,
        "base65536_decode",
        &one("text", PortValue::Text(s)),
        &json!({})
    )
    .is_err());
}
#[test]
fn unicode_and_stego_large_encodes_and_decodes_cancel() {
    struct Stop(CancellationToken);
    impl ProgressSink for Stop {
        fn emit(&self, e: ProgressEvent) {
            if matches!(e,ProgressEvent::NodeProgress{pct,..}if pct>0.0&&pct<1.0) {
                self.0.cancel();
            }
        }
    }
    let reg = super::default_registry();
    let data = vec![42u8; 50000];
    for stem in [
        "base69",
        "base2048",
        "base65536",
        "ecoji",
        "base32_padding_stego",
        "base64_padding_stego",
    ] {
        let e = run(
            &reg,
            &format!("{stem}_encode"),
            &one("data", PortValue::Bytes(data.clone().into())),
            &json!({}),
        )
        .unwrap();
        for (dir, i) in [
            ("encode", one("data", PortValue::Bytes(data.clone().into()))),
            ("decode", one("text", e["text"].clone())),
        ] {
            let cancel = CancellationToken::new();
            let o = GraphExecutor::run_node(
                &reg,
                &format!("{stem}_{dir}"),
                &i,
                &json!({}),
                &Stop(cancel.clone()),
                &cancel,
            );
            assert!(matches!(o, Err(CoreError::Cancelled)), "{stem}_{dir}");
        }
    }
}
