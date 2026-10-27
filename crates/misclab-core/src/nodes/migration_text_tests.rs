use super::prelude::*;
use crate::{
    cancel::CancellationToken,
    graph::executor::GraphExecutor,
    progress::{NullSink, ProgressEvent, ProgressSink},
};
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
fn text(id: &str, input: &str, params: &Value) -> Result<PortMap, CoreError> {
    run(id, &one("text", PortValue::Text(input.into())), params)
}
#[test]
fn pinned_text_vectors_cover_ten_operations_and_all_directions() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/ebctf-text-batch4.json")).unwrap();
    assert_eq!(
        fixture["revision"],
        "ea2bc36dae31c97060f7f2cddad8df5518afad7b"
    );
    let vectors = fixture["vectors"].as_array().unwrap();
    assert_eq!(vectors.len(), 214);
    assert_eq!(
        vectors
            .iter()
            .map(|v| v["operation"].as_str().unwrap())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        10
    );
    for (i, v) in vectors.iter().enumerate() {
        let id = v["node"].as_str().unwrap();
        let result = run(
            id,
            &serde_json::from_value(v["input"].clone()).unwrap(),
            &v["params"],
        );
        if v.get("error").is_some() {
            assert!(result.is_err(), "vector {i} {id}");
            continue;
        }
        let output = result.unwrap_or_else(|e| panic!("vector {i} {id}: {e}"));
        if let Some(expected) = v.get("text") {
            assert_eq!(
                output["text"].as_text().unwrap(),
                expected.as_str().unwrap(),
                "vector {i} {id}"
            );
        }
        if let Some(expected) = v.get("bytes") {
            let bytes: Vec<u8> = serde_json::from_value(expected.clone()).unwrap();
            assert_eq!(
                &*output["bytes"].as_bytes().unwrap(),
                bytes,
                "vector {i} {id}"
            );
        }
    }
}
#[test]
fn binary_text_codecs_preserve_every_byte_and_legacy_port_names() {
    let data: Vec<u8> = (0..=255).collect();
    for length in [0, 1, 2, 3, 4, 255, 256] {
        for (id, port, encode, decode) in [
            (
                "url",
                "text",
                json!({"mode":"full"}),
                json!({"utf8":false,"strict":true}),
            ),
            (
                "url",
                "text",
                json!({"mode":"standard"}),
                json!({"utf8":false,"strict":true}),
            ),
            (
                "url",
                "text",
                json!({"mode":"plus"}),
                json!({"plus":true,"utf8":false,"strict":true}),
            ),
            (
                "quoted_printable",
                "data",
                json!({"mode":"原始字节"}),
                json!({"strict":true}),
            ),
            (
                "js_hex",
                "data",
                json!({"mode":"nonascii"}),
                json!({"strict":true}),
            ),
            (
                "js_hex",
                "data",
                json!({"mode":"all"}),
                json!({"strict":true}),
            ),
        ] {
            let enc = run(
                &format!("{id}_encode"),
                &one(port, PortValue::Bytes(data[..length].to_vec().into())),
                &encode,
            )
            .unwrap();
            let dec = text(
                &format!("{id}_decode"),
                enc["text"].as_text().unwrap(),
                &decode,
            )
            .unwrap();
            assert_eq!(
                &*dec["bytes"].as_bytes().unwrap(),
                &data[..length],
                "{id} length {length}"
            );
        }
    }
}
#[test]
fn legacy_text_defaults_remain_compatible_and_new_modes_are_explicit() {
    for (id, input, expected) in [
        ("url_encode", " !*'()", "%20%21%2A%27%28%29"),
        ("url_decode", " a+b%20 ", " a+b  "),
        ("html_entity_encode", "'中文", "&#39;中文"),
        ("unicode_escape", "a中🙂", "a\\u4e2d\\ud83d\\ude42"),
        ("leetspeak", "#|)83457", "#|)beast"),
        ("nato_phonetic", "Juliett Xray unknown", "JX"),
    ] {
        assert_eq!(
            text(id, input, &json!({})).unwrap()["text"]
                .as_text()
                .unwrap(),
            expected,
            "{id}"
        );
    }
    let qp = run(
        "quoted_printable_encode",
        &one("data", PortValue::Text("a\rb\nc \n".into())),
        &json!({}),
    )
    .unwrap();
    assert_eq!(qp["text"].as_text().unwrap(), "ab\r\nc=20\r\n");
    assert_eq!(
        text("js_hex_decode", "中文🙂", &json!({"literal":"UTF-8"})).unwrap()["text"]
            .as_text()
            .unwrap(),
        "中文🙂"
    );
    assert_eq!(
        text("html_entity_decode", "&#X41; &#0;", &json!({})).unwrap()["text"]
            .as_text()
            .unwrap(),
        "A \0"
    );
}
#[test]
fn malformed_text_and_unicode_sequences_are_bounded_and_never_panic() {
    for (id, input, p) in [
        ("url_decode", "%中文", json!({"strict":true})),
        ("url_decode", "%FF", json!({})),
        ("unicode_unescape", "\\uD83D", json!({})),
        ("unicode_unescape", "\\uDE42", json!({})),
        ("unicode_unescape", "\\u{110000}", json!({})),
        ("unicode_unescape", "\\uD83D\\u0041", json!({})),
        ("utf7_decode", "+A-", json!({})),
        ("utf7_decode", "+AAAA-", json!({})),
        ("utf7_decode", "+2AA-", json!({})),
        ("utf7_decode", "+Ti1-", json!({})),
        ("punycode_decode", "xn--!", json!({})),
        (
            "punycode_decode",
            "xn--zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
            json!({}),
        ),
        ("quoted_printable_decode", "=中", json!({"strict":true})),
        ("quoted_printable_decode", "=0", json!({"strict":true})),
        ("js_hex_decode", "\\x中", json!({"strict":true})),
    ] {
        assert!(text(id, input, &p).is_err(), "{id} {input}");
    }
    assert_eq!(
        text("quoted_printable_decode", "=中文🙂", &json!({})).unwrap()["text"]
            .as_text()
            .unwrap(),
        "=中文🙂"
    );
    assert_eq!(
        text(
            "unicode_unescape",
            "\\u0041\\uD83D\\uDE42\\u0042",
            &json!({})
        )
        .unwrap()["text"]
            .as_text()
            .unwrap(),
        "A🙂B"
    );
    assert!(text("punycode_encode", &"中".repeat(4097), &json!({})).is_err());
    assert_eq!(
        text("html_entity_decode", &"&".repeat(10000), &json!({})).unwrap()["text"]
            .as_text()
            .unwrap()
            .len(),
        10000
    );
}
#[test]
fn mixed_radix_outputs_are_connectable_and_catalog_searchable() {
    let out = text(
        "mixed_radix_decode",
        "0x480o151;0b100001 0x41zz 0b2",
        &json!({}),
    )
    .unwrap();
    assert_eq!(out["decoded"].as_text().unwrap(), "Hi!");
    assert!(matches!(&out["rows"], PortValue::Json(rows) if rows.as_array().unwrap().len()==3));
    assert!(matches!(&out["errors"], PortValue::StringList(errors) if errors==&["0x41zz", "0b2"]));
    let encoded = run(
        "base64_encode",
        &one("text", out["decoded"].clone()),
        &json!({}),
    )
    .unwrap();
    assert_eq!(encoded["text"].as_text().unwrap(), "SGkh");
    let registry = super::default_registry();
    for (id, alias, category) in [
        ("utf7_encode", "utf7", "encoding.charset"),
        ("mixed_radix_decode", "mixHexOctBin", "numeric.radix"),
        ("leet_speak_decode", "leetSpeak", "encoding.fancy"),
        ("nato_alphabet_encode", "natoAlphabet", "encoding.fancy"),
    ] {
        let d = registry
            .descriptors()
            .into_iter()
            .find(|d| d.id == id)
            .unwrap();
        assert_eq!(d.subcategory_id, category);
        assert_eq!(crate::node::catalog::search_rank(&d, alias), Some(0));
        assert!(!d.examples.is_empty());
        assert!(d.params.iter().all(|p| !p.description.is_empty()));
    }
}
#[test]
fn punycode_cancels_during_both_directions() {
    struct Stop(CancellationToken);
    impl ProgressSink for Stop {
        fn emit(&self, event: ProgressEvent) {
            if matches!(event, ProgressEvent::NodeProgress {pct, ..} if pct>0.0 && pct<1.0) {
                self.0.cancel();
            }
        }
    }
    let data: String = (0x4e00..0x5000).filter_map(char::from_u32).collect();
    let encoded = text("punycode_encode", &data, &json!({})).unwrap();
    for (id, input) in [
        ("punycode_encode", data.as_str()),
        ("punycode_decode", encoded["text"].as_text().unwrap()),
    ] {
        let cancel = CancellationToken::new();
        let sink = Stop(cancel.clone());
        let out = GraphExecutor::run_node(
            &super::default_registry(),
            id,
            &one("text", PortValue::Text(input.into())),
            &json!({}),
            &sink,
            &cancel,
        );
        assert!(matches!(out, Err(CoreError::Cancelled)), "{id}");
    }
}
