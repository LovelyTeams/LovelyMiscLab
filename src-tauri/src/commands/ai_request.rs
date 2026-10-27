//! Async planning transport. Dropping the request future closes the local HTTP
//! request; a remote provider may still finish work it already accepted.
#[cfg(test)]
use std::time::Duration;

use misclab_core::{ai::ModelConfig, cancel::CancellationToken};

use crate::error::AppError;

pub(super) async fn chat(
    cfg: &ModelConfig,
    system: &str,
    user: &str,
    cancel: &CancellationToken,
) -> Result<String, AppError> {
    misclab_core::ai::chat_cancellable(cfg, system, user, cancel)
        .await
        .map_err(|error| AppError::new(error.code, error.message))
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
        time::Instant,
    };

    pub(crate) fn server(
        replies: Vec<String>,
        delay: Duration,
    ) -> (
        ModelConfig,
        tokio::sync::oneshot::Receiver<()>,
        thread::JoinHandle<Vec<Value>>,
    ) {
        server_with_status(replies, delay, 200)
    }

    fn server_with_status(
        replies: Vec<String>,
        delay: Duration,
        status: u16,
    ) -> (
        ModelConfig,
        tokio::sync::oneshot::Receiver<()>,
        thread::JoinHandle<Vec<Value>>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let (started, received) = tokio::sync::oneshot::channel();
        let handle = thread::spawn(move || {
            let mut started = Some(started);
            let mut bodies = vec![];
            for reply in replies {
                let deadline = Instant::now() + Duration::from_secs(10);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(e)
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                && Instant::now() < deadline =>
                        {
                            thread::sleep(Duration::from_millis(5))
                        }
                        Err(e) => panic!("fixture accept: {e}"),
                    }
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = vec![];
                let mut chunk = [0; 4096];
                let end = loop {
                    let n = stream.read(&mut chunk).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&chunk[..n]);
                    if let Some(pos) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        break pos + 4;
                    }
                };
                let header = String::from_utf8_lossy(&bytes[..end]);
                let length: usize = header
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse().unwrap())
                    })
                    .unwrap();
                while bytes.len() < end + length {
                    let n = stream.read(&mut chunk).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&chunk[..n]);
                }
                bodies.push(serde_json::from_slice(&bytes[end..end + length]).unwrap());
                if let Some(sender) = started.take() {
                    let _ = sender.send(());
                }
                thread::sleep(delay);
                let body = json!({"choices":[{"message":{"content":reply}}]}).to_string();
                let response = format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                let _ = stream.write_all(response.as_bytes());
            }
            bodies
        });
        (
            ModelConfig {
                model: "fixture".into(),
                api_key: String::new(),
                base_url: format!("http://{address}/v1"),
            },
            received,
            handle,
        )
    }

    #[tokio::test]
    async fn cancels_an_inflight_http_request_before_response() {
        let (cfg, started, server) = server(vec!["late".into()], Duration::from_millis(300));
        let cancel = CancellationToken::new();
        let token = cancel.clone();
        let task = tokio::spawn(async move { chat(&cfg, "system", "user", &token).await });
        started.await.unwrap();
        cancel.cancel();
        let error = tokio::time::timeout(Duration::from_millis(200), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err();
        assert_eq!(error.code, "cancelled");
        server.join().unwrap();
    }

    #[tokio::test]
    async fn reports_missing_configuration_and_empty_provider_response() {
        let token = CancellationToken::new();
        assert_eq!(
            chat(&ModelConfig::default(), "", "", &token)
                .await
                .unwrap_err()
                .code,
            "ai_config"
        );
        let (cfg, _, server) = server(vec![String::new()], Duration::ZERO);
        assert_eq!(
            chat(&cfg, "", "", &token).await.unwrap_err().code,
            "ai_empty"
        );
        server.join().unwrap();
        token.cancel();
        assert!(chat(&cfg, "", "", &token).await.is_err());
    }

    #[tokio::test]
    async fn reports_http_status_without_echoing_url_key_or_provider_body() {
        for status in [401, 403, 404, 429] {
            let (mut cfg, _, server) =
                server_with_status(vec!["secret-provider-body".into()], Duration::ZERO, status);
            cfg.api_key = "secret-test-key".into();
            let error = chat(&cfg, "system", "user", &CancellationToken::new())
                .await
                .unwrap_err();
            assert_eq!(error.code, "ai_http");
            assert!(error.message.contains(&format!("HTTP {status}")));
            for secret in [&cfg.api_key, &cfg.base_url, "secret-provider-body"] {
                assert!(!error.message.contains(secret));
            }
            server.join().unwrap();
        }
    }

    #[test]
    fn shared_sync_transport_preserves_text_and_vision_requests() {
        let (cfg, _, server) = server(vec!["中文🙂\n".into(), "picture".into()], Duration::ZERO);
        let cancel = CancellationToken::new();
        assert_eq!(
            misclab_core::ai::chat_with_cancel(&cfg, "system", "input", &cancel).unwrap(),
            "中文🙂\n"
        );
        assert_eq!(
            misclab_core::ai::vision_with_cancel(
                &cfg,
                "describe",
                "data:image/png;base64,test",
                &cancel
            )
            .unwrap(),
            "picture"
        );
        let bodies = server.join().unwrap();
        assert_eq!(bodies[0]["messages"][0]["content"], "system");
        assert_eq!(bodies[0]["messages"][1]["content"], "input");
        assert_eq!(
            bodies[1]["messages"][0]["content"][1]["image_url"]["url"],
            "data:image/png;base64,test"
        );
        cancel.cancel();
        assert!(matches!(
            misclab_core::ai::chat_with_cancel(&cfg, "", "", &cancel),
            Err(misclab_core::CoreError::Cancelled)
        ));
        assert!(matches!(
            misclab_core::ai::vision_with_cancel(&cfg, "", "", &cancel),
            Err(misclab_core::CoreError::Cancelled)
        ));
    }
}
