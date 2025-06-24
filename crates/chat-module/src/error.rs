//! 聊天模块错误处理

use thiserror::Error;

/// 聊天模块错误类型
#[derive(Error, Debug)]
pub enum ChatError {
    #[error("网络服务错误: {0}")]
    NetworkError(#[from] network_service::NetworkError),

    #[error("用户未找到: {0}")]
    UserNotFound(String),

    #[error("聊天室未found: {0}")]
    RoomNotFound(String),

    #[error("用户 {0} 未加入聊天室 {1}")]
    UserNotInRoom(String, String),

    #[error("聊天室 {0} 已存在")]
    RoomAlreadyExists(String),

    #[error("用户 {0} 已在聊天室 {1} 中")]
    UserAlreadyInRoom(String, String),

    #[error("消息为空")]
    EmptyMessage,

    #[error("无效的聊天室名称: {0}")]
    InvalidRoomName(String),

    #[error("无效的用户名: {0}")]
    InvalidUsername(String),

    #[error("序列化错误: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("内部错误: {0}")]
    InternalError(String),

    #[error("其他错误: {0}")]
    Other(#[from] anyhow::Error),
}

/// 聊天模块结果类型
pub type Result<T> = std::result::Result<T, ChatError>;
