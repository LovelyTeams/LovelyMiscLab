use super::prelude::*;
use crate::{
    cancel::CancellationToken,
    graph::executor::GraphExecutor,
    progress::{NullSink, ProgressEvent, ProgressSink},
};
use serde_json::{json, Value};
fn call(id: &str, port: &str, value: PortValue, p: &Value) -> Result<PortMap, CoreError> {
    GraphExecutor::run_node(
        &super::default_registry(),
        id,
        &one(port, value),
        p,
        &NullSink,
        &CancellationToken::new(),
    )
}
fn run(id: &str, s: &str, p: &Value) -> Result<PortMap, CoreError> {
    call(id, "text", PortValue::Text(s.into()), p)
}
#[test]
fn pinned_byte_text_vectors_cover_six_operations() {
    let f: Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/ebctf-byte-text-batch13.json"
    ))
    .unwrap();
    assert_eq!(f["revision"], "ea2bc36dae31c97060f7f2cddad8df5518afad7b");
    let vectors = f["vectors"].as_array().unwrap();
    assert_eq!(vectors.len(), 389);
    assert_eq!(
        vectors
            .iter()
            .map(|v| v["operation"].as_str().unwrap())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        6
    );
    let registry = super::default_registry();
    let mut errors = Vec::new();
    for (i, v) in vectors.iter().enumerate() {
        let id = v["node"].as_str().unwrap();
        let result = GraphExecutor::run_node(
            &registry,
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
            Err(e) => errors.push(format!("{i} {id} {:?}: {e}", v["input"])),
            Ok(out) => {
                if let Some(expected) = v["text"].as_str() {
                    if out["text"].as_text().unwrap() != expected {
                        errors.push(format!(
                            "{i} {id} {:?} {:?}: got {:?}, expected {expected:?}",
                            v["input"],
                            v["params"],
                            out["text"].as_text().unwrap()
                        ));
                    }
                }
                if id == "js_escape_decode" {
                    let expected: Vec<u8> = serde_json::from_value(v["utf16le"].clone()).unwrap();
                    assert_eq!(
                        out["utf16le"].as_bytes().unwrap().as_ref(),
                        expected.as_slice(),
                        "{i}"
                    );
                }
                if let Some(b) = v.get("bytes") {
                    let expected: Vec<u8> = serde_json::from_value(b.clone()).unwrap();
                    assert_eq!(
                        out["bytes"].as_bytes().unwrap().as_ref(),
                        expected.as_slice(),
                        "{i}"
                    );
                }
            }
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
#[test]
fn byte_text_roundtrips_all_bytes_and_short_tails() {
    let all: Vec<u8> = (0..=255).collect();
    for (encode, decode, p) in [
        ("bubblebabble_encode", "bubblebabble_decode", json!({})),
        ("yenc_encode", "yenc_decode", json!({"width":1})),
        ("yenc_encode", "yenc_decode", json!({"width":128})),
        (
            "to_hexdump",
            "from_hexdump",
            json!({"style":"xxd","width":17}),
        ),
    ] {
        for n in [0, 1, 2, 3, 16, 31, 32, 33, 255, 256] {
            let encoded = call(
                encode,
                "data",
                PortValue::Bytes(all[..n].to_vec().into()),
                &p,
            )
            .unwrap();
            let decoded = run(decode, encoded["text"].as_text().unwrap(), &p).unwrap();
            assert_eq!(
                decoded["bytes"].as_bytes().unwrap().as_ref(),
                &all[..n],
                "{encode} {n}"
            );
        }
    }
    for n in 0..=50 {
        let b: Vec<u8> = (0..n).collect();
        let encoded = call(
            "bech32_encode",
            "data",
            PortValue::Bytes(b.clone().into()),
            &json!({"hrp":"x"}),
        )
        .unwrap();
        let decoded = run(
            "bech32_decode",
            encoded["text"].as_text().unwrap(),
            &json!({}),
        )
        .unwrap();
        assert_eq!(decoded["bytes"].as_bytes().unwrap().as_ref(), b.as_slice());
    }
}
#[test]
fn bech32_standard_checks_padding_and_checksum() {
    for s in ["A12UEL5L", "a12uel5l"] {
        assert_eq!(
            run("bech32_decode", s, &json!({})).unwrap()["bytes"]
                .as_bytes()
                .unwrap()
                .len(),
            0
        );
    }
    for s in ["A1G7SGD8", "a1lqfn3a", "A12uEL5L", "a1qqqqqq", "a12uel5l "] {
        assert!(run("bech32_decode", s, &json!({})).is_err(), "{s}");
    }
    let encoded = call(
        "bech32_encode",
        "data",
        PortValue::Bytes(vec![0x42; 20].into()),
        &json!({}),
    )
    .unwrap();
    let s = encoded["text"].as_text().unwrap();
    for i in 0..s.len() {
        let mut bytes = s.as_bytes().to_vec();
        bytes[i] = if bytes[i] == b'q' { b'p' } else { b'q' };
        assert!(
            run(
                "bech32_decode",
                std::str::from_utf8(&bytes).unwrap(),
                &json!({})
            )
            .is_err(),
            "{i}"
        );
    }
    for hrp in ["", "a b", "中文", &"x".repeat(84)] {
        assert!(call(
            "bech32_encode",
            "data",
            PortValue::Bytes(Vec::new().into()),
            &json!({"hrp":hrp})
        )
        .is_err());
    }
    assert!(call(
        "bech32_encode",
        "data",
        PortValue::Bytes(vec![0; 51].into()),
        &json!({"hrp":"xx"})
    )
    .is_err());
}
#[test]
fn escape_surrogates_and_fullwidth_keep_unicode_intact() {
    for s in ["", "\0 café 中文🙂", "@*_+-./", "\u{feff}ＡＢＣ"] {
        let e = run("js_escape_encode", s, &json!({})).unwrap();
        assert_eq!(
            run("js_escape_decode", e["text"].as_text().unwrap(), &json!({})).unwrap()["text"]
                .as_text()
                .unwrap(),
            s
        );
    }
    assert_eq!(
        run("js_escape_encode", "é", &json!({})).unwrap()["text"]
            .as_text()
            .unwrap(),
        "%u00E9"
    );
    assert!(run("js_escape_decode", "%uD800", &json!({})).is_err());
    let o = run("js_escape_decode", "%uD800", &json!({"strict":false})).unwrap();
    assert!(matches!(o["validUnicode"], PortValue::Bool(false)));
    assert_eq!(o["utf16le"].as_bytes().unwrap().as_ref(), &[0, 0xd8]);
    let ascii: String = (32u8..=126).map(|b| b as char).collect();
    let o = run("fullwidth_encode", &ascii, &json!({})).unwrap();
    let f = o["text"].as_text().unwrap();
    assert_eq!(
        run("fullwidth_decode", f, &json!({})).unwrap()["text"]
            .as_text()
            .unwrap(),
        ascii
    );
    assert_eq!(
        run("fullwidth_halfwidth", f, &json!({})).unwrap()["text"]
            .as_text()
            .unwrap(),
        ascii
    );
}
#[test]
fn hexdump_keeps_legacy_and_bounds_repeat_expansion() {
    let legacy = call(
        "to_hexdump",
        "data",
        PortValue::Text("Hello".into()),
        &json!({}),
    )
    .unwrap();
    assert_eq!(
        legacy["text"].as_text().unwrap(),
        "00000000  48 65 6c 6c 6f                                   Hello\n"
    );
    assert_eq!(
        run(
            "from_hexdump",
            legacy["text"].as_text().unwrap(),
            &json!({})
        )
        .unwrap()["bytes"]
            .as_bytes()
            .unwrap()
            .as_ref(),
        b"Hello"
    );
    let canonical="00000000  00 01 02 03 04 05 06 07  08 09 0a 0b 0c 0d 0e 0f  |................|\n*\n00000030";
    let o = run("from_hexdump", canonical, &json!({"style":"xxd"})).unwrap();
    assert_eq!(
        o["bytes"].as_bytes().unwrap().as_ref(),
        (0u8..16).cycle().take(48).collect::<Vec<_>>()
    );
    for dump in [
        "00000000: 4142  AB\n*\n00000007: 4344  CD",
        "00000000: 4142  AB\n*",
        "00000000: 414  junk",
    ] {
        assert!(run("from_hexdump", dump, &json!({"style":"xxd"})).is_err());
    }
    assert!(run(
        "from_hexdump",
        "00000000: 4142  AB\n*\nffffffffffffffff: 4344  CD",
        &json!({"style":"xxd","maxBytes":1024})
    )
    .is_err());
    assert!(run(
        "from_hexdump",
        canonical,
        &json!({"style":"xxd","maxBytes":47})
    )
    .is_err());
}
#[test]
fn byte_text_errors_and_cancellation_are_observable() {
    assert!(run("yenc_decode", "=", &json!({})).is_err());
    assert!(run("yenc_decode", "🙂", &json!({})).is_err());
    for s in ["xexex", "xexax-bexax", "xexab-bexax"] {
        assert!(run("bubblebabble_decode", s, &json!({})).is_err(), "{s}");
    }
    struct Stop(CancellationToken);
    impl ProgressSink for Stop {
        fn emit(&self, e: ProgressEvent) {
            if matches!(e, ProgressEvent::NodeProgress { .. }) {
                self.0.cancel();
            }
        }
    }
    let reg = super::default_registry();
    for (id, port, s, p) in [
        ("to_hexdump", "data", "Hello", json!({"style":"xxd"})),
        (
            "from_hexdump",
            "text",
            "00000000: 4142  AB",
            json!({"style":"xxd"}),
        ),
        ("yenc_encode", "data", "Hello", json!({})),
        ("yenc_decode", "text", "k", json!({})),
        ("bubblebabble_encode", "data", "Hello", json!({})),
        ("bubblebabble_decode", "text", "xexax", json!({})),
        ("js_escape_encode", "text", "Hello", json!({})),
        ("js_escape_decode", "text", "%u0041", json!({})),
        ("fullwidth_encode", "text", "Hello", json!({})),
        ("fullwidth_decode", "text", "Ａ", json!({})),
        ("bech32_encode", "data", "00", json!({})),
        ("bech32_decode", "text", "a12uel5l", json!({})),
    ] {
        let cancel = CancellationToken::new();
        assert!(
            matches!(
                GraphExecutor::run_node(
                    &reg,
                    id,
                    &one(port, PortValue::Text(s.into())),
                    &p,
                    &Stop(cancel.clone()),
                    &cancel
                ),
                Err(CoreError::Cancelled)
            ),
            "{id}"
        );
    }
}
