use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use misclab_core::{
    ai::{AiConfig, ModelConfig},
    cancel::CancellationToken,
    graph::{executor::GraphExecutor, port::PortValue},
    node::{NodeEnv, PortMap},
    progress::NullSink,
    CoreError,
};
use serde_json::json;

fn assert_node_cancels(node: &'static str, input: &'static str, value: &'static str) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let (started_tx, started_rx) = mpsc::channel();
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(e) => panic!("accept: {e}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut chunk = [0; 4096];
        let header_end = loop {
            let n = stream.read(&mut chunk).unwrap();
            assert!(n > 0);
            bytes.extend_from_slice(&chunk[..n]);
            if let Some(pos) = bytes.windows(4).position(|s| s == b"\r\n\r\n") {
                break pos + 4;
            }
        };
        let length: usize = String::from_utf8_lossy(&bytes[..header_end])
            .lines()
            .find_map(|line| {
                let (key, value) = line.split_once(':')?;
                key.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse().unwrap())
            })
            .unwrap();
        while bytes.len() < header_end + length {
            let n = stream.read(&mut chunk).unwrap();
            assert!(n > 0);
            bytes.extend_from_slice(&chunk[..n]);
        }
        started_tx.send(()).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(500)))
            .unwrap();
        let close_result = stream.read(&mut chunk);
        let closed = match close_result {
            Ok(0) => true,
            Err(error) => matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
            ),
            _ => false,
        };
        // The client must stop before this deliberately late provider reply.
        let body = json!({"choices":[{"message":{"content":"late result"}}]}).to_string();
        let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
        closed
    });
    let cfg = ModelConfig {
        model: "fixture".into(),
        api_key: String::new(),
        base_url: url,
    };
    let env = NodeEnv {
        ai: AiConfig {
            llm: cfg.clone(),
            vision: cfg,
        },
        ..Default::default()
    };
    let cancel = CancellationToken::new();
    let worker_token = cancel.clone();
    let (result_tx, result_rx) = mpsc::channel();
    let worker = thread::spawn(move || {
        let registry = misclab_core::nodes::default_registry();
        let inputs = PortMap::from([(input.into(), PortValue::Text(value.into()))]);
        let result = GraphExecutor::run_node_with_env(
            &registry,
            node,
            &inputs,
            &json!({}),
            &env,
            &NullSink,
            &worker_token,
        );
        let _ = result_tx.send(result);
    });
    started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    cancel.cancel();
    let result = result_rx.recv_timeout(Duration::from_millis(300));
    worker.join().unwrap();
    let request_closed = server.join().unwrap();
    assert!(
        matches!(result, Ok(Err(CoreError::Cancelled))),
        "{node} must cancel before the provider replies: {result:?}"
    );
    assert!(
        request_closed,
        "cancellation must close the pending HTTP request"
    );
}

#[test]
fn text_ai_node_cancels_while_provider_is_responding() {
    assert_node_cancels("ai_judge", "text", "Hello");
}

#[test]
fn vision_ai_node_cancels_while_provider_is_responding() {
    assert_node_cancels("ai_vision", "image", "data:image/png;base64,aGVsbG8=");
}
