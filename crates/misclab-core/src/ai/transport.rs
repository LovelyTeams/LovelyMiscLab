use std::time::Duration;

use super::ModelConfig;
use crate::{cancel::CancellationToken, error::CoreError};

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct AiRequestError {
    pub code: &'static str,
    pub message: String,
}

impl AiRequestError {
    pub(super) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
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
        _ => format!("AI 请求失败：{}", error.without_url()),
    };
    AiRequestError::new("ai_http", message)
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
            .and_then(reqwest::Response::error_for_status)
            .map_err(request_error)?;
        response.json().await.map_err(|error| {
            AiRequestError::new(
                "ai_parse",
                format!("AI 响应解析失败：{}", error.without_url()),
            )
        })
    };
    tokio::select! {
        biased;
        _ = async { while !cancel.is_cancelled() { tokio::time::sleep(Duration::from_millis(25)).await; } } => Err(AiRequestError::new("cancelled", "已取消 AI 请求")),
        result = request => { check_cancel(cancel)?; result }
    }
}
