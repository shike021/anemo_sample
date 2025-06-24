//! 授时模块错误处理

use thiserror::Error;

/// 授时模块错误类型
#[derive(Error, Debug)]
pub enum TimeSyncError {
    #[error("网络服务错误: {0}")]
    NetworkError(#[from] network_service::NetworkError),

    #[error("时间请求超时")]
    RequestTimeout,

    #[error("无效的时间戳: {0}")]
    InvalidTimestamp(i64),

    #[error("同步失败: {0}")]
    SyncFailed(String),

    #[error("心跳服务未启动")]
    HeartbeatNotStarted,

    #[error("心跳服务已启动")]
    HeartbeatAlreadyStarted,

    #[error("无效的同步间隔: {0}ms")]
    InvalidSyncInterval(u64),

    #[error("时间偏移过大: {0}ms")]
    TimeOffsetTooLarge(i64),

    #[error("序列化错误: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("系统时间错误: {0}")]
    SystemTimeError(String),

    #[error("内部错误: {0}")]
    InternalError(String),

    #[error("其他错误: {0}")]
    Other(#[from] anyhow::Error),
}

/// 授时模块结果类型
pub type Result<T> = std::result::Result<T, TimeSyncError>;
