//! Explicitly opt-in live integration bridge. Never linked into the application.
//! Credentials stay in this child process; stdin/stdout carry only test tasks.
use std::io::{BufRead, Write};
use std::sync::Arc;

use crate::{error::AppError, jobs::JobManager};
use misclab_core::{
    cancel::CancellationToken,
    graph::{executor::GraphExecutor, model::SerializedGraph},
    node::NodeEnv,
    progress::{ProgressEvent, ProgressSink},
};
use serde_json::{json, Value};

fn send(id: &str, kind: &str, value: Value) {
    let mut out = std::io::stdout().lock();
    writeln!(
        out,
        "AI_LIVE:{}",
        json!({"id":id,"kind":kind,"value":value})
    )
    .unwrap();
    out.flush().unwrap();
}
struct Sink(String);
impl ProgressSink for Sink {
    fn emit(&self, event: ProgressEvent) {
        let event = match event {
            ProgressEvent::NodeEntered { node } => json!({"kind":"nodeEntered","node":node}),
            ProgressEvent::NodeDone { node } => json!({"kind":"nodeDone","node":node}),
            ProgressEvent::NodeFailed { node, error } => {
                json!({"kind":"nodeFailed","node":node,"error":error})
            }
            ProgressEvent::NodeProgress { node, pct } => {
                json!({"kind":"nodeProgress","node":node,"pct":pct})
            }
            ProgressEvent::Log { node, message, .. } => {
                json!({"kind":"log","node":node,"message":message,"level":"info"})
            }
        };
        send(&self.0, "event", event);
    }
}

#[test]
#[ignore = "Requires explicit user authorization and saved AI configuration"]
fn live_agent_bridge() {
    assert_eq!(std::env::var("MISCLAB_LIVE_AI").as_deref(), Ok("1"));
    let cfg_path = std::path::PathBuf::from(std::env::var_os("APPDATA").unwrap())
        .join("com.lovelymisclab.app/settings.json");
    let env: NodeEnv = serde_json::from_slice(&std::fs::read(&cfg_path).unwrap()).unwrap();
    assert!(env.ai.llm.is_configured());
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let registry = Arc::new(misclab_core::nodes::default_registry());
    let jobs = Arc::new(JobManager::default());
    send("ready", "result", json!({"configured":true}));
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let Ok(request) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let id = request["id"].as_str().unwrap_or_default().to_owned();
        let op = request["op"].as_str().unwrap_or_default();
        if op == "cancel" {
            jobs.cancel(request["job"].as_str().unwrap_or_default());
            send(&id, "result", json!(null));
            continue;
        }
        if !["generate", "decide", "run"].contains(&op) {
            send(&id, "error", json!("Unsupported test operation"));
            continue;
        }
        let op = op.to_owned();
        let registry = registry.clone();
        let jobs = jobs.clone();
        // Pick up settings the user saves during the live test without exporting them.
        let env: NodeEnv = serde_json::from_slice(&std::fs::read(&cfg_path).unwrap()).unwrap();
        runtime.spawn(async move {
            let cancel = CancellationToken::new();
            let job = jobs.start(cancel.clone());
            send(&id, "event", json!({"kind":"jobStarted","job":job}));
            // Test-only one-shot HTTP faults exercise the production transport.
            // The fixture config has no saved key; user settings stay untouched.
            let cfg = if ["decide", "generate"].contains(&op.as_str()) {
                match request["fault"].as_str() {
                    Some(kind @ ("truncated" | "interrupted" | "http503")) => {
                        send(&id, "event", json!({"kind":"log","level":"info","message":format!("测试注入 HTTP 故障：{kind}（本次未请求 DeepSeek）")}));
                        super::super::ai_request::tests::server_with_response(
                            vec!["{\"choices\":".into()], std::time::Duration::ZERO,
                            if kind == "http503" { 503 } else { 200 }, true,
                            if kind == "interrupted" { 128 } else { 0 },
                        ).0
                    }
                    _ => {
                        send(&id, "event", json!({"kind":"log","level":"info","message":format!("真实模型请求：{}", env.ai.llm.model)}));
                        env.ai.llm.clone()
                    }
                }
            } else { env.ai.llm.clone() };
            let result: Result<Value, AppError> = if op == "decide" {
                super::super::ai_solver::decide_cancellable(&cfg, request["prompt"].as_str().unwrap_or_default(), &cancel)
                    .await.and_then(|decision| serde_json::to_value(decision).map_err(|e| AppError::new("serialize", e.to_string())))
            } else if op == "generate" {
                let prompt = request["prompt"].as_str().unwrap_or_default();
                let only: Option<Vec<String>> = request.get("catalogIds").and_then(|v| serde_json::from_value(v.clone()).ok());
                super::generate_filtered_cancellable(&registry, &cfg, prompt, &cancel, |message| {
                    send(
                        &id,
                        "event",
                        json!({"kind":"log","message":message,"level":"info"}),
                    );
                }, only.as_deref())
                .await
                .and_then(|plan| {
                    serde_json::to_value(plan)
                        .map_err(|e| AppError::new("serialize", e.to_string()))
                })
            } else {
                let graph: Result<SerializedGraph, _> =
                    serde_json::from_value(request["graph"].clone());
                match graph {
                    Err(e) => Err(AppError::new("input", e.to_string())),
                    Ok(graph)
                        if graph.nodes.iter().any(|n| {
                            // Exercise the full built-in local-processing catalog.
                            // No custom scripts, arbitrary HTTP or file export.
                            registry.get(&n.descriptor_id).is_none()
                                || ["http_request", "file_output", "ai_vision"]
                                    .contains(&n.descriptor_id.as_str())
                        }) =>
                    {
                        Err(AppError::new(
                            "test_scope",
                            "Live test only executes approved text/file/media fixture nodes",
                        ))
                    }
                    Ok(graph) if graph.nodes.iter().any(|n| {
                        let param = match n.descriptor_id.as_str() { "file_import" => "path", "image_input" => "image", _ => return false };
                        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.git/solver-fixtures").canonicalize();
                        let path = std::path::Path::new(n.params[param].as_str().unwrap_or_default()).canonicalize();
                        !matches!((root, path), (Ok(root), Ok(path)) if path.starts_with(&root) && path.file_name().is_some_and(|n| n != "cases.json"))
                    }) => Err(AppError::new("test_scope", "Live test may only read generated fixture inputs")),
                    Ok(graph) => {
                        let sink = Sink(id.clone());
                        tokio::task::spawn_blocking(move || {
                            let out = GraphExecutor::new(&registry, &graph)?
                                .with_env(env)
                                .run(&sink, &cancel)?;
                            serde_json::to_value(out)
                                .map_err(|e| AppError::new("serialize", e.to_string()))
                        })
                        .await
                        .unwrap_or_else(|e| Err(AppError::new("join", e.to_string())))
                    }
                }
            };
            jobs.finish(&job);
            match result {
                Ok(value) => send(&id, "result", value),
                Err(error) => send(&id, "error", json!(error)),
            }
        });
    }
}
