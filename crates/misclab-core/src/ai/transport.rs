use std::time::Duration;

use super::ModelConfig;
use crate::{cancel::CancellationToken, error::CoreError};

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct AiRequestError {
    pub code: &'static str,
    pub message: String,
    pub retryable: bool,
    pub retry_after_ms: Option<u64>,
}

impl AiRequestError {
    pub(super) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            retryable: false,
            retry_after_ms: None,
        }
    }
}

impl From<AiRequestError> for CoreError {
    fn from(error: AiRequestError) -> Self {
        if error.code == "cancelled" {
            Self::Cancelled
        } else {
            Self::Other(error.message)
        }
    }
}

pub(super) fn check_cancel(cancel: &CancellationToken) -> Result<(), AiRequestError> {
    if cancel.is_cancelled() {
        Err(AiRequestError::new("cancelled", "已取消 AI 请求"))
    } else {
        Ok(())
    }
}

fn request_error(error: reqwest::Error) -> AiRequestError {
    let retryable = match error.status().map(|status| status.as_u16()) {
        Some(status) => status == 408 || status == 429 || (500..600).contains(&status),
        None => {
            !error.is_builder()
                && (error.is_timeout()
                    || error.is_connect()
                    || error.is_body()
                    || error.is_decode()
                    || error.is_request())
        }
    };
    // Never include the provider URL or error body: they can contain credentials.
    let message = match error.status().map(|status| status.as_u16()) {
        Some(401) => {
            "模型服务鉴权失败（HTTP 401），请核对文本/识图模型的 API Key 与 Base URL 是否匹配。"
                .into()
        }
        Some(403) => "模型服务拒绝访问（HTTP 403），请检查密钥权限及模型访问权限。".into(),
        Some(404) => "模型接口或模型不存在（HTTP 404），请检查 Base URL 和模型名称。".into(),
        Some(429) => "模型服务限流或额度不足（HTTP 429），请稍后重试或检查服务额度。".into(),
        _ if error.is_timeout() => "AI 请求超时，请稍后重试。".into(),
        _ if error.is_body() || error.is_decode() => {
            "读取模型服务响应失败（连接中断或内容损坏），可以重试当前请求。".into()
        }
        _ => format!("AI 请求失败：{}", error.without_url()),
    };
    let mut result = AiRequestError::new("ai_http", message);
    result.retryable = retryable;
    result
}

pub(super) async fn post(
    cfg: &ModelConfig,
    body: serde_json::Value,
    cancel: &CancellationToken,
) -> Result<serde_json::Value, AiRequestError> {
    check_cancel(cancel)?;
    let request = async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(90))
            .build()
            .map_err(request_error)?;
        let response = client
            .post(cfg.endpoint())
            .bearer_auth(&cfg.api_key)
            .json(&body)
            .send()
            .await
            .map_err(request_error)?;
        let retry_after_ms = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse::<u64>().ok())
            .map(|seconds| seconds.saturating_mul(1000));
        let response = response.error_for_status().map_err(|error| {
            let mut error = request_error(error);
            error.retry_after_ms = retry_after_ms;
            error
        })?;
        let bytes = response.bytes().await.map_err(request_error)?;
        serde_json::from_slice(&bytes).map_err(|error| {
            // This is the provider's HTTP envelope, not model-authored plan JSON.
            // A gateway page or truncated envelope may recover on a new request.
            let mut result = AiRequestError::new("ai_response", if error.is_eof() {
                "模型服务响应为空或 JSON 被截断，可以重试当前请求。"
            } else {
                "模型服务返回了非 JSON 或损坏的响应，可以重试；持续失败时请检查 API 地址和服务状态。"
            });
            result.retryable = true;
            result
        })
    };
    tokio::select! {
        biased;
        _ = async { while !cancel.is_cancelled() { tokio::time::sleep(Duration::from_millis(25)).await; } } => Err(AiRequestError::new("cancelled", "已取消 AI 请求")),
        result = request => { check_cancel(cancel)?; result }
    }
}
