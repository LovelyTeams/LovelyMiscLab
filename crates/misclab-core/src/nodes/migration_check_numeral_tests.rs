use super::prelude::*;
use crate::{
    cancel::CancellationToken,
    graph::executor::GraphExecutor,
    progress::{NullSink, ProgressEvent, ProgressSink},
};
use serde_json::{json, Value};
fn run(id: &str, s: &str, p: &Value) -> Result<PortMap, CoreError> {
    GraphExecutor::run_node(
        &super::default_registry(),
        id,
        &one("text", PortValue::Text(s.into())),
        p,
        &NullSink,
        &CancellationToken::new(),
    )
}
#[test]
fn pinned_check_digit_and_numeral_vectors_cover_eleven_operations() {
    let f: Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/ebctf-check-numerals-batch11.json"
    ))
    .unwrap();
    assert_eq!(f["revision"], "ea2bc36dae31c97060f7f2cddad8df5518afad7b");
    let vs = f["vectors"].as_array().unwrap();
    assert_eq!(vs.len(), 749);
    assert_eq!(
        vs.iter()
            .map(|v| v["operation"].as_str().unwrap())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        11
    );
    let registry = super::default_registry();
    let mut errors = Vec::new();
    for (i, v) in vs.iter().enumerate() {
        let id = v["node"].as_str().unwrap();
        let r = GraphExecutor::run_node(
            &registry,
            id,
            &serde_json::from_value(v["input"].clone()).unwrap(),
            &v["params"],
            &NullSink,
            &CancellationToken::new(),
        );
        if v.get("error").is_some() {
            if r.is_ok() {
                errors.push(format!("{i} {id}: expected error"));
            }
            continue;
        }
        match r {
            Err(e) => errors.push(format!("{i} {id}: {e}")),
            Ok(o) => {
                if o["text"].as_text().unwrap() != v["text"].as_str().unwrap() {
                    errors.push(format!(
                        "{i} {id} input {:?}: actual {:?}, expected {:?}",
                        v["input"],
                        o["text"].as_text().unwrap(),
                        v["text"]
                    ));
                }
            }
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
#[test]
fn check_digit_nodes_expose_boolean_and_corrected_values() {
    for (stem, body, expected) in [
        ("luhn", "7992739871", "3"),
        ("isbn", "030640615", "2"),
        ("isbn", "097522980", "X"),
        ("isbn", "978030640615", "7"),
        ("ean13", "400638133393", "1"),
        ("upc_a", "03600029145", "2"),
        ("cnid_check", "00000000000000000", "1"),
    ] {
        let out = run(&format!("{stem}_calculate"), body, &json!({})).unwrap();
        assert_eq!(out["check"].as_text().unwrap(), expected);
        assert_eq!(out["full"].as_text().unwrap(), format!("{body}{expected}"));
        for suffix in "0123456789X".chars() {
            let r = run(
                &format!("{stem}_validate"),
                &format!("{body}{suffix}"),
                &json!({}),
            );
            // Luhn accepts arbitrary-length digit strings and upstream removes
            // non-digits, so a trailing X is removed rather than rejected.
            if suffix == 'X' && stem == "luhn" {
                assert!(r.is_ok());
                continue;
            }
            if suffix == 'X' && !matches!(stem, "isbn" | "cnid_check") {
                assert!(r.is_err());
                continue;
            }
            if suffix == 'X' && body.len() == 12 {
                assert!(r.is_err());
                continue;
            }
            let o = r.unwrap();
            assert!(matches!(&o["valid"],PortValue::Bool(b)if *b==(suffix.to_string()==expected)));
            assert_eq!(o["check"].as_text().unwrap(), expected);
        }
    }
    let tables: Value = serde_json::from_str(include_str!("checkdigit_tables.json")).unwrap();
    assert_eq!(tables["iban"].as_object().unwrap().len(), 88);
    assert_eq!(tables["bank"].as_array().unwrap().len(), 21);
    let o = run("iban_calculate", "GBWEST12345698765432", &json!({})).unwrap();
    assert_eq!(o["text"].as_text().unwrap(), "GB82WEST12345698765432");
    let o = run("bank_bin_identify", "621700", &json!({})).unwrap();
    assert!(
        matches!(&o["data"],PortValue::Json(v)if v["matchedPrefix"]=="621700"&&v["issuer"]=="中国建设银行"&&v["referenceOnly"]==true)
    );
    let o = run("bank_bin_identify", "999999", &json!({})).unwrap();
    assert!(
        matches!(&o["data"],PortValue::Json(v)if v["brand"]=="未知"&&v["matchedPrefix"].is_null())
    );
}
#[test]
fn historic_numerals_have_exact_positional_values_and_unicode_symbols() {
    for (mode, decimal, expected) in [
        ("vigesimal", "20", ". 0"),
        ("vigesimal", "400", ". 0 0"),
        ("longcount", "360", ". 0 0"),
        ("longcount", "7200", ". 0 0 0"),
    ] {
        assert_eq!(
            run(
                "maya_numerals_encode",
                decimal,
                &json!({"mode":mode,"form":"dotbar"})
            )
            .unwrap()["text"]
                .as_text()
                .unwrap(),
            expected
        );
    }
    for n in 0..60 {
        let s = n.to_string();
        for (stem, p) in [
            ("maya_numerals", json!({})),
            ("maya_numerals", json!({"form":"dotbar","mode":"longcount"})),
            ("babylonian_numerals", json!({})),
            ("babylonian_numerals", json!({"form":"ascii"})),
        ] {
            let e = run(&format!("{stem}_encode"), &s, &p).unwrap();
            let d = run(&format!("{stem}_decode"), e["text"].as_text().unwrap(), &p).unwrap();
            assert_eq!(d["text"].as_text().unwrap(), s);
        }
    }
    for (s, cp) in [
        ("1", 0x133fa),
        ("10", 0x13386),
        ("100", 0x13362),
        ("1000", 0x131bc),
        ("10000", 0x130ad),
        ("100000", 0x13190),
        ("1000000", 0x13068),
    ] {
        assert_eq!(
            run("egyptian_numerals_encode", s, &json!({})).unwrap()["text"]
                .as_text()
                .unwrap(),
            char::from_u32(cp).unwrap().to_string()
        );
    }
    for n in [
        359, 360, 361, 399, 400, 401, 7199, 7200, 7201, 143999, 144000,
    ] {
        let e = run(
            "maya_numerals_encode",
            &n.to_string(),
            &json!({"mode":"longcount"}),
        )
        .unwrap();
        assert_eq!(
            run(
                "maya_numerals_decode",
                e["text"].as_text().unwrap(),
                &json!({"mode":"longcount"})
            )
            .unwrap()["text"]
                .as_text()
                .unwrap(),
            n.to_string()
        );
    }
    for (stem, p) in [
        ("maya_numerals", json!({})),
        ("babylonian_numerals", json!({})),
    ] {
        let s = "1234567890".repeat(30);
        let e = run(&format!("{stem}_encode"), &s, &p).unwrap();
        assert_eq!(
            run(&format!("{stem}_decode"), e["text"].as_text().unwrap(), &p).unwrap()["text"]
                .as_text()
                .unwrap(),
            s
        );
    }
}
#[test]
fn pips_cover_all_domino_pairs_and_preserve_separators() {
    for orientation in ["horizontal", "vertical"] {
        let base = if orientation == "horizontal" {
            0x1f031
        } else {
            0x1f063
        };
        let p = json!({"system":"domino","orientation":orientation});
        for a in 0..7 {
            for b in 0..7 {
                let s = format!("{a}{b}");
                let e = run("pip_numerals_encode", &s, &p).unwrap();
                let expected = char::from_u32(base + 7 * a + b).unwrap().to_string();
                assert_eq!(e["text"].as_text().unwrap(), expected);
                assert_eq!(
                    run("pip_numerals_decode", &expected, &p).unwrap()["text"]
                        .as_text()
                        .unwrap(),
                    s
                );
            }
        }
        let s = "00 12/34|56,\n(66)\t";
        let e = run("pip_numerals_encode", s, &p).unwrap();
        assert_eq!(
            run("pip_numerals_decode", e["text"].as_text().unwrap(), &p).unwrap()["text"]
                .as_text()
                .unwrap(),
            s
        );
    }
    for (id, s, p) in [
        (
            "maya_numerals_decode",
            "...--- 0",
            json!({"mode":"longcount","form":"dotbar"}),
        ),
        ("maya_numerals_encode", "01", json!({})),
        ("babylonian_numerals_decode", "0 |", json!({"form":"ascii"})),
        ("egyptian_numerals_encode", "10000000000", json!({})),
        ("pip_numerals_encode", "123", json!({"system":"domino"})),
        ("pip_numerals_decode", "🎲", json!({})),
        ("pip_numerals_decode", "🀰", json!({"system":"domino"})),
    ] {
        assert!(run(id, s, &p).is_err(), "{id} {s}");
    }
}
#[test]
fn historic_numeral_work_cancels_in_both_directions() {
    struct Stop(CancellationToken);
    impl ProgressSink for Stop {
        fn emit(&self, e: ProgressEvent) {
            if matches!(e,ProgressEvent::NodeProgress{pct,..}if pct>0.0&&pct<1.0) {
                self.0.cancel();
            }
        }
    }
    for (id, s) in [
        ("maya_numerals_encode", "9".repeat(1000)),
        ("babylonian_numerals_encode", "9".repeat(1000)),
        ("maya_numerals_decode", "𝋡 ".repeat(1000)),
        ("babylonian_numerals_decode", "𒐕 ".repeat(1000)),
        ("egyptian_numerals_encode", "9999999999".into()),
        ("egyptian_numerals_decode", "𓏺".repeat(1000)),
        ("pip_numerals_encode", "123456".repeat(1000)),
        ("pip_numerals_decode", "⚀⚁⚂⚃⚄⚅".repeat(1000)),
    ] {
        let cancel = CancellationToken::new();
        let out = GraphExecutor::run_node(
            &super::default_registry(),
            id,
            &one("text", PortValue::Text(s)),
            &json!({}),
            &Stop(cancel.clone()),
            &cancel,
        );
        assert!(matches!(out, Err(CoreError::Cancelled)), "{id}: {out:?}");
    }
}
