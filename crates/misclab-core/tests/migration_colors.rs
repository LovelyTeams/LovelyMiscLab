use misclab_core::{
    cancel::CancellationToken,
    error::CoreError,
    graph::{executor::GraphExecutor, port::PortValue},
    node::{registry::NodeRegistry, PortMap},
    nodes::default_registry,
    progress::NullSink,
};
use serde_json::{json, Value};
fn run(reg: &NodeRegistry, id: &str, s: &str, p: Value) -> Result<PortMap, CoreError> {
    GraphExecutor::run_node(
        reg,
        id,
        &[("text".into(), PortValue::Text(s.into()))].into(),
        &p,
        &NullSink,
        &CancellationToken::new(),
    )
}
#[test]
fn pinned_color_vectors_cover_all_names_formats_directions_and_case_options() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/ebctf-color-batch16.json")).unwrap();
    assert_eq!(
        fixture["revision"],
        "ea2bc36dae31c97060f7f2cddad8df5518afad7b"
    );
    assert_eq!(fixture["namedColors"], 147);
    let vectors = fixture["vectors"].as_array().unwrap();
    assert_eq!(vectors.len(), 3932);
    let reg = default_registry();
    let mut errors = Vec::new();
    for (i, v) in vectors.iter().enumerate() {
        let id = v["node"].as_str().unwrap();
        let out = run(
            &reg,
            id,
            v["input"]["text"]["value"].as_str().unwrap(),
            v["params"].clone(),
        );
        if v.get("error").is_some() {
            if out.is_ok() {
                errors.push(format!("{i} {id}: expected error"));
            }
            continue;
        }
        match out {
            Err(e) => errors.push(format!("{i} {id}: {e}")),
            Ok(out) => {
                if out["text"].as_text().unwrap() != v["text"].as_str().unwrap() {
                    errors.push(format!(
                        "{i} {id} {:?} {:?}: got {:?}, expected {:?}",
                        v["input"],
                        v["params"],
                        out["text"].as_text().unwrap(),
                        v["text"]
                    ));
                }
            }
        }
    }
    assert!(
        errors.is_empty(),
        "{} differences\n{}",
        errors.len(),
        errors.into_iter().take(30).collect::<Vec<_>>().join("\n")
    );
}
#[test]
fn structured_rgb_is_exact_and_names_have_deterministic_ties() {
    let reg = default_registry();
    for (s, expected) in [
        ("#123456", [0x12, 0x34, 0x56]),
        ("1,2,3", [1, 2, 3]),
        ("1,1,1", [255, 255, 255]),
        ("hsl(0,100%,50%)", [255, 0, 0]),
        ("hsv(240,1,1)", [0, 0, 255]),
        ("cmyk(0,0,0,1)", [0, 0, 0]),
    ] {
        let out = run(&reg, "color_info", s, json!({})).unwrap();
        assert_eq!(out["bytes"].as_bytes().unwrap().as_ref(), expected);
        for (i, key) in ["red", "green", "blue"].into_iter().enumerate() {
            let PortValue::Number(n) = out[key] else {
                panic!()
            };
            assert_eq!(n, expected[i] as f64);
        }
        let PortValue::Json(j) = &out["json"] else {
            panic!()
        };
        assert_eq!(j["rgb"], json!(expected));
    }
    for (color, name) in [("cyan", "aqua"), ("magenta", "fuchsia"), ("grey", "gray")] {
        let out = run(&reg, "color_info", color, json!({})).unwrap();
        let PortValue::Json(j) = &out["json"] else {
            panic!()
        };
        assert_eq!(j["name"], name);
    }
}
#[test]
fn color_graph_flows_into_binary_analysis_and_matches_standalone() {
    let reg = default_registry();
    let graph = serde_json::from_value(json!({"nodes":[
        {"id":"c","descriptorId":"color_info","params":{}},
        {"id":"s","descriptorId":"hex_stats","params":{}}
    ],"edges":[{"from":{"node":"c","port":"bytes"},"to":{"node":"s","port":"data"}}]}))
    .unwrap();
    let seed = [(
        ("c".into(), "text".into()),
        PortValue::Text("#123456".into()),
    )]
    .into();
    let out = GraphExecutor::new(&reg, &graph)
        .unwrap()
        .with_seed_inputs(seed)
        .run(&NullSink, &CancellationToken::new())
        .unwrap();
    let direct = run(&reg, "color_info", "#123456", json!({})).unwrap();
    assert_eq!(
        serde_json::to_value(&out["c"]).unwrap(),
        serde_json::to_value(direct).unwrap()
    );
    assert!(out["s"]["text"].as_text().unwrap().contains('3'));
}
#[test]
fn invalid_color_data_is_rejected_without_nonfinite_results() {
    let reg = default_registry();
    for s in [
        "rgb(NaN,0,0)",
        "rgb(Infinity,0,0)",
        "hsl(NaN,1,1)",
        "hsl(1x,50,50)",
        "rgb(10,20,30)garbage",
        "0 0 oops 0",
    ] {
        assert!(run(&reg, "color_info", s, json!({})).is_err(), "{s}");
    }
    for p in [
        json!({"from":"bogus"}),
        json!({"to":"bogus"}),
        json!({"from":"int"}),
    ] {
        assert!(run(&reg, "color_convert_encode", "16777216", p).is_err());
    }
    assert!(run(&reg, "color_info", &"0".repeat(4097), json!({})).is_err());
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert!(matches!(
        GraphExecutor::run_node(
            &reg,
            "color_info",
            &[("text".into(), PortValue::Text("red".into()))].into(),
            &json!({}),
            &NullSink,
            &cancel
        ),
        Err(CoreError::Cancelled)
    ));
}
