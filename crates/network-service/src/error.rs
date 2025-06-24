//! 网络服务错误处理

use thiserror::Error;

/// 网络服务错误类型
#[derive(Debug, Error)]
pub enum NetworkError {
    /// 配置错误
    #[error("配置错误: {0}")]
    ConfigError(String),

    /// 连接错误
    #[error("连接错误: {0}")]
    ConnectionError(String),

    /// 消息发送错误
    #[error("消息发送错误: {0}")]
    SendError(String),

    /// 消息接收错误
    #[error("消息接收错误: {0}")]
    ReceiveError(String),

    /// 序列化错误
    #[error("序列化错误: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// IO错误
    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),

    /// 超时错误
    #[error("操作超时")]
    TimeoutError,

    /// 节点不存在
    #[error("节点不存在: {0}")]
    NodeNotFound(String),

    /// 内部错误
    #[error("内部错误: {0}")]
    InternalError(String),

    /// Anemo网络错误
    #[error("Anemo错误: {0:?}")]
    AnemoError(anemo::types::PeerEvent),

    #[error("其他错误: {0}")]
    Other(String),
}

impl From<anemo::types::PeerEvent> for NetworkError {
    fn from(event: anemo::types::PeerEvent) -> Self {
        NetworkError::AnemoError(event)
    }
}

/// 网络服务结果类型
pub type Result<T> = std::result::Result<T, NetworkError>;

impl NetworkError {
    /// 创建配置错误
    pub fn config_error(msg: impl Into<String>) -> Self {
        NetworkError::ConfigError(msg.into())
    }

    /// 创建连接错误
    pub fn connection_error(msg: impl Into<String>) -> Self {
        NetworkError::ConnectionError(msg.into())
    }

    /// 创建发送错误
    pub fn send_error(msg: impl Into<String>) -> Self {
        NetworkError::SendError(msg.into())
    }

    /// 创建接收错误
    pub fn receive_error(msg: impl Into<String>) -> Self {
        NetworkError::ReceiveError(msg.into())
    }

    /// 创建节点不存在错误
    pub fn node_not_found(node_id: impl Into<String>) -> Self {
        NetworkError::NodeNotFound(node_id.into())
    }

    /// 创建内部错误
    pub fn internal_error(msg: impl Into<String>) -> Self {
        NetworkError::InternalError(msg.into())
    }

    /// 创建其他错误
    pub fn other(msg: impl Into<String>) -> Self {
        NetworkError::Other(msg.into())
    }
}
