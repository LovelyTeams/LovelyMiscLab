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
fn run(id: &str, data: &[u8], p: &Value) -> Result<PortMap, CoreError> {
    call(id, PortValue::Bytes(data.into()), p)
}
fn complete(o: &PortMap) -> bool {
    matches!(&o["complete"], PortValue::Bool(true))
}
fn bson(fields: &[u8]) -> Vec<u8> {
    let mut b = ((fields.len() + 5) as i32).to_le_bytes().to_vec();
    b.extend(fields);
    b.push(0);
    b
}
fn field(code: u8, name: &str, payload: &[u8]) -> Vec<u8> {
    let mut f = vec![code];
    f.extend(name.as_bytes());
    f.push(0);
    f.extend(payload);
    f
}
#[test]
fn pinned_object_serialization_vectors_cover_three_operations() {
    let f: Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/ebctf-object-serial-batch10.json"
    ))
    .unwrap();
    assert_eq!(f["revision"], "ea2bc36dae31c97060f7f2cddad8df5518afad7b");
    let vs = f["vectors"].as_array().unwrap();
    assert_eq!(vs.len(), 249);
    let reg = super::default_registry();
    let mut errors = Vec::new();
    let bson_tail = regex::Regex::new(r"\n\n⚠ 仅消费 (\d+)/(\d+) 字节$").unwrap();
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
        let mut expected = v["text"].as_str().unwrap().to_string();
        // The upstream BSON cursor stops before the final NUL. The Rust parser
        // consumes it and checks it, rather than claiming one byte is unused.
        if id == "bson_parse" {
            if let Some(c) = bson_tail.captures(&expected) {
                assert_eq!(
                    c[1].parse::<usize>().unwrap() + 1,
                    c[2].parse::<usize>().unwrap()
                );
                expected.truncate(c.get(0).unwrap().start());
            }
        }
        match result {
            Err(e) => errors.push(format!("{i} {id}: {e}")),
            Ok(o) => {
                if o["text"].as_text().unwrap() != expected {
                    errors.push(format!(
                        "{i} {id} input {:?}: actual {:?}, expected {:?}",
                        v["input"],
                        o["text"].as_text().unwrap(),
                        expected
                    ));
                }
            }
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
#[test]
fn bson_document_boundaries_nested_siblings_and_additional_types() {
    let nested = bson(&field(16, "n", &1i32.to_le_bytes()));
    let mut f = field(3, "child", &nested);
    f.extend(field(16, "after", &42i32.to_le_bytes()));
    let data = bson(&f);
    let o = run("bson_parse", &data, &json!({})).unwrap();
    assert!(complete(&o));
    let PortValue::Json(v) = &o["tree"] else {
        panic!()
    };
    assert_eq!(v[1]["name"], "after");
    assert_eq!(v[1]["value"], 42);
    assert_eq!(v[0]["value"][0]["name"], "n");
    assert!(o["remainder"].as_bytes().unwrap().is_empty());
    for n in 0..data.len() {
        let o = run("bson_parse", &data[..n], &json!({})).unwrap();
        assert!(!complete(&o), "prefix {n}");
    }
    for code in [6, 13, 14, 19] {
        let payload = match code {
            13 | 14 => vec![2, 0, 0, 0, b'x', 0],
            19 => (0..16).collect(),
            _ => vec![],
        };
        let o = run("bson_parse", &bson(&field(code, "x", &payload)), &json!({})).unwrap();
        assert!(complete(&o));
    }
    let mut scope = vec![16, 0, 0, 0, 2, 0, 0, 0, b'x', 0];
    scope.extend(bson(&[]));
    // 4 length + 6 code string + 5 empty document = 15.
    scope[0] = 15;
    let o = run("bson_parse", &bson(&field(15, "code", &scope)), &json!({})).unwrap();
    assert!(complete(&o));
    for data in [
        bson(&field(8, "bad", &[2])),
        bson(&field(2, "bad", &[2, 0, 0, 0, b'x', b'!'])),
        bson(&field(5, "bad", &[4, 0, 0, 0, 2, 1, 0, 0, 0])),
        vec![5, 0, 0, 0, 1],
        bson(&field(2, "bad", &[255, 255, 255, 255])),
    ] {
        assert!(!complete(&run("bson_parse", &data, &json!({})).unwrap()));
    }
}
#[test]
fn php_uses_byte_lengths_preserves_integers_and_nested_metadata() {
    let source = "O:3:\"类\":2:{s:1:\"n\";i:9223372036854775807;s:3:\"文\";s:7:\"中🙂\";}";
    let o = call(
        "php_serialize_parse",
        PortValue::Text(source.into()),
        &json!({}),
    )
    .unwrap();
    assert!(complete(&o));
    let PortValue::Json(v) = &o["tree"] else {
        panic!()
    };
    assert_eq!(v["className"], "类");
    assert_eq!(v["value"][0]["value"]["value"], "9223372036854775807");
    assert_eq!(v["value"][1]["value"]["length"], 7);
    assert_eq!(v["value"][1]["value"]["value"], "中🙂");
    assert!(o["text"].as_text().unwrap().contains("string[7]"));
    let bytes: Vec<u8> = (0..=255).collect();
    let mut raw = b"s:256:\"".to_vec();
    raw.extend(&bytes);
    raw.extend(b"\";");
    let o = run("php_serialize_parse", &raw, &json!({})).unwrap();
    assert!(complete(&o));
    assert!(matches!(&o["tree"],PortValue::Json(v)if v["rawHex"]==hex::encode(bytes)));
    for (s, value) in [
        ("d:INF;", "Infinity"),
        ("d:-INF;", "-Infinity"),
        ("d:NAN;", "NaN"),
    ] {
        let o = run("php_serialize_parse", s.as_bytes(), &json!({})).unwrap();
        assert!(matches!(&o["tree"],PortValue::Json(v)if v["value"]==value));
    }
    for s in [
        "b:2;",
        "s:1:\"中\";",
        "s:-1:\"\";",
        "s:100:\"x\";",
        "a:-1:{}",
        "O:3:\"Foo\":1:{}",
        "r:0;",
        "i:12abc;",
        "i:;",
        "d:;",
        "d:0x10;",
        "N",
        "a:1:{N;N;}",
    ] {
        assert!(
            !complete(&run("php_serialize_parse", s.as_bytes(), &json!({})).unwrap()),
            "{s}"
        );
    }
}
#[test]
fn java_modified_utf8_flags_and_unparsed_markers_are_explicit() {
    let mut bytes = vec![0xac, 0xed, 0, 5, 0x74, 0, 8];
    bytes.extend([0xc0, 0x80, 0xed, 0xa0, 0xbd, 0xed, 0xb9, 0x82]);
    let o = run("java_serialize_identify", &bytes, &json!({})).unwrap();
    assert!(complete(&o));
    assert!(
        matches!(&o["tree"],PortValue::Json(v)if v["markers"][0]["string"]["text"]=="\0🙂"&&v["markers"][0]["string"]["utf16"]==json!([0,0xd83d,0xde42]))
    );
    let o = run(
        "java_serialize_identify",
        &hex::decode("aced0005740003eda080").unwrap(),
        &json!({}),
    )
    .unwrap();
    assert!(
        matches!(&o["tree"],PortValue::Json(v)if v["markers"][0]["string"]["utf16"]==json!([0xd800])&&v["markers"][0]["string"]["rawHex"]=="eda080")
    );
    let class = hex::decode("aced0005720003466f6f00000000000000010200007870").unwrap();
    let o = run("java_serialize_identify", &class, &json!({})).unwrap();
    assert!(complete(&o));
    assert!(
        matches!(&o["tree"],PortValue::Json(v)if v["markers"][0]["flagNames"]==json!(["SC_SERIALIZABLE"]))
    );
    let o = run(
        "java_serialize_identify",
        &[0xac, 0xed, 0, 5, 0x75, 0x76],
        &json!({}),
    )
    .unwrap();
    assert!(!complete(&o));
    assert!(matches!(&o["tree"],PortValue::Json(v)if v["unsupportedOffsets"]==json!([4,5])));
    for h in [
        "aced000574000141",
        "aced0005770300ff80",
        "aced00057c000000000000000141",
    ] {
        let data = hex::decode(h).unwrap();
        for n in 5..data.len() {
            assert!(!complete(
                &run("java_serialize_identify", &data[..n], &json!({})).unwrap()
            ));
        }
    }
    for h in [
        "aced000574000100",
        "aced0005740002c181",
        "aced0005740004f09f9982",
        "aced00057cffffffffffffffff",
    ] {
        assert!(!complete(
            &run(
                "java_serialize_identify",
                &hex::decode(h).unwrap(),
                &json!({})
            )
            .unwrap()
        ));
    }
}
#[test]
fn object_inspection_cancels_during_scanning_and_limits_recursion() {
    struct Stop(CancellationToken);
    impl ProgressSink for Stop {
        fn emit(&self, e: ProgressEvent) {
            if matches!(e,ProgressEvent::NodeProgress{pct,..}if pct>0.0&&pct<1.0) {
                self.0.cancel();
            }
        }
    }
    let mut fields = Vec::new();
    for i in 0i32..1000 {
        fields.extend(field(16, &i.to_string(), &i.to_le_bytes()));
    }
    let mut java = vec![0xac, 0xed, 0, 5];
    java.extend(vec![0x70; 1000]);
    let mut php = String::from("a:1000:{");
    for i in 0..1000 {
        php.push_str(&format!("i:{i};N;"));
    }
    php.push('}');
    for (id, data) in [
        ("bson_parse", bson(&fields)),
        ("java_serialize_identify", java),
        ("php_serialize_parse", php.into_bytes()),
    ] {
        let cancel = CancellationToken::new();
        let o = GraphExecutor::run_node(
            &super::default_registry(),
            id,
            &one("data", PortValue::Bytes(data.into())),
            &json!({}),
            &Stop(cancel.clone()),
            &cancel,
        );
        assert!(matches!(o, Err(CoreError::Cancelled)), "{id}: {o:?}");
    }
    let mut nested = bson(&[]);
    for _ in 0..10 {
        nested = bson(&field(3, "nested", &nested));
    }
    assert!(!complete(
        &run("bson_parse", &nested, &json!({"maxDepth":3})).unwrap()
    ));
    let source = format!("{}N;{}", "a:1:{i:0;".repeat(10), "}".repeat(10));
    assert!(!complete(
        &run(
            "php_serialize_parse",
            source.as_bytes(),
            &json!({"maxDepth":3})
        )
        .unwrap()
    ));
}
