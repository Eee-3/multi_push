use anyhow::Result;
use async_trait::async_trait;
use common::{
    MessageType, PlatformFactory, PlatformInfo, PushError, PushInitConfig, PushPlatform,
    PushPlatformCapabilities, PushResult,
};
use log::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const PLATFORM_NAME: &str = "wxwork";
const BASE_URL: &str = "https://qyapi.weixin.qq.com/cgi-bin/webhook/send";

/// 企业微信机器人配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WxWorkConfig {
    pub token: String,
}

impl PushInitConfig for WxWorkConfig {
    fn platform_name(&self) -> &str {
        PLATFORM_NAME
    }

    fn webhook_url(&self) -> String {
        format!("{BASE_URL}?key={}", self.token)
    }

    fn secret(&self) -> Option<&str> {
        None // 企业微信机器人通常不使用独立的 secret
    }

    fn timeout(&self) -> u64 {
        30
    }

    fn retry_count(&self) -> u32 {
        3
    }
}

/// 企业微信群机器人推送平台
pub struct WxWorkGroupBotPlatform {
    config: WxWorkConfig,
    http_client: Client,
}

#[async_trait]
impl PushPlatformCapabilities for WxWorkGroupBotPlatform {
    async fn init(&mut self) -> Result<(), PushError> {
        // No specific init logic needed for WxWork bot
        Ok(())
    }

    async fn send_text(&self, content: &str) -> Result<PushResult, PushError> {
        let payload = WxWorkTextPayload {
            msgtype: "text".to_string(),
            text: WxWorkText {
                content: content.to_string(),
                mentioned_list: vec![],
                mentioned_mobile_list: vec![],
            },

        };
        self.send_request(payload).await
    }
    async fn send_text_with_mention(&self, content: &str, mention_list: Vec<String>) -> std::result::Result<PushResult, PushError>  {
        let payload = WxWorkTextPayload {
            msgtype: "text".to_string(),
            text: WxWorkText {
                content: content.to_string(),
                mentioned_list: mention_list,
                mentioned_mobile_list: vec![],
            },
        };
        self.send_request(payload).await
    }

    async fn send_markdown(&self, content: &str) -> Result<PushResult, PushError> {
        let payload = WxWorkMarkdownPayload {
            msgtype: "markdown".to_string(),
            markdown: WxWorkMarkdown {
                content: content.to_string(),
            },
        };
        self.send_request(payload).await
    }

    async fn send_rich(
        &self,
        _title: &str,
        _content: &str,
        _url: Option<&str>,
    ) -> Result<PushResult, PushError> {
        Err(PushError::PlatformError(
            "WxWork Bot does not support rich text messages directly.".to_string(),
        ))
    }

    async fn send_image(
        &self,
        _image_url: &str,
        _caption: Option<&str>,
    ) -> Result<PushResult, PushError> {
        Err(PushError::PlatformError(
            "WxWork Bot image sending requires pre-uploading.".to_string(),
        ))
    }

    async fn send_link(
        &self,
        _title: &str,
        _description: &str,
        _url: &str,
        _image_url: Option<&str>,
    ) -> Result<PushResult, PushError> {
        Err(PushError::PlatformError(
            "WxWork Bot does not support link messages.".to_string(),
        ))
    }

    async fn send(&self, message: MessageType) -> Result<PushResult, PushError> {
        match message {
            MessageType::Text(content) => self.send_text(&content).await,
            MessageType::Markdown(content) => self.send_markdown(&content).await,
            _ => Err(PushError::MessageError(
                "Unsupported message type for WxWork Bot".to_string(),
            )),
        }
    }

    async fn health_check(&self) -> Result<bool, PushError> {
        // A simple health check could be trying to send a test message to a dev-only bot
        // For now, we assume it's healthy if the client can be built.
        Ok(true)
    }

    fn platform_info(&self) -> PlatformInfo {
        PlatformInfo {
            name: PLATFORM_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            features: vec!["text".to_string(), "markdown".to_string()],
            supports_markdown: true,
            supports_rich_text: false,
            supports_images: false,
        }
    }
}

impl PushPlatform<WxWorkConfig> for WxWorkGroupBotPlatform {
    fn new(config: WxWorkConfig) -> Self
    where
        Self: Sized,
    {
        Self {
            config,
            http_client: Client::new(),
        }
    }
}

impl WxWorkGroupBotPlatform {
    async fn send_request<T: Serialize>(&self, payload: T) -> Result<PushResult, PushError> {
        let response = self
            .http_client
            .post(self.config.webhook_url())
            .json(&payload)
            .send()
            .await
            .map_err(|e| PushError::NetworkError(e.to_string()))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| PushError::NetworkError(e.to_string()))?;

        if status.is_success() {
            let wx_response: WxWorkResponse =
                serde_json::from_str(&text).map_err(|e| PushError::PlatformError(e.to_string()))?;
            if wx_response.errcode == 0 {
                Ok(PushResult {
                    success: true,
                    response: Some(text),
                    ..Default::default()
                })
            } else {
                Err(PushError::PlatformError(format!(
                    "WxWork API Error: code={}, message={}",
                    wx_response.errcode, wx_response.errmsg
                )))
            }
        } else {
            Err(PushError::NetworkError(format!(
                "Request failed with status: {}, body: {}",
                status, text
            )))
        }
    }
}

// --- WxWork API Payload Structs ---

#[derive(Serialize)]
struct WxWorkTextPayload {
    msgtype: String,
    text: WxWorkText,
}

#[derive(Serialize)]
struct WxWorkText {
    content: String,
    mentioned_list: Vec<String>,
    mentioned_mobile_list: Vec<String>,
}

#[derive(Serialize)]
struct WxWorkMarkdownPayload {
    msgtype: String,
    markdown: WxWorkMarkdown,
}

#[derive(Serialize)]
struct WxWorkMarkdown {
    content: String,
}

#[derive(Deserialize)]
struct WxWorkResponse {
    errcode: i32,
    errmsg: String,
}

// --- Platform Factory ---

pub struct WxWorkPlatformFactory;

impl PlatformFactory for WxWorkPlatformFactory {
    fn create(&self, config: Value) -> Result<Box<dyn PushPlatformCapabilities>, PushError> {
        let config: WxWorkConfig =
            serde_json::from_value(config).map_err(|e| PushError::ConfigError(e.to_string()))?;
        let platform = WxWorkGroupBotPlatform::new(config);
        Ok(Box::new(platform))
    }

    fn name(&self) -> &'static str {
        PLATFORM_NAME
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use super::*;

    #[tokio::test]
    async fn test_text_message() {
        let wx_work_platform = WxWorkGroupBotPlatform::new(WxWorkConfig {
            token: env::var("WXWORK_TOKEN").unwrap_or("".to_string()),
        });
    let result = wx_work_platform.send_text_with_mention("Test",vec!["@all".to_string()]).await;
        assert!(result.is_ok());
        println!("{:?}", result.unwrap());
    }
}
