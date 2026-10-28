use misclab_core::{
    cancel::CancellationToken,
    error::CoreError,
    graph::{executor::GraphExecutor, port::PortValue},
    node::{registry::NodeRegistry, PortMap},
    nodes::default_registry,
    progress::{NullSink, ProgressEvent, ProgressSink},
};
use serde_json::{json, Value};

fn run(reg: &NodeRegistry, id: &str, inputs: PortMap, params: Value) -> Result<PortMap, CoreError> {
    GraphExecutor::run_node(
        reg,
        id,
        &inputs,
        &params,
        &NullSink,
        &CancellationToken::new(),
    )
}
fn input(key: &str, s: &str) -> PortMap {
    [(key.into(), PortValue::Text(s.into()))].into()
}
fn byte_input(key: &str, b: &[u8]) -> PortMap {
    [(key.into(), PortValue::Bytes(b.into()))].into()
}
fn data(out: &PortMap, key: &str) -> Vec<u8> {
    out[key].as_bytes().unwrap().to_vec()
}
fn info(out: &PortMap) -> &Value {
    let PortValue::Json(j) = &out["json"] else {
        panic!()
    };
    j
}
fn message(headers: &str, body: &str) -> String {
    format!("Content-Type: multipart/mixed; boundary=x\r\n\r\n--x\r\n{headers}\r\n{body}\r\n--x--")
}

#[test]
fn pinned_mail_vectors_cover_both_directions_encodings_forks_and_crc_reports() {
    let f: Value = serde_json::from_str(include_str!("fixtures/ebctf-mail-batch22.json")).unwrap();
    assert_eq!(f["revision"], "ea2bc36dae31c97060f7f2cddad8df5518afad7b");
    assert_eq!(f["vectors"].as_array().unwrap().len(), 294);
    let reg = default_registry();
    for (i, v) in f["vectors"].as_array().unwrap().iter().enumerate() {
        let id = v["node"].as_str().unwrap();
        let out = run(
            &reg,
            id,
            serde_json::from_value(v["input"].clone()).unwrap(),
            v["params"].clone(),
        );
        if v.get("error").is_some() {
            assert!(out.is_err(), "{i} {id}");
            continue;
        }
        let out = out.unwrap_or_else(|e| panic!("{i} {id}: {e}"));
        assert_eq!(
            out["text"].as_text().unwrap(),
            v["text"].as_str().unwrap(),
            "{i} {id}"
        );
        for port in ["bytes", "resource"] {
            if let Some(expected) = v.get(port) {
                assert_eq!(
                    data(&out, port),
                    serde_json::from_value::<Vec<u8>>(expected.clone()).unwrap(),
                    "{i} {id} {port}"
                );
            }
        }
        if let Some(valid) = v.get("validCrc") {
            assert_eq!(info(&out)["crcValid"], *valid);
        }
    }
}

#[test]
fn mime_binary_attachments_and_all_line_endings_roundtrip_exactly() {
    let reg = default_registry();
    let all: Vec<u8> = (0..=255).cycle().take(4096).collect();
    for encoding in ["auto", "base64", "quoted-printable"] {
        for bytes in [
            vec![],
            all.clone(),
            b"a\r\nb\nc\r\0\xff\r\n".to_vec(),
            "中文😀\r\n".as_bytes().to_vec(),
        ] {
            let enc = run(
                &reg,
                "mime_multipart_encode",
                byte_input("data", &bytes),
                json!({"encoding":encoding,"filename":"附件\\\".bin"}),
            )
            .unwrap();
            assert_eq!(
                data(&enc, "bytes"),
                enc["text"].as_text().unwrap().as_bytes()
            );
            let dec = run(
                &reg,
                "mime_multipart_decode",
                [("data".into(), enc["bytes"].clone())].into(),
                json!({}),
            )
            .unwrap();
            assert_eq!(data(&dec, "bytes"), bytes, "{encoding}");
            assert_eq!(info(&dec)["parts"][0]["name"], "附件\\\".bin");
            assert_eq!(info(&dec)["closed"], true);
        }
    }
    let list = ["", "  | \t", "a\r\nb\r\n", "😀中文\r"]
        .map(str::to_owned)
        .to_vec();
    let enc = run(
        &reg,
        "mime_multipart_encode",
        [("data".into(), PortValue::StringList(list.clone()))].into(),
        json!({}),
    )
    .unwrap();
    for (i, s) in list.iter().enumerate() {
        let dec = run(
            &reg,
            "mime_multipart_decode",
            [("data".into(), enc["text"].clone())].into(),
            json!({"part":i+1}),
        )
        .unwrap();
        assert_eq!(data(&dec, "bytes"), s.as_bytes());
        assert_eq!(info(&dec)["count"], list.len());
    }
    // CRLF normalization belongs only to the upstream-compatible report.
    let dec = run(
        &reg,
        "mime_multipart_decode",
        input("data", &message("", "a\r\nb\r\n")),
        json!({}),
    )
    .unwrap();
    assert_eq!(data(&dec, "bytes"), b"a\r\nb\r\n");
    assert_eq!(dec["decoded"].as_text().unwrap(), "a\r\nb\r\n");
    assert_eq!(dec["text"].as_text().unwrap(), "part 1：(无类型)\na\nb\n");
}

#[test]
fn mime_folding_preambles_boundaries_qp_unicode_and_nested_parts_are_not_lost() {
    let reg = default_registry();
    let wire = "Content-Type: multipart/mixed;\r\n\tboundary=\"a:b\"\r\n\r\npreamble --a:b is not a delimiter\r\n--a:b\t\r\nContent-Type: text/plain;\r\n name=\"a;b.txt\"\r\nContent-Transfer-Encoding: quoted-printable\r\n\r\n中文=20=F0=9F=98=80=\r\nxyz\r\n--a:b-extra\r\n --a:b\r\n--a:b--\t\r\nepilogue";
    let dec = run(
        &reg,
        "mime_multipart_decode",
        input("data", wire),
        json!({}),
    )
    .unwrap();
    assert_eq!(
        data(&dec, "bytes"),
        "中文 😀xyz\r\n--a:b-extra\r\n --a:b".as_bytes()
    );
    assert_eq!(info(&dec)["parts"][0]["name"], "a;b.txt");
    let inner = message("Content-Transfer-Encoding: base64\r\n", "AP+A");
    let outer = format!("--z\r\nContent-Type: multipart/mixed; boundary=x\r\n\r\n{inner}\r\n--z--");
    let dec = run(
        &reg,
        "mime_multipart_decode",
        input("data", &outer),
        json!({"boundary":"z"}),
    )
    .unwrap();
    assert_eq!(data(&dec, "bytes"), inner.as_bytes());
    let nested = run(
        &reg,
        "mime_multipart_decode",
        [("data".into(), dec["bytes"].clone())].into(),
        json!({}),
    )
    .unwrap();
    assert_eq!(data(&nested, "bytes"), [0, 255, 128]);
    let partial = "Content-Type: multipart/mixed; boundary=x\n\n--x\n\nhello\n";
    let dec = run(
        &reg,
        "mime_multipart_decode",
        input("data", partial),
        json!({}),
    )
    .unwrap();
    assert_eq!(info(&dec)["closed"], false);
    assert!(!info(&dec)["warnings"].as_array().unwrap().is_empty());
    assert_eq!(data(&dec, "bytes"), b"hello\n");
    let unknown = run(
        &reg,
        "mime_multipart_decode",
        input(
            "data",
            &message("Content-Transfer-Encoding: invented\r\n", "raw"),
        ),
        json!({}),
    )
    .unwrap();
    assert_eq!(data(&unknown, "bytes"), b"raw");
    assert!(unknown["text"].as_text().unwrap().contains("未知传输编码"));
    let qp = run(
        &reg,
        "mime_multipart_decode",
        input(
            "data",
            &message(
                "Content-Transfer-Encoding: quoted_printable\r\n",
                "=QZ=2=\n=41",
            ),
        ),
        json!({}),
    )
    .unwrap();
    assert_eq!(data(&qp, "bytes"), b"=QZ=2A");
    let empty = run(
        &reg,
        "mime_multipart_decode",
        input("data", "--x--"),
        json!({"boundary":"x"}),
    )
    .unwrap();
    assert_eq!(info(&empty)["count"], 0);
    assert_eq!(data(&empty, "bytes"), b"");
}

#[test]
fn mime_invalid_parameters_ambiguous_headers_and_resource_limits_are_explicit() {
    let reg = default_registry();
    for p in [
        json!({"boundary":"x\r\ny"}),
        json!({"boundary":"a".repeat(71)}),
        json!({"boundary":"x"}),
        json!({"filename":"x\nInjected: y"}),
        json!({"contentType":"invalid"}),
        json!({"contentType":"text/plain\r\n"}),
        json!({"encoding":"nope"}),
        json!({"boundary":false}),
    ] {
        assert!(
            run(
                &reg,
                "mime_multipart_encode",
                input("data", "abc\n--x\ndef"),
                p.clone()
            )
            .is_err(),
            "{p}"
        );
    }
    let colliding = "a\n------ctf-part1\nbody";
    let enc = run(
        &reg,
        "mime_multipart_encode",
        input("data", colliding),
        json!({}),
    )
    .unwrap();
    assert!(enc["text"]
        .as_text()
        .unwrap()
        .contains("boundary=\"----ctf-part1-1\""));
    let dec = run(
        &reg,
        "mime_multipart_decode",
        [("data".into(), enc["text"].clone())].into(),
        json!({}),
    )
    .unwrap();
    assert_eq!(data(&dec, "bytes"), colliding.as_bytes());
    for body in ["A", "!!!!", "SGVsbG8==", "AA=A", "AB=="] {
        assert!(run(
            &reg,
            "mime_multipart_decode",
            input(
                "data",
                &message("Content-Transfer-Encoding: base64\r\n", body)
            ),
            json!({})
        )
        .is_err());
    }
    for (wire, p) in [
        (
            message(
                "Content-Type: text/plain\r\nContent-Type: text/html\r\n",
                "x",
            ),
            json!({}),
        ),
        (
            "Content-Type: multipart/mixed; boundary=x; boundary=y\r\n\r\n--x--".into(),
            json!({}),
        ),
        ("--x\nNo colon\n\na\n--x--".into(), json!({"boundary":"x"})),
        (
            format!("--x\nA: {}\n\nx\n--x--", "x".repeat(65536)),
            json!({"boundary":"x"}),
        ),
        (message("", "x"), json!({"part":2})),
        (message("", "x"), json!({"part":1.5})),
        (message("", "x"), json!({"part":null})),
    ] {
        assert!(run(&reg, "mime_multipart_decode", input("data", &wire), p).is_err());
    }
    let list = vec!["a".into(); 257];
    assert!(run(
        &reg,
        "mime_multipart_encode",
        [("data".into(), PortValue::StringList(list))].into(),
        json!({})
    )
    .is_err());
    let many = "--x\n\na\n".repeat(257) + "--x--";
    assert!(run(
        &reg,
        "mime_multipart_decode",
        input("data", &many),
        json!({"boundary":"x"})
    )
    .is_err());
    assert!(run(
        &reg,
        "mime_multipart_encode",
        byte_input("data", &[255]),
        json!({"encoding":"raw"})
    )
    .is_err());
    assert!(run(
        &reg,
        "mime_multipart_encode",
        input("data", &"a".repeat(16 * 1024 * 1024 + 1)),
        json!({})
    )
    .is_err());
}

#[test]
fn binhex_forks_text_encodings_crc_vector_and_bounded_previews() {
    let reg = default_registry();
    let source: Vec<u8> = (0..=255).cycle().take(16384).collect();
    let resource = vec![0x90; 511];
    let mut inputs = byte_input("data", &source);
    inputs.extend(byte_input("resource", &resource));
    let enc = run(
        &reg,
        "binhex_encode",
        inputs,
        json!({"filename":"中文.dat","fileType":"BINA","creator":"TEST","flags":32769}),
    )
    .unwrap();
    let dec = run(
        &reg,
        "binhex_decode",
        [("text".into(), enc["text"].clone())].into(),
        json!({"strictCrc":true}),
    )
    .unwrap();
    assert_eq!(data(&dec, "bytes"), source);
    assert_eq!(data(&dec, "resource"), resource);
    assert_eq!(info(&dec)["name"], "中文.dat");
    assert_eq!(info(&dec)["flags"], 32769);
    assert_eq!(info(&dec)["creator"], "TEST");
    assert_eq!(info(&dec)["fileType"], "BINA");
    assert!(dec["text"].as_text().unwrap().len() < 10000);
    assert!(dec["text"].as_text().unwrap().contains("截断"));
    for (mode, expected) in [
        ("auto", vec![0xe9]),
        ("Latin-1", vec![0xe9]),
        ("UTF-8", vec![0xc3, 0xa9]),
    ] {
        let enc = run(
            &reg,
            "binhex_encode",
            input("data", "é"),
            json!({"textEncoding":mode}),
        )
        .unwrap();
        let dec = run(
            &reg,
            "binhex_decode",
            [("text".into(), enc["text"].clone())].into(),
            json!({}),
        )
        .unwrap();
        assert_eq!(data(&dec, "bytes"), expected);
    }
    let enc = run(&reg, "binhex_encode", input("data", "123456789"), json!({})).unwrap();
    let dec = run(
        &reg,
        "binhex_decode",
        [("text".into(), enc["text"].clone())].into(),
        json!({}),
    )
    .unwrap();
    assert_eq!(info(&dec)["crc"]["data"]["calculated"], "31c3"); // CRC-16/XMODEM check value.
    assert_eq!(info(&dec)["crc"]["resource"]["calculated"], "0000");
    // MacRoman filenames are decoded independently of the UTF-8 data preview.
    // CRC fields are intentionally zero: the default forensic mode retains the
    // bytes and reports invalid checksums instead of silently repairing them.
    let mut mac_name = vec![0; 27];
    mac_name[0] = 1;
    mac_name[1] = 0x8e;
    let mac = run(
        &reg,
        "binhex_decode",
        input("text", &hqx(&mac_name)),
        json!({"nameEncoding":"MacRoman"}),
    )
    .unwrap();
    assert_eq!(info(&mac)["name"], "é");
    assert_eq!(info(&mac)["nameHex"], "8e");
    assert_eq!(info(&mac)["crcValid"], false);
    for p in [
        json!({"strictCrc":"true"}),
        json!({"nameEncoding":"unknown"}),
        json!({"nameEncoding":null}),
    ] {
        assert!(run(
            &reg,
            "binhex_decode",
            [("text".into(), enc["text"].clone())].into(),
            p
        )
        .is_err());
    }
    let with_preamble = format!("Mail: subject\n{}", enc["text"].as_text().unwrap());
    assert!(run(
        &reg,
        "binhex_decode",
        input("text", &with_preamble),
        json!({"strictCrc":true})
    )
    .is_ok());
}

// Deliberately no compression: independent HQX bit packing constructs malformed
// packets without depending on the encoder under test.
fn hqx(raw: &[u8]) -> String {
    let alphabet = b"!\"#$%&'()*+,-012345689@ABCDEFGHIJKLMNPQRSTUVXYZ[`abcdefhijklmpqr";
    let mut s = String::from(":");
    for chunk in raw.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |v, (i, b)| v | ((*b as u32) << (16 - 8 * i)));
        for i in 0..(chunk.len() * 8).div_ceil(6) {
            s.push(alphabet[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    s.push(':');
    s
}
#[test]
fn binhex_corruption_lengths_rle_and_hqx_errors_are_detected() {
    let reg = default_registry();
    let f: Value = serde_json::from_str(include_str!("fixtures/ebctf-mail-batch22.json")).unwrap();
    for v in f["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["validCrc"] == false)
    {
        assert!(run(
            &reg,
            "binhex_decode",
            serde_json::from_value(v["input"].clone()).unwrap(),
            json!({"strictCrc":true})
        )
        .is_err());
    }
    for raw in [
        vec![],
        vec![0x90],
        vec![0x90, 2],
        vec![63; 22],
        vec![1, 97, 0, 0],
        vec![0; 26],
    ] {
        assert!(run(&reg, "binhex_decode", input("text", &hqx(&raw)), json!({})).is_err());
    }
    for encoded in [":z:", ":\u{4e2d}:", ":!:", ":!\":", ":!!!#:"] {
        assert!(run(&reg, "binhex_decode", input("text", encoded), json!({})).is_err());
    }
    let mut raw = vec![0; 28];
    raw[0] = 1;
    raw[1] = b'a';
    raw[13] = 0xff;
    raw[14] = 0xff;
    raw[15] = 0xff;
    raw[16] = 0xff;
    assert!(run(&reg, "binhex_decode", input("text", &hqx(&raw)), json!({})).is_err());
    // 66K tiny RLE instructions would expand beyond the 16 MiB fork budget.
    let mut bomb = vec![65];
    for _ in 0..66000 {
        bomb.extend_from_slice(&[0x90, 255]);
    }
    assert!(run(&reg, "binhex_decode", input("text", &hqx(&bomb)), json!({})).is_err());
    for params in [
        json!({"filename":"中".repeat(22)}),
        json!({"filename":"x\ny"}),
        json!({"fileType":"BAD"}),
        json!({"flags":65536}),
        json!({"flags":1.5}),
        json!({"creator":false}),
        json!({"textEncoding":"bad"}),
        json!({"textEncoding":"Latin-1"}),
    ] {
        assert!(
            run(&reg, "binhex_encode", input("data", "中文"), params.clone()).is_err(),
            "{params}"
        );
    }
}

#[test]
fn mail_nodes_chain_binary_ports_and_graph_matches_standalone() {
    let reg = default_registry();
    let graph = serde_json::from_value(json!({"nodes":[
        {"id":"mime","descriptorId":"mime_multipart_encode","params":{}},
        {"id":"parse","descriptorId":"mime_multipart_decode","params":{}},
        {"id":"hqx","descriptorId":"binhex_encode","params":{}},
        {"id":"unhqx","descriptorId":"binhex_decode","params":{"strictCrc":true}},
        {"id":"next","descriptorId":"coalesce","params":{}}
    ],"edges":[
        {"from":{"node":"mime","port":"bytes"},"to":{"node":"parse","port":"data"}},
        {"from":{"node":"parse","port":"bytes"},"to":{"node":"hqx","port":"data"}},
        {"from":{"node":"hqx","port":"text"},"to":{"node":"unhqx","port":"text"}},
        {"from":{"node":"unhqx","port":"bytes"},"to":{"node":"next","port":"a"}}
    ]}))
    .unwrap();
    let source: Vec<u8> = (0..=255).collect();
    let out = GraphExecutor::new(&reg, &graph)
        .unwrap()
        .with_seed_inputs(
            [(
                ("mime".into(), "data".into()),
                PortValue::Bytes(source.clone().into()),
            )]
            .into(),
        )
        .run(&NullSink, &CancellationToken::new())
        .unwrap();
    let mut inputs = byte_input("data", &source);
    for (node, id, params, port, next_port) in [
        ("mime", "mime_multipart_encode", json!({}), "bytes", "data"),
        ("parse", "mime_multipart_decode", json!({}), "bytes", "data"),
        ("hqx", "binhex_encode", json!({}), "text", "text"),
        (
            "unhqx",
            "binhex_decode",
            json!({"strictCrc":true}),
            "bytes",
            "a",
        ),
    ] {
        let direct = run(&reg, id, inputs, params).unwrap();
        assert_eq!(
            serde_json::to_value(&out[node]).unwrap(),
            serde_json::to_value(&direct).unwrap()
        );
        inputs = [(next_port.into(), direct[port].clone())].into();
    }
    assert_eq!(data(&out["next"], "output"), source);
}

#[test]
fn all_mail_directions_observe_cancellation_before_returning_outputs() {
    struct Stop(CancellationToken);
    impl ProgressSink for Stop {
        fn emit(&self, e: ProgressEvent) {
            if matches!(e,ProgressEvent::NodeProgress{pct,..} if pct>0.1 && pct<1.0) {
                self.0.cancel();
            }
        }
    }
    let reg = default_registry();
    let hqx = run(&reg, "binhex_encode", input("data", "hello"), json!({})).unwrap();
    for (id, inputs) in [
        ("binhex_encode", input("data", "hello")),
        (
            "binhex_decode",
            [("text".into(), hqx["text"].clone())].into(),
        ),
        ("mime_multipart_encode", input("data", "hello|world")),
        (
            "mime_multipart_decode",
            input("data", &message("", "hello")),
        ),
    ] {
        let cancel = CancellationToken::new();
        let out = GraphExecutor::run_node(
            &reg,
            id,
            &inputs,
            &json!({}),
            &Stop(cancel.clone()),
            &cancel,
        );
        assert!(matches!(out, Err(CoreError::Cancelled)), "{id}");
    }
}
