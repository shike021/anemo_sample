use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// 聊天消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatMessage {
    /// 用户文本消息
    Text {
        sender: String,
        content: String,
        timestamp: u64,
    },
    /// 用户加入聊天室
    UserJoined { username: String, timestamp: u64 },
    /// 用户离开聊天室
    UserLeft { username: String, timestamp: u64 },
    /// 心跳消息
    Heartbeat { timestamp: u64 },
}

impl ChatMessage {
    /// 创建文本消息
    pub fn new_text(sender: String, content: String) -> Self {
        Self::Text {
            sender,
            content,
            timestamp: current_timestamp(),
        }
    }

    /// 创建用户加入消息
    pub fn new_user_joined(username: String) -> Self {
        Self::UserJoined {
            username,
            timestamp: current_timestamp(),
        }
    }

    /// 创建用户离开消息
    pub fn new_user_left(username: String) -> Self {
        Self::UserLeft {
            username,
            timestamp: current_timestamp(),
        }
    }

    /// 创建心跳消息
    pub fn new_heartbeat() -> Self {
        Self::Heartbeat {
            timestamp: current_timestamp(),
        }
    }

    /// 获取消息的发送者（如果有）
    pub fn sender(&self) -> Option<&str> {
        match self {
            ChatMessage::Text { sender, .. } => Some(sender),
            ChatMessage::UserJoined { username, .. } => Some(username),
            ChatMessage::UserLeft { username, .. } => Some(username),
            ChatMessage::Heartbeat { .. } => None,
        }
    }

    /// 获取消息时间戳
    pub fn timestamp(&self) -> u64 {
        match self {
            ChatMessage::Text { timestamp, .. } => *timestamp,
            ChatMessage::UserJoined { timestamp, .. } => *timestamp,
            ChatMessage::UserLeft { timestamp, .. } => *timestamp,
            ChatMessage::Heartbeat { timestamp } => *timestamp,
        }
    }

    /// 格式化消息用于显示
    pub fn format_for_display(&self) -> String {
        match self {
            ChatMessage::Text {
                sender, content, ..
            } => {
                format!("[{}]: {}", sender, content)
            }
            ChatMessage::UserJoined { username, .. } => {
                format!("*** {} 加入了聊天室 ***", username)
            }
            ChatMessage::UserLeft { username, .. } => {
                format!("*** {} 离开了聊天室 ***", username)
            }
            ChatMessage::Heartbeat { .. } => "*** 心跳 ***".to_string(),
        }
    }
}

/// 聊天请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub message: ChatMessage,
}

/// 聊天响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub success: bool,
    pub error_msg: Option<String>,
}

impl ChatResponse {
    pub fn success() -> Self {
        Self {
            success: true,
            error_msg: None,
        }
    }

    pub fn error(msg: String) -> Self {
        Self {
            success: false,
            error_msg: Some(msg),
        }
    }
}

/// 获取当前时间戳
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = ChatMessage::new_text("Alice".to_string(), "Hello World".to_string());
        assert_eq!(msg.sender(), Some("Alice"));

        let join_msg = ChatMessage::new_user_joined("Bob".to_string());
        assert_eq!(join_msg.sender(), Some("Bob"));
    }

    #[test]
    fn test_message_formatting() {
        let msg = ChatMessage::new_text("Alice".to_string(), "Hello".to_string());
        let formatted = msg.format_for_display();
        assert!(formatted.contains("Alice"));
        assert!(formatted.contains("Hello"));
    }
}
