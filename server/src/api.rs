use common::{MessageType, PushResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 推送请求体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushRequest {
    /// 目标平台
    pub platform: String,
    /// 平台的配置信息 (e.g., webhook url, secret)
    /// 使用 serde_json::Value 以支持不同平台的异构配置
    pub config: Value,
    /// 消息内容
    pub message: MessageType,
}

/// 推送响应体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushResponse {
    /// 推送结果
    pub result: PushResult,
}
