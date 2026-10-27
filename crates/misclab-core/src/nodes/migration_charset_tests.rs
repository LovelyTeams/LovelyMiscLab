use super::prelude::*;
use crate::{
    cancel::CancellationToken,
    graph::executor::GraphExecutor,
    progress::{NullSink, ProgressEvent, ProgressSink},
};
use serde_json::{json, Value};
fn call(id: &str, input: PortValue, p: &Value) -> Result<PortMap, CoreError> {
    GraphExecutor::run_node(
        &super::default_registry(),
        id,
        &one("data", input),
        p,
        &NullSink,
        &CancellationToken::new(),
    )
}
fn run(id: &str, s: &str, p: &Value) -> Result<PortMap, CoreError> {
    call(id, PortValue::Text(s.into()), p)
}
#[test]
fn pinned_charset_vectors_cover_nine_operations() {
    let f: Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/ebctf-charsets-batch12.json"
    ))
    .unwrap();
    assert_eq!(f["revision"], "ea2bc36dae31c97060f7f2cddad8df5518afad7b");
    let vectors = f["vectors"].as_array().unwrap();
    assert_eq!(f["referenceDecoder"]["package"], "text-encoding@0.7.0");
    assert_eq!(vectors.len(), 1003);
    assert_eq!(
        vectors
            .iter()
            .map(|v| v["operation"].as_str().unwrap())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        9
    );
    let reg = super::default_registry();
    let mut errors = Vec::new();
    for (i, v) in vectors.iter().enumerate() {
        let r = GraphExecutor::run_node(
            &reg,
            v["node"].as_str().unwrap(),
            &serde_json::from_value(v["input"].clone()).unwrap(),
            &v["params"],
            &NullSink,
            &CancellationToken::new(),
        );
        if v.get("error").is_some() {
            if r.is_ok() {
                errors.push(format!("{i}: expected error"));
            }
            continue;
        }
        match r {
            Err(e) => errors.push(format!("{i}: {e}")),
            Ok(out) => {
                // The independent 2018 polyfill consumes ASCII after an invalid
                // EUC-KR lead. The current WHATWG decoder restores that byte.
                let expected = if i == 189 {
                    assert_eq!(v["node"], "euc_kr_decode");
                    assert_eq!(v["input"]["data"]["value"], json!([255, 254, 65, 0]));
                    assert_eq!(v["text"], "\u{fffd}\u{fffd}\0");
                    "\u{fffd}\u{fffd}A\0"
                } else {
                    v["text"].as_str().unwrap()
                };
                if out["text"].as_text().unwrap() != expected {
                    errors.push(format!(
                        "{i} {} {:?} {:?}: got {:?}, expected {:?}",
                        v["node"],
                        v["input"],
                        v["params"],
                        out["text"].as_text(),
                        v["text"]
                    ));
                }
            }
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
#[test]
fn charset_all_bytes_remain_available_even_when_text_is_lossy() {
    let all: Vec<u8> = (0..=255).collect();
    for stem in [
        "gb_charset",
        "big5",
        "shift_jis",
        "euc_kr",
        "latin_charset",
        "ebcdic",
        "utf16",
    ] {
        for format in ["hex", "latin1", "auto"] {
            let encoded = if format == "latin1" {
                all.iter().map(|&b| b as char).collect()
            } else {
                hex::encode(&all)
            };
            let p = json!({"format":format});
            let direct = call(
                &format!("{stem}_decode"),
                PortValue::Bytes(all.clone().into()),
                &p,
            )
            .unwrap();
            let represented = run(&format!("{stem}_decode"), &encoded, &p).unwrap();
            assert_eq!(
                direct["text"].as_text().unwrap(),
                represented["text"].as_text().unwrap(),
                "{stem} {format}"
            );
            assert_eq!(
                direct["sourceBytes"].as_bytes().unwrap().as_ref(),
                all.as_slice()
            );
            assert_eq!(
                represented["sourceBytes"].as_bytes().unwrap().as_ref(),
                all.as_slice()
            );
        }
    }
}
#[test]
fn charset_standard_roundtrips_and_preserves_raw_channels() {
    for (stem, s, p) in [
        ("gb_charset", "中文", json!({})),
        ("gb_charset", "中文𠀀🙂", json!({"charset":"gb18030"})),
        ("big5", "中文繁體", json!({})),
        ("shift_jis", "日本語ｶﾀｶﾅ", json!({})),
        ("euc_kr", "한국어 한글", json!({})),
        ("latin_charset", "café €", json!({})),
        ("ebcdic", "Hello, World! []{}", json!({})),
        ("utf16", "\0中文🙂", json!({"endian":"LE"})),
        ("utf16", "\0中文🙂", json!({"endian":"BE","bom":false})),
    ] {
        let enc = run(&format!("{stem}_encode"), s, &p).unwrap();
        assert_eq!(
            enc["sourceBytes"].as_bytes().unwrap().as_ref(),
            s.as_bytes()
        );
        let bytes = enc["bytes"].clone();
        let dec = call(&format!("{stem}_decode"), bytes.clone(), &p).unwrap();
        assert_eq!(dec["text"].as_text().unwrap(), s);
        assert_eq!(dec["bytes"].as_bytes().unwrap().as_ref(), s.as_bytes());
        assert_eq!(
            dec["sourceBytes"].as_bytes().unwrap(),
            bytes.as_bytes().unwrap()
        );
        assert!(matches!(dec["lossy"], PortValue::Bool(false)));
    }
    assert_eq!(
        run("gb_charset_encode", "🙂", &json!({"charset":"gb18030"})).unwrap()["text"]
            .as_text()
            .unwrap(),
        "95308532"
    );
    assert!(run("gb_charset_encode", "🙂", &json!({})).is_err());
    let fallback = run("gb_charset_encode", "🙂", &json!({"codecMode":"upstream"})).unwrap();
    assert_eq!(fallback["text"].as_text().unwrap(), "f09f9982");
    assert!(matches!(fallback["lossy"], PortValue::Bool(true)));
    assert!(call(
        "gb_charset_encode",
        PortValue::Bytes(vec![0xff].into()),
        &json!({})
    )
    .is_err());
}
#[test]
fn charset_bom_invalid_sequences_and_aliases_are_explicit() {
    for (stem, p, bytes) in [
        ("gb_charset", json!({}), vec![0x81]),
        ("big5", json!({}), vec![0xff]),
        ("shift_jis", json!({}), vec![0x81]),
        ("euc_kr", json!({}), vec![0x81]),
        ("utf16", json!({}), vec![0xd8, 0x00]),
        ("ebcdic", json!({}), vec![0x00]),
    ] {
        let o = call(
            &format!("{stem}_decode"),
            PortValue::Bytes(bytes.clone().into()),
            &p,
        )
        .unwrap();
        assert!(matches!(o["lossy"], PortValue::Bool(true)), "{stem}");
        let mut strict = p;
        strict["strict"] = json!(true);
        assert!(call(
            &format!("{stem}_decode"),
            PortValue::Bytes(bytes.into()),
            &strict
        )
        .is_err());
    }
    assert_eq!(
        run(
            "latin_charset_decode",
            "80",
            &json!({"charset":"iso-8859-1"})
        )
        .unwrap()["text"]
            .as_text()
            .unwrap(),
        "€"
    );
    assert_eq!(
        run("utf16_decode", "fffe4100", &json!({"endian":"BE"})).unwrap()["text"]
            .as_text()
            .unwrap(),
        "A"
    );
    assert_eq!(
        run("utf16_decode", "fefffeff0041", &json!({})).unwrap()["text"]
            .as_text()
            .unwrap(),
        "A"
    );
    for (id, key) in [
        ("gb_charset_encode", "charset"),
        ("utf16_decode", "endian"),
        ("ebcdic_encode", "codepage"),
        ("big5_decode", "format"),
        ("mojibake_decode", "preset"),
    ] {
        assert!(run(id, "", &json!({key:"invalid"})).is_err());
    }
}
#[test]
fn quwei_and_mojibake_separate_reversible_data_from_losses() {
    for s in ["中文", "123456", "中1中", "5中", "A1234B中56国"] {
        let out = run("gb2312_quwei_encode", s, &json!({})).unwrap();
        assert_eq!(
            run(
                "gb2312_quwei_decode",
                out["text"].as_text().unwrap(),
                &json!({})
            )
            .unwrap()["text"]
                .as_text()
                .unwrap(),
            s
        );
    }
    assert!(run("gb2312_quwei_encode", "🙂", &json!({})).is_err());
    assert!(run("gb2312_quwei_decode", "0000", &json!({"strict":true})).is_err());
    assert_eq!(
        run("gb2312_quwei_decode", "0000", &json!({})).unwrap()["text"]
            .as_text()
            .unwrap(),
        "?"
    );
    let out = run("mojibake_encode", "café 中文", &json!({})).unwrap();
    assert_eq!(
        run(
            "mojibake_decode",
            out["text"].as_text().unwrap(),
            &json!({})
        )
        .unwrap()["text"]
            .as_text()
            .unwrap(),
        "café 中文"
    );
    let lossy = run("mojibake_encode", "中文", &json!({"preset":"gbk_as_utf8"})).unwrap();
    assert!(matches!(lossy["lossy"], PortValue::Bool(true)));
    assert!(run(
        "mojibake_encode",
        "中文",
        &json!({"preset":"gbk_as_utf8","strict":true})
    )
    .is_err());
}
#[test]
fn charset_limits_and_cancellation_apply_to_all_directions() {
    struct Stop(CancellationToken);
    impl ProgressSink for Stop {
        fn emit(&self, e: ProgressEvent) {
            if matches!(e, ProgressEvent::NodeProgress { .. }) {
                self.0.cancel();
            }
        }
    }
    let reg = super::default_registry();
    for stem in [
        "gb_charset",
        "big5",
        "shift_jis",
        "euc_kr",
        "latin_charset",
        "ebcdic",
        "utf16",
        "gb2312_quwei",
        "mojibake",
    ] {
        for dir in ["encode", "decode"] {
            let id = format!("{stem}_{dir}");
            let cancel = CancellationToken::new();
            assert!(matches!(
                GraphExecutor::run_node(
                    &reg,
                    &id,
                    &one("data", PortValue::Text(String::new())),
                    &json!({}),
                    &Stop(cancel.clone()),
                    &cancel
                ),
                Err(CoreError::Cancelled)
            ));
            assert!(run(&id, &"A".repeat(1_048_577), &json!({})).is_err());
        }
    }
}
