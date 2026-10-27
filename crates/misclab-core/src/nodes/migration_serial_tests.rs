use super::prelude::*;
use crate::{
    cancel::CancellationToken,
    graph::executor::GraphExecutor,
    progress::{NullSink, ProgressEvent, ProgressSink},
};
use serde_json::{json, Value};
fn run(id: &str, bytes: &[u8], p: &Value) -> Result<PortMap, CoreError> {
    GraphExecutor::run_node(
        &super::default_registry(),
        id,
        &one("data", PortValue::Bytes(bytes.into())),
        p,
        &NullSink,
        &CancellationToken::new(),
    )
}
fn accepted(o: &PortMap) -> bool {
    matches!(&o["complete"], PortValue::Bool(true))
}
#[test]
fn pinned_serial_vectors_cover_three_operations_and_all_input_modes() {
    let f: Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/ebctf-serial-batch9.json"
    ))
    .unwrap();
    assert_eq!(f["revision"], "ea2bc36dae31c97060f7f2cddad8df5518afad7b");
    let vs = f["vectors"].as_array().unwrap();
    assert_eq!(vs.len(), 500);
    let reg = super::default_registry();
    let mut errors = Vec::new();
    for (i, v) in vs.iter().enumerate() {
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
                errors.push(format!("{i} {id} expected error"));
            }
            continue;
        }
        match result {
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
fn containers_keep_exact_integers_duplicate_keys_binary_and_remainder() {
    let o = run(
        "msgpack_parse",
        &hex::decode("82a16ecfffffffffffffffffa16ed38000000000000000ff").unwrap(),
        &json!({}),
    )
    .unwrap();
    assert!(!accepted(&o));
    assert_eq!(o["remainder"].as_bytes().unwrap().as_ref(), &[255]);
    let PortValue::Json(tree) = &o["tree"] else {
        panic!()
    };
    assert_eq!(tree["value"][0]["key"]["value"], "n");
    assert_eq!(tree["value"][1]["key"]["value"], "n");
    assert_eq!(tree["value"][0]["value"]["value"], "18446744073709551615");
    assert_eq!(tree["value"][1]["value"]["value"], "-9223372036854775808");
    let o = run(
        "cbor_parse",
        &hex::decode("3bffffffffffffffff").unwrap(),
        &json!({}),
    )
    .unwrap();
    assert!(matches!(&o["tree"],PortValue::Json(v)if v["value"]=="-18446744073709551616"));
    let payload: Vec<u8> = (0..=255).collect();
    for (id, mut header) in [
        ("cbor_parse", vec![0x59, 1, 0]),
        ("msgpack_parse", vec![0xc5, 1, 0]),
    ] {
        header.extend(&payload);
        let o = run(id, &header, &json!({})).unwrap();
        assert!(accepted(&o));
        assert!(matches!(&o["tree"],PortValue::Json(v)if v["value"]==hex::encode(&payload)));
    }
    let o = run("msgpack_parse", &[0xa2, 0xff, 0], &json!({})).unwrap();
    assert!(matches!(&o["tree"],PortValue::Json(v)if v["rawHex"]=="ff00"));
    let o = run(
        "cbor_parse",
        &hex::decode("5b000000000000000141").unwrap(),
        &json!({}),
    )
    .unwrap();
    assert!(accepted(&o));
    assert!(matches!(&o["tree"],PortValue::Json(v)if v["value"]=="41"));
    let o = run("cbor_parse", &[0xf8, 32], &json!({})).unwrap();
    assert!(matches!(&o["tree"],PortValue::Json(v)if v["value"]==32));
}
#[test]
fn truncated_lengths_and_invalid_cbor_markers_do_not_create_values() {
    for (id, hexes) in [
        (
            "msgpack_parse",
            vec![
                "cc",
                "cd00",
                "ce0000",
                "cf0000",
                "d0",
                "d100",
                "d20000",
                "d30000",
                "ca00",
                "cb00",
                "d4ff",
                "c4ff00",
                "c5000500",
                "d90261",
                "da000261",
                "db0000000261",
                "ddffffffff",
                "df00000001c0",
                "c1",
            ],
        ),
        (
            "cbor_parse",
            vec![
                "18",
                "1900",
                "1a0000",
                "1b0000",
                "ff",
                "1f",
                "3f",
                "df",
                "f818",
                "f81f",
                "5f6161ff",
                "7f4100ff",
                "5f5fff",
                "9f01",
                "bf01ff",
                "5bffffffffffffffff",
                "82",
                "a201",
                "1c",
            ],
        ),
        (
            "protobuf_wire_parse",
            vec![
                "08ffffffffffffffffffff",
                "080112ff",
                "0080",
                "ffffffffffffffffff01",
                "0e",
            ],
        ),
    ] {
        for h in hexes {
            let o = run(id, &hex::decode(h).unwrap(), &json!({})).unwrap();
            assert!(!accepted(&o), "{id} {h}");
            assert!(!o["error"].as_text().unwrap().is_empty(), "{id} {h}");
        }
    }
    // Every prefix of a declared scalar/blob is rejected, rather than padding
    // unavailable bytes with zero as the JavaScript reference sometimes does.
    for (id, h) in [
        ("msgpack_parse", "cb400921fb54442d18"),
        ("msgpack_parse", "d90548656c6c6f"),
        ("cbor_parse", "fb400921fb54442d18"),
        ("cbor_parse", "6548656c6c6f"),
        ("protobuf_wire_parse", "120548656c6c6f"),
    ] {
        let bytes = hex::decode(h).unwrap();
        for end in 0..bytes.len() {
            assert!(
                !accepted(&run(id, &bytes[..end], &json!({})).unwrap()),
                "{id} prefix {end}"
            );
        }
    }
}
#[test]
fn protobuf_retains_partial_fields_and_nested_offsets() {
    let o = run(
        "protobuf_wire_parse",
        &hex::decode("089601120208011d0000803f").unwrap(),
        &json!({}),
    )
    .unwrap();
    assert!(accepted(&o));
    let PortValue::Json(tree) = &o["tree"] else {
        panic!()
    };
    assert_eq!(tree[0]["value"], "150");
    assert_eq!(tree[0]["zigzag"], "75");
    assert_eq!(tree[1]["nested"][0]["offset"], 0);
    assert_eq!(tree[1]["nested"][0]["absoluteOffset"], 5);
    assert_eq!(tree[2]["float"], "1");
    let o = run("protobuf_wire_parse", &[8, 1, 0x12, 10, 0xff], &json!({})).unwrap();
    assert!(!accepted(&o));
    assert!(matches!(&o["tree"],PortValue::Json(v)if v.as_array().unwrap().len()==1));
    assert_eq!(o["remainder"].as_bytes().unwrap().as_ref(), &[255]);
}
#[test]
fn serialization_depth_item_bounds_and_inflight_cancellation() {
    for (id, marker) in [("msgpack_parse", 0x91), ("cbor_parse", 0x81)] {
        let mut data = vec![marker; 10];
        data.push(1);
        let o = run(id, &data, &json!({"maxDepth":4})).unwrap();
        assert!(!accepted(&o));
        let mut large = if id == "msgpack_parse" {
            vec![0xdd, 0, 0, 0x4e, 0x21]
        } else {
            vec![0x99, 0x4e, 0x21]
        };
        large.extend(vec![0; 20_001]);
        let o = run(id, &large, &json!({})).unwrap();
        assert!(!accepted(&o));
    }
    struct Stop(CancellationToken);
    impl ProgressSink for Stop {
        fn emit(&self, e: ProgressEvent) {
            if matches!(e,ProgressEvent::NodeProgress{pct,..}if pct>0.0&&pct<1.0) {
                self.0.cancel();
            }
        }
    }
    for (id, mut data, item) in [
        ("msgpack_parse", vec![0xdc, 3, 0xe8], vec![0]),
        ("cbor_parse", vec![0x99, 3, 0xe8], vec![0]),
        ("protobuf_wire_parse", vec![], vec![8, 0]),
    ] {
        for _ in 0..1000 {
            data.extend(&item);
        }
        let cancel = CancellationToken::new();
        let r = GraphExecutor::run_node(
            &super::default_registry(),
            id,
            &one("data", PortValue::Bytes(data.into())),
            &json!({}),
            &Stop(cancel.clone()),
            &cancel,
        );
        assert!(matches!(r, Err(CoreError::Cancelled)), "{id}: {r:?}");
    }
}
