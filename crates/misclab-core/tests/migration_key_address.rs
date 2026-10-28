use base64::{engine::general_purpose::STANDARD, Engine as _};
use misclab_core::{
    cancel::CancellationToken,
    error::CoreError,
    graph::{executor::GraphExecutor, port::PortValue},
    node::{registry::NodeRegistry, PortMap},
    nodes::default_registry,
    progress::{NullSink, ProgressEvent, ProgressSink},
};
use serde_json::{json, Value};

fn run(reg: &NodeRegistry, id: &str, inputs: PortMap, p: Value) -> Result<PortMap, CoreError> {
    GraphExecutor::run_node(reg, id, &inputs, &p, &NullSink, &CancellationToken::new())
}
fn text(port: &str, value: &str) -> PortMap {
    [(port.into(), PortValue::Text(value.into()))].into()
}
fn bytes(port: &str, value: &[u8]) -> PortMap {
    [(port.into(), PortValue::Bytes(value.into()))].into()
}
fn info(out: &PortMap) -> &Value {
    let PortValue::Json(v) = &out["json"] else {
        panic!()
    };
    v
}
fn field(b: &[u8]) -> Vec<u8> {
    let mut out = (b.len() as u32).to_be_bytes().to_vec();
    out.extend_from_slice(b);
    out
}
fn blob(kind: &str, fields: &[&[u8]]) -> Vec<u8> {
    let mut out = field(kind.as_bytes());
    for f in fields {
        out.extend(field(f));
    }
    out
}

#[test]
fn pinned_reference_keys_addresses_fingerprints_and_all_networks_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/ebctf-key-address-batch23.json")).unwrap();
    assert_eq!(
        fixture["revision"],
        "ea2bc36dae31c97060f7f2cddad8df5518afad7b"
    );
    assert_eq!(fixture["vectors"].as_array().unwrap().len(), 300);
    let reg = default_registry();
    for (i, v) in fixture["vectors"].as_array().unwrap().iter().enumerate() {
        let id = v["node"].as_str().unwrap();
        let result = run(
            &reg,
            id,
            serde_json::from_value(v["input"].clone()).unwrap(),
            v["params"].clone(),
        );
        if v.get("error").is_some() {
            assert!(result.is_err(), "{i} {id}");
            continue;
        }
        let out = result.unwrap_or_else(|e| panic!("{i} {id}: {e}"));
        if v.get("reportDifference").is_none() {
            assert_eq!(
                out["text"].as_text().unwrap(),
                v["text"].as_str().unwrap(),
                "{i} {id}"
            );
        }
        if let Some(b) = v.get("bytes") {
            assert_eq!(
                out["bytes"].as_bytes().unwrap().as_ref(),
                serde_json::from_value::<Vec<u8>>(b.clone()).unwrap(),
                "{i} {id}"
            );
        }
        if let Some(s) = v.get("scriptPubKey") {
            assert_eq!(
                hex::encode(out["script"].as_bytes().unwrap()),
                s.as_str().unwrap(),
                "{i}"
            );
        }
        for (key, port) in [("canonical", "address"), ("fingerprint", "fingerprint")] {
            if let Some(s) = v.get(key) {
                assert_eq!(
                    out[port].as_text().unwrap(),
                    s.as_str().unwrap(),
                    "{i} {id}"
                );
            }
        }
        for key in ["valid", "network"] {
            if let Some(value) = v.get(key) {
                assert_eq!(info(&out)[key], *value, "{i} {id}");
            }
        }
    }
}
#[test]
fn all_published_bip350_witness_vectors_validate_or_reject_with_exact_scripts() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/bip350-segwit.json")).unwrap();
    assert_eq!(fixture["valid"].as_array().unwrap().len(), 8);
    assert_eq!(fixture["invalid"].as_array().unwrap().len(), 15);
    let reg = default_registry();
    for v in fixture["valid"].as_array().unwrap() {
        for address in [
            v["address"].as_str().unwrap().to_lowercase(),
            v["address"].as_str().unwrap().to_uppercase(),
        ] {
            let out = run(
                &reg,
                "bitcoin_address_inspect",
                text("text", &address),
                json!({"requireValid":true}),
            )
            .unwrap();
            assert_eq!(info(&out)["valid"], true);
            assert_eq!(
                hex::encode(out["script"].as_bytes().unwrap()),
                v["scriptPubKey"].as_str().unwrap()
            );
            assert_eq!(out["address"].as_text().unwrap(), address.to_lowercase());
        }
    }
    for v in fixture["invalid"].as_array().unwrap() {
        assert!(
            run(
                &reg,
                "bitcoin_address_inspect",
                text("text", v["address"].as_str().unwrap()),
                json!({})
            )
            .is_err(),
            "{v}"
        );
    }
}
#[test]
fn bitcoin_checksum_success_alone_does_not_validate_unknown_versions_or_lengths() {
    let reg = default_registry();
    for (version, len, valid) in [
        (0, 20, true),
        (5, 20, true),
        (0x6f, 20, true),
        (0xc4, 20, true),
        (42, 20, false),
        (0, 19, false),
        (0, 21, false),
        (0, 0, false),
    ] {
        let mut payload = vec![version];
        payload.extend(vec![7; len]);
        let enc = run(
            &reg,
            "base58check_encode",
            bytes("data", &payload),
            json!({}),
        )
        .unwrap();
        let inputs = [("text".into(), enc["text"].clone())].into();
        let out = run(&reg, "bitcoin_address_inspect", inputs, json!({})).unwrap();
        assert_eq!(info(&out)["checksumValid"], true);
        assert_eq!(info(&out)["valid"], valid);
        assert_eq!(out.contains_key("script"), valid);
        let strict = run(
            &reg,
            "bitcoin_address_inspect",
            [("text".into(), enc["text"].clone())].into(),
            json!({"requireValid":true}),
        );
        assert_eq!(strict.is_ok(), valid);
    }
    let address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNb";
    let out = run(
        &reg,
        "bitcoin_address_inspect",
        text("text", address),
        json!({}),
    )
    .unwrap();
    assert_eq!(info(&out)["valid"], false);
    assert!(!out.contains_key("script"));
    assert!(run(
        &reg,
        "bitcoin_address_inspect",
        text("text", address),
        json!({"requireValid":true})
    )
    .is_err());
}
#[test]
fn eip55_matches_standard_without_prefix_and_distinguishes_unchecked_case() {
    let reg = default_registry();
    let canonical = "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed";
    for address in [
        canonical.to_owned(),
        canonical[2..].to_owned(),
        canonical.replacen("0x", "0X", 1),
    ] {
        let out = run(
            &reg,
            "ethereum_address_inspect",
            text("text", &address),
            json!({"requireChecksum":true}),
        )
        .unwrap();
        assert_eq!(out["address"].as_text().unwrap(), canonical);
        assert_eq!(info(&out)["valid"], true);
        assert_eq!(info(&out)["checksumMatches"], true);
        assert!(out["text"].as_text().unwrap().contains("校验通过"));
    }
    let lower = canonical.to_lowercase();
    let out = run(
        &reg,
        "ethereum_address_inspect",
        text("text", &lower),
        json!({}),
    )
    .unwrap();
    assert_eq!(info(&out)["valid"], true);
    assert_eq!(info(&out)["checksumMatches"], false);
    assert!(run(
        &reg,
        "ethereum_address_inspect",
        text("text", &lower),
        json!({"requireChecksum":true})
    )
    .is_err());
    let wrong = canonical.replacen('A', "a", 1);
    let out = run(
        &reg,
        "ethereum_address_inspect",
        text("text", &wrong),
        json!({}),
    )
    .unwrap();
    assert_eq!(info(&out)["valid"], false);
    for (id, p) in [
        ("ethereum_address_inspect", json!({"requireChecksum":1})),
        ("bitcoin_address_inspect", json!({"requireValid":"true"})),
    ] {
        assert!(run(&reg, id, text("text", canonical), p).is_err());
    }
    assert!(run(
        &reg,
        "ethereum_address_inspect",
        text("text", &"a".repeat(4097)),
        json!({})
    )
    .is_err());
}
#[test]
fn ssh_field_limits_type_mismatches_and_signed_mpints_are_not_silently_accepted() {
    let reg = default_registry();
    let ed = blob("ssh-ed25519", &[&[1; 32]]);
    for b in [
        vec![],
        vec![255; 4],
        blob("ssh-ed25519", &[&[1; 31]]),
        blob("ssh-rsa", &[&[0x80], &[0, 0x80]]),
        blob("ssh-rsa", &[&[0; 8193], &[1]]),
        blob("ssh-dss", &[&[1], &[2], &[3]]),
        blob("ecdsa-sha2-nistp256", &[b"nistp384", &[4; 97]]),
        blob("ecdsa-sha2-nistp256", &[b"nistp256", &[4; 64]]),
        blob("bad\0type", &[b"a"]),
        blob("ssh-ed25519", &[&[1; 32], b"extra"]),
        vec![0; 256 * 1024 + 1],
    ] {
        assert!(run(&reg, "ssh_public_key_parse", bytes("data", &b), json!({})).is_err());
    }
    let wrong = format!("ssh-rsa {}", STANDARD.encode(&ed));
    assert!(run(
        &reg,
        "ssh_public_key_parse",
        text("data", &wrong),
        json!({})
    )
    .is_err());
    let mut truncated = ed.clone();
    truncated.pop();
    assert!(run(
        &reg,
        "ssh_public_key_parse",
        bytes("data", &truncated),
        json!({})
    )
    .is_err());
    for p in [json!({"line":0}), json!({"line":1.2}), json!({"line":null})] {
        assert!(run(&reg, "ssh_public_key_parse", bytes("data", &ed), p).is_err());
    }
    assert!(run(
        &reg,
        "ssh_public_key_parse",
        text("data", &"x".repeat(512 * 1024 + 1)),
        json!({})
    )
    .is_err());
    // A high bit with the required leading zero is a positive 128, not -128.
    let positive = blob("ssh-rsa", &[&[1, 0, 1], &[0, 128]]);
    let out = run(
        &reg,
        "ssh_public_key_parse",
        bytes("data", &positive),
        json!({}),
    )
    .unwrap();
    assert_eq!(info(&out)["fields"]["n"], "128");
}
#[test]
fn ssh_line_selection_security_key_prefix_and_unknown_types_preserve_bytes() {
    let reg = default_registry();
    let ed = blob("ssh-ed25519", &[&[1; 32]]);
    let sk = blob(
        "sk-ssh-ed25519@openssh.com",
        &[&[7; 32], "ssh:中文".as_bytes()],
    );
    let text_input = format!(
        "# 注释\r\n\r\nssh-ed25519 {} first\r\nsk-ssh-ed25519@openssh.com {} 第二个",
        STANDARD.encode(&ed),
        STANDARD.encode(&sk)
    );
    let out = run(
        &reg,
        "ssh_public_key_parse",
        text("data", &text_input),
        json!({"line":2}),
    )
    .unwrap();
    assert_eq!(out["bytes"].as_bytes().unwrap().as_ref(), sk);
    assert_eq!(info(&out)["comment"], "第二个");
    assert_eq!(info(&out)["fields"]["application"], "ssh:中文");
    assert!(run(
        &reg,
        "ssh_public_key_parse",
        text("data", &text_input),
        json!({"line":3})
    )
    .is_err());
    let unknown = blob("future-key", &[b"\0\xffother"]);
    let out = run(
        &reg,
        "ssh_public_key_parse",
        bytes("data", &unknown),
        json!({}),
    )
    .unwrap();
    assert!(!info(&out)["warnings"].as_array().unwrap().is_empty());
    assert_eq!(out["bytes"].as_bytes().unwrap().as_ref(), unknown);
    let long = format!(
        "ssh-ed25519 {} {}",
        STANDARD.encode(&ed),
        "😀".repeat(10000)
    );
    let out = run(&reg, "ssh_public_key_parse", text("data", &long), json!({})).unwrap();
    assert!(out["text"].as_text().unwrap().len() < 1500);
    assert_eq!(
        info(&out)["comment"].as_str().unwrap().chars().count(),
        10000
    );
}
#[test]
fn inspection_graphs_match_standalone_and_preserve_typed_downstream_values() {
    let reg = default_registry();
    for (id, port, input, output) in [
        (
            "ssh_public_key_parse",
            "data",
            PortValue::Bytes(blob("ssh-ed25519", &[&[1; 32]]).into()),
            "fingerprint",
        ),
        (
            "bitcoin_address_inspect",
            "text",
            PortValue::Text("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".into()),
            "script",
        ),
        (
            "ethereum_address_inspect",
            "text",
            PortValue::Text("0x52908400098527886E0F7030069857D2E4169EE7".into()),
            "address",
        ),
    ] {
        let graph=serde_json::from_value(json!({"nodes":[{"id":"inspect","descriptorId":id,"params":{}},{"id":"next","descriptorId":"coalesce","params":{}}],"edges":[{"from":{"node":"inspect","port":output},"to":{"node":"next","port":"a"}}]})).unwrap();
        let out = GraphExecutor::new(&reg, &graph)
            .unwrap()
            .with_seed_inputs([(("inspect".into(), port.into()), input.clone())].into())
            .run(&NullSink, &CancellationToken::new())
            .unwrap();
        let direct = run(&reg, id, [(port.into(), input)].into(), json!({})).unwrap();
        assert_eq!(
            serde_json::to_value(&direct).unwrap(),
            serde_json::to_value(&out["inspect"]).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&direct[output]).unwrap(),
            serde_json::to_value(&out["next"]["output"]).unwrap()
        );
    }
}
#[test]
fn key_and_address_nodes_observe_cancellation_before_output() {
    struct Stop(CancellationToken);
    impl ProgressSink for Stop {
        fn emit(&self, e: ProgressEvent) {
            if matches!(e,ProgressEvent::NodeProgress{pct,..}if pct>0.1&&pct<1.0) {
                self.0.cancel();
            }
        }
    }
    let reg = default_registry();
    for (id, inputs) in [
        (
            "ssh_public_key_parse",
            bytes("data", &blob("ssh-ed25519", &[&[1; 32]])),
        ),
        (
            "bitcoin_address_inspect",
            text("text", "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"),
        ),
        (
            "ethereum_address_inspect",
            text("text", "0x52908400098527886E0F7030069857D2E4169EE7"),
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
