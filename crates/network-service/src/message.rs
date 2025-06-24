//! 网络消息定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// 消息类型标识符
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageType(pub String);

impl MessageType {
    pub fn new(type_name: &str) -> Self {
        Self(type_name.to_string())
    }

    /// 聊天消息类型
    pub fn chat() -> Self {
        Self("chat".to_string())
    }

    /// 授时消息类型
    pub fn timesync() -> Self {
        Self("timesync".to_string())
    }

    /// 系统消息类型
    pub fn system() -> Self {
        Self("system".to_string())
    }
}

/// 网络消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMessage {
    /// 唯一消息ID
    pub id: Uuid,
    /// 消息类型
    pub message_type: MessageType,
    /// 发送者ID
    pub sender: String,
    /// 消息内容（JSON格式）
    pub payload: serde_json::Value,
    /// 时间戳
    pub timestamp: u64,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

impl NetworkMessage {
    /// 创建新消息
    pub fn new(message_type: MessageType, sender: String, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type,
            sender,
            payload,
            timestamp: current_timestamp(),
            metadata: HashMap::new(),
        }
    }

    /// 添加元数据
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// 获取元数据
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// 序列化为字节
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// 从字节反序列化
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// 广播选项
#[derive(Debug, Clone)]
pub struct BroadcastOptions {
    /// 排除的节点列表
    pub exclude_nodes: Vec<String>,
    /// 是否等待响应
    pub wait_for_response: bool,
    /// 超时时间（毫秒）
    pub timeout_ms: Option<u64>,
    /// 重试次数
    pub retry_count: u32,
}

impl Default for BroadcastOptions {
    fn default() -> Self {
        Self {
            exclude_nodes: Vec::new(),
            wait_for_response: false,
            timeout_ms: Some(5000),
            retry_count: 0,
        }
    }
}

/// 单播选项
#[derive(Debug, Clone)]
pub struct UnicastOptions {
    /// 是否等待响应
    pub wait_for_response: bool,
    /// 超时时间（毫秒）
    pub timeout_ms: Option<u64>,
    /// 重试次数
    pub retry_count: u32,
}

impl Default for UnicastOptions {
    fn default() -> Self {
        Self {
            wait_for_response: false,
            timeout_ms: Some(5000),
            retry_count: 0,
        }
    }
}

/// 聊天消息负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatPayload {
    pub content: String,
    pub chat_type: ChatType,
}

/// 聊天类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatType {
    Text,
    Image,
    File,
}

/// 授时消息负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSyncPayload {
    pub request_type: TimeSyncRequestType,
    pub timestamp: Option<u64>,
}

/// 授时请求类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeSyncRequestType {
    GetTime,
    SyncTime { timestamp: u64 },
}

/// 获取当前时间戳
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
