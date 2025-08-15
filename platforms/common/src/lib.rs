use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;

/// 推送平台错误类型
#[derive(Debug, thiserror::Error)]
pub enum PushError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Message error: {0}")]
    MessageError(String),

    #[error("Platform error: {0}")]
    PlatformError(String),
}

/// 消息类型枚举
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum MessageType {
    /// 纯文本消息
    Text(String),
    /// Markdown格式消息
    Markdown(String),
    /// 富文本消息
    Rich {
        title: String,
        content: String,
        url: Option<String>,
    },
    /// 图片消息
    Image {
        url: String,
        caption: Option<String>,
    },
    /// 链接消息
    Link {
        title: String,
        description: String,
        url: String,
        image_url: Option<String>,
    },
}

/// 消息优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Normal,
    High,
    Urgent,
}

/// 推送结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushResult {
    /// 消息ID
    pub message_id: Option<String>,
    /// 是否成功
    pub success: bool,
    /// 响应信息
    pub response: Option<String>,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

impl Default for PushResult {
    fn default() -> Self {
        Self {
            message_id: None,
            success: false,
            response: None,
            timestamp: Utc::now(),
        }
    }
}

/// 初始化配置trait
pub trait PushInitConfig: Send + Sync {
    /// 获取平台名称
    fn platform_name(&self) -> &str;

    /// 获取webhook URL
    fn webhook_url(&self) -> String;

    /// 获取密钥
    fn secret(&self) -> Option<&str>;

    /// 获取超时时间（秒）
    fn timeout(&self) -> u64;

    /// 获取重试次数
    fn retry_count(&self) -> u32;
}

/// 推送平台能力trait（用于dyn兼容）
#[async_trait]
pub trait PushPlatformCapabilities: Send + Sync {
    /// 初始化平台
    async fn init(&mut self) -> Result<(), PushError>;

    /// 发送纯文本消息
    async fn send_text(&self, content: &str) -> Result<PushResult, PushError>;
    async fn send_text_with_mention(&self, content: &str,mention_list:Vec<String>) -> Result<PushResult, PushError>;

    /// 发送Markdown消息
    async fn send_markdown(&self, content: &str) -> Result<PushResult, PushError>;

    /// 发送富文本消息
    async fn send_rich(
        &self,
        title: &str,
        content: &str,
        url: Option<&str>,
    ) -> Result<PushResult, PushError>;

    /// 发送图片消息
    async fn send_image(
        &self,
        image_url: &str,
        caption: Option<&str>,
    ) -> Result<PushResult, PushError>;

    /// 发送链接消息
    async fn send_link(
        &self,
        title: &str,
        description: &str,
        url: &str,
        image_url: Option<&str>,
    ) -> Result<PushResult, PushError>;

    /// 通用发送方法
    async fn send(&self, message: MessageType) -> Result<PushResult, PushError>;

    /// 检查平台健康状态
    async fn health_check(&self) -> Result<bool, PushError>;

    /// 获取平台信息
    fn platform_info(&self) -> PlatformInfo;
}

/// 推送平台trait（用于具体实现）
pub trait PushPlatform<C: PushInitConfig>: PushPlatformCapabilities {
    /// 创建一个新的推送平台实例
    fn new(config: C) -> Self
    where
        Self: Sized;
}

/// 平台信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    /// 平台名称
    pub name: String,
    /// 版本
    pub version: String,
    /// 支持的特性
    pub features: Vec<String>,
    /// 是否支持Markdown
    pub supports_markdown: bool,
    /// 是否支持富文本
    pub supports_rich_text: bool,
    /// 是否支持图片
    pub supports_images: bool,
}

/// 消息构建器
pub struct MessageBuilder {
    message_type: MessageType,
    priority: Priority,
    mentions: Vec<String>,
}

impl MessageBuilder {
    /// 创建文本消息构建器
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::Text(content.into()),
            priority: Priority::Normal,
            mentions: Vec::new(),
        }
    }

    /// 创建Markdown消息构建器
    pub fn markdown(content: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::Markdown(content.into()),
            priority: Priority::Normal,
            mentions: Vec::new(),
        }
    }

    /// 创建富文本消息构建器
    pub fn rich(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::Rich {
                title: title.into(),
                content: content.into(),
                url: None,
            },
            priority: Priority::Normal,
            mentions: Vec::new(),
        }
    }

    /// 设置优先级
    pub fn priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// 添加@提及
    pub fn mention(mut self, user: impl Into<String>) -> Self {
        self.mentions.push(user.into());
        self
    }

    /// 添加多个@提及
    pub fn mentions(mut self, users: Vec<String>) -> Self {
        self.mentions.extend(users);
        self
    }

    /// 构建消息
    pub fn build(self) -> MessageType {
        self.message_type
    }
}

/// 平台工厂trait
pub trait PlatformFactory: Send + Sync {
    /// 根据JSON Value创建平台实例
    fn create(&self, config: Value) -> Result<Box<dyn PushPlatformCapabilities>, PushError>;

    /// 获取平台名称
    fn name(&self) -> &'static str;
}

/// 平台注册表
pub struct PlatformRegistry {
    factories: std::collections::HashMap<String, Box<dyn PlatformFactory>>,
}

impl PlatformRegistry {
    /// 创建新的注册表
    pub fn new() -> Self {
        Self {
            factories: std::collections::HashMap::new(),
        }
    }

    /// 注册平台工厂
    pub fn register(&mut self, factory: Box<dyn PlatformFactory>) {
        self.factories.insert(factory.name().to_string(), factory);
    }

    /// 获取平台工厂
    pub fn get_factory(&self, name: &str) -> Option<&Box<dyn PlatformFactory>> {
        self.factories.get(name)
    }

    /// 获取所有支持的平名名称
    pub fn list_platforms(&self) -> Vec<String> {
        self.factories.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 测试用的mock配置
    struct MockConfig;

    // impl PushInitConfig for MockConfig {
    //     fn platform_name(&self) -> &'static str {
    //         "mock"
    //     }
    //
    //     fn webhook_url(&self) -> &str {
    //         "https://mock.example.com/webhook"
    //     }
    //
    //     fn secret(&self) -> Option<&str> {
    //         Some("mock-secret")
    //     }
    //
    //     fn timeout(&self) -> u64 {
    //         30
    //     }
    //
    //     fn retry_count(&self) -> u32 {
    //         3
    //     }
    // }

    #[test]
    fn test_message_builder() {
        let text_msg = MessageBuilder::text("Hello World").build();
        match text_msg {
            MessageType::Text(content) => assert_eq!(content, "Hello World"),
            _ => panic!("Expected text message"),
        }

        let md_msg = MessageBuilder::markdown("# Hello").build();
        match md_msg {
            MessageType::Markdown(content) => assert_eq!(content, "# Hello"),
            _ => panic!("Expected markdown message"),
        }
    }

    #[test]
    fn test_push_result_default() {
        let result = PushResult::default();
        assert_eq!(result.success, false);
        assert!(result.message_id.is_none());
    }

    #[test]
    fn test_platform_registry() {
        let registry = PlatformRegistry::new();
        assert!(registry.list_platforms().is_empty());
    }
}
